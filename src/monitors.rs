//! Display enumeration and 1-based screen selection.
//!
//! Screens are numbered from 1 in the order Windows reports them, which matches
//! the display numbering shown in Windows display settings for the primary
//! display plus additional displays.

use crate::geometry::{Monitor, Rect};

/// A display as reported by the system.
#[derive(Debug, Clone, PartialEq)]
pub struct Display {
    /// 1-based index, as accepted on the command line.
    pub index: usize,
    /// Whether this is the primary display.
    pub primary: bool,
    /// Bounds in physical pixels.
    pub bounds: Rect,
    /// DPI scale: 1.0 at 96 DPI, 1.5 at 144 DPI.
    pub scale: f64,
    /// Device name such as \\.\DISPLAY1.
    pub name: String,
}

impl Display {
    /// The monitor used for coordinate transforms over this display.
    pub fn monitor(&self) -> Monitor {
        Monitor::new(self.bounds, self.scale)
    }
}

/// Errors selecting a screen.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ScreenError {
    /// No displays were reported at all.
    NoDisplays,
    /// The requested index is outside 1..=available.
    OutOfRange { requested: usize, available: usize },
}

impl std::fmt::Display for ScreenError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreenError::NoDisplays => write!(f, "no displays were detected"),
            ScreenError::OutOfRange { requested, available } => write!(f,
                "screen {requested} does not exist; {available} display(s) available (1-{available})"
            ),
        }
    }
}

impl std::error::Error for ScreenError {}

/// Pick a display by its 1-based index.
pub fn select(displays: &[Display], index: usize) -> Result<&Display, ScreenError> {
    if displays.is_empty() {
        return Err(ScreenError::NoDisplays);
    }
    displays
        .iter()
        .find(|d| d.index == index)
        .ok_or(ScreenError::OutOfRange {
            requested: index,
            available: displays.len(),
        })
}

/// The primary display, or the first one when none is marked primary.
pub fn primary(displays: &[Display]) -> Result<&Display, ScreenError> {
    if displays.is_empty() {
        return Err(ScreenError::NoDisplays);
    }
    Ok(displays.iter().find(|d| d.primary).unwrap_or(&displays[0]))
}

/// Display containing a point, for resolving which screen a window is on.
pub fn containing(displays: &[Display], x: i32, y: i32) -> Option<&Display> {
    displays.iter().find(|d| {
        x >= d.bounds.x
            && x < d.bounds.x + d.bounds.width
            && y >= d.bounds.y
            && y < d.bounds.y + d.bounds.height
    })
}

/// Display with the largest overlap with a rectangle.
pub fn best_for_rect<'a>(displays: &'a [Display], rect: &Rect) -> Option<&'a Display> {
    displays
        .iter()
        .max_by_key(|d| overlap_area(&d.bounds, rect))
}

/// Area of intersection between two rectangles, 0 when they do not overlap.
pub fn overlap_area(a: &Rect, b: &Rect) -> i64 {
    let w = (a.right().min(b.right()) - a.x.max(b.x)).max(0) as i64;
    let h = (a.bottom().min(b.bottom()) - a.y.max(b.y)).max(0) as i64;
    w * h
}

#[cfg(test)]
mod tests {
    use super::*;

    fn displays() -> Vec<Display> {
        vec![
            Display {
                index: 1,
                primary: true,
                bounds: Rect::new(0, 0, 1920, 1080),
                scale: 1.0,
                name: "\\\\.\\DISPLAY1".into(),
            },
            Display {
                index: 2,
                primary: false,
                bounds: Rect::new(1920, 0, 2560, 1440),
                scale: 1.5,
                name: "\\\\.\\DISPLAY2".into(),
            },
            Display {
                index: 3,
                primary: false,
                bounds: Rect::new(-1280, 0, 1280, 1024),
                scale: 1.0,
                name: "\\\\.\\DISPLAY3".into(),
            },
        ]
    }

    #[test]
    fn selects_by_one_based_index() {
        let d = displays();
        assert_eq!(select(&d, 1).unwrap().name, "\\\\.\\DISPLAY1");
        assert_eq!(select(&d, 2).unwrap().name, "\\\\.\\DISPLAY2");
        assert_eq!(select(&d, 3).unwrap().name, "\\\\.\\DISPLAY3");
    }

    #[test]
    fn zero_is_out_of_range_because_indexing_is_one_based() {
        assert_eq!(
            select(&displays(), 0),
            Err(ScreenError::OutOfRange {
                requested: 0,
                available: 3
            })
        );
    }

    #[test]
    fn too_large_index_is_out_of_range() {
        assert!(matches!(
            select(&displays(), 4),
            Err(ScreenError::OutOfRange { .. })
        ));
    }

    #[test]
    fn empty_display_list_reports_no_displays() {
        assert_eq!(select(&[], 1), Err(ScreenError::NoDisplays));
        assert_eq!(primary(&[]), Err(ScreenError::NoDisplays));
    }

    #[test]
    fn primary_prefers_the_marked_display() {
        let mut d = displays();
        d[0].primary = false;
        d[1].primary = true;
        assert_eq!(primary(&d).unwrap().index, 2);
    }

    #[test]
    fn primary_falls_back_to_first_when_unmarked() {
        let mut d = displays();
        for x in &mut d {
            x.primary = false;
        }
        assert_eq!(primary(&d).unwrap().index, 1);
    }

    #[test]
    fn containing_locates_the_right_display() {
        let d = displays();
        assert_eq!(containing(&d, 100, 100).unwrap().index, 1);
        assert_eq!(containing(&d, 2000, 100).unwrap().index, 2);
        assert_eq!(containing(&d, -1000, 100).unwrap().index, 3);
        assert!(containing(&d, 99999, 99999).is_none());
    }

    #[test]
    fn best_for_rect_picks_largest_overlap() {
        let d = displays();
        // Mostly on display 2.
        let r = Rect::new(1900, 100, 500, 400);
        assert_eq!(best_for_rect(&d, &r).unwrap().index, 2);
        // Entirely on display 1.
        assert_eq!(
            best_for_rect(&d, &Rect::new(10, 10, 50, 50)).unwrap().index,
            1
        );
    }

    #[test]
    fn overlap_area_is_zero_for_disjoint_rects() {
        assert_eq!(
            overlap_area(&Rect::new(0, 0, 10, 10), &Rect::new(100, 0, 10, 10)),
            0
        );
        assert_eq!(
            overlap_area(&Rect::new(0, 0, 10, 10), &Rect::new(5, 5, 10, 10)),
            25
        );
    }

    #[test]
    fn monitor_exposes_bounds_and_scale() {
        let d = &displays()[1];
        let m = d.monitor();
        assert_eq!(m.bounds, Rect::new(1920, 0, 2560, 1440));
        assert_eq!(m.scale, 1.5);
    }
}
