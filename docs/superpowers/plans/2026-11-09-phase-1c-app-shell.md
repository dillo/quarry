# Phase 1C: App Shell Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn the `quarry` app crate into the first usable Quarry window. It opens on a built-in sample database, with:
- a schema sidebar,
- a completion-free SQL editor,
- Run (⌘↵) and Cancel (⌘.) of the selection or the statement under the cursor,
- a virtualized results grid that streams,
- a live status bar,
- window commands (⌘\\ sidebar, ⌘⇧T theme, ⌘1 and ⌘2 focus),
- keyboard focus for interactive rows, tabs and segments.

**Architecture:**
- **`model`:** GPUI-free view models. They are pure and fully unit-tested and mutation-tested.
- **`views`:** GPUI views that render the models. The workspace is split into state (`mod.rs`), run driving (`runner.rs`) and drawing (`layout.rs`).
- **`shell`:** actions, key bindings, menus and the main window.
- **Polling:** the workspace polls the engine `Session`'s channels every 33 ms instead of awaiting them. This keeps headless tests deterministic and batches redraws.
- **Status:** every task's code was built and gated in a scratch clone first, and the tasks were replayed in order on a fresh clone of `main` with fmt, clippy, tests, machete, complexity and typos passing after each.

**Tech Stack:** Rust 1.98.1, `gpui-kit` =0.9.0 (`gpui-component` editor, `DataTable`, `v_resizable`), `async-channel`, `quarry-engine` (Phase 1A), `quarry-ui-kit` (Phase 1B).

**Spec:** `docs/superpowers/specs/2026-11-04-quarry-design.md` §4.2 (Workspace) and §9 (Phase 1). The user split Phase 1's remainder into this plan (1C) and Phase 1D: tabs with drafts, Run all, the Messages and Plan views, ⌘K, and copy/export. ADR 0006 (the editor never gets a completion provider), ADR 0007 (Session usage) and ADR 0008 (Aurora).

## Global Constraints

- **Branch and commits:** create the branch `phase-1c-app-shell` from `main`; this plan is its first commit. Use Conventional Commits, with no trailers.
- **Dependency direction:** `quarry` may depend on `quarry-ui-kit`, `quarry-engine`, `quarry-challenges` and `quarry-store`; `check-deps` enforces it. `quarry-ui-kit` still depends on no workspace crate.
- **Coverage:** `app` and `ui-kit` need at least 80% lines and 70% regions at the end of every task. Every file in `crates/app/src/model` must stay at 100%.
- **Mutation testing:** at least 90% caught, and in practice 0 missed, for the core crates and `crates/app/src/model` / `crates/app/src/sample` (`.cargo/mutants.toml`).
- **Per function:** cyclomatic ≤ 10, cognitive ≤ 15, ≤ 15 statements, ≤ 40 lines, ≤ 5 arguments, nesting ≤ 4. This applies to tests too.
- **Banned outside `#[test]` functions:** `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!` and `dbg!`. Test helpers that aren't `#[test]` functions count as outside. No lint suppression attributes, and no `as` casts.
- **No completion of any kind:** never attach a completion, hover, definition or code-action provider to the editor. A test asserts `lsp().completion_provider.is_none()`.
- **Contrast:** every new color goes through a `Role`, and `tests/contrast.rs` covers it. Change palette values only together with that test.
- **Headless testing:**
  - Interactive roots that tests find by id call `.test_support()`.
  - Tests never await the session.
  - **Every task ends with `just check` passing.** Don't commit anything under `.superpowers/`.

## File Structure

```
crates/app/
  Cargo.toml                 lib + bin; gpui-kit (assets), async-channel, quarry-engine, quarry-ui-kit
  src/main.rs                start-up only (excluded from coverage)
  src/lib.rs                 module map
  src/sample/{mod.rs, seed.sql}      built-in sample database
  src/model/                 GPUI-free view models (mutation-tested)
    target.rs                what a Run sends
    run.rs                   RunModel / Phase from RunEvents
    drain.rs                 non-blocking channel drain
    format.rs                cells, counts, durations, positions
    status.rs                status-bar summary, failure banner text
    schema_tree.rs           schema-tree rows
  src/views/
    editor.rs                the SQL editor (no providers)
    results.rs               ResultsDelegate for DataTable
    schema.rs                SchemaTree view
    workspace/{mod.rs, runner.rs, layout.rs}
  src/shell/{mod.rs, actions.rs, window.rs}
  tests/{sample.rs, workspace.rs, shell.rs}
crates/ui-kit/               FocusRing role, keyboard_focusable, Preference::next
.cargo/mutants.toml          mutation scope
docs/adr/0009-app-shell.md
```

---


### Task 1: The app crate as a library, with the sample database

Quarry's binary crate gains a library half so views and models are testable. The first module opens the built-in sample database: an in-memory SQLite copy seeded from `seed.sql`, a small space agency with 200,000 telemetry rows to stream and cancel.

**Files:**
- Modify: `crates/app/Cargo.toml`
- Create: `crates/app/src/lib.rs`
- Create: `crates/app/src/sample/seed.sql`
- Create: `crates/app/src/sample/mod.rs`
- Create: `crates/app/tests/sample.rs`
- Modify: `deny.toml`

**Interfaces:**
- Consumes: `quarry_engine::sqlite::SqliteDriver::{open_in_memory, execute_script}`, `quarry_engine::session::Session::{start, run, schema}`.
- Produces: `quarry::sample::{open() -> Result<Session, EngineError>, SEED: &str, NAME: &str}`.

- [ ] **Step 1: Write the failing tests**

`crates/app/tests/sample.rs`:
```rust
//! The built-in sample database opens, seeds and answers queries.

use quarry::sample;
use quarry_engine::{schema::TableKind, session::RunEvent, value::Value};

fn first_cell(sql: &str) -> Option<Value> {
    let session = sample::open().ok()?;
    let handle = session.run(sql);
    while let Ok(event) = handle.events.recv_blocking() {
        if let RunEvent::Rows(rows) = event {
            return rows.into_iter().next()?.into_iter().next();
        }
    }
    None
}

#[test]
fn the_sample_has_a_large_table_to_stream() {
    assert_eq!(
        first_cell("SELECT count(*) FROM telemetry"),
        Some(Value::Int(200_000))
    );
}

#[test]
fn the_sample_has_nulls_to_show() {
    assert_eq!(
        first_cell("SELECT count(*) FROM astronauts WHERE call_sign IS NULL"),
        Some(Value::Int(5))
    );
}

#[test]
fn the_schema_lists_every_table_and_the_view() {
    let session = sample::open();
    let schema = session
        .as_ref()
        .ok()
        .and_then(|session| session.schema().recv_blocking().ok())
        .and_then(Result::ok)
        .unwrap_or_default();
    let names: Vec<(&str, TableKind)> = schema
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table.kind))
        .collect();
    assert_eq!(
        names,
        [
            ("astronauts", TableKind::Table),
            ("crew_assignments", TableKind::Table),
            ("mission_crews", TableKind::View),
            ("missions", TableKind::Table),
            ("spacecraft", TableKind::Table),
            ("telemetry", TableKind::Table),
        ]
    );
}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry --test sample`
Expected: compile error, `quarry` has no library target (`could not find `sample``) or `unresolved import`.

- [ ] **Step 3: Implement**

`crates/app/Cargo.toml`:
```toml
[package]
name = "quarry"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true
license.workspace = true

[lib]
path = "src/lib.rs"

[[bin]]
name = "quarry"
path = "src/main.rs"

[dependencies]
quarry-engine = { path = "../engine" }

[lints]
workspace = true
```

`crates/app/src/lib.rs`:
```rust
//! Quarry: a native macOS SQL client for learning and real work.

pub mod sample;
```

`crates/app/src/sample/seed.sql`:
```sql
-- The built-in sample database: a small space agency.
CREATE TABLE spacecraft (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  crew_capacity INTEGER NOT NULL
);
INSERT INTO spacecraft (id, name, crew_capacity) VALUES
  (1, 'Kestrel', 3), (2, 'Heron', 4), (3, 'Albatross', 7), (4, 'Wren', 2);

CREATE TABLE astronauts (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  nationality TEXT NOT NULL,
  joined INTEGER NOT NULL,
  call_sign TEXT
);
INSERT INTO astronauts (id, name, nationality, joined, call_sign) VALUES
  (1, 'Amara Okafor', 'Nigeria', 2019, 'Comet'),
  (2, 'Kenji Watanabe', 'Japan', 2017, NULL),
  (3, 'Lucía Fernández', 'Argentina', 2021, 'Pampa'),
  (4, 'Ingrid Solberg', 'Norway', 2016, 'Fjord'),
  (5, 'Ravi Menon', 'India', 2018, NULL),
  (6, 'Chloé Martin', 'France', 2020, 'Mistral'),
  (7, 'Tomasz Nowak', 'Poland', 2022, NULL),
  (8, 'Aiyana Redcloud', 'United States', 2015, 'Hawk'),
  (9, 'Mei Lin', 'Singapore', 2019, 'Lotus'),
  (10, 'Kwame Mensah', 'Ghana', 2023, NULL),
  (11, 'Sofia Rossi', 'Italy', 2018, 'Vesta'),
  (12, 'Jonas Becker', 'Germany', 2016, NULL);

CREATE TABLE missions (
  id INTEGER PRIMARY KEY,
  name TEXT NOT NULL,
  launched TEXT NOT NULL,
  spacecraft_id INTEGER NOT NULL REFERENCES spacecraft (id),
  outcome TEXT NOT NULL CHECK (outcome IN ('success', 'partial', 'aborted'))
);
INSERT INTO missions (id, name, launched, spacecraft_id, outcome) VALUES
  (1, 'Dawn Relay', '2020-03-14', 1, 'success'),
  (2, 'Cold Harbor', '2020-09-02', 2, 'success'),
  (3, 'Tidewatch', '2021-01-21', 1, 'partial'),
  (4, 'Long Meridian', '2021-07-30', 3, 'success'),
  (5, 'Glass Orchard', '2022-02-11', 2, 'aborted'),
  (6, 'Northern Choir', '2022-10-05', 3, 'success'),
  (7, 'Quiet Lantern', '2023-04-18', 4, 'success'),
  (8, 'Ember Field', '2023-11-27', 3, 'partial'),
  (9, 'Salt Comet', '2024-06-09', 2, 'success'),
  (10, 'Far Orchard', '2025-02-03', 3, 'success');

CREATE TABLE crew_assignments (
  mission_id INTEGER NOT NULL REFERENCES missions (id),
  astronaut_id INTEGER NOT NULL REFERENCES astronauts (id),
  role TEXT NOT NULL,
  PRIMARY KEY (mission_id, astronaut_id)
);
INSERT INTO crew_assignments (mission_id, astronaut_id, role) VALUES
  (1, 1, 'commander'), (1, 4, 'pilot'), (1, 8, 'engineer'),
  (2, 2, 'commander'), (2, 5, 'pilot'), (2, 9, 'scientist'), (2, 12, 'engineer'),
  (3, 4, 'commander'), (3, 6, 'pilot'),
  (4, 8, 'commander'), (4, 1, 'pilot'), (4, 3, 'scientist'), (4, 11, 'engineer'),
  (4, 5, 'medic'), (4, 9, 'scientist'),
  (5, 2, 'commander'), (5, 7, 'pilot'),
  (6, 1, 'commander'), (6, 4, 'pilot'), (6, 6, 'scientist'), (6, 8, 'engineer'),
  (7, 9, 'commander'), (7, 3, 'pilot'),
  (8, 11, 'commander'), (8, 1, 'pilot'), (8, 10, 'engineer'), (8, 12, 'scientist'),
  (9, 5, 'commander'), (9, 6, 'pilot'), (9, 4, 'engineer'),
  (10, 8, 'commander'), (10, 1, 'pilot'), (10, 9, 'scientist'), (10, 3, 'medic');

-- 200,000 readings, so streaming and cancelling have something to chew on.
CREATE TABLE telemetry (
  id INTEGER PRIMARY KEY,
  mission_id INTEGER NOT NULL REFERENCES missions (id),
  seconds INTEGER NOT NULL,
  altitude_km REAL NOT NULL,
  velocity_kms REAL NOT NULL
);
WITH RECURSIVE n(i) AS (SELECT 1 UNION ALL SELECT i + 1 FROM n WHERE i < 200000)
INSERT INTO telemetry (id, mission_id, seconds, altitude_km, velocity_kms)
SELECT i, 1 + (i % 10), i / 10, round(400 + (i % 997) * 0.25, 2), round(7.6 + (i % 113) * 0.001, 3)
FROM n;

CREATE VIEW mission_crews AS
SELECT m.name AS mission, a.name AS astronaut, c.role
FROM crew_assignments c
JOIN missions m ON m.id = c.mission_id
JOIN astronauts a ON a.id = c.astronaut_id;
```

`crates/app/src/sample/mod.rs`:
```rust
//! The built-in sample database every new workspace opens.

use quarry_engine::{error::EngineError, session::Session, sqlite::SqliteDriver};

/// The script that creates and fills the sample database.
pub const SEED: &str = include_str!("seed.sql");

/// What the status bar calls the sample database.
pub const NAME: &str = "Sample · in memory";

/// Opens a fresh in-memory copy of the sample database on its own session.
///
/// # Errors
/// Returns the engine's error if `SQLite` cannot open or seed the database.
pub fn open() -> Result<Session, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script(SEED)?;
    Ok(Session::start(Box::new(driver)))
}
```

Edit `deny.toml`:

Replace:
```toml
wildcards = "deny"
```
with:
```toml
wildcards = "deny"
# Workspace crates depend on each other by path; none is published.
allow-wildcard-paths = true
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry --test sample`
Expected: 3 tests pass.

Run: `just check`
Expected: exit 0.

`cargo deny check` needs `allow-wildcard-paths`, because `quarry` now depends on a workspace crate by path; the `deny.toml` edit below is part of this step.

- [ ] **Step 5: Commit**

