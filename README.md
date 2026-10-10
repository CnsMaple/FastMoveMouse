# fastmovemouse

Keyboard-driven UI element picker for Windows. It scans a screen, the focused
window, or the taskbars, and prints a letter for every element it can reach,
so you can drive any application without a mouse.

## Build

```
cargo build --release
```

Output: `target/x86_64-pc-windows-gnu/release/fastmovemouse.exe`
(~600 KB, self-contained, no runtime DLLs).

### Toolchain note

`.cargo/config.toml` pins the `x86_64-pc-windows-gnu` target with `gcc` as the
linker, because this machine has MinGW-w64 but no MSVC build tools. Git for
Windows also ships a `link.exe` that shadows the MSVC linker. Build with
`cargo +stable-x86_64-pc-windows-gnu`, or make that toolchain the default:

```
rustup default stable-x86_64-pc-windows-gnu
```

Delete `.cargo/config.toml` to build with MSVC instead.

## Usage

```
fastmovemouse.exe [OPTIONS]
```

### Choosing what to scan

| Flag | Scans |
| --- | --- |
| `--focus` | The window that currently has focus (default) |
| `--screen <N>` | Every visible element on screen `N`, numbered from 1 |
| `--statusbar` | The taskbar on every display (start, tray, clock) |

The three are mutually exclusive; supplying more than one is an error.

### Choosing the mouse behaviour

`--action` decides what happens after a label matches:

| Value | Behaviour |
| --- | --- |
| `move` | Move the cursor only (default) |
| `click` | Move and left-click |
| `double-click` | Move and double-click |
| `right-click` | Move and right-click |
| `middle-click` | Move and middle-click |

### Other options

| Flag | Meaning |
| --- | --- |
| `--list-screens` | Print the detected screens and exit |
| `--min-area <PX>` | Ignore elements smaller than this (default 16) |
| `--all-elements` | Also offer elements that have children |
| `--config <PATH>` | Read settings from a JSON file |
| `--write-default-config <PATH>` | Write the defaults and exit |
| `--show` | Draw the labels on screen and wait for a key press |
| `--show-for <SECONDS>` | How long to wait for a selection (default 8) |
| `--trace-keys` | Log every keyboard hook decision to stderr |
| `--delay <MS>` | Wait before scanning, so a target window can be brought forward |
| `--move-duration <MS>` | Cursor travel time; 0 teleports (default 350) |
| `--move-easing <CURVE>` | `linear`, `ease-out`, `ease-in-out`, `damped` |
| `--move-fps <FPS>` | Animation sampling rate (default 120) |

## Examples

```
# What screens are available?
fastmovemouse.exe --list-screens

# List every element on the second screen
fastmovemouse.exe --screen 2

# Elements of the focused window, ignoring small ones
fastmovemouse.exe --focus --min-area 400

# Taskbar buttons across all displays, clicking on match
fastmovemouse.exe --statusbar --action click

# Show the labels, then press a letter to click that element
fastmovemouse.exe --focus --show --action click

# Wait up to 30 seconds for a choice
fastmovemouse.exe --screen 1 --show --action click --show-for 30
```

## Output

```
target: screen 1 (\.DISPLAY2); all visible elements
bounds: 1920x1080 at (0, 0)
action: move
elements: 347 (from 358 raw)

  A     47x30    at ( 1371,   201)
  B     46x30    at ( 1418,   201)
  ...
```

Each element gets a label. Up to 26 elements use `A`-`Z`; up to 676 use `AA`-`ZZ`;
up to 17576 use `AAA`-`ZZZ`. Labels share prefixes, so typing narrows the
selection one character at a time until a single element remains.

`elements` counts the offered targets; `from N raw` is the raw count before
duplicate and unreachable rectangles were removed.

## Picking an element

With `--show`, the labels are drawn on screen over the target and the program
waits for you to press a key:

1. Type the first letter shown on an element. Candidates that do not start with
   that letter disappear.
2. With more than 26 elements, labels are two or three letters long and each
   keystroke narrows the selection one level.
3. When one element remains, the action runs and the overlay closes.
4. Press `Esc` to cancel without moving the cursor, or wait for `--show-for` to
   expire.

## Start-up delay

`--delay` waits before scanning. The wait happens *before* the window is chosen,
which is what makes the flag useful: bind the tool to a shortcut or run it from a
terminal, and the delay gives you time to bring the intended window to the front.

```
# Wait one second, then scan whatever is in front
fastmovemouse.exe --focus --show --delay 1000

# A suffix may be used instead of bare milliseconds
fastmovemouse.exe --focus --show --delay 2s
```

A notice is written to stderr so the wait is not mistaken for a hang:

```
waiting 2s before scanning; bring the target window to the front
```

Values above 600000 ms (ten minutes) are rejected, so a mistyped number cannot
leave the process looking stuck.

## Cursor movement

The cursor glides to its target instead of jumping. This is not only cosmetic:
after a single `SetCursorPos` Windows does not repaint the pointer until the user
moves a physical mouse, so the cursor appears to vanish at the destination.
Incremental moves force the redraw.

