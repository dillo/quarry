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
        Command::CoverageGate { report } => coverage(root, report),
        Command::MutationGate { outcomes } => mutation(root, outcomes),
    }
}

fn manifests(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut found = Vec::new();
    for path in files::member_manifests(root)? {
        found.push((files::display(&path), files::read(&root.join(&path))?));
    }
    let mut violations = checks::manifests::check(&found);
    let root_text = files::display(&root.canonicalize().map_err(|source| XtaskError::Io {
        path: files::display(root),
        source,
    })?);
    violations.extend(checks::manifests::check_members(
        &metadata(root)?,
        &root_text,
        &files::SOURCE_ROOTS,
    )?);
    Ok(violations)
}

fn metadata(root: &Path) -> Result<String, XtaskError> {
    let args = ["metadata", "--format-version", "1", "--no-deps"];
    process::run_capture("cargo", &args, root)
}

fn deps(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    checks::deps::check_metadata(&metadata(root)?)
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

fn coverage(root: &Path, report: &Path) -> Result<Vec<Violation>, XtaskError> {
    let targets = Thresholds::load(root)?.coverage;
    checks::coverage::check_report(&files::read(report)?, root, &targets)
}

fn mutation(root: &Path, outcomes: &Path) -> Result<Vec<Violation>, XtaskError> {
    let target = Thresholds::load(root)?.mutation;
    checks::mutation::check_outcomes(&files::read(outcomes)?, &target)
}

fn registry(root: &Path) -> Result<Vec<Exception>, XtaskError> {
    let path = root.join(EXCEPTIONS_PATH);
    checks::exceptions::parse_registry(&files::read(&path)?, &files::display(&path))
}
