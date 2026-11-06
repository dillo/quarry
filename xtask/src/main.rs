//! Entry point: parses the gate to run, runs it, and reports violations.

use std::{path::Path, process::ExitCode};

use clap::Parser;
use xtask::{cli::Cli, commands};

fn main() -> ExitCode {
    let cli = Cli::parse();
    let root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap_or(Path::new("."));
    match commands::run(&cli.command, root) {
        Ok(violations) if violations.is_empty() => ExitCode::SUCCESS,
        Ok(violations) => {
            for violation in &violations {
                eprintln!("✗ {violation}");
            }
            eprintln!("{} violation(s)", violations.len());
            ExitCode::FAILURE
        }
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(2)
        }
    }
}
