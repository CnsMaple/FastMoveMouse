//! Mouse behaviour performed once a label has been resolved.

use serde::{Deserialize, Serialize};
use std::str::FromStr;

/// What to do at the target point after the picker resolves.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum MouseAction {
    /// Move the cursor and stop.
    #[default]
    Move,
    /// Move, then press and release the left button.
    Click,
    /// Move, then double-click the left button.
    DoubleClick,
    /// Move, then press and release the right button.
    RightClick,
    /// Move, then press and release the middle button.
    MiddleClick,
}

impl MouseAction {
    /// Every action, in a stable order, for help text and tests.
    pub const ALL: [MouseAction; 5] = [
        MouseAction::Move,
        MouseAction::Click,
        MouseAction::DoubleClick,
        MouseAction::RightClick,
        MouseAction::MiddleClick,
    ];

    /// Whether this action presses a mouse button.
    pub fn is_click(self) -> bool {
        !matches!(self, MouseAction::Move)
    }

    /// The button this action uses, if any.
    pub fn button(self) -> Option<MouseButton> {
        match self {
            MouseAction::Move => None,
            MouseAction::Click | MouseAction::DoubleClick => Some(MouseButton::Left),
            MouseAction::RightClick => Some(MouseButton::Right),
            MouseAction::MiddleClick => Some(MouseButton::Middle),
        }
    }

    /// Number of press/release cycles.
    pub fn click_count(self) -> u32 {
        match self {
            MouseAction::DoubleClick => 2,
            MouseAction::Move => 0,
            _ => 1,
        }
    }
}

/// A mouse button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MouseButton {
    Left,
    Right,
    Middle,
}

/// Raised when an action name is not recognised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseActionError(pub String);

impl std::fmt::Display for ParseActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "unknown action {:?}; expected one of: move, click, double-click, right-click, middle-click",
            self.0
        )
    }
}

impl std::error::Error for ParseActionError {}

impl FromStr for MouseAction {
    type Err = ParseActionError;

    /// Parse an action name, ignoring case and treating underscores as dashes.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let norm = s.trim().to_ascii_lowercase().replace('_', "-");
        match norm.as_str() {
            "move" | "none" | "hover" => Ok(MouseAction::Move),
            "click" | "left" | "left-click" | "single" => Ok(MouseAction::Click),
            "double-click" | "doubleclick" | "double" | "dblclick" => Ok(MouseAction::DoubleClick),
            "right-click" | "rightclick" | "right" | "context" => Ok(MouseAction::RightClick),
            "middle-click" | "middleclick" | "middle" => Ok(MouseAction::MiddleClick),
            _ => Err(ParseActionError(s.to_string())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_canonical_names() {
        assert_eq!("move".parse(), Ok(MouseAction::Move));
        assert_eq!("click".parse(), Ok(MouseAction::Click));
        assert_eq!("double-click".parse(), Ok(MouseAction::DoubleClick));
        assert_eq!("right-click".parse(), Ok(MouseAction::RightClick));
        assert_eq!("middle-click".parse(), Ok(MouseAction::MiddleClick));
    }

    #[test]
    fn parsing_is_case_and_separator_insensitive() {
        assert_eq!("Double_Click".parse(), Ok(MouseAction::DoubleClick));
        assert_eq!("  RIGHT-CLICK  ".parse(), Ok(MouseAction::RightClick));
        assert_eq!("Move".parse(), Ok(MouseAction::Move));
    }

    #[test]
    fn aliases_resolve() {
        assert_eq!("dblclick".parse(), Ok(MouseAction::DoubleClick));
        assert_eq!("left".parse(), Ok(MouseAction::Click));
        assert_eq!("context".parse(), Ok(MouseAction::RightClick));
        assert_eq!("hover".parse(), Ok(MouseAction::Move));
    }

    #[test]
    fn unknown_action_is_rejected() {
        assert!("teleport".parse::<MouseAction>().is_err());
        assert!("".parse::<MouseAction>().is_err());
    }

    #[test]
    fn move_performs_no_click() {
        assert!(!MouseAction::Move.is_click());
        assert_eq!(MouseAction::Move.click_count(), 0);
        assert_eq!(MouseAction::Move.button(), None);
    }

    #[test]
    fn click_counts_and_buttons() {
        assert_eq!(MouseAction::Click.click_count(), 1);
        assert_eq!(MouseAction::Click.button(), Some(MouseButton::Left));
        assert_eq!(MouseAction::DoubleClick.click_count(), 2);
        assert_eq!(MouseAction::DoubleClick.button(), Some(MouseButton::Left));
        assert_eq!(MouseAction::RightClick.click_count(), 1);
        assert_eq!(MouseAction::RightClick.button(), Some(MouseButton::Right));
        assert_eq!(MouseAction::MiddleClick.button(), Some(MouseButton::Middle));
    }

    #[test]
    fn every_action_round_trips_through_its_name() {
        for a in MouseAction::ALL {
            let name = match a {
                MouseAction::Move => "move",
                MouseAction::Click => "click",
                MouseAction::DoubleClick => "double-click",
                MouseAction::RightClick => "right-click",
                MouseAction::MiddleClick => "middle-click",
            };
            assert_eq!(name.parse(), Ok(a), "{name} did not round-trip");
        }
    }

    #[test]
    fn json_round_trip_uses_kebab_case() {
        let text = serde_json::to_string(&MouseAction::DoubleClick).unwrap();
        assert_eq!(text, "\"double-click\"");
        assert_eq!(
            serde_json::from_str::<MouseAction>("\"right-click\"").unwrap(),
            MouseAction::RightClick
        );
    }

    #[test]
    fn default_action_is_move() {
        assert_eq!(MouseAction::default(), MouseAction::Move);
    }
}
