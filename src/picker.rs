//! Overlay session state machine.
//!
//! The picker owns the decision logic of one overlay session: which elements are
//! candidates, what happens when a letter is typed, and what the final target
//! is. It never touches Win32, so the whole interaction can be tested headlessly.

use crate::geometry::Rect;
use crate::keys::{vk_to_alpha, KeyState};
use crate::label::{self, Labelled};

/// What the caller should do after feeding an event to the picker.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PickOutcome {
    /// Nothing to do.
    Idle,
    /// Draw (or redraw) the overlay with the current candidates.
    Redraw,
    /// Hide the overlay; the user cancelled or the session finished without a
    /// target.
    Dismiss,
    /// Move the cursor to this point and hide the overlay.
    Jump { x: i32, y: i32 },
}

/// One overlay session.
#[derive(Debug, Clone, Default)]
pub struct Picker {
    candidates: Vec<Labelled<Rect>>,
    /// Original element count, retained for reporting.
    total_elements: usize,
    visible: bool,
}

impl Picker {
    /// Start a session over the given elements.
    ///
    /// Returns None when there is nothing to offer, so the caller does not
    /// show an empty overlay.
    pub fn start(elements: Vec<Rect>) -> Option<Self> {
        if elements.is_empty() {
            return None;
        }
        let labels = label::allocate_labels(elements.len()).ok()?;
        let total = elements.len();
        let candidates = labels
            .into_iter()
            .zip(elements)
            .map(|(label, value)| Labelled::new(label, value))
            .collect();
        Some(Self {
            candidates,
            total_elements: total,
            visible: true,
        })
    }

    pub fn is_visible(&self) -> bool {
        self.visible
    }

    pub fn total_elements(&self) -> usize {
        self.total_elements
    }

    pub fn candidates(&self) -> &[Labelled<Rect>] {
        &self.candidates
    }

    pub fn candidate_count(&self) -> usize {
        self.candidates.len()
    }

    /// Hide without picking anything.
    pub fn cancel(&mut self) -> PickOutcome {
        self.visible = false;
        PickOutcome::Dismiss
    }

    /// Handle a typed letter.
    ///
    /// Narrows the candidate set; when exactly one remains the cursor target is
    /// returned and the session ends.
    pub fn type_letter(&mut self, letter: char) -> PickOutcome {
        if !self.visible || self.candidates.is_empty() {
            return PickOutcome::Idle;
        }
        let narrowed = label::filter_by_prefix(&self.candidates, letter);
        if narrowed.is_empty() {
            // No candidate carries this prefix: ignore rather than dismiss, so a
            // stray key does not throw away the session.
            return PickOutcome::Idle;
        }
        self.candidates = narrowed;

        if self.candidates.len() == 1 {
            let rect = self.candidates[0].value;
            let (x, y) = rect.center();
            self.visible = false;
            return PickOutcome::Jump { x, y };
        }
        // Keep the remaining labels dense so the next key press is predictable.
        self.candidates = label::relabel(&self.candidates);
        PickOutcome::Redraw
    }

    /// Interpret a key press in the context of the current modifier state.
    pub fn on_key(&mut self, vk: u32, state: &KeyState) -> PickOutcome {
        // Ignore anything typed with a modifier held, so Ctrl+C during a session
        // is not swallowed as the letter "C".
        if state.is_down(0x11) || state.is_down(0xA2) || state.is_down(0xA3) {
            return PickOutcome::Idle;
        }
        match vk_to_alpha(vk) {
            Some(c) => self.type_letter(c),
            None => PickOutcome::Idle,
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

    #[test]
    fn start_with_no_elements_is_none() {
        assert!(Picker::start(vec![]).is_none());
    }

    #[test]
    fn start_labels_all_elements() {
        let p = Picker::start(rects(3)).unwrap();
        assert!(p.is_visible());
        assert_eq!(p.candidate_count(), 3);
        assert_eq!(p.candidates()[0].label, "A");
        assert_eq!(p.candidates()[2].label, "C");
    }

    #[test]
    fn typing_the_only_branch_jumps_to_that_element() {
        let mut p = Picker::start(rects(3)).unwrap();
        // 3 elements -> single-char labels, so one letter resolves immediately.
        let out = p.type_letter('B');
        // index 1 -> x = 1*100, width 50, centre 125
        assert_eq!(out, PickOutcome::Jump { x: 125, y: 20 });
        assert!(!p.is_visible());
    }

    #[test]
    fn unknown_letter_is_ignored_not_dismissed() {
        let mut p = Picker::start(rects(3)).unwrap();
        assert_eq!(p.type_letter('z'), PickOutcome::Idle);
        assert!(p.is_visible());
        assert_eq!(p.candidate_count(), 3);
    }

    #[test]
    fn narrows_progressively_when_more_than_26() {
        let mut p = Picker::start(rects(60)).unwrap();
        assert_eq!(p.candidates()[0].label, "AA");
        // "A" keeps the 26 elements starting with A...
        assert_eq!(p.type_letter('A'), PickOutcome::Redraw);
        assert_eq!(p.candidate_count(), 26);
        // ...then "B" selects the second of those.
        assert_eq!(p.type_letter('B'), PickOutcome::Jump { x: 125, y: 20 });
        assert!(!p.is_visible());
    }

    #[test]
    fn cancel_dismisses_and_stops_responding() {
        let mut p = Picker::start(rects(5)).unwrap();
        assert_eq!(p.cancel(), PickOutcome::Dismiss);
        assert!(!p.is_visible());
        assert_eq!(p.type_letter('A'), PickOutcome::Idle);
    }

    #[test]
    fn key_with_ctrl_held_is_ignored() {
        let mut p = Picker::start(rects(5)).unwrap();
        let mut state = KeyState::new();
        state.press(0xA2); // left control
        assert_eq!(p.on_key(0x41, &state), PickOutcome::Idle);
        assert!(p.is_visible());
    }

    #[test]
    fn letter_key_without_modifier_picks() {
        let mut p = Picker::start(rects(5)).unwrap();
        let state = KeyState::new();
        // 'C' is index 2 -> x = 2*100, width 50, centre 225
        assert_eq!(p.on_key(0x43, &state), PickOutcome::Jump { x: 225, y: 20 });
    }

    #[test]
    fn non_letter_key_is_ignored() {
        let mut p = Picker::start(rects(5)).unwrap();
        let state = KeyState::new();
        assert_eq!(p.on_key(0x0D, &state), PickOutcome::Idle); // Enter
        assert!(p.is_visible());
    }

    #[test]
    fn jump_lands_on_rect_centre() {
        let mut p = Picker::start(vec![Rect::new(10, 20, 100, 60)]).unwrap();
        assert_eq!(p.type_letter('A'), PickOutcome::Jump { x: 60, y: 50 });
    }

    #[test]
    fn every_element_in_a_full_grid_is_reachable_by_its_label() {
        for target in [0usize, 1, 25, 26, 675] {
            let mut p = Picker::start(rects(676)).unwrap();
            let label = p.candidates()[target].label.clone();
            let expected = p.candidates()[target].value;
            let mut last = PickOutcome::Idle;
            for c in label.chars() {
                last = p.type_letter(c);
            }
            assert_eq!(
                last,
                PickOutcome::Jump {
                    x: expected.center().0,
                    y: expected.center().1
                },
                "label {} for index {} did not resolve",
                label,
                target
            );
        }
    }
}
