//! View models: plain data and pure functions the views render.
//!
//! Nothing here depends on GPUI, so every rule is unit-tested and
//! mutation-tested like the core crates.

pub mod drain;
pub mod format;
pub mod messages;
pub mod plan_tree;
pub mod schema_tree;
pub mod statements;
pub mod status;
pub mod tabs;
pub mod target;
