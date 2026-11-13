//! The timed cross-fade between two palettes.

use std::time::{Duration, Instant};

use crate::palette::Colors;

/// A cross-fade from one set of colors to another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    from: Colors,
    to: Colors,
    started: Instant,
    duration: Duration,
}

impl Transition {
    /// Starts fading at `started`; a zero `duration` switches instantly.
    pub fn new(from: Colors, to: Colors, started: Instant, duration: Duration) -> Self {
        Self {
            from,
            to,
            started,
            duration,
        }
    }

    /// The colors to draw at `now`.
    pub fn colors(&self, now: Instant) -> Colors {
        Colors::mix(&self.from, &self.to, ease_out(self.progress(now)))
    }

    /// Whether the fade has finished at `now`.
    pub fn is_done(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }

    /// The colors the fade ends on.
    pub fn target(&self) -> Colors {
        self.to
    }

    fn progress(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        let elapsed = now.saturating_duration_since(self.started);
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }
}

/// Cubic ease-out: fast start, gentle landing.
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Transition, ease_out};
    use crate::palette::{Colors, Palette};

    fn fade(duration: Duration) -> (Transition, Instant, Colors, Colors) {
        let (from, to) = (Palette::aurora().colors(), Palette::dawn().colors());
        let start = Instant::now();
        (Transition::new(from, to, start, duration), start, from, to)
    }

    #[test]
    fn fades_from_start_to_target_over_its_duration() {
        let (fade, start, from, to) = fade(Duration::from_millis(200));
        assert_eq!(fade.colors(start), from);
        assert!(!fade.is_done(start + Duration::from_millis(100)));
        let midway = fade.colors(start + Duration::from_millis(100));
        assert_eq!(midway, Colors::mix(&from, &to, ease_out(0.5)));
        assert_eq!(fade.colors(start + Duration::from_millis(500)), to);
        assert!(fade.is_done(start + Duration::from_millis(200)));
        assert_eq!(fade.target(), to);
    }

    #[test]
    fn a_zero_duration_switches_instantly() {
        let (fade, start, _, to) = fade(Duration::ZERO);
        assert_eq!(fade.colors(start), to);
        assert!(fade.is_done(start));
    }

    #[test]
    fn times_before_the_start_show_the_start_colors() {
        let (fade, start, from, _) = fade(Duration::from_millis(200));
        let earlier = start
            .checked_sub(Duration::from_millis(50))
            .unwrap_or(start);
        assert_eq!(fade.colors(earlier), from);
    }

    #[test]
    fn ease_out_is_monotonic_with_fixed_ends() {
        assert!((ease_out(0.0)).abs() < f32::EPSILON);
        assert!((ease_out(1.0) - 1.0).abs() < f32::EPSILON);
        assert!(ease_out(0.5) > 0.5, "ease-out runs ahead of linear");
        assert!(ease_out(0.25) < ease_out(0.75));
    }
}
