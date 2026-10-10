//! UI Automation element scanning.
//!
//! This is the COM boundary. Every call that can fail returns a Result so that
//! a broken automation server degrades instead of crashing, which the original
//! C++ did not do (it dereferenced a possibly-null IUIAutomation).

use crate::geometry::{sanitize, Monitor, Rect};
use windows::Win32::Foundation::{HWND, LPARAM, RECT};
use windows::Win32::System::Com::{
    CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER, COINIT_MULTITHREADED,
};
use windows::Win32::UI::Accessibility::{
    CUIAutomation, IUIAutomation, IUIAutomationElement, TreeScope_Children, TreeScope_Subtree,
};
use windows::Win32::UI::HiDpi::{GetDpiForSystem, GetDpiForWindow};

/// COM must be uninitialised on the same thread that initialised it, so the
/// guard keeps the pairing explicit and panic-free.
pub struct ComGuard {
    initialised: bool,
}

impl ComGuard {
    /// Enter a multithreaded apartment, as Microsoft requires for UI Automation
    /// clients that inspect the whole desktop.
    pub fn new() -> Self {
        // S_FALSE (already initialised) and S_OK both mean usable.
        let hr = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
        Self {
            initialised: hr.is_ok(),
        }
    }
}

impl Drop for ComGuard {
    fn drop(&mut self) {
        if self.initialised {
            unsafe { CoUninitialize() };
        }
    }
}

impl Default for ComGuard {
    fn default() -> Self {
        Self::new()
    }
}

/// Errors raised while scanning a window.
#[derive(Debug)]
pub enum ScanError {
    /// COM could not be initialised on this thread.
    ComInit,
    /// The automation object could not be created.
    AutomationUnavailable,
    /// The window handle did not resolve to an element.
    NoElementForWindow,
    /// Enumerating the subtree failed.
    Enumerate,
}

impl std::fmt::Display for ScanError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let msg = match self {
            ScanError::ComInit => "failed to initialise COM",
            ScanError::AutomationUnavailable => "UI Automation is unavailable",
            ScanError::NoElementForWindow => "no UI Automation element for the window",
            ScanError::Enumerate => "failed to enumerate UI Automation elements",
        };
        f.write_str(msg)
    }
}

impl std::error::Error for ScanError {}

/// Convert a Win32 RECT into a Rect.
fn rect_from_win32(r: RECT) -> Rect {
    Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top)
}

/// DPI scale of the monitor hosting a window, falling back to the system DPI.
pub fn window_scale(hwnd: HWND) -> f64 {
    let dpi = unsafe {
        let d = GetDpiForWindow(hwnd);
        if d == 0 {
            GetDpiForSystem()
        } else {
            d
        }
    };
    if dpi == 0 {
        1.0
    } else {
        dpi as f64 / 96.0
    }
}

/// Bounding rectangle of a window in physical pixels.
pub fn window_rect(hwnd: HWND) -> Rect {
    use windows::Win32::Foundation::RECT as W32RECT;
    use windows::Win32::UI::WindowsAndMessaging::GetWindowRect;
    let mut r = W32RECT::default();
    unsafe {
        let _ = GetWindowRect(hwnd, &mut r);
    }
    rect_from_win32(r)
}

/// Collect candidate rectangles from a window subtree.
///
/// The scan runs on the calling thread. Results are filtered to elements that
/// intersect `region` (in physical pixels) and are converted to logical pixels
/// using `monitor`.
pub fn scan_window(
    hwnd: HWND,
    leaves_only: bool,
    min_area: i64,
    monitor: &Monitor,
    region: Option<Rect>,
) -> Result<Vec<Rect>, ScanError> {
    let _com = ComGuard::new();
    if !_com.initialised {
        return Err(ScanError::ComInit);
    }

    let automation: IUIAutomation = unsafe {
        CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER)
            .map_err(|_| ScanError::AutomationUnavailable)?
    };

    let root = unsafe {
        automation
            .ElementFromHandle(hwnd)
            .map_err(|_| ScanError::NoElementForWindow)?
    };

    // A true condition matches every element; we filter afterwards.
    let condition = unsafe {
        automation
            .CreateTrueCondition()
            .map_err(|_| ScanError::Enumerate)?
    };

    let array =
        unsafe { root.FindAll(TreeScope_Subtree, &condition) }.map_err(|_| ScanError::Enumerate)?;

    let len = unsafe { array.Length() }.unwrap_or(0);

    let mut physical = Vec::new();
    for i in 0..len {
        let Ok(element) = (unsafe { array.GetElement(i) }) else {
            continue;
        };
        if leaves_only && has_children(&element, &condition) {
            continue;
        }
        let Some(rect) = element_rect(&element) else {
            continue;
        };
        // Keep only elements overlapping the requested region. The region is in
        // physical pixels, matching what UIA reports.
        if let Some(region) = region {
            if !rect.intersects(&region) {
                continue;
            }
        }
        physical.push(rect);
    }

    // UIA reports physical pixels; the picker works in logical pixels.
    let logical: Vec<Rect> = physical
        .iter()
        .map(|r| monitor.physical_to_logical(r))
        .collect();

    Ok(sanitize(&logical, &monitor.bounds, min_area))
}

