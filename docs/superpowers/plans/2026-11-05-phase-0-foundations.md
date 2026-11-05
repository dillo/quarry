# Phase 0: Foundations and GPUI Spike Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Stand up the Quarry Cargo workspace with every quality gate from the design spec enforced locally and in CI, and run a throwaway GPUI spike that confirms (or rules out) frosted glass, a completion-free SQL editor, and a 1M-row virtualized table.

**Architecture:**
- **Workspace:** five empty product crates (`app`, `ui-kit`, `engine`, `challenges`, `store`) plus an `xtask` crate. `xtask` implements the workspace-specific gates in Rust. Each gate is a pure `checks::<gate>` module fed by a thin I/O layer in `commands.rs`, and adding a gate is itself an extension point.
- **Tooling:** generic tools (clippy, cargo-deny, llvm-cov, cargo-mutants, typos, machete) are wired together by a `justfile`, so `just check` is identical to CI.
- **Spike:** a separate, ungated Cargo project under `spikes/`.

**Tech Stack:**
- Rust 1.98.1 (edition 2024).
- `xtask` crate: clap 4, serde, serde_json, toml 1, regex, walkdir, thiserror 2, tempfile (dev).
- Gate tools: cargo-nextest, cargo-llvm-cov, cargo-deny, cargo-machete, typos, cargo-mutants, rust-code-analysis-cli 0.0.25.
- Workflow: just, lefthook, GitHub Actions (`macos-14`).
- Spike: `gpui-kit` 0.9.0.

**Spec:** `docs/superpowers/specs/2026-11-04-quarry-design.md` (§3.1–3.3, §7, §9 Phase 0, §10).

## Global Constraints

- Platform: macOS 14+, Apple Silicon (arm64) only. CI runner: `macos-14`.
- Toolchain pinned in `rust-toolchain.toml`: `1.98.1`, components `rustfmt`, `clippy`, `llvm-tools`.
- Package names: `quarry` (binary, in `crates/app`), `quarry-ui-kit`, `quarry-engine`, `quarry-challenges`, `quarry-store`, `xtask`. All `publish = false`.
- Dependency direction (spec §3.2):
  - `quarry` → `quarry-ui-kit`, `quarry-challenges`, `quarry-store`, `quarry-engine`
  - `quarry-challenges` → `quarry-engine`
  - `quarry-store` → `quarry-engine`
  - `quarry-engine`, `quarry-ui-kit` and `xtask` → no workspace crates
- Per-function limits:
  - Cyclomatic ≤ 10, cognitive ≤ 15, logical lines (statements) ≤ 15, physical lines ≤ 40.
  - Clippy early warnings: nesting ≤ 4, arguments ≤ 5, type complexity ≤ 250.
  - **These apply to test functions too.**
- Coverage (lines / regions):
  - `engine`, `challenges`, `store`: 95 / 90.
  - `ui-kit`, `app`: 80 / 70.
  - `xtask`: 90 / 85. This is a plan-level decision for tooling code, which the spec does not cover. Record it in ADR 0004.
- Mutation score ≥ 90% on `quarry-engine`, `quarry-challenges` and `quarry-store`.
- Banned outside tests: `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`.
- `unsafe_code` is forbidden in every crate.
- Library errors use `thiserror`. `anyhow` is used only in `crates/app`, and isn't needed in Phase 0.
- Every lint suppression attribute must be registered in `quality/exceptions.toml` with a reason.
- Module names `utils`, `helpers`, `misc` and `common` are forbidden (single responsibility, spec §7.1).
- Every crate manifest must contain `[lints] workspace = true`.
- Tests return `Result<(), Box<dyn std::error::Error>>` and use `?`, so test helpers never need `unwrap`.
- Test fixtures must never contain a literal lint-suppression attribute. Build them with `format!` (see Task 5), or the exceptions gate will flag its own tests.
- **No autocomplete or completion provider, ever** (spec §1). The spike must prove the editor shows no completions.
- Work on branch `phase-0-foundations` (Tasks 1–12) and `phase-0-spike` (Tasks 13–16). Use Conventional Commit messages.

---

## File Structure

```
quarry/
├─ rust-toolchain.toml        toolchain pin (Task 1)
├─ Cargo.toml                 workspace, shared deps, workspace lints (Tasks 1–2)
├─ clippy.toml rustfmt.toml   lint thresholds, formatting (Task 1)
├─ .cargo/config.toml         `cargo xtask` alias (Task 2)
├─ crates/
│  ├─ app/src/main.rs         `quarry` binary, empty main (Task 1)
│  ├─ ui-kit/src/lib.rs       crate docs only (Task 1)
│  ├─ engine/src/lib.rs       crate docs only (Task 1)
│  ├─ challenges/src/lib.rs   crate docs only (Task 1)
│  └─ store/src/lib.rs        crate docs only (Task 1)
├─ xtask/
│  ├─ src/main.rs             parse → run → print → exit code (Task 2)
│  ├─ src/lib.rs              module list (Task 2)
│  ├─ src/cli.rs              one subcommand per gate (Tasks 2–8)
│  ├─ src/commands.rs         I/O half of each gate (Tasks 2–8)
│  ├─ src/error.rs            XtaskError (Task 2)
│  ├─ src/violation.rs        Violation (Task 2)
│  ├─ src/files.rs            finding sources and manifests (Task 2)
│  ├─ src/process.rs          running external tools (Task 2)
│  ├─ src/thresholds.rs       numeric thresholds + loader (Task 2)
│  ├─ src/checks/mod.rs       gate module list (Tasks 2–8)
│  ├─ src/checks/names.rs     (Task 2)   manifests.rs (Task 3)  deps.rs (Task 4)
│  ├─ src/checks/exceptions.rs (Task 5)  complexity.rs (Task 6)
│  ├─ src/checks/coverage.rs  (Task 7)   mutation.rs (Task 8)
│  └─ tests/                  support/mod.rs, files.rs, commands.rs (Tasks 2–8)
├─ quality/thresholds.toml    every numeric threshold (Task 2)
├─ quality/exceptions.toml    registered suppressions (Task 2)
├─ deny.toml _typos.toml justfile lefthook.yml (Task 9)
├─ .github/                   workflows, dependabot, PR template (Task 10)
├─ README.md CONTRIBUTING.md docs/adr/ (Task 11)
└─ spikes/gpui-spike/         throwaway spike, own workspace (Tasks 13–15)
   docs/spikes/phase-0-gpui.md, docs/adr/0006-*.md (Task 16)
```

---

### Task 1: Workspace skeleton, toolchain and lint configuration

**Files:**
- Create: `rust-toolchain.toml`, `Cargo.toml`, `clippy.toml`, `rustfmt.toml`
- Create: `crates/{engine,challenges,store,ui-kit}/Cargo.toml` and `src/lib.rs`
- Create: `crates/app/Cargo.toml`, `crates/app/src/main.rs`
- Modify: `.gitignore`

**Interfaces:**
- Consumes: nothing.
- Produces: package names `quarry`, `quarry-engine`, `quarry-challenges`, `quarry-store` and `quarry-ui-kit`; the `[workspace.lints]` tables, which every later crate inherits with `[lints] workspace = true`; and the `[workspace.package]` fields `version`, `edition`, `rust-version` and `publish`.

- [ ] **Step 1: Create the branch**

```bash
cd ~/Dev/projects/quarry
git checkout -b phase-0-foundations
```

- [ ] **Step 2: Pin the toolchain.** Create `rust-toolchain.toml`:

```toml
[toolchain]
channel = "1.98.1"
components = ["rustfmt", "clippy", "llvm-tools"]
profile = "minimal"
```

- [ ] **Step 3: Create the workspace manifest.** Create `Cargo.toml`. It doesn't list `xtask` or `[workspace.dependencies]` yet; Task 2 adds both.

```toml
[workspace]
resolver = "3"
members = ["crates/*"]
exclude = ["spikes"]

[workspace.package]
version = "0.0.0"
edition = "2024"
rust-version = "1.98"
publish = false

[workspace.lints.rust]
unsafe_code = "forbid"
missing_docs = "deny"

[workspace.lints.clippy]
# Pedantic is the baseline. The two allows below are workspace-wide policy
# decisions (reviewed here, not per-site): `must_use_candidate` fires on almost
# every pure function, and `module_name_repetitions` fights clear naming such
# as `error::XtaskError`.
pedantic = { level = "deny", priority = -1 }
must_use_candidate = "allow"
module_name_repetitions = "allow"
# Restriction lints from the design spec §7.1.
unwrap_used = "deny"
expect_used = "deny"
panic = "deny"
todo = "deny"
unimplemented = "deny"
dbg_macro = "deny"
# Size and complexity limits (thresholds live in clippy.toml).
cognitive_complexity = "deny"
excessive_nesting = "deny"
too_many_arguments = "deny"
type_complexity = "deny"

[profile.dev]
debug = "line-tables-only"
```

- [ ] **Step 4: Create `clippy.toml` and `rustfmt.toml`**

`clippy.toml`:
```toml
# Early-warning limits. The authoritative gate is `cargo xtask complexity`,
# which reads quality/thresholds.toml.
cognitive-complexity-threshold = 15
too-many-lines-threshold = 40
too-many-arguments-threshold = 5
excessive-nesting-threshold = 4
type-complexity-threshold = 250
allow-unwrap-in-tests = true
allow-expect-in-tests = true
allow-panic-in-tests = true
allow-dbg-in-tests = true
```

`rustfmt.toml`:
```toml
edition = "2024"
max_width = 100
```

- [ ] **Step 5: Create the four library crates.** For each of `engine`, `challenges`, `store` and `ui-kit`, create `crates/<dir>/Cargo.toml`. This one is for `engine`; replace `quarry-engine` with `quarry-challenges`, `quarry-store` or `quarry-ui-kit` for the others:

```toml
[package]
name = "quarry-engine"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[lints]
workspace = true
```

Then create each `src/lib.rs`. Each one contains only its crate doc line:

- `crates/engine/src/lib.rs`: `//! Database layer: the `Driver` trait, the shared value model, and drivers.`
- `crates/challenges/src/lib.rs`: `//! SQL challenge tracks: loading, sandboxes, grading, and hints.`
- `crates/store/src/lib.rs`: `//! Local persistence: settings, connections, history, and progress.`
- `crates/ui-kit/src/lib.rs`: `//! The Aurora design system: theme tokens and reusable components.`

- [ ] **Step 6: Create the app crate**

`crates/app/Cargo.toml`:
```toml
[package]
name = "quarry"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[[bin]]
name = "quarry"
path = "src/main.rs"

[lints]
workspace = true
```

`crates/app/src/main.rs`:
```rust
//! Quarry: a native macOS SQL client for learning and real work.

fn main() {}
```

- [ ] **Step 7: Extend `.gitignore`.** Replace its contents with:

```
.superpowers/
/target
mutants.out*/
spikes/*/target
```

- [ ] **Step 8: Verify that the build, format and lint checks pass**

Run: `cargo build --workspace && cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: all three succeed with no warnings. The first run downloads toolchain 1.98.1 if it's missing.

- [ ] **Step 9: Prove the lints are enforced.** Temporarily add this line inside `main` in `crates/app/src/main.rs`:

```rust
    let _ = Some(1).unwrap();
```

Run: `cargo clippy --workspace --all-targets -- -D warnings`
Expected: FAIL with `used `unwrap()` on an `Option` value` (`clippy::unwrap_used`). Remove the line, then run clippy again. Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add rust-toolchain.toml Cargo.toml Cargo.lock clippy.toml rustfmt.toml crates .gitignore
git commit -m "build: add workspace skeleton with pinned toolchain and strict lints"
```

---

### Task 2: xtask foundation and the module-name gate

`xtask` is the home of every workspace-specific gate. This task builds its shared parts (errors, violations, file discovery, process running and thresholds) plus the first gate, `check-names`. Every later gate is added the same way: one `checks/` module, one `Command` variant, one `commands.rs` arm. That's the extension-point rule applied to our own tooling.

**Files:**
- Modify: `Cargo.toml` (members, `[workspace.dependencies]`)
- Create: `.cargo/config.toml`, `quality/thresholds.toml`, `quality/exceptions.toml`
- Create: `xtask/Cargo.toml`, `xtask/src/{main,lib,cli,commands,error,violation,files,process,thresholds}.rs`
- Create: `xtask/src/checks/mod.rs`, `xtask/src/checks/names.rs`
- Test: `xtask/tests/support/mod.rs`, `xtask/tests/files.rs`, `xtask/tests/commands.rs`

**Interfaces:**
- Produces, used by every later task:
  - `xtask::violation::Violation { location: String, message: String }` and `Violation::new(location: impl Into<String>, message: impl Into<String>) -> Violation`
  - `xtask::error::XtaskError`, with variants `Io`, `Walk`, `Json { what, source }`, `Toml { path, source }`, `Command { command, detail }` and `Pattern(regex::Error)`
  - `files::source_dirs(&Path) -> Vec<PathBuf>`, `files::rust_files(&Path) -> Result<Vec<PathBuf>, XtaskError>`, `files::member_manifests(&Path) -> Result<Vec<PathBuf>, XtaskError>`, `files::read(&Path) -> Result<String, XtaskError>` and `files::display(&Path) -> String`
  - `process::run_capture(program: &str, args: &[&str], cwd: &Path) -> Result<String, XtaskError>`
  - `thresholds::{Thresholds, Limits, CoverageTarget, MutationTarget}` and `Thresholds::load(root: &Path) -> Result<Thresholds, XtaskError>`
  - `commands::run(command: &cli::Command, root: &Path) -> Result<Vec<Violation>, XtaskError>`
  - The test helpers `fixture()` and `count()` in `xtask/tests/commands.rs`

