//! The I/O half of each gate: gathers inputs, then calls the pure check.

use std::path::Path;

use crate::{checks, cli::Command, error::XtaskError, files, violation::Violation};

/// Runs one gate against the workspace at `root`.
///
/// # Errors
/// Returns an error when an input cannot be read or a tool fails to run.
pub fn run(command: &Command, root: &Path) -> Result<Vec<Violation>, XtaskError> {
    match command {
        Command::CheckNames => Ok(checks::names::check_paths(&files::rust_files(root)?)),
    }
}
