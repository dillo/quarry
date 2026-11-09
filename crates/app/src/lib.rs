//! Quarry: a native macOS SQL client for learning and real work.
//!
//! [`model`] holds the GPUI-free view models; [`views`] renders them;
//! [`shell`] wires actions, key bindings, menus and the window.

pub mod model;
pub mod sample;
pub mod shell;
pub mod views;