- [ ] **Step 1: Register xtask in the workspace.** In `Cargo.toml`, change `members` and add the shared dependencies after `[workspace.package]`:

```toml
members = ["crates/*", "xtask"]
```

```toml
[workspace.dependencies]
clap = { version = "4.6.7", features = ["derive"] }
regex = "1.13.1"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0.154"
tempfile = "3.27.0"
thiserror = "2.0.21"
toml = "1.2.0"
walkdir = "2.5.0"
```

- [ ] **Step 2: Create `xtask/Cargo.toml` and the `cargo xtask` alias**

`xtask/Cargo.toml`:
```toml
[package]
name = "xtask"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true

[dependencies]
clap.workspace = true
regex.workspace = true
serde.workspace = true
serde_json.workspace = true
thiserror.workspace = true
toml.workspace = true
walkdir.workspace = true

[dev-dependencies]
tempfile.workspace = true

[lints]
workspace = true
```

`.cargo/config.toml`:
```toml
[alias]
xtask = "run --quiet --package xtask --"
```

- [ ] **Step 3: Create the quality files**

`quality/thresholds.toml`:
```toml
# Every numeric quality gate lives here (design spec §7.2, §7.3).
# Changing a number is a reviewed decision: say why in the PR.

[complexity]          # per function
cyclomatic = 10
cognitive = 15
logical_lines = 15    # statements
physical_lines = 40   # lines containing code

# Coverage targets per crate directory, in percent.
[coverage.engine]
lines = 95.0
regions = 90.0

[coverage.challenges]
lines = 95.0
regions = 90.0

[coverage.store]
lines = 95.0
regions = 90.0

[coverage.ui-kit]
lines = 80.0
regions = 70.0

[coverage.app]
lines = 80.0
regions = 70.0

[coverage.xtask]
lines = 90.0
regions = 85.0

[mutation]
min_score = 90.0      # percent of viable mutants caught (core crates)
```

`quality/exceptions.toml`:
```toml
# Every lint suppression in the workspace, with the reason it is justified.
# `cargo xtask check-exceptions` fails on any suppression missing from this
# file, any entry without a reason, and any entry that no longer matches.
#
# [[exception]]
# file = "crates/engine/src/example.rs"
# lint = "clippy::too_many_lines"
# reason = "Why this is the clearest form of this code."
#
# Waiving the complexity gate for one function (no source attribute needed):
# [[exception]]
# file = "crates/engine/src/example.rs"
# lint = "quarry::complexity"
# function = "parse_literal"
# reason = "Why."
```

- [ ] **Step 4: Write the failing tests first.** Create the test support module, the file and process tests, and the first gate tests.

`xtask/tests/support/mod.rs`:
```rust
//! Shared fixture helpers for the integration tests.

use std::{fs, path::Path};

/// Error type for fallible test helpers.
pub type TestError = Box<dyn std::error::Error>;

/// Writes `contents` to `root/relative`, creating parent directories.
///
/// # Errors
/// Returns an error when a directory or the file cannot be written.
pub fn write(root: &Path, relative: &str, contents: &str) -> Result<(), TestError> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}
```

`xtask/tests/files.rs`:
```rust
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
```

`xtask/tests/commands.rs` (later tasks append one test each):
```rust
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

const ENGINE_SOURCE: &str = "//! Engine.\n\n/// Adds one.\npub fn add_one(x: i32) -> i32 {\n    x + 1\n}\n";

/// A one-crate workspace that passes every gate, with the real quality files.
fn fixture() -> Result<tempfile::TempDir, TestError> {
    let dir = tempfile::tempdir()?;
    write(dir.path(), "Cargo.toml", ROOT_MANIFEST)?;
    write(dir.path(), "crates/engine/Cargo.toml", ENGINE_MANIFEST)?;
    write(dir.path(), "crates/engine/src/lib.rs", ENGINE_SOURCE)?;
    let workspace = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    for name in ["quality/thresholds.toml", "quality/exceptions.toml"] {
        write(dir.path(), name, &std::fs::read_to_string(workspace.join(name))?)?;
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
```

- [ ] **Step 5: Run the tests to verify they fail**

Run: `cargo test -p xtask`
Expected: FAIL to compile with `can't find crate for `xtask`` or unresolved imports. The library doesn't exist yet.

- [ ] **Step 6: Create the shared modules**

`xtask/src/lib.rs`:
```rust
//! Quality gates for the Quarry workspace.
//!
//! Each gate is a pure function in [`checks`]; [`commands`] does the I/O that
//! feeds it (reading files, running tools) and returns the violations found.

pub mod checks;
pub mod cli;
pub mod commands;
pub mod error;
pub mod files;
pub mod process;
pub mod thresholds;
pub mod violation;
```

`xtask/src/main.rs`:
```rust
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
```

`xtask/src/violation.rs`:
```rust
//! The single result type every gate reports.

use std::fmt;

/// One broken rule, with where it happened and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Where the rule was broken: a path, `path:line`, or a crate name.
    pub location: String,
    /// What is wrong, phrased so the reader knows what to change.
    pub message: String,
}

impl Violation {
    /// Creates a violation at `location` with `message`.
    pub fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::Violation;

    #[test]
    fn displays_location_then_message() {
        let violation = Violation::new("crates/engine/src/utils.rs", "rename it");
        assert_eq!(
            violation.to_string(),
            "crates/engine/src/utils.rs: rename it"
        );
    }
}
```

`xtask/src/error.rs`:
```rust
//! Errors that stop a gate from running at all (as opposed to violations).

/// A failure to gather a gate's inputs.
#[derive(Debug, thiserror::Error)]
pub enum XtaskError {
    /// A file could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The file or directory involved.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A directory tree could not be walked.
    #[error("cannot walk {path}: {detail}")]
    Walk {
        /// The directory being walked.
        path: String,
        /// What went wrong.
        detail: String,
    },
    /// A tool printed JSON we could not parse.
    #[error("invalid JSON from {what}: {source}")]
    Json {
        /// Which tool or file produced the JSON.
        what: String,
        /// The parse error.
        #[source]
        source: serde_json::Error,
    },
    /// A TOML file could not be parsed.
    #[error("invalid TOML in {path}: {source}")]
    Toml {
        /// The file involved.
        path: String,
        /// The parse error.
        #[source]
        source: toml::de::Error,
    },
    /// An external command failed to start or exited unsuccessfully.
    #[error("command `{command}` failed: {detail}")]
    Command {
        /// The command line that was run.
        command: String,
        /// Its error output or start-up error.
        detail: String,
    },
    /// A built-in regular expression failed to compile.
    #[error("invalid built-in pattern: {0}")]
    Pattern(#[from] regex::Error),
}
```

