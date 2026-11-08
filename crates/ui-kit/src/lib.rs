//! The Aurora design system: theme tokens and reusable components.
//!
//! Call [`Aurora::install`] once after `gpui_kit::init`; components then read
//! the current colors with [`Aurora::colors`]. Views that render while
//! [`Aurora::is_transitioning`] should call `window.request_animation_frame()`
//! so the theme cross-fade animates.

pub mod appearance;
pub mod aurora;
pub mod color;
pub mod component_theme;
pub mod palette;
pub mod tokens;
pub mod transition;

pub use appearance::{Appearance, Preference};
pub use aurora::Aurora;
pub use palette::{Colors, Palette, Role};
