//! The I/O half of each gate: gathers inputs, then calls the pure check.

use std::path::Path;

use crate::{
    checks::{self, exceptions::Exception},
    cli::Command,
    error::XtaskError,
    files, process,
    thresholds::Thresholds,
    violation::Violation,
};

/// Location of the exceptions registry, relative to the workspace root.
pub const EXCEPTIONS_PATH: &str = "quality/exceptions.toml";

/// Runs one gate against the workspace at `root`.
///
/// # Errors
/// Returns an error when an input cannot be read or a tool fails to run.
pub fn run(command: &Command, root: &Path) -> Result<Vec<Violation>, XtaskError> {
    match command {
        Command::CheckNames => Ok(checks::names::check_paths(&files::rust_files(root)?)),
        Command::CheckManifests => manifests(root),
        Command::CheckDeps => deps(root),
        Command::CheckExceptions => exceptions(root),
        Command::Complexity => complexity(root),
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

fn exceptions(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let scanner = checks::exceptions::Scanner::new()?;
    let mut found = Vec::new();
    for path in files::rust_files(root)? {
        let source = files::read(&root.join(&path))?;
        found.extend(scanner.find(&files::display(&path), &source));
    }
    Ok(checks::exceptions::check(&found, &registry(root)?))
}

fn complexity(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let limits = Thresholds::load(root)?.complexity;
    let mut report = String::new();
    for dir in files::source_dirs(root) {
        let dir = files::display(&dir);
        let args = ["-m", "-O", "json", "-p", dir.as_str()];
        report.push_str(&process::run_capture(
            "rust-code-analysis-cli",
            &args,
            root,
        )?);
        report.push('\n');
    }
    checks::complexity::check_report(&report, root, &limits, &registry(root)?)
}

fn registry(root: &Path) -> Result<Vec<Exception>, XtaskError> {
    let path = root.join(EXCEPTIONS_PATH);
    checks::exceptions::parse_registry(&files::read(&path)?, &files::display(&path))
}
