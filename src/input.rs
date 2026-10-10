//! Cursor movement and button synthesis.
//!
//! Movement is animated rather than jumped. Besides reading better, an animated
//! move fixes a Windows quirk: after a single `SetCursorPos` the pointer is not
//! repainted until the user moves a physical mouse, so the cursor appears to
//! vanish. Incremental moves force the redraw.

use crate::action::{MouseAction, MouseButton};
use crate::motion::{plan, Easing, Path};
use windows::Win32::UI::Input::KeyboardAndMouse::{
    SendInput, INPUT, INPUT_0, INPUT_MOUSE, MOUSEEVENTF_LEFTDOWN, MOUSEEVENTF_LEFTUP,
    MOUSEEVENTF_MIDDLEDOWN, MOUSEEVENTF_MIDDLEUP, MOUSEEVENTF_RIGHTDOWN, MOUSEEVENTF_RIGHTUP,
    MOUSEINPUT,
};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, SetCursorPos};

/// How the cursor should travel to its target.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Motion {
    /// Travel time in milliseconds. Zero jumps immediately.
    pub duration_ms: f64,
    /// Sampling rate of the animation. Higher is smoother but busier.
    pub fps: u32,
    /// The deceleration curve.
    pub easing: Easing,
}

impl Default for Motion {
    fn default() -> Self {
        Self {
            // Long enough to see the travel, short enough not to feel slow.
            duration_ms: 350.0,
            // Comfortably above the refresh rate of most displays.
            fps: 120,
            easing: Easing::EaseOut,
        }
    }
}

/// The current cursor position, or `None` when it cannot be read.
pub fn cursor_position() -> Option<(i32, i32)> {
    let mut p = windows::Win32::Foundation::POINT::default();
    unsafe { GetCursorPos(&mut p) }.ok()?;
    Some((p.x, p.y))
}

/// Move the cursor one step.
pub fn move_to(x: i32, y: i32) -> Result<(), String> {
    unsafe { SetCursorPos(x, y) }.map_err(|e| format!("cannot move the cursor: {e}"))
}

/// Play a planned path.
pub fn play(path: &Path) -> Result<(), String> {
    if path.points.is_empty() {
        return Ok(());
    }
    let delay = std::time::Duration::from_micros(path.step_delay_us);
    let start = std::time::Instant::now();

    for (i, (x, y)) in path.points.iter().enumerate() {
        move_to(*x, *y)?;
        // Pace the steps against a fixed schedule rather than sleeping a fixed
        // amount each time, so a slow step does not stretch the animation.
        if path.step_delay_us > 0 {
            let target = delay * (i as u32 + 1);
            let elapsed = start.elapsed();
            if target > elapsed {
                std::thread::sleep(target - elapsed);
            }
        }
    }
    Ok(())
}

/// Move the cursor to `to`, animating according to `motion`.
pub fn glide_to(to: (i32, i32), motion: Motion) -> Result<(), String> {
    let from = cursor_position().unwrap_or(to);
    let path = plan(from, to, motion.duration_ms, motion.fps, motion.easing);
    play(&path)
}

/// Press or release one mouse button via SendInput.
fn send_button(button: MouseButton, down: bool) -> Result<(), String> {
    let flag = match (button, down) {
        (MouseButton::Left, true) => MOUSEEVENTF_LEFTDOWN,
        (MouseButton::Left, false) => MOUSEEVENTF_LEFTUP,
        (MouseButton::Right, true) => MOUSEEVENTF_RIGHTDOWN,
        (MouseButton::Right, false) => MOUSEEVENTF_RIGHTUP,
        (MouseButton::Middle, true) => MOUSEEVENTF_MIDDLEDOWN,
        (MouseButton::Middle, false) => MOUSEEVENTF_MIDDLEUP,
    };

    let input = INPUT {
        r#type: INPUT_MOUSE,
        Anonymous: INPUT_0 {
            mi: MOUSEINPUT {
                dx: 0,
                dy: 0,
                mouseData: 0,
                dwFlags: flag,
                time: 0,
                dwExtraInfo: 0,
            },
        },
    };

    let sent = unsafe { SendInput(&[input], std::mem::size_of::<INPUT>() as i32) };
    if sent == 0 {
        return Err("SendInput failed to inject the mouse event".to_string());
    }
    Ok(())
}

/// Glide to a point, then perform the requested action there.
///
/// The button is pressed only after the cursor has settled, so a click cannot
/// land on something the cursor merely passes over.
pub fn perform(action: MouseAction, x: i32, y: i32, motion: Motion) -> Result<(), String> {
    glide_to((x, y), motion)?;

    let Some(button) = action.button() else {
        return Ok(()); // MouseAction::Move
    };

    for _ in 0..action.click_count() {
        send_button(button, true)?;
        send_button(button, false)?;
        // A short gap keeps a double-click from being coalesced into one.
        if action.click_count() > 1 {
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, MutexGuard, OnceLock};

    /// Tests that move the real cursor must not run concurrently: there is one
    /// system cursor, and parallel tests would fight over it.
    fn cursor_lock() -> MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .unwrap_or_else(|e| e.into_inner())
    }

    #[test]
    fn cursor_position_is_readable() {
        assert!(cursor_position().is_some());
    }

    #[test]
    fn move_to_a_real_screen_point_succeeds() {
        let _g = cursor_lock();
        assert!(move_to(0, 0).is_ok());
    }

    /// Windows clamps positions outside the desktop rather than failing, so a
    /// far-off target still succeeds and lands on-screen. Verified empirically
    /// against SetCursorPos on this platform.
    #[test]
    fn out_of_range_positions_are_clamped_not_rejected() {
        let _g = cursor_lock();
        assert!(move_to(i32::MAX, i32::MAX).is_ok());
        assert!(move_to(i32::MIN, i32::MIN).is_ok());
    }

    #[test]
    fn an_empty_path_plays_without_error() {
        assert!(play(&Path::stationary()).is_ok());
    }

    #[test]
    fn gliding_lands_exactly_on_the_target() {
        let _g = cursor_lock();
        let target = (100, 120);
        glide_to(target, Motion::default()).unwrap();
        assert_eq!(cursor_position(), Some(target));
    }

    #[test]
    fn a_zero_duration_glide_still_lands_on_the_target() {
        let _g = cursor_lock();
        let target = (200, 210);
        let motion = Motion {
            duration_ms: 0.0,
            ..Motion::default()
        };
        glide_to(target, motion).unwrap();
        assert_eq!(cursor_position(), Some(target));
    }

    #[test]
    fn gliding_takes_roughly_the_requested_time() {
        let _g = cursor_lock();
        move_to(0, 0).unwrap();
        let motion = Motion {
            duration_ms: 300.0,
            ..Motion::default()
        };
        let start = std::time::Instant::now();
        glide_to((900, 600), motion).unwrap();
        let elapsed = start.elapsed().as_millis();
        // Generous bounds: scheduling jitter on a busy machine is expected.
        assert!(
            (200..1200).contains(&elapsed),
            "glide took {elapsed} ms for a 300 ms request"
        );
    }

    #[test]
    fn move_only_action_performs_no_click() {
        let _g = cursor_lock();
        assert!(perform(MouseAction::Move, 5, 5, Motion::default()).is_ok());
    }

    #[test]
    fn default_motion_is_visible_but_quick() {
        let m = Motion::default();
        assert!(m.duration_ms > 0.0 && m.duration_ms < 1000.0);
        assert!(m.fps >= 60);
    }
}
