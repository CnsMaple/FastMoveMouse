//! Keyboard-driven UI element picker for Windows.
//!
//! The crate is split so that everything decidable without Windows lives in
//! pure modules and is unit tested:
//!
//! * [`label`]    - label allocation and prefix narrowing
//! * [`geometry`] - rectangle maths and DPI transforms
//! * [`monitors`] - display model and screen selection
//! * [`targets`]  - what to scan: screen, focus or taskbars
//! * [`action`]   - mouse behaviour after a label matches
//! * [`app`]      - scan report construction and formatting
//! * [`keys`]     - virtual-key tracking and chord matching
//! * [`config`]   - bindings and JSON persistence
//! * [`picker`]   - overlay session state machine
//!
//! The remaining modules ([`scan`], [`overlay`], [`platform`], [`input`])
//! form the Win32/COM boundary and are exercised by integration tests on a real
//! desktop.

pub mod action;
pub mod app;
pub mod config;
pub mod geometry;
pub mod keys;
pub mod label;
pub mod monitors;
pub mod motion;
pub mod picker;
pub mod session;
pub mod startup;
pub mod targets;

#[cfg(windows)]
pub mod hook;
#[cfg(windows)]
pub mod input;
#[cfg(windows)]
pub mod overlay;
#[cfg(windows)]
pub mod platform;
#[cfg(windows)]
pub mod scan;

pub use action::MouseAction;
pub use app::{format_report, ScanReport};
pub use config::{Chord, Config, ConfigError};
pub use geometry::{Monitor, Rect};
pub use monitors::{Display, ScreenError};
pub use targets::{Target, TargetError};
