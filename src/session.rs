//! Interactive session logic.
//!
//! The session turns a stream of key events into picker outcomes. It is kept
//! free of Win32 so the whole interaction can be unit tested: the caller only
//! reports which virtual keys have newly been pressed.

use crate::action::MouseAction;
use crate::geometry::Rect;
use crate::keys::{vk_to_alpha, KeyState};
use crate::label::Labelled;
use crate::picker::{PickOutcome, Picker};

/// Escape, which cancels a session.
pub const VK_ESCAPE: u32 = 0x1B;

/// What the caller should do after a poll.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Step {
    /// Nothing changed; keep waiting.
    Wait,
    /// The candidate set changed and the overlay must be repainted.
    Redraw,
    /// The user picked this point; perform the action, then stop.
    Act { x: i32, y: i32 },
    /// The user cancelled.
    Cancel,
}

/// One interactive picking session.
#[derive(Debug, Clone)]
pub struct Session {
    picker: Picker,
    /// The action to perform once a single candidate remains.
    pub action: MouseAction,
}

impl Session {
    /// Start over the given elements, or None when there is nothing to pick.
    pub fn start(elements: Vec<Rect>, action: MouseAction) -> Option<Self> {
        Some(Self {
            picker: Picker::start(elements)?,
            action,
        })
    }

    pub fn candidates(&self) -> &[Labelled<Rect>] {
        self.picker.candidates()
    }

    pub fn candidate_count(&self) -> usize {
        self.picker.candidate_count()
    }

    pub fn is_visible(&self) -> bool {
        self.picker.is_visible()
    }

    /// Feed one newly pressed key.
    ///
    /// Escape cancels. Letters narrow the candidates; when one remains the
    /// caller is told to act. Other keys are ignored so a stray press does not
    /// throw the session away.
    pub fn on_key(&mut self, vk: u32, state: &KeyState) -> Step {
        if vk == VK_ESCAPE {
            self.picker.cancel();
            return Step::Cancel;
        }

        // Ignore letters typed with a modifier held, so Ctrl+C is not read as
        // the letter C.
        if state.is_down(0x11) || state.is_down(0xA2) || state.is_down(0xA3) {
            return Step::Wait;
        }

        let Some(letter) = vk_to_alpha(vk) else {
            return Step::Wait;
        };

        match self.picker.type_letter(letter) {
            PickOutcome::Jump { x, y } => Step::Act { x, y },
            PickOutcome::Redraw => Step::Redraw,
            // An unmatched letter leaves the session alone.
            PickOutcome::Idle => Step::Wait,
            PickOutcome::Dismiss => Step::Cancel,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rects(n: usize) -> Vec<Rect> {
        (0..n)
            .map(|i| Rect::new(i as i32 * 100, 0, 50, 40))
            .collect()
    }

    fn session(n: usize) -> Session {
        Session::start(rects(n), MouseAction::Click).expect("non-empty")
    }

    #[test]
    fn empty_input_yields_no_session() {
        assert!(Session::start(vec![], MouseAction::Move).is_none());
    }

    #[test]
    fn a_letter_resolving_one_element_tells_the_caller_to_act() {
        let mut s = session(3);
        let state = KeyState::new();
        // index 2 -> x = 200, width 50, centre 225
        assert_eq!(s.on_key(0x43, &state), Step::Act { x: 225, y: 20 });
    }

    #[test]
    fn escape_cancels() {
        let mut s = session(5);
        assert_eq!(s.on_key(VK_ESCAPE, &KeyState::new()), Step::Cancel);
        assert!(!s.is_visible());
    }

    #[test]
    fn an_unmatched_letter_keeps_waiting() {
        let mut s = session(3);
        assert_eq!(s.on_key(0x5A, &KeyState::new()), Step::Wait); // Z matches nothing
        assert!(s.is_visible());
        assert_eq!(s.candidate_count(), 3);
    }

    #[test]
    fn a_non_letter_key_is_ignored() {
        let mut s = session(3);
        // Enter must not cancel or act.
        assert_eq!(s.on_key(0x0D, &KeyState::new()), Step::Wait);
        assert!(s.is_visible());
    }

    #[test]
    fn narrowing_over_many_elements_asks_for_a_redraw() {
        let mut s = session(60);
        // 60 elements use two-letter labels, so the first letter narrows.
        assert_eq!(s.on_key(0x41, &KeyState::new()), Step::Redraw);
        assert_eq!(s.candidate_count(), 26);
        // The second letter resolves.
        assert_eq!(
            s.on_key(0x42, &KeyState::new()),
            Step::Act { x: 125, y: 20 }
        );
    }

    #[test]
    fn letters_with_ctrl_held_are_ignored() {
        let mut s = session(3);
        let mut state = KeyState::new();
        state.press(0xA2); // left control
        assert_eq!(s.on_key(0x41, &state), Step::Wait);
        assert!(s.is_visible());
    }

    #[test]
    fn the_action_is_carried_through_untouched() {
        let s = Session::start(rects(2), MouseAction::RightClick).unwrap();
        assert_eq!(s.action, MouseAction::RightClick);
    }

    #[test]
    fn every_element_is_reachable_by_typing_its_label() {
        let all = rects(676);
        for target in [0usize, 25, 26, 675] {
            let mut s = Session::start(all.clone(), MouseAction::Move).unwrap();
            let label = s.candidates()[target].label.clone();
            let expected = all[target].center();
            let mut last = Step::Wait;
            for c in label.chars() {
                last = s.on_key(c as u32, &KeyState::new());
            }
            assert_eq!(
                last,
                Step::Act {
                    x: expected.0,
                    y: expected.1,
                },
                "label {label} for index {target} did not resolve"
            );
        }
    }
}
