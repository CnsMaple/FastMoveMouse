//! One-shot scan-and-report flow.
//!
//! There is no global hook: the process resolves a target, scans it, prints the
//! labelled elements, and exits. That keeps the tool scriptable and removes the
//! keyboard-hook failure modes (callback time budget, hook removal, and the
//! key-state reentrancy that previously panicked).

use crate::action::MouseAction;
use crate::geometry::{drop_contained, Rect};
use crate::label::Labelled;

/// Where the elements came from, for reporting.
#[derive(Debug, Clone, PartialEq)]
pub enum Source {
    /// A single window.
    Window { title: String, rect: Rect },
    /// Every visible element on one display.
    Screen { number: usize, name: String },
    /// Several taskbar windows, one per display.
    Taskbars { count: usize, rect: Rect },
    /// Synthetic input, used by tests.
    Provided { rect: Rect },
}

/// A completed scan.
#[derive(Debug, Clone, PartialEq)]
pub struct ScanReport {
    /// Logical-pixel bounds covered by the scan.
    pub bounds: Rect,
    /// Labelled candidate rectangles, already filtered and relabelled.
    pub items: Vec<Labelled<Rect>>,
    /// What was scanned.
    pub source: Source,
    /// How many raw elements the scan returned before filtering.
    pub raw_count: usize,
}