/// Every visible top-level window with a non-empty title.
///
/// Used when scanning a whole screen: the screen is covered by whatever windows
/// happen to be on it, so each is scanned and the results are merged.
pub fn top_level_windows() -> Vec<HWND> {
    use windows::core::BOOL;
    use windows::Win32::UI::WindowsAndMessaging::{
        EnumWindows, GetWindowTextLengthW, IsWindowVisible,
    };

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let out = unsafe { &mut *(lparam.0 as *mut Vec<HWND>) };
        let visible = unsafe { IsWindowVisible(hwnd) }.as_bool();
        let titled = unsafe { GetWindowTextLengthW(hwnd) } > 0;
        // Skip untitled shells (tooltips, IME windows) and hidden windows: they
        // contribute no reachable targets and cost a full automation walk each.
        if visible && titled {
            out.push(hwnd);
        }
        BOOL(1)
    }

    let mut out: Vec<HWND> = Vec::new();
    let ptr = &mut out as *mut Vec<HWND>;
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(ptr as isize));
    }
    out
}

/// Find the taskbar windows, one per display.
///
/// Windows uses `Shell_TrayWnd` for the primary taskbar and creates a
/// `Shell_SecondaryTrayWnd` for each additional display.
pub fn taskbar_windows() -> Vec<HWND> {
    use windows::core::BOOL;
    use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetClassNameW, IsWindowVisible};

    struct Search {
        found: Vec<HWND>,
    }

    unsafe extern "system" fn cb(hwnd: HWND, lparam: LPARAM) -> BOOL {
        let search = unsafe { &mut *(lparam.0 as *mut Search) };
        if unsafe { IsWindowVisible(hwnd) }.as_bool() {
            let mut buf = [0u16; 256];
            let n = unsafe { GetClassNameW(hwnd, &mut buf) };
            if n > 0 {
                let class = String::from_utf16_lossy(&buf[..n as usize]);
                if crate::targets::is_taskbar_class(&class) {
                    search.found.push(hwnd);
                }
            }
        }
        BOOL(1)
    }

    let mut search = Search { found: Vec::new() };
    let ptr = &mut search as *mut Search;
    unsafe {
        let _ = EnumWindows(Some(cb), LPARAM(ptr as isize));
    }
    search.found
}

/// An element has children when FindFirst under TreeScope_Children succeeds.
///
/// The result is an owned COM interface, so a successful call is itself the
/// answer; there is no null pointer to test.
fn has_children(
    element: &IUIAutomationElement,
    condition: &windows::Win32::UI::Accessibility::IUIAutomationCondition,
) -> bool {
    unsafe { element.FindFirst(TreeScope_Children, condition) }.is_ok()
}

fn element_rect(element: &IUIAutomationElement) -> Option<Rect> {
    let r = unsafe { element.CurrentBoundingRectangle() }.ok()?;
    Some(rect_from_win32(r))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_conversion_preserves_geometry() {
        let r = RECT {
            left: 10,
            top: 20,
            right: 110,
            bottom: 70,
        };
        assert_eq!(rect_from_win32(r), Rect::new(10, 20, 100, 50));
    }

    #[test]
    fn window_scale_is_positive() {
        // A null window falls back to the system DPI, which is always > 0.
        let s = window_scale(HWND(std::ptr::null_mut()));
        assert!(s > 0.0, "scale was {s}");
    }
}