```bash
git add crates/app/Cargo.toml crates/app/src/lib.rs crates/app/src/sample/mod.rs crates/app/src/sample/seed.sql crates/app/tests/sample.rs deny.toml
git commit -m "feat(app): add the library target and the built-in sample database"
```


---

### Task 2: Run models: target, run state and channel draining

The pure core of running a query: which text a Run sends, how `RunEvent`s fold into the state the grid and status bar show, and how the UI takes waiting events without awaiting. The app's view models join mutation testing (ADR 0009, written in Task 7). Each file carries its tests in a `#[cfg(test)]` module: write that module first, run `cargo nextest run -p quarry --lib` and see it fail to compile, then add the code above it.

**Files:**
- Create: `crates/app/Cargo.toml`
- Create: `crates/app/src/lib.rs`
- Create: `crates/app/src/model/mod.rs`
- Create: `crates/app/src/model/target.rs`
- Create: `crates/app/src/model/run.rs`
- Create: `crates/app/src/model/drain.rs`
- Create: `.cargo/mutants.toml`
- Modify: `justfile`

**Interfaces:**
- Consumes: `quarry_engine::splitter::{split, statement_at}`, `quarry_engine::session::RunEvent`, `async_channel::Receiver`.
- Produces: `model::target::run_target(&str, Range<usize>) -> Option<Range<usize>>`; `model::run::{RunModel, Phase}` with `RunModel::{default, started(Instant), apply(RunEvent), phase, columns, rows, statements, is_running, elapsed(Instant)}`; `model::drain::{drain(&Receiver<T>) -> Drained<T>, Drained { items, closed }}`.

- [ ] **Step 1: Implement**

`crates/app/Cargo.toml`:
```toml
[package]
name = "quarry"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true
license.workspace = true

[lib]
path = "src/lib.rs"

[[bin]]
name = "quarry"
path = "src/main.rs"

[dependencies]
async-channel.workspace = true
quarry-engine = { path = "../engine" }

[lints]
workspace = true
```

`crates/app/src/lib.rs`:
```rust
//! Quarry: a native macOS SQL client for learning and real work.
//!
//! [`model`] holds the GPUI-free view models.

pub mod model;
pub mod sample;
```

`crates/app/src/model/mod.rs`:
```rust
//! View models: plain data and pure functions the views render.
//!
//! Nothing here depends on GPUI, so every rule is unit-tested and
//! mutation-tested like the core crates.

pub mod drain;
pub mod run;
pub mod target;
```

`crates/app/src/model/target.rs`:
```rust
//! Which part of the editor a Run executes.

use std::ops::Range;

use quarry_engine::splitter;

/// The byte range a Run sends: the selection when one exists, otherwise the
/// statement under the cursor. `None` when there is nothing to run.
pub fn run_target(text: &str, selection: Range<usize>) -> Option<Range<usize>> {
    if selection.is_empty() {
        return splitter::statement_at(text, selection.end);
    }
    text.get(selection.clone())
        .filter(|selected| !splitter::split(selected).is_empty())
        .map(|_| selection)
}

#[cfg(test)]
mod tests {
    use super::run_target;

    const SCRIPT: &str = "SELECT 1;\nSELECT 2;\n";

    #[test]
    fn a_cursor_runs_the_statement_it_sits_in() {
        assert_eq!(run_target(SCRIPT, 3..3), Some(0..8));
        assert_eq!(run_target(SCRIPT, 12..12), Some(10..18));
    }

    #[test]
    fn a_selection_runs_exactly_what_is_selected() {
        assert_eq!(run_target(SCRIPT, 0..18), Some(0..18));
    }

    #[test]
    fn a_selection_of_only_comments_runs_nothing() {
        assert_eq!(run_target("-- note\nSELECT 1", 0..7), None);
    }

    #[test]
    fn an_empty_editor_runs_nothing() {
        assert_eq!(run_target("", 0..0), None);
        assert_eq!(run_target("   ", 2..2), None);
    }

    #[test]
    fn a_selection_off_a_char_boundary_runs_nothing() {
        assert_eq!(run_target("SELECT 'é'", 9..10), None);
    }
}
```

`crates/app/src/model/run.rs`:
```rust
//! The state of the latest run, built from the engine's [`RunEvent`]s.

use std::time::{Duration, Instant};

use quarry_engine::{
    error::EngineError,
    result::{Column, Row, Summary},
    session::RunEvent,
};

/// Where a run is in its life.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Nothing has run yet.
    Idle,
    /// Statements are running; rows may still arrive.
    Running,
    /// Every statement finished; the summary is the last statement's.
    Finished(Summary),
    /// A statement failed or the run was cancelled.
    Failed(EngineError),
}

/// The latest run: its phase and the result of its most recent statement.
///
/// The grid shows one result. When a run has several statements, each
/// `Started` replaces the previous statement's columns and rows.
#[derive(Debug, Clone)]
pub struct RunModel {
    phase: Phase,
    columns: Vec<Column>,
    rows: Vec<Row>,
    started_at: Option<Instant>,
    statements: usize,
}

impl Default for RunModel {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            columns: Vec::new(),
            rows: Vec::new(),
            started_at: None,
            statements: 0,
        }
    }
}

impl RunModel {
    /// A run that began at `now`, with no results yet.
    pub fn started(now: Instant) -> Self {
        Self {
            phase: Phase::Running,
            started_at: Some(now),
            ..Self::default()
        }
    }

    /// Folds one event into the model.
    pub fn apply(&mut self, event: RunEvent) {
        match event {
            RunEvent::Started { .. } => self.begin_statement(),
            RunEvent::Columns(columns) => self.columns = columns,
            RunEvent::Rows(batch) => self.rows.extend(batch),
            RunEvent::Finished(summary) => self.phase = Phase::Finished(summary),
            RunEvent::Failed(error) => self.phase = Phase::Failed(error),
            RunEvent::Done => self.finish(),
        }
    }

    /// Where the run is.
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// The current statement's columns (empty for statements without rows).
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The current statement's rows received so far.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Statements started so far in this run.
    pub fn statements(&self) -> usize {
        self.statements
    }

    /// Whether rows may still arrive.
    pub fn is_running(&self) -> bool {
        self.phase == Phase::Running
    }

    /// Time since the run began, for the live timer; zero before any run.
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.started_at
            .map_or(Duration::ZERO, |start| now.saturating_duration_since(start))
    }

    fn begin_statement(&mut self) {
        self.phase = Phase::Running;
        self.columns.clear();
        self.rows.clear();
        self.statements += 1;
    }

    /// `Done` without `Finished` or `Failed` means the worker vanished.
    fn finish(&mut self) {
        if self.phase == Phase::Running {
            self.phase = Phase::Failed(EngineError::WorkerGone);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use quarry_engine::{
        error::EngineError,
        result::{Column, Summary},
        session::RunEvent,
        value::Value,
    };

    use super::{Phase, RunModel};

    fn column(name: &str) -> Column {
        Column {
            name: name.to_owned(),
            type_name: String::new(),
        }
    }

    fn summary(rows: u64) -> Summary {
        Summary {
            rows,
            rows_affected: None,
            elapsed: Duration::from_millis(2),
        }
    }

    fn started(index: usize) -> RunEvent {
        RunEvent::Started { index, range: 0..1 }
    }

    #[test]
    fn a_new_model_is_idle_and_empty() {
        let model = RunModel::default();
        assert_eq!(model.phase(), &Phase::Idle);
        assert!(model.columns().is_empty() && model.rows().is_empty());
        assert!(!model.is_running());
        assert_eq!(model.statements(), 0);
    }

    #[test]
    fn rows_stream_in_and_the_summary_finishes_the_run() {
        let mut model = RunModel::started(Instant::now());
        assert!(model.is_running());
        model.apply(started(0));
        model.apply(RunEvent::Columns(vec![column("n")]));
        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]]));
        model.apply(RunEvent::Rows(vec![vec![Value::Int(2)]]));
        assert_eq!(model.rows().len(), 2);
        assert!(model.is_running());
        model.apply(RunEvent::Finished(summary(2)));
        model.apply(RunEvent::Done);
        assert_eq!(model.phase(), &Phase::Finished(summary(2)));
        assert_eq!(model.columns(), [column("n")]);
        assert_eq!(model.statements(), 1);
    }

    #[test]
    fn each_statement_replaces_the_previous_result() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(RunEvent::Columns(vec![column("a")]));
        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]]));
        model.apply(RunEvent::Finished(summary(1)));
        model.apply(started(1));
        assert!(model.is_running());
        assert!(model.columns().is_empty() && model.rows().is_empty());
        assert_eq!(model.statements(), 2);
    }

    #[test]
    fn a_failure_keeps_its_error() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(RunEvent::Failed(EngineError::Cancelled));
        model.apply(RunEvent::Done);
        assert_eq!(model.phase(), &Phase::Failed(EngineError::Cancelled));
        assert!(!model.is_running());
    }

    #[test]
    fn done_while_still_running_means_the_worker_is_gone() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(RunEvent::Done);
        assert_eq!(model.phase(), &Phase::Failed(EngineError::WorkerGone));
    }

    #[test]
    fn elapsed_counts_from_the_start_and_is_zero_when_idle() {
        let start = Instant::now();
        let model = RunModel::started(start);
        let later = start + Duration::from_millis(1500);
        assert_eq!(model.elapsed(later), Duration::from_millis(1500));
        assert_eq!(RunModel::default().elapsed(later), Duration::ZERO);
    }
}
```

`crates/app/src/model/drain.rs`:
```rust
//! Takes everything waiting on a channel without blocking or awaiting.
//!
//! The UI polls the session's channels once a frame instead of awaiting
//! them: a waker fired from the session thread would wake GPUI's executor
//! from outside, and polling also batches many row events into one redraw.

use async_channel::{Receiver, TryRecvError};

/// What one poll found.
#[derive(Debug, PartialEq)]
pub struct Drained<T> {
    /// Every message that was waiting, in order.
    pub items: Vec<T>,
    /// Whether the sender is gone, so nothing more will arrive.
    pub closed: bool,
}

/// Takes every waiting message from `receiver`.
pub fn drain<T>(receiver: &Receiver<T>) -> Drained<T> {
    let mut items = Vec::new();
    loop {
        match receiver.try_recv() {
            Ok(item) => items.push(item),
            Err(TryRecvError::Empty) => {
                return Drained {
                    items,
                    closed: false,
                };
            }
            Err(TryRecvError::Closed) => {
                return Drained {
                    items,
                    closed: true,
                };
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Drained, drain};

    #[test]
    fn takes_everything_waiting_and_stays_open() {
        let (sender, receiver) = async_channel::unbounded();
        sender.try_send(1).unwrap();
        sender.try_send(2).unwrap();
        assert_eq!(
            drain(&receiver),
            Drained {
                items: vec![1, 2],
                closed: false
            }
        );
        assert_eq!(
            drain(&receiver),
            Drained {
                items: vec![],
                closed: false
            }
        );
    }

    #[test]
    fn reports_a_closed_channel_after_its_last_items() {
        let (sender, receiver) = async_channel::unbounded();
        sender.try_send(7).unwrap();
        drop(sender);
        assert_eq!(
            drain(&receiver),
            Drained {
                items: vec![7],
                closed: true
            }
        );
    }
}
```

`.cargo/mutants.toml`:
```toml
# cargo-mutants settings (`just mutants`).
#
# The app's GPUI views and start-up code render and wire things; headless
# tests cover them, but most mutants there change only pixels. Its view
# models (`crates/app/src/model`, `crates/app/src/sample`) are mutation-tested
# like the core crates (ADR 0009).
exclude_globs = [
    "crates/app/src/views/**",
    "crates/app/src/shell/**",
    "crates/app/src/main.rs",
]
```

Edit `justfile`:

Replace:
```make
core_packages := "-p quarry-engine -p quarry-challenges -p quarry-store"
```
with:
```make
core_packages := "-p quarry-engine -p quarry-challenges -p quarry-store -p quarry"
```


- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry --lib`
Expected: 13 unit tests pass.

Run: `just check`
Expected: exit 0.

Run: `cargo mutants -p quarry`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Step 3: Commit**

```bash
git add .cargo/mutants.toml crates/app/Cargo.toml crates/app/src/lib.rs crates/app/src/model/drain.rs crates/app/src/model/mod.rs crates/app/src/model/run.rs crates/app/src/model/target.rs justfile
git commit -m "feat(app): add the run target, run state and channel-draining models"
```


---

### Task 3: Display models: formatting, status summary and schema-tree rows

How values, counts, durations and positions read on screen; what the status bar says about a run; and which rows the schema tree shows. Each file carries its tests in a `#[cfg(test)]` module: write that module first, run `cargo nextest run -p quarry --lib` and see it fail to compile, then add the code above it.

**Files:**
- Modify: `crates/app/src/model/mod.rs`
- Create: `crates/app/src/model/format.rs`
- Create: `crates/app/src/model/status.rs`
- Create: `crates/app/src/model/schema_tree.rs`

**Interfaces:**
- Consumes: `RunModel`, `Phase` (Task 2); `quarry_engine::schema::{Schema, Table, ColumnInfo, TableKind}`; `quarry_engine::value::Value`.
- Produces: `model::format::{cell_text, aligns_right, count, rows, duration, position, CELL_LIMIT}`; `model::status::{run_summary(&RunModel, Duration) -> String, failure(&RunModel) -> Option<(String, bool)>}`; `model::schema_tree::{rows(&Schema, &BTreeSet<String>) -> Vec<TreeRow>, toggle, TreeRow}`.

- [ ] **Step 1: Implement**

`crates/app/src/model/mod.rs`:
```rust
//! View models: plain data and pure functions the views render.
//!
//! Nothing here depends on GPUI, so every rule is unit-tested and
//! mutation-tested like the core crates.

pub mod drain;
pub mod format;
pub mod run;
pub mod schema_tree;
pub mod status;
pub mod target;
```

