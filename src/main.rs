//! Command-line entry point.
//!
//! The tool is one-shot: it resolves a target, scans it, prints the labelled
//! elements, and exits. No global keyboard hook is installed.

use clap::Parser;
use fastmovemouse::action::MouseAction;
use fastmovemouse::app::{build_report, format_report, Source};
use fastmovemouse::config::Config;
use fastmovemouse::geometry::Rect;
use fastmovemouse::targets::Target;
use std::path::PathBuf;
use std::str::FromStr;

/// Keyboard-driven UI element picker.
///
/// Scans a screen, the focused window, or the taskbars, and prints a letter for
/// every element it can reach. Choose which elements to list with --screen,
/// --focus or --statusbar, and what the mouse should do with --action.
#[derive(Parser, Debug)]
#[command(name = "fastmovemouse", version, about)]
struct Cli {
    /// Scan this screen, numbered from 1.
    #[arg(long, value_name = "N")]
    screen: Option<usize>,

    /// Scan the window that currently has focus (the default).
    #[arg(long)]
    focus: bool,

    /// Scan the taskbar on every display.
    #[arg(long)]
    statusbar: bool,

    /// What to do when a label matches: move, click, double-click,
    /// right-click or middle-click.
    #[arg(long, value_name = "ACTION", default_value = "move")]
    action: String,

    /// Minimum element area in pixels to offer as a target.
    #[arg(long, value_name = "PX", default_value_t = 16)]
    min_area: i64,

    /// Also offer elements that have children.
    #[arg(long)]
    all_elements: bool,

    /// List the available screens and exit.
    #[arg(long)]
    list_screens: bool,

    /// Read settings from this JSON file.
    #[arg(long, value_name = "PATH")]
    config: Option<PathBuf>,

    /// Write the default settings to a file and exit.
    #[arg(long, value_name = "PATH")]
    write_default_config: Option<PathBuf>,

    /// Draw the labels on screen and wait for a key press.
    #[arg(long)]
    show: bool,

    /// How long to wait for a selection, in seconds.
    #[arg(long, value_name = "SECONDS", default_value_t = 8.0)]
    show_for: f64,

    /// Log every keyboard hook decision to stderr.
    #[arg(long)]
    trace_keys: bool,

    /// Cursor travel time in milliseconds; 0 moves instantly.
    #[arg(long, value_name = "MS", default_value_t = 350.0)]
    move_duration: f64,

    /// Cursor motion curve: linear, ease-out, ease-in-out or damped.
    #[arg(long, value_name = "CURVE", default_value = "ease-out")]
    move_easing: String,

    /// Animation sampling rate for the cursor movement.
    #[arg(long, value_name = "FPS", default_value_t = 120)]
    move_fps: u32,

    /// Wait this long before scanning, so the target window can be brought to the
    /// front. Accepts milliseconds, or a suffix such as 2s.
    ///
    /// A leading dash is allowed through so that a negative value produces the
    /// real "invalid delay" message rather than a clap argument error.
    #[arg(
        long,
        value_name = "MS",
        default_value = "0",
        allow_negative_numbers = true
    )]
    delay: String,
}

fn main() {
    let cli = Cli::parse();
    if let Err(e) = run(&cli) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}