impl ScanReport {
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

/// Build a report from raw rectangles: deduplicate, label, and record bounds.
///
/// Containment filtering is applied because a container often shares the bounds
/// of its only child, which would waste a label on the same visual target.
pub fn build_report(mut rects: Vec<Rect>, source: Source, bounds: Rect) -> ScanReport {
    let raw_count = rects.len();
    rects.sort_by_key(|r| (r.y, r.x));
    let rects = drop_contained(&rects);

    let items = crate::label::allocate_labels(rects.len())
        .map(|labels| {
            labels
                .into_iter()
                .zip(rects)
                .map(|(label, value)| Labelled::new(label, value))
                .collect()
        })
        .unwrap_or_default();

    ScanReport {
        bounds,
        items,
        source,
        raw_count,
    }
}

/// Render a report as human-readable text.
pub fn format_report(report: &ScanReport, action: MouseAction) -> String {
    let mut out = String::new();
    out.push_str(&format!("target: {}\n", describe_source(&report.source)));
    out.push_str(&format!(
        "bounds: {}x{} at ({}, {})\n",
        report.bounds.width, report.bounds.height, report.bounds.x, report.bounds.y
    ));
    out.push_str(&format!("action: {}\n", action_name(action)));
    out.push_str(&format!(
        "elements: {} (from {} raw)\n",
        report.items.len(),
        report.raw_count
    ));

    if report.items.is_empty() {
        out.push_str("\nno candidate elements found\n");
        return out;
    }

    out.push('\n');
    let width = report
        .items
        .iter()
        .map(|i| i.label.len())
        .max()
        .unwrap_or(1);
    for item in &report.items {
        let r = &item.value;
        out.push_str(&format!(
            "  {:<width$}  {:>5}x{:<5} at ({:>5}, {:>5})\n",
            item.label,
            r.width,
            r.height,
            r.x,
            r.y,
            width = width
        ));
    }
    out
}

fn describe_source(source: &Source) -> String {
    match source {
        Source::Window { title, .. } => format!("focused window {title:?}"),
        Source::Screen { number, name } => {
            format!("screen {number} ({name}); all visible elements")
        }
        Source::Taskbars { count, .. } => format!("{count} taskbar window(s)"),
        Source::Provided { .. } => "provided rectangles".to_string(),
    }
}

/// Stable display name for an action.
pub fn action_name(action: MouseAction) -> &'static str {
    match action {
        MouseAction::Move => "move",
        MouseAction::Click => "click",
        MouseAction::DoubleClick => "double-click",
        MouseAction::RightClick => "right-click",
        MouseAction::MiddleClick => "middle-click",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rects() -> Vec<Rect> {
        vec![
            Rect::new(0, 0, 20, 20),
            Rect::new(1, 1, 18, 18), // comparable container, should be dropped
            Rect::new(200, 200, 50, 50),
        ]
    }

    fn bounds() -> Rect {
        Rect::new(0, 0, 300, 300)
    }

    #[test]
    fn containment_is_filtered_before_labelling() {
        let r = build_report(rects(), Source::Provided { rect: bounds() }, bounds());
        assert_eq!(r.raw_count, 3);
        assert_eq!(r.len(), 2);
    }

    /// Scanning a screen merges rects from many windows, so a full-screen window
    /// root must not erase the real targets inside it.
    #[test]
    fn window_root_does_not_erase_real_targets() {
        let input = vec![
            Rect::new(0, 0, 1920, 1080), // maximized window root
            Rect::new(10, 10, 47, 30),
            Rect::new(100, 100, 22, 22),
        ];
        let wide = Rect::new(0, 0, 1920, 1080);
        let r = build_report(input, Source::Provided { rect: wide }, wide);
        assert_eq!(r.len(), 3, "targets were dropped: {:?}", r.items);
    }

    #[test]
    fn items_are_labelled_from_a() {
        let r = build_report(rects(), Source::Provided { rect: bounds() }, bounds());
        assert_eq!(r.items[0].label, "A");
        assert_eq!(r.items[1].label, "B");
    }

    #[test]
    fn items_are_ordered_top_to_bottom_then_left_to_right() {
        let input = vec![
            Rect::new(200, 0, 10, 10),
            Rect::new(0, 0, 10, 10),
            Rect::new(0, 100, 10, 10),
        ];
        let r = build_report(input, Source::Provided { rect: bounds() }, bounds());
        let xs: Vec<i32> = r.items.iter().map(|i| i.value.x).collect();
        assert_eq!(xs, vec![0, 200, 0]);
    }

    #[test]
    fn empty_input_yields_an_empty_report() {
        let r = build_report(vec![], Source::Provided { rect: bounds() }, bounds());
        assert!(r.is_empty());
        assert_eq!(r.len(), 0);
    }

    #[test]
    fn report_text_mentions_target_action_and_count() {
        let r = build_report(rects(), Source::Provided { rect: bounds() }, bounds());
        let text = format_report(&r, MouseAction::DoubleClick);
        assert!(text.contains("double-click"), "{text}");
        assert!(text.contains("elements: 2"), "{text}");
        assert!(text.contains("from 3 raw"), "{text}");
    }

    #[test]
    fn empty_report_says_so() {
        let r = build_report(vec![], Source::Provided { rect: bounds() }, bounds());
        let text = format_report(&r, MouseAction::Move);
        assert!(text.contains("no candidate elements"), "{text}");
    }

    #[test]
    fn labels_are_rendered_for_large_sets() {
        let many: Vec<Rect> = (0..30).map(|i| Rect::new(i * 40, 0, 20, 20)).collect();
        let wide = Rect::new(0, 0, 2000, 100);
        let r = build_report(many, Source::Provided { rect: wide }, wide);
        let text = format_report(&r, MouseAction::Click);
        assert!(text.contains("AA"), "{text}");
        assert!(text.contains("click"), "{text}");
    }

    #[test]
    fn screen_source_describes_the_display() {
        let r = build_report(
            vec![Rect::new(0, 0, 50, 50)],
            Source::Screen {
                number: 2,
                name: "DISPLAY2".into(),
            },
            bounds(),
        );
        let text = format_report(&r, MouseAction::Move);
        assert!(text.contains("screen 2"), "{text}");
        assert!(text.contains("DISPLAY2"), "{text}");
    }

    #[test]
    fn taskbar_source_reports_the_count() {
        let r = build_report(
            vec![Rect::new(0, 0, 50, 50)],
            Source::Taskbars {
                count: 3,
                rect: bounds(),
            },
            bounds(),
        );
        let text = format_report(&r, MouseAction::Move);
        assert!(text.contains("3 taskbar"), "{text}");
    }

    #[test]
    fn action_names_are_stable() {
        assert_eq!(action_name(MouseAction::Move), "move");
        assert_eq!(action_name(MouseAction::Click), "click");
        assert_eq!(action_name(MouseAction::DoubleClick), "double-click");
        assert_eq!(action_name(MouseAction::RightClick), "right-click");
        assert_eq!(action_name(MouseAction::MiddleClick), "middle-click");
    }
}