`crates/app/src/model/format.rs`:
```rust
//! How values, counts, durations and positions read on screen.

use std::time::Duration;

use quarry_engine::value::Value;

/// The longest text a grid cell shows before it is cut with `…`.
pub const CELL_LIMIT: usize = 200;

/// A cell's text: one line, at most [`CELL_LIMIT`] characters.
pub fn cell_text(value: &Value) -> String {
    let text = value.to_string().replace(['\n', '\r'], " ");
    match text.char_indices().nth(CELL_LIMIT) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    }
}

/// Numbers line up on the right, by digit; everything else on the left.
pub fn aligns_right(value: &Value) -> bool {
    matches!(value, Value::Int(_) | Value::Float(_))
}

/// `1234567` reads as `1,234,567`.
pub fn count(n: u64) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// `"1 row"`, `"1,000 rows"`.
pub fn rows(n: u64) -> String {
    if n == 1 {
        "1 row".to_owned()
    } else {
        format!("{} rows", count(n))
    }
}

/// Milliseconds with one decimal under a second, seconds with two under a
/// minute, then minutes and seconds.
pub fn duration(elapsed: Duration) -> String {
    let millis = elapsed.as_secs_f64() * 1000.0;
    if millis < 1000.0 {
        format!("{millis:.1} ms")
    } else if elapsed.as_secs() < 60 {
        format!("{:.2} s", elapsed.as_secs_f64())
    } else {
        format!(
            "{} m {:02} s",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60
        )
    }
}

/// A 0-based line and column, shown 1-based: `"Ln 6, Col 26"`.
pub fn position(line: u32, column: u32) -> String {
    format!(
        "Ln {}, Col {}",
        line.saturating_add(1),
        column.saturating_add(1)
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use quarry_engine::value::Value;

    use super::{CELL_LIMIT, aligns_right, cell_text, count, duration, position, rows};

    #[test]
    fn cells_are_one_line() {
        let value = Value::Text("two\nlines\r\n".to_owned());
        assert_eq!(cell_text(&value), "two lines  ");
    }

    #[test]
    fn long_cells_are_cut_on_a_character_boundary() {
        let exact = "é".repeat(CELL_LIMIT);
        assert_eq!(cell_text(&Value::Text(exact.clone())), exact);
        let long = "é".repeat(CELL_LIMIT + 1);
        assert_eq!(cell_text(&Value::Text(long)), format!("{exact}…"));
    }

    #[test]
    fn values_keep_their_engine_display() {
        assert_eq!(cell_text(&Value::Null), "NULL");
        assert_eq!(cell_text(&Value::Float(1.0)), "1.0");
    }

    #[test]
    fn only_numbers_align_right() {
        assert!(aligns_right(&Value::Int(1)));
        assert!(aligns_right(&Value::Float(1.5)));
        assert!(!aligns_right(&Value::Text("1".to_owned())));
        assert!(!aligns_right(&Value::Null));
        assert!(!aligns_right(&Value::Bytes(vec![1])));
    }

    #[test]
    fn counts_group_thousands() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1000), "1,000");
        assert_eq!(count(123_456), "123,456");
        assert_eq!(count(1_234_567), "1,234,567");
    }

    #[test]
    fn row_counts_agree_in_number() {
        assert_eq!(rows(0), "0 rows");
        assert_eq!(rows(1), "1 row");
        assert_eq!(rows(2000), "2,000 rows");
    }

    #[test]
    fn durations_pick_a_readable_unit() {
        assert_eq!(duration(Duration::from_micros(2140)), "2.1 ms");
        assert_eq!(duration(Duration::from_micros(999_940)), "999.9 ms");
        assert_eq!(duration(Duration::from_millis(1000)), "1.00 s");
        assert_eq!(duration(Duration::from_millis(1240)), "1.24 s");
        assert_eq!(duration(Duration::from_secs(59)), "59.00 s");
        assert_eq!(duration(Duration::from_secs(60)), "1 m 00 s");
        assert_eq!(duration(Duration::from_secs(65)), "1 m 05 s");
    }

    #[test]
    fn positions_are_one_based() {
        assert_eq!(position(5, 25), "Ln 6, Col 26");
        assert_eq!(position(u32::MAX, 0), format!("Ln {}, Col 1", u32::MAX));
    }
}
```

`crates/app/src/model/status.rs`:
```rust
//! The run summary the status bar shows.

use std::time::Duration;

use quarry_engine::{error::EngineError, result::Summary};

use super::{
    format,
    run::{Phase, RunModel},
};

/// What the status bar says about the latest run, given the live elapsed
/// time for a run still going.
pub fn run_summary(model: &RunModel, elapsed: Duration) -> String {
    match model.phase() {
        Phase::Idle => "Ready".to_owned(),
        Phase::Running => running(model, elapsed),
        Phase::Finished(summary) => finished(summary),
        Phase::Failed(EngineError::Cancelled) => "Cancelled".to_owned(),
        Phase::Failed(_) => "Error".to_owned(),
    }
}

/// The message above the grid when a run failed: the database's own words,
/// and whether it was only a cancel (shown quietly, not as an error).
pub fn failure(model: &RunModel) -> Option<(String, bool)> {
    match model.phase() {
        Phase::Failed(error) => Some((error.to_string(), *error == EngineError::Cancelled)),
        _ => None,
    }
}

fn running(model: &RunModel, elapsed: Duration) -> String {
    let received = u64::try_from(model.rows().len()).unwrap_or(u64::MAX);
    format!(
        "Running · {} · {}",
        format::duration(elapsed),
        format::rows(received)
    )
}

fn finished(summary: &Summary) -> String {
    let took = format::duration(summary.elapsed);
    match summary.rows_affected {
        Some(0) => format!("Done · {took}"),
        Some(1) => format!("1 row affected · {took}"),
        Some(n) => format!("{} rows affected · {took}", format::count(n)),
        None => format!("{} · {took}", format::rows(summary.rows)),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use quarry_engine::{error::EngineError, result::Summary, session::RunEvent, value::Value};

    use super::{failure, run_summary};
    use crate::model::run::RunModel;

    const MS: Duration = Duration::from_millis(3);

    fn finished(rows: u64, rows_affected: Option<u64>) -> String {
        let mut model = RunModel::started(Instant::now());
        model.apply(RunEvent::Finished(Summary {
            rows,
            rows_affected,
            elapsed: MS,
        }));
        run_summary(&model, Duration::ZERO)
    }

    #[test]
    fn only_a_failed_run_has_a_failure_message() {
        let mut model = RunModel::started(Instant::now());
        assert_eq!(failure(&model), None);
        model.apply(RunEvent::Failed(EngineError::Sql(
            "no such table: t".to_owned(),
        )));
        assert_eq!(
            failure(&model),
            Some(("no such table: t".to_owned(), false))
        );
        model.apply(RunEvent::Failed(EngineError::Cancelled));
        assert_eq!(failure(&model), Some(("Query cancelled.".to_owned(), true)));
    }

    #[test]
    fn idle_reads_ready() {
        assert_eq!(run_summary(&RunModel::default(), MS), "Ready");
    }

    #[test]
    fn a_running_query_shows_the_live_timer_and_rows_so_far() {
        let mut model = RunModel::started(Instant::now());
        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]; 1000]));
        assert_eq!(
            run_summary(&model, Duration::from_millis(1500)),
            "Running · 1.50 s · 1,000 rows"
        );
    }

    #[test]
    fn a_query_reports_its_rows() {
        assert_eq!(finished(4, None), "4 rows · 3.0 ms");
        assert_eq!(finished(1, None), "1 row · 3.0 ms");
    }

    #[test]
    fn a_change_reports_rows_affected() {
        assert_eq!(finished(0, Some(1)), "1 row affected · 3.0 ms");
        assert_eq!(finished(0, Some(1200)), "1,200 rows affected · 3.0 ms");
    }

    #[test]
    fn a_statement_that_changes_nothing_just_reads_done() {
        assert_eq!(finished(0, Some(0)), "Done · 3.0 ms");
    }

    #[test]
    fn failures_say_whether_they_were_cancelled() {
        let mut model = RunModel::started(Instant::now());
        model.apply(RunEvent::Failed(EngineError::Cancelled));
        assert_eq!(run_summary(&model, MS), "Cancelled");
        model.apply(RunEvent::Failed(EngineError::Sql("boom".to_owned())));
        assert_eq!(run_summary(&model, MS), "Error");
    }
}
```

`crates/app/src/model/schema_tree.rs`:
```rust
//! The rows of the sidebar's schema tree.

use std::collections::BTreeSet;

use quarry_engine::schema::{ColumnInfo, Schema, Table, TableKind};

/// One visible row of the tree.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TreeRow {
    /// A table or view; `expanded` when its columns are listed below it.
    Table {
        /// Table name.
        name: String,
        /// What the row's trailing text says: the column count, or "view".
        detail: String,
        /// Whether the table's columns are shown.
        expanded: bool,
    },
    /// A column of the expanded table above it.
    Column {
        /// Column name.
        name: String,
        /// Declared type, with `· PK` for primary-key columns.
        detail: String,
    },
}

/// The rows to show, given which tables are expanded.
pub fn rows(schema: &Schema, expanded: &BTreeSet<String>) -> Vec<TreeRow> {
    let mut rows = Vec::new();
    for table in &schema.tables {
        let open = expanded.contains(&table.name);
        rows.push(table_row(table, open));
        if open {
            rows.extend(table.columns.iter().map(column_row));
        }
    }
    rows
}

/// Expands `name` when collapsed and collapses it when expanded.
pub fn toggle(expanded: &mut BTreeSet<String>, name: &str) {
    if !expanded.remove(name) {
        expanded.insert(name.to_owned());
    }
}

fn table_row(table: &Table, expanded: bool) -> TreeRow {
    let detail = match table.kind {
        TableKind::View => "view".to_owned(),
        TableKind::Table => table.columns.len().to_string(),
    };
    TreeRow::Table {
        name: table.name.clone(),
        detail,
        expanded,
    }
}

fn column_row(column: &ColumnInfo) -> TreeRow {
    let type_name = if column.type_name.is_empty() {
        "any"
    } else {
        column.type_name.as_str()
    };
    let detail = if column.primary_key {
        format!("{type_name} · PK")
    } else {
        type_name.to_owned()
    };
    TreeRow::Column {
        name: column.name.clone(),
        detail,
    }
}

#[cfg(test)]
mod tests {
    use std::collections::BTreeSet;

    use quarry_engine::schema::{ColumnInfo, Schema, Table, TableKind};

    use super::{TreeRow, rows, toggle};

    fn column(name: &str, type_name: &str, primary_key: bool) -> ColumnInfo {
        ColumnInfo {
            name: name.to_owned(),
            type_name: type_name.to_owned(),
            primary_key,
        }
    }

    fn schema() -> Schema {
        Schema {
            tables: vec![
                Table {
                    name: "missions".to_owned(),
                    kind: TableKind::Table,
                    columns: vec![column("id", "INTEGER", true), column("name", "TEXT", false)],
                },
                Table {
                    name: "roster".to_owned(),
                    kind: TableKind::View,
                    columns: vec![column("who", "", false)],
                },
            ],
        }
    }

    fn table(name: &str, detail: &str, expanded: bool) -> TreeRow {
        TreeRow::Table {
            name: name.to_owned(),
            detail: detail.to_owned(),
            expanded,
        }
    }

    fn col(name: &str, detail: &str) -> TreeRow {
        TreeRow::Column {
            name: name.to_owned(),
            detail: detail.to_owned(),
        }
    }

    #[test]
    fn collapsed_tables_show_their_column_count_or_view() {
        assert_eq!(
            rows(&schema(), &BTreeSet::new()),
            [
                table("missions", "2", false),
                table("roster", "view", false)
            ]
        );
    }

    #[test]
    fn an_expanded_table_lists_its_columns_with_types() {
        let expanded = BTreeSet::from(["missions".to_owned(), "roster".to_owned()]);
        assert_eq!(
            rows(&schema(), &expanded),
            [
                table("missions", "2", true),
                col("id", "INTEGER · PK"),
                col("name", "TEXT"),
                table("roster", "view", true),
                col("who", "any"),
            ]
        );
    }

    #[test]
    fn toggling_twice_collapses_again() {
        let mut expanded = BTreeSet::new();
        toggle(&mut expanded, "missions");
        assert!(expanded.contains("missions"));
        toggle(&mut expanded, "missions");
        assert!(expanded.is_empty());
    }
}
```


- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry --lib`
Expected: 31 unit tests pass.

Run: `just check`
Expected: exit 0.

Run: `cargo mutants -p quarry`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/model/format.rs crates/app/src/model/mod.rs crates/app/src/model/schema_tree.rs crates/app/src/model/status.rs
git commit -m "feat(app): add value formatting, the run summary and schema-tree rows"
```


---

### Task 4: Keyboard focus in ui-kit

Interactive sidebar rows, tabs and segments become keyboard stops: Tab reaches them, Enter and Space click them (GPUI already turns those keys into a click on the focused element, on key-up), and a `FocusRing` border shows while they're focused. Each component keeps a transparent 1 px border so gaining focus never moves it. `Preference::next` gives View › Theme its cycle.

**Files:**
- Modify: `crates/ui-kit/src/appearance.rs`
- Modify: `crates/ui-kit/src/palette.rs`
- Modify: `crates/ui-kit/src/component_theme.rs`
- Modify: `crates/ui-kit/src/components/mod.rs`
- Modify: `crates/ui-kit/src/components/sidebar_item.rs`
- Modify: `crates/ui-kit/src/components/tabs.rs`
- Modify: `crates/ui-kit/src/components/segmented.rs`
- Modify: `crates/ui-kit/tests/contrast.rs`
- Create: `crates/ui-kit/tests/keyboard.rs`

**Interfaces:**
- Produces: `Role::FocusRing` (Aurora `0xcbb8ffff`, Dawn `0x593fbcff`, also the `gpui-component` `ring` color); `components::keyboard_focusable(Stateful<Div>, &Colors) -> Stateful<Div>`; `Preference::next(self) -> Preference` (Dark → Light → System → Dark).

- [ ] **Step 1: Write the failing tests**

