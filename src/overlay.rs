//! Always-on-top layered overlay that draws one labelled box per candidate.
//!
//! Plain Win32: a popup window with WS_EX_LAYERED | WS_EX_TRANSPARENT |
//! WS_EX_TOPMOST | WS_EX_NOACTIVATE. Colour-key transparency keeps the window
//! non-interactive without per-pixel alpha bookkeeping, so the keyboard hook
//! stays the only input path.

use crate::geometry::Rect;
use crate::label::Labelled;
use windows::core::PCWSTR;
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, EndPaint, FillRect,
    InvalidateRect, RoundRect, SelectObject, SetBkMode, SetTextColor, UpdateWindow, DT_CENTER,
    DT_SINGLELINE, DT_VCENTER, PS_SOLID, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GetClientRect, LoadCursorW,
    PeekMessageW, RegisterClassExW, SetLayeredWindowAttributes, SetWindowPos, ShowWindow,
    TranslateMessage, CS_HREDRAW, CS_VREDRAW, HWND_TOPMOST, IDC_ARROW, LWA_COLORKEY, MSG,
    PM_REMOVE, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOWNOACTIVATE, WM_DESTROY, WM_PAINT,
    WNDCLASSEXW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST,
    WS_EX_TRANSPARENT, WS_POPUP,
};

/// Colour keyed out of the overlay: anything painted this colour is fully
/// transparent and click-through.
const TRANSPARENT_KEY: COLORREF = COLORREF(0x00FF00FF); // magenta

/// Fill colour of a label box (COLORREF is 0x00BBGGRR).
const LABEL_FILL: COLORREF = COLORREF(0x005BD5FF); // light amber
/// Border colour of a label box.
const LABEL_BORDER: COLORREF = COLORREF(0x00000000); // black
/// Text colour drawn inside a label box.
const LABEL_TEXT: COLORREF = COLORREF(0x00000000); // black

const CLASS_NAME: &str = "FastMoveMouseOverlay";

fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
}

/// A created overlay window.
pub struct Overlay {
    hwnd: HWND,
}

/// The overlay is created, shown and destroyed on the thread that owns the
/// message loop, so moving it between threads is never required.
unsafe impl Send for Overlay {}

impl Overlay {
    /// Create the hidden overlay window.
    pub fn new() -> Result<Self, String> {
        let hinstance = unsafe { GetModuleHandleW(None) }.map_err(|e| e.to_string())?;
        let class = wide(CLASS_NAME);
        let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|e| e.to_string())?;

        let wc = WNDCLASSEXW {
            cbSize: std::mem::size_of::<WNDCLASSEXW>() as u32,
            style: CS_HREDRAW | CS_VREDRAW,
            lpfnWndProc: Some(wnd_proc),
            hInstance: hinstance.into(),
            hCursor: cursor,
            lpszClassName: PCWSTR(class.as_ptr()),
            ..Default::default()
        };
        // Registering an already-registered class fails harmlessly; ignore it.
        unsafe {
            RegisterClassExW(&wc);
        }

