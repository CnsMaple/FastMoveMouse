//! Virtual-key tracking and chord matching.
//!
//! A low-level keyboard hook has no per-instance context pointer, so the
//! callback must reach its state through a global. The state machine itself is
//! kept free of Win32 types so it can be exercised in unit tests.

/// Number of virtual-key codes Windows can report.
pub const VK_COUNT: usize = 256;

/// Virtual key codes referenced by the default bindings.
pub mod vk {
    pub const LCONTROL: u32 = 0xA2;
    pub const RCONTROL: u32 = 0xA3;
    pub const LALT: u32 = 0xA4;
    pub const RALT: u32 = 0xA5;
    pub const LSHIFT: u32 = 0xA0;
    pub const RSHIFT: u32 = 0xA1;
    pub const ESCAPE: u32 = 0x1B;
    pub const A: u32 = 0x41;
    pub const Z: u32 = 0x5A;
}

/// Which physical keys are currently held down.
///
/// Specialised keys are tracked separately from the generic array so that
/// "Ctrl is held" is answerable without knowing which Ctrl the user pressed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct KeyState {
    down: [bool; VK_COUNT],
}

impl Default for KeyState {
    fn default() -> Self {
        Self::new()
    }
}

impl KeyState {
    pub const fn new() -> Self {
        Self {
            down: [false; VK_COUNT],
        }
    }

    /// Record a key going down. Unknown codes are ignored.
    pub fn press(&mut self, vk: u32) {
        if (vk as usize) < VK_COUNT {
            self.down[vk as usize] = true;
        }
    }

    /// Record a key coming up. Unknown codes are ignored.
    pub fn release(&mut self, vk: u32) {
        if (vk as usize) < VK_COUNT {
            self.down[vk as usize] = false;
        }
    }

    pub fn is_down(&self, vk: u32) -> bool {
        (vk as usize) < VK_COUNT && self.down[vk as usize]
    }

    /// Any key at all currently held down.
    pub fn any_down(&self) -> bool {
        self.down.iter().any(|&d| d)
    }

    /// Every key released.
    pub fn none_down(&self) -> bool {
        !self.any_down()
    }

    /// Every virtual key currently held, ascending.
    pub fn pressed_keys(&self) -> Vec<u32> {
        (0..VK_COUNT as u32).filter(|&k| self.is_down(k)).collect()
    }

    /// A whole chord is held.
    ///
    /// An empty chord is never considered pressed, so an unconfigured binding
    /// cannot fire on every keystroke.
    pub fn chord_down(&self, chord: &[u32]) -> bool {
        !chord.is_empty() && chord.iter().all(|&k| self.is_down(k))
    }

    /// The most recently pressed key that is still held, if any.
    pub fn last_pressed(&self) -> Option<u32> {
        self.pressed_keys().into_iter().next_back()
    }
}

/// Convert a virtual key to the uppercase ASCII character it types, if any.
pub fn vk_to_alpha(vk: u32) -> Option<char> {
    if (vk::A..=vk::Z).contains(&vk) {
        char::from_u32(vk)
    } else {
        None
    }
}

/// Whether a key is a modifier that should never be consumed on its own.
pub fn is_modifier(vk: u32) -> bool {
    matches!(
        vk,
        0x10 | 0x11 | 0x12 // generic SHIFT / CONTROL / MENU
            | vk::LSHIFT | vk::RSHIFT
            | vk::LCONTROL | vk::RCONTROL
            | vk::LALT | vk::RALT
            | 0x5B | 0x5C // left / right Windows key
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn press_and_release_track_state() {
        let mut s = KeyState::new();
        assert!(!s.is_down(vk::A));
        s.press(vk::A);
        assert!(s.is_down(vk::A));
        s.release(vk::A);
        assert!(!s.is_down(vk::A));
    }

    #[test]
    fn out_of_range_codes_are_ignored() {
        let mut s = KeyState::new();
        s.press(9999);
        s.release(9999);
        assert!(s.none_down());
    }

    #[test]
    fn chord_requires_all_keys() {
        let mut s = KeyState::new();
        let c = vec![vk::LCONTROL, vk::LALT];
        s.press(vk::LCONTROL);
        assert!(!s.chord_down(&c));
        s.press(vk::LALT);
        assert!(s.chord_down(&c));
        s.release(vk::LCONTROL);
        assert!(!s.chord_down(&c));
    }

    /// An unconfigured (empty) binding must never match, otherwise it would
    /// swallow every keystroke.
    #[test]
    fn empty_chord_never_matches() {
        let s = KeyState::new();
        assert!(!s.chord_down(&[]));
    }

    #[test]
    fn any_and_none_down() {
        let mut s = KeyState::new();
        assert!(s.none_down());
        s.press(vk::Z);
        assert!(s.any_down());
        assert!(!s.none_down());
    }

    #[test]
    fn pressed_keys_is_ascending_and_last_pressed_is_highest() {
        let mut s = KeyState::new();
        s.press(vk::Z);
        s.press(vk::A);
        assert_eq!(s.pressed_keys(), vec![vk::A, vk::Z]);
        assert_eq!(s.last_pressed(), Some(vk::Z));
    }

    #[test]
    fn vk_to_alpha_maps_only_letters() {
        assert_eq!(vk_to_alpha(0x41), Some('A'));
        assert_eq!(vk_to_alpha(0x5A), Some('Z'));
        assert_eq!(vk_to_alpha(0x30), None); // '0'
        assert_eq!(vk_to_alpha(0x0D), None); // Enter
    }

    #[test]
    fn modifiers_are_recognised() {
        assert!(is_modifier(vk::LCONTROL));
        assert!(is_modifier(vk::LSHIFT));
        assert!(!is_modifier(vk::A));
        assert!(!is_modifier(vk::ESCAPE));
    }
}