Apply this change to `crates/ui-kit/tests/contrast.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/tests/contrast.rs b/crates/ui-kit/tests/contrast.rs
index f4eaac6..6fc8e12 100644
--- a/crates/ui-kit/tests/contrast.rs
+++ b/crates/ui-kit/tests/contrast.rs
@@ -133,3 +133,13 @@ fn aurora_badges_are_readable_on_every_surface() {
 fn dawn_badges_are_readable_on_every_surface() {
     assert_eq!(badge_failures(&Palette::dawn()), Vec::<String>::new());
 }
+
+#[test]
+fn the_focus_ring_stands_out_on_every_surface() {
+    for palette in [Palette::aurora(), Palette::dawn()] {
+        assert_eq!(
+            failures(&palette, &[Role::FocusRing], 3.0),
+            Vec::<String>::new()
+        );
+    }
+}
```

`crates/ui-kit/tests/keyboard.rs`:
```rust
//! Keyboard stops: Tab reaches interactive components, Enter and Space click
//! them, and components without a handler are skipped.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, test::TestEventExt,
};
use quarry_ui_kit::{
    Aurora, Preference,
    components::{sidebar_item::SidebarItem, tabs::Tab},
};

#[derive(Default)]
struct Presses {
    row: Cell<usize>,
    tab: Cell<usize>,
}

struct Stops {
    presses: Rc<Presses>,
}

impl Render for Stops {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (row, tab) = (Rc::clone(&self.presses), Rc::clone(&self.presses));
        div()
            .size_full()
            .child(SidebarItem::new("plain", "No handler"))
            .child(
                SidebarItem::new("row", "Row")
                    .on_click(move |_, _, _| row.row.set(row.row.get() + 1)),
            )
            .child(Tab::new("tab", "Tab").on_click(move |_, _, _| tab.tab.set(tab.tab.get() + 1)))
    }
}

fn open(cx: &mut TestAppContext) -> (&mut VisualTestContext, Rc<Presses>) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let presses = Rc::new(Presses::default());
    let view = Rc::clone(&presses);
    let (_, cx) = cx.add_window_view(move |_, _| Stops { presses: view });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (cx, presses)
}

/// Moves keyboard focus `stops` times, redrawing so the focused element
/// registers its key listeners.
fn tab(cx: &mut VisualTestContext, stops: usize) {
    cx.update(|window, cx| {
        for _ in 0..stops {
            window.focus_next(cx);
        }
        window.draw(cx).clear(cx);
    });
}

/// Presses and releases `key`; GPUI clicks the focused element on release.
fn press(cx: &mut VisualTestContext, key: &str) {
    cx.update(|window, cx| {
        window.key_down(key, false, cx);
        window.key_up(key, cx);
    });
}

#[gpui_kit::test]
fn the_first_stop_skips_a_row_without_a_handler(cx: &mut TestAppContext) {
    let (cx, presses) = open(cx);
    tab(cx, 1);
    press(cx, "enter");
    assert_eq!((presses.row.get(), presses.tab.get()), (1, 0));
}

#[gpui_kit::test]
fn space_clicks_and_the_next_stop_is_the_tab(cx: &mut TestAppContext) {
    let (cx, presses) = open(cx);
    tab(cx, 2);
    press(cx, "space");
    assert_eq!((presses.row.get(), presses.tab.get()), (0, 1));
}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry-ui-kit`
Expected: `no variant named FocusRing`, and the keyboard tests' counters stay at 0.

- [ ] **Step 3: Implement**

Apply this change to `crates/ui-kit/src/appearance.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/src/appearance.rs b/crates/ui-kit/src/appearance.rs
index 836f13d..1c4b6b7 100644
--- a/crates/ui-kit/src/appearance.rs
+++ b/crates/ui-kit/src/appearance.rs
@@ -30,6 +30,16 @@ impl Preference {
             Self::System => system,
         }
     }
+
+    /// The next setting in the View › Theme cycle: Dark, Light, System.
+    #[must_use]
+    pub fn next(self) -> Self {
+        match self {
+            Self::Dark => Self::Light,
+            Self::Light => Self::System,
+            Self::System => Self::Dark,
+        }
+    }
 }
 
 impl From<gpui_kit::WindowAppearance> for Appearance {
@@ -51,6 +61,13 @@ mod tests {
 
     use super::{Appearance, Preference};
 
+    #[test]
+    fn the_theme_cycle_visits_every_setting() {
+        assert_eq!(Preference::Dark.next(), Preference::Light);
+        assert_eq!(Preference::Light.next(), Preference::System);
+        assert_eq!(Preference::System.next(), Preference::Dark);
+    }
+
     #[test]
     fn fixed_preferences_ignore_the_system() {
         for system in [Appearance::Dark, Appearance::Light] {
```

Apply this change to `crates/ui-kit/src/palette.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/src/palette.rs b/crates/ui-kit/src/palette.rs
index f95852c..2cbe9fa 100644
--- a/crates/ui-kit/src/palette.rs
+++ b/crates/ui-kit/src/palette.rs
@@ -59,11 +59,13 @@ pub enum Role {
     LineNumber,
     /// Editor current-line wash.
     ActiveLine,
+    /// The keyboard focus ring (WCAG non-text contrast, 3:1).
+    FocusRing,
 }
 
 impl Role {
     /// Every role, in declaration order.
-    pub const ALL: [Self; 26] = [
+    pub const ALL: [Self; 27] = [
         Self::BackgroundStart,
         Self::BackgroundMiddle,
         Self::BackgroundEnd,
@@ -90,6 +92,7 @@ impl Role {
         Self::SyntaxComment,
         Self::LineNumber,
         Self::ActiveLine,
+        Self::FocusRing,
     ];
 
     fn index(self) -> usize {
@@ -97,7 +100,7 @@ impl Role {
     }
 }
 
-const AURORA: [(Role, u32); 26] = [
+const AURORA: [(Role, u32); 27] = [
     (Role::BackgroundStart, 0x1b15_30ff),
     (Role::BackgroundMiddle, 0x0e14_24ff),
     (Role::BackgroundEnd, 0x0b1b_22ff),
@@ -124,9 +127,10 @@ const AURORA: [(Role, u32); 26] = [
     (Role::SyntaxComment, 0xe6e4_f080),
     (Role::LineNumber, 0xe6e4_f071),
     (Role::ActiveLine, 0xffff_ff0a),
+    (Role::FocusRing, 0xcbb8_ffff),
 ];
 
-const DAWN: [(Role, u32); 26] = [
+const DAWN: [(Role, u32); 27] = [
     (Role::BackgroundStart, 0xf3ee_ffff),
     (Role::BackgroundMiddle, 0xeef3_ffff),
     (Role::BackgroundEnd, 0xe9f8_f6ff),
@@ -153,6 +157,7 @@ const DAWN: [(Role, u32); 26] = [
     (Role::SyntaxComment, 0x6b65_85ff),
     (Role::LineNumber, 0x2420_3a82),
     (Role::ActiveLine, 0x8b6c_ff0f),
+    (Role::FocusRing, 0x593f_bcff),
 ];
 
 /// A theme's colors as `0xRRGGBBAA` values, one per [`Role`].
```

Apply this change to `crates/ui-kit/src/component_theme.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/src/component_theme.rs b/crates/ui-kit/src/component_theme.rs
index 1f46150..5e5c092 100644
--- a/crates/ui-kit/src/component_theme.rs
+++ b/crates/ui-kit/src/component_theme.rs
@@ -58,7 +58,7 @@ fn colors(p: &Palette) -> Value {
         "accent.foreground": c(Role::Text),
         "primary.background": c(Role::AccentStart),
         "primary.foreground": c(Role::OnAccent),
-        "ring": c(Role::AccentStart),
+        "ring": c(Role::FocusRing),
         "caret": c(Role::AccentText),
         "selection.background": c(Role::Selection),
         "list.active.background": c(Role::Selection),
```

Apply this change to `crates/ui-kit/src/components/mod.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/src/components/mod.rs b/crates/ui-kit/src/components/mod.rs
index 8d1969e..17ae2bc 100644
--- a/crates/ui-kit/src/components/mod.rs
+++ b/crates/ui-kit/src/components/mod.rs
@@ -11,7 +11,22 @@ pub mod tabs;
 
 use std::rc::Rc;
 
-use gpui_kit::{App, ClickEvent, Window};
+use gpui_kit::{App, ClickEvent, Div, InteractiveElement, Stateful, Styled, Window};
+
+use crate::palette::{Colors, Role};
 
 /// A click callback shared by components.
 pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
+
+/// Makes an interactive element a keyboard stop. Tab reaches it, Enter and
+/// Space click it (GPUI turns them into a click), and its border turns
+/// [`Role::FocusRing`] while it has keyboard focus.
+///
+/// The element must already have a 1 px border (transparent when it has no
+/// visible one), so gaining focus never changes its size.
+pub fn keyboard_focusable(element: Stateful<Div>, colors: &Colors) -> Stateful<Div> {
+    let ring = colors.get(Role::FocusRing);
+    element
+        .tab_index(0)
+        .focus_visible(move |style| style.border_color(ring))
+}
```

Apply this change to `crates/ui-kit/src/components/sidebar_item.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/src/components/sidebar_item.rs b/crates/ui-kit/src/components/sidebar_item.rs
index 249c729..940f0b9 100644
--- a/crates/ui-kit/src/components/sidebar_item.rs
+++ b/crates/ui-kit/src/components/sidebar_item.rs
@@ -1,6 +1,6 @@
 //! A sidebar row: label, optional trailing text, selectable.
 //!
-//! Keyboard focus and navigation are added with the workspace in Phase 1C.
+//! A row with a click handler is a keyboard stop (see [`keyboard_focusable`]).
 
 use std::rc::Rc;
 
@@ -9,7 +9,7 @@ use gpui_kit::{
     SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
 };
 
-use super::ClickHandler;
+use super::{ClickHandler, keyboard_focusable};
 use crate::{
     Aurora,
     palette::{Colors, Role},
@@ -103,6 +103,8 @@ impl RenderOnce for SidebarItem {
             .px(px(space::SM))
             .py(px(5.))
             .rounded(px(radius::SM))
+            .border_1()
+            .border_color(gpui_kit::transparent_black())
             .text_size(px(text::SMALL))
             .text_color(colors.get(Role::Text))
             .cursor_pointer()
@@ -115,13 +117,14 @@ impl RenderOnce for SidebarItem {
             row = row.bg(background);
         }
         if let Some(border) = style.border {
-            row = row.border_1().border_color(border);
+            row = row.border_color(border);
         }
         if let Some(hover) = style.hover {
             row = row.hover(move |s| s.bg(hover));
         }
         if let Some(handler) = self.on_click {
-            row = row.on_click(move |event, window, cx| handler(event, window, cx));
+            row = keyboard_focusable(row, &colors)
+                .on_click(move |event, window, cx| handler(event, window, cx));
         }
         row.test_support()
     }
```

Apply this change to `crates/ui-kit/src/components/tabs.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/src/components/tabs.rs b/crates/ui-kit/src/components/tabs.rs
index fa6dd43..ab7eb36 100644
--- a/crates/ui-kit/src/components/tabs.rs
+++ b/crates/ui-kit/src/components/tabs.rs
@@ -1,6 +1,6 @@
 //! Editor tabs.
 //!
-//! Keyboard focus and navigation are added with the workspace in Phase 1C.
+//! A tab with a click handler is a keyboard stop (see [`keyboard_focusable`]).
 
 use std::rc::Rc;
 
@@ -9,7 +9,7 @@ use gpui_kit::{
     RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
 };
 
-use super::ClickHandler;
+use super::{ClickHandler, keyboard_focusable};
 use crate::{
     Aurora,
     palette::{Colors, Role},
@@ -91,6 +91,8 @@ impl RenderOnce for Tab {
             .px(px(space::MD + 2.))
             .py(px(space::SM - 1.))
             .rounded_t(px(radius::SM + 1.))
+            .border_1()
+            .border_color(gpui_kit::transparent_black())
             .text_size(px(text::SMALL))
             .bg(style.background)
             .text_color(style.text)
@@ -100,7 +102,8 @@ impl RenderOnce for Tab {
             tab = tab.hover(move |s| s.bg(hover));
         }
         if let Some(handler) = self.on_click {
-            tab = tab.on_click(move |event, window, cx| handler(event, window, cx));
+            tab = keyboard_focusable(tab, &colors)
+                .on_click(move |event, window, cx| handler(event, window, cx));
         }
         tab.test_support()
     }
```

