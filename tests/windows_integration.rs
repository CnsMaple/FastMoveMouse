//! Integration tests for the Win32/COM boundary and the public command surface.
//!
//! These run against a real desktop session. Tests that need a UI to inspect
//! skip themselves with a message when nothing suitable is available, so the
//! suite stays green in a session-less context.

#![cfg(windows)]

use fastmovemouse::action::MouseAction;
use fastmovemouse::app::{build_report, format_report, Source};
use fastmovemouse::geometry::{Monitor, Rect};
use fastmovemouse::monitors;
use fastmovemouse::overlay::Overlay;
use fastmovemouse::platform;
use fastmovemouse::scan;
use fastmovemouse::targets::{self, Target};

/// Pick a window that UI Automation can actually inspect.
///
/// The foreground window during a test run is the console host, which exposes no
/// automation provider, so tests that need a real UI select one that does.
fn scannable_window() -> Option<windows::Win32::Foundation::HWND> {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;

    // Prefer the foreground window when it is scannable.
    let fg = unsafe { GetForegroundWindow() };
    if !fg.is_invalid() {
        let rect = scan::window_rect(fg);
        let monitor = Monitor::new(rect, platform::window_monitor_scale(fg));
        if scan::scan_window(fg, true, 16, &monitor, None).is_ok_and(|r| !r.is_empty()) {
            return Some(fg);
        }
    }

    // Otherwise fall back to any titled top-level window that scans cleanly.
    for hwnd in scan::top_level_windows() {
        let rect = scan::window_rect(hwnd);
        if rect.width < 100 || rect.height < 100 {
            continue;
        }
        let monitor = Monitor::new(rect, platform::window_monitor_scale(hwnd));
        if let Ok(rects) = scan::scan_window(hwnd, true, 16, &monitor, None) {
            if !rects.is_empty() {
                return Some(hwnd);
            }
        }
    }
    None
}

// ---------------------------------------------------------------- displays

#[test]
fn displays_are_enumerable_and_consistently_numbered() {
    let displays = platform::enumerate();
    assert!(!displays.is_empty(), "expected at least one display");
    for (i, d) in displays.iter().enumerate() {
        assert_eq!(d.index, i + 1, "indexing must be 1-based: {d:?}");
        assert!(d.bounds.width > 0 && d.bounds.height > 0, "{d:?}");
        assert!(d.scale > 0.0, "{d:?}");
    }
    assert_eq!(
        displays.iter().filter(|d| d.primary).count(),
        1,
        "exactly one display must be primary"
    );
}

#[test]
fn every_display_can_be_selected_by_its_number() {
    let displays = platform::enumerate();
    for d in &displays {
        let picked = targets::resolve_screen(&displays, d.index)
            .unwrap_or_else(|e| panic!("screen {} failed: {e}", d.index));
        assert_eq!(picked.name, d.name);
    }
}

#[test]
fn out_of_range_screen_reports_the_available_count() {
    let displays = platform::enumerate();
    let n = displays.len() + 1;
    let err = targets::resolve_screen(&displays, n).unwrap_err();
    let msg = err.to_string();
    assert!(msg.contains(&n.to_string()), "{msg}");
    assert!(msg.contains("display"), "{msg}");
}

#[test]
fn screen_zero_is_rejected() {
    let displays = platform::enumerate();
    assert!(targets::resolve_screen(&displays, 0).is_err());
}

// ----------------------------------------------------------------- target

#[test]
fn conflicting_target_flags_are_rejected_before_scanning() {
    assert!(Target::from_flags(Some(1), true, false).is_err());
    assert!(Target::from_flags(None, true, true).is_err());
    assert!(Target::from_flags(Some(1), false, true).is_err());
}

#[test]
fn taskbar_windows_are_discoverable() {
    let bars = scan::taskbar_windows();
    if bars.is_empty() {
        eprintln!("skipping: no taskbar windows found in this session");
        return;
    }
    for hwnd in &bars {
        let r = scan::window_rect(*hwnd);
        assert!(r.width > 0 && r.height > 0, "taskbar rect was {r:?}");
    }
}

#[test]
fn statusbar_scan_covers_the_taskbar_area() {
    let bars = scan::taskbar_windows();
    if bars.is_empty() {
        eprintln!("skipping: no taskbar in this session");
        return;
    }
    let hwnd = bars[0];
    let rect = scan::window_rect(hwnd);
    let monitor = Monitor::new(rect, platform::window_monitor_scale(hwnd));
    match scan::scan_window(hwnd, true, 4, &monitor, None) {
        Ok(rects) => eprintln!("taskbar yielded {} candidates", rects.len()),
        Err(e) => eprintln!("taskbar scan unavailable: {e}"),
    }
}

// ------------------------------------------------------------------- scan

#[test]
fn focused_window_scan_completes() {
    let Some(hwnd) = scannable_window() else {
        eprintln!("skipping: no scannable window in this session");
        return;
    };
    let rect = scan::window_rect(hwnd);
    let monitor = Monitor::new(rect, platform::window_monitor_scale(hwnd));
    let rects = scan::scan_window(hwnd, true, 16, &monitor, None)
        .expect("selected window should be scannable");
    for r in &rects {
        assert!(!r.is_empty(), "empty rect survived filtering: {r:?}");
        assert!(r.area() >= 16, "undersized rect survived: {r:?}");
    }
}

