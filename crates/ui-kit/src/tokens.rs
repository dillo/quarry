//! Non-color design tokens: type, spacing, radii and motion.

use std::time::Duration;

/// The interface font: SF Pro, through GPUI's system-font alias.
pub const UI_FONT: &str = ".SystemUIFont";

/// The code font. SF Mono is not loadable by name, so Quarry uses Menlo,
/// which ships with every Mac (see ADR 0008).
pub const MONO_FONT: &str = "Menlo";

/// Type scale in points.
pub mod text {
    /// Captions, badges, keyboard hints.
    pub const CAPTION: f32 = 11.0;
    /// Sidebar items, tabs, status bar.
    pub const SMALL: f32 = 12.0;
    /// Body text and the editor.
    pub const BODY: f32 = 13.0;
    /// Panel titles.
    pub const TITLE: f32 = 17.0;
    /// The Home greeting.
    pub const DISPLAY: f32 = 24.0;
}

/// Spacing on a 4-point grid.
pub mod space {
    /// 4 points.
    pub const XS: f32 = 4.0;
    /// 8 points.
    pub const SM: f32 = 8.0;
    /// 12 points.
    pub const MD: f32 = 12.0;
    /// 16 points.
    pub const LG: f32 = 16.0;
    /// 24 points.
    pub const XL: f32 = 24.0;
}

/// Corner radii.
pub mod radius {
    /// Items, buttons, badges.
    pub const SM: f32 = 6.0;
    /// Panels.
    pub const MD: f32 = 10.0;
    /// Cards and the window.
    pub const LG: f32 = 14.0;
    /// Pills (badges, segmented control thumb).
    pub const PILL: f32 = 999.0;
}

/// Window chrome.
pub mod chrome {
    /// Height reserved at the top of the window for the traffic lights.
    pub const TITLEBAR_HEIGHT: f32 = 40.0;
    /// Where the traffic lights sit, from the window's top-left corner.
    pub const TRAFFIC_LIGHTS: (f32, f32) = (14.0, 14.0);
}

/// Animation durations.
pub mod motion {
    use super::Duration;

    /// Hover and press feedback.
    pub const FAST: Duration = Duration::from_millis(150);
    /// The theme cross-fade.
    pub const THEME: Duration = Duration::from_millis(250);
}
