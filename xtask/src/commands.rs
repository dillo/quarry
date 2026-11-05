//! The I/O half of each gate: gathers inputs, then calls the pure check.

use std::path::Path;

use crate::{checks, cli::Command, error::XtaskError, files, process, violation::Violation};

/// Runs one gate against the workspace at `root`.
///
/// # Errors
/// Returns an error when an input cannot be read or a tool fails to run.
pub fn run(command: &Command, root: &Path) -> Result<Vec<Violation>, XtaskError> {
    match command {
        Command::CheckNames => Ok(checks::names::check_paths(&files::rust_files(root)?)),
        Command::CheckManifests => manifests(root),
        Command::CheckDeps => deps(root),
    }
}

fn manifests(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut found = Vec::new();
    for path in files::member_manifests(root)? {
        found.push((files::display(&path), files::read(&root.join(&path))?));
    }
    Ok(checks::manifests::check(&found))
}

fn deps(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let args = ["metadata", "--format-version", "1", "--no-deps"];
    checks::deps::check_metadata(&process::run_capture("cargo", &args, root)?)
}
