//! Rectangle helpers and DPI coordinate transforms.
//!
//! Windows reports UI Automation bounding rectangles in **physical** pixels,
//! while a per-monitor-DPI-aware window addresses **logical** pixels. The
//! conversions here are pure so they can be tested without a display.

/// An integer rectangle in screen coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
}

impl Rect {
    pub const fn new(x: i32, y: i32, width: i32, height: i32) -> Self {
        Self {
            x,
            y,
            width,
            height,
        }
    }

    pub const fn right(&self) -> i32 {
        self.x + self.width
    }

    pub const fn bottom(&self) -> i32 {
        self.y + self.height
    }

    pub const fn center(&self) -> (i32, i32) {
        (self.x + self.width / 2, self.y + self.height / 2)
    }

    /// True when the rectangle has no drawable area.
    pub const fn is_empty(&self) -> bool {
        self.width <= 0 || self.height <= 0
    }

    /// Area in square pixels; 0 for degenerate rectangles.
    pub const fn area(&self) -> i64 {
        if self.is_empty() {
            0
        } else {
            self.width as i64 * self.height as i64
        }
    }

    /// True when the two rectangles share at least one pixel.
    pub fn intersects(&self, other: &Rect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && self.x < other.right()
            && other.x < self.right()
            && self.y < other.bottom()
            && other.y < self.bottom()
    }

    /// True when `other` lies entirely inside `self`.
    pub fn contains(&self, other: &Rect) -> bool {
        !self.is_empty()
            && !other.is_empty()
            && other.x >= self.x
            && other.y >= self.y
            && other.right() <= self.right()
            && other.bottom() <= self.bottom()
    }

    /// Shift by a delta.
    pub const fn translate(&self, dx: i32, dy: i32) -> Rect {
        Rect::new(self.x + dx, self.y + dy, self.width, self.height)
    }

    /// Position relative to this rectangle's origin.
    pub const fn relative_to(&self, origin: &Rect) -> Rect {
        Rect::new(
            self.x - origin.x,
            self.y - origin.y,
            self.width,
            self.height,
        )
    }
}

/// A monitor's bounds plus its DPI scale.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Monitor {
    /// Monitor bounds in logical (DPI-adjusted) screen pixels.
    pub bounds: Rect,
    /// Scale factor: 1.0 at 96 DPI, 1.5 at 144 DPI, 2.0 at 192 DPI.
    pub scale: f64,
}

impl Monitor {
    pub const fn new(bounds: Rect, scale: f64) -> Self {
        Self { bounds, scale }
    }

    /// Convert a physical-pixel rectangle to logical pixels.
    ///
    /// Coordinates are made monitor-relative before scaling, then rebased, so a
    /// secondary monitor with a non-zero origin does not double-apply an offset.
    pub fn physical_to_logical(&self, rect: &Rect) -> Rect {
        if self.scale <= 0.0 || self.scale == 1.0 {
            return *rect;
        }
        let rel_x = rect.x - self.bounds.x;
        let rel_y = rect.y - self.bounds.y;
        Rect::new(
            self.bounds.x + (rel_x as f64 / self.scale).round() as i32,
            self.bounds.y + (rel_y as f64 / self.scale).round() as i32,
            (rect.width as f64 / self.scale).round() as i32,
            (rect.height as f64 / self.scale).round() as i32,
        )
    }

    /// Convert a logical-pixel rectangle to physical pixels.
    pub fn logical_to_physical(&self, rect: &Rect) -> Rect {
        if self.scale <= 0.0 || self.scale == 1.0 {
            return *rect;
        }
        let rel_x = rect.x - self.bounds.x;
        let rel_y = rect.y - self.bounds.y;
        Rect::new(
            self.bounds.x + (rel_x as f64 * self.scale).round() as i32,
            self.bounds.y + (rel_y as f64 * self.scale).round() as i32,
            (rect.width as f64 * self.scale).round() as i32,
            (rect.height as f64 * self.scale).round() as i32,
        )
    }
}

/// Drop degenerate rectangles and those that are not usable click targets.
///
/// UI Automation reports many zero-sized and offscreen elements; keeping them
/// would produce labels that cannot be reached by the cursor.
pub fn sanitize(rects: &[Rect], screen: &Rect, min_area: i64) -> Vec<Rect> {
    rects
        .iter()
        .copied()
        .filter(|r| !r.is_empty())
        .filter(|r| r.area() >= min_area)
        // The centre, not just any pixel, must be reachable: the cursor is placed
        // at the centre, so a rectangle hanging off the edge would send the
        // pointer somewhere the user cannot see.
        .filter(|r| screen.contains(&Rect::new(r.center().0, r.center().1, 1, 1)))
        .collect()
}