fn run(cli: &Cli) -> Result<(), String> {
    // Writing a default config needs no desktop session.
    if let Some(path) = &cli.write_default_config {
        Config::default().save(path).map_err(|e| e.to_string())?;
        println!("wrote default config to {}", path.display());
        return Ok(());
    }

    let delay = fastmovemouse::startup::Delay::parse(&cli.delay).map_err(|e| e.to_string())?;
    let action = MouseAction::from_str(&cli.action).map_err(|e| e.to_string())?;
    let easing =
        fastmovemouse::motion::Easing::from_str(&cli.move_easing).map_err(|e| e.to_string())?;
    let motion = fastmovemouse::input::Motion {
        duration_ms: cli.move_duration,
        fps: cli.move_fps,
        easing,
    };

    #[cfg(windows)]
    {
        let displays = fastmovemouse::platform::enumerate();

        if cli.list_screens {
            print_screens(&displays);
            return Ok(());
        }

        let target =
            Target::from_flags(cli.screen, cli.focus, cli.statusbar).map_err(|e| e.to_string())?;

        // Wait before scanning, not after: the point is to let the user bring the
        // intended window forward so that it is the window that gets scanned.
        if let Some(notice) = delay.notice() {
            eprintln!("{notice}");
        }
        delay.wait();

        let report = scan_target(&target, cli, &displays)?;
        print!("{}", format_report(&report, action));

        if cli.show {
            if cli.trace_keys {
                fastmovemouse::hook::set_tracing(true);
            }
            match interactive_pick(&report, action, cli.show_for)? {
                Some((x, y)) => {
                    fastmovemouse::input::perform(action, x, y, motion)?;
                    println!(
                        "\nperformed {} at ({x}, {y}) [{} curve, {:.0} ms]",
                        action_name(action),
                        easing.name(),
                        cli.move_duration
                    );
                }
                None => println!("\ncancelled"),
            }
        }
        Ok(())
    }

    #[cfg(not(windows))]
    {
        let _ = (cli, action, motion, delay);
        Err("this tool only runs on Windows".to_string())
    }
}

#[cfg(windows)]
fn scan_target(
    target: &Target,
    cli: &Cli,
    displays: &[fastmovemouse::Display],
) -> Result<fastmovemouse::ScanReport, String> {
    use fastmovemouse::geometry::Monitor;
    use fastmovemouse::scan;
    use fastmovemouse::targets::resolve_screen;

    let leaves_only = !cli.all_elements;

    match target {
        Target::Focus => {
            let hwnd = unsafe { windows::Win32::UI::WindowsAndMessaging::GetForegroundWindow() };
            if hwnd.is_invalid() {
                return Err("no window currently has focus".to_string());
            }
            let rect = scan::window_rect(hwnd);
            let scale = fastmovemouse::platform::window_monitor_scale(hwnd);
            let monitor = Monitor::new(rect, scale);
            let title = window_title(hwnd);
            let rects = scan::scan_window(hwnd, leaves_only, cli.min_area, &monitor, None)
                .map_err(|e| e.to_string())?;
            Ok(build_report(
                rects,
                Source::Window { title, rect },
                monitor.bounds,
            ))
        }

        Target::Screen(n) => {
            let display = resolve_screen(displays, *n).map_err(|e| e.to_string())?;
            let monitor = display.monitor();

            // Scan every top-level window that overlaps this display.
            let mut all = Vec::new();
            for hwnd in scan::top_level_windows() {
                let wr = scan::window_rect(hwnd);
                if !wr.intersects(&display.bounds) {
                    continue;
                }
                match scan::scan_window(
                    hwnd,
                    leaves_only,
                    cli.min_area,
                    &monitor,
                    Some(display.bounds),
                ) {
                    Ok(rects) => all.extend(rects),
                    // A window can vanish or deny automation; keep scanning.
                    Err(_) => continue,
                }
            }
            Ok(build_report(
                all,
                Source::Screen {
                    number: *n,
                    name: display.name.clone(),
                },
                monitor.bounds,
            ))
        }

        Target::StatusBar => {
            let bars = scan::taskbar_windows();
            if bars.is_empty() {
                return Err("no taskbar windows were found".to_string());
            }
            let mut all = Vec::new();
            let mut union: Option<Rect> = None;
            for hwnd in &bars {
                let wr = scan::window_rect(*hwnd);
                union = Some(match union {
                    None => wr,
                    Some(u) => {
                        let x = u.x.min(wr.x);
                        let y = u.y.min(wr.y);
                        let r = u.right().max(wr.right());
                        let b = u.bottom().max(wr.bottom());
                        Rect::new(x, y, r - x, b - y)
                    }
                });
                let scale = fastmovemouse::platform::window_monitor_scale(*hwnd);
                let monitor = Monitor::new(wr, scale);
                if let Ok(rects) =
                    scan::scan_window(*hwnd, leaves_only, cli.min_area, &monitor, None)
                {
                    all.extend(rects);
                }
            }
            let bounds = union.unwrap_or_default();
            Ok(build_report(
                all,
                Source::Taskbars {
                    count: bars.len(),
                    rect: bounds,
                },
                bounds,
            ))
        }
    }
}

