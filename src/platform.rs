//! Win32 display enumeration.
//!
//! Kept separate from [`crate::monitors`] so the selection rules stay pure and
//! testable without a desktop.

use crate::geometry::Rect;
use crate::monitors::Display;
use windows::core::BOOL;
use windows::Win32::Foundation::{LPARAM, RECT, TRUE};
use windows::Win32::Graphics::Gdi::{
    EnumDisplayMonitors, GetMonitorInfoW, HDC, HMONITOR, MONITORINFOEXW, MONITOR_DEFAULTTONEAREST,
};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};

/// Collect all displays, ordered primary first and then by position.
pub fn enumerate() -> Vec<Display> {
    let mut found: Vec<(HMONITOR, MONITORINFOEXW)> = Vec::new();
    let ptr = &mut found as *mut Vec<(HMONITOR, MONITORINFOEXW)>;

    unsafe {
        let _ = EnumDisplayMonitors(None, None, Some(enum_proc), LPARAM(ptr as isize));
    }

    let mut displays: Vec<Display> = found
        .into_iter()
        .map(|(handle, info)| {
            let r = info.monitorInfo.rcMonitor;
            let name = String::from_utf16_lossy(&info.szDevice)
                .trim_end_matches('\0')
                .to_string();
            Display {
                index: 0,                                   // assigned below
                primary: info.monitorInfo.dwFlags & 1 != 0, // MONITORINFOF_PRIMARY
                bounds: Rect::new(r.left, r.top, r.right - r.left, r.bottom - r.top),
                scale: monitor_scale(handle),
                name,
            }
        })
        .collect();

    // Present the primary display first so numbering matches user expectation.
    displays.sort_by_key(|d| (!d.primary, d.bounds.x, d.bounds.y));
    for (i, d) in displays.iter_mut().enumerate() {
        d.index = i + 1;
    }
    displays
}

unsafe extern "system" fn enum_proc(
    handle: HMONITOR,
    _hdc: HDC,
    _rect: *mut RECT,
    data: LPARAM,
) -> BOOL {
    let list = unsafe { &mut *(data.0 as *mut Vec<(HMONITOR, MONITORINFOEXW)>) };
    let mut info = MONITORINFOEXW::default();
    info.monitorInfo.cbSize = std::mem::size_of::<MONITORINFOEXW>() as u32;
    if unsafe { GetMonitorInfoW(handle, &mut info.monitorInfo) }.as_bool() {
        list.push((handle, info));
    }
    TRUE
}

/// DPI scale of a monitor, defaulting to 1.0 when it cannot be read.
pub fn monitor_scale(handle: HMONITOR) -> f64 {
    let mut x = 0u32;
    let mut y = 0u32;
    match unsafe { GetDpiForMonitor(handle, MDT_EFFECTIVE_DPI, &mut x, &mut y) } {
        Ok(()) if x > 0 => x as f64 / 96.0,
        _ => 1.0,
    }
}

/// Scale of the monitor hosting a window.
pub fn window_monitor_scale(hwnd: windows::Win32::Foundation::HWND) -> f64 {
    let handle =
        unsafe { windows::Win32::Graphics::Gdi::MonitorFromWindow(hwnd, MONITOR_DEFAULTTONEAREST) };
    monitor_scale(handle)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enumeration_reports_at_least_one_display() {
        let d = enumerate();
        assert!(!d.is_empty(), "no displays detected");
        for (i, disp) in d.iter().enumerate() {
            assert_eq!(disp.index, i + 1, "indexing must be 1-based and dense");
            assert!(disp.bounds.width > 0 && disp.bounds.height > 0, "{disp:?}");
            assert!(disp.scale > 0.0, "{disp:?}");
        }
    }

    #[test]
    fn exactly_one_display_is_primary() {
        let d = enumerate();
        assert_eq!(d.iter().filter(|x| x.primary).count(), 1);
    }

    #[test]
    fn primary_is_first() {
        let d = enumerate();
        assert!(d[0].primary, "primary display should sort first: {d:?}");
    }
}
