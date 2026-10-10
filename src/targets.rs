//! Resolving what to scan: a numbered screen, the focused window, or the
//! taskbars across all displays.
//!
//! The selection rules are pure so they can be tested without a desktop.

use crate::monitors::{self, Display, ScreenError};

/// What the scan should cover.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub enum Target {
    /// Every visible element on one screen, identified by its 1-based number.
    Screen(usize),
    /// The elements of the window that currently has focus.
    #[default]
    Focus,
    /// The taskbar (start button, tray, clock) on every display.
    StatusBar,
}

impl Target {
    /// Build a target from the command-line flags.
    ///
    /// `--screen` and `--focus` are mutually exclusive; `--statusbar` likewise.
    /// An explicit selector always wins over the positional default.
    pub fn from_flags(
        screen: Option<usize>,
        focus: bool,
        statusbar: bool,
    ) -> Result<Self, TargetError> {
        let chosen: Vec<Target> = [
            screen.map(Target::Screen),
            focus.then_some(Target::Focus),
            statusbar.then_some(Target::StatusBar),
        ]
        .into_iter()
        .flatten()
        .collect();

        match chosen.len() {
            0 => Ok(Target::default()),
            1 => Ok(chosen.into_iter().next().expect("length checked")),
            _ => Err(TargetError::Conflicting),
        }
    }

    /// Human-readable description used in output.
    pub fn describe(&self) -> String {
        match self {
            Target::Screen(n) => format!("screen {n}"),
            Target::Focus => "the focused window".to_string(),
            Target::StatusBar => "all taskbars".to_string(),
        }
    }
}

/// Raised when the requested target cannot be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TargetError {
    /// More than one selector was supplied.
    Conflicting,
    /// A screen number could not be matched to a display.
    Screen(ScreenError),
    /// A screen number of 0 was supplied; numbering starts at 1.
    ScreenZero,
}

impl std::fmt::Display for TargetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TargetError::Conflicting => write!(
                f,
                "--screen, --focus and --statusbar are mutually exclusive; pick one"
            ),
            TargetError::Screen(e) => write!(f, "{e}"),
            TargetError::ScreenZero => {
                write!(
                    f,
                    "screen numbers start at 1; use 1 for the primary display"
                )
            }
        }
    }
}

impl std::error::Error for TargetError {}

impl From<ScreenError> for TargetError {
    fn from(e: ScreenError) -> Self {
        TargetError::Screen(e)
    }
}

/// Validate a screen number and resolve it against the detected displays.
pub fn resolve_screen(displays: &[Display], number: usize) -> Result<Display, TargetError> {
    if number == 0 {
        return Err(TargetError::ScreenZero);
    }
    Ok(monitors::select(displays, number)?.clone())
}

/// Window classes that make up the taskbar across displays.
///
/// `Shell_TrayWnd` hosts the primary taskbar; Windows creates one
/// `Shell_SecondaryTrayWnd` per additional display.
pub const TASKBAR_CLASSES: [&str; 2] = ["Shell_TrayWnd", "Shell_SecondaryTrayWnd"];

/// Whether a window class name identifies a taskbar.
pub fn is_taskbar_class(class: &str) -> bool {
    TASKBAR_CLASSES.contains(&class)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::geometry::Rect;

    fn displays() -> Vec<Display> {
        vec![
            Display {
                index: 1,
                primary: true,
                bounds: Rect::new(0, 0, 1920, 1080),
                scale: 1.0,
                name: "DISPLAY1".into(),
            },
            Display {
                index: 2,
                primary: false,
                bounds: Rect::new(1920, 0, 2560, 1440),
                scale: 1.5,
                name: "DISPLAY2".into(),
            },
        ]
    }

    #[test]
    fn no_flags_defaults_to_focus() {
        assert_eq!(
            Target::from_flags(None, false, false).unwrap(),
            Target::Focus
        );
    }

    #[test]
    fn single_selectors_resolve() {
        assert_eq!(
            Target::from_flags(Some(2), false, false).unwrap(),
            Target::Screen(2)
        );
        assert_eq!(
            Target::from_flags(None, true, false).unwrap(),
            Target::Focus
        );
        assert_eq!(
            Target::from_flags(None, false, true).unwrap(),
            Target::StatusBar
        );
    }

    #[test]
    fn conflicting_selectors_are_rejected() {
        assert_eq!(
            Target::from_flags(Some(1), true, false),
            Err(TargetError::Conflicting)
        );
        assert_eq!(
            Target::from_flags(Some(1), false, true),
            Err(TargetError::Conflicting)
        );
        assert_eq!(
            Target::from_flags(None, true, true),
            Err(TargetError::Conflicting)
        );
        assert_eq!(
            Target::from_flags(Some(1), true, true),
            Err(TargetError::Conflicting)
        );
    }

    #[test]
    fn screen_zero_is_a_clear_error() {
        assert_eq!(resolve_screen(&displays(), 0), Err(TargetError::ScreenZero));
        assert!(TargetError::ScreenZero.to_string().contains("start at 1"));
    }

    #[test]
    fn valid_screen_numbers_resolve_to_the_display() {
        assert_eq!(resolve_screen(&displays(), 1).unwrap().name, "DISPLAY1");
        assert_eq!(resolve_screen(&displays(), 2).unwrap().name, "DISPLAY2");
    }

    #[test]
    fn unknown_screen_number_names_the_available_range() {
        let e = resolve_screen(&displays(), 5).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("screen 5"), "{msg}");
        assert!(msg.contains("2 display"), "{msg}");
    }

    #[test]
    fn taskbar_classes_are_recognised() {
        assert!(is_taskbar_class("Shell_TrayWnd"));
        assert!(is_taskbar_class("Shell_SecondaryTrayWnd"));
        assert!(!is_taskbar_class("Notepad"));
        assert!(!is_taskbar_class(""));
    }

    #[test]
    fn describe_is_readable() {
        assert_eq!(Target::Screen(3).describe(), "screen 3");
        assert_eq!(Target::Focus.describe(), "the focused window");
        assert_eq!(Target::StatusBar.describe(), "all taskbars");
    }
}