`xtask/src/files.rs`:
```rust
//! Finding the workspace files the gates inspect.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::XtaskError;

/// Top-level directories that hold workspace source code.
pub const SOURCE_ROOTS: [&str; 2] = ["crates", "xtask"];

/// Returns the source roots that exist under `root`, relative to it.
pub fn source_dirs(root: &Path) -> Vec<PathBuf> {
    SOURCE_ROOTS
        .iter()
        .map(PathBuf::from)
        .filter(|dir| root.join(dir).is_dir())
        .collect()
}

/// Lists every `.rs` file under the source roots, relative to `root`, sorted.
///
/// # Errors
/// Returns an error when a directory cannot be read.
pub fn rust_files(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    let mut found = Vec::new();
    for dir in source_dirs(root) {
        for entry in WalkDir::new(root.join(&dir)) {
            let entry = entry.map_err(|error| XtaskError::Walk {
                path: dir.display().to_string(),
                detail: error.to_string(),
            })?;
            if entry.path().extension().is_some_and(|ext| ext == "rs") {
                found.push(relative(root, entry.path()));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Lists `xtask/Cargo.toml` and every `crates/*/Cargo.toml`, relative to `root`.
///
/// # Errors
/// Returns an error when the `crates` directory cannot be read.
pub fn member_manifests(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    let mut found = Vec::new();
    if root.join("xtask/Cargo.toml").is_file() {
        found.push(PathBuf::from("xtask/Cargo.toml"));
    }
    let crates = root.join("crates");
    if crates.is_dir() {
        for entry in std::fs::read_dir(&crates).map_err(|source| io_error(&crates, source))? {
            let manifest = entry
                .map_err(|source| io_error(&crates, source))?
                .path()
                .join("Cargo.toml");
            if manifest.is_file() {
                found.push(relative(root, &manifest));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Reads a whole file as UTF-8.
///
/// # Errors
/// Returns an error when the file is missing or unreadable.
pub fn read(path: &Path) -> Result<String, XtaskError> {
    std::fs::read_to_string(path).map_err(|source| io_error(path, source))
}

/// Renders a path with forward slashes, the form every gate reports.
pub fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn io_error(path: &Path, source: std::io::Error) -> XtaskError {
    XtaskError::Io {
        path: display(path),
        source,
    }
}
```

`xtask/src/process.rs`:
```rust
//! Running external tools and capturing what they print.

use std::{path::Path, process::Command};

use crate::error::XtaskError;

/// Runs `program` with `args` in `cwd` and returns its standard output.
///
/// # Errors
/// Returns an error when the program cannot start, exits unsuccessfully, or
/// prints output that is not UTF-8.
pub fn run_capture(program: &str, args: &[&str], cwd: &Path) -> Result<String, XtaskError> {
    let command = format!("{program} {}", args.join(" "));
    let output = Command::new(program)
        .args(args)
        .current_dir(cwd)
        .output()
        .map_err(|error| XtaskError::Command {
            command: command.clone(),
            detail: error.to_string(),
        })?;
    if !output.status.success() {
        let detail = String::from_utf8_lossy(&output.stderr).into_owned();
        return Err(XtaskError::Command { command, detail });
    }
    String::from_utf8(output.stdout).map_err(|error| XtaskError::Command {
        command,
        detail: error.to_string(),
    })
}
```

`xtask/src/thresholds.rs`:
```rust
//! Loads `quality/thresholds.toml`, the single source of every numeric gate.

use std::{collections::BTreeMap, path::Path};

use serde::Deserialize;

use crate::{error::XtaskError, files};

/// Location of the thresholds file, relative to the workspace root.
pub const PATH: &str = "quality/thresholds.toml";

/// Maximum allowed value of each metric, per function.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Limits {
    /// Cyclomatic complexity (independent paths).
    pub cyclomatic: f64,
    /// Cognitive complexity (how hard it is to follow).
    pub cognitive: f64,
    /// Logical lines, i.e. statements.
    pub logical_lines: f64,
    /// Physical lines that contain code.
    pub physical_lines: f64,
}

/// Minimum coverage percentages for one crate.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct CoverageTarget {
    /// Minimum line coverage, in percent.
    pub lines: f64,
    /// Minimum region coverage, in percent.
    pub regions: f64,
}

/// The minimum share of mutants the tests must catch.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MutationTarget {
    /// Minimum score, in percent: (caught + timeout) / (caught + timeout + missed).
    pub min_score: f64,
}

/// Every numeric quality threshold.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Thresholds {
    /// Per-function size and complexity limits.
    pub complexity: Limits,
    /// Coverage targets keyed by crate directory name.
    pub coverage: BTreeMap<String, CoverageTarget>,
    /// Mutation testing target.
    pub mutation: MutationTarget,
}

impl Thresholds {
    /// Parses thresholds TOML; `origin` names the source in errors.
    ///
    /// # Errors
    /// Returns an error when the TOML is invalid or a section is missing.
    pub fn parse(source: &str, origin: &str) -> Result<Self, XtaskError> {
        toml::from_str(source).map_err(|source| XtaskError::Toml {
            path: origin.to_owned(),
            source,
        })
    }

    /// Loads the thresholds file of the workspace at `root`.
    ///
    /// # Errors
    /// Returns an error when the file is missing or invalid.
    pub fn load(root: &Path) -> Result<Self, XtaskError> {
        let path = root.join(PATH);
        Self::parse(&files::read(&path)?, &files::display(&path))
    }
}
```

- [ ] **Step 7: Create the CLI, the dispatcher and the names gate**

`xtask/src/cli.rs` (only the first gate for now):
```rust
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
}
```

`xtask/src/commands.rs`:
```rust
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
```

`xtask/src/checks/mod.rs`:
```rust
//! Pure quality checks: each takes already-gathered input and returns violations.

pub mod names;
```

`xtask/src/checks/names.rs`:
```rust
//! Rejects catch-all module names, which hide more than one responsibility.

use std::path::{Path, PathBuf};

use crate::{files, violation::Violation};

/// Module names that never describe a single responsibility.
pub const FORBIDDEN: [&str; 4] = ["utils", "helpers", "misc", "common"];

/// Returns a violation for every path whose file or directory name is forbidden.
pub fn check_paths(paths: &[PathBuf]) -> Vec<Violation> {
    paths
        .iter()
        .filter(|path| has_forbidden_part(path))
        .map(|path| {
            Violation::new(
                files::display(path),
                "catch-all module name; name the module after its single responsibility",
            )
        })
        .collect()
}

fn has_forbidden_part(path: &Path) -> bool {
    path.iter().any(|part| {
        let part = part.to_string_lossy();
        let stem = part.strip_suffix(".rs").unwrap_or(&part);
        FORBIDDEN.contains(&stem)
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::check_paths;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn flags_forbidden_file_names() {
        let found = check_paths(&paths(&["crates/engine/src/utils.rs", "xtask/src/misc.rs"]));
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].location, "crates/engine/src/utils.rs");
    }

    #[test]
    fn flags_forbidden_directory_names() {
        let found = check_paths(&paths(&["crates/store/src/helpers/mod.rs"]));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn accepts_names_that_merely_contain_a_forbidden_word() {
        let found = check_paths(&paths(&[
            "crates/engine/src/common_table.rs",
            "src/value.rs",
        ]));
        assert!(found.is_empty());
    }
}
```

- [ ] **Step 8: Run the tests to verify they pass**

Run: `cargo test -p xtask`
Expected: PASS. That's 4 unit tests (violation and names) plus 8 integration tests (6 in `files.rs`, 2 in `commands.rs`).

- [ ] **Step 9: Run the gate on the real workspace, and check lints and formatting**

Run: `cargo xtask check-names; echo "exit $?"`
Expected: no output and `exit 0`.
Run: `cargo fmt --all -- --check && cargo clippy --workspace --all-targets -- -D warnings`
Expected: PASS.

- [ ] **Step 10: Commit**

```bash
git add Cargo.toml Cargo.lock .cargo quality xtask
git commit -m "feat(xtask): add gate framework and module-name gate"
```

---

### Task 3: Gate that every crate inherits the workspace lints

**Files:**
- Create: `xtask/src/checks/manifests.rs`
- Modify: `xtask/src/checks/mod.rs`, `xtask/src/cli.rs`, `xtask/src/commands.rs`
- Test: `xtask/tests/commands.rs` (append)

**Interfaces:**
- Consumes: `files::member_manifests`, `files::read`, `files::display` and `Violation` (Task 2).
- Produces: `checks::manifests::check(manifests: &[(String, String)]) -> Vec<Violation>` and `Command::CheckManifests`.

- [ ] **Step 1: Write the failing integration test.** Append to `xtask/tests/commands.rs`:

```rust
#[test]
fn manifests_gate() -> Result<(), TestError> {
    let dir = fixture()?;
    assert_eq!(count(&Command::CheckManifests, dir.path())?, 0);
    let opted_out = ENGINE_MANIFEST.replace("workspace = true", "workspace = false");
    write(dir.path(), "crates/engine/Cargo.toml", &opted_out)?;
    assert_eq!(count(&Command::CheckManifests, dir.path())?, 1);
    Ok(())
}
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p xtask --test commands manifests_gate`
Expected: FAIL to compile with `no variant ... named `CheckManifests``.

- [ ] **Step 3: Implement the pure check, including its unit tests.** Create `xtask/src/checks/manifests.rs`:

```rust
//! Requires every crate to opt in to the workspace lint table.

use serde::Deserialize;

use crate::violation::Violation;

#[derive(Deserialize)]
struct Manifest {
    lints: Option<LintsTable>,
}

#[derive(Deserialize)]
struct LintsTable {
    workspace: Option<bool>,
}

/// Checks `(path, contents)` pairs; flags manifests without `[lints] workspace = true`.
pub fn check(manifests: &[(String, String)]) -> Vec<Violation> {
    manifests
        .iter()
        .filter(|(_, source)| !inherits_workspace_lints(source))
        .map(|(path, _)| {
            Violation::new(
                path,
                "must contain `[lints] workspace = true` so the workspace lints apply",
            )
        })
        .collect()
}

fn inherits_workspace_lints(source: &str) -> bool {
    toml::from_str::<Manifest>(source)
        .ok()
        .and_then(|manifest| manifest.lints)
        .and_then(|lints| lints.workspace)
        == Some(true)
}

#[cfg(test)]
mod tests {
    use super::check;

    fn one(source: &str) -> Vec<(String, String)> {
        vec![("crates/engine/Cargo.toml".to_owned(), source.to_owned())]
    }

    #[test]
    fn accepts_manifest_inheriting_workspace_lints() {
        assert!(
            check(&one(
                "[package]\nname = \"a\"\n\n[lints]\nworkspace = true\n"
            ))
            .is_empty()
        );
    }

    #[test]
    fn flags_manifest_without_lints_table() {
        let found = check(&one("[package]\nname = \"a\"\n"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, "crates/engine/Cargo.toml");
    }

    #[test]
    fn flags_manifest_opting_out_or_invalid() {
        assert_eq!(check(&one("[lints]\nworkspace = false\n")).len(), 1);
        assert_eq!(check(&one("not = [valid")).len(), 1);
    }
}
```

- [ ] **Step 4: Wire the gate in, in three one-line additions**

In `xtask/src/checks/mod.rs`, add `pub mod manifests;` and keep the list alphabetical.

In `xtask/src/cli.rs`, add this variant after `CheckNames`:
```rust
    /// Require every crate manifest to inherit the workspace lints.
    CheckManifests,
```

In `xtask/src/commands.rs`, add this match arm after the `CheckNames` arm:
```rust
        Command::CheckManifests => manifests(root),
```
Then add this function after `run`:
```rust
fn manifests(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let mut found = Vec::new();
    for path in files::member_manifests(root)? {
        found.push((files::display(&path), files::read(&root.join(&path))?));
    }
    Ok(checks::manifests::check(&found))
}
```

- [ ] **Step 5: Run the tests and the gate**

Run: `cargo test -p xtask`
Expected: PASS, including 3 new unit tests and `manifests_gate`.
Run: `cargo xtask check-manifests; echo "exit $?"`
Expected: `exit 0`.

- [ ] **Step 6: Commit**

```bash
git add xtask
git commit -m "feat(xtask): add workspace-lints inheritance gate"
```

---

### Task 4: Dependency-direction gate

**Files:**
- Create: `xtask/src/checks/deps.rs`
- Modify: `xtask/src/checks/mod.rs`, `xtask/src/cli.rs`, `xtask/src/commands.rs`
- Test: `xtask/tests/commands.rs` (append)

**Interfaces:**
- Consumes: `process::run_capture` (Task 2).
- Produces: `checks::deps::RULES: &[(&str, &[&str])]`, `checks::deps::check_metadata(json: &str) -> Result<Vec<Violation>, XtaskError>` and `Command::CheckDeps`. **When a later phase adds a workspace crate, it must add a rule to `RULES`**, or this gate fails with "no dependency rule".

- [ ] **Step 1: Write the failing integration test.** Append to `xtask/tests/commands.rs`:

```rust
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
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p xtask --test commands deps_gate`
Expected: FAIL to compile with `no variant ... named `CheckDeps``.

- [ ] **Step 3: Implement the pure check, including its unit tests.** Create `xtask/src/checks/deps.rs`:

```rust
//! Enforces the one-way dependency direction between workspace crates.

use serde::Deserialize;

use crate::{error::XtaskError, violation::Violation};

/// For each workspace package, the workspace packages it may depend on.
/// Adding a crate means adding a rule here (design spec §3.2).
pub const RULES: &[(&str, &[&str])] = &[
    (
        "quarry",
        &[
            "quarry-ui-kit",
            "quarry-challenges",
            "quarry-store",
            "quarry-engine",
        ],
    ),
    ("quarry-ui-kit", &[]),
    ("quarry-challenges", &["quarry-engine"]),
    ("quarry-store", &["quarry-engine"]),
    ("quarry-engine", &[]),
    ("xtask", &[]),
];

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    path: Option<String>,
}

/// Checks `cargo metadata --format-version 1 --no-deps` output against [`RULES`].
///
/// # Errors
/// Returns an error when the metadata is not valid JSON of the expected shape.
pub fn check_metadata(json: &str) -> Result<Vec<Violation>, XtaskError> {
    let metadata: Metadata = serde_json::from_str(json).map_err(|source| XtaskError::Json {
        what: "cargo metadata".to_owned(),
        source,
    })?;
    Ok(metadata.packages.iter().flat_map(check_package).collect())
}

fn check_package(package: &Package) -> Vec<Violation> {
    let Some(allowed) = allowed_for(&package.name) else {
        return vec![Violation::new(
            &package.name,
            "no dependency rule; add one to RULES in xtask/src/checks/deps.rs",
        )];
    };
    package
        .dependencies
        .iter()
        .filter(|dep| dep.path.is_some() && !allowed.contains(&dep.name.as_str()))
        .map(|dep| Violation::new(&package.name, format!("must not depend on {}", dep.name)))
        .collect()
}

fn allowed_for(name: &str) -> Option<&'static [&'static str]> {
    RULES
        .iter()
        .find(|(package, _)| *package == name)
        .map(|(_, allowed)| *allowed)
}

#[cfg(test)]
mod tests {
    use super::check_metadata;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn metadata(package: &str, dep: &str, path: Option<&str>) -> String {
        let path = path.map_or("null".to_owned(), |p| format!("\"{p}\""));
        format!(
            r#"{{"packages":[{{"name":"{package}","dependencies":[{{"name":"{dep}","path":{path}}}]}}]}}"#
        )
    }

    #[test]
    fn allows_declared_direction() -> TestResult {
        let json = metadata("quarry-store", "quarry-engine", Some("/w/crates/engine"));
        assert!(check_metadata(&json)?.is_empty());
        Ok(())
    }

    #[test]
    fn flags_reversed_direction() -> TestResult {
        let json = metadata("quarry-engine", "quarry-ui-kit", Some("/w/crates/ui-kit"));
        let found = check_metadata(&json)?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].message, "must not depend on quarry-ui-kit");
        Ok(())
    }

    #[test]
    fn ignores_registry_dependencies() -> TestResult {
        assert!(check_metadata(&metadata("quarry-engine", "serde", None))?.is_empty());
        Ok(())
    }

    #[test]
    fn flags_package_without_rule() -> TestResult {
        let found = check_metadata(&metadata("quarry-new", "serde", None))?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, "quarry-new");
        Ok(())
    }

    #[test]
    fn rejects_malformed_metadata() {
        assert!(check_metadata("{").is_err());
    }
}
```

- [ ] **Step 4: Wire the gate in**

`xtask/src/checks/mod.rs`: add `pub mod deps;`.

`xtask/src/cli.rs`, after `CheckManifests`:
```rust
    /// Enforce the crate dependency direction.
    CheckDeps,
```

`xtask/src/commands.rs`:
- Change the `use` block to:
```rust
use crate::{checks, cli::Command, error::XtaskError, files, process, violation::Violation};
```
- Add this arm after `CheckManifests`:
```rust
        Command::CheckDeps => deps(root),
```
- Add this function:
```rust
fn deps(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let args = ["metadata", "--format-version", "1", "--no-deps"];
    checks::deps::check_metadata(&process::run_capture("cargo", &args, root)?)
}
```

- [ ] **Step 5: Run the tests and the gate**

Run: `cargo test -p xtask`
Expected: PASS, including 5 new unit tests and `deps_gate`.
Run: `cargo xtask check-deps; echo "exit $?"`
Expected: `exit 0`.

- [ ] **Step 6: Commit**

```bash
git add xtask
git commit -m "feat(xtask): enforce one-way crate dependency direction"
```

---

### Task 5: Lint-exceptions registry gate

**Files:**
- Create: `xtask/src/checks/exceptions.rs`
- Modify: `xtask/src/checks/mod.rs`, `xtask/src/cli.rs`, `xtask/src/commands.rs`
- Test: `xtask/tests/commands.rs` (append)

**Interfaces:**
- Produces:
  - `checks::exceptions::COMPLEXITY_LINT = "quarry::complexity"`
  - `Exception { file, lint, function: Option<String>, reason }`
  - `Suppression { file, line, lint }`
  - `parse_registry(source: &str, origin: &str) -> Result<Vec<Exception>, XtaskError>`
  - `Scanner::new() -> Result<Scanner, XtaskError>` and `Scanner::find(&self, file: &str, source: &str) -> Vec<Suppression>`
  - `check(found: &[Suppression], registry: &[Exception]) -> Vec<Violation>`
  - `commands::EXCEPTIONS_PATH`, the private `commands::registry(root)` (Task 6 reuses it) and `Command::CheckExceptions`

**Important:** the scanner runs over `xtask` itself. Never write a literal lint-suppression attribute in a test string. Build it with `format!`, as the tests below do.

- [ ] **Step 1: Write the failing integration test.** Append to `xtask/tests/commands.rs`:

```rust
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
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p xtask --test commands exceptions_gate`
Expected: FAIL to compile with `no variant ... named `CheckExceptions``.

- [ ] **Step 3: Implement the pure check, including its unit tests.** Create `xtask/src/checks/exceptions.rs`:

```rust
//! Every lint suppression must be listed, with a reason, in
//! `quality/exceptions.toml`, so each one is visible in review.

use regex::Regex;
use serde::Deserialize;

use crate::{error::XtaskError, violation::Violation};

/// Registry lint name for a waiver of the `complexity` gate. These entries
/// have no source attribute, so they are never reported as stale.
pub const COMPLEXITY_LINT: &str = "quarry::complexity";

/// One registered, reviewed lint suppression.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Exception {
    /// File the suppression lives in, relative to the workspace root.
    pub file: String,
    /// Full lint name, e.g. `clippy::too_many_lines`.
    pub lint: String,
    /// Function name; used only by complexity waivers.
    #[serde(default)]
    pub function: Option<String>,
    /// Why the suppression is justified.
    pub reason: String,
}

/// One lint suppression attribute found in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suppression {
    /// File it was found in, relative to the workspace root.
    pub file: String,
    /// 1-based line of the attribute.
    pub line: usize,
    /// The lint it suppresses.
    pub lint: String,
}

#[derive(Deserialize)]
struct Registry {
    #[serde(default)]
    exception: Vec<Exception>,
}

/// Parses the registry file contents; `origin` names the file in errors.
///
/// # Errors
/// Returns an error when the contents are not valid registry TOML.
pub fn parse_registry(source: &str, origin: &str) -> Result<Vec<Exception>, XtaskError> {
    toml::from_str::<Registry>(source)
        .map(|registry| registry.exception)
        .map_err(|source| XtaskError::Toml {
            path: origin.to_owned(),
            source,
        })
}

/// Finds lint suppression attributes in Rust source.
pub struct Scanner {
    pattern: Regex,
}

impl Scanner {
    /// Builds the scanner.
    ///
    /// # Errors
    /// Returns an error only if the built-in pattern is invalid.
    pub fn new() -> Result<Self, XtaskError> {
        let pattern =
            Regex::new(r"(?:#!?\[\s*|cfg_attr\([^)]*,\s*)(?:allow|expect)\s*\(([^)]*)\)")?;
        Ok(Self { pattern })
    }

    /// Returns every suppressed lint in `source`, which came from `file`.
    pub fn find(&self, file: &str, source: &str) -> Vec<Suppression> {
        let mut found = Vec::new();
        for captures in self.pattern.captures_iter(source) {
            let start = captures.get(0).map_or(0, |m| m.start());
            let line = source[..start].matches('\n').count() + 1;
            let list = captures.get(1).map_or("", |m| m.as_str());
            for lint in lint_names(list) {
                found.push(Suppression {
                    file: file.to_owned(),
                    line,
                    lint,
                });
            }
        }
        found
    }
}

fn lint_names(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty() && !item.starts_with("reason"))
        .map(str::to_owned)
        .collect()
}

/// Reports unregistered suppressions, reasonless entries, and stale entries.
pub fn check(found: &[Suppression], registry: &[Exception]) -> Vec<Violation> {
    let mut violations: Vec<Violation> = found
        .iter()
        .filter(|s| {
            !registry
                .iter()
                .any(|e| e.file == s.file && e.lint == s.lint)
        })
        .map(|s| {
            Violation::new(
                format!("{}:{}", s.file, s.line),
                format!(
                    "{} is suppressed but not registered in quality/exceptions.toml",
                    s.lint
                ),
            )
        })
        .collect();
    violations.extend(registry.iter().filter_map(|e| registry_problem(e, found)));
    violations
}

fn registry_problem(entry: &Exception, found: &[Suppression]) -> Option<Violation> {
    if entry.reason.trim().is_empty() {
        return Some(Violation::new(
            &entry.file,
            format!("exception for {} has no reason", entry.lint),
        ));
    }
    let used = found
        .iter()
        .any(|s| s.file == entry.file && s.lint == entry.lint);
    (entry.lint != COMPLEXITY_LINT && !used).then(|| {
        Violation::new(
            &entry.file,
            format!(
                "exception for {} matches no suppression; remove it",
                entry.lint
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{COMPLEXITY_LINT, Exception, Scanner, check, parse_registry};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Builds an attribute without writing it literally, so this file does not
    /// trip the scanner when the gate runs on the workspace itself.
    fn attr(inner: bool, kind: &str, lints: &str) -> String {
        let bang = if inner { "!" } else { "" };
        format!("#{bang}[{kind}({lints})]")
    }

    fn entry(file: &str, lint: &str, reason: &str) -> Exception {
        Exception {
            file: file.to_owned(),
            lint: lint.to_owned(),
            function: None,
            reason: reason.to_owned(),
        }
    }

    #[test]
    fn finds_outer_inner_and_expect_attributes() -> TestResult {
        let source = format!(
            "{}\nfn a() {{}}\n{}\n",
            attr(true, "allow", "dead_code"),
            attr(false, "expect", "clippy::too_many_lines, reason = \"x\"")
        );
        let found = Scanner::new()?.find("f.rs", &source);
        let lints: Vec<_> = found.iter().map(|s| (s.lint.as_str(), s.line)).collect();
        assert_eq!(lints, [("dead_code", 1), ("clippy::too_many_lines", 3)]);
        Ok(())
    }

    #[test]
    fn finds_attributes_inside_cfg_attr() -> TestResult {
        let source = format!("#[cfg_attr(test, {}(clippy::a, clippy::b))]", "allow");
        assert_eq!(Scanner::new()?.find("f.rs", &source).len(), 2);
        Ok(())
    }

    #[test]
    fn flags_unregistered_suppression() -> TestResult {
        let found = Scanner::new()?.find("f.rs", &attr(false, "allow", "clippy::x"));
        let violations = check(&found, &[]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].location, "f.rs:1");
        Ok(())
    }

    #[test]
    fn accepts_registered_suppression() -> TestResult {
        let found = Scanner::new()?.find("f.rs", &attr(false, "allow", "clippy::x"));
        assert!(check(&found, &[entry("f.rs", "clippy::x", "generated code")]).is_empty());
        Ok(())
    }

    #[test]
    fn flags_reasonless_and_stale_entries() {
        let registry = [
            entry("f.rs", "clippy::x", " "),
            entry("g.rs", "clippy::y", "why"),
        ];
        let messages: Vec<_> = check(&[], &registry)
            .into_iter()
            .map(|v| v.message)
            .collect();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("no reason"));
        assert!(messages[1].contains("matches no suppression"));
    }

    #[test]
    fn complexity_waivers_are_never_stale() {
        assert!(check(&[], &[entry("f.rs", COMPLEXITY_LINT, "parser table")]).is_empty());
    }

    #[test]
    fn parses_registry_and_reports_bad_toml() -> TestResult {
        let src = "[[exception]]\nfile = \"f.rs\"\nlint = \"clippy::x\"\nreason = \"why\"\n";
        assert_eq!(
            parse_registry(src, "e.toml")?,
            [entry("f.rs", "clippy::x", "why")]
        );
        assert!(parse_registry("", "e.toml")?.is_empty());
        assert!(parse_registry("[[exception]]\nfile = 1", "e.toml").is_err());
        Ok(())
    }
}
```

- [ ] **Step 4: Wire the gate in**

`xtask/src/checks/mod.rs`: add `pub mod exceptions;`.

`xtask/src/cli.rs`, after `CheckDeps`:
```rust
    /// Require every lint suppression to be registered with a reason.
    CheckExceptions,
```

`xtask/src/commands.rs`:
- Change the `use` block to:
```rust
use crate::{
    checks::{self, exceptions::Exception},
    cli::Command,
    error::XtaskError,
    files, process,
    violation::Violation,
};

/// Location of the exceptions registry, relative to the workspace root.
pub const EXCEPTIONS_PATH: &str = "quality/exceptions.toml";
```
- Add this arm:
```rust
        Command::CheckExceptions => exceptions(root),
```
- Add these two functions:
```rust
fn exceptions(root: &Path) -> Result<Vec<Violation>, XtaskError> {
    let scanner = checks::exceptions::Scanner::new()?;
    let mut found = Vec::new();
    for path in files::rust_files(root)? {
        let source = files::read(&root.join(&path))?;
        found.extend(scanner.find(&files::display(&path), &source));
    }
    Ok(checks::exceptions::check(&found, &registry(root)?))
}

fn registry(root: &Path) -> Result<Vec<Exception>, XtaskError> {
    let path = root.join(EXCEPTIONS_PATH);
    checks::exceptions::parse_registry(&files::read(&path)?, &files::display(&path))
}
```

- [ ] **Step 5: Run the tests and the gate**

Run: `cargo test -p xtask`
Expected: PASS, including 7 new unit tests and `exceptions_gate`.
Run: `cargo xtask check-exceptions; echo "exit $?"`
Expected: `exit 0`. If it reports a suppression inside `xtask/`, a test string contains a literal attribute; rebuild that string with `format!`.

- [ ] **Step 6: Commit**

```bash
git add xtask
git commit -m "feat(xtask): require every lint suppression to be registered"
```

---

### Task 6: Complexity gate (cyclomatic, cognitive, statements, lines)

**Files:**
- Create: `xtask/src/checks/complexity.rs`
- Modify: `xtask/src/checks/mod.rs`, `xtask/src/cli.rs`, `xtask/src/commands.rs`
- Test: `xtask/tests/commands.rs` (append)

**Interfaces:**
- Consumes: `thresholds::{Thresholds, Limits}` (Task 2), `exceptions::{COMPLEXITY_LINT, Exception}` and `commands::registry` (Task 5).
- Produces: `checks::complexity::check_report(report: &str, root: &Path, limits: &Limits, waivers: &[Exception]) -> Result<Vec<Violation>, XtaskError>` and `Command::Complexity`.

**Tool facts (verified against rust-code-analysis-cli 0.0.25):**
- `rust-code-analysis-cli -m -O json -p <dir>` prints **one JSON object per file, one per line**.
- Each object is a "space" with `kind` (`unit`, `impl`, `function`, …), `name`, `start_line`, `metrics.cyclomatic.sum`, `metrics.cognitive.sum`, `metrics.loc.lloc` (statements), `metrics.loc.ploc` (lines containing code) and a nested `spaces` array.
- Functions inside `impl` blocks are nested one level down.

- [ ] **Step 1: Install the tool if it's missing**

Run: `cargo install --locked rust-code-analysis-cli@0.0.25 && (command -v asdf >/dev/null && asdf reshim rust || true) && rust-code-analysis-cli --version`
Expected: prints `rust-code-analysis-cli 0.0.25`.

- [ ] **Step 2: Write the failing integration test.** Append to `xtask/tests/commands.rs`:

```rust
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
```

- [ ] **Step 3: Run it to verify it fails**

Run: `cargo test -p xtask --test commands complexity_gate`
Expected: FAIL to compile with `no variant ... named `Complexity``.

- [ ] **Step 4: Implement the pure check, including its unit tests.** Create `xtask/src/checks/complexity.rs`:

```rust
//! Per-function size and complexity limits, measured by `rust-code-analysis-cli`.

use std::path::Path;

use serde::Deserialize;

use crate::{
    checks::exceptions::{COMPLEXITY_LINT, Exception},
    error::XtaskError,
    thresholds::Limits,
    violation::Violation,
};

#[derive(Deserialize)]
struct Space {
    kind: String,
    name: Option<String>,
    start_line: u64,
    metrics: Metrics,
    #[serde(default)]
    spaces: Vec<Space>,
}

#[derive(Deserialize)]
struct Metrics {
    cyclomatic: Sum,
    cognitive: Sum,
    loc: Loc,
}

#[derive(Deserialize)]
struct Sum {
    sum: f64,
}

#[derive(Deserialize)]
struct Loc {
    lloc: f64,
    ploc: f64,
}

/// Checks a report (one JSON object per file) against `limits`.
/// Functions waived in `waivers` with [`COMPLEXITY_LINT`] are skipped.
///
/// # Errors
/// Returns an error when the report is not valid JSON of the expected shape.
pub fn check_report(
    report: &str,
    root: &Path,
    limits: &Limits,
    waivers: &[Exception],
) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for unit in parse_units(report)? {
        let file = relative_name(unit.name.as_deref().unwrap_or_default(), root);
        if Path::new(&file).extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let mut functions = Vec::new();
        collect_functions(&unit, &mut functions);
        for function in functions
            .into_iter()
            .filter(|f| !is_waived(&file, f, waivers))
        {
            violations.extend(measure(&file, function, limits));
        }
    }
    Ok(violations)
}

fn parse_units(report: &str) -> Result<Vec<Space>, XtaskError> {
    serde_json::Deserializer::from_str(report)
        .into_iter::<Space>()
        .collect::<Result<_, _>>()
        .map_err(|source| XtaskError::Json {
            what: "rust-code-analysis-cli".to_owned(),
            source,
        })
}

fn relative_name(name: &str, root: &Path) -> String {
    let path = Path::new(name);
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

fn collect_functions<'a>(space: &'a Space, out: &mut Vec<&'a Space>) {
    for child in &space.spaces {
        if child.kind == "function" {
            out.push(child);
        }
        collect_functions(child, out);
    }
}

fn is_waived(file: &str, function: &Space, waivers: &[Exception]) -> bool {
    waivers.iter().any(|w| {
        w.lint == COMPLEXITY_LINT
            && w.file == file
            && w.function.as_deref() == function.name.as_deref()
    })
}

fn measure(file: &str, function: &Space, limits: &Limits) -> Vec<Violation> {
    let m = &function.metrics;
    let name = function.name.as_deref().unwrap_or("<anonymous>");
    let location = format!("{file}:{} fn {name}", function.start_line);
    [
        ("cyclomatic complexity", m.cyclomatic.sum, limits.cyclomatic),
        ("cognitive complexity", m.cognitive.sum, limits.cognitive),
        ("logical lines", m.loc.lloc, limits.logical_lines),
        ("physical lines", m.loc.ploc, limits.physical_lines),
    ]
    .into_iter()
    .filter(|(_, value, max)| value > max)
    .map(|(metric, value, max)| {
        Violation::new(&location, format!("{metric} {value} exceeds {max}"))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::check_report;
    use crate::{
        checks::exceptions::{COMPLEXITY_LINT, Exception},
        thresholds::Limits,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const LIMITS: Limits = Limits {
        cyclomatic: 10.0,
        cognitive: 15.0,
        logical_lines: 15.0,
        physical_lines: 40.0,
    };

    fn function(name: &str, cyclomatic: f64, lloc: f64) -> String {
        format!(
            r#"{{"kind":"function","name":"{name}","start_line":3,"metrics":{{"cyclomatic":{{"sum":{cyclomatic}}},"cognitive":{{"sum":1}},"loc":{{"lloc":{lloc},"ploc":5}}}},"spaces":[]}}"#
        )
    }

    fn unit(file: &str, children: &str) -> String {
        format!(
            r#"{{"kind":"unit","name":"{file}","start_line":1,"metrics":{{"cyclomatic":{{"sum":1}},"cognitive":{{"sum":0}},"loc":{{"lloc":0,"ploc":0}}}},"spaces":[{children}]}}"#
        )
    }

    #[test]
    fn accepts_functions_within_limits() -> TestResult {
        let report = unit("/w/crates/a/src/lib.rs", &function("ok", 2.0, 4.0));
        assert!(check_report(&report, Path::new("/w"), &LIMITS, &[])?.is_empty());
        Ok(())
    }

    #[test]
    fn reports_each_metric_over_its_limit() -> TestResult {
        let report = unit("/w/crates/a/src/lib.rs", &function("big", 11.0, 16.0));
        let found = check_report(&report, Path::new("/w"), &LIMITS, &[])?;
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].location, "crates/a/src/lib.rs:3 fn big");
        assert_eq!(found[0].message, "cyclomatic complexity 11 exceeds 10");
        Ok(())
    }

    #[test]
    fn finds_functions_nested_in_impls_across_files() -> TestResult {
        let nested = unit("a.rs", &unit("impl", &function("deep", 12.0, 1.0)));
        let report = format!("{nested}\n{}", unit("b.rs", &function("too", 12.0, 1.0)));
        assert_eq!(
            check_report(&report, Path::new("/w"), &LIMITS, &[])?.len(),
            2
        );
        Ok(())
    }

    #[test]
    fn skips_waived_functions_and_non_rust_units() -> TestResult {
        let waiver = Exception {
            file: "a.rs".to_owned(),
            lint: COMPLEXITY_LINT.to_owned(),
            function: Some("big".to_owned()),
            reason: "lookup table".to_owned(),
        };
        let report = format!(
            "{}\n{}",
            unit("a.rs", &function("big", 30.0, 1.0)),
            unit("a.py", &function("x", 30.0, 1.0))
        );
        assert!(check_report(&report, Path::new("/w"), &LIMITS, &[waiver])?.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_malformed_report() {
        assert!(check_report("{", Path::new("/w"), &LIMITS, &[]).is_err());
    }
}
```

- [ ] **Step 5: Wire the gate in**

`xtask/src/checks/mod.rs`: add `pub mod complexity;`.

`xtask/src/cli.rs`, after `CheckExceptions`:
```rust
    /// Enforce per-function size and complexity limits.
    Complexity,
```

`xtask/src/commands.rs`:
- Add `thresholds::Thresholds,` to the `use crate::{...}` block, before `violation::Violation`.
- Add this arm:
```rust
        Command::Complexity => complexity(root),
```
- Add this function:
```rust
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
```

- [ ] **Step 6: Run the tests and the gate on the real workspace**

Run: `cargo test -p xtask`
Expected: PASS, including 5 new unit tests and `complexity_gate`.
Run: `cargo xtask complexity; echo "exit $?"`
Expected: `exit 0`. Every function in `xtask`, tests included, is within 10/15/15/40. If one isn't, split it; don't add a waiver.

- [ ] **Step 7: Commit**

```bash
git add xtask
git commit -m "feat(xtask): gate per-function cyclomatic, cognitive and size limits"
```

---

### Task 7: Per-crate coverage gate

**Files:**
- Create: `xtask/src/checks/coverage.rs`
- Modify: `xtask/src/checks/mod.rs`, `xtask/src/cli.rs`, `xtask/src/commands.rs`
- Test: `xtask/tests/commands.rs` (append)

**Interfaces:**
- Consumes: `thresholds::CoverageTarget` (Task 2).
- Produces: `checks::coverage::check_report(json: &str, root: &Path, targets: &BTreeMap<String, CoverageTarget>) -> Result<Vec<Violation>, XtaskError>` and `Command::CoverageGate { report: PathBuf }`.

**Tool facts:**
- `cargo llvm-cov ... --json --summary-only` writes `{"data":[{"files":[{"filename": <absolute path>, "summary": {"lines": {"count","covered"}, "regions": {"count","covered"}}}]}]}`.
- **`--output-path` fails if its directory doesn't exist**, so the `justfile` creates it first (Task 9).
- A crate with zero instrumented lines passes. Files that belong to no crate target are violations, so nothing escapes the gate.

- [ ] **Step 1: Write the failing integration test.** In `xtask/tests/commands.rs`, change `use std::path::Path;` to `use std::path::{Path, PathBuf};`, then append:

```rust
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
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p xtask --test commands coverage_gate`
Expected: FAIL to compile with `no variant ... named `CoverageGate``.

- [ ] **Step 3: Implement the pure check, including its unit tests.** Create `xtask/src/checks/coverage.rs`:

```rust
//! Per-crate line and region coverage thresholds.

use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

use serde::Deserialize;

use crate::{error::XtaskError, thresholds::CoverageTarget, violation::Violation};

#[derive(Deserialize)]
struct Report {
    data: Vec<Export>,
}

#[derive(Deserialize)]
struct Export {
    files: Vec<FileEntry>,
}

#[derive(Deserialize)]
struct FileEntry {
    filename: String,
    summary: Summary,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
struct Summary {
    lines: Counts,
    regions: Counts,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
struct Counts {
    count: u32,
    covered: u32,
}

/// Checks an `llvm-cov` JSON summary report; `targets` is keyed by the crate's
/// directory name (`engine`, `ui-kit`, ..., `xtask`).
///
/// # Errors
/// Returns an error when the report is not valid JSON of the expected shape.
pub fn check_report(
    json: &str,
    root: &Path,
    targets: &BTreeMap<String, CoverageTarget>,
) -> Result<Vec<Violation>, XtaskError> {
    let report: Report = serde_json::from_str(json).map_err(|source| XtaskError::Json {
        what: "llvm-cov report".to_owned(),
        source,
    })?;
    let mut totals: BTreeMap<String, Summary> = BTreeMap::new();
    let mut violations = Vec::new();
    for file in report.data.iter().flat_map(|export| &export.files) {
        match crate_of(&file.filename, root).filter(|name| targets.contains_key(name)) {
            Some(name) => add(totals.entry(name).or_default(), file.summary),
            None => violations.push(Violation::new(
                &file.filename,
                "file belongs to no crate with a coverage target",
            )),
        }
    }
    for (name, summary) in &totals {
        if let Some(target) = targets.get(name) {
            violations.extend(judge(name, *summary, *target));
        }
    }
    Ok(violations)
}

fn crate_of(filename: &str, root: &Path) -> Option<String> {
    let relative = Path::new(filename).strip_prefix(root).ok()?;
    let mut parts = relative.components().filter_map(|part| match part {
        Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
        _ => None,
    });
    match parts.next()?.as_str() {
        "crates" => parts.next(),
        "xtask" => Some("xtask".to_owned()),
        _ => None,
    }
}

fn add(total: &mut Summary, file: Summary) {
    total.lines.count = total.lines.count.saturating_add(file.lines.count);
    total.lines.covered = total.lines.covered.saturating_add(file.lines.covered);
    total.regions.count = total.regions.count.saturating_add(file.regions.count);
    total.regions.covered = total.regions.covered.saturating_add(file.regions.covered);
}

fn judge(name: &str, summary: Summary, target: CoverageTarget) -> Vec<Violation> {
    [
        ("line", summary.lines, target.lines),
        ("region", summary.regions, target.regions),
    ]
    .into_iter()
    .filter_map(|(metric, counts, min)| {
        percent(counts)
            .filter(|actual| *actual < min)
            .map(|actual| {
                Violation::new(
                    name,
                    format!("{metric} coverage {actual:.1}% is below {min}%"),
                )
            })
    })
    .collect()
}

fn percent(counts: Counts) -> Option<f64> {
    (counts.count > 0).then(|| f64::from(counts.covered) * 100.0 / f64::from(counts.count))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::Path};

    use super::check_report;
    use crate::thresholds::CoverageTarget;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn targets() -> BTreeMap<String, CoverageTarget> {
        BTreeMap::from([
            (
                "engine".to_owned(),
                CoverageTarget {
                    lines: 95.0,
                    regions: 90.0,
                },
            ),
            (
                "xtask".to_owned(),
                CoverageTarget {
                    lines: 90.0,
                    regions: 85.0,
                },
            ),
        ])
    }

    fn file(name: &str, lines: (u32, u32), regions: (u32, u32)) -> String {
        format!(
            r#"{{"filename":"{name}","summary":{{"lines":{{"count":{},"covered":{}}},"regions":{{"count":{},"covered":{}}}}}}}"#,
            lines.0, lines.1, regions.0, regions.1
        )
    }

    fn report(files: &[String]) -> String {
        format!(r#"{{"data":[{{"files":[{}]}}]}}"#, files.join(","))
    }

    #[test]
    fn passes_when_every_crate_meets_its_target() -> TestResult {
        let json = report(&[file("/w/crates/engine/src/a.rs", (100, 96), (10, 9))]);
        assert!(check_report(&json, Path::new("/w"), &targets())?.is_empty());
        Ok(())
    }

    #[test]
    fn aggregates_files_per_crate_before_judging() -> TestResult {
        let json = report(&[
            file("/w/crates/engine/src/a.rs", (50, 50), (10, 10)),
            file("/w/crates/engine/src/b.rs", (50, 40), (10, 10)),
        ]);
        let found = check_report(&json, Path::new("/w"), &targets())?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, "engine");
        assert_eq!(found[0].message, "line coverage 90.0% is below 95%");
        Ok(())
    }

    #[test]
    fn crates_without_code_pass() -> TestResult {
        let json = report(&[file("/w/xtask/src/lib.rs", (0, 0), (0, 0))]);
        assert!(check_report(&json, Path::new("/w"), &targets())?.is_empty());
        Ok(())
    }

    #[test]
    fn flags_files_outside_any_target() -> TestResult {
        let json = report(&[
            file("/w/crates/mystery/src/a.rs", (1, 1), (1, 1)),
            file("/elsewhere/x.rs", (1, 1), (1, 1)),
        ]);
        assert_eq!(check_report(&json, Path::new("/w"), &targets())?.len(), 2);
        Ok(())
    }

    #[test]
    fn rejects_malformed_report() {
        assert!(check_report("[]", Path::new("/w"), &targets()).is_err());
    }
}
```

- [ ] **Step 4: Wire the gate in**

`xtask/src/checks/mod.rs`: add `pub mod coverage;`.

`xtask/src/cli.rs`:
- Add `use std::path::PathBuf;` above the `clap` import.
- Add this variant after `Complexity`:
```rust
    /// Enforce per-crate coverage thresholds.
    CoverageGate {
        /// Path to the `cargo llvm-cov --json --summary-only` report.
        #[arg(long)]
        report: PathBuf,
    },
```

`xtask/src/commands.rs`:
- Add this arm:
```rust
        Command::CoverageGate { report } => coverage(root, report),
```
- Add this function:
```rust
fn coverage(root: &Path, report: &Path) -> Result<Vec<Violation>, XtaskError> {
    let targets = Thresholds::load(root)?.coverage;
    checks::coverage::check_report(&files::read(report)?, root, &targets)
}
```

- [ ] **Step 5: Run the tests, then measure real coverage**

Run: `cargo test -p xtask`
Expected: PASS.
Run: `cargo install --locked cargo-nextest@0.9.151 cargo-llvm-cov@0.9.1` (skip if already installed), then:
```bash
mkdir -p target/llvm-cov
cargo llvm-cov nextest --workspace --ignore-filename-regex '(^|/)main\.rs$' --json --summary-only --output-path target/llvm-cov/coverage.json
cargo xtask coverage-gate --report target/llvm-cov/coverage.json; echo "exit $?"
```
Expected: `exit 0`. Reference measurement from verifying this plan: `xtask` at about 98% lines and 95% regions.

- [ ] **Step 6: Commit**

```bash
git add xtask
git commit -m "feat(xtask): enforce per-crate line and region coverage"
```

---

### Task 8: Mutation-score gate

**Files:**
- Create: `xtask/src/checks/mutation.rs`
- Modify: `xtask/src/checks/mod.rs`, `xtask/src/cli.rs`, `xtask/src/commands.rs`
- Test: `xtask/tests/commands.rs` (append)

**Interfaces:**
- Consumes: `thresholds::MutationTarget` (Task 2).
- Produces: `checks::mutation::check_outcomes(json: &str, target: &MutationTarget) -> Result<Vec<Violation>, XtaskError>` and `Command::MutationGate { outcomes: PathBuf }`.

**Tool facts (verified against cargo-mutants 27.1.0):**
- `mutants.out/outcomes.json` has these top-level keys: `total_mutants`, `missed`, `caught`, `timeout`, `unviable`, `success` and `outcomes`.
- The score is `(caught + timeout) / (caught + timeout + missed)`. Unviable mutants don't count.
- `cargo mutants` exits with code 2 when it misses mutants. The `justfile` treats exit codes 0 and 2 as "ran", then lets this gate decide.
- **With zero mutants** (true of the empty core crates throughout Phase 0), cargo-mutants exits 0 and writes **no** `outcomes.json`. The `just mutants` recipe (Task 9) therefore clears old results first and skips the gate when the file is absent. This gate itself still treats a missing file as an error.

- [ ] **Step 1: Write the failing integration test.** Append to `xtask/tests/commands.rs`:

```rust
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
```

- [ ] **Step 2: Run it to verify it fails**

Run: `cargo test -p xtask --test commands mutation_gate`
Expected: FAIL to compile with `no variant ... named `MutationGate``.

- [ ] **Step 3: Implement the pure check, including its unit tests.** Create `xtask/src/checks/mutation.rs`:

```rust
//! Minimum mutation score from a `cargo mutants` run.

use serde::Deserialize;

use crate::{error::XtaskError, thresholds::MutationTarget, violation::Violation};

#[derive(Deserialize)]
struct Outcomes {
    caught: u32,
    missed: u32,
    timeout: u32,
}

/// Checks the contents of `mutants.out/outcomes.json` against `target`.
/// Unviable mutants (ones that do not compile) are not counted.
///
/// # Errors
/// Returns an error when the file is not valid JSON of the expected shape.
pub fn check_outcomes(json: &str, target: &MutationTarget) -> Result<Vec<Violation>, XtaskError> {
    let outcomes: Outcomes = serde_json::from_str(json).map_err(|source| XtaskError::Json {
        what: "cargo mutants outcomes".to_owned(),
        source,
    })?;
    let detected = outcomes.caught.saturating_add(outcomes.timeout);
    let total = detected.saturating_add(outcomes.missed);
    if total == 0 {
        return Ok(Vec::new());
    }
    let score = f64::from(detected) * 100.0 / f64::from(total);
    if score >= target.min_score {
        return Ok(Vec::new());
    }
    let message = format!(
        "mutation score {score:.1}% is below {}% ({} mutants missed)",
        target.min_score, outcomes.missed
    );
    Ok(vec![Violation::new("mutants.out/outcomes.json", message)])
}

#[cfg(test)]
mod tests {
    use super::check_outcomes;
    use crate::thresholds::MutationTarget;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const TARGET: MutationTarget = MutationTarget { min_score: 90.0 };

    fn outcomes(caught: u32, missed: u32, timeout: u32) -> String {
        format!(
            r#"{{"caught":{caught},"missed":{missed},"timeout":{timeout},"unviable":7,"total_mutants":0}}"#
        )
    }

    #[test]
    fn passes_at_or_above_target() -> TestResult {
        assert!(check_outcomes(&outcomes(8, 1, 1), &TARGET)?.is_empty());
        Ok(())
    }

    #[test]
    fn fails_below_target() -> TestResult {
        let found = check_outcomes(&outcomes(8, 2, 0), &TARGET)?;
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].message,
            "mutation score 80.0% is below 90% (2 mutants missed)"
        );
        Ok(())
    }

    #[test]
    fn passes_when_there_is_nothing_to_mutate() -> TestResult {
        assert!(check_outcomes(&outcomes(0, 0, 0), &TARGET)?.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_malformed_outcomes() {
        assert!(check_outcomes("{}", &TARGET).is_err());
    }
}
```

- [ ] **Step 4: Wire the gate in**

`xtask/src/checks/mod.rs`: add `pub mod mutation;`. The final list is `complexity, coverage, deps, exceptions, manifests, mutation, names`.

`xtask/src/cli.rs`, after `CoverageGate`:
```rust
    /// Enforce the minimum mutation score.
    MutationGate {
        /// Path to `mutants.out/outcomes.json`.
        #[arg(long)]
        outcomes: PathBuf,
    },
```

`xtask/src/commands.rs`:
- Add this arm:
```rust
        Command::MutationGate { outcomes } => mutation(root, outcomes),
```
- Add this function:
```rust
fn mutation(root: &Path, outcomes: &Path) -> Result<Vec<Violation>, XtaskError> {
    let target = Thresholds::load(root)?.mutation;
    checks::mutation::check_outcomes(&files::read(outcomes)?, &target)
}
```

- [ ] **Step 5: Run the tests, and check the tool's output format once for real**

Run: `cargo test -p xtask`
Expected: PASS, with 33 unit tests and 14 integration tests in total.
Run: `cargo install --locked cargo-mutants@27.1.0` (skip if already installed), then:
```bash
cargo mutants -p xtask -f xtask/src/checks/mutation.rs
cargo xtask mutation-gate --outcomes mutants.out/outcomes.json; echo "exit $?"
```
Expected: cargo-mutants reports about 8 mutants, with 7 caught and 1 unviable, and the gate prints `exit 0`. Delete `mutants.out/` afterwards; it's gitignored anyway.

- [ ] **Step 6: Commit**

```bash
git add xtask
git commit -m "feat(xtask): gate the mutation score"
```

---

### Task 9: Tool configuration, `justfile`, git hooks, and a passing `just check`

**Files:**
- Create: `deny.toml`, `_typos.toml`, `justfile`, `lefthook.yml`

**Interfaces:**
- Produces: the `just` recipes `setup`, `check`, `fmt`, `lint`, `quality`, `deny`, `machete`, `typos`, `complexity`, `test`, `coverage` and `mutants diff=""`. CI (Task 10) calls `just check` and `just mutants`.

- [ ] **Step 1: Create `deny.toml`**

```toml
# cargo-deny: vulnerabilities, licenses, duplicate versions, and sources.
# Every skip or ignore added here is a reviewed exception: say why inline.

[graph]
all-features = true

[advisories]
yanked = "deny"
ignore = []

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Zlib",
]
confidence-threshold = 0.9
# The allow-list anticipates later phases; not every license is used yet.
unused-allowed-license = "allow"

[licenses.private]
ignore = true

[bans]
multiple-versions = "deny"
wildcards = "deny"
skip = []

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

- [ ] **Step 2: Create `_typos.toml`**

```toml
[files]
extend-exclude = ["docs/design/mockups/*", "spikes/**/target", "target"]
```

- [ ] **Step 3: Create the `justfile`**

```just
# Quarry task runner. `just check` runs exactly what CI runs.

