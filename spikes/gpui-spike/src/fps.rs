//! Counts rendered frames per second.

use std::time::{Duration, Instant};

pub struct FrameCounter {
    window_start: Instant,
    frames: u32,
    last_fps: u32,
}

impl FrameCounter {
    pub fn new() -> Self {
        Self { window_start: Instant::now(), frames: 0, last_fps: 0 }
    }

    /// Records one frame; returns the frames counted in the last full second.
    pub fn tick(&mut self) -> u32 {
        self.frames += 1;
        if self.window_start.elapsed() >= Duration::from_secs(1) {
            self.last_fps = self.frames;
            self.frames = 0;
            self.window_start = Instant::now();
        }
        self.last_fps
    }
}