/// Show the labels and let the user pick one, then perform the action.
///
/// Keys are polled with GetAsyncKeyState rather than intercepted with a hook: the
/// process is short-lived, needs no elevated install, and cannot be removed by
/// the OS hook timeout. Polling also leaves the user's keystrokes intact for the
/// application underneath.
///
/// Returns the chosen point when the user picked an element.
#[cfg(windows)]
fn interactive_pick(
    report: &fastmovemouse::ScanReport,
    action: fastmovemouse::MouseAction,
    seconds: f64,
) -> Result<Option<(i32, i32)>, String> {
    use fastmovemouse::hook::{self, KeyboardHook};
    use fastmovemouse::keys::KeyState;
    use fastmovemouse::overlay::Overlay;
    use fastmovemouse::session::{Session, Step};
    use std::time::{Duration, Instant};

    if report.items.is_empty() {
        eprintln!("note: nothing to show");
        return Ok(None);
    }

    // A margin keeps boxes near an edge from being clipped by the window.
    let margin = 64;
    let b = report.bounds;
    let region = Rect::new(
        b.x - margin,
        b.y - margin,
        b.width + margin * 2,
        b.height + margin * 2,
    );

    let mut session = Session::start(report.items.iter().map(|i| i.value).collect(), action)
        .ok_or("no candidates to pick from")?;

    let overlay = Overlay::new()?;
    overlay.show(region, session.candidates());

    // The hook exists only while the labels are on screen, and swallows only the
    // keys the session can act on. Everything else reaches the application
    // underneath.
    let _hook = KeyboardHook::install()?;

    let mut keys = KeyState::new();
    let deadline = Instant::now() + Duration::from_secs_f64(seconds.max(1.0));
    let mut result = None;

    while Instant::now() < deadline {
        if !overlay.pump() {
            break;
        }

        if let Some(vk) = hook::take_pending() {
            // The key is already swallowed by the hook; record it so the session
            // can reason about modifiers and then act on it.
            keys.press(vk);
            let step = session.on_key(vk, &keys);
            keys.release(vk);

            match step {
                Step::Act { x, y } => {
                    result = Some((x, y));
                    overlay.hide();
                    return Ok(result);
                }
                Step::Cancel => {
                    overlay.hide();
                    return Ok(None);
                }
                Step::Redraw => overlay.show(region, session.candidates()),
                Step::Wait => {}
            }
        }

        std::thread::sleep(Duration::from_millis(12));
    }

    overlay.hide();
    eprintln!("timed out waiting for a selection");
    Ok(result)
}

/// Stable display name for an action.
#[cfg(windows)]
fn action_name(action: MouseAction) -> &'static str {
    fastmovemouse::app::action_name(action)
}

#[cfg(windows)]
fn window_title(hwnd: windows::Win32::Foundation::HWND) -> String {
    use windows::Win32::UI::WindowsAndMessaging::{GetWindowTextLengthW, GetWindowTextW};
    let len = unsafe { GetWindowTextLengthW(hwnd) };
    if len <= 0 {
        return String::new();
    }
    let mut buf = vec![0u16; len as usize + 1];
    let n = unsafe { GetWindowTextW(hwnd, &mut buf) };
    String::from_utf16_lossy(&buf[..n as usize])
}

#[cfg(windows)]
fn print_screens(displays: &[fastmovemouse::Display]) {
    println!("screens: {}", displays.len());
    for d in displays {
        println!(
            "  {}: {:>5}x{:<5} at ({:>5}, {:>5})  scale {:.2}{}  {}",
            d.index,
            d.bounds.width,
            d.bounds.height,
            d.bounds.x,
            d.bounds.y,
            d.scale,
            if d.primary { "  [primary]" } else { "" },
            d.name
        );
    }
}
