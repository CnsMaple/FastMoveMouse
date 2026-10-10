//! Chord bindings and their JSON representation.
//!
//! The binding model is deliberately independent of Win32 so the defaults and
//! the (de)serialisation can be unit tested.

use crate::keys::vk;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// A named chord such as "Ctrl+Alt+Space".
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Chord {
    /// Virtual key codes that must all be held.
    pub keys: Vec<u32>,
}

impl Chord {
    pub fn new(keys: impl IntoIterator<Item = u32>) -> Self {
        Self {
            keys: keys.into_iter().collect(),
        }
    }

    /// Parse a human-readable chord such as `"Ctrl+Alt+Space"`.
    ///
    /// Recognised names are case-insensitive; a single character is read as a
    /// literal key. Unknown names are reported rather than silently dropped.
    pub fn parse(spec: &str) -> Result<Self, ConfigError> {
        let mut keys = Vec::new();
        for raw in spec.split('+') {
            let part = raw.trim();
            if part.is_empty() {
                return Err(ConfigError::BadChord(spec.to_string()));
            }
            let code = match part.to_ascii_lowercase().as_str() {
                "ctrl" | "control" => vk::LCONTROL,
                "alt" => vk::LALT,
                "shift" => vk::LSHIFT,
                "win" | "super" | "meta" => 0x5B,
                "esc" | "escape" => vk::ESCAPE,
                "space" => 0x20,
                "tab" => 0x09,
                "enter" | "return" => 0x0D,
                _ => {
                    let up = part.to_ascii_uppercase();
                    let mut chars = up.chars();
                    match (chars.next(), chars.next()) {
                        (Some(c), None) if c.is_ascii_alphanumeric() => c as u32,
                        (Some('F'), Some(_)) => part[1..]
                            .parse::<u32>()
                            .ok()
                            .filter(|n| (1..=24).contains(n))
                            .map(|n| 0x70 + n - 1)
                            .ok_or_else(|| ConfigError::BadChord(spec.to_string()))?,
                        _ => return Err(ConfigError::BadChord(spec.to_string())),
                    }
                }
            };
            keys.push(code);
        }
        if keys.is_empty() {
            return Err(ConfigError::BadChord(spec.to_string()));
        }
        Ok(Self { keys })
    }

    pub fn is_empty(&self) -> bool {
        self.keys.is_empty()
    }

    /// Number of keys in the chord.
    pub fn chord_len(&self) -> usize {
        self.keys.len()
    }
}

/// Raised while parsing a chord string.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ConfigError {
    BadChord(String),
    Io(String),
    Json(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ConfigError::BadChord(s) => write!(f, "unrecognised key chord: {s}"),
            ConfigError::Io(e) => write!(f, "cannot read config: {e}"),
            ConfigError::Json(e) => write!(f, "invalid config JSON: {e}"),
        }
    }
}

impl std::error::Error for ConfigError {}

/// Key bindings for the picker.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    /// Show or hide the overlay.
    pub toggle: Chord,
    /// Exit the process.
    pub quit: Chord,
    /// Minimum element area, in physical pixels, to be offered as a target.
    pub min_area: i64,
    /// Collect only leaf elements (those without children).
    pub leaves_only: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            // Ctrl+Alt+Space toggles, Ctrl+Alt+Q quits. Both were chosen to be
            // unlikely to collide with application shortcuts.
            toggle: Chord::new([vk::LCONTROL, vk::LALT, 0x20]),
            quit: Chord::new([vk::LCONTROL, vk::LALT, 0x51]),
            min_area: 16,
            leaves_only: true,
        }
    }
}

impl Config {
    /// Load from a JSON file, falling back to defaults when absent.
    ///
    /// A missing file is not an error; a malformed one is, so a typo does not
    /// silently run with unexpected bindings.
    pub fn load_or_default(path: &Path) -> Result<Self, ConfigError> {
        if !path.exists() {
            return Ok(Self::default());
        }
        let text = std::fs::read_to_string(path).map_err(|e| ConfigError::Io(e.to_string()))?;
        if text.trim().is_empty() {
            return Ok(Self::default());
        }
        serde_json::from_str(&text).map_err(|e| ConfigError::Json(e.to_string()))
    }

    /// Write the config as pretty JSON, creating parent directories.
    pub fn save(&self, path: &Path) -> Result<(), ConfigError> {
        if let Some(parent) = path.parent() {
            if !parent.as_os_str().is_empty() {
                std::fs::create_dir_all(parent).map_err(|e| ConfigError::Io(e.to_string()))?;
            }
        }
        let text =
            serde_json::to_string_pretty(self).map_err(|e| ConfigError::Json(e.to_string()))?;
        std::fs::write(path, text).map_err(|e| ConfigError::Io(e.to_string()))
    }

