//! The command-line interface: one subcommand per quality gate.

use std::path::PathBuf;

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
    /// Enforce per-function size and complexity limits.
    Complexity,
    /// Enforce per-crate coverage thresholds.
    CoverageGate {
        /// Path to the `cargo llvm-cov --json --summary-only` report.
        #[arg(long)]
        report: PathBuf,
    },
    /// Enforce the minimum mutation score.
    MutationGate {
        /// Path to `mutants.out/outcomes.json`.
        #[arg(long)]
        outcomes: PathBuf,
    },
}