Apply this change to `crates/ui-kit/src/components/segmented.rs` (`git apply`, or edit by hand):
```diff
diff --git a/crates/ui-kit/src/components/segmented.rs b/crates/ui-kit/src/components/segmented.rs
index fd35444..9ad9ab5 100644
--- a/crates/ui-kit/src/components/segmented.rs
+++ b/crates/ui-kit/src/components/segmented.rs
@@ -1,6 +1,6 @@
 //! A segmented control: one of a few options (e.g. Playground / Connections).
 //!
-//! Keyboard focus and navigation are added with the workspace in Phase 1C.
+//! With a change handler, each segment is a keyboard stop (see [`keyboard_focusable`]).
 
 use std::rc::Rc;
 
@@ -9,6 +9,7 @@ use gpui_kit::{
     StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
 };
 
+use super::keyboard_focusable;
 use crate::{
     Aurora,
     palette::{Colors, Role},
@@ -119,6 +120,8 @@ fn segment(
         .px(px(10.))
         .py(px(4.))
         .rounded(px(radius::SM))
+        .border_1()
+        .border_color(gpui_kit::transparent_black())
         .text_size(px(text::CAPTION))
         .bg(style.background)
         .text_color(style.text)
@@ -127,13 +130,11 @@ fn segment(
     if let Some(hover) = style.hover {
         segment = segment.hover(move |s| s.bg(hover));
     }
-    segment
-        .on_click(move |_, window, cx| {
-            if let Some(handler) = &handler {
-                handler(index, window, cx);
-            }
-        })
-        .test_support()
+    if let Some(handler) = handler {
+        segment = keyboard_focusable(segment, colors)
+            .on_click(move |_, window, cx| handler(index, window, cx));
+    }
+    segment.test_support()
 }
 
 #[cfg(test)]
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry-ui-kit`
Expected: 57 tests pass, including `the_selected_tab_sits_left_of_the_next` (the transparent border keeps tabs aligned).

Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/ui-kit/src/appearance.rs crates/ui-kit/src/component_theme.rs crates/ui-kit/src/components/mod.rs crates/ui-kit/src/components/segmented.rs crates/ui-kit/src/components/sidebar_item.rs crates/ui-kit/src/components/tabs.rs crates/ui-kit/src/palette.rs crates/ui-kit/tests/contrast.rs crates/ui-kit/tests/keyboard.rs
git commit -m "feat(ui-kit): make interactive rows, tabs and segments keyboard stops with a focus ring"
```


---

### Task 5: The workspace: editor, results grid, schema sidebar, Run and Cancel

The window's content. `Workspace` owns the `Session`; `runner.rs` turns ⌘↵ into a run and polls the session every `POLL` (33 ms) with `drain`, never awaiting it (ADR 0009); `layout.rs` draws the sidebar, the editor over a draggable divider, the results grid with a failure banner, and the status bar. The schema re-reads after every run.

**Files:**
- Modify: `crates/app/Cargo.toml`
- Modify: `crates/app/src/lib.rs`
- Create: `crates/app/src/shell/mod.rs`
- Create: `crates/app/src/shell/actions.rs`
- Create: `crates/app/src/views/mod.rs`
- Create: `crates/app/src/views/editor.rs`
- Create: `crates/app/src/views/results.rs`
- Create: `crates/app/src/views/schema.rs`
- Create: `crates/app/src/views/workspace/mod.rs`
- Create: `crates/app/src/views/workspace/runner.rs`
- Create: `crates/app/src/views/workspace/layout.rs`
- Create: `crates/app/tests/workspace.rs`

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces: `shell::actions::{RunStatement, CancelRun, ToggleSidebar, CycleTheme, Quit, WORKSPACE, WORKSPACE_EDITOR, key_bindings, menus, install}`; `views::workspace::{Workspace, POLL, SIDEBAR_WIDTH}` with `Workspace::{new(Session, &str, &mut Window, &mut Context<Self>), editor, schema, run_model, sidebar_open}`; `views::schema::{SchemaTree, table_row_id}`; `views::results::ResultsDelegate`; `views::editor::{new_editor, WELCOME}`.
- **Binding note:** the editor binds ⌘↵ and ⌘. itself; Quarry's bindings in `Workspace > Input` are deeper and win.
- **Test note:** the session thread delivers events in real time, so tests advance the clock by `POLL` and pump the executor (`settle`) instead of awaiting.

- [ ] **Step 1: Write the failing tests**

`crates/app/tests/workspace.rs`:
```rust
//! The workspace in a headless window: running, cancelling, the schema tree
//! and the window commands.

use std::time::Duration;

use gpui_kit::{App, Entity, TestAppContext, VisualTestContext, Window, test::TestWindowExt};
use quarry::{
    model::run::Phase,
    sample,
    shell::actions,
    views::{
        schema::table_row_id,
        workspace::{POLL, Workspace},
    },
};
use quarry_engine::error::EngineError;
use quarry_ui_kit::{Aurora, Preference};

/// How long a test waits for the session thread before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

fn open(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let (workspace, cx) = cx.add_window_view(|window, cx| {
        let session = sample::open()
            .unwrap_or_else(|error| unreachable!("the sample database opens: {error}"));
        Workspace::new(session, sample::NAME, window, cx)
    });
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| state.focus(window, cx));
    });
    settle(cx, |_, _| true);
    (workspace, cx)
}

/// Replaces the editor's text and puts the cursor at the end.
fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &str) {
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| {
            state.set_value(sql.to_owned(), window, cx);
            state.set_selected_range(sql.len()..sql.len(), cx);
        });
    });
}

/// Pumps the executor until `done` holds or [`PATIENCE`] runs out; the
/// session thread delivers events in real time.
fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
    let deadline = std::time::Instant::now() + PATIENCE;
    while std::time::Instant::now() < deadline {
        cx.executor().advance_clock(POLL);
        cx.run_until_parked();
        if cx.update(|window, cx| done(window, cx)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Runs the editor's statement with ⌘↵ and waits for the run to end.
fn run(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| {
        let model = workspace.read(cx).run_model(cx);
        !model.is_running() && model.phase() != &Phase::Idle
    });
}

fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
    cx.update(|_, cx| workspace.read(cx).run_model(cx).phase().clone())
}

#[gpui_kit::test]
fn the_editor_never_has_a_completion_provider(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.update(|_, cx| {
        let editor = workspace.read(cx).editor().read(cx);
        assert!(editor.lsp().completion_provider.is_none());
    });
}

#[gpui_kit::test]
fn command_return_in_the_editor_runs_the_statement(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT name FROM astronauts ORDER BY id");
    run(&workspace, cx);
    assert!(matches!(phase(&workspace, cx), Phase::Finished(_)));
    cx.update(|_, cx| {
        let model = workspace.read(cx).run_model(cx);
        assert_eq!(model.rows().len(), 12);
        assert_eq!(model.columns()[0].name, "name");
    });
}

#[gpui_kit::test]
fn command_return_leaves_the_editor_text_alone(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT 1");
    run(&workspace, cx);
    cx.update(|_, cx| {
        let text = workspace.read(cx).editor().read(cx).value();
        assert_eq!(text.as_ref(), "SELECT 1");
    });
}

#[gpui_kit::test]
fn a_bad_statement_shows_the_database_error(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT * FROM nowhere");
    run(&workspace, cx);
    assert_eq!(
        phase(&workspace, cx),
        Phase::Failed(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find("run-failure").is_some());
    });
}

#[gpui_kit::test]
fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(
        &workspace,
        cx,
        "SELECT count(*) FROM telemetry a, telemetry b",
    );
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| workspace.read(cx).run_model(cx).is_running());
    cx.simulate_keystrokes("cmd-.");
    settle(cx, |_, cx| !workspace.read(cx).run_model(cx).is_running());
    assert_eq!(phase(&workspace, cx), Phase::Failed(EngineError::Cancelled));
}

#[gpui_kit::test]
fn creating_a_table_adds_it_to_the_schema_tree(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "CREATE TABLE notes (body TEXT)");
    run(&workspace, cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("notes")).is_some()
    });
    cx.update(|window, _| assert!(window.try_find(table_row_id("notes")).is_some()));
}

#[gpui_kit::test]
fn clicking_a_table_expands_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("missions")).is_some()
    });
    cx.update(|window, cx| window.click(table_row_id("missions"), cx));
    cx.update(|_, cx| {
        let tree = workspace.read(cx).schema().read(cx);
        assert!(tree.expanded().contains("missions"));
    });
}

#[gpui_kit::test]
fn command_backslash_toggles_the_sidebar(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(!workspace.read(cx).sidebar_open()));
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(workspace.read(cx).sidebar_open()));
}

#[gpui_kit::test]
fn command_shift_t_cycles_the_theme(cx: &mut TestAppContext) {
    let (_, cx) = open(cx);
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::Light));
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::System));
}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry --test workspace`
Expected: `unresolved import quarry::views` / `quarry::shell`.

- [ ] **Step 3: Implement**

`crates/app/Cargo.toml`:
```toml
[package]
name = "quarry"
version.workspace = true
edition.workspace = true
rust-version.workspace = true
publish.workspace = true
license.workspace = true

[lib]
path = "src/lib.rs"

[[bin]]
name = "quarry"
path = "src/main.rs"

[dependencies]
async-channel.workspace = true
gpui-kit = { workspace = true, features = ["assets"] }
quarry-engine = { path = "../engine" }
quarry-ui-kit = { path = "../ui-kit" }

[dev-dependencies]
gpui-kit = { workspace = true, features = ["test-support", "assets"] }

[lints]
workspace = true
```

`crates/app/src/lib.rs`:
```rust
//! Quarry: a native macOS SQL client for learning and real work.
//!
//! [`model`] holds the GPUI-free view models; [`views`] renders them;
//! [`shell`] wires actions, key bindings, menus and the window.

pub mod model;
pub mod sample;
pub mod shell;
pub mod views;
```

`crates/app/src/shell/mod.rs`:
```rust
//! Application start-up: actions, key bindings and the menu bar.

pub mod actions;
```

`crates/app/src/shell/actions.rs`:
```rust
//! Every command, its key binding and its menu.
//!
//! Adding a command means declaring it in `actions!`, adding one row to
//! [`key_bindings`] and one menu item to [`menus`].

use gpui_kit::{App, KeyBinding, Menu, MenuItem, actions};

actions!(
    quarry,
    [
        /// Runs the selection, or the statement under the cursor.
        RunStatement,
        /// Cancels the running query.
        CancelRun,
        /// Shows or hides the sidebar.
        ToggleSidebar,
        /// Cycles the theme: Dark, Light, System.
        CycleTheme,
        /// Quits Quarry.
        Quit,
    ]
);

/// The key context of the workspace's root element.
pub const WORKSPACE: &str = "Workspace";

/// The editor's own key context, inside the workspace. Bindings here win
/// over the editor's built-in ⌘↵ (newline) and ⌘. (code actions).
pub const WORKSPACE_EDITOR: &str = "Workspace > Input";

/// Every key binding Quarry adds.
pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE)),
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-\\", ToggleSidebar, None),
        KeyBinding::new("cmd-shift-t", CycleTheme, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]
}

/// The macOS menu bar.
pub fn menus() -> Vec<Menu> {
    vec![
        Menu::new("Quarry").items([MenuItem::action("Quit Quarry", Quit)]),
        Menu::new("Query").items([
            MenuItem::action("Run Statement", RunStatement),
            MenuItem::action("Cancel", CancelRun),
        ]),
        Menu::new("View").items([
            MenuItem::action("Toggle Sidebar", ToggleSidebar),
            MenuItem::action("Cycle Theme", CycleTheme),
        ]),
    ]
}

/// Registers the bindings, the menu bar and the app-wide actions.
pub fn install(cx: &mut App) {
    cx.bind_keys(key_bindings());
    cx.set_menus(menus());
    cx.on_action(|_: &Quit, cx| cx.quit());
}
```

`crates/app/src/views/mod.rs`:
```rust
//! GPUI views: each renders one region of the workspace.

pub mod editor;
pub mod results;
pub mod schema;
pub mod workspace;
```

`crates/app/src/views/editor.rs`:
```rust
//! The SQL editor: highlighting and line numbers, and no completion of any
//! kind. Quarry never attaches a completion or other language provider.

use gpui_kit::{App, AppContext, Entity, Window, component::input::EditorState};

/// What a new workspace's editor starts with.
pub const WELCOME: &str = "-- Welcome to Quarry. ⌘↵ runs the statement under the cursor.\n\
SELECT name, nationality, call_sign\n\
FROM astronauts\n\
ORDER BY joined;\n";

/// A SQL editor holding `text`.
pub fn new_editor(text: &str, window: &mut Window, cx: &mut App) -> Entity<EditorState> {
    let text = text.to_owned();
    cx.new(|cx| {
        EditorState::new(window, cx)
            .language("sql")
            .line_number(true)
            .default_value(text)
    })
}
```

`crates/app/src/views/results.rs`:
```rust
//! The results grid: the latest run's rows in a virtualized table.

use gpui_kit::{
    AnyElement, App, Context, IntoElement, ParentElement, SharedString, Styled, Window,
    component::table::{Column, TableDelegate, TableState},
    div,
    prelude::FluentBuilder,
    px,
};
use quarry_engine::{session::RunEvent, value::Value};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{MONO_FONT, text},
};

use crate::model::{
    format,
    run::{Phase, RunModel},
};

/// Default width of a result column.
pub const COLUMN_WIDTH: f32 = 160.0;

/// Feeds the latest [`RunModel`] to the table.
#[derive(Default)]
pub struct ResultsDelegate {
    model: RunModel,
    columns: Vec<Column>,
}

impl ResultsDelegate {
    /// The run being shown.
    pub fn model(&self) -> &RunModel {
        &self.model
    }

    /// Shows `model` from scratch, e.g. when a new run starts.
    pub fn reset(&mut self, model: RunModel) {
        self.model = model;
        self.columns.clear();
    }

    /// Folds `event` in; returns whether the table's columns changed.
    pub fn apply(&mut self, event: RunEvent) -> bool {
        let reshapes = matches!(event, RunEvent::Started { .. } | RunEvent::Columns(_));
        self.model.apply(event);
        if reshapes {
            self.columns = self.model.columns().iter().map(table_column).collect();
        }
        reshapes
    }

    fn value(&self, row: usize, col: usize) -> Option<&Value> {
        self.model.rows().get(row)?.get(col)
    }
}

fn table_column(column: &quarry_engine::result::Column) -> Column {
    Column::new(column.name.clone(), column.name.clone()).width(px(COLUMN_WIDTH))
}

impl TableDelegate for ResultsDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.model.rows().len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns
            .get(col_ix)
            .cloned()
            .unwrap_or_else(|| Column::new("", ""))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let value = self.value(row_ix, col_ix).cloned().unwrap_or(Value::Null);
        cell(&value, cx)
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &App) -> String {
        self.value(row_ix, col_ix)
            .map(format::cell_text)
            .unwrap_or_default()
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        empty_state(self.model.phase(), cx)
    }
}

/// One cell: Menlo, numbers on the right, `NULL` dim and italic.
fn cell(value: &Value, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    div()
        .w_full()
        .font_family(MONO_FONT)
        .text_size(px(text::BODY))
        .when(format::aligns_right(value), Styled::text_right)
        .when(value.is_null(), |cell| {
            cell.italic().text_color(colors.get(Role::TextMuted))
        })
        .child(SharedString::from(format::cell_text(value)))
        .into_any_element()
}

/// What the grid says when there are no rows to show.
fn empty_state(phase: &Phase, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    let (message, role) = match phase {
        Phase::Idle => ("Run a statement with ⌘↵.".to_owned(), Role::TextMuted),
        Phase::Running => ("Running…".to_owned(), Role::TextMuted),
        Phase::Finished(_) | Phase::Failed(_) => ("No rows.".to_owned(), Role::TextMuted),
    };
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(24.))
        .text_size(px(text::BODY))
        .text_color(colors.get(role))
        .child(message)
        .into_any_element()
}
```

`crates/app/src/views/schema.rs`:
```rust
//! The sidebar's schema tree: tables and views, expandable to their columns.