        let title = wide("");
        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_NOACTIVATE
                    | WS_EX_TOOLWINDOW,
                PCWSTR(class.as_ptr()),
                PCWSTR(title.as_ptr()),
                WS_POPUP,
                0,
                0,
                100,
                100,
                None,
                None,
                Some(hinstance.into()),
                None,
            )
        }
        .map_err(|e| e.to_string())?;

        // Key out the background so the window is invisible except where boxes
        // are painted, and clicks pass straight through.
        unsafe {
            let _ = SetLayeredWindowAttributes(hwnd, TRANSPARENT_KEY, 0, LWA_COLORKEY);
        }

        Ok(Self { hwnd })
    }

    pub fn hwnd(&self) -> HWND {
        self.hwnd
    }

    /// Store the items to paint and mark the window dirty.
    ///
    /// `origin` is the screen position of the overlay window. Items are given in
    /// absolute screen coordinates and are converted to window-relative
    /// coordinates when painting, because GDI draws in client space.
    pub fn set_items(&self, origin: Rect, items: &[Labelled<Rect>]) {
        OVERLAY_STATE.with(|s| {
            let mut state = s.borrow_mut();
            state.origin = origin;
            state.items = items.to_vec();
        });
        unsafe {
            let _ = InvalidateRect(Some(self.hwnd), None, true);
        }
    }

    /// Position, size, paint and show the overlay over the given bounds.
    ///
    /// `bounds` is in absolute screen coordinates and may have a negative origin
    /// on a display positioned left of or above the primary one.
    pub fn show(&self, bounds: Rect, items: &[Labelled<Rect>]) {
        self.set_items(bounds, items);
        unsafe {
            let _ = SetWindowPos(
                self.hwnd,
                Some(HWND_TOPMOST),
                bounds.x,
                bounds.y,
                bounds.width.max(1),
                bounds.height.max(1),
                SWP_NOACTIVATE | SWP_SHOWWINDOW,
            );
            let _ = ShowWindow(self.hwnd, SW_SHOWNOACTIVATE);
            let _ = UpdateWindow(self.hwnd);
        }
    }

    pub fn hide(&self) {
        unsafe {
            let _ = ShowWindow(self.hwnd, SW_HIDE);
        }
    }

    /// Pump pending messages without blocking.
    ///
    /// Returns false once WM_QUIT has been seen.
    pub fn pump(&self) -> bool {
        let mut msg = MSG::default();
        loop {
            let more = unsafe { PeekMessageW(&mut msg, None, 0, 0, PM_REMOVE) }.as_bool();
            if !more {
                break;
            }
            unsafe {
                let _ = TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            if msg.message == 0x0012 {
                // WM_QUIT
                return false;
            }
        }
        true
    }
}

impl Drop for Overlay {
    fn drop(&mut self) {
        unsafe {
            let _ = DestroyWindow(self.hwnd);
        }
    }
}

/// What the overlay is currently painting.
#[derive(Debug, Clone, Default)]
pub struct OverlayState {
    /// Screen position of the overlay window, used to convert absolute
    /// coordinates into the window-relative space GDI draws in.
    pub origin: Rect,
    /// Items to paint, in absolute screen coordinates.
    pub items: Vec<Labelled<Rect>>,
}

impl OverlayState {
    /// An empty state, usable in a const context for the thread-local.
    pub const fn new() -> Self {
        Self {
            origin: Rect::new(0, 0, 0, 0),
            items: Vec::new(),
        }
    }

    /// Convert an absolute screen rectangle into window-relative coordinates.
    pub fn to_client(&self, rect: &Rect) -> Rect {
        rect.relative_to(&self.origin)
    }
}

thread_local! {
    /// Painting state for the overlay window owned by this thread.
    #[allow(clippy::missing_const_for_thread_local)]
    static OVERLAY_STATE: std::cell::RefCell<OverlayState> =
        const { std::cell::RefCell::new(OverlayState::new()) };
}

unsafe extern "system" fn wnd_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_PAINT => {
            paint(hwnd);
            LRESULT(0)
        }
        WM_DESTROY => LRESULT(0),
        _ => unsafe { DefWindowProcW(hwnd, msg, wparam, lparam) },
    }
}

/// Geometry of the box drawn for one item, centred on the element.
fn label_box(rect: &Rect, label: &str) -> Rect {
    let (cx, cy) = rect.center();
    let box_w = (label.chars().count() as i32 * 10 + 14).max(22);
    let box_h = 20;
    Rect::new(cx - box_w / 2, cy - box_h / 2, box_w, box_h)
}