set shell := ["bash", "-euo", "pipefail", "-c"]

coverage_report := "target/llvm-cov/coverage.json"
core_packages := "-p quarry-engine -p quarry-challenges -p quarry-store"

default: check

# Install every tool the gates need, at pinned versions, and the git hooks.
setup:
    cargo install --locked cargo-nextest@0.9.151 cargo-llvm-cov@0.9.1 cargo-deny@0.20.2 cargo-machete@0.9.2 typos-cli@1.52.0 cargo-mutants@27.1.0 rust-code-analysis-cli@0.0.25
    brew install just lefthook
    if command -v asdf >/dev/null; then asdf reshim rust; fi
    lefthook install

# Every gate, in order from fastest to slowest.
check: fmt lint quality deny machete typos complexity coverage

fmt:
    cargo fmt --all -- --check

lint:
    cargo clippy --workspace --all-targets -- -D warnings

# Workspace rules: module names, lint inheritance, dependency direction, exceptions.
quality:
    cargo xtask check-names
    cargo xtask check-manifests
    cargo xtask check-deps
    cargo xtask check-exceptions

deny:
    cargo deny check

machete:
    cargo machete

typos:
    typos

complexity:
    cargo xtask complexity

test:
    cargo nextest run --workspace

# Runs the tests under coverage, then enforces the per-crate thresholds.
coverage:
    mkdir -p target/llvm-cov
    cargo llvm-cov nextest --workspace --ignore-filename-regex '(^|/)main\.rs$' --json --summary-only --output-path {{coverage_report}}
    cargo xtask coverage-gate --report {{coverage_report}}