use std::collections::BTreeSet;

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
};
use quarry_engine::{error::EngineError, schema::Schema};
use quarry_ui_kit::{
    Aurora, Role,
    components::sidebar_item::SidebarItem,
    tokens::{space, text},
};

use crate::model::schema_tree::{self, TreeRow};

/// The schema as last read, or why it could not be read.
pub struct SchemaTree {
    schema: Result<Schema, EngineError>,
    expanded: BTreeSet<String>,
}

impl Default for SchemaTree {
    fn default() -> Self {
        Self {
            schema: Ok(Schema::default()),
            expanded: BTreeSet::new(),
        }
    }
}

impl SchemaTree {
    /// Shows a freshly read schema, keeping which tables are expanded.
    pub fn set(&mut self, schema: Result<Schema, EngineError>, cx: &mut Context<Self>) {
        self.schema = schema;
        cx.notify();
    }

    /// Names of the tables whose columns are listed.
    pub fn expanded(&self) -> &BTreeSet<String> {
        &self.expanded
    }

    fn toggle(&mut self, name: &str, cx: &mut Context<Self>) {
        schema_tree::toggle(&mut self.expanded, name);
        cx.notify();
    }

    fn row(row: TreeRow, cx: &mut Context<Self>) -> AnyElement {
        match row {
            TreeRow::Table {
                name,
                detail,
                expanded,
            } => table_row(name, detail, expanded, cx),
            TreeRow::Column { name, detail } => column_row(name, detail, cx),
        }
    }
}

/// The element id of a table's row: `table:<name>`.
pub fn table_row_id(name: &str) -> SharedString {
    SharedString::from(format!("table:{name}"))
}

fn table_row(
    name: String,
    detail: String,
    expanded: bool,
    cx: &mut Context<SchemaTree>,
) -> AnyElement {
    let label = SharedString::from(name.clone());
    SidebarItem::new(table_row_id(&name), label)
        .trailing(detail)
        .selected(expanded)
        .on_click(cx.listener(move |tree, _, _, cx| tree.toggle(&name, cx)))
        .into_any_element()
}

fn column_row(name: String, detail: String, cx: &Context<SchemaTree>) -> AnyElement {
    let colors = Aurora::colors(cx);
    div()
        .flex()
        .justify_between()
        .pl(px(space::LG + space::SM))
        .pr(px(space::SM))
        .py(px(2.))
        .text_size(px(text::CAPTION))
        .text_color(colors.get(Role::TextSecondary))
        .child(name)
        .child(div().text_color(colors.get(Role::TextMuted)).child(detail))
        .into_any_element()
}

impl Render for SchemaTree {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let list = div().flex().flex_col().gap(px(1.));
        match &self.schema {
            Err(error) => list
                .text_size(px(text::SMALL))
                .text_color(colors.get(Role::Danger))
                .child(format!("Cannot read the schema: {error}")),
            Ok(schema) => {
                let rows = schema_tree::rows(schema, &self.expanded);
                let children: Vec<AnyElement> =
                    rows.into_iter().map(|row| Self::row(row, cx)).collect();
                list.children(children)
            }
        }
    }
}
```

`crates/app/src/views/workspace/mod.rs`:
```rust
//! The workspace: schema sidebar, editor over results, and the status bar.
//!
//! This module owns the workspace's state and its window commands;
//! [`runner`] drives runs against the [`Session`] and [`layout`] draws it.

mod layout;
mod runner;

use std::time::Duration;

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Task, Window,
    component::{input::EditorState, table::TableState},
};
use quarry_engine::session::{RunHandle, Session};
use quarry_ui_kit::Aurora;

use super::{editor, results::ResultsDelegate, schema::SchemaTree};
use crate::{
    model::run::RunModel,
    shell::actions::{CycleTheme, ToggleSidebar},
};

/// How often the workspace polls the session: about 30 times a second.
pub const POLL: Duration = Duration::from_millis(33);

/// Width of the open sidebar.
pub const SIDEBAR_WIDTH: f32 = 240.0;

/// The main window's content.
pub struct Workspace {
    session: Session,
    database: String,
    editor: Entity<EditorState>,
    results: Entity<TableState<ResultsDelegate>>,
    schema: Entity<SchemaTree>,
    sidebar_open: bool,
    focus: FocusHandle,
    current: Option<RunHandle>,
    run: Option<Task<()>>,
    schema_read: Option<Task<()>>,
}

impl Workspace {
    /// A workspace on `session`, labelled `database` in the status bar.
    pub fn new(
        session: Session,
        database: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = editor::new_editor(editor::WELCOME, window, cx);
        let results = cx.new(|cx| TableState::new(ResultsDelegate::default(), window, cx));
        cx.observe_window_appearance(window, |_, _, cx| Aurora::system_appearance_changed(cx))
            .detach();
        let mut workspace = Self {
            session,
            database: database.to_owned(),
            editor,
            results,
            schema: cx.new(|_| SchemaTree::default()),
            sidebar_open: true,
            focus: cx.focus_handle(),
            current: None,
            run: None,
            schema_read: None,
        };
        workspace.read_schema(cx);
        workspace
    }

    /// The editor, so callers can focus it or set its text.
    pub fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// The sidebar's schema tree.
    pub fn schema(&self) -> &Entity<SchemaTree> {
        &self.schema
    }

    /// The run the grid shows.
    pub fn run_model<'a>(&self, cx: &'a App) -> &'a RunModel {
        self.results.read(cx).delegate().model()
    }

    /// Whether the sidebar is shown.
    pub fn sidebar_open(&self) -> bool {
        self.sidebar_open
    }

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    fn cycle_theme(_: &mut Self, _: &CycleTheme, _: &mut Window, cx: &mut Context<Self>) {
        let next = Aurora::preference(cx).next();
        Aurora::set_preference(next, cx);
    }
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
```

`crates/app/src/views/workspace/runner.rs`:
```rust
//! Drives runs: reads the editor, submits the target, polls the session for
//! events and the schema, and folds them into the grid and the sidebar.
//!
//! The session's channels are polled once per [`POLL`] rather than awaited
//! (see [`crate::model::drain`]); each poll also redraws, which keeps the
//! live timer moving.

use std::time::Instant;

use async_channel::Receiver;
use gpui_kit::{App, Context, Task, Window};
use quarry_engine::{error::EngineError, schema::Schema, session::RunEvent};

use super::{POLL, Workspace};
use crate::{
    model::{
        drain::{Drained, drain},
        run::RunModel,
        target,
    },
    shell::actions::{CancelRun, RunStatement},
};

impl Workspace {
    pub(super) fn run_statement(
        &mut self,
        _: &RunStatement,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(sql) = self.target_sql(cx) else {
            return;
        };
        let handle = self.session.run(sql);
        self.results.update(cx, |table, cx| {
            table
                .delegate_mut()
                .reset(RunModel::started(Instant::now()));
            table.refresh(cx);
        });
        self.run = Some(Self::pump(handle.events.clone(), cx));
        self.current = Some(handle);
        cx.notify();
    }

    pub(super) fn cancel(&mut self, _: &CancelRun, _: &mut Window, _: &mut Context<Self>) {
        if let Some(handle) = &self.current {
            handle.cancel();
        }
    }

    /// Reads the schema after any queued runs and shows it in the sidebar.
    pub(super) fn read_schema(&mut self, cx: &mut Context<Self>) {
        let reply = self.session.schema();
        let tree = self.schema.clone();
        self.schema_read = Some(cx.spawn(async move |_, cx| {
            let mut answer = None;
            while answer.is_none() {
                cx.background_executor().timer(POLL).await;
                answer = schema_answer(drain(&reply));
            }
            if let Some(schema) = answer {
                tree.update(cx, |tree, cx| tree.set(schema, cx));
            }
        }));
    }

    /// The SQL a Run sends: the selection, or the statement under the cursor.
    fn target_sql(&self, cx: &App) -> Option<String> {
        let state = self.editor.read(cx);
        let script = state.value();
        let range = target::run_target(&script, state.selected_range())?;
        script.get(range).map(str::to_owned)
    }

    /// Polls the run's events once per [`POLL`] until the run ends.
    fn pump(events: Receiver<RunEvent>, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            let mut more = true;
            while more {
                cx.background_executor().timer(POLL).await;
                let drained = drain(&events);
                more = this
                    .update(cx, |w, cx| w.receive(drained, cx))
                    .unwrap_or(false);
            }
        })
    }

    /// Applies one poll's events; returns whether the run goes on.
    fn receive(&mut self, drained: Drained<RunEvent>, cx: &mut Context<Self>) -> bool {
        let ended = drained.closed || drained.items.contains(&RunEvent::Done);
        self.apply(drained.items, cx);
        if ended {
            self.run_ended(cx);
        }
        cx.notify();
        !ended
    }

    fn apply(&mut self, events: Vec<RunEvent>, cx: &mut Context<Self>) {
        self.results.update(cx, |table, cx| {
            let delegate = table.delegate_mut();
            let mut reshaped = false;
            for event in events {
                reshaped |= delegate.apply(event);
            }
            if reshaped {
                table.refresh(cx);
            }
            cx.notify();
        });
    }

    /// Settles a run that never said `Done`, releases its handle and
    /// re-reads the schema, which the run may have changed.
    fn run_ended(&mut self, cx: &mut Context<Self>) {
        if self.run_model(cx).is_running() {
            self.apply(vec![RunEvent::Done], cx);
        }
        self.current = None;
        self.read_schema(cx);
    }
}

/// The schema reply, once it has come: the reply itself, or `WorkerGone`
/// when the channel closed without one.
fn schema_answer(
    drained: Drained<Result<Schema, EngineError>>,
) -> Option<Result<Schema, EngineError>> {
    let Drained { mut items, closed } = drained;
    items
        .pop()
        .or_else(|| closed.then_some(Err(EngineError::WorkerGone)))
}
```

`crates/app/src/views/workspace/layout.rs`:
```rust
//! Draws the workspace: sidebar, editor over results, status bar.

use std::time::Instant;

use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        input::Editor,
        resizable::{resizable_panel, v_resizable},
        table::DataTable,
    },
    div,
    prelude::FluentBuilder,
    px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::{
        status_bar::StatusBar,
        surface::{glass_panel, sidebar_surface, window_background},
    },
    tokens::{chrome, space, text},
};

use super::{SIDEBAR_WIDTH, Workspace};
use crate::{
    model::{format, status},
    shell::actions::WORKSPACE,
};

/// The editor panel's starting height; the divider below it drags.
pub const EDITOR_HEIGHT: f32 = 220.0;

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if Aurora::is_transitioning(cx) {
            window.request_animation_frame();
        }
        let colors = Aurora::colors(cx);
        div()
            .id("workspace")
            .key_context(WORKSPACE)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::run_statement))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::cycle_theme))
            .size_full()
            .flex()
            .bg(window_background(&colors))
            .text_color(colors.get(Role::Text))
            .when(self.sidebar_open, |root| root.child(self.sidebar(cx)))
            .child(self.center(cx))
    }
}

impl Workspace {
    fn sidebar(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        sidebar_surface(&colors)
            .id("sidebar")
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .pt(px(chrome::TITLEBAR_HEIGHT))
            .px(px(space::SM))
            .flex()
            .flex_col()
            .gap(px(space::SM))
            .overflow_y_scroll()
            .child(
                div()
                    .px(px(space::SM))
                    .text_size(px(text::CAPTION))
                    .text_color(colors.get(Role::TextMuted))
                    .child("TABLES"),
            )
            .child(self.schema.clone())
            .into_any_element()
    }

    fn center(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let editor = glass_panel(&colors)
            .size_full()
            .p(px(space::SM))
            .child(Editor::new(&self.editor).appearance(false).h_full());
        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .pt(px(chrome::TITLEBAR_HEIGHT))
            .child(
                div().flex_1().px(px(space::MD)).pb(px(space::SM)).child(
                    v_resizable("editor-results")
                        .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
                        .child(resizable_panel().child(self.results_panel(cx))),
                ),
            )
            .child(self.status_bar(cx))
            .into_any_element()
    }

    fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let failure = status::failure(self.run_model(cx));
        glass_panel(&colors)
            .id("results")
            .size_full()
            .mt(px(space::SM))
            .flex()
            .flex_col()
            .overflow_hidden()
            .children(failure.map(|(message, cancelled)| failure_banner(message, cancelled, cx)))
            .child(
                div()
                    .flex_1()
                    .child(DataTable::new(&self.results).bordered(false)),
            )
            .into_any_element()
    }

    fn status_bar(&self, cx: &Context<Self>) -> AnyElement {
        let model = self.run_model(cx);
        let summary = status::run_summary(model, model.elapsed(Instant::now()));
        let cursor = self.editor.read(cx).cursor_position();
        let place = format::position(cursor.line, cursor.character);
        div()
            .id("status")
            .child(
                StatusBar::new()
                    .left(format!("● {}", self.database))
                    .right(format!("{summary} · {place}")),
            )
            .into_any_element()
    }
}

/// The run's error above the grid; a cancel reads quietly.
fn failure_banner(message: String, cancelled: bool, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    let role = if cancelled {
        Role::TextSecondary
    } else {
        Role::Danger
    };
    div()
        .id("run-failure")
        .px(px(space::MD))
        .py(px(space::SM))
        .border_b_1()
        .border_color(colors.get(Role::Border))
        .text_size(px(text::BODY))
        .text_color(colors.get(role))
        .child(message)
        .test_support()
        .into_any_element()
}
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry --test workspace`
Expected: 9 tests pass. Run the file 10 times (`for i in $(seq 10); do cargo nextest run -p quarry --test workspace || break; done`) to confirm none are flaky.

Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/Cargo.toml crates/app/src/lib.rs crates/app/src/shell/actions.rs crates/app/src/shell/mod.rs crates/app/src/views/editor.rs crates/app/src/views/mod.rs crates/app/src/views/results.rs crates/app/src/views/schema.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/runner.rs crates/app/tests/workspace.rs
git commit -m "feat(app): add the workspace with editor, streaming results, schema sidebar, Run and Cancel"
```


