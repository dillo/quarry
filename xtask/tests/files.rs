//! Integration tests for file discovery and process execution.

mod support;

use std::path::{Path, PathBuf};

use support::{TestError, write};
use xtask::{files, process};

#[test]
fn rust_files_lists_only_rust_sources_under_source_roots() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    write(dir.path(), "crates/engine/src/lib.rs", "")?;
    write(dir.path(), "crates/engine/src/value/mod.rs", "")?;
    write(dir.path(), "crates/engine/Cargo.toml", "")?;
    write(dir.path(), "xtask/src/main.rs", "")?;
    write(dir.path(), "spikes/demo/src/main.rs", "")?;
    let expected: Vec<PathBuf> = [
        "crates/engine/src/lib.rs",
        "crates/engine/src/value/mod.rs",
        "xtask/src/main.rs",
    ]
    .iter()
    .map(PathBuf::from)
    .collect();
    assert_eq!(files::rust_files(dir.path())?, expected);
    Ok(())
}

#[test]
fn member_manifests_lists_xtask_and_each_crate() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    write(dir.path(), "crates/store/Cargo.toml", "")?;
    write(dir.path(), "crates/engine/Cargo.toml", "")?;
    write(dir.path(), "crates/notes.txt", "")?;
    write(dir.path(), "xtask/Cargo.toml", "")?;
    let found: Vec<String> = files::member_manifests(dir.path())?
        .iter()
        .map(|p| files::display(p))
        .collect();
    assert_eq!(
        found,
        [
            "crates/engine/Cargo.toml",
            "crates/store/Cargo.toml",
            "xtask/Cargo.toml"
        ]
    );
    Ok(())
}

#[test]
fn empty_workspace_has_no_sources_or_manifests() -> Result<(), TestError> {
    let dir = tempfile::tempdir()?;
    assert!(files::rust_files(dir.path())?.is_empty());
    assert!(files::member_manifests(dir.path())?.is_empty());
    Ok(())
}

#[test]
fn read_reports_missing_file() {
    let error = files::read(Path::new("/definitely/not/here.toml"));
    assert!(error.is_err_and(|e| e.to_string().contains("/definitely/not/here.toml")));
}

#[test]
fn run_capture_returns_stdout() -> Result<(), TestError> {
    let output = process::run_capture("cargo", &["--version"], Path::new("."))?;
    assert!(output.starts_with("cargo "));
    Ok(())
}

#[test]
fn run_capture_reports_failures() {
    let missing = process::run_capture("no-such-program-for-xtask", &[], Path::new("."));
    assert!(missing.is_err());
    let failing = process::run_capture("cargo", &["no-such-subcommand-for-xtask"], Path::new("."));
    assert!(failing.is_err_and(|e| e.to_string().contains("no-such-subcommand-for-xtask")));
}
