//! Integration tests: each gate run end to end against a fixture workspace.

mod support;

use std::path::Path;

use support::{TestError, write};
use xtask::{
    cli::Command,
    commands,
    thresholds::{MutationTarget, Thresholds},
};

const ROOT_MANIFEST: &str = "[workspace]\nresolver = \"3\"\nmembers = [\"crates/*\"]\n\n[workspace.lints.rust]\nunsafe_code = \"forbid\"\n";

const ENGINE_MANIFEST: &str = "[package]\nname = \"quarry-engine\"\nversion = \"0.0.0\"\nedition = \"2024\"\n\n[lints]\nworkspace = true\n";

const ENGINE_SOURCE: &str =
    "//! Engine.\n\n/// Adds one.\npub fn add_one(x: i32) -> i32 {\n    x + 1\n}\n";

/// A one-crate workspace that passes every gate, with the real quality files.
fn fixture() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    write(dir.path(), "Cargo.toml", ROOT_MANIFEST)?;
    write(dir.path(), "crates/engine/Cargo.toml", ENGINE_MANIFEST)?;
    write(dir.path(), "crates/engine/src/lib.rs", ENGINE_SOURCE)?;
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for name in ["quality/thresholds.toml", "quality/exceptions.toml"] {
        write(
            dir.path(),
            name,
            &std::fs::read_to_string(workspace.join(name))?,
        )?;
    }
    Ok(dir)
}

/// Runs `command` on the workspace at `root` and counts the violations.
fn count(command: &Command, root: &Path) -> Result<usize, TestError> {
    Ok(commands::run(command, root)?.len())
}

#[test]
fn repository_thresholds_match_the_design_spec() -> Result<(), TestError> {
    let thresholds = Thresholds::load(&Path::new(env!("CARGO_MANIFEST_DIR")).join(".."))?;
    let limits = thresholds.complexity;
    let actual = (
        limits.cyclomatic,
        limits.cognitive,
        limits.logical_lines,
        limits.physical_lines,
    );
    assert_eq!(actual, (10.0, 15.0, 15.0, 40.0));
    for core in ["engine", "challenges", "store"] {
        let target = thresholds.coverage[core];
        assert_eq!((target.lines, target.regions), (95.0, 90.0), "{core}");
    }
    for ui in ["ui-kit", "app"] {
        let target = thresholds.coverage[ui];
        assert_eq!((target.lines, target.regions), (80.0, 70.0), "{ui}");
    }
    assert_eq!(thresholds.mutation, MutationTarget { min_score: 90.0 });
    Ok(())
}

#[test]
fn names_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    assert_eq!(count(&Command::CheckNames, dir.path())?, 0);
    write(dir.path(), "crates/engine/src/utils.rs", "")?;
    assert_eq!(count(&Command::CheckNames, dir.path())?, 1);
    Ok(())
}

#[test]
fn manifests_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    assert_eq!(count(&Command::CheckManifests, dir.path())?, 0);
    let opted_out = ENGINE_MANIFEST.replace("workspace = true", "workspace = false");
    write(dir.path(), "crates/engine/Cargo.toml", &opted_out)?;
    assert_eq!(count(&Command::CheckManifests, dir.path())?, 1);
    Ok(())
}