| Curve | Character |
| --- | --- |
| `linear` | Constant speed |
| `ease-out` | Fast start, decelerates into the target (default) |
| `ease-in-out` | Smooth at both ends |
| `damped` | Covers most of the distance early, then glides to a stop |

```
# A slow, pronounced glide
fastmovemouse.exe --focus --show --move-duration 800 --move-easing damped

# Instant, as before
fastmovemouse.exe --focus --show --move-duration 0
```

The path always ends exactly on the target, whatever the curve or rounding. Moves
shorter than 8 pixels are not animated, since interpolating them looks like a
stutter rather than motion. With a clicking action, the button is pressed only
after the cursor has settled, so a click cannot land on something the cursor
merely passes over.

### Key interception

While the labels are on screen a low-level keyboard hook (`WH_KEYBOARD_LL`) is
installed, and **only the keys the session can act on are swallowed**: the letters
`A`-`Z` and `Esc`. Every other key, including modifiers, Enter, Tab and function
keys, is passed straight through, so shortcuts and text entry keep working behind
the overlay.

The hook exists only for the lifetime of one session. It is installed when the
labels appear and removed as soon as you pick or cancel, so ordinary typing is
never affected. Key-up events are always passed through, so a swallowed letter can
never leave a key stuck down in the application underneath.

The callback does the minimum possible work: it decides whether to swallow the key
and records the code. Scanning, painting and input synthesis all happen on the main
thread. This matters because Windows silently removes a hook whose callback
exceeds its time budget.

Use `--trace-keys` to see the decisions:

```
[hook] vk 0x0d (other) -> passed through
[hook] vk 0x41 (A) -> consumed
```

## Configuration file

```json
{
  "toggle": { "keys": [162, 164, 32] },
  "quit": { "keys": [162, 164, 81] },
  "min_area": 16,
  "leaves_only": true
}
```

Command-line flags override the file. A missing file is not an error; a
malformed one is, so a typo cannot silently change behaviour.

## Architecture

The crate is split so that everything decidable without Windows is pure and
unit tested; Win32 and COM calls sit behind narrow boundaries.

| Module | Responsibility | Tested |
| --- | --- | --- |
| `label` | Label allocation (base-26) and prefix narrowing | unit |
| `geometry` | Rectangle maths, DPI transforms, candidate filtering | unit |
| `motion` | Cursor movement curves and path planning | unit |
| `startup` | Start-up delay parsing and validation | unit |
| `monitors` | Display model and 1-based screen selection | unit |
| `targets` | What to scan: screen, focus, or taskbars | unit |
| `action` | Mouse behaviour selection and parsing | unit |
| `app` | Report construction and text formatting | unit |
| `keys` | Virtual-key tracking and chord matching | unit |
| `config` | Chord parsing and JSON persistence | unit |
| `picker` | Overlay session state machine | unit |
| `session` | Interactive picking decisions | unit |
| `hook` | Selective keyboard interception | unit + manual |
| `scan` | UI Automation collection (COM) | integration |
| `platform` | Display enumeration and DPI | integration |
| `overlay` | Layered topmost window and GDI painting | integration |
| `input` | Animated cursor movement and button synthesis | integration |

## Design notes

**Scanning is scoped.** A UI Automation walk of an entire desktop takes roughly
2.6 s; a single window takes about 0.5 s. Screen and taskbar scans therefore
merge per-window scans rather than walking the desktop root.

**Window roots do not swallow their contents.** A maximized window reports a
root element spanning the whole screen. Containment filtering only removes a
rectangle when the container is at most 4x its area, so real targets inside a
maximized window survive.

**Off-screen elements are dropped.** The cursor lands on an element centre, so
a rectangle whose centre lies outside the screen is unreachable and is excluded
even though it overlaps the display.

**COM is initialised per scan and paired.** `scan::ComGuard` calls
`CoInitializeEx(COINIT_MULTITHREADED)` and `CoUninitialize()` together in a
`Drop` impl, so an early return cannot unbalance the apartment.

**Labels are converted to window coordinates.** Element rectangles are absolute
screen coordinates, while GDI paints in window-relative space. The overlay stores
its own origin and translates every rectangle before drawing, so labels land
correctly on a display positioned left of or above the primary one, where the
origin is negative.

**DPI handling is monitor-relative.** Coordinates are rebased against the target
monitor origin before scaling, so displays with non-zero origins are correct.

## Testing

```
cargo test --release
```

184 tests: 168 unit tests plus 16 integration tests that enumerate real
displays, discover real taskbar windows, run the full COM scan path, and create
an overlay window. Integration tests skip themselves with a message when the
session offers nothing suitable.

Not covered by tests, and requiring a human:

* the visual result of the overlay on screen (verified manually: labels align on
  both the primary display and a display with a negative origin);
* whether a synthetic click actually lands on the intended control in a
  third-party application;
* behaviour across many monitors with mixed DPI settings beyond what the
  transform tests assert.