# Mutation testing of the core crates. Pass a diff file to test only changed code.
# cargo-mutants exits 2 when mutants are missed; the score gate decides pass/fail.
# With nothing to mutate it writes no outcomes.json, so there is nothing to gate.
mutants diff="":
    rm -rf mutants.out mutants.out.old
    status=0; cargo mutants {{core_packages}} {{ if diff != "" { "--in-diff " + diff } else { "" } }} || status=$?; \
    if [ "$status" -ne 0 ] && [ "$status" -ne 2 ]; then exit "$status"; fi
    if [ -f mutants.out/outcomes.json ]; then cargo xtask mutation-gate --outcomes mutants.out/outcomes.json; else echo "No mutants under the active filters; nothing to gate."; fi
```

- [ ] **Step 4: Create `lefthook.yml`**

```yaml
# Installed by `just setup`. Fast checks before each commit; the full gate
# (identical to CI) before each push.
pre-commit:
  parallel: true
  commands:
    fmt:
      glob: "*.rs"
      run: cargo fmt --all -- --check
    clippy:
      glob: "*.{rs,toml}"
      run: cargo clippy --workspace --all-targets -- -D warnings
pre-push:
  commands:
    check:
      run: just check
```

- [ ] **Step 5: Install every tool and the hooks**

Run: `brew install just && just setup`
Expected: the tools install, and `lefthook install` prints `sync hooks: ✔️ (pre-commit, pre-push)`.

- [ ] **Step 6: Run the full gate**

Run: `just check`
Expected: every recipe passes. If `cargo deny` fails:
- **License:** a dependency uses a license missing from `allow`. Add it only if it's permissive (MIT, BSD or Apache style), and note why.
- **Duplicate versions:** add a `[[bans.skip]]` entry with the crate, version and a reason.

Never weaken a category wholesale.

- [ ] **Step 7: Prove the gates catch problems.** Each command below must fail. Undo each change afterwards.
- `touch crates/engine/src/utils.rs && just quality` should fail with `catch-all module name`. Then `rm crates/engine/src/utils.rs`.
- Add `quarry-ui-kit = { path = "../ui-kit" }` under a new `[dependencies]` table in `crates/engine/Cargo.toml`, then run `just quality`. It should fail with `must not depend on quarry-ui-kit`. Then revert with `git checkout crates/engine/Cargo.toml`.

- [ ] **Step 8: Commit.** This commit also exercises the pre-commit hook.

```bash
git add deny.toml _typos.toml justfile lefthook.yml
git commit -m "build: add just check pipeline, cargo-deny policy and git hooks"
```

---

### Task 10: GitHub Actions, Dependabot and the PR template

**Files:**
- Create: `.github/workflows/ci.yml`, `.github/workflows/mutants.yml`, `.github/dependabot.yml`, `.github/pull_request_template.md`

**Interfaces:**
- Consumes: `just check` and `just mutants` (Task 9).
- Produces: the status checks named **`check`** (from CI) and **`mutants`**. Task 12's branch protection requires both.

- [ ] **Step 1: Create `.github/workflows/ci.yml`**

```yaml
name: CI

