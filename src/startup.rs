//! Start-up delay handling.
//!
//! A delay before scanning gives the user time to bring the intended window to
//! the foreground, which is how the tool is normally driven: bind it to a
//! shortcut or run it from a terminal, then switch windows.
//!
//! The parsing and validation are pure, so the accepted range and the
//! formatting of the notice are unit tested.

/// The longest delay accepted, in milliseconds.
//
// A guard against a mistyped value holding the terminal for hours. Ten minutes
// is far longer than any plausible window switch.
pub const MAX_DELAY_MS: u64 = 600_000;

/// A validated start-up delay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Delay {
    millis: u64,
}

impl Delay {
    /// No delay at all.
    pub const fn none() -> Self {
        Self { millis: 0 }
    }

    /// Validate a delay in milliseconds.
    ///
    /// Rejects values above `MAX_DELAY_MS` so a mistyped number cannot leave the
    /// process apparently hung.
    pub fn from_millis(millis: u64) -> Result<Self, DelayError> {
        if millis > MAX_DELAY_MS {
            return Err(DelayError::TooLong {
                requested: millis,
                max: MAX_DELAY_MS,
            });
        }
        Ok(Self { millis })
    }

    /// Parse a delay from a command-line value, accepting a plain number of
    /// milliseconds or a suffix such as `1s` or `500ms`.
    pub fn parse(value: &str) -> Result<Self, DelayError> {
        let text = value.trim();
        if text.is_empty() {
            return Err(DelayError::Invalid(value.to_string()));
        }

        let lower = text.to_ascii_lowercase();
        let (digits, scale) = if let Some(d) = lower.strip_suffix("ms") {
            (d, 1u64)
        } else if let Some(d) = lower.strip_suffix('s') {
            (d, 1000u64)
        } else {
            (lower.as_str(), 1u64)
        };

        let number: f64 = digits
            .trim()
            .parse()
            .map_err(|_| DelayError::Invalid(value.to_string()))?;
        if !number.is_finite() || number < 0.0 {
            return Err(DelayError::Invalid(value.to_string()));
        }

        Self::from_millis((number * scale as f64).round() as u64)
    }

    pub const fn as_millis(&self) -> u64 {
        self.millis
    }

    /// Whether there is anything to wait for.
    pub const fn is_zero(&self) -> bool {
        self.millis == 0
    }

    /// The duration to sleep.
    pub const fn duration(&self) -> std::time::Duration {
        std::time::Duration::from_millis(self.millis)
    }

    /// Notice shown on stderr before waiting, or None when there is no delay.
    ///
    /// Written to stderr rather than stdout so piping the element listing is not
    /// disturbed.
    pub fn notice(&self) -> Option<String> {
        if self.is_zero() {
            return None;
        }
        Some(format!(
            "waiting {} before scanning; bring the target window to the front",
            self.describe()
        ))
    }

    /// Human-readable length, using the largest whole unit that fits.
    pub fn describe(&self) -> String {
        if self.millis >= 1000 && self.millis.is_multiple_of(1000) {
            format!("{}s", self.millis / 1000)
        } else {
            format!("{}ms", self.millis)
        }
    }

    /// Sleep for the delay.
    pub fn wait(&self) {
        if !self.is_zero() {
            std::thread::sleep(self.duration());
        }
    }
}

impl Default for Delay {
    fn default() -> Self {
        Self::none()
    }
}

/// Raised when a delay value is unusable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DelayError {
    /// The value could not be read as a number.
    Invalid(String),
    /// The value exceeds the accepted maximum.
    TooLong { requested: u64, max: u64 },
}

impl std::fmt::Display for DelayError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DelayError::Invalid(v) => write!(
                f,
                "invalid delay {v:?}; expected milliseconds such as 500, or a suffix such as 2s"
            ),
            DelayError::TooLong { requested, max } => {
                write!(f, "delay {requested}ms is longer than the {max}ms maximum")
            }
        }
    }
}

impl std::error::Error for DelayError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_milliseconds_are_accepted() {
        assert_eq!(Delay::parse("500").unwrap().as_millis(), 500);
        assert_eq!(Delay::parse("0").unwrap().as_millis(), 0);
    }

    #[test]
    fn suffixes_are_accepted() {
        assert_eq!(Delay::parse("2s").unwrap().as_millis(), 2000);
        assert_eq!(Delay::parse("250ms").unwrap().as_millis(), 250);
        assert_eq!(Delay::parse("1.5s").unwrap().as_millis(), 1500);
        assert_eq!(Delay::parse("1.5S").unwrap().as_millis(), 1500);
    }

    #[test]
    fn surrounding_whitespace_is_ignored() {
        assert_eq!(Delay::parse("  300  ").unwrap().as_millis(), 300);
    }

    #[test]
    fn invalid_values_are_rejected() {
        for bad in ["", "abc", "12abc", "--", "1s2", "nan", "inf"] {
            assert!(Delay::parse(bad).is_err(), "{bad:?} should be rejected");
        }
    }

    #[test]
    fn negative_values_are_rejected() {
        assert!(Delay::parse("-100").is_err());
        assert!(Delay::parse("-1s").is_err());
    }

    #[test]
    fn an_excessive_delay_is_rejected_with_a_clear_message() {
        let e = Delay::from_millis(MAX_DELAY_MS + 1).unwrap_err();
        let msg = e.to_string();
        assert!(msg.contains("maximum"), "{msg}");
        assert!(Delay::from_millis(MAX_DELAY_MS).is_ok());
    }

    #[test]
    fn zero_is_reported_as_no_delay() {
        let d = Delay::from_millis(0).unwrap();
        assert!(d.is_zero());
        assert_eq!(d.notice(), None);
        assert_eq!(d.duration(), std::time::Duration::ZERO);
    }

    #[test]
    fn a_nonzero_delay_produces_a_notice() {
        // 1500 ms is not a whole number of seconds, so it is shown in milliseconds.
        let d = Delay::from_millis(1500).unwrap();
        let notice = d.notice().expect("notice");
        assert!(notice.contains("1500ms"), "{notice}");
        assert!(notice.contains("front"), "{notice}");

        // A whole number of seconds is shown in seconds.
        let whole = Delay::from_millis(3000).unwrap();
        assert!(whole.notice().unwrap().contains("3s"));
    }

    #[test]
    fn describe_uses_seconds_for_whole_seconds() {
        assert_eq!(Delay::from_millis(2000).unwrap().describe(), "2s");
        assert_eq!(Delay::from_millis(500).unwrap().describe(), "500ms");
        assert_eq!(Delay::from_millis(1500).unwrap().describe(), "1500ms");
    }

    #[test]
    fn waiting_for_zero_returns_immediately() {
        let start = std::time::Instant::now();
        Delay::none().wait();
        assert!(start.elapsed().as_millis() < 50);
    }

    #[test]
    fn waiting_actually_waits() {
        let d = Delay::from_millis(120).unwrap();
        let start = std::time::Instant::now();
        d.wait();
        let elapsed = start.elapsed().as_millis();
        assert!((100..1000).contains(&elapsed), "waited {elapsed} ms");
    }

    #[test]
    fn default_is_no_delay() {
        assert!(Delay::default().is_zero());
    }
}
