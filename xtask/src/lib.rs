//! Quality gates for the Quarry workspace.
//!
//! Each gate is a pure function in [`checks`]; [`commands`] does the I/O that
//! feeds it (reading files, running tools) and returns the violations found.

pub mod checks;
pub mod cli;
pub mod commands;
pub mod error;
pub mod files;
pub mod process;
pub mod thresholds;
pub mod violation;
