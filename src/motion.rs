//! Cursor movement curves and path planning.
//!
//! The cursor is animated rather than teleported, for two reasons: the movement
//! reads as intentional, and Windows only repaints the pointer when its position
//! changes incrementally. A single jump to the target leaves the pointer
//! invisible until the user moves a physical mouse.
//!
//! The curve and the sampling are pure, so the whole motion is unit tested.

use std::str::FromStr;

/// How the cursor slows down as it approaches the target.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Easing {
    /// Constant speed.
    Linear,
    /// Fast start, pronounced deceleration into the target.
    #[default]
    EaseOut,
    /// Gentle acceleration and deceleration; smooth at both ends.
    EaseInOut,
    /// A stronger ease-out, so the cursor glides to a stop.
    Damped,
}

impl Easing {
    /// Every variant, in a stable order, for help text and tests.
    pub const ALL: [Easing; 4] = [
        Easing::Linear,
        Easing::EaseOut,
        Easing::EaseInOut,
        Easing::Damped,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Easing::Linear => "linear",
            Easing::EaseOut => "ease-out",
            Easing::EaseInOut => "ease-in-out",
            Easing::Damped => "damped",
        }
    }

    /// Map normalised progress in `0..=1` to an eased position in `0..=1`.
    ///
    /// Every curve satisfies `f(0) == 0` and `f(1) == 1`, so the cursor always
    /// arrives exactly at the target.
    pub fn apply(self, t: f64) -> f64 {
        let t = t.clamp(0.0, 1.0);
        match self {
            Easing::Linear => t,
            // Decelerating cubic: fast departure, slow arrival.
            Easing::EaseOut => 1.0 - (1.0 - t).powi(3),
            // Standard smoothstep: zero velocity at both ends.
            Easing::EaseInOut => {
                if t < 0.5 {
                    4.0 * t * t * t
                } else {
                    1.0 - (-2.0 * t + 2.0).powi(3) / 2.0
                }
            }
            // A critically damped approach: 1 - (1 + k t) e^(-k t), scaled so
            // it reaches 1 at t = 1 while keeping a long, soft tail.
            Easing::Damped => {
                const K: f64 = 6.0;
                let raw = 1.0 - (1.0 + K * t) * (-K * t).exp();
                let end = 1.0 - (1.0 + K) * (-K).exp();
                if end.abs() < f64::EPSILON {
                    t
                } else {
                    (raw / end).clamp(0.0, 1.0)
                }
            }
        }
    }
}

/// Raised when an easing name is not recognised.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseEasingError(pub String);

impl std::fmt::Display for ParseEasingError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let names: Vec<&'static str> = Easing::ALL.iter().map(|e| e.name()).collect();
        write!(
            f,
            "unknown easing {:?}; expected one of: {}",
            self.0,
            names.join(", ")
        )
    }
}

impl std::error::Error for ParseEasingError {}

impl FromStr for Easing {
    type Err = ParseEasingError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.trim().to_ascii_lowercase().replace('_', "-").as_str() {
            "linear" | "none" | "constant" => Ok(Easing::Linear),
            "ease-out" | "easeout" | "out" | "decelerate" => Ok(Easing::EaseOut),
            "ease-in-out" | "easeinout" | "smooth" | "smoothstep" => Ok(Easing::EaseInOut),
            "damped" | "damp" | "spring" => Ok(Easing::Damped),
            _ => Err(ParseEasingError(s.to_string())),
        }
    }
}

/// A planned cursor path: the points to visit, in order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Path {
    /// Intermediate positions, excluding the starting point.
    pub points: Vec<(i32, i32)>,
    /// Delay between successive points, in microseconds.
    pub step_delay_us: u64,
}