on:
  pull_request:
  push:
    branches: [main]

permissions:
  contents: read

concurrency:
  group: ci-${{ github.ref }}
  cancel-in-progress: true

env:
  CARGO_TERM_COLOR: always

jobs:
  check:
    name: check
    runs-on: macos-14
    timeout-minutes: 45
    steps:
      - uses: actions/checkout@v4
      - name: Install the toolchain from rust-toolchain.toml
        run: rustup toolchain install
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with:
          tool: just@1.59.0,cargo-nextest@0.9.151,cargo-llvm-cov@0.9.1,cargo-deny@0.20.2,cargo-machete@0.9.2,typos@1.52.0
      - name: Install rust-code-analysis-cli
        run: cargo install --locked rust-code-analysis-cli@0.0.25
      - name: Run every gate
        run: just check
```

- [ ] **Step 2: Create `.github/workflows/mutants.yml`**

```yaml
name: Mutation testing

on:
  pull_request:
  schedule:
    - cron: "0 6 * * 1" # Mondays 06:00 UTC: full run over the core crates
  workflow_dispatch:

permissions:
  contents: read

jobs:
  mutants:
    name: mutants
    runs-on: macos-14
    timeout-minutes: 120
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
      - name: Install the toolchain from rust-toolchain.toml
        run: rustup toolchain install
      - uses: Swatinem/rust-cache@v2
      - uses: taiki-e/install-action@v2
        with:
          tool: just@1.59.0,cargo-mutants@27.1.0
      - name: Diff against the base branch (pull requests only)
        if: github.event_name == 'pull_request'
        run: git diff origin/${{ github.base_ref }}...HEAD > pr.diff
      - name: Mutation test and gate
        run: just mutants ${{ github.event_name == 'pull_request' && 'pr.diff' || '' }}
```

- [ ] **Step 3: Create `.github/dependabot.yml`**

```yaml
version: 2
updates:
  - package-ecosystem: cargo
    directory: /
    schedule:
      interval: weekly
    open-pull-requests-limit: 5
  - package-ecosystem: github-actions
    directory: /
    schedule:
      interval: weekly
```

- [ ] **Step 4: Create `.github/pull_request_template.md`**

```markdown
## What and why

<!-- One paragraph. Link the spec section or issue. -->