    /// The config path next to the executable.
    pub fn default_path() -> PathBuf {
        std::env::current_exe()
            .ok()
            .and_then(|p| p.parent().map(|d| d.join("fastmovemouse.json")))
            .unwrap_or_else(|| PathBuf::from("fastmovemouse.json"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_named_chord() {
        let c = Chord::parse("Ctrl+Alt+Space").unwrap();
        assert_eq!(c.keys, vec![vk::LCONTROL, vk::LALT, 0x20]);
    }

    #[test]
    fn parse_is_case_insensitive_and_trims() {
        let a = Chord::parse("ctrl + alt + space").unwrap();
        let b = Chord::parse("CTRL+ALT+SPACE").unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn parse_single_letter() {
        assert_eq!(Chord::parse("Q").unwrap().keys, vec![0x51]);
    }

    #[test]
    fn parse_function_keys() {
        assert_eq!(Chord::parse("F1").unwrap().keys, vec![0x70]);
        assert_eq!(Chord::parse("F12").unwrap().keys, vec![0x7B]);
        assert_eq!(Chord::parse("F24").unwrap().keys, vec![0x87]);
        assert!(Chord::parse("F25").is_err());
        assert!(Chord::parse("F0").is_err());
    }

    #[test]
    fn parse_rejects_junk() {
        assert!(Chord::parse("NotAKey").is_err());
        assert!(Chord::parse("").is_err());
        assert!(Chord::parse("Ctrl++").is_err());
        assert!(Chord::parse("Ctrl+ThisIsLong").is_err());
    }

    #[test]
    fn default_bindings_are_distinct_and_non_empty() {
        let c = Config::default();
        assert!(!c.toggle.is_empty());
        assert!(!c.quit.is_empty());
        assert_ne!(c.toggle, c.quit);
    }

    #[test]
    fn json_roundtrip_preserves_values() {
        let c = Config {
            toggle: Chord::parse("Ctrl+Shift+A").unwrap(),
            quit: Chord::parse("Esc").unwrap(),
            min_area: 42,
            leaves_only: false,
        };
        let text = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&text).unwrap();
        assert_eq!(c, back);
    }

    /// Partial config files must fill in defaults rather than fail, so adding a
    /// field in a later version does not break existing files.
    #[test]
    fn partial_json_fills_defaults() {
        let back: Config = serde_json::from_str(r#"{"min_area": 7}"#).unwrap();
        assert_eq!(back.min_area, 7);
        assert_eq!(back.toggle, Config::default().toggle);
        assert_eq!(back.leaves_only, Config::default().leaves_only);
    }

    #[test]
    fn empty_chord_serialises_and_reloads() {
        let c = Config {
            toggle: Chord::new([]),
            quit: Chord::parse("Esc").unwrap(),
            min_area: 0,
            leaves_only: true,
        };
        let text = serde_json::to_string(&c).unwrap();
        let back: Config = serde_json::from_str(&text).unwrap();
        assert!(back.toggle.is_empty());
    }

    #[test]
    fn missing_file_yields_defaults() {
        let p = std::env::temp_dir().join("fmm_definitely_missing_9f3a.json");
        let _ = std::fs::remove_file(&p);
        assert_eq!(Config::load_or_default(&p).unwrap(), Config::default());
    }

    #[test]
    fn save_and_load_roundtrip_on_disk() {
        let dir = std::env::temp_dir().join("fmm_test_cfg");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("roundtrip.json");
        let c = Config {
            toggle: Chord::parse("Ctrl+Alt+T").unwrap(),
            quit: Chord::parse("Ctrl+Alt+Q").unwrap(),
            min_area: 33,
            leaves_only: false,
        };
        c.save(&p).unwrap();
        assert_eq!(Config::load_or_default(&p).unwrap(), c);
        let _ = std::fs::remove_file(&p);
    }

    #[test]
    fn malformed_file_is_an_error_not_a_default() {
        let dir = std::env::temp_dir().join("fmm_test_cfg");
        let _ = std::fs::create_dir_all(&dir);
        let p = dir.join("broken.json");
        std::fs::write(&p, "{ this is not json").unwrap();
        assert!(matches!(
            Config::load_or_default(&p),
            Err(ConfigError::Json(_))
        ));
        let _ = std::fs::remove_file(&p);
    }
}