/// A container must not be more than this many times the area of the rectangle
/// it would remove.
///
/// A window's own root element spans the whole window and contains every real
/// target inside it. Without a bound, dropping contained rectangles would delete
/// all of those targets and leave only the window frame. Requiring the container
/// to be comparable in size keeps the filter to genuine duplicates (a border and
/// its single child) rather than wholesale removal.
pub const MAX_CONTAINER_RATIO: i64 = 4;

/// Remove rectangles that are fully contained in a comparable-sized rectangle.
///
/// A parent border often shares its child's bounds; keeping both wastes a label
/// on the same visual target. Containers much larger than their contents are
/// treated as window roots and are ignored, so real targets survive.
pub fn drop_contained(rects: &[Rect]) -> Vec<Rect> {
    let mut result = Vec::with_capacity(rects.len());
    for (i, r) in rects.iter().enumerate() {
        let covered = rects.iter().enumerate().any(|(j, other)| {
            if i == j {
                return false;
            }
            // Identical rectangles: keep the first occurrence only.
            if *other == *r {
                return j < i;
            }
            if !other.contains(r) {
                return false;
            }
            // Only accept a container of comparable size; a much larger one is a
            // window root rather than a duplicate of this target.
            other.area() <= r.area().saturating_mul(MAX_CONTAINER_RATIO)
        });
        if !covered {
            result.push(*r);
        }
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rect_basics() {
        let r = Rect::new(10, 20, 100, 50);
        assert_eq!(r.right(), 110);
        assert_eq!(r.bottom(), 70);
        assert_eq!(r.center(), (60, 45));
        assert!(!r.is_empty());
        assert_eq!(r.area(), 5000);
    }

    #[test]
    fn degenerate_rects_are_empty_and_zero_area() {
        assert!(Rect::new(0, 0, 0, 10).is_empty());
        assert!(Rect::new(0, 0, 10, 0).is_empty());
        assert!(Rect::new(0, 0, -5, 5).is_empty());
        assert_eq!(Rect::new(0, 0, 0, 0).area(), 0);
    }

    #[test]
    fn intersection_detects_overlap() {
        let a = Rect::new(0, 0, 10, 10);
        assert!(a.intersects(&Rect::new(5, 5, 10, 10)));
        assert!(!a.intersects(&Rect::new(10, 0, 10, 10))); // touching, not overlapping
        assert!(!a.intersects(&Rect::new(100, 100, 5, 5)));
    }

    #[test]
    fn containment() {
        let outer = Rect::new(0, 0, 100, 100);
        assert!(outer.contains(&Rect::new(10, 10, 20, 20)));
        assert!(outer.contains(&outer));
        assert!(!outer.contains(&Rect::new(90, 90, 20, 20)));
    }

    #[test]
    fn translate_and_relative() {
        let r = Rect::new(50, 60, 10, 10);
        assert_eq!(r.translate(-50, -60), Rect::new(0, 0, 10, 10));
        let origin = Rect::new(40, 40, 100, 100);
        assert_eq!(r.relative_to(&origin), Rect::new(10, 20, 10, 10));
    }

    #[test]
    fn scale_one_is_identity() {
        let m = Monitor::new(Rect::new(0, 0, 1920, 1080), 1.0);
        let r = Rect::new(100, 200, 300, 400);
        assert_eq!(m.physical_to_logical(&r), r);
        assert_eq!(m.logical_to_physical(&r), r);
    }

    #[test]
    fn scale_two_halves_on_primary_monitor() {
        let m = Monitor::new(Rect::new(0, 0, 1920, 1080), 2.0);
        let got = m.physical_to_logical(&Rect::new(100, 200, 300, 400));
        assert_eq!(got, Rect::new(50, 100, 150, 200));
    }

    #[test]
    fn scale_two_doubles_back() {
        let m = Monitor::new(Rect::new(0, 0, 1920, 1080), 2.0);
        let got = m.logical_to_physical(&Rect::new(50, 100, 150, 200));
        assert_eq!(got, Rect::new(100, 200, 300, 400));
    }

    /// The C++ original subtracted `screen.geometry().x() * (scale - 1)` from
    /// absolute coordinates, which corrupts positions on a secondary monitor
    /// whose origin is non-zero. The transform must stay monitor-relative.
    #[test]
    fn secondary_monitor_origin_is_preserved() {
        let m = Monitor::new(Rect::new(1920, 0, 1920, 1080), 2.0);
        let got = m.physical_to_logical(&Rect::new(2020, 100, 400, 200));
        // x offset is 100 physical from the monitor origin -> 50 logical.
        assert_eq!(got, Rect::new(1970, 50, 200, 100));
    }

    #[test]
    fn negative_origin_monitor_is_preserved() {
        let m = Monitor::new(Rect::new(-1920, 0, 1920, 1080), 2.0);
        let got = m.physical_to_logical(&Rect::new(-1820, 100, 400, 200));
        assert_eq!(got, Rect::new(-1870, 50, 200, 100));
    }

    #[test]
    fn roundtrip_physical_logical_physical_is_stable() {
        let m = Monitor::new(Rect::new(1920, 0, 1920, 1080), 1.5);
        for r in [
            Rect::new(2000, 100, 333, 77),
            Rect::new(1920, 0, 1920, 1080),
            Rect::new(3839, 1079, 1, 1),
        ] {
            let back = m.logical_to_physical(&m.physical_to_logical(&r));
            assert!(
                (back.x - r.x).abs() <= 1
                    && (back.y - r.y).abs() <= 1
                    && (back.width - r.width).abs() <= 1
                    && (back.height - r.height).abs() <= 1,
                "roundtrip drifted for {r:?}: {back:?}"
            );
        }
    }

    #[test]
    fn sanitize_drops_empty_tiny_and_offscreen() {
        let screen = Rect::new(0, 0, 1920, 1080);
        let rects = [
            Rect::new(10, 10, 100, 100),     // keep
            Rect::new(10, 10, 0, 100),       // empty
            Rect::new(10, 10, 2, 2),         // below min area
            Rect::new(5000, 5000, 100, 100), // offscreen
        ];
        let got = sanitize(&rects, &screen, 16);
        assert_eq!(got, vec![Rect::new(10, 10, 100, 100)]);
    }

    /// A rectangle hanging off an edge still overlaps the screen, but the cursor
    /// lands on its centre, so an off-screen centre is unreachable and must be
    /// dropped.
    #[test]
    fn sanitize_drops_rects_whose_centre_is_offscreen() {
        let screen = Rect::new(0, 0, 1920, 1080);
        // y = -7, height 20: overlaps the top edge, centre at y = 3 (on screen).
        let grazes = Rect::new(364, -7, 456, 20);
        assert_eq!(grazes.center().1, 3);
        // Mostly above the screen: the centre is unreachable.
        let above = Rect::new(364, -100, 456, 20);
        assert_eq!(above.center().1, -90);
        assert_eq!(sanitize(&[grazes, above], &screen, 16), vec![grazes]);
    }

    #[test]
    fn sanitize_drops_rects_centred_below_or_right_of_the_screen() {
        let screen = Rect::new(0, 0, 1920, 1080);
        let below = Rect::new(100, 1070, 400, 40); // centre y = 1090
        let right = Rect::new(1900, 100, 200, 40); // centre x = 2000
        assert!(sanitize(&[below, right], &screen, 16).is_empty());
    }

    #[test]
    fn drop_contained_removes_comparable_inner_rects() {
        // A container of similar size is a duplicate of its child and is dropped.
        let rects = [
            Rect::new(0, 0, 20, 20),
            Rect::new(1, 1, 18, 18),
            Rect::new(200, 200, 50, 50),
        ];
        let got = drop_contained(&rects);
        assert_eq!(
            got,
            vec![Rect::new(0, 0, 20, 20), Rect::new(200, 200, 50, 50)]
        );
    }

    /// A window root spans the whole window and contains every real target. It
    /// must not be allowed to delete those targets.
    #[test]
    fn drop_contained_keeps_targets_inside_a_large_container() {
        let rects = [
            Rect::new(0, 0, 1920, 1080), // window root
            Rect::new(10, 10, 47, 30),   // a real button
            Rect::new(100, 100, 22, 22), // another real target
        ];
        let got = drop_contained(&rects);
        assert!(
            got.contains(&Rect::new(10, 10, 47, 30)),
            "button was swallowed by the window root: {got:?}"
        );
        assert!(got.contains(&Rect::new(100, 100, 22, 22)), "{got:?}");
    }

    #[test]
    fn container_ratio_boundary_behaves() {
        // Area ratio exactly at the bound: dropped.
        let at_bound = [Rect::new(0, 0, 40, 40), Rect::new(0, 0, 20, 20)];
        assert_eq!(drop_contained(&at_bound).len(), 1);
        // Ratio above the bound: both kept.
        let above = [Rect::new(0, 0, 100, 100), Rect::new(0, 0, 20, 20)];
        assert_eq!(drop_contained(&above).len(), 2);
    }

    #[test]
    fn drop_contained_keeps_only_first_of_identical_rects() {
        let rects = [Rect::new(5, 5, 10, 10), Rect::new(5, 5, 10, 10)];
        assert_eq!(drop_contained(&rects), vec![Rect::new(5, 5, 10, 10)]);
    }
}