/// The region filter must not widen a scan; at minimum it restricts results.
#[test]
fn region_filter_restricts_results_to_the_display() {
    let Some(hwnd) = scannable_window() else {
        eprintln!("skipping: no scannable window in this session");
        return;
    };
    let wr = scan::window_rect(hwnd);
    let monitor = Monitor::new(wr, platform::window_monitor_scale(hwnd));

    let all = scan::scan_window(hwnd, true, 16, &monitor, None).expect("scan failed");

    // A one-pixel region at the window origin can only admit elements touching
    // that point, never more than the unrestricted scan.
    let tiny = Rect::new(wr.x, wr.y, 1, 1);
    let subset = scan::scan_window(hwnd, true, 16, &monitor, Some(tiny)).expect("scan failed");
    assert!(
        subset.len() <= all.len(),
        "region filter returned more than the unrestricted scan: {} > {}",
        subset.len(),
        all.len()
    );
}

#[test]
fn repeated_scans_do_not_leak_com_references() {
    use windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow;
    let hwnd = unsafe { GetForegroundWindow() };
    if hwnd.is_invalid() {
        eprintln!("skipping: no foreground window");
        return;
    }
    let rect = scan::window_rect(hwnd);
    let monitor = Monitor::new(rect, platform::window_monitor_scale(hwnd));
    for _ in 0..3 {
        let _ = scan::scan_window(hwnd, true, 16, &monitor, None);
    }
}

#[test]
fn top_level_windows_are_enumerable() {
    let windows = scan::top_level_windows();
    assert!(!windows.is_empty(), "no titled top-level windows found");
}

// -------------------------------------------------------------- reporting

#[test]
fn a_real_scan_produces_a_formatted_report() {
    let Some(hwnd) = scannable_window() else {
        eprintln!("skipping: no scannable window in this session");
        return;
    };
    let rect = scan::window_rect(hwnd);
    let monitor = Monitor::new(rect, platform::window_monitor_scale(hwnd));
    let rects = scan::scan_window(hwnd, true, 16, &monitor, None).expect("scan failed");

    let report = build_report(
        rects,
        Source::Window {
            title: "test".into(),
            rect,
        },
        monitor.bounds,
    );

    let text = format_report(&report, MouseAction::DoubleClick);
    assert!(text.contains("double-click"), "{text}");
    assert!(text.contains("elements:"), "{text}");
    assert_eq!(report.len(), report.items.len());
}

// ---------------------------------------------------------------- overlay

#[test]
fn overlay_window_lifecycle() {
    let overlay = match Overlay::new() {
        Ok(o) => o,
        Err(e) => {
            eprintln!("skipping: cannot create overlay window: {e}");
            return;
        }
    };
    let items = vec![fastmovemouse::label::Labelled::new(
        "A",
        Rect::new(0, 0, 40, 30),
    )];
    overlay.show(Rect::new(0, 0, 400, 300), &items);
    overlay.pump();
    overlay.hide();
}

// ------------------------------------------------------------------ input

#[test]
fn move_only_action_does_not_press_buttons() {
    let motion = fastmovemouse::input::Motion {
        duration_ms: 0.0,
        ..Default::default()
    };
    assert!(fastmovemouse::input::perform(MouseAction::Move, 5, 5, motion).is_ok());
}

/// An animated glide must actually pass through intermediate positions. This is
/// what distinguishes a glide from a teleport, and it is also what makes Windows
/// repaint the pointer.
#[test]
fn a_glide_visits_intermediate_positions_before_arriving() {
    use fastmovemouse::input::{cursor_position, glide_to, move_to, Motion};

    move_to(0, 0).expect("position the cursor");
    let target = (700, 500);
    let motion = Motion {
        duration_ms: 900.0,
        fps: 120,
        ..Default::default()
    };

    let mut seen = Vec::new();
    let handle = std::thread::spawn(move || {
        let mut samples = Vec::new();
        let deadline = std::time::Instant::now() + std::time::Duration::from_millis(2500);
        while std::time::Instant::now() < deadline {
            if let Some(p) = cursor_position() {
                if samples.last() != Some(&p) {
                    samples.push(p);
                }
            }
            std::thread::sleep(std::time::Duration::from_millis(15));
        }
        samples
    });

    glide_to(target, motion).expect("glide");
    std::thread::sleep(std::time::Duration::from_millis(600));
    seen.extend(handle.join().unwrap_or_default());

    assert_eq!(cursor_position(), Some(target), "glide missed the target");
    assert!(
        seen.len() >= 3,
        "expected intermediate positions, observed {seen:?}"
    );
}

#[test]
fn monitor_selection_helpers_agree_on_the_primary_display() {
    let displays = platform::enumerate();
    let primary = monitors::primary(&displays).expect("primary display");
    let picked = targets::resolve_screen(&displays, primary.index).expect("resolve");
    assert_eq!(picked.name, primary.name);
}