---

### Task 6: ⌘1 and ⌘2: focus the sidebar and the editor

The editor keeps Tab for indentation, so two commands move focus between regions. ⌘1 focuses the sidebar's first table (opening the sidebar first if it's closed, then focusing on the next frame), and ⌘2 returns to the editor.

**Files:**
- Modify: `crates/app/src/shell/actions.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/tests/workspace.rs`

**Interfaces:**
- Produces: `shell::actions::{FocusSidebar, FocusEditor}` bound to `cmd-1` and `cmd-2`, in the View menu.
- The sidebar container is a tab group tracked by a `FocusHandle` with `tab_index(0).tab_stop(false)`: focusing it and calling `window.focus_next` lands on its first table row.

- [ ] **Step 1: Write the failing tests**

`crates/app/tests/workspace.rs`:
```rust
//! The workspace in a headless window: running, cancelling, the schema tree
//! and the window commands.

use std::time::Duration;

use gpui_kit::{
    App, Entity, Focusable, TestAppContext, VisualTestContext, Window,
    test::{TestEventExt, TestWindowExt},
};
use quarry::{
    model::run::Phase,
    sample,
    shell::actions,
    views::{
        schema::table_row_id,
        workspace::{POLL, Workspace},
    },
};
use quarry_engine::error::EngineError;
use quarry_ui_kit::{Aurora, Preference};

/// How long a test waits for the session thread before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

fn open(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let (workspace, cx) = cx.add_window_view(|window, cx| {
        let session = sample::open()
            .unwrap_or_else(|error| unreachable!("the sample database opens: {error}"));
        Workspace::new(session, sample::NAME, window, cx)
    });
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| state.focus(window, cx));
    });
    settle(cx, |_, _| true);
    (workspace, cx)
}

/// Replaces the editor's text and puts the cursor at the end.
fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &str) {
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| {
            state.set_value(sql.to_owned(), window, cx);
            state.set_selected_range(sql.len()..sql.len(), cx);
        });
    });
}

/// Pumps the executor until `done` holds or [`PATIENCE`] runs out; the
/// session thread delivers events in real time.
fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
    let deadline = std::time::Instant::now() + PATIENCE;
    while std::time::Instant::now() < deadline {
        cx.executor().advance_clock(POLL);
        cx.run_until_parked();
        if cx.update(|window, cx| done(window, cx)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

/// Runs the editor's statement with ⌘↵ and waits for the run to end.
fn run(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| {
        let model = workspace.read(cx).run_model(cx);
        !model.is_running() && model.phase() != &Phase::Idle
    });
}

fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
    cx.update(|_, cx| workspace.read(cx).run_model(cx).phase().clone())
}

#[gpui_kit::test]
fn the_editor_never_has_a_completion_provider(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.update(|_, cx| {
        let editor = workspace.read(cx).editor().read(cx);
        assert!(editor.lsp().completion_provider.is_none());
    });
}

#[gpui_kit::test]
fn command_return_in_the_editor_runs_the_statement(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT name FROM astronauts ORDER BY id");
    run(&workspace, cx);
    assert!(matches!(phase(&workspace, cx), Phase::Finished(_)));
    cx.update(|_, cx| {
        let model = workspace.read(cx).run_model(cx);
        assert_eq!(model.rows().len(), 12);
        assert_eq!(model.columns()[0].name, "name");
    });
}

#[gpui_kit::test]
fn command_return_leaves_the_editor_text_alone(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT 1");
    run(&workspace, cx);
    cx.update(|_, cx| {
        let text = workspace.read(cx).editor().read(cx).value();
        assert_eq!(text.as_ref(), "SELECT 1");
    });
}

#[gpui_kit::test]
fn a_bad_statement_shows_the_database_error(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT * FROM nowhere");
    run(&workspace, cx);
    assert_eq!(
        phase(&workspace, cx),
        Phase::Failed(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find("run-failure").is_some());
    });
}

#[gpui_kit::test]
fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(
        &workspace,
        cx,
        "SELECT count(*) FROM telemetry a, telemetry b",
    );
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| workspace.read(cx).run_model(cx).is_running());
    cx.simulate_keystrokes("cmd-.");
    settle(cx, |_, cx| !workspace.read(cx).run_model(cx).is_running());
    assert_eq!(phase(&workspace, cx), Phase::Failed(EngineError::Cancelled));
}

#[gpui_kit::test]
fn creating_a_table_adds_it_to_the_schema_tree(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "CREATE TABLE notes (body TEXT)");
    run(&workspace, cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("notes")).is_some()
    });
    cx.update(|window, _| assert!(window.try_find(table_row_id("notes")).is_some()));
}

#[gpui_kit::test]
fn clicking_a_table_expands_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("missions")).is_some()
    });
    cx.update(|window, cx| window.click(table_row_id("missions"), cx));
    cx.update(|_, cx| {
        let tree = workspace.read(cx).schema().read(cx);
        assert!(tree.expanded().contains("missions"));
    });
}

#[gpui_kit::test]
fn command_backslash_toggles_the_sidebar(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(!workspace.read(cx).sidebar_open()));
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(workspace.read(cx).sidebar_open()));
}

#[gpui_kit::test]
fn command_shift_t_cycles_the_theme(cx: &mut TestAppContext) {
    let (_, cx) = open(cx);
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::Light));
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::System));
}

#[gpui_kit::test]
fn command_one_focuses_the_first_table_and_return_expands_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("astronauts")).is_some()
    });
    cx.simulate_keystrokes("cmd-1");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.key_down("enter", false, cx);
        window.key_up("enter", cx);
    });
    cx.update(|_, cx| {
        let tree = workspace.read(cx).schema().read(cx);
        assert!(tree.expanded().contains("astronauts"));
    });
}

#[gpui_kit::test]
fn command_two_returns_focus_to_the_editor(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.simulate_keystrokes("cmd-1");
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        true
    });
    cx.simulate_keystrokes("cmd-2");
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().read(cx);
        assert!(editor.focus_handle(cx).is_focused(window));
    });
}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry --test workspace`
Expected: `cannot find value FocusSidebar`.

- [ ] **Step 3: Implement**

`crates/app/src/shell/actions.rs`:
```rust
//! Every command, its key binding and its menu.
//!
//! Adding a command means declaring it in `actions!`, adding one row to
//! [`key_bindings`] and one menu item to [`menus`].

use gpui_kit::{App, KeyBinding, Menu, MenuItem, actions};

actions!(
    quarry,
    [
        /// Runs the selection, or the statement under the cursor.
        RunStatement,
        /// Cancels the running query.
        CancelRun,
        /// Shows or hides the sidebar.
        ToggleSidebar,
        /// Moves keyboard focus to the sidebar's first table.
        FocusSidebar,
        /// Moves keyboard focus to the editor.
        FocusEditor,
        /// Cycles the theme: Dark, Light, System.
        CycleTheme,
        /// Quits Quarry.
        Quit,
    ]
);

/// The key context of the workspace's root element.
pub const WORKSPACE: &str = "Workspace";

/// The editor's own key context, inside the workspace. Bindings here win
/// over the editor's built-in ⌘↵ (newline) and ⌘. (code actions).
pub const WORKSPACE_EDITOR: &str = "Workspace > Input";

/// Every key binding Quarry adds.
pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE)),
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-\\", ToggleSidebar, None),
        KeyBinding::new("cmd-1", FocusSidebar, None),
        KeyBinding::new("cmd-2", FocusEditor, None),
        KeyBinding::new("cmd-shift-t", CycleTheme, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]
}

/// The macOS menu bar.
pub fn menus() -> Vec<Menu> {
    vec![
        Menu::new("Quarry").items([MenuItem::action("Quit Quarry", Quit)]),
        Menu::new("Query").items([
            MenuItem::action("Run Statement", RunStatement),
            MenuItem::action("Cancel", CancelRun),
        ]),
        Menu::new("View").items([
            MenuItem::action("Toggle Sidebar", ToggleSidebar),
            MenuItem::action("Focus Sidebar", FocusSidebar),
            MenuItem::action("Focus Editor", FocusEditor),
            MenuItem::action("Cycle Theme", CycleTheme),
        ]),
    ]
}

/// Registers the bindings, the menu bar and the app-wide actions.
pub fn install(cx: &mut App) {
    cx.bind_keys(key_bindings());
    cx.set_menus(menus());
    cx.on_action(|_: &Quit, cx| cx.quit());
}
```

`crates/app/src/views/workspace/mod.rs`:
```rust
//! The workspace: schema sidebar, editor over results, and the status bar.
//!
//! This module owns the workspace's state and its window commands;
//! [`runner`] drives runs against the [`Session`] and [`layout`] draws it.

mod layout;
mod runner;

use std::time::Duration;

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Task, Window,
    component::{input::EditorState, table::TableState},
};
use quarry_engine::session::{RunHandle, Session};
use quarry_ui_kit::Aurora;

use super::{editor, results::ResultsDelegate, schema::SchemaTree};
use crate::{
    model::run::RunModel,
    shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
};

/// How often the workspace polls the session: about 30 times a second.
pub const POLL: Duration = Duration::from_millis(33);

/// Width of the open sidebar.
pub const SIDEBAR_WIDTH: f32 = 240.0;

/// The main window's content.
pub struct Workspace {
    session: Session,
    database: String,
    editor: Entity<EditorState>,
    results: Entity<TableState<ResultsDelegate>>,
    schema: Entity<SchemaTree>,
    sidebar_open: bool,
    focus: FocusHandle,
    sidebar_focus: FocusHandle,
    current: Option<RunHandle>,
    run: Option<Task<()>>,
    schema_read: Option<Task<()>>,
}

impl Workspace {
    /// A workspace on `session`, labelled `database` in the status bar.
    pub fn new(
        session: Session,
        database: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = editor::new_editor(editor::WELCOME, window, cx);
        let results = cx.new(|cx| TableState::new(ResultsDelegate::default(), window, cx));
        cx.observe_window_appearance(window, |_, _, cx| Aurora::system_appearance_changed(cx))
            .detach();
        let mut workspace = Self {
            session,
            database: database.to_owned(),
            editor,
            results,
            schema: cx.new(|_| SchemaTree::default()),
            sidebar_open: true,
            focus: cx.focus_handle(),
            sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
            current: None,
            run: None,
            schema_read: None,
        };
        workspace.read_schema(cx);
        workspace
    }

    /// The editor, so callers can focus it or set its text.
    pub fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// The sidebar's schema tree.
    pub fn schema(&self) -> &Entity<SchemaTree> {
        &self.schema
    }

    /// The run the grid shows.
    pub fn run_model<'a>(&self, cx: &'a App) -> &'a RunModel {
        self.results.read(cx).delegate().model()
    }

    /// Whether the sidebar is shown.
    pub fn sidebar_open(&self) -> bool {
        self.sidebar_open
    }

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    /// Focuses the sidebar's first table. A closed sidebar opens first, and
    /// focus moves once it has been drawn and its rows are tab stops.
    fn focus_sidebar(&mut self, _: &FocusSidebar, window: &mut Window, cx: &mut Context<Self>) {
        let container = self.sidebar_focus.clone();
        if self.sidebar_open {
            focus_first_stop(&container, window, cx);
            return;
        }
        self.sidebar_open = true;
        window.on_next_frame(move |window, cx| focus_first_stop(&container, window, cx));
        cx.notify();
    }

    fn focus_editor(&mut self, _: &FocusEditor, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |state, cx| state.focus(window, cx));
    }

    fn cycle_theme(_: &mut Self, _: &CycleTheme, _: &mut Window, cx: &mut Context<Self>) {
        let next = Aurora::preference(cx).next();
        Aurora::set_preference(next, cx);
    }
}

/// Focuses the first keyboard stop inside `container`.
fn focus_first_stop(container: &FocusHandle, window: &mut Window, cx: &mut App) {
    window.focus(container, cx);
    window.focus_next(cx);
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
```

