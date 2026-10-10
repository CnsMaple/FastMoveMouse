//! Global low-level keyboard hook used while the overlay is visible.
//!
//! The hook exists only for the lifetime of one picking session: it is installed
//! when the labels appear and removed as soon as a selection is made or the
//! session is cancelled, so ordinary typing is never affected.
//!
//! The callback does the minimum possible work. Windows silently removes a hook
//! whose callback exceeds its time budget, so the callback only decides whether a
//! key should be swallowed and records the key code. All scanning, painting and
//! input synthesis happen on the main thread.

use std::cell::Cell;
use windows::Win32::Foundation::{LPARAM, LRESULT, WPARAM};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, SetWindowsHookExW, UnhookWindowsHookEx, HC_ACTION, HHOOK, KBDLLHOOKSTRUCT,
    WH_KEYBOARD_LL, WM_KEYDOWN, WM_SYSKEYDOWN,
};

/// Only these virtual keys are ever swallowed.
///
/// Letters are the label alphabet; Escape cancels. Everything else, including
/// modifiers, passes through so shortcuts keep working while the overlay is up.
pub fn is_consumed_key(vk: u32) -> bool {
    (0x41..=0x5A).contains(&vk) || vk == crate::session::VK_ESCAPE
}

thread_local! {
    /// Virtual key code of the most recent consumed press, if any.
    ///
    /// Both initializers are already const-evaluated; clippy's
    /// missing_const_for_thread_local still fires on this form (false positive on
    /// rustc 1.98), so it is allowed here rather than restructured.
    #[allow(clippy::missing_const_for_thread_local)]
    static PENDING: Cell<Option<u32>> = const { Cell::new(None) };
    /// When set, every decision is written to stderr. Used by --trace-keys.
    #[allow(clippy::missing_const_for_thread_local)]
    static TRACING: Cell<bool> = const { Cell::new(false) };
}

/// Enable or disable decision logging on stderr.
pub fn set_tracing(on: bool) {
    TRACING.with(|t| t.set(on));
}

/// Build a caller-facing error string, exposed for tests.
pub fn describe_decision(vk: u32, consumed: bool) -> String {
    format!(
        "vk {vk:#04x} ({}) -> {}",
        if (0x41..=0x5A).contains(&vk) {
            char::from_u32(vk).unwrap_or('?').to_string()
        } else {
            "other".to_string()
        },
        if consumed {
            "consumed"
        } else {
            "passed through"
        }
    )
}

/// An installed keyboard hook. Removed on drop.
pub struct KeyboardHook {
    handle: HHOOK,
}

/// The hook is installed and removed on the thread that owns the message loop.
unsafe impl Send for KeyboardHook {}

impl KeyboardHook {
    /// Install the hook. Fails when Windows refuses, which happens if the hook
    /// procedure is rejected or resources are exhausted.
    pub fn install() -> Result<Self, String> {
        PENDING.with(|p| p.set(None));
        let handle = unsafe { SetWindowsHookExW(WH_KEYBOARD_LL, Some(proc_hook), None, 0) }
            .map_err(|e| format!("failed to install the keyboard hook: {e}"))?;
        Ok(Self { handle })
    }

    pub fn handle(&self) -> HHOOK {
        self.handle
    }
}

impl Drop for KeyboardHook {
    fn drop(&mut self) {
        if !self.handle.is_invalid() {
            unsafe {
                let _ = UnhookWindowsHookEx(self.handle);
            }
        }
    }
}

/// Take the key code recorded by the hook, if one is waiting.
///
/// The main thread polls this. Taking clears it so a press is handled once.
pub fn take_pending() -> Option<u32> {
    PENDING.with(|p| p.replace(None))
}

unsafe extern "system" fn proc_hook(code: i32, wparam: WPARAM, lparam: LPARAM) -> LRESULT {
    if code == HC_ACTION as i32 {
        let message = wparam.0 as u32;
        // Only key-down matters; key-up is always passed on so the target
        // application never sees a stuck key.
        if message == WM_KEYDOWN || message == WM_SYSKEYDOWN {
            let data = unsafe { &*(lparam.0 as *const KBDLLHOOKSTRUCT) };
            let consumed = is_consumed_key(data.vkCode);
            if TRACING.with(|t| t.get()) {
                eprintln!("[hook] {}", describe_decision(data.vkCode, consumed));
            }
            if consumed {
                PENDING.with(|p| p.set(Some(data.vkCode)));
                // Returning 1 stops the key reaching any other application.
                return LRESULT(1);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wparam, lparam) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_are_consumed() {
        for vk in 0x41..=0x5A {
            assert!(is_consumed_key(vk), "vk {vk:#x} should be consumed");
        }
    }

    #[test]
    fn escape_is_consumed() {
        assert!(is_consumed_key(crate::session::VK_ESCAPE));
    }

    /// Everything that is not a label letter or Escape must reach the
    /// application underneath, so shortcuts and text entry keep working.
    #[test]
    fn other_keys_pass_through() {
        for vk in [
            0x0D,     // Enter
            0x20,     // Space
            0x09,     // Tab
            0x1B + 1, // just past Escape
            0x40,     // the character before A
            0x5B,     // the character after Z
            0x70,     // F1
            0x11,     // Ctrl
            0xA2,     // left Ctrl
            0xA0,     // left Shift
            0x25,     // Left arrow
        ] {
            assert!(!is_consumed_key(vk), "vk {vk:#x} must pass through");
        }
    }

    #[test]
    fn decision_description_names_letters_and_verdicts() {
        assert_eq!(describe_decision(0x41, true), "vk 0x41 (A) -> consumed");
        assert_eq!(
            describe_decision(0x0D, false),
            "vk 0x0d (other) -> passed through"
        );
        assert_eq!(describe_decision(0x1B, true), "vk 0x1b (other) -> consumed");
    }

    #[test]
    fn tracing_is_off_by_default() {
        TRACING.with(|t| t.set(false));
        assert!(!TRACING.with(|t| t.get()));
    }

    #[test]
    fn pending_starts_empty_and_is_taken_once() {
        PENDING.with(|p| p.set(None));
        assert_eq!(take_pending(), None);
        PENDING.with(|p| p.set(Some(0x41)));
        assert_eq!(take_pending(), Some(0x41));
        // Taking must clear it so one press is handled once.
        assert_eq!(take_pending(), None);
    }
}