impl Path {
    /// A path that does nothing, used when animation is disabled.
    pub fn stationary() -> Self {
        Self {
            points: Vec::new(),
            step_delay_us: 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.points.is_empty()
    }

    /// Where the path ends, if it moves at all.
    pub fn destination(&self) -> Option<(i32, i32)> {
        self.points.last().copied()
    }
}

/// The distance below which animation is skipped.
//
// Animating a two-pixel move would look like a stutter rather than motion.
pub const MIN_ANIMATED_DISTANCE: f64 = 8.0;

/// Plan a cursor path from `from` to `to`.
///
/// `duration_ms` is the intended travel time and `fps` the sampling rate. The
/// returned path always ends exactly on `to`, so the cursor lands precisely on
/// the target even though the intermediate steps are rounded.
pub fn plan(from: (i32, i32), to: (i32, i32), duration_ms: f64, fps: u32, easing: Easing) -> Path {
    if duration_ms <= 0.0 || fps == 0 {
        return Path {
            points: vec![to],
            step_delay_us: 0,
        };
    }

    let dx = (to.0 - from.0) as f64;
    let dy = (to.1 - from.1) as f64;
    let distance = (dx * dx + dy * dy).sqrt();
    if distance < MIN_ANIMATED_DISTANCE {
        // Too short to animate; jump so the cursor is never left behind.
        return Path {
            points: vec![to],
            step_delay_us: 0,
        };
    }

    let steps = ((duration_ms / 1000.0) * fps as f64).round() as usize;
    let steps = steps.clamp(1, 600);

    let mut points = Vec::with_capacity(steps);
    for i in 1..=steps {
        let t = i as f64 / steps as f64;
        let e = easing.apply(t);
        let x = from.0 + (dx * e).round() as i32;
        let y = from.1 + (dy * e).round() as i32;
        points.push((x, y));
    }
    // Guarantee arrival: rounding must never leave the cursor one pixel off.
    if let Some(last) = points.last_mut() {
        *last = to;
    }

    Path {
        points,
        step_delay_us: ((1_000_000.0 / fps as f64).round() as u64).max(1),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn approx(a: f64, b: f64) {
        assert!((a - b).abs() < 1e-9, "{a} != {b}");
    }

    #[test]
    fn every_curve_starts_at_zero_and_ends_at_one() {
        for e in Easing::ALL {
            approx(e.apply(0.0), 0.0);
            approx(e.apply(1.0), 1.0);
        }
    }

    #[test]
    fn every_curve_is_monotonic() {
        for e in Easing::ALL {
            let mut prev = -1.0;
            for i in 0..=100 {
                let v = e.apply(i as f64 / 100.0);
                assert!(v >= prev - 1e-12, "{} went backwards at {i}", e.name());
                prev = v;
            }
        }
    }

    #[test]
    fn every_curve_stays_within_range() {
        for e in Easing::ALL {
            for i in 0..=200 {
                let v = e.apply(i as f64 / 200.0);
                assert!((0.0..=1.0).contains(&v), "{} produced {v}", e.name());
            }
        }
    }

    #[test]
    fn values_outside_the_unit_range_are_clamped() {
        for e in Easing::ALL {
            approx(e.apply(-5.0), 0.0);
            approx(e.apply(5.0), 1.0);
        }
    }

    /// The defining property of an ease-out: it is ahead of linear throughout.
    #[test]
    fn ease_out_leads_linear_in_the_middle() {
        for i in 1..100 {
            let t = i as f64 / 100.0;
            assert!(
                Easing::EaseOut.apply(t) > t,
                "ease-out was not ahead of linear at t={t}"
            );
        }
    }

    #[test]
    fn linear_is_the_identity() {
        for i in 0..=10 {
            let t = i as f64 / 10.0;
            approx(Easing::Linear.apply(t), t);
        }
    }

    /// A damped curve should cover most of the distance early and then creep.
    #[test]
    fn damped_is_mostly_done_early() {
        let half = Easing::Damped.apply(0.5);
        assert!(half > 0.8, "damped only reached {half} by halfway");
    }

    #[test]
    fn ease_in_out_is_symmetric() {
        for i in 0..=50 {
            let t = i as f64 / 50.0;
            let a = Easing::EaseInOut.apply(t);
            let b = 1.0 - Easing::EaseInOut.apply(1.0 - t);
            assert!((a - b).abs() < 1e-9, "asymmetric at t={t}: {a} vs {b}");
        }
    }

    #[test]
    fn easing_names_round_trip() {
        for e in Easing::ALL {
            assert_eq!(e.name().parse(), Ok(e), "{} did not round-trip", e.name());
        }
    }

    #[test]
    fn parsing_is_case_and_separator_insensitive() {
        assert_eq!("Ease_Out".parse(), Ok(Easing::EaseOut));
        assert_eq!("  LINEAR  ".parse(), Ok(Easing::Linear));
        assert_eq!("smoothstep".parse(), Ok(Easing::EaseInOut));
    }

    #[test]
    fn unknown_easing_is_rejected_and_lists_options() {
        let e = "wobble".parse::<Easing>().unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("wobble"), "{msg}");
        assert!(msg.contains("ease-out"), "{msg}");
    }

    #[test]
    fn plan_ends_exactly_on_the_target() {
        for e in Easing::ALL {
            let p = plan((0, 0), (1000, 500), 300.0, 120, e);
            assert_eq!(p.destination(), Some((1000, 500)), "{} missed", e.name());
        }
    }

    #[test]
    fn plan_produces_the_expected_number_of_steps() {
        // 300 ms at 120 fps is 36 samples.
        let p = plan((0, 0), (1000, 0), 300.0, 120, Easing::Linear);
        assert_eq!(p.points.len(), 36);
    }

    #[test]
    fn step_delay_matches_the_requested_rate() {
        let p = plan((0, 0), (1000, 0), 300.0, 100, Easing::Linear);
        assert_eq!(p.step_delay_us, 10_000);
    }

    #[test]
    fn a_short_move_is_not_animated() {
        let p = plan((0, 0), (3, 2), 300.0, 120, Easing::EaseOut);
        assert_eq!(p.points, vec![(3, 2)]);
        assert_eq!(p.step_delay_us, 0);
    }

    #[test]
    fn zero_duration_jumps_straight_to_the_target() {
        let p = plan((0, 0), (500, 500), 0.0, 120, Easing::EaseOut);
        assert_eq!(p.points, vec![(500, 500)]);
    }

    #[test]
    fn a_zero_fps_rate_is_treated_as_no_animation() {
        let p = plan((0, 0), (500, 500), 300.0, 0, Easing::EaseOut);
        assert_eq!(p.points, vec![(500, 500)]);
    }

    #[test]
    fn path_points_are_distinct_for_a_long_move() {
        let p = plan((0, 0), (1000, 0), 400.0, 60, Easing::EaseOut);
        assert!(p.points.len() > 5, "too few samples: {}", p.points.len());
        // The cursor must make progress, not sit still while time passes.
        let unique: std::collections::HashSet<_> = p.points.iter().collect();
        assert!(unique.len() > 3, "path barely moved: {:?}", p.points);
    }

    #[test]
    fn ease_out_covers_more_ground_early_than_linear() {
        // 1000 ms at 60 fps gives 60 samples to compare.
        let out = plan((0, 0), (1000, 0), 1000.0, 60, Easing::EaseOut);
        let lin = plan((0, 0), (1000, 0), 1000.0, 60, Easing::Linear);
        assert_eq!(out.points.len(), 60);
        assert_eq!(lin.points.len(), 60);

        // At a quarter of the way through, ease-out should be further along.
        let q = out.points.len() / 4;
        assert!(
            out.points[q].0 > lin.points[q].0,
            "ease-out {} was not ahead of linear {} at step {q}",
            out.points[q].0,
            lin.points[q].0
        );
    }

    #[test]
    fn steep_requests_are_capped() {
        // 60 s at 1000 fps would be 60 000 steps; the cap keeps it sane.
        let p = plan((0, 0), (100, 100), 60_000.0, 1000, Easing::Linear);
        assert!(p.points.len() <= 600, "{} steps", p.points.len());
    }

    #[test]
    fn stationary_path_reports_itself_as_empty() {
        let p = Path::stationary();
        assert!(p.is_empty());
        assert_eq!(p.destination(), None);
    }
}
