//! The aurora pulse: a brief glow of light at the window's edges that
//! rises quickly and fades.

use std::time::{Duration, Instant};

/// How far into the pulse it is brightest.
const PEAK: f32 = 0.25;

/// One pulse, from when it started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pulse {
    started: Instant,
    duration: Duration,
}

impl Pulse {
    /// A pulse starting at `started` and lasting `duration`.
    pub fn new(started: Instant, duration: Duration) -> Self {
        Self { started, duration }
    }

    /// How bright it is at `now`, from 0 to 1: rising to 1 a quarter of
    /// the way through, then fading back to 0 at the end.
    pub fn opacity(&self, now: Instant) -> f32 {
        let t = self.progress(now);
        if t < PEAK {
            t / PEAK
        } else {
            (1.0 - t) / (1.0 - PEAK)
        }
    }

    /// Whether the pulse has finished at `now`.
    pub fn is_done(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }

    fn progress(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        let elapsed = now.saturating_duration_since(self.started);
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::Pulse;

    fn at(pulse: &Pulse, start: Instant, millis: u64) -> f32 {
        pulse.opacity(start + Duration::from_millis(millis))
    }

    #[test]
    fn rises_to_its_peak_then_fades_to_nothing() {
        let start = Instant::now();
        let pulse = Pulse::new(start, Duration::from_millis(400));
        assert!(at(&pulse, start, 0).abs() < 1e-6);
        assert!((at(&pulse, start, 50) - 0.5).abs() < 1e-3);
        assert!((at(&pulse, start, 100) - 1.0).abs() < 1e-3);
        assert!((at(&pulse, start, 250) - 0.5).abs() < 1e-3);
        assert!(at(&pulse, start, 400).abs() < 1e-6);
        assert!(at(&pulse, start, 900).abs() < 1e-6);
    }

    #[test]
    fn is_done_once_its_time_is_up() {
        let start = Instant::now();
        let pulse = Pulse::new(start, Duration::from_millis(400));
        assert!(!pulse.is_done(start + Duration::from_millis(399)));
        assert!(pulse.is_done(start + Duration::from_millis(400)));
        let instant = Pulse::new(start, Duration::ZERO);
        assert!(instant.is_done(start));
        assert!(instant.opacity(start).abs() < 1e-6);
    }
}