## Extension points (spec §3.3)

- [ ] This change goes through an extension point (trait, registry, or a new file), or I explain below why none applies.
- **Existing files modified, and why:**

## Single responsibility (spec §7.1)

- [ ] Each new or changed function and module has a single responsibility.
- **One sentence per new module, without "and":**

## Quality

- [ ] `just check` passes locally.
- [ ] Bug fixes start with a failing test.
- [ ] Any new lint suppression is registered in `quality/exceptions.toml` with a reason.
- [ ] No completion, suggestion, or code-insertion behavior was added to the editor (spec §1).
```

- [ ] **Step 5: Validate the YAML syntax locally**

Run: `ruby -ryaml -e 'ARGV.each { |f| YAML.load_file(f); puts "ok #{f}" }' .github/workflows/*.yml .github/dependabot.yml`
Expected: `ok` for each file. The workflows themselves are proven by the first PR in Task 12.

- [ ] **Step 6: Commit**

```bash
git add .github
git commit -m "ci: run just check and mutation testing on macos-14"
```

---

### Task 11: README, CONTRIBUTING and ADRs 0001–0005

**Files:**
- Create: `README.md`, `CONTRIBUTING.md`, `docs/adr/0000-template.md`, `docs/adr/0001-gpui-over-tauri.md`, `docs/adr/0002-sqlite-built-in-database.md`, `docs/adr/0003-driver-native-crates.md`, `docs/adr/0004-extension-points-and-quality-gates.md`, `docs/adr/0005-unsigned-signing-ready-releases.md`

**Interfaces:** none (documentation). ADR 0006 (Task 16) follows the same template.

- [ ] **Step 1: Create `docs/adr/0000-template.md`**

```markdown
# ADR NNNN: Title

- **Status:** Proposed | Accepted | Superseded by ADR NNNN
- **Date:** YYYY-MM-DD

## Context
What forces are at play; what problem needs a decision.

## Decision
What we decided, stated plainly.

## Consequences
What becomes easier, what becomes harder, and what we must watch.
```

- [ ] **Step 2: Create the five ADRs.** Each uses the template, with Status `Accepted` and Date `2026-11-05`, and the following content:

`0001-gpui-over-tauri.md`, "GPUI (via gpui-kit) over Tauri":
- **Context:** The UI must feel native and fast enough to beat Electron-based Beekeeper Studio, and it should be written in Rust.
- **Decision:**
  - Use GPUI, drawn on the GPU through Metal, with `gpui-component`.
  - Depend on the **`gpui-kit`** crate, pinned with `=0.9.0`. It pins the matching GPUI snapshot (`gpui-pre 0.3.11`) and provides `gpui_kit::application()`.
  - Don't depend on the `gpui` or `gpui-pre` crates directly, because their versions must match exactly.
  - Upgrades are deliberate PRs.
- **Consequences:**
  - The whole app is Rust, renders at 120 fps, and gets real frosted-glass windows.
  - The documentation is thin and the API changes between versions, which is why there's a Phase 0 spike.
  - GPUI-specific code is confined to `app` and `ui-kit`.

`0002-sqlite-built-in-database.md`, "SQLite (rusqlite, bundled) as the built-in database":
- **Context:** The challenges need a database that ships inside the app, resets instantly and works offline.
- **Decision:**
  - Use bundled SQLite.
  - Challenge sandboxes are built from track templates and loaded into memory with the backup API.
- **Consequences:**
  - Nothing to install, and resets take about 1 ms.
  - The SQL dialect differs slightly from Postgres, so challenges avoid dialect-specific features or say when they use one.

`0003-driver-native-crates.md`, "tokio-postgres and mysql_async over sqlx":
- **Context:** ⌘. must cancel a running query on the server.
- **Decision:**
  - Use `tokio-postgres`, which has a native cancel request, and `mysql_async`, which can cancel with `KILL QUERY` on a second connection.
  - Each is wrapped behind `engine::Driver`.
- **Consequences:** Real cancellation and row streaming, at the cost of two driver implementations instead of one.

`0004-extension-points-and-quality-gates.md`, "Extension points, single responsibility and enforced quality gates":
- **Context:** The codebase must scale so that features are additive. The user requires strict, automated quality enforcement.
- **Decision:**
  - Features enter through traits and registries (spec §3.3).
  - Gates: clippy pedantic plus the restriction lints; per-function limits of cyclomatic 10, cognitive 15, 15 statements and 40 lines; and per-crate coverage of 95/90 for the core crates and 80/70 for the UI crates.
  - **`xtask` coverage is 90/85.** This is a tooling target chosen in the Phase 0 plan.
  - Mutation score ≥ 90%; cargo-deny; a registered-exceptions file.
  - **`cargo-geiger` is deferred.** Spec §7.4 lists it as a report on `unsafe` code in dependencies. Version 0.14.0 crashed while this plan was being verified: it reports a missing `tests/support.rs` on a standard `tests/support/mod.rs` layout. Our own crates still forbid `unsafe`, and `cargo-deny` covers known vulnerabilities. Revisit when a working release exists.
  - The workspace-specific gates live in `xtask`, and adding a gate is itself an extension point.
- **Consequences:**
  - Gates exist from the first commit, so there's never a backlog to clear.
  - Every exception is visible in review.
  - Some code will be split into more functions than a looser codebase would have.

`0005-unsigned-signing-ready-releases.md`, "Unsigned releases, with signing ready to switch on":
- **Context:**
  - Distribution is a personal tap, `dillo/homebrew-tap`.
  - Homebrew's Gatekeeper requirements apply to the official `homebrew/cask` repository, not to personal taps.
  - There's no Apple Developer ID yet.
- **Decision:**
  - Ship an ad-hoc signed app.
  - The cask's postflight step removes the quarantine flag, as `brewery.rb` already does.
  - The release workflow's codesign and `notarytool` steps run only when `APPLE_DEVELOPER_ID_*` secrets exist.
- **Consequences:**
  - No cost now and no rework later.
  - Downloading the DMG directly, outside Homebrew, shows a Gatekeeper warning until signing is switched on.

- [ ] **Step 3: Create `README.md`**

```markdown
# Quarry

A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.

> **Status:** Phase 0 (foundations). Not usable yet. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).

- Built-in SQLite playground with story-driven SQL challenge tracks
- PostgreSQL and MySQL connections for everyday work
- No autocomplete, ever: you write every query yourself

## Development

    brew install just
    just setup     # installs pinned tools and git hooks
    just check     # every gate, identical to CI

See [CONTRIBUTING.md](CONTRIBUTING.md) for the engineering standards.
```

- [ ] **Step 4: Create `CONTRIBUTING.md`**

```markdown
# Contributing to Quarry

The design spec is the source of truth: `docs/superpowers/specs/2026-11-04-quarry-design.md`. Section 7 has the full standards; this is the working summary.

## Principles

1. Dependencies flow one way. `cargo xtask check-deps` enforces it, and a new crate needs a rule in `xtask/src/checks/deps.rs`.
2. Logic lives in library crates; GPUI views stay thin.
3. **Single responsibility** for every function and module. If its honest name needs "and", split it. No `utils`, `helpers`, `misc` or `common` modules.
4. Deep modules: a narrow public API that hides a lot.
5. Errors are values. `thiserror` in libraries; no `unwrap`, `expect` or `panic!` outside tests.
6. No `unsafe`.
7. Tests first for the core crates. Bug fixes start with a failing test.
8. **Extension points first:** new features go through a trait or registry (spec §3.3).
9. Record significant decisions as ADRs in `docs/adr/`.

## Limits (per function, tests included)

Cyclomatic ≤ 10 · cognitive ≤ 15 · statements ≤ 15 · lines ≤ 40 · nesting ≤ 4 · arguments ≤ 5. Numbers live in `quality/thresholds.toml`.

## Coverage

Core crates (`engine`, `challenges`, `store`): 95% lines / 90% regions. UI crates (`ui-kit`, `app`): 80 / 70. `xtask`: 90 / 85. Mutation score ≥ 90% on the core crates.

## Exceptions

A lint suppression attribute needs a written reason **and** an entry in `quality/exceptions.toml`. A complexity waiver uses `lint = "quarry::complexity"` plus `function = "<name>"`. CI rejects unregistered, reasonless or stale entries.

## Workflow

- `just check` before you push (the pre-push hook runs it).
- Conventional Commits (`feat:`, `fix:`, `docs:`, `build:`, `ci:`, `refactor:`, `test:`).
- Fill in the PR template, including how many existing files you touched.

## Adding a quality gate

Gates are an extension point too: add `xtask/src/checks/<gate>.rs` (a pure function plus unit tests), one `Command` variant, one `commands.rs` arm, and an integration test in `xtask/tests/commands.rs`.
```

- [ ] **Step 5: Correct the spec's GPUI dependency details.** In `docs/superpowers/specs/2026-11-04-quarry-design.md`:
- In the §2 table, change the UI row's choice cell to `**GPUI** via **\`gpui-kit\`** (bundles GPUI + \`gpui-component\`), pinned to an exact crates.io version`.
- In §10, change the first risk's mitigation to start `Pin \`gpui-kit\` with \`=\`;`.
- In §7.4, change the `cargo-geiger` item to: `a \`cargo-geiger\` report (deferred: 0.14.0 fails on this workspace, see ADR 0004)`.

- [ ] **Step 6: Verify the docs pass the gates**

Run: `just typos`
Expected: PASS. If a real word is flagged, fix the spelling. If it's a proper noun, add it under `[default.extend-words]` in `_typos.toml`.

- [ ] **Step 7: Commit**

```bash
git add README.md CONTRIBUTING.md docs/adr docs/superpowers/specs
git commit -m "docs: add README, contributing guide and ADRs 0001-0005"
```

---

### Task 12: Publish to GitHub and protect `main`

**This task is outward-facing. Before running anything, stop and confirm with the user:**
1. Should `dillo/quarry` be **public or private**?
2. Is it OK to push and open a PR?

Branch protection on a **private** repository requires GitHub Pro or a team plan. If the user wants it private and doesn't have Pro, skip Step 5 and tell them.

**Files:** none.

**Interfaces:**
- Consumes: the `check` and `mutants` status checks (Task 10).
- Produces: the `origin` remote, a merged Phase 0 foundations PR, and a protected `main`.

- [ ] **Step 1: Create the repository with the main branch only.** It holds the spec and mockups that were committed before this plan.

```bash
gh repo create dillo/quarry --<public|private> --description "A native macOS SQL client for learning and real work" --source . --remote origin
git push -u origin main
```

- [ ] **Step 2: Push the branch and open the PR**

```bash
git push -u origin phase-0-foundations
gh pr create --base main --head phase-0-foundations --title "Phase 0: workspace foundations and quality gates" --body "Implements Tasks 1–11 of docs/superpowers/plans/2026-11-05-phase-0-foundations.md."
```

- [ ] **Step 3: Watch CI**

Run: `gh pr checks --watch`
Expected: `check` and `mutants` both pass. If `check` fails only in CI, compare tool versions with the pins in `ci.yml` and the `justfile` first.

- [ ] **Step 4: Merge**

Run: `gh pr merge --squash --delete-branch`, then `git checkout main && git pull`.

- [ ] **Step 5: Protect `main`**

```bash
gh api -X PUT repos/dillo/quarry/branches/main/protection --input - <<'EOF'
{
  "required_status_checks": { "strict": true, "contexts": ["check", "mutants"] },
  "enforce_admins": false,
  "required_pull_request_reviews": null,
  "restrictions": null,
  "required_linear_history": true,
  "allow_force_pushes": false,
  "allow_deletions": false
}
EOF
```
Expected: JSON echoing the protection settings.
Verify: `gh api repos/dillo/quarry/branches/main/protection --jq '.required_status_checks.contexts'` prints `["check","mutants"]`.

---

### Task 13: Spike scaffold and experiment (a), frosted glass

The spike is **throwaway**:
- It lives in `spikes/gpui-spike` as its own Cargo workspace. The root `Cargo.toml` already has `exclude = ["spikes"]`, so it sits outside every quality gate.
- Its job is to answer three questions with evidence, not to be good code.

**Files:**
- Create: `spikes/gpui-spike/Cargo.toml`, `spikes/gpui-spike/src/main.rs`, `spikes/gpui-spike/src/glass.rs`, `spikes/gpui-spike/src/fps.rs`
- Create: `spikes/gpui-spike/src/editor.rs` and `spikes/gpui-spike/src/table.rs`. Their full code is below; Tasks 14 and 15 run them.

**Interfaces:**
- Produces: `glass::aurora_background() -> Background` and `glass::panel() -> Div`, used by the editor and table experiments; and `fps::FrameCounter::{new, tick}`.

**API facts (verified against gpui-kit 0.9.0 and its pinned gpui-pre 0.3.11):**
- Start the app with `gpui_kit::application().with_assets(gpui_kit::assets::Assets).run(...)`.
- Call `gpui_kit::init(cx)` first.
- Open windows with `gpui_kit::open_window(options, cx, |window, cx| -> Entity<V>)`, which mounts `Root` for you.
- `WindowOptions` has `window_background: WindowBackgroundAppearance::Blurred` and `titlebar: Some(TitlebarOptions { title, appears_transparent, traffic_light_position })`.
- Gradients use `linear_gradient(angle, linear_color_stop(color, pct), linear_color_stop(..))`.

- [ ] **Step 1: Create the branch and the spike crate**

```bash
git checkout main && git pull && git checkout -b phase-0-spike
```

`spikes/gpui-spike/Cargo.toml`:
```toml
# Throwaway Phase 0 spike (design spec §9). Its own workspace, outside the
# quality gates; findings go to docs/spikes/phase-0-gpui.md.
[package]
name = "gpui-spike"
version = "0.0.0"
edition = "2024"
publish = false

[workspace]

[dependencies]
gpui-kit = { version = "=0.9.0", features = ["tree-sitter-sql"] }
```

`spikes/gpui-spike/src/main.rs`:
```rust
//! Phase 0 GPUI spike: `cargo run --release -- glass|editor|table`.

mod editor;
mod fps;
mod glass;
mod table;

use gpui_kit::*;

fn main() {
    let experiment = std::env::args().nth(1).unwrap_or_else(|| "glass".into());
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            match experiment.as_str() {
                "editor" => open(cx, editor::EditorSpike::new),
                "table" => open(cx, table::TableSpike::new),
                _ => open(cx, |_, _| glass::GlassSpike),
            }
            cx.activate(true);
        });
}

/// Opens the spike window: blurred background, transparent title bar.
fn open<V: Render + 'static>(
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
) {
    let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_background: WindowBackgroundAppearance::Blurred,
        titlebar: Some(TitlebarOptions {
            title: Some("Quarry spike".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(14.), px(14.))),
        }),
        ..Default::default()
    };
    gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| build(window, cx)))
        .expect("open window");
}
```

`spikes/gpui-spike/src/glass.rs`:
```rust
//! Experiment (a): real frosted glass behind a translucent Aurora gradient.

use gpui_kit::*;

pub struct GlassSpike;

/// A floating Aurora panel: faint white fill, 1px light inner border.
pub fn panel() -> Div {
    div()
        .rounded(px(10.))
        .bg(rgba(0xffffff0a))
        .border_1()
        .border_color(rgba(0xffffff14))
        .p_4()
}

/// The Aurora background with partial alpha so the window blur shows through.
pub fn aurora_background() -> Background {
    linear_gradient(
        135.,
        linear_color_stop(rgba(0x1b1530d0), 0.),
        linear_color_stop(rgba(0x0b1b22d0), 1.),
    )
}

impl Render for GlassSpike {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .bg(aurora_background())
            .text_color(rgb(0xe6e4f0))
            .child(
                div()
                    .w(px(220.))
                    .h_full()
                    .pt(px(44.))
                    .px_3()
                    .bg(rgba(0xffffff09))
                    .child("Sidebar: the desktop should blur through here"),
            )
            .child(
                div()
                    .flex_1()
                    .p(px(44.))
                    .child(panel().child("A floating glass panel over the aurora gradient")),
            )
    }
}
```

`spikes/gpui-spike/src/fps.rs`:
```rust
//! Counts rendered frames per second.

use std::time::{Duration, Instant};

pub struct FrameCounter {
    window_start: Instant,
    frames: u32,
    last_fps: u32,
}

impl FrameCounter {
    pub fn new() -> Self {
        Self { window_start: Instant::now(), frames: 0, last_fps: 0 }
    }

    /// Records one frame; returns the frames counted in the last full second.
    pub fn tick(&mut self) -> u32 {
        self.frames += 1;
        if self.window_start.elapsed() >= Duration::from_secs(1) {
            self.last_fps = self.frames;
            self.frames = 0;
            self.window_start = Instant::now();
        }
        self.last_fps
    }
}
```

`spikes/gpui-spike/src/editor.rs`:
```rust
//! Experiment (b): the gpui-component editor with SQL highlighting and no
//! completion of any kind. No completion provider is ever attached.

use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::*;

const SAMPLE: &str = "-- Which astronauts flew more than 3 missions?\nSELECT a.name, count(*) AS flights\nFROM astronauts a\nJOIN crew_assignments ca ON ca.astronaut_id = a.id\nGROUP BY a.name\nHAVING count(*) > 3\nORDER BY flights DESC;\n";

pub struct EditorSpike {
    state: Entity<EditorState>,
}

impl EditorSpike {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("sql")
                .line_number(true)
                .default_value(SAMPLE)
        });
        Self { state }
    }
}

impl Render for EditorSpike {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .pt(px(44.))
            .p_4()
            .bg(crate::glass::aurora_background())
            .child(crate::glass::panel().size_full().child(Editor::new(&self.state).h_full()))
    }
}
```

`spikes/gpui-spike/src/table.rs`:
```rust
//! Experiment (c): a virtualized 1,000,000-row table with a live FPS readout.

use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::*;

use crate::fps::FrameCounter;

const ROWS: usize = 1_000_000;
const NAMES: [&str; 6] = ["id", "name", "nationality", "missions", "hours", "status"];

struct MillionRows {
    columns: Vec<Column>,
}

impl MillionRows {
    fn new() -> Self {
        let columns = NAMES.iter().map(|n| Column::new(*n, *n).width(px(160.))).collect();
        Self { columns }
    }
}

/// Deterministic cell text, computed on demand so no rows are stored.
fn cell(row: usize, col: usize) -> SharedString {
    match col {
        0 => row.to_string().into(),
        1 => format!("Astronaut {row}").into(),
        2 => ["Kenya", "Japan", "Brazil", "Norway"][row % 4].into(),
        3 => (row % 17).to_string().into(),
        4 => format!("{:.1}", (row % 9973) as f64 * 1.7).into(),
        _ => if row % 3 == 0 { "active" } else { "retired" }.into(),
    }
}

impl TableDelegate for MillionRows {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        ROWS
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        cell(row_ix, col_ix)
    }
}

pub struct TableSpike {
    state: Entity<TableState<MillionRows>>,
    frames: FrameCounter,
}

impl TableSpike {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| TableState::new(MillionRows::new(), window, cx));
        Self { state, frames: FrameCounter::new() }
    }
}

impl Render for TableSpike {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let fps = self.frames.tick();
        // Keep rendering continuously so the counter measures sustained frame rate.
        window.request_animation_frame();
        div()
            .size_full()
            .pt(px(44.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(crate::glass::aurora_background())
            .child(format!("{fps} fps · {ROWS} rows"))
            .child(crate::glass::panel().flex_1().child(DataTable::new(&self.state)))
    }
}
```

- [ ] **Step 2: Build**

Run: `cd spikes/gpui-spike && cargo build --release`
Expected: the build succeeds. The first build compiles GPUI and takes several minutes. If an API signature differs from the code above, fix it until it compiles, and **write down every difference** for the report in Task 16.

- [ ] **Step 3: Run experiment (a)**

Run: `cargo run --release -- glass`, with a colorful window or wallpaper behind the app.

Pass criteria (record each one as pass or fail, with a screenshot, in Task 16):
1. The desktop behind the window is visibly **blurred** through the sidebar and background.
2. The traffic lights sit at about (14, 14) over a transparent title bar.
3. The glass panel's 1px light border and 10px radius look crisp on a Retina display.
4. Resizing the window is smooth, with no flicker.

Save screenshots to `docs/spikes/img/glass.png` (⇧⌘4, then Space, then click the window).

- [ ] **Step 4: Commit**

```bash
git add spikes docs/spikes
git commit -m "spike: GPUI frosted-glass window with Aurora gradient"
```

---

### Task 14: Experiment (b), a completion-free SQL editor

**Files:**
- Already created: `spikes/gpui-spike/src/editor.rs` (Task 13)

**Facts (verified in gpui-base 0.9.0 source):**
- The editor's completion menu runs only when `lsp.completion_provider` is `Some`, and its default is `None` (`input/editor/lsp/mod.rs`). The spike never sets a provider.
- SQL highlighting comes from the `tree-sitter-sql` feature.

- [ ] **Step 1: Run experiment (b)**

Run: `cargo run --release -- editor`

Pass criteria (record each one):
1. SQL keywords, functions, strings, numbers and comments are highlighted distinctly.
2. Line numbers show.
3. **No completion UI appears** in any of these cases:
   - typing `SEL` and pausing for 2 s;
   - typing `a.` after `FROM astronauts a`;
   - pressing ⌃Space;
   - pressing ⌘I;
   - typing `(` after `count`.
4. Multiple cursors work (⌥-click or ⌘D), and so do undo and redo.
5. Typing stays responsive in a 5,000-line buffer. To test this, paste the sample about 700 times.

- [ ] **Step 2: Check the editor source for any other suggestion paths**

Run: `grep -rn "completion_provider\|fn show_completions\|inline_completion\|word_completion" ~/.cargo/registry/src/*/gpui-base-0.9.0/src/input | head -20` (with asdf, use `~/.asdf/installs/rust/1.98.1/registry/src/...`).
Expected: every completion path is guarded by `completion_provider`. Record any unguarded suggestion feature, such as word-based or inline completion. That would be a fail for this experiment.

- [ ] **Step 3: Commit the notes and screenshots**

```bash
git add docs/spikes
git commit -m "spike: verify SQL editor highlighting with no completion paths"
```

---

### Task 15: Experiment (c), a virtualized 1M-row table at 120 fps

**Files:**
- Already created: `spikes/gpui-spike/src/table.rs` (Task 13)

- [ ] **Step 1: Run experiment (c)**

Run: `cargo run --release -- table` on the built-in ProMotion display.

Pass criteria (record each one, with numbers):
1. The window opens in under 1 s, even with 1,000,000 rows.
2. The FPS readout stays at **≥ 110 fps** while flick-scrolling, and ≥ 58 fps on a 60 Hz display.
3. Dragging the scrollbar to the end shows row `999999` with no stall.
4. Memory stays under 300 MB. Check the spike's process in Activity Monitor.
5. Columns can be resized by dragging.

- [ ] **Step 2: Profile if it falls short.** If criterion 2 fails, record a 10 s trace with Instruments, using the Time Profiler and the Metal System Trace (`xcrun xctrace record --template 'Time Profiler' --launch -- target/release/gpui-spike table`), and note the hottest frames.

- [ ] **Step 3: Commit the measurements**

```bash
git add docs/spikes
git commit -m "spike: measure 1M-row virtualized table frame rate"
```

---

### Task 16: Spike report, ADR 0006 and the Phase 0 exit decision

**Files:**
- Create: `docs/spikes/phase-0-gpui.md`, `docs/adr/0006-phase-0-spike-outcome.md`

**Interfaces:**
- Produces: the go/no-go decisions that Phase 1 planning reads.

- [ ] **Step 1: Write `docs/spikes/phase-0-gpui.md` with this structure, filled in from Tasks 13–15**

```markdown
# Phase 0 GPUI spike: findings

