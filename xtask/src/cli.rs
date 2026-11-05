//! The command-line interface: one subcommand per quality gate.

use clap::{Parser, Subcommand};

/// Quarry workspace quality gates.
#[derive(Debug, Parser)]
#[command(name = "xtask")]
pub struct Cli {
    /// The gate to run.
    #[command(subcommand)]
    pub command: Command,
}

/// One quality gate.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Reject catch-all module names such as `utils.rs`.
    CheckNames,
    /// Require every crate manifest to inherit the workspace lints.
    CheckManifests,
    /// Enforce the crate dependency direction.
    CheckDeps,
    /// Require every lint suppression to be registered with a reason.
    CheckExceptions,
}
