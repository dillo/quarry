//! Integration tests: each gate run end to end against a fixture workspace.

mod support;

use std::path::{Path, PathBuf};

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
    let opted_out = ENGINE_MANIFEST.replace("\n[lints]\nworkspace = true\n", "");
    write(dir.path(), "crates/engine/Cargo.toml", &opted_out)?;
    assert_eq!(count(&Command::CheckManifests, dir.path())?, 1);
    Ok(())
}

#[test]
fn manifests_gate_flags_members_outside_the_scanned_roots() -> Result<(), TestError> {
    let dir = fixture()?;
    let root = ROOT_MANIFEST.replace("\"crates/*\"", "\"crates/*\", \"tools/foo\"");
    write(dir.path(), "Cargo.toml", &root)?;
    let tool = ENGINE_MANIFEST.replace("quarry-engine", "quarry-foo");
    write(dir.path(), "tools/foo/Cargo.toml", &tool)?;
    write(dir.path(), "tools/foo/src/lib.rs", "//! Foo.\n")?;
    let found = commands::run(&Command::CheckManifests, dir.path())?;
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].location, "tools/foo/Cargo.toml");
    Ok(())
}

#[test]
fn deps_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    assert_eq!(count(&Command::CheckDeps, dir.path())?, 0);
    let ui_kit = ENGINE_MANIFEST.replace("quarry-engine", "quarry-ui-kit");
    write(dir.path(), "crates/ui-kit/Cargo.toml", &ui_kit)?;
    write(dir.path(), "crates/ui-kit/src/lib.rs", "//! UI.\n")?;
    let reversed =
        format!("{ENGINE_MANIFEST}\n[dependencies]\nquarry-ui-kit = {{ path = \"../ui-kit\" }}\n");
    write(dir.path(), "crates/engine/Cargo.toml", &reversed)?;
    assert_eq!(count(&Command::CheckDeps, dir.path())?, 1);
    Ok(())
}

#[test]
fn exceptions_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    assert_eq!(count(&Command::CheckExceptions, dir.path())?, 0);
    // Built with format! so this file does not trip the gate it is testing.
    let suppression = format!("#![{}(dead_code)]\n{ENGINE_SOURCE}", "allow");
    write(dir.path(), "crates/engine/src/lib.rs", &suppression)?;
    assert_eq!(count(&Command::CheckExceptions, dir.path())?, 1);
    let empty = tempfile::tempdir()?;
    assert!(commands::run(&Command::CheckExceptions, empty.path()).is_err());
    Ok(())
}

#[test]
fn complexity_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    assert_eq!(count(&Command::Complexity, dir.path())?, 0);
    let arms = (0..12)
        .map(|n| format!("        {n} => {n},\n"))
        .collect::<Vec<_>>()
        .concat();
    let branchy = format!(
        "//! E.\n\n/// M.\npub fn map(x: i32) -> i32 {{\n    match x {{\n{arms}        _ => 0,\n    }}\n}}\n"
    );
    write(dir.path(), "crates/engine/src/lib.rs", &branchy)?;
    assert!(count(&Command::Complexity, dir.path())? >= 1);
    Ok(())
}

#[test]
fn coverage_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    let file = |name: &str, covered: u32| {
        format!(
            r#"{{"filename":"{name}","summary":{{"lines":{{"count":10,"covered":{covered}}},"regions":{{"count":10,"covered":{covered}}}}}}}"#
        )
    };
    let engine = dir
        .path()
        .join("crates/engine/src/lib.rs")
        .display()
        .to_string();
    write(
        dir.path(),
        "cov.json",
        &format!(r#"{{"data":[{{"files":[{}]}}]}}"#, file(&engine, 10)),
    )?;
    let gate = Command::CoverageGate {
        report: dir.path().join("cov.json"),
    };
    assert_eq!(count(&gate, dir.path())?, 0);
    write(
        dir.path(),
        "cov.json",
        &format!(r#"{{"data":[{{"files":[{}]}}]}}"#, file(&engine, 5)),
    )?;
    assert_eq!(count(&gate, dir.path())?, 2);
    let missing = Command::CoverageGate {
        report: PathBuf::from("/no/such/report.json"),
    };
    assert!(commands::run(&missing, dir.path()).is_err());
    Ok(())
}

#[test]
fn mutation_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    let gate = Command::MutationGate {
        outcomes: dir.path().join("outcomes.json"),
    };
    write(
        dir.path(),
        "outcomes.json",
        r#"{"caught":9,"missed":1,"timeout":0}"#,
    )?;
    assert_eq!(count(&gate, dir.path())?, 0);
    write(
        dir.path(),
        "outcomes.json",
        r#"{"caught":1,"missed":9,"timeout":0}"#,
    )?;
    assert_eq!(count(&gate, dir.path())?, 1);
    Ok(())
}