Versions: gpui-kit 0.9.0 (gpui-pre 0.3.11), Rust 1.98.1, macOS <version>, <Mac model>.

## (a) Frosted glass: PASS | FAIL
| Criterion | Result | Notes |
|---|---|---|
| Desktop blurs through | | |
| Traffic lights at (14,14), transparent title bar | | |
| Crisp 1px border / 10px radius | | |
| Smooth resize | | |
![glass](img/glass.png)

## (b) Completion-free SQL editor: PASS | FAIL
| Criterion | Result | Notes |
|---|---|---|
| SQL highlighting | | |
| Line numbers | | |
| No completion UI (5 triggers) | | |
| Multi-cursor, undo/redo | | |
| 5,000-line responsiveness | | |
Source audit: <every completion path and its guard>

## (c) 1M-row table: PASS | FAIL
| Criterion | Result | Measured |
|---|---|---|
| Opens < 1 s | | |
| ≥ 110 fps flick-scroll (ProMotion) | | |
| Jump to row 999999 | | |
| Memory < 300 MB | | |
| Column resize | | |

## API differences from the plan
<each signature you had to change in Task 13, or "none">

## Recommendations for Phase 1
<what to build on directly, what needs custom work, risks to carry>
```

- [ ] **Step 2: Write ADR 0006 using the template.** Status `Accepted`. Record one decision per experiment:
- **(a)** Use `WindowBackgroundAppearance::Blurred`, or the fallback you documented.
- **(b)** Use the `gpui-component` editor, or, if it failed, **build our own editor on GPUI's text input primitives** (spec §10).
- **(c)** Use `DataTable`, or a custom `v_virtual_list` grid.

- [ ] **Step 3: Tell the user the results.** Summarize each experiment as PASS or FAIL in one line, and say whether Phase 1 can start as specified or needs the documented fallbacks.

- [ ] **Step 4: Commit and open the spike PR**

```bash
git add docs/spikes docs/adr/0006-phase-0-spike-outcome.md
git commit -m "docs: record Phase 0 GPUI spike findings and ADR 0006"
git push -u origin phase-0-spike
gh pr create --base main --title "Phase 0: GPUI spike findings" --body "Spike results and ADR 0006. See docs/spikes/phase-0-gpui.md."
```

Expected: CI `check` passes. The spike is outside the workspace, so the gates ignore it; `typos` still scans its source.

---

## Phase 0 exit criteria (spec §9)

- [ ] `just check` passes locally and in CI on `main`. CI runs format, lint, the workspace rules, deny, machete, typos, complexity and coverage.
- [ ] The mutation workflow runs on PRs and weekly.
- [ ] `main` is protected and requires the `check` and `mutants` checks (or protection was skipped and the user was told why).
- [ ] ADRs 0001–0006 are written, and the spike report gives PASS/FAIL with evidence for (a), (b) and (c).