`crates/app/src/views/workspace/layout.rs`:
```rust
//! Draws the workspace: sidebar, editor over results, status bar.

use std::time::Instant;

use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        input::Editor,
        resizable::{resizable_panel, v_resizable},
        table::DataTable,
    },
    div,
    prelude::FluentBuilder,
    px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::{
        status_bar::StatusBar,
        surface::{glass_panel, sidebar_surface, window_background},
    },
    tokens::{chrome, space, text},
};

use super::{SIDEBAR_WIDTH, Workspace};
use crate::{
    model::{format, status},
    shell::actions::WORKSPACE,
};

/// The editor panel's starting height; the divider below it drags.
pub const EDITOR_HEIGHT: f32 = 220.0;

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if Aurora::is_transitioning(cx) {
            window.request_animation_frame();
        }
        let colors = Aurora::colors(cx);
        div()
            .id("workspace")
            .key_context(WORKSPACE)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::run_statement))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::focus_sidebar))
            .on_action(cx.listener(Self::focus_editor))
            .on_action(cx.listener(Self::cycle_theme))
            .size_full()
            .flex()
            .bg(window_background(&colors))
            .text_color(colors.get(Role::Text))
            .when(self.sidebar_open, |root| root.child(self.sidebar(cx)))
            .child(self.center(cx))
    }
}

impl Workspace {
    fn sidebar(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        sidebar_surface(&colors)
            .id("sidebar")
            .track_focus(&self.sidebar_focus)
            .tab_group()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .pt(px(chrome::TITLEBAR_HEIGHT))
            .px(px(space::SM))
            .flex()
            .flex_col()
            .gap(px(space::SM))
            .overflow_y_scroll()
            .child(
                div()
                    .px(px(space::SM))
                    .text_size(px(text::CAPTION))
                    .text_color(colors.get(Role::TextMuted))
                    .child("TABLES"),
            )
            .child(self.schema.clone())
            .into_any_element()
    }

    fn center(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let editor = glass_panel(&colors)
            .size_full()
            .p(px(space::SM))
            .child(Editor::new(&self.editor).appearance(false).h_full());
        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .pt(px(chrome::TITLEBAR_HEIGHT))
            .child(
                div().flex_1().px(px(space::MD)).pb(px(space::SM)).child(
                    v_resizable("editor-results")
                        .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
                        .child(resizable_panel().child(self.results_panel(cx))),
                ),
            )
            .child(self.status_bar(cx))
            .into_any_element()
    }

    fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let failure = status::failure(self.run_model(cx));
        glass_panel(&colors)
            .id("results")
            .size_full()
            .mt(px(space::SM))
            .flex()
            .flex_col()
            .overflow_hidden()
            .children(failure.map(|(message, cancelled)| failure_banner(message, cancelled, cx)))
            .child(
                div()
                    .flex_1()
                    .child(DataTable::new(&self.results).bordered(false)),
            )
            .into_any_element()
    }

    fn status_bar(&self, cx: &Context<Self>) -> AnyElement {
        let model = self.run_model(cx);
        let summary = status::run_summary(model, model.elapsed(Instant::now()));
        let cursor = self.editor.read(cx).cursor_position();
        let place = format::position(cursor.line, cursor.character);
        div()
            .id("status")
            .child(
                StatusBar::new()
                    .left(format!("● {}", self.database))
                    .right(format!("{summary} · {place}")),
            )
            .into_any_element()
    }
}

/// The run's error above the grid; a cancel reads quietly.
fn failure_banner(message: String, cancelled: bool, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    let role = if cancelled {
        Role::TextSecondary
    } else {
        Role::Danger
    };
    div()
        .id("run-failure")
        .px(px(space::MD))
        .py(px(space::SM))
        .border_b_1()
        .border_color(colors.get(Role::Border))
        .text_size(px(text::BODY))
        .text_color(colors.get(role))
        .child(message)
        .test_support()
        .into_any_element()
}
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry --test workspace`
Expected: 11 tests pass.

Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/shell/actions.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/tests/workspace.rs
git commit -m "feat(app): focus the sidebar with Cmd-1 and the editor with Cmd-2"
```


---

### Task 7: The main window, start-up and docs

`main` installs the actions and Aurora, then opens the main window on the sample database with the editor focused. ADR 0009 records the app's structure; the README and spec are updated.

**Files:**
- Modify: `crates/app/src/shell/mod.rs`
- Create: `crates/app/src/shell/window.rs`
- Modify: `crates/app/src/main.rs`
- Create: `crates/app/tests/shell.rs`
- Create: `docs/adr/0009-app-shell.md`
- Modify: `README.md`
- Modify: `docs/superpowers/specs/2026-11-04-quarry-design.md`

**Interfaces:**
- Produces: `shell::window::{options(&App) -> WindowOptions, open_main(&mut App) -> Result<(), EngineError>, WINDOW_SIZE}`.

- [ ] **Step 1: Write the failing tests**

`crates/app/tests/shell.rs`:
```rust
//! Start-up: the main window opens on the sample database, and every command
//! has a key binding and a menu item.

use gpui_kit::{MenuItem, TestAppContext, px};
use quarry::shell::{actions, window};
use quarry_ui_kit::{Aurora, Preference};

#[gpui_kit::test]
fn the_main_window_opens_on_the_sample(cx: &mut TestAppContext) {
    let opened = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        assert_eq!(Aurora::install(Preference::Dark, cx), Ok(()));
        window::open_main(cx)
    });
    assert_eq!(opened, Ok(()));
    assert_eq!(cx.windows().len(), 1);
}

#[gpui_kit::test]
fn the_window_has_a_transparent_titlebar_and_the_default_size(cx: &mut TestAppContext) {
    let options = cx.update(|cx| window::options(cx));
    let titlebar = options.titlebar.unwrap_or_default();
    assert!(titlebar.appears_transparent);
    assert_eq!(titlebar.title.as_deref(), Some("Quarry"));
    let (width, height) = window::WINDOW_SIZE;
    let bounds = options.window_bounds.map(|b| b.get_bounds().size);
    assert_eq!(
        bounds.map(|s| (s.width, s.height)),
        Some((px(width), px(height)))
    );
}

#[test]
fn every_menu_command_has_a_key_binding() {
    let bound: Vec<&str> = actions::key_bindings()
        .iter()
        .map(|binding| binding.action().name())
        .collect();
    let menus = actions::menus();
    let commands: Vec<&str> = menus
        .iter()
        .flat_map(|menu| &menu.items)
        .filter_map(|item| match item {
            MenuItem::Action { action, .. } => Some(action.name()),
            _ => None,
        })
        .collect();
    assert_eq!(commands.len(), 7);
    for command in commands {
        assert!(bound.contains(&command), "{command} has no key binding");
    }
}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry --test shell`
Expected: `unresolved import quarry::shell::window`.

- [ ] **Step 3: Implement**

`crates/app/src/shell/mod.rs`:
```rust
//! Application start-up: actions, the menu bar and the main window.

pub mod actions;
pub mod window;
```

`crates/app/src/shell/window.rs`:
```rust
//! Opens the main window on the sample database.

use gpui_kit::{
    App, AppContext, Bounds, TitlebarOptions, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, point, px, size,
};
use quarry_engine::error::EngineError;
use quarry_ui_kit::tokens::chrome;

use crate::{sample, views::workspace::Workspace};

/// The main window's initial size.
pub const WINDOW_SIZE: (f32, f32) = (1200.0, 760.0);

/// Window chrome: transparent title bar, traffic lights inset, opaque
/// Aurora background (blur stays deferred, ADR 0008).
pub fn options(cx: &App) -> WindowOptions {
    let (width, height) = WINDOW_SIZE;
    let (x, y) = chrome::TRAFFIC_LIGHTS;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(width), px(height)),
            cx,
        ))),
        window_background: WindowBackgroundAppearance::Opaque,
        titlebar: Some(TitlebarOptions {
            title: Some("Quarry".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(x), px(y))),
        }),
        ..Default::default()
    }
}

/// Opens a workspace on a fresh sample database and focuses its editor.
///
/// # Errors
/// Returns the engine's error when the sample database cannot be opened.
pub fn open_main(cx: &mut App) -> Result<(), EngineError> {
    let session = sample::open()?;
    let opened = gpui_kit::open_window(options(cx), cx, |window, cx| {
        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, window, cx));
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| state.focus(window, cx));
        workspace
    });
    opened.map(|_| ()).map_err(|error| EngineError::Open {
        target: "the main window".to_owned(),
        message: error.to_string(),
    })
}
```

`crates/app/src/main.rs`:
```rust
//! Quarry: a native macOS SQL client for learning and real work.

use gpui_kit::App;
use quarry::shell::{actions, window};
use quarry_ui_kit::{Aurora, Preference};

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            actions::install(cx);
            let started = Aurora::install(Preference::Dark, cx)
                .map_err(|error| error.to_string())
                .and_then(|()| window::open_main(cx).map_err(|error| error.to_string()));
            if let Err(error) = started {
                eprintln!("Quarry could not start: {error}");
                cx.quit();
                return;
            }
            cx.activate(true);
        });
}
```

`docs/adr/0009-app-shell.md`:
```markdown
# ADR 0009: App shell structure

- **Status:** Accepted
- **Date:** 2026-11-09

## Context
Phase 1C builds the first usable Quarry window: the sample database, the schema sidebar, the SQL editor, Run and Cancel, a streaming results grid and the status bar. The `app` crate's shape is set here for every later screen.

## Decision
- **Three layers in `app`:**
  - `model` holds GPUI-free view models: the run target, the run state built from `RunEvent`s, display formatting, the status summary and the schema-tree rows. They are pure, fully unit-tested and mutation-tested like the core crates (`.cargo/mutants.toml` excludes only `views`, `shell` and `main.rs`).
  - `views` renders them. The workspace is split into its state and commands (`workspace/mod.rs`), run driving (`workspace/runner.rs`) and drawing (`workspace/layout.rs`).
  - `shell` owns the actions, key bindings, menus and the main window. Adding a command is one `actions!` entry, one key binding and one menu item.
- **Polling, not awaiting:** the workspace polls the session's channels every 33 ms (`POLL`) with `try_recv`, rather than awaiting them. A waker fired from the session thread wakes GPUI's executor from outside, which the test scheduler rejects as non-deterministic. Polling also batches many row events into one redraw, and the same poll drives the live timer.
- **One result at a time:** a Run sends the selection, or the statement under the cursor. With several statements, each `Started` replaces the grid's result. The Messages view and one tab per statement come in Phase 1D.
- **Key bindings in the editor:** the editor binds ⌘↵ (newline) and ⌘. (code actions) itself. Quarry binds Run and Cancel again in the `Workspace > Input` context, which is deeper than the editor's own, so its bindings win.
- **Keyboard focus:** interactive sidebar rows, tabs and segments are tab stops (`keyboard_focusable`). Enter and Space click them, and a `FocusRing` border (3:1 on every surface) shows while they're focused. The editor keeps Tab for indentation, so ⌘1 focuses the sidebar's first table and ⌘2 returns to the editor. Components keep a transparent 1 px border so gaining focus never moves them.
- **Sample database:** `sample::open` seeds an in-memory SQLite database from `seed.sql`. It's a small space agency with 200,000 telemetry rows to stream and cancel. Phase 2's track sandboxes replace it.

## Consequences
- Logic that can be pure goes in `model` and carries the core crates' guarantees. GPUI code stays thin.
- Results reach the screen up to 33 ms late, which no one can see.
- ⌘1 on a closed sidebar focuses it on the next frame. GPUI's test platform doesn't run next-frame callbacks, so that path is checked by hand in the visual review.
```

Apply this change to `README.md` (`git apply`, or edit by hand):
```diff
diff --git a/README.md b/README.md
index 1baa779..15a6ad2 100644
--- a/README.md
+++ b/README.md
@@ -2,7 +2,7 @@
 
 A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.
 
-> **Status:** Phase 0 (foundations). Not usable yet. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
+> **Status:** Phase 1C. You can write and run SQL against a built-in sample database. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
 
 - Built-in SQLite playground with story-driven SQL challenge tracks
 - PostgreSQL and MySQL connections for everyday work
@@ -13,6 +13,7 @@ A native macOS SQL client for learning SQL by hand and for real work, built in R
     brew install just
     just setup     # installs pinned tools and git hooks
     just check     # every gate, identical to CI
+    cargo run -p quarry   # open Quarry on the sample database
 
 See [CONTRIBUTING.md](CONTRIBUTING.md) for the engineering standards.
```

Apply this change to `docs/superpowers/specs/2026-11-04-quarry-design.md` (`git apply`, or edit by hand):
```diff
diff --git a/docs/superpowers/specs/2026-11-04-quarry-design.md b/docs/superpowers/specs/2026-11-04-quarry-design.md
index e530882..b47e7f3 100644
--- a/docs/superpowers/specs/2026-11-04-quarry-design.md
+++ b/docs/superpowers/specs/2026-11-04-quarry-design.md
@@ -145,7 +145,7 @@ Reference mockups are in `docs/design/mockups/` (`visual-direction.html`, `light
 
 **Workspace (three columns)**
 
-1. **Sidebar** (frosted glass, toggled with ⌘\\)
+1. **Sidebar** (frosted glass, toggled with ⌘\\; ⌘1 focuses it and ⌘2 returns to the editor)
    - A Playground/Connections switch.
    - Tracks, each with a progress ring.
    - The schema tree: tables with row counts, and columns with their types. Clicking a table opens it in a browse tab. Dragging a name into the editor inserts it as text, which is a manual action and not autocomplete.
@@ -304,7 +304,7 @@ A coverage matrix in `content/COVERAGE.md` maps every concept to at least two ch
 | `engine`, `challenges`, `store` | ≥ 95% | ≥ 90% |
 | `ui-kit`, `app` | ≥ 80% | ≥ 70% |
 
-**Mutation testing** (`cargo-mutants`) runs on `engine`, `challenges` and `store`. PRs test only the changed code, and the whole codebase runs weekly. Gate: at least 90% of introduced bugs caught.
+**Mutation testing** (`cargo-mutants`) runs on `engine`, `challenges`, `store` and the `app` view models (`crates/app/src/model`, ADR 0009). PRs test only the changed code, and the whole codebase runs weekly. Gate: at least 90% of introduced bugs caught.
 
 ### 7.4 Linting and supply chain
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry --test shell`
Expected: 3 tests pass.

Run: `just check`
Expected: exit 0.

Run: `just mutants`
Expected: exit 0, 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Visual review (the user)**

Run `cargo run --release -p quarry` and ask the user to check, in Dark and Light (⌘⇧T):
1. The layout: sidebar with TABLES, editor panel, a draggable divider, the results panel and the status bar.
2. ⌘↵ on the welcome query fills the grid; NULL call signs are dim italic; numbers line up on the right.
3. `SELECT * FROM telemetry` streams (the status bar counts rows and time live); ⌘. cancels it and the banner reads "Query cancelled."
4. `SELECT * FROM nowhere` shows the error banner in the danger color.
5. Clicking a table lists its columns with types and `PK`; `CREATE TABLE notes (body TEXT)` adds `notes`.
6. ⌘\ hides the sidebar; **⌘1 with the sidebar hidden** opens it and shows the focus ring on the first table (this path can't run headlessly, ADR 0009); Enter expands it; ⌘2 returns to the editor.
7. There is no completion popup, ghost text or suggestion of any kind while typing.

Fix anything the user rejects before opening the PR.

- [ ] **Open the PR**

```bash
git push -u origin phase-1c-app-shell
gh pr create --base main --title "Phase 1C: app shell" --body "<summary of the seven tasks and the test plan>"
gh pr checks --watch
```

- [ ] **Step 5: Commit**

```bash
git add README.md crates/app/src/main.rs crates/app/src/shell/mod.rs crates/app/src/shell/window.rs crates/app/tests/shell.rs docs/adr/0009-app-shell.md docs/superpowers/specs/2026-11-04-quarry-design.md
git commit -m "feat(app): open the main window on the sample database and document the shell"
```


---