fn paint(hwnd: HWND) {
    unsafe {
        let mut ps = windows::Win32::Graphics::Gdi::PAINTSTRUCT::default();
        let hdc = BeginPaint(hwnd, &mut ps);

        let mut client = RECT::default();
        let _ = GetClientRect(hwnd, &mut client);

        // Fill the background with the key colour so only boxes remain visible.
        let bg = CreateSolidBrush(TRANSPARENT_KEY);
        let _ = FillRect(hdc, &client, bg);
        let _ = DeleteObject(bg.into());

        let _ = SetBkMode(hdc, TRANSPARENT);
        let _ = SetTextColor(hdc, LABEL_TEXT);

        let fill = CreateSolidBrush(LABEL_FILL);
        let pen = CreatePen(PS_SOLID, 1, LABEL_BORDER);
        let old_brush = SelectObject(hdc, fill.into());
        let old_pen = SelectObject(hdc, pen.into());

        OVERLAY_STATE.with(|s| {
            let state = s.borrow();
            for item in state.items.iter() {
                if item.value.is_empty() {
                    continue;
                }
                // Item rectangles are absolute screen coordinates; GDI draws in
                // client space, so translate by the window origin. Without this
                // every label is offset by the window position, which is only
                // invisible when the window happens to sit at (0, 0).
                let client = state.to_client(&item.value);
                let b = label_box(&client, &item.label);
                let _ = RoundRect(hdc, b.x, b.y, b.right(), b.bottom(), 6, 6);

                let mut text: Vec<u16> = item.label.encode_utf16().collect();
                let mut tr = RECT {
                    left: b.x,
                    top: b.y,
                    right: b.right(),
                    bottom: b.bottom(),
                };
                DrawTextW(
                    hdc,
                    &mut text,
                    &mut tr,
                    DT_CENTER | DT_VCENTER | DT_SINGLELINE,
                );
            }
        });

        SelectObject(hdc, old_brush);
        SelectObject(hdc, old_pen);
        let _ = DeleteObject(fill.into());
        let _ = DeleteObject(pen.into());

        let _ = EndPaint(hwnd, &ps);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_box_is_centred_on_the_element() {
        let r = Rect::new(100, 100, 200, 100); // centre (200, 150)
        let b = label_box(&r, "A");
        assert_eq!(b.center(), (200, 150));
        assert_eq!(b.height, 20);
    }

    #[test]
    fn label_box_grows_with_label_length() {
        let r = Rect::new(0, 0, 100, 100);
        assert!(label_box(&r, "AAA").width > label_box(&r, "A").width);
    }

    #[test]
    fn label_box_has_a_minimum_width() {
        let b = label_box(&Rect::new(0, 0, 10, 10), "");
        assert!(b.width >= 22);
    }

    #[test]
    fn client_coordinates_are_relative_to_the_window_origin() {
        let state = OverlayState {
            origin: Rect::new(-1928, -7, 1936, 1066),
            items: Vec::new(),
        };
        // A window on a display left of the primary one has a negative origin;
        // an element at the window's top-left maps to (0, 0) in client space.
        assert_eq!(
            state.to_client(&Rect::new(-1928, -7, 100, 50)),
            Rect::new(0, 0, 100, 50)
        );
        assert_eq!(
            state.to_client(&Rect::new(-1828, 93, 100, 50)),
            Rect::new(100, 100, 100, 50)
        );
    }

    #[test]
    fn client_coordinates_with_a_zero_origin_are_unchanged() {
        let state = OverlayState {
            origin: Rect::new(0, 0, 1920, 1080),
            items: Vec::new(),
        };
        let r = Rect::new(100, 200, 50, 40);
        assert_eq!(state.to_client(&r), r);
    }

    #[test]
    fn default_state_is_empty_and_at_the_origin() {
        let state = OverlayState::new();
        assert!(state.items.is_empty());
        assert_eq!(state.origin, Rect::new(0, 0, 0, 0));
    }

    #[test]
    fn transparent_key_is_not_a_painted_colour() {
        assert_ne!(TRANSPARENT_KEY, LABEL_FILL);
        assert_ne!(TRANSPARENT_KEY, LABEL_BORDER);
    }
}
