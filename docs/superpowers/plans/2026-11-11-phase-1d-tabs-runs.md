# Phase 1D: Tabs, Drafts, Run All, Messages and Plan — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Turn Phase 1C's single editor into query tabs whose drafts survive restarts. Add Run all with a result per statement, the Messages and Plan views, Explain, and every deferred minor from the 1C final review.

**Architecture:**
- **`quarry-store`:** the `DraftStore` trait and a migrated SQLite file.
- **`quarry-engine`:** `Driver::plan` and `Session::plan`.
- **`app` models:** GPUI-free and mutation-tested. They cover the tab list, each statement's output, messages and plan rows.
- **`app` views:**
  - a `QueryTab` entity per tab, with its own editor, run and results;
  - a `ResultViewRegistry` of Results, Messages and Plan;
  - the workspace, which owns the one `Session` and the tab list.
- **`app` shell:** the store's location, and quitting with the last window.
- **Status:** every task's code was built and gated in a scratch clone first, one commit per task, with `just check` passing at the end.

**Tech Stack:** Rust 1.98.1, `gpui-kit` =0.9.0, `rusqlite` 0.41.1, `async-channel`.

**Spec:** `docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md` (approved 2026-11-09). Parent: `docs/superpowers/specs/2026-11-04-quarry-design.md` §3.3 and §4.2. ADR 0009 (app shell) and ADR 0010 (written in Task 9).

## Global Constraints

- **Branch and commits:** work on `phase-1d-tabs-runs`, which already holds the spec and this plan. Use Conventional Commits, with no trailers.
- **No completion of any kind:** never attach a completion, hover, definition or code-action provider to the editor. The existing test keeps asserting it.
- **Dependency direction:**
  - `quarry` may depend on `quarry-ui-kit`, `quarry-engine`, `quarry-challenges` and `quarry-store`.
  - `quarry-store` may depend on `quarry-engine`, but needs no workspace crate in 1D.
  - `quarry-engine` depends on none.
  - `cargo xtask check-deps` enforces this.
- **Coverage:**
  - `engine` and `store` need at least 95% lines and 90% regions.
  - `app` needs at least 80% lines and 70% regions.
  - Every file in `crates/app/src/model` stays at 100%.
- **Mutation testing:** 0 missed for `engine`, `store`, `crates/app/src/model` and `crates/app/src/sample`.
- **Per function:** cyclomatic ≤ 10, cognitive ≤ 15, ≤ 15 statements, ≤ 40 lines, ≤ 5 arguments, nesting ≤ 4. This applies to tests too.
- **Banned outside `#[test]` functions:** `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` and `dbg!`.
  - Test helpers that aren't `#[test]` functions count as outside.
  - No lint suppression attributes, and no `as` casts.
- **Polling, never awaiting, the session:** GPUI code polls session channels with `model::drain` every `POLL` (33 ms). Background work uses GPUI's executor.
- **Row cap:** each statement keeps at most `ROW_LIMIT` = 100,000 rows and counts the rest.
- **Headless testing:**
  - Elements tests find by id call `.test_support()`.
  - Tests advance the clock by `POLL` and pump the executor (`settle`).
  - **Every task ends with `just check` passing.** Don't commit anything under `.superpowers/`.
- **Merging:** only with the user's explicit go-ahead.

## File Structure

```
crates/store/src/{lib.rs, drafts.rs, error.rs, migrations.rs, sqlite.rs}   draft store
crates/store/tests/drafts.rs
crates/engine/src/plan.rs, src/sqlite/explain.rs, tests/plan.rs            query plans
crates/app/src/model/
  tabs.rs           tab order, titles, active tab
  statements.rs     RunOutput: each statement's output (replaces run.rs)
  messages.rs       one line per statement
  plan_tree.rs      plan rows and PlanState
  status.rs, format.rs, drain.rs   (changed)
crates/app/src/views/
  query_tab/{mod.rs, pump.rs, layout.rs, explain.rs}   one tab: editor, run, results, plan
  results/{mod.rs, grid.rs, table.rs, messages.rs, plan.rs}   ResultViewRegistry and views
  workspace/{mod.rs, runs.rs, tabs.rs, drafts.rs, tab_strip.rs, layout.rs}
crates/app/src/shell/{actions.rs, store.rs, window.rs}
crates/app/tests/app/{main.rs, support.rs, workspace.rs, tabs.rs, runs.rs}
crates/app/tests/shell.rs
docs/adr/0010-tabs-and-drafts.md
```

---


### Task 1: `quarry-store`: the draft store

The store crate gets its first contents: the `DraftStore` trait for the query tabs' drafts, and `SqliteStore`, a local SQLite file migrated with `PRAGMA user_version`. A file written by a newer Quarry is refused and left byte-identical. The app wires it up in Task 6.

**Files:**
- Modify: `crates/store/Cargo.toml`
- Create: `crates/store/src/drafts.rs`
- Create: `crates/store/src/error.rs`
- Modify: `crates/store/src/lib.rs`
- Create: `crates/store/src/migrations.rs`
- Create: `crates/store/src/sqlite.rs`
- Create: `crates/store/tests/drafts.rs`

**Interfaces:**
- Consumes: `rusqlite` 0.41.1 (workspace dependency), `thiserror`.
- Produces: `quarry_store::drafts::{Draft { id: i64, title: String, text: String }, Drafts { tabs: Vec<Draft>, active: usize }, DraftStore: Send + Sync { load(&self) -> Result<Drafts, StoreError>; save(&self, &Drafts) -> Result<(), StoreError> }}`; `quarry_store::sqlite::SqliteStore::{open(&Path), in_memory()}`; `quarry_store::error::StoreError::{Open { path, message }, TooNew { found: i64, supported: i64 }, Sql(String)}`.

- [ ] **Step 1: Write the failing tests**

Create `crates/store/tests/drafts.rs`:
```rust
//! The draft store round-trips tabs through memory and through a file.

use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};

fn draft(id: i64, title: &str, text: &str) -> Draft {
    Draft {
        id,
        title: title.to_owned(),
        text: text.to_owned(),
    }
}

fn sample() -> Drafts {
    Drafts {
        tabs: vec![
            draft(7, "Query 2", "SELECT 2;"),
            draft(3, "Query 1", "SELECT 1;\n-- é ✓"),
        ],
        active: 1,
    }
}

#[test]
fn an_empty_store_has_no_tabs() {
    let store = SqliteStore::in_memory().unwrap();
    assert_eq!(store.load().unwrap(), Drafts::default());
}

#[test]
fn round_trips_in_memory_keeping_order() {
    let store = SqliteStore::in_memory().unwrap();
    store.save(&sample()).unwrap();
    assert_eq!(store.load().unwrap(), sample());
}

#[test]
fn a_save_replaces_the_previous_one() {
    let store = SqliteStore::in_memory().unwrap();
    store.save(&sample()).unwrap();
    let fewer = Drafts {
        tabs: vec![draft(9, "Query 5", "")],
        active: 0,
    };
    store.save(&fewer).unwrap();
    assert_eq!(store.load().unwrap(), fewer);
    store.save(&Drafts::default()).unwrap();
    assert_eq!(store.load().unwrap(), Drafts::default());
}

#[test]
fn clamps_an_active_index_past_the_last_tab() {
    let store = SqliteStore::in_memory().unwrap();
    let mut drafts = sample();
    drafts.active = 9;
    store.save(&drafts).unwrap();
    assert_eq!(store.load().unwrap().active, 1);
}

#[test]
fn round_trips_through_a_file_in_a_new_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Quarry").join("quarry.db");
    SqliteStore::open(&path).unwrap().save(&sample()).unwrap();
    assert_eq!(SqliteStore::open(&path).unwrap().load().unwrap(), sample());
}

#[test]
fn leaves_a_newer_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quarry.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 5).unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    let error = SqliteStore::open(&path).err();
    assert_eq!(
        error,
        Some(StoreError::TooNew {
            found: 5,
            supported: 1
        })
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn reports_a_path_that_cannot_be_created() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, "not a directory").unwrap();
    let error = SqliteStore::open(&blocker.join("quarry.db")).err();
    assert!(matches!(error, Some(StoreError::Open { .. })), "{error:?}");
}

#[test]
fn reports_a_file_that_is_not_a_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quarry.db");
    std::fs::write(&path, vec![b'x'; 4096]).unwrap();
    let error = SqliteStore::open(&path).err();
    assert!(matches!(error, Some(StoreError::Sql(_))), "{error:?}");
}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry-store`
Expected: `unresolved import quarry_store::drafts` (the crate is empty).

- [ ] **Step 3: Implement**

Change `crates/store/Cargo.toml` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/store/Cargo.toml b/crates/store/Cargo.toml
--- a/crates/store/Cargo.toml
+++ b/crates/store/Cargo.toml
@@ -6,5 +6,12 @@ rust-version.workspace = true
 publish.workspace = true
 license.workspace = true
 
+[dependencies]
+rusqlite.workspace = true
+thiserror.workspace = true
+
+[dev-dependencies]
+tempfile.workspace = true
+
 [lints]
 workspace = true
```

Create `crates/store/src/drafts.rs`:
```rust
//! The query tabs' drafts, saved so they survive a restart.

use crate::error::StoreError;

/// One query tab's saved text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The tab's identity, stable for the tab's lifetime.
    pub id: i64,
    /// The tab's title, such as "Query 3".
    pub title: String,
    /// The editor text.
    pub text: String,
}

/// Every open tab in order, and which one is active.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drafts {
    /// The tabs, left to right.
    pub tabs: Vec<Draft>,
    /// The index of the active tab in `tabs`.
    pub active: usize,
}

/// Loads and saves the drafts.
pub trait DraftStore: Send + Sync {
    /// Reads the saved drafts; an empty store has no tabs.
    ///
    /// # Errors
    /// Returns a [`StoreError`] when the store cannot be read.
    fn load(&self) -> Result<Drafts, StoreError>;

    /// Replaces the saved drafts with `drafts`.
    ///
    /// # Errors
    /// Returns a [`StoreError`] when the store cannot be written.
    fn save(&self, drafts: &Drafts) -> Result<(), StoreError>;
}
```

Create `crates/store/src/error.rs`:
```rust
//! Errors a user can see, each with a readable message.

/// Why the store could not be opened, read or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// The store file could not be opened or created.
    #[error("Cannot open {path}: {message}")]
    Open {
        /// The file being opened.
        path: String,
        /// Why it failed.
        message: String,
    },
    /// The file was written by a newer Quarry; it is left untouched.
    #[error("Saved by a newer Quarry (version {found}; this one reads up to {supported}).")]
    TooNew {
        /// The file's schema version.
        found: i64,
        /// The newest schema version this build knows.
        supported: i64,
    },
    /// `SQLite` reported an error.
    #[error("{0}")]
    Sql(String),
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::StoreError;

    #[test]
    fn messages_are_readable_sentences() {
        let open = StoreError::Open {
            path: "/x/quarry.db".to_owned(),
            message: "permission denied".to_owned(),
        };
        assert_eq!(
            open.to_string(),
            "Cannot open /x/quarry.db: permission denied"
        );
        let too_new = StoreError::TooNew {
            found: 4,
            supported: 1,
        };
        assert_eq!(
            too_new.to_string(),
            "Saved by a newer Quarry (version 4; this one reads up to 1)."
        );
        let sql = StoreError::from(rusqlite::Error::InvalidQuery);
        assert_eq!(
            sql,
            StoreError::Sql(rusqlite::Error::InvalidQuery.to_string())
        );
    }
}
```

Change `crates/store/src/lib.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/store/src/lib.rs b/crates/store/src/lib.rs
--- a/crates/store/src/lib.rs
+++ b/crates/store/src/lib.rs
@@ -1 +1,9 @@
 //! Local persistence: settings, connections, history, and progress.
+//!
+//! Phase 1D stores the query tabs' drafts. Each kind of data gets a trait
+//! ([`drafts::DraftStore`]) so the app can swap in a fake or a fallback.
+
+pub mod drafts;
+pub mod error;
+mod migrations;
+pub mod sqlite;
```

Create `crates/store/src/migrations.rs`:
```rust
//! Schema migrations, tracked with `PRAGMA user_version`.

use rusqlite::Connection;

use crate::error::StoreError;

/// Each step moves the schema up one version; step `n` makes version `n + 1`.
const STEPS: &[&str] = &["CREATE TABLE drafts (
        id INTEGER PRIMARY KEY,
        position INTEGER NOT NULL,
        title TEXT NOT NULL,
        text TEXT NOT NULL
    );
    CREATE TABLE state (key TEXT PRIMARY KEY, value INTEGER NOT NULL);"];

/// The schema version this build writes.
pub const VERSION: i64 = 1;

/// Brings the schema up to [`VERSION`] in one transaction.
///
/// # Errors
/// Returns [`StoreError::TooNew`] for a file from a newer build, without
/// changing it, or [`StoreError::Sql`] when a step fails.
pub fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
    let found: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if found > VERSION {
        return Err(StoreError::TooNew {
            found,
            supported: VERSION,
        });
    }
    let transaction = connection.transaction()?;
    for step in STEPS.iter().skip(usize::try_from(found).unwrap_or(0)) {
        transaction.execute_batch(step)?;
    }
    transaction.pragma_update(None, "user_version", VERSION)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{STEPS, VERSION, migrate};
    use crate::error::StoreError;

    fn version(connection: &Connection) -> i64 {
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn version_matches_the_number_of_steps() {
        assert_eq!(usize::try_from(VERSION).unwrap(), STEPS.len());
    }

    #[test]
    fn migrates_an_empty_database_and_is_idempotent() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        assert_eq!(version(&connection), VERSION);
        migrate(&mut connection).unwrap();
        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name IN ('drafts', 'state')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 2);
    }

    #[test]
    fn refuses_a_newer_schema() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 7).unwrap();
        assert_eq!(
            migrate(&mut connection),
            Err(StoreError::TooNew {
                found: 7,
                supported: VERSION
            })
        );
        assert_eq!(version(&connection), 7);
    }
}
```

Create `crates/store/src/sqlite.rs`:
```rust
//! The `SQLite` file behind every store trait.

use std::{
    path::Path,
    sync::{Mutex, MutexGuard, PoisonError},
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    migrations,
};

/// Quarry's local database: one file, migrated on open.
pub struct SqliteStore {
    connection: Mutex<Connection>,
}

impl SqliteStore {
    /// Opens (or creates) the store at `path`, creating its directory.
    ///
    /// # Errors
    /// Returns [`StoreError::Open`] when the file cannot be opened, or a
    /// migration error (see [`migrations::migrate`]).
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let open_error = |message: String| StoreError::Open {
            path: path.display().to_string(),
            message,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| open_error(error.to_string()))?;
        }
        let connection = Connection::open(path).map_err(|error| open_error(error.to_string()))?;
        Self::migrated(connection)
    }

    /// Opens a fresh store that lives only in memory.
    ///
    /// # Errors
    /// Returns [`StoreError::Sql`] if `SQLite` cannot allocate it.
    pub fn in_memory() -> Result<Self, StoreError> {
        Self::migrated(Connection::open_in_memory()?)
    }

    fn migrated(mut connection: Connection) -> Result<Self, StoreError> {
        migrations::migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// A position as `SQLite` stores it; no real tab list comes near `i64::MAX`.
fn index(position: usize) -> i64 {
    i64::try_from(position).unwrap_or(i64::MAX)
}

impl DraftStore for SqliteStore {
    fn load(&self) -> Result<Drafts, StoreError> {
        let connection = self.connection();
        let mut statement =
            connection.prepare("SELECT id, title, text FROM drafts ORDER BY position")?;
        let tabs = statement
            .query_map([], |row| {
                Ok(Draft {
                    id: row.get(0)?,
                    title: row.get(1)?,
                    text: row.get(2)?,
                })
            })?
            .collect::<Result<Vec<_>, _>>()?;
        let active: Option<i64> = connection
            .query_row("SELECT value FROM state WHERE key = 'active'", [], |row| {
                row.get(0)
            })
            .optional()?;
        let active = usize::try_from(active.unwrap_or(0)).unwrap_or(0);
        Ok(Drafts {
            active: active.min(tabs.len().saturating_sub(1)),
            tabs,
        })
    }

    fn save(&self, drafts: &Drafts) -> Result<(), StoreError> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        transaction.execute("DELETE FROM drafts", [])?;
        for (position, draft) in drafts.tabs.iter().enumerate() {
            transaction.execute(
                "INSERT INTO drafts (id, position, title, text) VALUES (?1, ?2, ?3, ?4)",
                params![draft.id, index(position), draft.title, draft.text],
            )?;
        }
        transaction.execute(
            "INSERT OR REPLACE INTO state (key, value) VALUES ('active', ?1)",
            params![index(drafts.active)],
        )?;
        transaction.commit()?;
        Ok(())
    }
}
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry-store`
Expected: 12 tests pass (4 unit, 8 integration).

Run: `just check`
Expected: exit 0.

Run: `cargo mutants -p quarry-store --in-diff <(git diff main)`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Step 5: Commit**

```bash
git add -A crates/store/Cargo.toml crates/store/src/drafts.rs crates/store/src/error.rs crates/store/src/lib.rs crates/store/src/migrations.rs crates/store/src/sqlite.rs crates/store/tests/drafts.rs
git commit -m "feat(store): add the draft store with versioned migrations"
```


---

### Task 2: Engine: query plans

Drivers learn to describe how they would run a statement. `Driver::plan` defaults to `PlanUnsupported`; SQLite builds a tree from `EXPLAIN QUERY PLAN`; `Session::plan` runs it on the session thread after any queued run, like `schema()`. The private `begin_schema` becomes `begin_read`, shared by both.

**Files:**
- Modify: `crates/engine/src/driver.rs`
- Modify: `crates/engine/src/error.rs`
- Modify: `crates/engine/src/lib.rs`
- Create: `crates/engine/src/plan.rs`
- Modify: `crates/engine/src/session.rs`
- Create: `crates/engine/src/sqlite/explain.rs`
- Modify: `crates/engine/src/sqlite/mod.rs`
- Create: `crates/engine/tests/plan.rs`

**Interfaces:**
- Consumes: `Driver`, `Session`, `EngineError` (Phase 1A).
- Produces: `quarry_engine::plan::{Plan { nodes: Vec<PlanNode> }, PlanNode { label, children }, PlanStep { id, parent, label }, Plan::from_steps(&[PlanStep])}`; `Driver::plan(&mut self, &str) -> Result<Plan, EngineError>` (default `Err(PlanUnsupported)`); `Session::plan(&self, impl Into<String>) -> Receiver<Result<Plan, EngineError>>`; `EngineError::PlanUnsupported` ("This database can't show query plans.").

- [ ] **Step 1: Write the failing tests**

Create `crates/engine/tests/plan.rs`:
```rust
//! Query plans from the `SQLite` driver and through a session.

use quarry_engine::{
    driver::{Canceller, Driver},
    error::EngineError,
    plan::PlanNode,
    result::{RowSink, Summary},
    schema::Schema,
    session::{RunEvent, Session},
    sqlite::SqliteDriver,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SPACE: &str = "CREATE TABLE crew (id INTEGER PRIMARY KEY, name TEXT, ship INTEGER);
    CREATE TABLE ships (id INTEGER PRIMARY KEY, name TEXT);
    CREATE INDEX crew_name ON crew (name);";

fn driver() -> Result<SqliteDriver, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script(SPACE)?;
    Ok(driver)
}

fn labels(nodes: &[PlanNode]) -> Vec<&str> {
    nodes.iter().map(|node| node.label.as_str()).collect()
}

#[test]
fn an_index_lookup_is_one_search_step() -> TestResult {
    let plan = driver()?.plan("SELECT id FROM crew WHERE name = 'Ada'")?;
    assert_eq!(
        labels(&plan.nodes),
        ["SEARCH crew USING COVERING INDEX crew_name (name=?)"]
    );
    assert!(plan.nodes[0].children.is_empty());
    Ok(())
}

#[test]
fn a_join_has_one_step_per_table() -> TestResult {
    let plan = driver()?
        .plan("SELECT crew.name, ships.name FROM crew JOIN ships ON ships.id = crew.ship")?;
    assert_eq!(
        labels(&plan.nodes),
        [
            "SCAN crew",
            "SEARCH ships USING INTEGER PRIMARY KEY (rowid=?)"
        ]
    );
    Ok(())
}

#[test]
fn a_correlated_subquery_nests_its_steps() -> TestResult {
    let plan = driver()?
        .plan("SELECT name, (SELECT count(*) FROM crew WHERE crew.ship = ships.id) FROM ships")?;
    assert_eq!(
        labels(&plan.nodes),
        ["SCAN ships", "CORRELATED SCALAR SUBQUERY 1"]
    );
    assert_eq!(labels(&plan.nodes[1].children), ["SCAN crew"]);
    Ok(())
}

#[test]
fn sql_that_cannot_be_planned_is_a_sql_error() -> TestResult {
    let error = driver()?.plan("SELECT * FROM nowhere").err();
    assert_eq!(
        error,
        Some(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    Ok(())
}

#[test]
fn a_session_plans_after_queued_runs() -> TestResult {
    let session = Session::start(Box::new(driver()?));
    let run = session.run("CREATE TABLE late (x INTEGER)");
    let plan = session.plan("SELECT x FROM late").recv_blocking()??;
    assert_eq!(labels(&plan.nodes), ["SCAN late"]);
    while run.events.recv_blocking()? != RunEvent::Done {}
    Ok(())
}

/// A driver that keeps the default `plan`.
struct Plain;

impl Driver for Plain {
    fn execute(&mut self, _sql: &str, _sink: &mut dyn RowSink) -> Result<Summary, EngineError> {
        Err(EngineError::Cancelled)
    }

    fn schema(&mut self) -> Result<Schema, EngineError> {
        Ok(Schema::default())
    }

    fn canceller(&self) -> Canceller {
        Canceller::default()
    }
}

#[test]
fn a_driver_without_plans_says_so() -> TestResult {
    let session = Session::start(Box::new(Plain));
    assert_eq!(
        session.plan("SELECT 1").recv_blocking()?,
        Err(EngineError::PlanUnsupported)
    );
    Ok(())
}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry-engine`
Expected: `unresolved import quarry_engine::plan`.

- [ ] **Step 3: Implement**

Change `crates/engine/src/driver.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/engine/src/driver.rs b/crates/engine/src/driver.rs
--- a/crates/engine/src/driver.rs
+++ b/crates/engine/src/driver.rs
@@ -7,6 +7,7 @@ use std::sync::{
 
 use crate::{
     error::EngineError,
+    plan::Plan,
     result::{RowSink, Summary},
     schema::Schema,
 };
@@ -29,6 +30,17 @@ pub trait Driver: Send {
     /// Returns an error when the catalog cannot be read.
     fn schema(&mut self) -> Result<Schema, EngineError>;
 
+    /// Describes how the database would run the first statement in `sql`,
+    /// without running it.
+    ///
+    /// # Errors
+    /// Returns [`EngineError::PlanUnsupported`] unless the driver overrides
+    /// it, or the database's error for SQL it cannot plan.
+    fn plan(&mut self, sql: &str) -> Result<Plan, EngineError> {
+        let _ = sql;
+        Err(EngineError::PlanUnsupported)
+    }
+
     /// The flag that interrupts whatever this driver is running.
     fn canceller(&self) -> Canceller;
 }
```

Change `crates/engine/src/error.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/engine/src/error.rs b/crates/engine/src/error.rs
--- a/crates/engine/src/error.rs
+++ b/crates/engine/src/error.rs
@@ -23,6 +23,9 @@ pub enum EngineError {
     /// No driver handles this connection string.
     #[error("Unsupported connection: {0}")]
     Unsupported(String),
+    /// The driver cannot show query plans.
+    #[error("This database can't show query plans.")]
+    PlanUnsupported,
     /// The session thread has stopped.
     #[error("The query worker stopped unexpectedly.")]
     WorkerGone,
@@ -51,6 +54,10 @@ mod tests {
             EngineError::Unsupported("x:y".to_owned()).to_string(),
             "Unsupported connection: x:y"
         );
+        assert_eq!(
+            EngineError::PlanUnsupported.to_string(),
+            "This database can't show query plans."
+        );
         assert_eq!(
             EngineError::WorkerGone.to_string(),
             "The query worker stopped unexpectedly."
```

Change `crates/engine/src/lib.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/engine/src/lib.rs b/crates/engine/src/lib.rs
--- a/crates/engine/src/lib.rs
+++ b/crates/engine/src/lib.rs
@@ -5,6 +5,7 @@
 
 pub mod driver;
 pub mod error;
+pub mod plan;
 pub mod registry;
 pub mod result;
 pub mod schema;
```

Create `crates/engine/src/plan.rs`:
```rust
//! A query plan: how the database intends to run a statement.

/// The steps a database will take to run one statement, as a tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// The top-level steps, in the order the database reports them.
    pub nodes: Vec<PlanNode>,
}

/// One step of a plan and the steps nested under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    /// The database's description of the step.
    pub label: String,
    /// Nested steps, in order.
    pub children: Vec<PlanNode>,
}

/// One reported step: its id, its parent's id (0 for a top-level step), and
/// its description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    /// The step's id; ids are positive.
    pub id: i64,
    /// The parent step's id, or 0.
    pub parent: i64,
    /// The step's description.
    pub label: String,
}

impl Plan {
    /// Builds the tree from flat steps, keeping their order.
    ///
    /// A step is nested only under a parent with a smaller id, so a malformed
    /// list (a step naming itself or a later step) cannot loop; such a step
    /// is dropped.
    pub fn from_steps(steps: &[PlanStep]) -> Self {
        Self {
            nodes: children(steps, 0),
        }
    }
}

fn children(steps: &[PlanStep], parent: i64) -> Vec<PlanNode> {
    steps
        .iter()
        .filter(|step| step.parent == parent && step.id > parent)
        .map(|step| PlanNode {
            label: step.label.clone(),
            children: children(steps, step.id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Plan, PlanNode, PlanStep};

    fn step(id: i64, parent: i64, label: &str) -> PlanStep {
        PlanStep {
            id,
            parent,
            label: label.to_owned(),
        }
    }

    fn leaf(label: &str) -> PlanNode {
        PlanNode {
            label: label.to_owned(),
            children: Vec::new(),
        }
    }

    #[test]
    fn nests_steps_under_their_parents_in_order() {
        let plan = Plan::from_steps(&[
            step(2, 0, "SCAN a"),
            step(4, 0, "CORRELATED SUBQUERY 1"),
            step(7, 4, "SEARCH b"),
            step(9, 4, "USE TEMP B-TREE"),
        ]);
        assert_eq!(
            plan.nodes,
            vec![
                leaf("SCAN a"),
                PlanNode {
                    label: "CORRELATED SUBQUERY 1".to_owned(),
                    children: vec![leaf("SEARCH b"), leaf("USE TEMP B-TREE")],
                },
            ]
        );
    }

    #[test]
    fn drops_steps_that_would_loop() {
        let plan = Plan::from_steps(&[step(3, 0, "SCAN a"), step(5, 5, "self"), step(4, 6, "x")]);
        assert_eq!(plan.nodes, vec![leaf("SCAN a")]);
    }

    #[test]
    fn drops_a_reachable_step_that_names_itself_as_parent() {
        let plan = Plan::from_steps(&[step(5, 0, "SCAN a"), step(5, 5, "self")]);
        assert_eq!(plan.nodes, vec![leaf("SCAN a")]);
    }

    #[test]
    fn no_steps_is_an_empty_plan() {
        assert_eq!(Plan::from_steps(&[]), Plan::default());
    }
}
```

Change `crates/engine/src/session.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/engine/src/session.rs b/crates/engine/src/session.rs
--- a/crates/engine/src/session.rs
+++ b/crates/engine/src/session.rs
@@ -5,7 +5,7 @@
 //! time, in submission order. Dropping a handle stops its run. Dropping the
 //! session stops runs: the current statement is interrupted, later statements
 //! do not start, and queued runs fail with [`EngineError::Cancelled`]. A
-//! schema read queued before the drop still completes.
+//! schema read or plan queued before the drop still completes.
 //!
 //! # Using a session from the UI
 //!
@@ -37,6 +37,7 @@ use async_channel::{Receiver, Sender};
 use crate::{
     driver::{Canceller, Driver},
     error::EngineError,
+    plan::Plan,
     result::{Column, Row, RowSink, Summary},
     schema::Schema,
     splitter,
@@ -108,6 +109,10 @@ enum Job {
         events: Sender<RunEvent>,
     },
     Schema(Sender<Result<Schema, EngineError>>),
+    Plan {
+        sql: String,
+        reply: Sender<Result<Plan, EngineError>>,
+    },
 }
 
 impl Session {
@@ -160,6 +165,20 @@ impl Session {
         }
         receiver
     }
+
+    /// Plans the first statement in `sql` on the session thread, after any
+    /// queued runs.
+    pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>> {
+        let (reply, receiver) = async_channel::bounded(1);
+        let job = Job::Plan {
+            sql: sql.into(),
+            reply,
+        };
+        if let Err(mpsc::SendError(Job::Plan { reply, .. })) = self.jobs.send(job) {
+            let _ = reply.try_send(Err(EngineError::WorkerGone));
+        }
+        receiver
+    }
 }
 
 impl RunHandle {
@@ -224,8 +243,8 @@ impl Shared {
         self.driver_canceller.reset();
     }
 
-    /// Clears the driver flag before a schema read, outside of any run.
-    fn begin_schema(&self) {
+    /// Clears the driver flag before a schema read or a plan, outside of any run.
+    fn begin_read(&self) {
         let _current = self.current();
         self.driver_canceller.reset();
     }
@@ -254,9 +273,13 @@ fn work(mut driver: Box<dyn Driver>, inbox: &mpsc::Receiver<Job>, shared: &Share
                 active.execute(driver.as_mut());
             }
             Job::Schema(reply) => {
-                shared.begin_schema();
+                shared.begin_read();
                 let _ = reply.send_blocking(driver.schema());
             }
+            Job::Plan { sql, reply } => {
+                shared.begin_read();
+                let _ = reply.send_blocking(driver.plan(&sql));
+            }
         }
     }
 }
```

Create `crates/engine/src/sqlite/explain.rs`:
```rust
//! Reading a statement's plan with `EXPLAIN QUERY PLAN`.

use rusqlite::Connection;

use super::convert;
use crate::{
    error::EngineError,
    plan::{Plan, PlanStep},
};

/// Plans the first statement in `sql`.
pub(super) fn plan(connection: &Connection, sql: &str) -> Result<Plan, EngineError> {
    let mut statement = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .map_err(convert::error)?;
    let steps = statement
        .query_map([], |row| {
            Ok(PlanStep {
                id: row.get(0)?,
                parent: row.get(1)?,
                label: row.get(3)?,
            })
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(convert::error)?;
    Ok(Plan::from_steps(&steps))
}
```

Change `crates/engine/src/sqlite/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/engine/src/sqlite/mod.rs b/crates/engine/src/sqlite/mod.rs
--- a/crates/engine/src/sqlite/mod.rs
+++ b/crates/engine/src/sqlite/mod.rs
@@ -1,6 +1,7 @@
 //! The built-in SQLite driver (bundled `SQLite`, no system library needed).
 
 mod convert;
+mod explain;
 mod introspect;
 mod run;
 
@@ -11,6 +12,7 @@ use rusqlite::Connection;
 use crate::{
     driver::{Canceller, Driver},
     error::EngineError,
+    plan::Plan,
     result::{RowSink, Summary},
     schema::Schema,
 };
@@ -81,6 +83,10 @@ impl Driver for SqliteDriver {
         introspect::schema(&self.connection)
     }
 
+    fn plan(&mut self, sql: &str) -> Result<Plan, EngineError> {
+        explain::plan(&self.connection, sql)
+    }
+
     fn canceller(&self) -> Canceller {
         self.canceller.clone()
     }
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry-engine`
Expected: all engine tests pass, including the 6 in `tests/plan.rs` and 4 unit tests in `plan.rs`.

Run: `just check`
Expected: exit 0.

Run: `cargo mutants -p quarry-engine --in-diff <(git diff main)`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Step 5: Commit**

```bash
git add -A crates/engine/src/driver.rs crates/engine/src/error.rs crates/engine/src/lib.rs crates/engine/src/plan.rs crates/engine/src/session.rs crates/engine/src/sqlite/explain.rs crates/engine/src/sqlite/mod.rs crates/engine/tests/plan.rs
git commit -m "feat(engine): plan statements through the driver and the session"
```


---

### Task 3: Model: the tab list

The pure rules for query tabs: their order, "Query N" titles (one more than the highest open), which tab is active, and what opening, closing and switching do. Tests live in the file's `#[cfg(test)]` module: write it first, run, and watch it fail to compile.

**Files:**
- Modify: `crates/app/src/model/mod.rs`
- Create: `crates/app/src/model/tabs.rs`

**Interfaces:**
- Produces: `model::tabs::{TabId = i64, Tab { id, title }, TabList}` with `TabList::{restore(Vec<Tab>, usize), tabs, active, active_id, open() -> TabId, close(TabId) -> Option<TabId>, next, previous, activate(TabId) -> bool}`. `close` returns the id of a fresh tab when it closed the only one.

- [ ] **Step 1: Implement**

Change `crates/app/src/model/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/model/mod.rs b/crates/app/src/model/mod.rs
--- a/crates/app/src/model/mod.rs
+++ b/crates/app/src/model/mod.rs
@@ -8,4 +8,5 @@ pub mod format;
 pub mod run;
 pub mod schema_tree;
 pub mod status;
+pub mod tabs;
 pub mod target;
```

Create `crates/app/src/model/tabs.rs`:
```rust
//! The query tabs: their order, titles, and which one is active.

/// A tab's identity; it never changes and is never reused while open.
pub type TabId = i64;

/// The prefix of every generated title, as in "Query 3".
const TITLE_PREFIX: &str = "Query ";

/// One tab's identity and title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    /// The tab's identity.
    pub id: TabId,
    /// The title shown on the tab.
    pub title: String,
}

/// The open tabs, left to right, with one always active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabList {
    tabs: Vec<Tab>,
    active: usize,
    next_id: TabId,
}

impl TabList {
    /// Restores saved tabs; with none, opens "Query 1". `active` is clamped.
    pub fn restore(tabs: Vec<Tab>, active: usize) -> Self {
        let next_id = tabs.iter().map(|tab| tab.id).max().unwrap_or(0) + 1;
        let mut list = Self {
            active: active.min(tabs.len().saturating_sub(1)),
            tabs,
            next_id,
        };
        if list.tabs.is_empty() {
            list.open();
        }
        list
    }

    /// The tabs, left to right.
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// The active tab's index.
    pub fn active(&self) -> usize {
        self.active
    }

    /// The active tab's identity.
    pub fn active_id(&self) -> TabId {
        self.tabs[self.active].id
    }

    /// Opens a new "Query N" right after the active tab and activates it.
    pub fn open(&mut self) -> TabId {
        let id = self.next_id;
        self.next_id += 1;
        let title = format!("{TITLE_PREFIX}{}", self.next_number());
        let at = if self.tabs.is_empty() {
            0
        } else {
            self.active + 1
        };
        self.tabs.insert(at, Tab { id, title });
        self.active = at;
        id
    }

    /// Closes tab `id` and activates its right neighbour, or its left one if
    /// it was last. Closing the only tab opens a fresh one, whose id is
    /// returned. Returns `None` when `id` is not open or no tab was opened.
    pub fn close(&mut self, id: TabId) -> Option<TabId> {
        let index = self.index_of(id)?;
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            return Some(self.open());
        }
        if index < self.active || self.active == self.tabs.len() {
            self.active -= 1;
        }
        None
    }

    /// Activates the next tab, wrapping to the first.
    pub fn next(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
    }

    /// Activates the previous tab, wrapping to the last.
    pub fn previous(&mut self) {
        self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
    }

    /// Activates tab `id`; returns whether it is open.
    pub fn activate(&mut self, id: TabId) -> bool {
        self.index_of(id).map(|index| self.active = index).is_some()
    }

    fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    /// One more than the highest "Query N" in use, so titles never repeat
    /// among open tabs.
    fn next_number(&self) -> u64 {
        self.tabs
            .iter()
            .filter_map(|tab| tab.title.strip_prefix(TITLE_PREFIX)?.parse::<u64>().ok())
            .max()
            .unwrap_or(0)
            + 1
    }
}

#[cfg(test)]
mod tests {
    use super::{Tab, TabList};

    fn tab(id: i64, title: &str) -> Tab {
        Tab {
            id,
            title: title.to_owned(),
        }
    }

    fn titles(list: &TabList) -> Vec<&str> {
        list.tabs().iter().map(|tab| tab.title.as_str()).collect()
    }

    fn three() -> TabList {
        TabList::restore(
            vec![tab(4, "Query 1"), tab(9, "Query 2"), tab(2, "Notes")],
            1,
        )
    }

    #[test]
    fn restoring_nothing_opens_query_one() {
        let list = TabList::restore(Vec::new(), 3);
        assert_eq!(titles(&list), ["Query 1"]);
        assert_eq!(list.active(), 0);
        assert_eq!(list.active_id(), 1);
    }

    #[test]
    fn restoring_keeps_order_and_clamps_the_active_tab() {
        let list = TabList::restore(vec![tab(4, "Query 1"), tab(9, "Query 2")], 7);
        assert_eq!(titles(&list), ["Query 1", "Query 2"]);
        assert_eq!(list.active(), 1);
        assert_eq!(three().active_id(), 9);
    }

    #[test]
    fn open_inserts_after_the_active_tab_with_the_next_number_and_a_new_id() {
        let mut list = three();
        let id = list.open();
        assert_eq!(id, 10);
        assert_eq!(titles(&list), ["Query 1", "Query 2", "Query 3", "Notes"]);
        assert_eq!(list.active(), 2);
        assert_eq!(list.open(), 11);
        assert_eq!(titles(&list)[3], "Query 4");
    }

    #[test]
    fn numbers_ignore_titles_that_are_not_query_numbers() {
        let mut list = TabList::restore(vec![tab(1, "Query x"), tab(2, "Query 7b")], 0);
        list.open();
        assert_eq!(titles(&list)[1], "Query 1");
    }

    #[test]
    fn closing_the_active_tab_activates_its_right_neighbour() {
        let mut list = three();
        assert_eq!(list.close(9), None);
        assert_eq!(titles(&list), ["Query 1", "Notes"]);
        assert_eq!(list.active_id(), 2);
    }

    #[test]
    fn closing_the_last_active_tab_activates_its_left_neighbour() {
        let mut list = three();
        list.next();
        list.close(2);
        assert_eq!(list.active_id(), 9);
        assert_eq!(list.active(), 1);
    }

    #[test]
    fn closing_a_tab_left_of_the_active_one_keeps_the_active_tab() {
        let mut list = three();
        list.close(4);
        assert_eq!(list.active_id(), 9);
        assert_eq!(list.active(), 0);
    }

    #[test]
    fn closing_a_tab_right_of_the_active_one_keeps_the_active_tab() {
        let mut list = three();
        list.close(2);
        assert_eq!(list.active_id(), 9);
        assert_eq!(list.active(), 1);
    }

    #[test]
    fn closing_the_only_tab_opens_a_fresh_one() {
        let mut list = TabList::restore(vec![tab(5, "Query 3")], 0);
        assert_eq!(list.close(5), Some(6));
        assert_eq!(titles(&list), ["Query 1"]);
        assert_eq!(list.active_id(), 6);
    }

    #[test]
    fn closing_an_unknown_tab_changes_nothing() {
        let mut list = three();
        assert_eq!(list.close(99), None);
        assert_eq!(list, three());
    }

    #[test]
    fn next_and_previous_wrap() {
        let mut list = three();
        list.next();
        assert_eq!(list.active(), 2);
        list.next();
        assert_eq!(list.active(), 0);
        list.previous();
        assert_eq!(list.active(), 2);
        list.previous();
        assert_eq!(list.active(), 1);
    }

    #[test]
    fn activate_selects_an_open_tab_only() {
        let mut list = three();
        assert!(list.activate(2));
        assert_eq!(list.active(), 2);
        assert!(!list.activate(99));
        assert_eq!(list.active(), 2);
    }
}
```


- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry --lib model::tabs`
Expected: 12 unit tests pass.

Run: `just check`
Expected: exit 0.

Run: `cargo mutants -p quarry --in-diff <(git diff main)`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Step 3: Commit**

```bash
git add -A crates/app/src/model/mod.rs crates/app/src/model/tabs.rs
git commit -m "feat(app): add the tab list model"
```


---

### Task 4: Model: per-statement output, messages and plan rows

`RunOutput` replaces 1C's `RunModel`: it keeps every statement's columns, rows (100,000 at most), and summary or error, plus a Waiting phase for a run queued behind another. `messages` gives one line per statement; `plan_tree` flattens a `Plan`. The status bar times the whole run, from the first statement's start until `Done`, so queued time never counts (M7); `failure` returns a `Failure` struct (M14); `cell_text` truncates before copying and maps every control character to a space (M4); `RunModel` and its unused `Clone` go (M6). A worker that vanishes mid-run marks its unfinished statement `WorkerGone`, and a finished run's time is never less than its statements' own engine times, since polled timestamps are only as precise as the poll. The grid and status bar switch to `RunOutput` in the same task so the tree stays green.

**Files:**
- Modify: `crates/app/src/model/drain.rs`
- Modify: `crates/app/src/model/format.rs`
- Create: `crates/app/src/model/messages.rs`
- Modify: `crates/app/src/model/mod.rs`
- Create: `crates/app/src/model/plan_tree.rs`
- Delete: `crates/app/src/model/run.rs`
- Create: `crates/app/src/model/statements.rs`
- Modify: `crates/app/src/model/status.rs`
- Modify: `crates/app/src/views/results.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/src/views/workspace/runner.rs`
- Modify: `crates/app/tests/workspace.rs`

**Interfaces:**
- Consumes: `quarry_engine::{session::RunEvent, result::Summary, plan::Plan}` (Task 2).
- Produces: `model::statements::{RunOutput, StatementOutput, Phase { Idle, Waiting, Running, Finished, Failed(EngineError) }, ROW_LIMIT, LABEL_CHARS, label}` with `RunOutput::{submitted(script), apply(RunEvent, Instant), end_closed, phase, statements, statement(usize), is_running, elapsed(Instant)}` and `StatementOutput::{index, label, columns, rows, received, truncated, returns_rows, outcome}`; `model::status::{Failure { message, cancelled }, run_summary(&RunOutput, Duration), failure(&RunOutput), outcome_text(&StatementOutput, &Summary)}`; `model::messages::{MessageLine { label, text, is_error }, lines(&RunOutput)}`; `model::plan_tree::{PlanRow { depth, label }, rows(&Plan)}`; `ResultsDelegate::{output, shown, reset(RunOutput), apply(RunEvent, Instant) -> bool}`; `Workspace::run_output`.

- [ ] **Step 1: Write the failing tests**

Change `crates/app/tests/workspace.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/workspace.rs b/crates/app/tests/workspace.rs
--- a/crates/app/tests/workspace.rs
+++ b/crates/app/tests/workspace.rs
@@ -8,7 +8,7 @@ use gpui_kit::{
     test::{TestEventExt, TestWindowExt},
 };
 use quarry::{
-    model::run::Phase,
+    model::statements::Phase,
     sample,
     shell::actions,
     views::{
@@ -71,13 +71,13 @@ fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bo
 fn run(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
     cx.simulate_keystrokes("cmd-enter");
     settle(cx, |_, cx| {
-        let model = workspace.read(cx).run_model(cx);
-        !model.is_running() && model.phase() != &Phase::Idle
+        let output = workspace.read(cx).run_output(cx);
+        !output.is_running() && output.phase() != &Phase::Idle
     });
 }
 
 fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
-    cx.update(|_, cx| workspace.read(cx).run_model(cx).phase().clone())
+    cx.update(|_, cx| workspace.read(cx).run_output(cx).phase().clone())
 }
 
 #[gpui_kit::test]
@@ -106,11 +106,11 @@ fn command_return_in_the_editor_runs_the_statement(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx);
     write(&workspace, cx, "SELECT name FROM astronauts ORDER BY id");
     run(&workspace, cx);
-    assert!(matches!(phase(&workspace, cx), Phase::Finished(_)));
+    assert_eq!(phase(&workspace, cx), Phase::Finished);
     cx.update(|_, cx| {
-        let model = workspace.read(cx).run_model(cx);
-        assert_eq!(model.rows().len(), 12);
-        assert_eq!(model.columns()[0].name, "name");
+        let statement = &workspace.read(cx).run_output(cx).statements()[0];
+        assert_eq!(statement.rows().len(), 12);
+        assert_eq!(statement.columns()[0].name, "name");
     });
 }
 
@@ -149,9 +149,9 @@ fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
         "SELECT count(*) FROM telemetry a, telemetry b",
     );
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, cx| workspace.read(cx).run_model(cx).is_running());
+    settle(cx, |_, cx| workspace.read(cx).run_output(cx).is_running());
     cx.simulate_keystrokes("cmd-.");
-    settle(cx, |_, cx| !workspace.read(cx).run_model(cx).is_running());
+    settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
     assert_eq!(phase(&workspace, cx), Phase::Failed(EngineError::Cancelled));
 }
 
@@ -164,11 +164,14 @@ fn running_again_during_a_run_replaces_it(cx: &mut TestAppContext) {
         "SELECT count(*) FROM telemetry a, telemetry b",
     );
     cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, cx| workspace.read(cx).run_model(cx).is_running());
+    settle(cx, |_, cx| workspace.read(cx).run_output(cx).is_running());
     write(&workspace, cx, "SELECT 1");
     run(&workspace, cx);
-    assert!(matches!(phase(&workspace, cx), Phase::Finished(_)));
-    cx.update(|_, cx| assert_eq!(workspace.read(cx).run_model(cx).rows().len(), 1));
+    assert_eq!(phase(&workspace, cx), Phase::Finished);
+    cx.update(|_, cx| {
+        let output = workspace.read(cx).run_output(cx);
+        assert_eq!(output.statements()[0].rows().len(), 1);
+    });
 }
 
 #[gpui_kit::test]
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry`
Expected: (unit tests beside the code; write them first and see `cannot find type RunOutput`).

- [ ] **Step 3: Implement**

Change `crates/app/src/model/drain.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/model/drain.rs b/crates/app/src/model/drain.rs
--- a/crates/app/src/model/drain.rs
+++ b/crates/app/src/model/drain.rs
@@ -1,8 +1,9 @@
 //! Takes everything waiting on a channel without blocking or awaiting.
 //!
 //! The UI polls the session's channels every `POLL` (33 ms) instead of
-//! awaiting them: a waker fired from the session thread would wake GPUI's executor
-//! from outside, and polling also batches many row events into one redraw.
+//! awaiting them: a waker fired from the session thread would wake GPUI's
+//! executor from outside, and polling also batches many row events into one
+//! redraw.
 
 use async_channel::{Receiver, TryRecvError};
 
```

Change `crates/app/src/model/format.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/model/format.rs b/crates/app/src/model/format.rs
--- a/crates/app/src/model/format.rs
+++ b/crates/app/src/model/format.rs
@@ -7,15 +7,28 @@ use quarry_engine::value::Value;
 /// The longest text a grid cell shows before it is cut with `…`.
 pub const CELL_LIMIT: usize = 200;
 
-/// A cell's text: one line, at most [`CELL_LIMIT`] characters.
+/// A cell's text: one line, at most [`CELL_LIMIT`] characters. Long text
+/// is cut before anything is copied, and control characters such as line
+/// breaks and tabs read as spaces.
 pub fn cell_text(value: &Value) -> String {
-    let text = value.to_string().replace(['\n', '\r'], " ");
-    match text.char_indices().nth(CELL_LIMIT) {
-        Some((cut, _)) => format!("{}…", &text[..cut]),
-        None => text,
+    match value {
+        Value::Text(text) => one_line(text),
+        other => one_line(&other.to_string()),
     }
 }
 
+fn one_line(text: &str) -> String {
+    let mut line: String = text
+        .chars()
+        .take(CELL_LIMIT)
+        .map(|c| if c.is_control() { ' ' } else { c })
+        .collect();
+    if text.chars().nth(CELL_LIMIT).is_some() {
+        line.push('…');
+    }
+    line
+}
+
 /// Numbers line up on the right, by digit; everything else on the left.
 pub fn aligns_right(value: &Value) -> bool {
     matches!(value, Value::Int(_) | Value::Float(_))
@@ -81,6 +94,8 @@ mod tests {
     fn cells_are_one_line() {
         let value = Value::Text("two\nlines\r\n".to_owned());
         assert_eq!(cell_text(&value), "two lines  ");
+        let tabs = Value::Text("a\tb\u{7}c".to_owned());
+        assert_eq!(cell_text(&tabs), "a b c");
     }
 
     #[test]
```

Create `crates/app/src/model/messages.rs`:
```rust
//! The Messages view: one line per statement of a run.

use quarry_engine::error::EngineError;

use super::{statements::RunOutput, status};

/// One statement's line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MessageLine {
    /// The statement's label, as in "2 · INSERT INTO crew…".
    pub label: String,
    /// What happened: rows, rows affected, "Done", the error, or "Cancelled".
    pub text: String,
    /// Whether the line reports an error (shown in the danger colour).
    pub is_error: bool,
}

/// A line for every statement that started, in order.
pub fn lines(output: &RunOutput) -> Vec<MessageLine> {
    output
        .statements()
        .iter()
        .map(|statement| {
            let (text, is_error) = match statement.outcome() {
                None => ("Running…".to_owned(), false),
                Some(Ok(summary)) => (status::outcome_text(statement, summary), false),
                Some(Err(EngineError::Cancelled)) => ("Cancelled".to_owned(), false),
                Some(Err(error)) => (error.to_string(), true),
            };
            MessageLine {
                label: statement.label().to_owned(),
                text,
                is_error,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use quarry_engine::{error::EngineError, result::Summary, session::RunEvent};

    use super::{MessageLine, lines};
    use crate::model::statements::RunOutput;

    fn line(label: &str, text: &str, is_error: bool) -> MessageLine {
        MessageLine {
            label: label.to_owned(),
            text: text.to_owned(),
            is_error,
        }
    }

    #[test]
    fn every_started_statement_gets_a_line() {
        let script = "INSERT INTO t VALUES (1);SELECT 1;SELECT x;SELECT 2";
        let mut output = RunOutput::submitted(script);
        let events = vec![
            RunEvent::Started {
                index: 0,
                range: 0..24,
            },
            RunEvent::Finished(Summary {
                rows: 0,
                rows_affected: Some(2),
                elapsed: Duration::from_millis(1),
            }),
            RunEvent::Started {
                index: 1,
                range: 25..33,
            },
            RunEvent::Failed(EngineError::Sql("no such column: x".to_owned())),
        ];
        for event in events {
            output.apply(event, Instant::now());
        }
        assert_eq!(
            lines(&output),
            [
                line(
                    "1 · INSERT INTO t VALUES (1)",
                    "2 rows affected · 1.0 ms",
                    false
                ),
                line("2 · SELECT 1", "no such column: x", true),
            ]
        );
    }

    #[test]
    fn running_and_cancelled_statements_are_not_errors() {
        let mut output = RunOutput::submitted("SELECT 1;SELECT 2");
        output.apply(
            RunEvent::Started {
                index: 0,
                range: 0..8,
            },
            Instant::now(),
        );
        assert_eq!(lines(&output), [line("1 · SELECT 1", "Running…", false)]);
        output.apply(RunEvent::Failed(EngineError::Cancelled), Instant::now());
        assert_eq!(lines(&output), [line("1 · SELECT 1", "Cancelled", false)]);
    }

    #[test]
    fn a_vanished_workers_statement_is_an_error_line() {
        let mut output = RunOutput::submitted("SELECT 1");
        let started = RunEvent::Started {
            index: 0,
            range: 0..8,
        };
        output.apply(started, Instant::now());
        output.end_closed();
        assert_eq!(
            lines(&output),
            [line(
                "1 · SELECT 1",
                &EngineError::WorkerGone.to_string(),
                true
            )]
        );
    }
}
```

Change `crates/app/src/model/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/model/mod.rs b/crates/app/src/model/mod.rs
--- a/crates/app/src/model/mod.rs
+++ b/crates/app/src/model/mod.rs
@@ -5,8 +5,10 @@
 
 pub mod drain;
 pub mod format;
-pub mod run;
+pub mod messages;
+pub mod plan_tree;
 pub mod schema_tree;
+pub mod statements;
 pub mod status;
 pub mod tabs;
 pub mod target;
```

Create `crates/app/src/model/plan_tree.rs`:
```rust
//! The Plan view's rows: a query plan flattened with each step's depth.

use quarry_engine::plan::{Plan, PlanNode};

/// One step of a plan, indented by `depth`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    /// How deeply the step is nested; top-level steps are 0.
    pub depth: usize,
    /// The database's description of the step.
    pub label: String,
}

/// Every step, depth first, in the database's order.
pub fn rows(plan: &Plan) -> Vec<PlanRow> {
    let mut rows = Vec::new();
    push(&plan.nodes, 0, &mut rows);
    rows
}

fn push(nodes: &[PlanNode], depth: usize, rows: &mut Vec<PlanRow>) {
    for node in nodes {
        rows.push(PlanRow {
            depth,
            label: node.label.clone(),
        });
        push(&node.children, depth + 1, rows);
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::plan::{Plan, PlanNode};

    use super::{PlanRow, rows};

    fn node(label: &str, children: Vec<PlanNode>) -> PlanNode {
        PlanNode {
            label: label.to_owned(),
            children,
        }
    }

    fn row(depth: usize, label: &str) -> PlanRow {
        PlanRow {
            depth,
            label: label.to_owned(),
        }
    }

    #[test]
    fn flattens_depth_first_keeping_order() {
        let plan = Plan {
            nodes: vec![
                node(
                    "SCAN ships",
                    vec![node("SUBQUERY", vec![node("SCAN crew", vec![])])],
                ),
                node("USE TEMP B-TREE", vec![]),
            ],
        };
        assert_eq!(
            rows(&plan),
            [
                row(0, "SCAN ships"),
                row(1, "SUBQUERY"),
                row(2, "SCAN crew"),
                row(0, "USE TEMP B-TREE"),
            ]
        );
    }

    #[test]
    fn an_empty_plan_has_no_rows() {
        assert!(rows(&Plan::default()).is_empty());
    }
}
```

Delete `crates/app/src/model/run.rs` with `git rm crates/app/src/model/run.rs`.

Create `crates/app/src/model/statements.rs`:
```rust
//! One run's output, statement by statement, built from the engine's
//! [`RunEvent`]s.

use std::time::{Duration, Instant};

use quarry_engine::{
    error::EngineError,
    result::{Column, Row, Summary},
    session::RunEvent,
};

/// The most rows of one statement Quarry keeps; the rest are only counted.
pub const ROW_LIMIT: usize = 100_000;

/// The most characters of SQL a statement's label shows.
pub const LABEL_CHARS: usize = 24;

/// Where a run is in its life.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Nothing has run yet.
    Idle,
    /// Submitted, but queued behind another run.
    Waiting,
    /// Statements are running; rows may still arrive.
    Running,
    /// Every statement finished.
    Finished,
    /// A statement failed or the run was cancelled.
    Failed(EngineError),
}

/// What one statement of a run produced.
#[derive(Debug, PartialEq)]
pub struct StatementOutput {
    index: usize,
    label: String,
    columns: Vec<Column>,
    rows: Vec<Row>,
    received: u64,
    outcome: Option<Result<Summary, EngineError>>,
}

impl StatementOutput {
    fn new(index: usize, sql: &str) -> Self {
        Self {
            index,
            label: label(index, sql),
            columns: Vec::new(),
            rows: Vec::new(),
            received: 0,
            outcome: None,
        }
    }

    /// The statement's 0-based position in the script.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Its number and first words, as in "3 · SELECT name FROM…".
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Its columns (empty for a statement that returns no rows).
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The rows kept so far: at most [`ROW_LIMIT`].
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Every row the statement has streamed, kept or not.
    pub fn received(&self) -> u64 {
        self.received
    }

    /// Whether the statement streamed more rows than Quarry keeps.
    pub fn truncated(&self) -> bool {
        self.received > u64::try_from(self.rows.len()).unwrap_or(u64::MAX)
    }

    /// Whether the statement returns rows, so it gets a result tab.
    pub fn returns_rows(&self) -> bool {
        !self.columns.is_empty()
    }

    /// How it ended, once it has.
    pub fn outcome(&self) -> Option<&Result<Summary, EngineError>> {
        self.outcome.as_ref()
    }

    /// Keeps the batch's rows up to [`ROW_LIMIT`] and counts all of them.
    fn take(&mut self, batch: Vec<Row>) {
        let arrived = u64::try_from(batch.len()).unwrap_or(u64::MAX);
        self.received = self.received.saturating_add(arrived);
        let room = ROW_LIMIT.saturating_sub(self.rows.len());
        self.rows.extend(batch.into_iter().take(room));
    }
}

/// "3 · SELECT name FROM…": the 1-based number and the statement's first
/// [`LABEL_CHARS`] characters, with runs of whitespace made single spaces.
pub fn label(index: usize, sql: &str) -> String {
    let words = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut short: String = words.chars().take(LABEL_CHARS).collect();
    if words.chars().nth(LABEL_CHARS).is_some() {
        short.push('…');
    }
    format!("{} · {short}", index + 1)
}

/// One run: its phase and every statement that has started.
#[derive(Debug, PartialEq)]
pub struct RunOutput {
    script: String,
    phase: Phase,
    statements: Vec<StatementOutput>,
    started_at: Option<Instant>,
    ended_at: Option<Instant>,
}

impl Default for RunOutput {
    fn default() -> Self {
        Self {
            script: String::new(),
            phase: Phase::Idle,
            statements: Vec::new(),
            started_at: None,
            ended_at: None,
        }
    }
}

impl RunOutput {
    /// A run of `script` that is queued and has not started.
    pub fn submitted(script: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            phase: Phase::Waiting,
            ..Self::default()
        }
    }

    /// Folds one event in. `now` starts the clock at the first statement
    /// and stops it at `Done`.
    pub fn apply(&mut self, event: RunEvent, now: Instant) {
        match event {
            RunEvent::Started { index, range } => self.begin(index, &range, now),
            RunEvent::Columns(columns) => self.current(|statement| statement.columns = columns),
            RunEvent::Rows(batch) => self.current(|statement| statement.take(batch)),
            RunEvent::Finished(summary) => {
                self.current(|statement| statement.outcome = Some(Ok(summary)));
            }
            RunEvent::Failed(error) => self.fail(error),
            RunEvent::Done => self.finish(now),
        }
    }

    /// The channel closed without `Done`: the worker vanished.
    /// A failure already recorded is kept; anything else becomes `WorkerGone`.
    pub fn end_closed(&mut self) {
        if !matches!(self.phase, Phase::Failed(_)) {
            self.phase = Phase::Failed(EngineError::WorkerGone);
        }
        self.fail_unfinished();
    }

    /// Where the run is.
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// Every statement that has started, in order.
    pub fn statements(&self) -> &[StatementOutput] {
        &self.statements
    }

    /// The statement at `index`, if it has started.
    pub fn statement(&self, index: usize) -> Option<&StatementOutput> {
        self.statements.get(index)
    }

    /// Whether the run is queued or rows may still arrive.
    pub fn is_running(&self) -> bool {
        matches!(self.phase, Phase::Waiting | Phase::Running)
    }

    /// The whole run's time. While it goes on: from the first statement
    /// starting until `now`. Once it has ended: the larger of the wall time
    /// until `Done` and the sum of the statements' own times, because the
    /// timestamps the view passes are only as precise as its poll, and a
    /// fast run can otherwise read zero. The live reading never exceeds the
    /// final one, so the time never jumps backwards. Zero before any
    /// statement starts, so time spent queued never counts.
    pub fn elapsed(&self, now: Instant) -> Duration {
        let Some(start) = self.started_at else {
            return Duration::ZERO;
        };
        match self.ended_at {
            Some(end) => end.saturating_duration_since(start).max(self.engine_time()),
            None => now.saturating_duration_since(start),
        }
    }

    /// The sum of the times the engine reported for finished statements.
    fn engine_time(&self) -> Duration {
        self.statements
            .iter()
            .filter_map(|statement| statement.outcome().and_then(|o| o.as_ref().ok()))
            .map(|summary| summary.elapsed)
            .sum()
    }

    fn begin(&mut self, index: usize, range: &std::ops::Range<usize>, now: Instant) {
        self.phase = Phase::Running;
        self.started_at.get_or_insert(now);
        let sql = self.script.get(range.clone()).unwrap_or_default();
        self.statements.push(StatementOutput::new(index, sql));
    }

    fn current(&mut self, change: impl FnOnce(&mut StatementOutput)) {
        if let Some(statement) = self.statements.last_mut() {
            change(statement);
        }
    }

    /// The running statement, if any, ends with `error`, and so does the run.
    fn fail(&mut self, error: EngineError) {
        if let Some(statement) = self.statements.last_mut()
            && statement.outcome.is_none()
        {
            statement.outcome = Some(Err(error.clone()));
        }
        self.phase = Phase::Failed(error);
    }

    /// `Done` stops the clock; with a statement still unfinished it means
    /// the worker vanished.
    fn finish(&mut self, now: Instant) {
        self.ended_at = Some(now);
        if matches!(self.phase, Phase::Failed(_)) {
            return;
        }
        let unfinished = self.statements.iter().any(|s| s.outcome.is_none());
        self.phase = if unfinished {
            Phase::Failed(EngineError::WorkerGone)
        } else {
            Phase::Finished
        };
        self.fail_unfinished();
    }

    /// Every statement without an outcome was lost with the worker.
    fn fail_unfinished(&mut self) {
        for statement in &mut self.statements {
            statement
                .outcome
                .get_or_insert(Err(EngineError::WorkerGone));
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

    use super::{LABEL_CHARS, Phase, ROW_LIMIT, RunOutput, label};

    const SCRIPT: &str = "SELECT 1 AS n;\nCREATE TABLE t (x);";

    fn column(name: &str) -> Column {
        Column {
            name: name.to_owned(),
            type_name: String::new(),
        }
    }

    fn summary(millis: u64) -> Summary {
        Summary {
            rows: 1,
            rows_affected: None,
            elapsed: Duration::from_millis(millis),
        }
    }

    fn started(index: usize) -> RunEvent {
        let range = if index == 0 { 0..13 } else { 15..34 };
        RunEvent::Started { index, range }
    }

    fn rows(count: usize) -> RunEvent {
        RunEvent::Rows(vec![vec![Value::Int(1)]; count])
    }

    fn feed(output: &mut RunOutput, events: Vec<RunEvent>) {
        for event in events {
            output.apply(event, Instant::now());
        }
    }

    #[test]
    fn a_new_output_is_idle_and_empty() {
        let output = RunOutput::default();
        assert_eq!(output.phase(), &Phase::Idle);
        assert!(output.statements().is_empty());
        assert!(!output.is_running());
    }

    #[test]
    fn a_submitted_run_waits_until_its_first_statement_starts() {
        let mut output = RunOutput::submitted(SCRIPT);
        assert_eq!(output.phase(), &Phase::Waiting);
        assert!(output.is_running());
        feed(&mut output, vec![started(0)]);
        assert_eq!(output.phase(), &Phase::Running);
    }

    #[test]
    fn each_statement_keeps_its_own_columns_rows_and_outcome() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Columns(vec![column("n")]),
                rows(2),
                RunEvent::Finished(summary(2)),
                started(1),
                RunEvent::Finished(summary(3)),
                RunEvent::Done,
            ],
        );
        assert_eq!(output.phase(), &Phase::Finished);
        let [first, second] = output.statements() else {
            panic!("two statements");
        };
        assert_eq!(first.label(), "1 · SELECT 1 AS n");
        assert_eq!((first.index(), second.index()), (0, 1));
        assert_eq!(first.columns(), [column("n")]);
        assert_eq!(first.rows().len(), 2);
        assert!(first.returns_rows() && !second.returns_rows());
        assert_eq!(second.outcome(), Some(&Ok(summary(3))));
        assert_eq!(output.statement(1), Some(second));
        assert_eq!(output.statement(2), None);
    }

    #[test]
    fn a_statement_keeps_only_the_first_rows_but_counts_them_all() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(&mut output, vec![started(0), rows(ROW_LIMIT - 1)]);
        let statement = |output: &RunOutput| output.statements()[0].truncated();
        assert!(!statement(&output));
        feed(&mut output, vec![rows(1)]);
        assert!(!statement(&output), "exactly the limit is not truncated");
        feed(&mut output, vec![rows(3), rows(2)]);
        let first = &output.statements()[0];
        assert_eq!(first.rows().len(), ROW_LIMIT);
        assert_eq!(first.received(), 100_005);
        assert!(first.truncated());
    }

    #[test]
    fn a_failure_ends_the_running_statement_and_the_run() {
        let mut output = RunOutput::submitted(SCRIPT);
        let error = EngineError::Sql("boom".to_owned());
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Finished(summary(1)),
                started(1),
                RunEvent::Failed(error.clone()),
                RunEvent::Done,
            ],
        );
        assert_eq!(output.phase(), &Phase::Failed(error.clone()));
        assert_eq!(output.statements()[0].outcome(), Some(&Ok(summary(1))));
        assert_eq!(output.statements()[1].outcome(), Some(&Err(error)));
    }

    #[test]
    fn a_queued_run_can_fail_before_any_statement() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![RunEvent::Failed(EngineError::Cancelled), RunEvent::Done],
        );
        assert_eq!(output.phase(), &Phase::Failed(EngineError::Cancelled));
        assert!(output.statements().is_empty());
    }

    #[test]
    fn a_failure_after_a_finished_statement_leaves_its_outcome() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Finished(summary(1)),
                RunEvent::Failed(EngineError::Cancelled),
            ],
        );
        assert_eq!(output.statements()[0].outcome(), Some(&Ok(summary(1))));
        assert_eq!(output.phase(), &Phase::Failed(EngineError::Cancelled));
    }

    #[test]
    fn done_with_a_statement_unfinished_means_the_worker_is_gone() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(&mut output, vec![started(0), RunEvent::Done]);
        assert_eq!(output.phase(), &Phase::Failed(EngineError::WorkerGone));
    }

    #[test]
    fn done_with_no_statements_is_finished() {
        let mut output = RunOutput::submitted("-- nothing");
        feed(&mut output, vec![RunEvent::Done]);
        assert_eq!(output.phase(), &Phase::Finished);
    }

    #[test]
    fn a_channel_closed_after_finishing_means_the_worker_is_gone() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![started(0), RunEvent::Finished(summary(1))],
        );
        output.end_closed();
        assert_eq!(output.phase(), &Phase::Failed(EngineError::WorkerGone));
    }

    #[test]
    fn a_channel_closed_after_a_failure_keeps_the_real_error() {
        let mut output = RunOutput::submitted(SCRIPT);
        let error = EngineError::Sql("no such table".to_owned());
        feed(
            &mut output,
            vec![started(0), RunEvent::Failed(error.clone())],
        );
        output.end_closed();
        assert_eq!(output.phase(), &Phase::Failed(error));
    }

    #[test]
    fn elapsed_runs_from_the_first_start_until_done() {
        let start = Instant::now();
        let at = |millis| start + Duration::from_millis(millis);
        let mut output = RunOutput::submitted(SCRIPT);
        assert_eq!(output.elapsed(at(900)), Duration::ZERO, "queued time");
        output.apply(started(0), at(1000));
        output.apply(RunEvent::Finished(summary(2)), at(1200));
        output.apply(started(1), at(1300));
        assert_eq!(output.elapsed(at(1500)), Duration::from_millis(500));
        output.apply(RunEvent::Finished(summary(3)), at(1600));
        output.apply(RunEvent::Done, at(1700));
        assert_eq!(output.elapsed(at(9000)), Duration::from_millis(700));
    }

    #[test]
    fn a_vanished_worker_fails_the_statement_it_left_unfinished() {
        let mut done = RunOutput::submitted(SCRIPT);
        feed(&mut done, vec![started(0), RunEvent::Done]);
        let gone = Some(&Err(EngineError::WorkerGone));
        assert_eq!(done.statements()[0].outcome(), gone);
        let mut closed = RunOutput::submitted(SCRIPT);
        feed(&mut closed, vec![started(0)]);
        closed.end_closed();
        assert_eq!(closed.statements()[0].outcome(), gone);
    }

    #[test]
    fn a_closed_channel_keeps_earlier_outcomes() {
        let mut output = RunOutput::submitted(SCRIPT);
        let error = EngineError::Sql("boom".to_owned());
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Finished(summary(1)),
                started(1),
                RunEvent::Failed(error.clone()),
            ],
        );
        output.end_closed();
        assert_eq!(output.statements()[0].outcome(), Some(&Ok(summary(1))));
        assert_eq!(output.statements()[1].outcome(), Some(&Err(error)));
    }

    #[test]
    fn a_finished_run_is_never_shorter_than_its_statements_own_time() {
        let now = Instant::now();
        let mut output = RunOutput::submitted(SCRIPT);
        output.apply(started(0), now);
        output.apply(RunEvent::Finished(summary(2)), now);
        output.apply(started(1), now);
        output.apply(RunEvent::Finished(summary(3)), now);
        assert_eq!(output.elapsed(now), Duration::ZERO, "live reading");
        output.apply(RunEvent::Done, now);
        assert_eq!(output.elapsed(now), Duration::from_millis(5));
    }

    #[test]
    fn a_failed_run_stops_the_clock_at_done() {
        let start = Instant::now();
        let mut output = RunOutput::submitted(SCRIPT);
        output.apply(started(0), start);
        output.apply(RunEvent::Failed(EngineError::Cancelled), start);
        output.apply(RunEvent::Done, start + Duration::from_millis(40));
        let later = start + Duration::from_secs(60);
        assert_eq!(output.elapsed(later), Duration::from_millis(40));
    }

    #[test]
    fn labels_collapse_whitespace_and_cut_long_sql() {
        assert_eq!(label(0, "SELECT\n\t  1"), "1 · SELECT 1");
        let exact = "x".repeat(LABEL_CHARS);
        assert_eq!(label(4, &exact), format!("5 · {exact}"));
        let long = "é".repeat(LABEL_CHARS + 1);
        assert_eq!(label(1, &long), format!("2 · {}…", "é".repeat(LABEL_CHARS)));
    }

    #[test]
    fn a_range_outside_the_script_gives_an_empty_label() {
        let mut output = RunOutput::submitted("SELECT 1");
        output.apply(
            RunEvent::Started {
                index: 0,
                range: 3..99,
            },
            Instant::now(),
        );
        assert_eq!(output.statements()[0].label(), "1 · ");
    }
}
```

Change `crates/app/src/model/status.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/model/status.rs b/crates/app/src/model/status.rs
--- a/crates/app/src/model/status.rs
+++ b/crates/app/src/model/status.rs
@@ -1,4 +1,4 @@
-//! The run summary the status bar shows.
+//! The run summary the status bar shows, and the failure banner's message.
 
 use std::time::Duration;
 
@@ -6,127 +6,192 @@ use quarry_engine::{error::EngineError, result::Summary};
 
 use super::{
     format,
-    run::{Phase, RunModel},
+    statements::{Phase, RunOutput, StatementOutput},
 };
 
-/// What the status bar says about the latest run, given the live elapsed
-/// time for a run still going.
-pub fn run_summary(model: &RunModel, elapsed: Duration) -> String {
-    match model.phase() {
+/// The message above the results when a run failed.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Failure {
+    /// The database's own words, or Quarry's for a cancel or a lost worker.
+    pub message: String,
+    /// Whether it was only a cancel, shown quietly rather than as an error.
+    pub cancelled: bool,
+}
+
+/// What the status bar says about a run, given its elapsed time (see
+/// [`RunOutput::elapsed`]).
+pub fn run_summary(output: &RunOutput, elapsed: Duration) -> String {
+    match output.phase() {
         Phase::Idle => "Ready".to_owned(),
-        Phase::Running => running(model, elapsed),
-        Phase::Finished(summary) => finished(model, summary),
-        Phase::Failed(EngineError::Cancelled) => "Cancelled".to_owned(),
+        Phase::Waiting => "Waiting for the previous run…".to_owned(),
+        Phase::Running => running(output, elapsed),
+        Phase::Finished => finished(output, elapsed),
+        Phase::Failed(error) if is_cancel(error) => "Cancelled".to_owned(),
         Phase::Failed(_) => "Error".to_owned(),
     }
 }
 
-/// The message above the grid when a run failed: the database's own words,
-/// and whether it was only a cancel (shown quietly, not as an error).
-pub fn failure(model: &RunModel) -> Option<(String, bool)> {
-    match model.phase() {
-        Phase::Failed(error) => Some((error.to_string(), *error == EngineError::Cancelled)),
+/// The failure banner's message, for a failed run.
+pub fn failure(output: &RunOutput) -> Option<Failure> {
+    match output.phase() {
+        Phase::Failed(error) => Some(Failure {
+            message: error.to_string(),
+            cancelled: is_cancel(error),
+        }),
         _ => None,
     }
 }
 
-fn running(model: &RunModel, elapsed: Duration) -> String {
+/// How a finished statement reads: "4 rows · 3.0 ms",
+/// "2 rows affected · 1.0 ms" or "Done · 1.0 ms".
+pub fn outcome_text(statement: &StatementOutput, summary: &Summary) -> String {
+    let took = format::duration(summary.elapsed);
+    match summary.rows_affected {
+        Some(0) => format!("Done · {took}"),
+        Some(1) => format!("1 row affected · {took}"),
+        Some(n) => format!("{} rows affected · {took}", format::count(n)),
+        None => format!("{} · {took}", rows_text(statement, summary.rows)),
+    }
+}
+
+fn is_cancel(error: &EngineError) -> bool {
+    *error == EngineError::Cancelled
+}
+
+fn running(output: &RunOutput, elapsed: Duration) -> String {
+    let rows = output
+        .statements()
+        .last()
+        .map(|statement| rows_text(statement, statement.received()))
+        .unwrap_or_default();
+    format!("Running · {} · {rows}", format::duration(elapsed))
+}
+
+/// One statement reads as its outcome; several as a count and their total.
+fn finished(output: &RunOutput, elapsed: Duration) -> String {
+    if let [statement] = output.statements()
+        && let Some(Ok(summary)) = statement.outcome()
+    {
+        return outcome_text(statement, summary);
+    }
+    let count = u64::try_from(output.statements().len()).unwrap_or(u64::MAX);
     format!(
-        "Running · {} · {}",
-        format::duration(elapsed),
-        rows_text(model, model.received())
+        "{} statements · {}",
+        format::count(count),
+        format::duration(elapsed)
     )
 }
 
-/// "1,000 rows", or "100,000 of 250,000 rows" when the grid keeps fewer
+/// "1,000 rows", or "100,000 of 250,000 rows" when Quarry keeps fewer
 /// than the `total` the statement streamed.
-fn rows_text(model: &RunModel, total: u64) -> String {
-    if model.truncated() {
-        let kept = u64::try_from(model.rows().len()).unwrap_or(u64::MAX);
+fn rows_text(statement: &StatementOutput, total: u64) -> String {
+    if statement.truncated() {
+        let kept = u64::try_from(statement.rows().len()).unwrap_or(u64::MAX);
         format!("{} of {}", format::count(kept), format::rows(total))
     } else {
         format::rows(total)
     }
 }
 
-fn finished(model: &RunModel, summary: &Summary) -> String {
-    let took = format::duration(summary.elapsed);
-    match summary.rows_affected {
-        Some(0) => format!("Done · {took}"),
-        Some(1) => format!("1 row affected · {took}"),
-        Some(n) => format!("{} rows affected · {took}", format::count(n)),
-        None => format!("{} · {took}", rows_text(model, summary.rows)),
-    }
-}
-
 #[cfg(test)]
 mod tests {
     use std::time::{Duration, Instant};
 
     use quarry_engine::{error::EngineError, result::Summary, session::RunEvent, value::Value};
 
-    use super::{failure, run_summary};
-    use crate::model::run::RunModel;
+    use super::{Failure, failure, run_summary};
+    use crate::model::statements::RunOutput;
 
     const MS: Duration = Duration::from_millis(3);
 
-    fn finished(rows: u64, rows_affected: Option<u64>) -> String {
-        let mut model = RunModel::started(Instant::now());
-        model.apply(RunEvent::Finished(Summary {
+    fn started(index: usize) -> RunEvent {
+        RunEvent::Started { index, range: 0..1 }
+    }
+
+    fn summary(rows: u64, rows_affected: Option<u64>) -> RunEvent {
+        RunEvent::Finished(Summary {
             rows,
             rows_affected,
             elapsed: MS,
-        }));
-        run_summary(&model, Duration::ZERO)
+        })
+    }
+
+    fn output(events: Vec<RunEvent>) -> RunOutput {
+        let mut output = RunOutput::submitted("x");
+        for event in events {
+            output.apply(event, Instant::now());
+        }
+        output
+    }
+
+    fn finished(rows: u64, rows_affected: Option<u64>) -> String {
+        let output = output(vec![
+            started(0),
+            summary(rows, rows_affected),
+            RunEvent::Done,
+        ]);
+        run_summary(&output, output.elapsed(Instant::now()))
     }
 
     #[test]
     fn only_a_failed_run_has_a_failure_message() {
-        let mut model = RunModel::started(Instant::now());
-        assert_eq!(failure(&model), None);
-        model.apply(RunEvent::Failed(EngineError::Sql(
-            "no such table: t".to_owned(),
-        )));
+        let mut run = output(vec![started(0)]);
+        assert_eq!(failure(&run), None);
+        run.apply(
+            RunEvent::Failed(EngineError::Sql("no such table: t".to_owned())),
+            Instant::now(),
+        );
         assert_eq!(
-            failure(&model),
-            Some(("no such table: t".to_owned(), false))
+            failure(&run),
+            Some(Failure {
+                message: "no such table: t".to_owned(),
+                cancelled: false
+            })
+        );
+        let cancelled = output(vec![RunEvent::Failed(EngineError::Cancelled)]);
+        assert_eq!(
+            failure(&cancelled),
+            Some(Failure {
+                message: "Query cancelled.".to_owned(),
+                cancelled: true
+            })
         );
-        model.apply(RunEvent::Failed(EngineError::Cancelled));
-        assert_eq!(failure(&model), Some(("Query cancelled.".to_owned(), true)));
     }
 
     #[test]
-    fn idle_reads_ready() {
-        assert_eq!(run_summary(&RunModel::default(), MS), "Ready");
+    fn idle_reads_ready_and_queued_reads_waiting() {
+        assert_eq!(run_summary(&RunOutput::default(), MS), "Ready");
+        assert_eq!(
+            run_summary(&RunOutput::submitted("x"), MS),
+            "Waiting for the previous run…"
+        );
     }
 
     #[test]
     fn a_running_query_shows_the_live_timer_and_rows_so_far() {
-        let mut model = RunModel::started(Instant::now());
-        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]; 1000]));
+        let run = output(vec![
+            started(0),
+            RunEvent::Rows(vec![vec![Value::Int(1)]; 1000]),
+        ]);
         assert_eq!(
-            run_summary(&model, Duration::from_millis(1500)),
+            run_summary(&run, Duration::from_millis(1500)),
             "Running · 1.50 s · 1,000 rows"
         );
     }
 
     #[test]
-    fn a_truncated_run_says_how_many_rows_the_grid_keeps() {
-        let mut model = RunModel::started(Instant::now());
-        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]; 250_000]));
+    fn a_truncated_run_says_how_many_rows_quarry_keeps() {
+        let mut run = output(vec![
+            started(0),
+            RunEvent::Rows(vec![vec![Value::Int(1)]; 250_000]),
+        ]);
         assert_eq!(
-            run_summary(&model, Duration::from_millis(1500)),
+            run_summary(&run, Duration::from_millis(1500)),
             "Running · 1.50 s · 100,000 of 250,000 rows"
         );
-        model.apply(RunEvent::Finished(Summary {
-            rows: 250_000,
-            rows_affected: None,
-            elapsed: MS,
-        }));
-        assert_eq!(
-            run_summary(&model, Duration::ZERO),
-            "100,000 of 250,000 rows · 3.0 ms"
-        );
+        run.apply(summary(250_000, None), Instant::now());
+        run.apply(RunEvent::Done, Instant::now());
+        assert_eq!(run_summary(&run, MS), "100,000 of 250,000 rows · 3.0 ms");
     }
 
     #[test]
@@ -146,12 +211,42 @@ mod tests {
         assert_eq!(finished(0, Some(0)), "Done · 3.0 ms");
     }
 
+    #[test]
+    fn several_statements_read_as_a_count_and_the_run_time() {
+        let run = output(vec![
+            started(0),
+            summary(1, None),
+            started(1),
+            summary(0, Some(2)),
+            RunEvent::Done,
+        ]);
+        assert_eq!(
+            run_summary(&run, Duration::from_millis(6)),
+            "2 statements · 6.0 ms"
+        );
+    }
+
+    #[test]
+    fn a_script_with_no_statements_reads_done() {
+        let run = output(vec![RunEvent::Done]);
+        assert_eq!(run_summary(&run, Duration::ZERO), "0 statements · 0.0 ms");
+    }
+
+    #[test]
+    fn a_running_run_before_any_rows_reads_zero_rows() {
+        let run = output(vec![started(0)]);
+        assert_eq!(run_summary(&run, MS), "Running · 3.0 ms · 0 rows");
+    }
+
     #[test]
     fn failures_say_whether_they_were_cancelled() {
-        let mut model = RunModel::started(Instant::now());
-        model.apply(RunEvent::Failed(EngineError::Cancelled));
-        assert_eq!(run_summary(&model, MS), "Cancelled");
-        model.apply(RunEvent::Failed(EngineError::Sql("boom".to_owned())));
-        assert_eq!(run_summary(&model, MS), "Error");
+        let mut run = output(vec![started(0)]);
+        run.apply(RunEvent::Failed(EngineError::Cancelled), Instant::now());
+        assert_eq!(run_summary(&run, MS), "Cancelled");
+        run.apply(
+            RunEvent::Failed(EngineError::Sql("boom".to_owned())),
+            Instant::now(),
+        );
+        assert_eq!(run_summary(&run, MS), "Error");
     }
 }
```

Change `crates/app/src/views/results.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/results.rs b/crates/app/src/views/results.rs
--- a/crates/app/src/views/results.rs
+++ b/crates/app/src/views/results.rs
@@ -1,4 +1,4 @@
-//! The results grid: the latest run's rows in a virtualized table.
+//! The results grid: one statement's rows in a virtualized table.
 
 use gpui_kit::{
     AnyElement, App, Context, IntoElement, ParentElement, SharedString, Styled, Window,
@@ -7,7 +7,9 @@ use gpui_kit::{
     prelude::FluentBuilder,
     px,
 };
-use quarry_engine::{session::RunEvent, value::Value};
+use std::time::Instant;
+
+use quarry_engine::{result::Row, session::RunEvent, value::Value};
 use quarry_ui_kit::{
     Aurora, Role,
     tokens::{MONO_FONT, text},
@@ -15,48 +17,73 @@ use quarry_ui_kit::{
 
 use crate::model::{
     format,
-    run::{Phase, RunModel},
+    statements::{Phase, RunOutput, StatementOutput},
 };
 
 /// Default width of a result column.
 pub const COLUMN_WIDTH: f32 = 160.0;
 
-/// Feeds the latest [`RunModel`] to the table.
+/// Feeds one statement of a [`RunOutput`] to the table.
 #[derive(Default)]
 pub struct ResultsDelegate {
-    model: RunModel,
+    output: RunOutput,
+    shown: usize,
     columns: Vec<Column>,
 }
 
 impl ResultsDelegate {
     /// The run being shown.
-    pub fn model(&self) -> &RunModel {
-        &self.model
+    pub fn output(&self) -> &RunOutput {
+        &self.output
+    }
+
+    /// The index of the statement the grid shows.
+    pub fn shown(&self) -> usize {
+        self.shown
     }
 
-    /// Shows `model` from scratch, e.g. when a new run starts.
-    pub fn reset(&mut self, model: RunModel) {
-        self.model = model;
+    /// Shows `output` from scratch, e.g. when a new run starts.
+    pub fn reset(&mut self, output: RunOutput) {
+        self.output = output;
+        self.shown = 0;
         self.columns.clear();
     }
 
-    /// The channel closed without `Done`; see [`RunModel::end_closed`].
+    /// The channel closed without `Done`; see [`RunOutput::end_closed`].
     pub fn end_closed(&mut self) {
-        self.model.end_closed();
+        self.output.end_closed();
     }
 
-    /// Folds `event` in; returns whether the table's columns changed.
-    pub fn apply(&mut self, event: RunEvent) -> bool {
+    /// Folds `event` in, showing each statement as it starts; returns
+    /// whether the table's columns changed.
+    pub fn apply(&mut self, event: RunEvent, now: Instant) -> bool {
+        if let RunEvent::Started { index, .. } = &event {
+            self.shown = *index;
+        }
         let reshapes = matches!(event, RunEvent::Started { .. } | RunEvent::Columns(_));
-        self.model.apply(event);
+        self.output.apply(event, now);
         if reshapes {
-            self.columns = self.model.columns().iter().map(table_column).collect();
+            self.columns = self.shown_columns();
         }
         reshapes
     }
 
+    fn shown_columns(&self) -> Vec<Column> {
+        self.output
+            .statement(self.shown)
+            .map(|statement| statement.columns().iter().map(table_column).collect())
+            .unwrap_or_default()
+    }
+
+    fn rows(&self) -> &[Row] {
+        self.output
+            .statement(self.shown)
+            .map(StatementOutput::rows)
+            .unwrap_or_default()
+    }
+
     fn value(&self, row: usize, col: usize) -> Option<&Value> {
-        self.model.rows().get(row)?.get(col)
+        self.rows().get(row)?.get(col)
     }
 }
 
@@ -70,7 +97,7 @@ impl TableDelegate for ResultsDelegate {
     }
 
     fn rows_count(&self, _: &App) -> usize {
-        self.model.rows().len()
+        self.rows().len()
     }
 
     fn column(&self, col_ix: usize, _: &App) -> Column {
@@ -87,8 +114,7 @@ impl TableDelegate for ResultsDelegate {
         _: &mut Window,
         cx: &mut Context<TableState<Self>>,
     ) -> impl IntoElement {
-        let value = self.value(row_ix, col_ix).cloned().unwrap_or(Value::Null);
-        cell(&value, cx)
+        cell(self.value(row_ix, col_ix).unwrap_or(&Value::Null), cx)
     }
 
     fn cell_text(&self, row_ix: usize, col_ix: usize, _: &App) -> String {
@@ -102,7 +128,7 @@ impl TableDelegate for ResultsDelegate {
         _: &mut Window,
         cx: &mut Context<TableState<Self>>,
     ) -> impl IntoElement {
-        empty_state(self.model.phase(), cx)
+        empty_state(self.output.phase(), cx)
     }
 }
 
@@ -126,8 +152,9 @@ fn empty_state(phase: &Phase, cx: &App) -> AnyElement {
     let colors = Aurora::colors(cx);
     let (message, role) = match phase {
         Phase::Idle => ("Run a statement with ⌘↵.".to_owned(), Role::TextMuted),
+        Phase::Waiting => ("Waiting for the previous run…".to_owned(), Role::TextMuted),
         Phase::Running => ("Running…".to_owned(), Role::TextMuted),
-        Phase::Finished(_) | Phase::Failed(_) => ("No rows.".to_owned(), Role::TextMuted),
+        Phase::Finished | Phase::Failed(_) => ("No rows.".to_owned(), Role::TextMuted),
     };
     div()
         .size_full()
```

Change `crates/app/src/views/workspace/layout.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/layout.rs b/crates/app/src/views/workspace/layout.rs
--- a/crates/app/src/views/workspace/layout.rs
+++ b/crates/app/src/views/workspace/layout.rs
@@ -25,7 +25,10 @@ use quarry_ui_kit::{
 
 use super::{SIDEBAR_WIDTH, Workspace};
 use crate::{
-    model::{format, status},
+    model::{
+        format,
+        status::{self, Failure},
+    },
     shell::actions::WORKSPACE,
 };
 
@@ -108,7 +111,7 @@ impl Workspace {
 
     fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
         let colors = Aurora::colors(cx);
-        let failure = status::failure(self.run_model(cx));
+        let failure = status::failure(self.run_output(cx));
         glass_panel(&colors)
             .id("results")
             .size_full()
@@ -116,7 +119,7 @@ impl Workspace {
             .flex()
             .flex_col()
             .overflow_hidden()
-            .children(failure.map(|(message, cancelled)| failure_banner(message, cancelled, cx)))
+            .children(failure.map(|failure| failure_banner(failure, cx)))
             .child(
                 div()
                     .flex_1()
@@ -126,8 +129,8 @@ impl Workspace {
     }
 
     fn status_bar(&self, cx: &Context<Self>) -> AnyElement {
-        let model = self.run_model(cx);
-        let summary = status::run_summary(model, model.elapsed(Instant::now()));
+        let output = self.run_output(cx);
+        let summary = status::run_summary(output, output.elapsed(Instant::now()));
         let cursor = self.editor.read(cx).cursor_position();
         let place = format::position(cursor.line, cursor.character);
         div()
@@ -142,9 +145,9 @@ impl Workspace {
 }
 
 /// The run's error above the grid; a cancel reads quietly.
-fn failure_banner(message: String, cancelled: bool, cx: &App) -> AnyElement {
+fn failure_banner(failure: Failure, cx: &App) -> AnyElement {
     let colors = Aurora::colors(cx);
-    let role = if cancelled {
+    let role = if failure.cancelled {
         Role::TextSecondary
     } else {
         Role::Danger
@@ -157,7 +160,7 @@ fn failure_banner(message: String, cancelled: bool, cx: &App) -> AnyElement {
         .border_color(colors.get(Role::Border))
         .text_size(px(text::BODY))
         .text_color(colors.get(role))
-        .child(message)
+        .child(failure.message)
         .test_support()
         .into_any_element()
 }
```

Change `crates/app/src/views/workspace/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/mod.rs b/crates/app/src/views/workspace/mod.rs
--- a/crates/app/src/views/workspace/mod.rs
+++ b/crates/app/src/views/workspace/mod.rs
@@ -17,7 +17,7 @@ use quarry_ui_kit::Aurora;
 
 use super::{editor, results::ResultsDelegate, schema::SchemaTree};
 use crate::{
-    model::run::RunModel,
+    model::statements::RunOutput,
     shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
 };
 
@@ -84,9 +84,9 @@ impl Workspace {
         &self.schema
     }
 
-    /// The run the grid shows.
-    pub fn run_model<'a>(&self, cx: &'a App) -> &'a RunModel {
-        self.results.read(cx).delegate().model()
+    /// The latest run.
+    pub fn run_output<'a>(&self, cx: &'a App) -> &'a RunOutput {
+        self.results.read(cx).delegate().output()
     }
 
     /// Whether the user can drag result columns into a new order.
```

Change `crates/app/src/views/workspace/runner.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/runner.rs b/crates/app/src/views/workspace/runner.rs
--- a/crates/app/src/views/workspace/runner.rs
+++ b/crates/app/src/views/workspace/runner.rs
@@ -15,7 +15,7 @@ use super::{POLL, Workspace};
 use crate::{
     model::{
         drain::{Drained, drain},
-        run::RunModel,
+        statements::RunOutput,
         target,
     },
     shell::actions::{CancelRun, RunStatement},
@@ -31,11 +31,9 @@ impl Workspace {
         let Some(sql) = self.target_sql(cx) else {
             return;
         };
-        let handle = self.session.run(sql);
+        let handle = self.session.run(sql.clone());
         self.results.update(cx, |table, cx| {
-            table
-                .delegate_mut()
-                .reset(RunModel::started(Instant::now()));
+            table.delegate_mut().reset(RunOutput::submitted(sql));
             table.refresh(cx);
         });
         self.run = Some(Self::pump(handle.events.clone(), cx));
@@ -105,9 +103,10 @@ impl Workspace {
     fn apply(&mut self, events: Vec<RunEvent>, vanished: bool, cx: &mut Context<Self>) {
         self.results.update(cx, |table, cx| {
             let delegate = table.delegate_mut();
+            let now = Instant::now();
             let mut reshaped = false;
             for event in events {
-                reshaped |= delegate.apply(event);
+                reshaped |= delegate.apply(event, now);
             }
             if vanished {
                 delegate.end_closed();
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every app test passes (64 unit tests, plus the existing integration tests).

Run: `just check`
Expected: exit 0.

Run: `cargo mutants -p quarry --in-diff <(git diff main)`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Step 5: Commit**

```bash
git add -A crates/app/src/model/drain.rs crates/app/src/model/format.rs crates/app/src/model/messages.rs crates/app/src/model/mod.rs crates/app/src/model/plan_tree.rs crates/app/src/model/run.rs crates/app/src/model/statements.rs crates/app/src/model/status.rs crates/app/src/views/results.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/runner.rs crates/app/tests/workspace.rs
git commit -m "feat(app): keep each statement's output, with messages and plan rows"
```


---

### Task 5: Refactor: the editor and run move into `QueryTab`

No change in behaviour: today's single editor, grid and run move out of `Workspace` into a `QueryTab` entity, ready for several tabs in Task 6. The workspace still submits runs (it owns the `Session`) and re-reads the schema when a tab reports `RunEnded`. Folded in: a new run clears the grid's selection and scrolls to the top (M3); cells borrow their value (M4); the fields are `run` (the handle) and `pump` (the polling task) (M9); the test helper returns `Result` instead of calling `unreachable!` (M11); the shell, not the workspace, follows the system appearance (M12); a test covers a channel that closes without `Done`. The session's event buffer grows from 16 to 128 batches (M2): with a 33 ms poll, the 200,000-row sample streams in about 0.16 s instead of 1 s. `EVENT_BUFFER` becomes public, and the engine's `BurstDriver` test double sends exactly that many batches, so the test that drops a session mid-statement still fills the buffer (with a fixed 16 it would spin forever).

**Files:**
- Modify: `crates/app/src/shell/window.rs`
- Modify: `crates/app/src/views/mod.rs`
- Create: `crates/app/src/views/query_tab/layout.rs`
- Create: `crates/app/src/views/query_tab/mod.rs`
- Create: `crates/app/src/views/query_tab/pump.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Delete: `crates/app/src/views/workspace/runner.rs`
- Create: `crates/app/src/views/workspace/runs.rs`
- Modify: `crates/app/tests/workspace.rs`
- Modify: `crates/engine/src/session.rs`
- Modify: `crates/engine/tests/session.rs`

**Interfaces:**
- Consumes: Tasks 3–4.
- Produces: `views::query_tab::{QueryTab, QueryTabEvent::RunEnded}` with `QueryTab::{new(&str, &mut Window, &mut Context<Self>), editor, output, results, target_sql, start(String, RunHandle, cx), follow(String, Receiver<RunEvent>, cx), cancel}`; `Workspace::{active_tab, editor(&App) -> Entity<EditorState>}`; `views::query_tab::layout::EDITOR_HEIGHT`. `quarry_engine::session::EVENT_BUFFER` (now `pub`, 128). `Workspace::new` keeps its 1C signature here.

- [ ] **Step 1: Write the failing tests**

Change `crates/app/tests/workspace.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/workspace.rs b/crates/app/tests/workspace.rs
--- a/crates/app/tests/workspace.rs
+++ b/crates/app/tests/workspace.rs
@@ -16,36 +16,37 @@ use quarry::{
         workspace::{POLL, Workspace},
     },
 };
-use quarry_engine::error::EngineError;
+use quarry_engine::{error::EngineError, session::RunEvent};
 use quarry_ui_kit::{Aurora, Preference};
 
 /// How long a test waits for the session thread before giving up.
 const PATIENCE: Duration = Duration::from_secs(10);
 
-fn open(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
+/// A workspace on a fresh sample database, with its editor focused.
+fn open(
+    cx: &mut TestAppContext,
+) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
     let installed = cx.update(|cx| {
         gpui_kit::init(cx);
         actions::install(cx);
         Aurora::install(Preference::Dark, cx)
     });
     assert_eq!(installed, Ok(()));
-    let (workspace, cx) = cx.add_window_view(|window, cx| {
-        let session = sample::open()
-            .unwrap_or_else(|error| unreachable!("the sample database opens: {error}"));
-        Workspace::new(session, sample::NAME, window, cx)
-    });
+    let session = sample::open()?;
+    let (workspace, cx) =
+        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, window, cx));
     cx.update(|window, cx| {
-        let editor = workspace.read(cx).editor().clone();
+        let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| state.focus(window, cx));
     });
     settle(cx, |_, _| true);
-    (workspace, cx)
+    Ok((workspace, cx))
 }
 
 /// Replaces the editor's text and puts the cursor at the end.
 fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &str) {
     cx.update(|window, cx| {
-        let editor = workspace.read(cx).editor().clone();
+        let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| {
             state.set_value(sql.to_owned(), window, cx);
             state.set_selected_range(sql.len()..sql.len(), cx);
@@ -82,9 +83,10 @@ fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
 
 #[gpui_kit::test]
 fn the_editor_never_has_a_completion_provider(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     cx.update(|_, cx| {
-        let editor = workspace.read(cx).editor().read(cx);
+        let editor = workspace.read(cx).editor(cx);
+        let editor = editor.read(cx);
         let lsp = editor.lsp();
         assert!(lsp.completion_provider.is_none());
         assert!(lsp.hover_provider.is_none());
@@ -95,15 +97,18 @@ fn the_editor_never_has_a_completion_provider(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn result_columns_cannot_be_dragged_out_of_order(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     write(&workspace, cx, "SELECT name, nationality FROM astronauts");
     run(&workspace, cx);
-    cx.update(|_, cx| assert!(!workspace.read(cx).columns_movable(cx)));
+    cx.update(|_, cx| {
+        let tab = workspace.read(cx).active_tab().read(cx);
+        assert!(!tab.results().read(cx).col_movable);
+    });
 }
 
 #[gpui_kit::test]
 fn command_return_in_the_editor_runs_the_statement(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     write(&workspace, cx, "SELECT name FROM astronauts ORDER BY id");
     run(&workspace, cx);
     assert_eq!(phase(&workspace, cx), Phase::Finished);
@@ -116,18 +121,18 @@ fn command_return_in_the_editor_runs_the_statement(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn command_return_leaves_the_editor_text_alone(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     write(&workspace, cx, "SELECT 1");
     run(&workspace, cx);
     cx.update(|_, cx| {
-        let text = workspace.read(cx).editor().read(cx).value();
+        let text = workspace.read(cx).editor(cx).read(cx).value();
         assert_eq!(text.as_ref(), "SELECT 1");
     });
 }
 
 #[gpui_kit::test]
 fn a_bad_statement_shows_the_database_error(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     write(&workspace, cx, "SELECT * FROM nowhere");
     run(&workspace, cx);
     assert_eq!(
@@ -142,7 +147,7 @@ fn a_bad_statement_shows_the_database_error(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     write(
         &workspace,
         cx,
@@ -157,7 +162,7 @@ fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn running_again_during_a_run_replaces_it(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     write(
         &workspace,
         cx,
@@ -176,7 +181,7 @@ fn running_again_during_a_run_replaces_it(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn creating_a_table_adds_it_to_the_schema_tree(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     write(&workspace, cx, "CREATE TABLE notes (body TEXT)");
     run(&workspace, cx);
     settle(cx, |window, cx| {
@@ -188,7 +193,7 @@ fn creating_a_table_adds_it_to_the_schema_tree(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn clicking_a_table_expands_it(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     settle(cx, |window, cx| {
         window.draw(cx).clear(cx);
         window.try_find(table_row_id("missions")).is_some()
@@ -202,7 +207,7 @@ fn clicking_a_table_expands_it(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn command_backslash_toggles_the_sidebar(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-\\");
     cx.update(|_, cx| assert!(!workspace.read(cx).sidebar_open()));
     cx.simulate_keystrokes("cmd-\\");
@@ -211,7 +216,7 @@ fn command_backslash_toggles_the_sidebar(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn command_shift_t_cycles_the_theme(cx: &mut TestAppContext) {
-    let (_, cx) = open(cx);
+    let (_, cx) = open(cx).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-shift-t");
     cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::Light));
     cx.simulate_keystrokes("cmd-shift-t");
@@ -220,7 +225,7 @@ fn command_shift_t_cycles_the_theme(cx: &mut TestAppContext) {
 
 #[gpui_kit::test]
 fn command_one_focuses_the_first_table_and_return_expands_it(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     settle(cx, |window, cx| {
         window.draw(cx).clear(cx);
         window.try_find(table_row_id("astronauts")).is_some()
@@ -239,14 +244,15 @@ fn command_one_focuses_the_first_table_and_return_expands_it(cx: &mut TestAppCon
 
 #[gpui_kit::test]
 fn command_two_returns_focus_to_the_editor(cx: &mut TestAppContext) {
-    let (workspace, cx) = open(cx);
+    let (workspace, cx) = open(cx).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-1");
     settle(cx, |window, cx| {
         window.draw(cx).clear(cx);
         true
     });
     cx.update(|window, cx| {
-        let editor = workspace.read(cx).editor().read(cx);
+        let editor = workspace.read(cx).editor(cx);
+        let editor = editor.read(cx);
         assert!(
             !editor.focus_handle(cx).is_focused(window),
             "cmd-1 left focus in the editor"
@@ -254,7 +260,45 @@ fn command_two_returns_focus_to_the_editor(cx: &mut TestAppContext) {
     });
     cx.simulate_keystrokes("cmd-2");
     cx.update(|window, cx| {
-        let editor = workspace.read(cx).editor().read(cx);
+        let editor = workspace.read(cx).editor(cx);
+        let editor = editor.read(cx);
         assert!(editor.focus_handle(cx).is_focused(window));
     });
 }
+
+#[gpui_kit::test]
+fn a_run_whose_channel_closes_without_done_reports_a_lost_worker(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    let (sender, events) = async_channel::unbounded();
+    let started = RunEvent::Started {
+        index: 0,
+        range: 0..8,
+    };
+    assert!(sender.try_send(started).is_ok());
+    drop(sender);
+    cx.update(|_, cx| {
+        let tab = workspace.read(cx).active_tab().clone();
+        tab.update(cx, |tab, cx| tab.follow("SELECT 1".to_owned(), events, cx));
+    });
+    settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
+    assert_eq!(
+        phase(&workspace, cx),
+        Phase::Failed(EngineError::WorkerGone)
+    );
+    cx.update(|window, cx| {
+        window.draw(cx).clear(cx);
+        assert!(window.try_find("run-failure").is_some());
+    });
+}
+
+#[gpui_kit::test]
+fn a_new_run_clears_the_grid_selection(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    write(&workspace, cx, "SELECT * FROM astronauts");
+    run(&workspace, cx);
+    let results = cx.update(|_, cx| workspace.read(cx).active_tab().read(cx).results().clone());
+    cx.update(|_, cx| results.update(cx, |table, cx| table.set_selected_row(5, cx)));
+    write(&workspace, cx, "SELECT 1");
+    run(&workspace, cx);
+    cx.update(|_, cx| assert_eq!(results.read(cx).selected_row(), None));
+}
```

Change `crates/engine/tests/session.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/engine/tests/session.rs b/crates/engine/tests/session.rs
--- a/crates/engine/tests/session.rs
+++ b/crates/engine/tests/session.rs
@@ -5,7 +5,7 @@ use quarry_engine::{
     error::EngineError,
     result::{RowSink, Summary},
     schema::Schema,
-    session::{RunEvent, RunHandle, Session},
+    session::{EVENT_BUFFER, RunEvent, RunHandle, Session},
     sqlite::SqliteDriver,
     value::Value,
 };
@@ -23,7 +23,7 @@ type TestResult = Result<(), Box<dyn std::error::Error>>;
 const ENDLESS: &str =
     "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
 
-/// Streams 100,000 rows: far more than the event buffer holds.
+/// Streams 100,000 rows (200 batches): more than the event buffer holds.
 const MANY: &str = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 100000) SELECT x FROM n";
 
 fn session() -> Result<Session, EngineError> {
@@ -93,8 +93,9 @@ impl Driver for GateDriver {
     }
 }
 
-/// A driver whose statements each send 16 empty batches, more than the event
-/// buffer holds after `Started`, then succeed whatever the cancel flag says.
+/// A driver whose statements each send [`EVENT_BUFFER`] empty batches, more
+/// than the event buffer holds after `Started`, then succeed whatever the
+/// cancel flag says.
 #[derive(Default)]
 struct BurstDriver {
     canceller: Canceller,
@@ -102,7 +103,7 @@ struct BurstDriver {
 
 impl Driver for BurstDriver {
     fn execute(&mut self, _sql: &str, sink: &mut dyn RowSink) -> Result<Summary, EngineError> {
-        for _ in 0..16 {
+        for _ in 0..EVENT_BUFFER {
             sink.rows(Vec::new());
         }
         Ok(Summary {
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry`
Expected: the two new tests fail to compile (`no method named follow`, `no method named active_tab`).

- [ ] **Step 3: Implement**

Change `crates/app/src/shell/window.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/shell/window.rs b/crates/app/src/shell/window.rs
--- a/crates/app/src/shell/window.rs
+++ b/crates/app/src/shell/window.rs
@@ -5,7 +5,7 @@ use gpui_kit::{
     WindowOptions, point, px, size,
 };
 use quarry_engine::error::EngineError;
-use quarry_ui_kit::tokens::chrome;
+use quarry_ui_kit::{Aurora, tokens::chrome};
 
 use crate::{sample, views::workspace::Workspace};
 
@@ -34,14 +34,18 @@ pub fn options(cx: &App) -> WindowOptions {
 }
 
 /// Opens a workspace on a fresh sample database and focuses its editor.
+/// The window follows the system's light and dark appearance.
 ///
 /// # Errors
 /// Returns the engine's error when the sample database cannot be opened.
 pub fn open_main(cx: &mut App) -> Result<(), EngineError> {
     let session = sample::open()?;
     let opened = gpui_kit::open_window(options(cx), cx, |window, cx| {
+        window
+            .observe_window_appearance(|_, cx| Aurora::system_appearance_changed(cx))
+            .detach();
         let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, window, cx));
-        let editor = workspace.read(cx).editor().clone();
+        let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| state.focus(window, cx));
         workspace
     });
```

Change `crates/app/src/views/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/mod.rs b/crates/app/src/views/mod.rs
--- a/crates/app/src/views/mod.rs
+++ b/crates/app/src/views/mod.rs
@@ -1,6 +1,7 @@
 //! GPUI views: each renders one region of the workspace.
 
 pub mod editor;
+pub mod query_tab;
 pub mod results;
 pub mod schema;
 pub mod workspace;
```

Create `crates/app/src/views/query_tab/layout.rs`:
```rust
//! Draws a query tab: the editor over its results, with a draggable divider.

use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestSupportExt, Window,
    component::{
        input::Editor,
        resizable::{resizable_panel, v_resizable},
        table::DataTable,
    },
    div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::surface::glass_panel,
    tokens::{space, text},
};

use super::QueryTab;
use crate::model::status::{self, Failure};

/// The editor panel's starting height; the divider below it drags.
pub const EDITOR_HEIGHT: f32 = 220.0;

impl Render for QueryTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let editor = glass_panel(&colors)
            .size_full()
            .p(px(space::SM))
            .child(Editor::new(&self.editor).appearance(false).h_full());
        v_resizable("editor-results")
            .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
            .child(resizable_panel().child(self.results_panel(cx)))
    }
}

impl QueryTab {
    fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let failure = status::failure(self.output(cx));
        glass_panel(&colors)
            .id("results")
            .size_full()
            .mt(px(space::SM))
            .flex()
            .flex_col()
            .overflow_hidden()
            .children(failure.map(|failure| failure_banner(failure, cx)))
            .child(
                div()
                    .flex_1()
                    .child(DataTable::new(&self.results).bordered(false)),
            )
            .into_any_element()
    }
}

/// The run's error above the grid; a cancel reads quietly.
fn failure_banner(failure: Failure, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    let role = if failure.cancelled {
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
        .child(failure.message)
        .test_support()
        .into_any_element()
}
```

Create `crates/app/src/views/query_tab/mod.rs`:
```rust
//! One query tab: its editor, its latest run, and that run's results.
//!
//! The workspace owns the [`quarry_engine::session::Session`] and submits
//! runs; a tab follows its run's events (see [`pump`]) and tells the
//! workspace when the run ends.

mod layout;
mod pump;

use async_channel::Receiver;
use gpui_kit::{
    App, AppContext, Context, Entity, EventEmitter, Task, Window,
    component::{input::EditorState, table::TableState},
};
use quarry_engine::session::{RunEvent, RunHandle};

use super::{editor, results::ResultsDelegate};
use crate::model::{statements::RunOutput, target};

/// What a tab tells the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryTabEvent {
    /// The tab's run ended; the schema may have changed.
    RunEnded,
}

/// A query tab.
pub struct QueryTab {
    editor: Entity<EditorState>,
    results: Entity<TableState<ResultsDelegate>>,
    run: Option<RunHandle>,
    pump: Option<Task<()>>,
}

impl EventEmitter<QueryTabEvent> for QueryTab {}

impl QueryTab {
    /// A tab whose editor holds `text`.
    pub fn new(text: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // The delegate indexes rows by model column, so a dragged header
        // would end up over the wrong data.
        let results =
            cx.new(|cx| TableState::new(ResultsDelegate::default(), window, cx).col_movable(false));
        Self {
            editor: editor::new_editor(text, window, cx),
            results,
            run: None,
            pump: None,
        }
    }

    /// The editor, so callers can focus it or read its text.
    pub fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// The latest run.
    pub fn output<'a>(&self, cx: &'a App) -> &'a RunOutput {
        self.results.read(cx).delegate().output()
    }

    /// The results grid's state.
    pub fn results(&self) -> &Entity<TableState<ResultsDelegate>> {
        &self.results
    }

    /// The SQL a Run sends: the selection, or the statement under the cursor.
    pub fn target_sql(&self, cx: &App) -> Option<String> {
        let state = self.editor.read(cx);
        let script = state.value();
        let range = target::run_target(&script, state.selected_range())?;
        script.get(range).map(str::to_owned)
    }

    /// Shows the run of `script` that `handle` follows, replacing the last.
    pub fn start(&mut self, script: String, handle: RunHandle, cx: &mut Context<Self>) {
        self.follow(script, handle.events.clone(), cx);
        self.run = Some(handle);
    }

    /// Shows the run of `script` whose events arrive on `events`.
    pub fn follow(&mut self, script: String, events: Receiver<RunEvent>, cx: &mut Context<Self>) {
        self.run = None;
        self.results.update(cx, |table, cx| {
            table.delegate_mut().reset(RunOutput::submitted(script));
            table.clear_selection(cx);
            table.scroll_to_row(0, cx);
            table.refresh(cx);
        });
        self.pump = Some(Self::pump(events, cx));
        cx.notify();
    }

    /// Cancels the tab's run, if one is going.
    pub fn cancel(&self) {
        if let Some(handle) = &self.run {
            handle.cancel();
        }
    }
}
```

Create `crates/app/src/views/query_tab/pump.rs`:
```rust
//! Follows a run: polls its events once per [`POLL`] and folds them into
//! the grid.
//!
//! The channel is polled rather than awaited (see [`crate::model::drain`]);
//! each poll also redraws, which keeps the live timer moving.

use std::time::Instant;

use async_channel::Receiver;
use gpui_kit::{Context, Task};
use quarry_engine::session::RunEvent;

use super::{QueryTab, QueryTabEvent};
use crate::{
    model::drain::{Drained, drain},
    views::workspace::POLL,
};

impl QueryTab {
    /// Polls the run's events once per [`POLL`] until the run ends.
    pub(super) fn pump(events: Receiver<RunEvent>, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            let mut more = true;
            while more {
                cx.background_executor().timer(POLL).await;
                let drained = drain(&events);
                more = this
                    .update(cx, |tab, cx| tab.receive(drained, cx))
                    .unwrap_or(false);
            }
        })
    }

    /// Applies one poll's events; returns whether the run goes on.
    fn receive(&mut self, drained: Drained<RunEvent>, cx: &mut Context<Self>) -> bool {
        let done = drained.items.contains(&RunEvent::Done);
        let vanished = drained.closed && !done;
        let ended = drained.closed || done;
        self.apply(drained.items, vanished, cx);
        if ended {
            self.run = None;
            cx.emit(QueryTabEvent::RunEnded);
        }
        cx.notify();
        !ended
    }

    /// Folds the events in; a channel that closed without `Done` ends the run
    /// as a vanished worker, unless an earlier event already failed it.
    fn apply(&mut self, events: Vec<RunEvent>, vanished: bool, cx: &mut Context<Self>) {
        self.results.update(cx, |table, cx| {
            let delegate = table.delegate_mut();
            let now = Instant::now();
            let mut reshaped = false;
            for event in events {
                reshaped |= delegate.apply(event, now);
            }
            if vanished {
                delegate.end_closed();
            }
            if reshaped {
                table.refresh(cx);
            }
            cx.notify();
        });
    }
}
```

Change `crates/app/src/views/workspace/layout.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/layout.rs b/crates/app/src/views/workspace/layout.rs
--- a/crates/app/src/views/workspace/layout.rs
+++ b/crates/app/src/views/workspace/layout.rs
@@ -1,40 +1,26 @@
-//! Draws the workspace: sidebar, editor over results, status bar.
+//! Draws the workspace: sidebar, the query tab, status bar.
 
 use std::time::Instant;
 
 use gpui_kit::{
-    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render,
-    StatefulInteractiveElement, Styled, TestSupportExt, Window,
-    component::{
-        input::Editor,
-        resizable::{resizable_panel, v_resizable},
-        table::DataTable,
-    },
-    div,
-    prelude::FluentBuilder,
-    px,
+    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render,
+    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
 };
 use quarry_ui_kit::{
     Aurora, Role,
     components::{
         status_bar::StatusBar,
-        surface::{glass_panel, sidebar_surface, window_background},
+        surface::{sidebar_surface, window_background},
     },
     tokens::{chrome, space, text},
 };
 
 use super::{SIDEBAR_WIDTH, Workspace};
 use crate::{
-    model::{
-        format,
-        status::{self, Failure},
-    },
+    model::{format, status},
     shell::actions::WORKSPACE,
 };
 
-/// The editor panel's starting height; the divider below it drags.
-pub const EDITOR_HEIGHT: f32 = 220.0;
-
 impl Render for Workspace {
     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
         if Aurora::is_transitioning(cx) {
@@ -87,51 +73,28 @@ impl Workspace {
     }
 
     fn center(&self, cx: &Context<Self>) -> AnyElement {
-        let colors = Aurora::colors(cx);
-        let editor = glass_panel(&colors)
-            .size_full()
-            .p(px(space::SM))
-            .child(Editor::new(&self.editor).appearance(false).h_full());
         div()
             .flex_1()
             .h_full()
             .flex()
             .flex_col()
             .pt(px(chrome::TITLEBAR_HEIGHT))
-            .child(
-                div().flex_1().px(px(space::MD)).pb(px(space::SM)).child(
-                    v_resizable("editor-results")
-                        .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
-                        .child(resizable_panel().child(self.results_panel(cx))),
-                ),
-            )
-            .child(self.status_bar(cx))
-            .into_any_element()
-    }
-
-    fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
-        let colors = Aurora::colors(cx);
-        let failure = status::failure(self.run_output(cx));
-        glass_panel(&colors)
-            .id("results")
-            .size_full()
-            .mt(px(space::SM))
-            .flex()
-            .flex_col()
-            .overflow_hidden()
-            .children(failure.map(|failure| failure_banner(failure, cx)))
             .child(
                 div()
                     .flex_1()
-                    .child(DataTable::new(&self.results).bordered(false)),
+                    .px(px(space::MD))
+                    .pb(px(space::SM))
+                    .child(self.tab.clone()),
             )
+            .child(self.status_bar(cx))
             .into_any_element()
     }
 
     fn status_bar(&self, cx: &Context<Self>) -> AnyElement {
-        let output = self.run_output(cx);
+        let tab = self.tab.read(cx);
+        let output = tab.output(cx);
         let summary = status::run_summary(output, output.elapsed(Instant::now()));
-        let cursor = self.editor.read(cx).cursor_position();
+        let cursor = tab.editor().read(cx).cursor_position();
         let place = format::position(cursor.line, cursor.character);
         div()
             .id("status")
@@ -143,24 +106,3 @@ impl Workspace {
             .into_any_element()
     }
 }
-
-/// The run's error above the grid; a cancel reads quietly.
-fn failure_banner(failure: Failure, cx: &App) -> AnyElement {
-    let colors = Aurora::colors(cx);
-    let role = if failure.cancelled {
-        Role::TextSecondary
-    } else {
-        Role::Danger
-    };
-    div()
-        .id("run-failure")
-        .px(px(space::MD))
-        .py(px(space::SM))
-        .border_b_1()
-        .border_color(colors.get(Role::Border))
-        .text_size(px(text::BODY))
-        .text_color(colors.get(role))
-        .child(failure.message)
-        .test_support()
-        .into_any_element()
-}
```

Change `crates/app/src/views/workspace/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/mod.rs b/crates/app/src/views/workspace/mod.rs
--- a/crates/app/src/views/workspace/mod.rs
+++ b/crates/app/src/views/workspace/mod.rs
@@ -1,21 +1,25 @@
-//! The workspace: schema sidebar, editor over results, and the status bar.
+//! The workspace: schema sidebar, the query tab, and the status bar.
 //!
 //! This module owns the workspace's state and its window commands;
-//! [`runner`] drives runs against the [`Session`] and [`layout`] draws it.
+//! [`runs`] submits runs and reads the schema, and [`layout`] draws it.
 
 mod layout;
-mod runner;
+mod runs;
 
 use std::time::Duration;
 
 use gpui_kit::{
     App, AppContext, Context, Entity, FocusHandle, Focusable, Task, Window,
-    component::{input::EditorState, table::TableState},
+    component::input::EditorState,
 };
-use quarry_engine::session::{RunHandle, Session};
+use quarry_engine::session::Session;
 use quarry_ui_kit::Aurora;
 
-use super::{editor, results::ResultsDelegate, schema::SchemaTree};
+use super::{
+    editor,
+    query_tab::{QueryTab, QueryTabEvent},
+    schema::SchemaTree,
+};
 use crate::{
     model::statements::RunOutput,
     shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
@@ -31,14 +35,11 @@ pub const SIDEBAR_WIDTH: f32 = 240.0;
 pub struct Workspace {
     session: Session,
     database: String,
-    editor: Entity<EditorState>,
-    results: Entity<TableState<ResultsDelegate>>,
+    tab: Entity<QueryTab>,
     schema: Entity<SchemaTree>,
     sidebar_open: bool,
     focus: FocusHandle,
     sidebar_focus: FocusHandle,
-    current: Option<RunHandle>,
-    run: Option<Task<()>>,
     schema_read: Option<Task<()>>,
 }
 
@@ -50,48 +51,43 @@ impl Workspace {
         window: &mut Window,
         cx: &mut Context<Self>,
     ) -> Self {
-        let editor = editor::new_editor(editor::WELCOME, window, cx);
-        // The delegate indexes rows by model column, so a dragged header
-        // would end up over the wrong data.
-        let results =
-            cx.new(|cx| TableState::new(ResultsDelegate::default(), window, cx).col_movable(false));
-        cx.observe_window_appearance(window, |_, _, cx| Aurora::system_appearance_changed(cx))
-            .detach();
+        let tab = cx.new(|cx| QueryTab::new(editor::WELCOME, window, cx));
+        cx.subscribe(&tab, |workspace, _, event, cx| match event {
+            QueryTabEvent::RunEnded => workspace.read_schema(cx),
+        })
+        .detach();
         let mut workspace = Self {
             session,
             database: database.to_owned(),
-            editor,
-            results,
+            tab,
             schema: cx.new(|_| SchemaTree::default()),
             sidebar_open: true,
             focus: cx.focus_handle(),
             sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
-            current: None,
-            run: None,
             schema_read: None,
         };
         workspace.read_schema(cx);
         workspace
     }
 
-    /// The editor, so callers can focus it or set its text.
-    pub fn editor(&self) -> &Entity<EditorState> {
-        &self.editor
+    /// The active query tab.
+    pub fn active_tab(&self) -> &Entity<QueryTab> {
+        &self.tab
     }
 
-    /// The sidebar's schema tree.
-    pub fn schema(&self) -> &Entity<SchemaTree> {
-        &self.schema
+    /// The active tab's editor.
+    pub fn editor(&self, cx: &App) -> Entity<EditorState> {
+        self.tab.read(cx).editor().clone()
     }
 
-    /// The latest run.
+    /// The active tab's latest run.
     pub fn run_output<'a>(&self, cx: &'a App) -> &'a RunOutput {
-        self.results.read(cx).delegate().output()
+        self.tab.read(cx).output(cx)
     }
 
-    /// Whether the user can drag result columns into a new order.
-    pub fn columns_movable(&self, cx: &App) -> bool {
-        self.results.read(cx).col_movable
+    /// The sidebar's schema tree.
+    pub fn schema(&self) -> &Entity<SchemaTree> {
+        &self.schema
     }
 
     /// Whether the sidebar is shown.
@@ -118,7 +114,8 @@ impl Workspace {
     }
 
     fn focus_editor(&mut self, _: &FocusEditor, window: &mut Window, cx: &mut Context<Self>) {
-        self.editor.update(cx, |state, cx| state.focus(window, cx));
+        self.editor(cx)
+            .update(cx, |state, cx| state.focus(window, cx));
     }
 
     fn cycle_theme(_: &mut Self, _: &CycleTheme, _: &mut Window, cx: &mut Context<Self>) {
```

Delete `crates/app/src/views/workspace/runner.rs` with `git rm crates/app/src/views/workspace/runner.rs`.

Create `crates/app/src/views/workspace/runs.rs`:
```rust
//! Submits the active tab's runs to the session, and reads the schema.
//!
//! The schema reply is polled once per [`POLL`] rather than awaited (see
//! [`crate::model::drain`]).

use gpui_kit::{Context, Window};
use quarry_engine::{error::EngineError, schema::Schema};

use super::{POLL, Workspace};
use crate::{
    model::drain::{Drained, drain},
    shell::actions::{CancelRun, RunStatement},
};

impl Workspace {
    pub(super) fn run_statement(
        &mut self,
        _: &RunStatement,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(sql) = self.tab.read(cx).target_sql(cx) else {
            return;
        };
        let handle = self.session.run(sql.clone());
        self.tab.update(cx, |tab, cx| tab.start(sql, handle, cx));
        cx.notify();
    }

    pub(super) fn cancel(&mut self, _: &CancelRun, _: &mut Window, cx: &mut Context<Self>) {
        self.tab.read(cx).cancel();
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

Change `crates/engine/src/session.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/engine/src/session.rs b/crates/engine/src/session.rs
--- a/crates/engine/src/session.rs
+++ b/crates/engine/src/session.rs
@@ -44,7 +44,9 @@ use crate::{
 };
 
 /// Batches buffered per run before the session thread waits for the reader.
-const EVENT_BUFFER: usize = 16;
+/// A reader polling every 33 ms streams 200,000 rows in about 0.16 s with
+/// 128 batches, against about 1 s with 16 (ADR 0010).
+pub const EVENT_BUFFER: usize = 128;
 
 /// Something that happened while running a script.
 #[derive(Debug, Clone, PartialEq)]
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every app test passes; `workspace.rs` has 15 tests.

Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add -A crates/app/src/shell/window.rs crates/app/src/views/mod.rs crates/app/src/views/query_tab/layout.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/query_tab/pump.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/runner.rs crates/app/src/views/workspace/runs.rs crates/app/tests/workspace.rs crates/engine/src/session.rs crates/engine/tests/session.rs
git commit -m "refactor(app): move the editor and its run into a query tab"
```


---

### Task 6: Query tabs and saved drafts

⌘T, ⌘W, ⌘⇧] and ⌘⇧[ open, close and switch tabs, drawn as a strip in the title bar. Drafts load at start and save one second after the last edit (each edit restarts the timer), at once when tabs change, and when Quarry quits; saves run on the background executor one after another. `shell::store` opens `~/Library/Application Support/Quarry/quarry.db`; if it can't, a `Forgetful` store keeps nothing and the status bar says "Drafts aren't being saved", with the reason on hover. If the file opens but its drafts can't be loaded, the workspace falls back to `Forgetful` the same way, so a save never overwrites drafts still on disk; closing the window saves at once, like quitting.

**Files:**
- Modify: `crates/app/Cargo.toml`
- Modify: `crates/app/src/main.rs`
- Modify: `crates/app/src/shell/actions.rs`
- Modify: `crates/app/src/shell/mod.rs`
- Create: `crates/app/src/shell/store.rs`
- Modify: `crates/app/src/shell/window.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Create: `crates/app/src/views/workspace/drafts.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/src/views/workspace/runs.rs`
- Create: `crates/app/src/views/workspace/tab_strip.rs`
- Create: `crates/app/src/views/workspace/tabs.rs`
- Modify: `crates/app/tests/shell.rs`
- Create: `crates/app/tests/tabs.rs`
- Modify: `crates/app/tests/workspace.rs`

**Interfaces:**
- Consumes: Tasks 1, 3, 5.
- Produces: actions `NewTab` (cmd-t), `CloseTab` (cmd-w), `NextTab` (cmd-}), `PreviousTab` (cmd-{), with File and View menu items; `QueryTabEvent::Edited` and `QueryTab::text`; `views::workspace::{SavedTabs { store: Arc<dyn DraftStore>, problem: Option<String> }, AUTOSAVE, tab_element_id(TabId)}`; `Workspace::new(Session, &str, SavedTabs, &mut Window, &mut Context<Self>)`, `Workspace::{tabs, activate_tab, drafts_problem, snapshot}`; `shell::store::{default_path, saved_tabs(Option<&Path>) -> SavedTabs, Forgetful, FOLDER, FILE}`; `window::open_main(SavedTabs, &mut App)`.
- **Event note:** the editor emits `InputEvent::Change` for typing but not for `set_value`, so the autosave tests type with `simulate_input`.

- [ ] **Step 1: Write the failing tests**

Change `crates/app/tests/shell.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/shell.rs b/crates/app/tests/shell.rs
--- a/crates/app/tests/shell.rs
+++ b/crates/app/tests/shell.rs
@@ -1,8 +1,11 @@
 //! Start-up: the main window opens on the sample database, and every command
 //! has a key binding and a menu item.
 
+use std::path::Path;
+
 use gpui_kit::{MenuItem, TestAppContext, px};
-use quarry::shell::{actions, window};
+use quarry::shell::{actions, store, window};
+use quarry_store::drafts::Drafts;
 use quarry_ui_kit::{Aurora, Preference};
 
 #[gpui_kit::test]
@@ -11,7 +14,7 @@ fn the_main_window_opens_on_the_sample(cx: &mut TestAppContext) {
         gpui_kit::init(cx);
         actions::install(cx);
         assert_eq!(Aurora::install(Preference::Dark, cx), Ok(()));
-        window::open_main(cx)
+        window::open_main(store::saved_tabs(None), cx)
     });
     assert_eq!(opened, Ok(()));
     assert_eq!(cx.windows().len(), 1);
@@ -46,8 +49,42 @@ fn every_menu_command_has_a_key_binding() {
             _ => None,
         })
         .collect();
-    assert_eq!(commands.len(), 7);
+    assert_eq!(commands.len(), 11);
     for command in commands {
         assert!(bound.contains(&command), "{command} has no key binding");
     }
 }
+
+#[test]
+fn the_store_lives_in_application_support() {
+    let path = store::default_path();
+    let path = path.as_deref().map(Path::to_string_lossy);
+    assert!(
+        path.as_deref()
+            .is_some_and(|p| p.ends_with("Library/Application Support/Quarry/quarry.db")),
+        "{path:?}"
+    );
+}
+
+#[test]
+fn a_store_that_cannot_open_keeps_nothing_and_says_why() {
+    let dir = tempfile::tempdir().expect("a temporary folder");
+    let blocker = dir.path().join("file");
+    std::fs::write(&blocker, "not a folder").expect("a file");
+    let saved = store::saved_tabs(Some(&blocker.join("quarry.db")));
+    assert!(saved.problem.is_some_and(|p| p.starts_with("Cannot open")));
+    assert_eq!(saved.store.load(), Ok(Drafts::default()));
+    assert_eq!(saved.store.save(&Drafts::default()), Ok(()));
+    let none = store::saved_tabs(None);
+    assert_eq!(
+        none.problem.as_deref(),
+        Some("There is no home folder to save drafts in.")
+    );
+}
+
+#[test]
+fn a_store_that_opens_has_no_problem() {
+    let dir = tempfile::tempdir().expect("a temporary folder");
+    let saved = store::saved_tabs(Some(&dir.path().join("quarry.db")));
+    assert_eq!(saved.problem, None);
+}
```

Create `crates/app/tests/tabs.rs`:
```rust
//! Query tabs: opening, closing and switching them, and keeping their
//! drafts across restarts.

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

use gpui_kit::{App, Entity, TestAppContext, VisualTestContext, Window, test::TestWindowExt};
use quarry::{
    model::statements::Phase,
    sample,
    shell::actions,
    views::workspace::{AUTOSAVE, POLL, SavedTabs, Workspace, tab_element_id},
};
use quarry_engine::error::EngineError;
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};
use quarry_ui_kit::{Aurora, Preference};

/// How long a test waits for the session thread before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

/// A store whose every read and write fails.
struct Broken;

impl DraftStore for Broken {
    fn load(&self) -> Result<Drafts, StoreError> {
        Err(StoreError::Sql("disk I/O error".to_owned()))
    }

    fn save(&self, _: &Drafts) -> Result<(), StoreError> {
        Err(StoreError::Sql("disk I/O error".to_owned()))
    }
}

fn install(cx: &mut TestAppContext) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
}

/// A workspace on a fresh sample database, saving to `store`.
fn open(
    cx: &mut TestAppContext,
    store: Arc<dyn DraftStore>,
) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
    let session = sample::open()?;
    let saved = SavedTabs {
        store,
        problem: None,
    };
    let (workspace, cx) =
        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, saved, window, cx));
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| state.focus(window, cx));
    });
    settle(cx, |_, _| true);
    Ok((workspace, cx))
}

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

fn titles(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| {
        let tabs = workspace.read(cx).tabs().tabs();
        tabs.iter().map(|tab| tab.title.clone()).collect()
    })
}

fn active(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> usize {
    cx.update(|_, cx| workspace.read(cx).tabs().active())
}

fn text(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| workspace.read(cx).editor(cx).read(cx).value().to_string())
}

fn draft(id: i64, title: &str, text: &str) -> Draft {
    Draft {
        id,
        title: title.to_owned(),
        text: text.to_owned(),
    }
}

#[gpui_kit::test]
fn a_fresh_start_opens_query_one_with_the_welcome_query(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
    assert!(text(&workspace, cx).starts_with("-- Welcome to Quarry."));
}

#[gpui_kit::test]
fn command_t_opens_an_empty_tab_and_command_w_closes_it(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2"]);
    assert_eq!(active(&workspace, cx), 1);
    assert_eq!(text(&workspace, cx), "");
    cx.simulate_keystrokes("cmd-w");
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
    assert!(text(&workspace, cx).starts_with("-- Welcome"));
}

#[gpui_kit::test]
fn closing_the_only_tab_opens_a_fresh_one(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-w");
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
    assert_eq!(text(&workspace, cx), "");
}

#[gpui_kit::test]
fn next_and_previous_switch_tabs_and_wrap(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t cmd-t");
    assert_eq!(active(&workspace, cx), 2);
    cx.simulate_keystrokes("cmd-}");
    assert_eq!(active(&workspace, cx), 0);
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(active(&workspace, cx), 2);
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(active(&workspace, cx), 1);
}

#[gpui_kit::test]
fn clicking_a_tab_activates_it(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    let first = cx.update(|_, cx| workspace.read(cx).tabs().tabs()[0].id);
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.click(tab_element_id(first), cx);
    });
    assert_eq!(active(&workspace, cx), 0);
}

#[gpui_kit::test]
fn each_tab_keeps_its_own_results(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-enter");
    let finished = |cx: &mut VisualTestContext| {
        settle(cx, |_, cx| {
            workspace.read(cx).run_output(cx).phase() == &Phase::Finished
        });
    };
    finished(cx);
    cx.simulate_keystrokes("cmd-t");
    cx.simulate_input("SELECT 1");
    cx.simulate_keystrokes("cmd-enter");
    finished(cx);
    let rows = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            workspace.read(cx).run_output(cx).statements()[0]
                .rows()
                .len()
        })
    };
    assert_eq!(rows(cx), 1);
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(rows(cx), 12);
}

#[gpui_kit::test]
fn drafts_come_back_in_a_new_workspace(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    cx.simulate_input("SELECT 42");
    cx.executor().advance_clock(AUTOSAVE);
    settle(cx, |_, _| true);
    let saved = store.load().expect("the drafts load");
    assert_eq!(
        (saved.active, saved.tabs[1].text.as_str()),
        (1, "SELECT 42")
    );
    drop(workspace);
    let mut again = TestAppContext::single();
    install(&mut again);
    let (restored, again) = open(&mut again, store).expect("the sample database opens");
    assert_eq!(titles(&restored, again), ["Query 1", "Query 2"]);
    assert_eq!(
        (active(&restored, again), text(&restored, again)),
        (1, "SELECT 42".to_owned())
    );
}

#[gpui_kit::test]
fn an_edit_is_saved_only_after_a_pause(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    store
        .save(&Drafts {
            tabs: vec![draft(1, "Query 1", "")],
            active: 0,
        })
        .expect("a first save");
    let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_input("SELECT 7");
    cx.run_until_parked();
    assert_eq!(store.load().expect("the drafts load").tabs[0].text, "");
    cx.executor().advance_clock(AUTOSAVE);
    cx.run_until_parked();
    assert_eq!(
        store.load().expect("the drafts load").tabs[0].text,
        "SELECT 7"
    );
}

#[gpui_kit::test]
fn a_failing_store_shows_a_warning(cx: &mut TestAppContext) {
    install(cx);
    let (workspace, cx) = open(cx, Arc::new(Broken)).expect("the sample database opens");
    cx.update(|window, cx| {
        assert_eq!(workspace.read(cx).drafts_problem(), Some("disk I/O error"));
        window.draw(cx).clear(cx);
        assert!(window.try_find("drafts-warning").is_some());
    });
}

/// A store whose load fails but whose save would succeed, noting any save.
struct Unreadable {
    saved: Arc<AtomicBool>,
}

impl DraftStore for Unreadable {
    fn load(&self) -> Result<Drafts, StoreError> {
        Err(StoreError::Sql("file is not a database".to_owned()))
    }

    fn save(&self, _: &Drafts) -> Result<(), StoreError> {
        self.saved.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[gpui_kit::test]
fn a_store_that_failed_to_load_is_never_saved_over(cx: &mut TestAppContext) {
    install(cx);
    let saved = Arc::new(AtomicBool::new(false));
    let store = Arc::new(Unreadable {
        saved: Arc::clone(&saved),
    });
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_input("SELECT 1");
    cx.executor().advance_clock(AUTOSAVE);
    settle(cx, |_, _| true);
    cx.simulate_keystrokes("cmd-t");
    settle(cx, |_, _| true);
    assert!(!saved.load(Ordering::SeqCst));
    cx.update(|_, cx| assert!(workspace.read(cx).drafts_problem().is_some()));
}

#[gpui_kit::test]
fn closing_the_window_saves_the_last_edits(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_input("SELECT 9");
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    let loaded = store.load().expect("the drafts load");
    assert!(loaded.tabs[0].text.starts_with("SELECT 9"));
}
```

Change `crates/app/tests/workspace.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/workspace.rs b/crates/app/tests/workspace.rs
--- a/crates/app/tests/workspace.rs
+++ b/crates/app/tests/workspace.rs
@@ -1,7 +1,7 @@
 //! The workspace in a headless window: running, cancelling, the schema tree
 //! and the window commands.
 
-use std::time::Duration;
+use std::{sync::Arc, time::Duration};
 
 use gpui_kit::{
     App, Entity, Focusable, TestAppContext, VisualTestContext, Window,
@@ -10,10 +10,10 @@ use gpui_kit::{
 use quarry::{
     model::statements::Phase,
     sample,
-    shell::actions,
+    shell::{actions, store::Forgetful},
     views::{
         schema::table_row_id,
-        workspace::{POLL, Workspace},
+        workspace::{POLL, SavedTabs, Workspace},
     },
 };
 use quarry_engine::{error::EngineError, session::RunEvent};
@@ -33,8 +33,12 @@ fn open(
     });
     assert_eq!(installed, Ok(()));
     let session = sample::open()?;
+    let saved = SavedTabs {
+        store: Arc::new(Forgetful),
+        problem: None,
+    };
     let (workspace, cx) =
-        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, window, cx));
+        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, saved, window, cx));
     cx.update(|window, cx| {
         let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| state.focus(window, cx));
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry`
Expected: `unresolved import quarry::views::workspace::SavedTabs` and friends.

- [ ] **Step 3: Implement**

Change `crates/app/Cargo.toml` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/Cargo.toml b/crates/app/Cargo.toml
--- a/crates/app/Cargo.toml
+++ b/crates/app/Cargo.toml
@@ -17,10 +17,12 @@ path = "src/main.rs"
 async-channel.workspace = true
 gpui-kit = { workspace = true, features = ["assets"] }
 quarry-engine = { path = "../engine" }
+quarry-store = { path = "../store" }
 quarry-ui-kit = { path = "../ui-kit" }
 
 [dev-dependencies]
 gpui-kit = { workspace = true, features = ["test-support", "assets"] }
+tempfile.workspace = true
 
 [lints]
 workspace = true
```

Change `crates/app/src/main.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/main.rs b/crates/app/src/main.rs
--- a/crates/app/src/main.rs
+++ b/crates/app/src/main.rs
@@ -1,7 +1,7 @@
 //! Quarry: a native macOS SQL client for learning and real work.
 
 use gpui_kit::App;
-use quarry::shell::{actions, window};
+use quarry::shell::{actions, store, window};
 use quarry_ui_kit::{Aurora, Preference};
 
 fn main() {
@@ -12,7 +12,10 @@ fn main() {
             actions::install(cx);
             let started = Aurora::install(Preference::Dark, cx)
                 .map_err(|error| error.to_string())
-                .and_then(|()| window::open_main(cx).map_err(|error| error.to_string()));
+                .and_then(|()| {
+                    let saved = store::saved_tabs(store::default_path().as_deref());
+                    window::open_main(saved, cx).map_err(|error| error.to_string())
+                });
             if let Err(error) = started {
                 eprintln!("Quarry could not start: {error}");
                 cx.quit();
```

Change `crates/app/src/shell/actions.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/shell/actions.rs b/crates/app/src/shell/actions.rs
--- a/crates/app/src/shell/actions.rs
+++ b/crates/app/src/shell/actions.rs
@@ -12,6 +12,14 @@ actions!(
         RunStatement,
         /// Cancels the running query.
         CancelRun,
+        /// Opens a new query tab.
+        NewTab,
+        /// Closes the active query tab.
+        CloseTab,
+        /// Activates the next query tab.
+        NextTab,
+        /// Activates the previous query tab.
+        PreviousTab,
         /// Shows or hides the sidebar.
         ToggleSidebar,
         /// Moves keyboard focus to the sidebar's first table.
@@ -39,6 +47,10 @@ pub fn key_bindings() -> Vec<KeyBinding> {
         KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
+        KeyBinding::new("cmd-t", NewTab, None),
+        KeyBinding::new("cmd-w", CloseTab, None),
+        KeyBinding::new("cmd-}", NextTab, None),
+        KeyBinding::new("cmd-{", PreviousTab, None),
         KeyBinding::new("cmd-\\", ToggleSidebar, None),
         KeyBinding::new("cmd-1", FocusSidebar, None),
         KeyBinding::new("cmd-2", FocusEditor, None),
@@ -51,11 +63,17 @@ pub fn key_bindings() -> Vec<KeyBinding> {
 pub fn menus() -> Vec<Menu> {
     vec![
         Menu::new("Quarry").items([MenuItem::action("Quit Quarry", Quit)]),
+        Menu::new("File").items([
+            MenuItem::action("New Tab", NewTab),
+            MenuItem::action("Close Tab", CloseTab),
+        ]),
         Menu::new("Query").items([
             MenuItem::action("Run Statement", RunStatement),
             MenuItem::action("Cancel", CancelRun),
         ]),
         Menu::new("View").items([
+            MenuItem::action("Next Tab", NextTab),
+            MenuItem::action("Previous Tab", PreviousTab),
             MenuItem::action("Toggle Sidebar", ToggleSidebar),
             MenuItem::action("Focus Sidebar", FocusSidebar),
             MenuItem::action("Focus Editor", FocusEditor),
```

Change `crates/app/src/shell/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/shell/mod.rs b/crates/app/src/shell/mod.rs
--- a/crates/app/src/shell/mod.rs
+++ b/crates/app/src/shell/mod.rs
@@ -1,4 +1,6 @@
-//! Application start-up: actions, the menu bar and the main window.
+//! Application start-up: actions, the menu bar, the local store and the
+//! main window.
 
 pub mod actions;
+pub mod store;
 pub mod window;
```

Create `crates/app/src/shell/store.rs`:
```rust
//! Where Quarry keeps its local database, and what happens when it can't.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use quarry_store::{
    drafts::{DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};

use crate::views::workspace::SavedTabs;

/// The store's folder under the home folder.
pub const FOLDER: &str = "Library/Application Support/Quarry";

/// The store's file name.
pub const FILE: &str = "quarry.db";

/// `~/Library/Application Support/Quarry/quarry.db`, or `None` without a
/// home folder.
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(FOLDER).join(FILE))
}

/// Opens the store at `path`. When it can't be opened, drafts live only
/// as long as Quarry runs, and the reason says why.
pub fn saved_tabs(path: Option<&Path>) -> SavedTabs {
    let Some(path) = path else {
        return forgetful("There is no home folder to save drafts in.".to_owned());
    };
    match SqliteStore::open(path) {
        Ok(store) => SavedTabs {
            store: Arc::new(store),
            problem: None,
        },
        Err(error) => forgetful(error.to_string()),
    }
}

fn forgetful(problem: String) -> SavedTabs {
    SavedTabs {
        store: Arc::new(Forgetful),
        problem: Some(problem),
    }
}

/// A store that keeps nothing: it loads no tabs and every save succeeds.
pub struct Forgetful;

impl DraftStore for Forgetful {
    fn load(&self) -> Result<Drafts, StoreError> {
        Ok(Drafts::default())
    }

    fn save(&self, _: &Drafts) -> Result<(), StoreError> {
        Ok(())
    }
}
```

Change `crates/app/src/shell/window.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/shell/window.rs b/crates/app/src/shell/window.rs
--- a/crates/app/src/shell/window.rs
+++ b/crates/app/src/shell/window.rs
@@ -7,7 +7,10 @@ use gpui_kit::{
 use quarry_engine::error::EngineError;
 use quarry_ui_kit::{Aurora, tokens::chrome};
 
-use crate::{sample, views::workspace::Workspace};
+use crate::{
+    sample,
+    views::workspace::{SavedTabs, Workspace},
+};
 
 /// The main window's initial size.
 pub const WINDOW_SIZE: (f32, f32) = (1200.0, 760.0);
@@ -33,18 +36,19 @@ pub fn options(cx: &App) -> WindowOptions {
     }
 }
 
-/// Opens a workspace on a fresh sample database and focuses its editor.
-/// The window follows the system's light and dark appearance.
+/// Opens a workspace on a fresh sample database with the tabs in `saved`,
+/// and focuses its editor. The window follows the system's light and dark
+/// appearance.
 ///
 /// # Errors
 /// Returns the engine's error when the sample database cannot be opened.
-pub fn open_main(cx: &mut App) -> Result<(), EngineError> {
+pub fn open_main(saved: SavedTabs, cx: &mut App) -> Result<(), EngineError> {
     let session = sample::open()?;
     let opened = gpui_kit::open_window(options(cx), cx, |window, cx| {
         window
             .observe_window_appearance(|_, cx| Aurora::system_appearance_changed(cx))
             .detach();
-        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, window, cx));
+        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, saved, window, cx));
         let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| state.focus(window, cx));
         workspace
```

Change `crates/app/src/views/query_tab/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/query_tab/mod.rs b/crates/app/src/views/query_tab/mod.rs
--- a/crates/app/src/views/query_tab/mod.rs
+++ b/crates/app/src/views/query_tab/mod.rs
@@ -9,8 +9,11 @@ mod pump;
 
 use async_channel::Receiver;
 use gpui_kit::{
-    App, AppContext, Context, Entity, EventEmitter, Task, Window,
-    component::{input::EditorState, table::TableState},
+    App, AppContext, Context, Entity, EventEmitter, Subscription, Task, Window,
+    component::{
+        input::{EditorState, InputEvent},
+        table::TableState,
+    },
 };
 use quarry_engine::session::{RunEvent, RunHandle};
 
@@ -22,6 +25,8 @@ use crate::model::{statements::RunOutput, target};
 pub enum QueryTabEvent {
     /// The tab's run ended; the schema may have changed.
     RunEnded,
+    /// The user changed the editor's text.
+    Edited,
 }
 
 /// A query tab.
@@ -30,6 +35,7 @@ pub struct QueryTab {
     results: Entity<TableState<ResultsDelegate>>,
     run: Option<RunHandle>,
     pump: Option<Task<()>>,
+    _edits: Subscription,
 }
 
 impl EventEmitter<QueryTabEvent> for QueryTab {}
@@ -41,11 +47,18 @@ impl QueryTab {
         // would end up over the wrong data.
         let results =
             cx.new(|cx| TableState::new(ResultsDelegate::default(), window, cx).col_movable(false));
+        let editor = editor::new_editor(text, window, cx);
+        let edits = cx.subscribe(&editor, |_, _, event: &InputEvent, cx| {
+            if matches!(event, InputEvent::Change) {
+                cx.emit(QueryTabEvent::Edited);
+            }
+        });
         Self {
-            editor: editor::new_editor(text, window, cx),
+            editor,
             results,
             run: None,
             pump: None,
+            _edits: edits,
         }
     }
 
@@ -54,6 +67,11 @@ impl QueryTab {
         &self.editor
     }
 
+    /// The editor's text.
+    pub fn text(&self, cx: &App) -> String {
+        self.editor.read(cx).value().to_string()
+    }
+
     /// The latest run.
     pub fn output<'a>(&self, cx: &'a App) -> &'a RunOutput {
         self.results.read(cx).delegate().output()
```

Create `crates/app/src/views/workspace/drafts.rs`:
```rust
//! Saving the tabs' drafts: a second after the last edit, at once when the
//! tabs change, when the window closes and when Quarry quits.
//!
//! Writes run on the background executor, one after another, so a later
//! snapshot never lands before an earlier one.

use std::{sync::Arc, time::Duration};

use gpui_kit::{App, AppContext, Context, WindowId};
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
};

use super::Workspace;
use crate::shell::store::Forgetful;

/// How long after the last edit the drafts are saved.
pub const AUTOSAVE: Duration = Duration::from_secs(1);

/// Where the drafts are kept, and why they may not be.
pub struct SavedTabs {
    /// The store the workspace loads from and saves to.
    pub store: Arc<dyn DraftStore>,
    /// Why drafts are not being kept across restarts, if they are not.
    pub problem: Option<String>,
}

impl SavedTabs {
    /// The store to use, the drafts it holds and the warning to show. A store
    /// that cannot be read is replaced by one that keeps nothing, so a later
    /// save never overwrites what it holds.
    pub(super) fn open(self) -> (Arc<dyn DraftStore>, Drafts, Option<String>) {
        match self.store.load() {
            Ok(drafts) => (self.store, drafts, self.problem),
            Err(error) => (
                Arc::new(Forgetful),
                Drafts::default(),
                self.problem.or(Some(error.to_string())),
            ),
        }
    }
}

impl Workspace {
    /// Saves the drafts when the window `id` closes, so edits from the last
    /// second are not lost with the pending autosave.
    pub(super) fn save_on_close(id: WindowId, cx: &mut Context<Self>) {
        let this = cx.weak_entity();
        cx.on_window_closed(move |cx, closed| {
            if closed == id {
                let _ = this.read_with(cx, Workspace::save_before_quit);
            }
        })
        .detach();
    }

    /// Why drafts aren't being saved, if they aren't.
    pub fn drafts_problem(&self) -> Option<&str> {
        self.save_problem
            .as_deref()
            .or(self.open_problem.as_deref())
    }

    /// Every tab's title and text, and which one is active.
    pub fn snapshot(&self, cx: &App) -> Drafts {
        let tabs = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                Some(Draft {
                    id: tab.id,
                    title: tab.title.clone(),
                    text: self.views.get(&tab.id)?.read(cx).text(cx),
                })
            })
            .collect();
        Drafts {
            tabs,
            active: self.tabs.active(),
        }
    }

    /// Saves after [`AUTOSAVE`], unless another edit comes first.
    pub(super) fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.autosave = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AUTOSAVE).await;
            let _ = this.update(cx, Workspace::save_now);
        }));
    }

    /// Saves a snapshot now, after any save still being written.
    pub(super) fn save_now(&mut self, cx: &mut Context<Self>) {
        self.autosave = None;
        let drafts = self.snapshot(cx);
        let store = Arc::clone(&self.store);
        let previous = self.saving.take();
        self.saving = Some(cx.spawn(async move |this, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            let saved = cx
                .background_spawn(async move { store.save(&drafts) })
                .await;
            let _ = this.update(cx, |workspace, cx| workspace.saved(saved, cx));
        }));
    }

    /// Saves synchronously, for when Quarry is quitting.
    pub(super) fn save_before_quit(&self, cx: &App) {
        let _ = self.store.save(&self.snapshot(cx));
    }

    fn saved(&mut self, saved: Result<(), StoreError>, cx: &mut Context<Self>) {
        self.save_problem = saved.err().map(|error| error.to_string());
        cx.notify();
    }
}
```

Change `crates/app/src/views/workspace/layout.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/layout.rs b/crates/app/src/views/workspace/layout.rs
--- a/crates/app/src/views/workspace/layout.rs
+++ b/crates/app/src/views/workspace/layout.rs
@@ -3,8 +3,9 @@
 use std::time::Instant;
 
 use gpui_kit::{
-    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render,
-    StatefulInteractiveElement, Styled, Window, div, prelude::FluentBuilder, px,
+    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
+    StatefulInteractiveElement, Styled, TestSupportExt, Window, component::tooltip::Tooltip, div,
+    prelude::FluentBuilder, px,
 };
 use quarry_ui_kit::{
     Aurora, Role,
@@ -33,6 +34,10 @@ impl Render for Workspace {
             .track_focus(&self.focus)
             .on_action(cx.listener(Self::run_statement))
             .on_action(cx.listener(Self::cancel))
+            .on_action(cx.listener(Self::new_tab))
+            .on_action(cx.listener(Self::close_tab))
+            .on_action(cx.listener(Self::next_tab))
+            .on_action(cx.listener(Self::previous_tab))
             .on_action(cx.listener(Self::toggle_sidebar))
             .on_action(cx.listener(Self::focus_sidebar))
             .on_action(cx.listener(Self::focus_editor))
@@ -78,31 +83,44 @@ impl Workspace {
             .h_full()
             .flex()
             .flex_col()
-            .pt(px(chrome::TITLEBAR_HEIGHT))
+            .child(self.tab_strip(cx))
             .child(
                 div()
                     .flex_1()
                     .px(px(space::MD))
                     .pb(px(space::SM))
-                    .child(self.tab.clone()),
+                    .child(self.active_tab().clone()),
             )
             .child(self.status_bar(cx))
             .into_any_element()
     }
 
     fn status_bar(&self, cx: &Context<Self>) -> AnyElement {
-        let tab = self.tab.read(cx);
+        let tab = self.active_tab().read(cx);
         let output = tab.output(cx);
         let summary = status::run_summary(output, output.elapsed(Instant::now()));
         let cursor = tab.editor().read(cx).cursor_position();
         let place = format::position(cursor.line, cursor.character);
+        let warning = self.drafts_problem().map(drafts_warning);
         div()
             .id("status")
             .child(
                 StatusBar::new()
                     .left(format!("● {}", self.database))
+                    .left(div().children(warning))
                     .right(format!("{summary} · {place}")),
             )
             .into_any_element()
     }
 }
+
+/// The quiet warning that drafts aren't being saved, with the reason on hover.
+fn drafts_warning(problem: &str) -> AnyElement {
+    let reason = SharedString::from(problem.to_owned());
+    div()
+        .id("drafts-warning")
+        .child("Drafts aren't being saved")
+        .tooltip(move |window, cx| Tooltip::new(reason.clone()).build(window, cx))
+        .test_support()
+        .into_any_element()
+}
```

Change `crates/app/src/views/workspace/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/mod.rs b/crates/app/src/views/workspace/mod.rs
--- a/crates/app/src/views/workspace/mod.rs
+++ b/crates/app/src/views/workspace/mod.rs
@@ -1,27 +1,35 @@
-//! The workspace: schema sidebar, the query tab, and the status bar.
+//! The workspace: schema sidebar, query tabs, and the status bar.
 //!
 //! This module owns the workspace's state and its window commands;
+//! [`tabs`] opens, closes and switches tabs, [`drafts`] saves them,
 //! [`runs`] submits runs and reads the schema, and [`layout`] draws it.
 
+mod drafts;
 mod layout;
 mod runs;
+mod tab_strip;
+mod tabs;
 
-use std::time::Duration;
+use std::{collections::BTreeMap, sync::Arc, time::Duration};
 
 use gpui_kit::{
-    App, AppContext, Context, Entity, FocusHandle, Focusable, Task, Window,
+    App, AppContext, Context, Entity, FocusHandle, Focusable, Subscription, Task, Window,
     component::input::EditorState,
 };
 use quarry_engine::session::Session;
+use quarry_store::drafts::{DraftStore, Drafts};
 use quarry_ui_kit::Aurora;
 
-use super::{
-    editor,
-    query_tab::{QueryTab, QueryTabEvent},
-    schema::SchemaTree,
+pub use self::{
+    drafts::{AUTOSAVE, SavedTabs},
+    tab_strip::tab_element_id,
 };
+use super::{editor, query_tab::QueryTab, schema::SchemaTree};
 use crate::{
-    model::statements::RunOutput,
+    model::{
+        statements::RunOutput,
+        tabs::{Tab, TabId, TabList},
+    },
     shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
 };
 
@@ -32,57 +40,87 @@ pub const POLL: Duration = Duration::from_millis(33);
 pub const SIDEBAR_WIDTH: f32 = 240.0;
 
 /// The main window's content.
+///
+/// Every tab in `tabs` has a view in `views`.
 pub struct Workspace {
     session: Session,
     database: String,
-    tab: Entity<QueryTab>,
+    tabs: TabList,
+    views: BTreeMap<TabId, Entity<QueryTab>>,
+    store: Arc<dyn DraftStore>,
+    open_problem: Option<String>,
+    save_problem: Option<String>,
+    autosave: Option<Task<()>>,
+    saving: Option<Task<()>>,
     schema: Entity<SchemaTree>,
     sidebar_open: bool,
     focus: FocusHandle,
     sidebar_focus: FocusHandle,
     schema_read: Option<Task<()>>,
+    _quit: Subscription,
 }
 
 impl Workspace {
-    /// A workspace on `session`, labelled `database` in the status bar.
+    /// A workspace on `session`, labelled `database` in the status bar,
+    /// with the tabs saved in `saved`.
     pub fn new(
         session: Session,
         database: &str,
+        saved: SavedTabs,
         window: &mut Window,
         cx: &mut Context<Self>,
     ) -> Self {
-        let tab = cx.new(|cx| QueryTab::new(editor::WELCOME, window, cx));
-        cx.subscribe(&tab, |workspace, _, event, cx| match event {
-            QueryTabEvent::RunEnded => workspace.read_schema(cx),
-        })
-        .detach();
+        let (store, drafts, problem) = saved.open();
         let mut workspace = Self {
             session,
             database: database.to_owned(),
-            tab,
+            tabs: restored_tabs(&drafts),
+            views: BTreeMap::new(),
+            store,
+            open_problem: problem,
+            save_problem: None,
+            autosave: None,
+            saving: None,
             schema: cx.new(|_| SchemaTree::default()),
             sidebar_open: true,
             focus: cx.focus_handle(),
             sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
             schema_read: None,
+            _quit: cx.on_app_quit(|workspace, cx| {
+                workspace.save_before_quit(cx);
+                async {}
+            }),
         };
+        workspace.add_views(&drafts, window, cx);
         workspace.read_schema(cx);
+        Self::save_on_close(window.window_handle().window_id(), cx);
         workspace
     }
 
-    /// The active query tab.
-    pub fn active_tab(&self) -> &Entity<QueryTab> {
-        &self.tab
+    /// Creates a view per tab, with its saved text; a fresh start shows the
+    /// welcome query.
+    fn add_views(&mut self, drafts: &Drafts, window: &mut Window, cx: &mut Context<Self>) {
+        let ids: Vec<TabId> = self.tabs.tabs().iter().map(|tab| tab.id).collect();
+        for id in ids {
+            let saved = drafts.tabs.iter().find(|draft| draft.id == id);
+            let text = saved.map_or(editor::WELCOME, |draft| draft.text.as_str());
+            self.add_view(id, text, window, cx);
+        }
+    }
+
+    /// The open tabs and which is active.
+    pub fn tabs(&self) -> &TabList {
+        &self.tabs
     }
 
     /// The active tab's editor.
     pub fn editor(&self, cx: &App) -> Entity<EditorState> {
-        self.tab.read(cx).editor().clone()
+        self.active_tab().read(cx).editor().clone()
     }
 
     /// The active tab's latest run.
     pub fn run_output<'a>(&self, cx: &'a App) -> &'a RunOutput {
-        self.tab.read(cx).output(cx)
+        self.active_tab().read(cx).output(cx)
     }
 
     /// The sidebar's schema tree.
@@ -130,6 +168,19 @@ fn focus_first_stop(container: &FocusHandle, window: &mut Window, cx: &mut App)
     window.focus_next(cx);
 }
 
+/// The saved tabs as a [`TabList`]; none saved opens "Query 1".
+fn restored_tabs(drafts: &Drafts) -> TabList {
+    let tabs = drafts
+        .tabs
+        .iter()
+        .map(|draft| Tab {
+            id: draft.id,
+            title: draft.title.clone(),
+        })
+        .collect();
+    TabList::restore(tabs, drafts.active)
+}
+
 impl Focusable for Workspace {
     fn focus_handle(&self, _: &App) -> FocusHandle {
         self.focus.clone()
```

Change `crates/app/src/views/workspace/runs.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/runs.rs b/crates/app/src/views/workspace/runs.rs
--- a/crates/app/src/views/workspace/runs.rs
+++ b/crates/app/src/views/workspace/runs.rs
@@ -19,16 +19,17 @@ impl Workspace {
         _: &mut Window,
         cx: &mut Context<Self>,
     ) {
-        let Some(sql) = self.tab.read(cx).target_sql(cx) else {
+        let tab = self.active_tab().clone();
+        let Some(sql) = tab.read(cx).target_sql(cx) else {
             return;
         };
         let handle = self.session.run(sql.clone());
-        self.tab.update(cx, |tab, cx| tab.start(sql, handle, cx));
+        tab.update(cx, |tab, cx| tab.start(sql, handle, cx));
         cx.notify();
     }
 
     pub(super) fn cancel(&mut self, _: &CancelRun, _: &mut Window, cx: &mut Context<Self>) {
-        self.tab.read(cx).cancel();
+        self.active_tab().read(cx).cancel();
     }
 
     /// Reads the schema after any queued runs and shows it in the sidebar.
```

Create `crates/app/src/views/workspace/tab_strip.rs`:
```rust
//! The strip of query tabs along the title bar, with a "+" for a new tab.

use gpui_kit::{
    AnyElement, Context, ElementId, IntoElement, ParentElement, Styled, Window,
    prelude::FluentBuilder, px,
};
use quarry_ui_kit::{
    components::tabs::{Tab, tab_bar},
    tokens::chrome,
};

use super::Workspace;
use crate::{model::tabs::TabId, shell::actions::NewTab};

/// Room left of the tabs for the traffic lights when the sidebar is hidden.
pub const TRAFFIC_LIGHTS_CLEARANCE: f32 = 78.0;

/// The element id of tab `id`'s button, as in "tab-3".
pub fn tab_element_id(id: TabId) -> ElementId {
    ElementId::Name(format!("tab-{id}").into())
}

impl Workspace {
    pub(super) fn tab_strip(&self, cx: &Context<Self>) -> AnyElement {
        let active = self.tabs.active_id();
        let tabs = self.tabs.tabs().iter().map(|tab| {
            let id = tab.id;
            Tab::new(tab_element_id(id), tab.title.clone())
                .selected(id == active)
                .on_click(cx.listener(move |workspace, _, window, cx| {
                    workspace.activate_tab(id, window, cx);
                }))
        });
        let new_tab = Tab::new("new-tab", "+").on_click(|_, window: &mut Window, cx| {
            window.dispatch_action(Box::new(NewTab), cx);
        });
        tab_bar()
            .h(px(chrome::TITLEBAR_HEIGHT))
            .when(!self.sidebar_open, |strip| {
                strip.pl(px(TRAFFIC_LIGHTS_CLEARANCE))
            })
            .children(tabs)
            .child(new_tab)
            .into_any_element()
    }
}
```

Create `crates/app/src/views/workspace/tabs.rs`:
```rust
//! The workspace's query tabs: opening, closing and switching.

use gpui_kit::{AppContext, Context, Entity, Window};

use super::Workspace;
use crate::{
    model::tabs::TabId,
    shell::actions::{CloseTab, NewTab, NextTab, PreviousTab},
    views::query_tab::{QueryTab, QueryTabEvent},
};

impl Workspace {
    /// Creates the view for tab `id`, whose editor holds `text`.
    pub(super) fn add_view(
        &mut self,
        id: TabId,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.new(|cx| QueryTab::new(text, window, cx));
        cx.subscribe(&view, |workspace, _, event, cx| match event {
            QueryTabEvent::RunEnded => workspace.read_schema(cx),
            QueryTabEvent::Edited => workspace.schedule_save(cx),
        })
        .detach();
        self.views.insert(id, view);
    }

    /// The active tab's view.
    pub fn active_tab(&self) -> &Entity<QueryTab> {
        &self.views[&self.tabs.active_id()]
    }

    pub(super) fn new_tab(&mut self, _: &NewTab, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.tabs.open();
        self.add_view(id, "", window, cx);
        self.tabs_changed(window, cx);
    }

    pub(super) fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        let closing = self.tabs.active_id();
        let opened = self.tabs.close(closing);
        self.views.remove(&closing);
        if let Some(id) = opened {
            self.add_view(id, "", window, cx);
        }
        self.tabs_changed(window, cx);
    }

    pub(super) fn next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.next();
        self.tabs_changed(window, cx);
    }

    pub(super) fn previous_tab(
        &mut self,
        _: &PreviousTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tabs.previous();
        self.tabs_changed(window, cx);
    }

    /// Activates tab `id`, as a click on it does.
    pub fn activate_tab(&mut self, id: TabId, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.activate(id) {
            self.tabs_changed(window, cx);
        }
    }

    /// Saves, focuses the active tab's editor, and redraws.
    fn tabs_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_now(cx);
        self.editor(cx)
            .update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }
}
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every app test passes, including 11 in `tests/tabs.rs` and 6 in `tests/shell.rs`.

Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add -A crates/app/Cargo.toml crates/app/src/main.rs crates/app/src/shell/actions.rs crates/app/src/shell/mod.rs crates/app/src/shell/store.rs crates/app/src/shell/window.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/workspace/drafts.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/runs.rs crates/app/src/views/workspace/tab_strip.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/shell.rs crates/app/tests/tabs.rs crates/app/tests/workspace.rs
git commit -m "feat(app): open, close and switch query tabs whose drafts survive restarts"
```


---

### Task 7: Run all, result tabs, and the Results and Messages views

⌘⇧R runs the whole script. Each statement that returns rows gets a result tab (the strip shows once there are two), and the grid starts on the first. The views under the editor become a registry, `ResultViewRegistry`, switched with a segmented control; Messages lists what every statement did. A script whose statements return no rows opens on Messages. The GPUI tests move into one `tests/app` binary sharing `support.rs`, so their helpers aren't copied per file.

**Files:**
- Modify: `crates/app/src/shell/actions.rs`
- Modify: `crates/app/src/views/query_tab/layout.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Modify: `crates/app/src/views/query_tab/pump.rs`
- Move: `crates/app/src/views/results.rs` → `crates/app/src/views/results/grid.rs`
- Create: `crates/app/src/views/results/messages.rs`
- Create: `crates/app/src/views/results/mod.rs`
- Create: `crates/app/src/views/results/table.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/runs.rs`
- Create: `crates/app/tests/app/main.rs`
- Create: `crates/app/tests/app/runs.rs`
- Create: `crates/app/tests/app/support.rs`
- Move: `crates/app/tests/tabs.rs` → `crates/app/tests/app/tabs.rs`
- Move: `crates/app/tests/workspace.rs` → `crates/app/tests/app/workspace.rs`
- Modify: `crates/app/tests/shell.rs`

**Interfaces:**
- Consumes: Tasks 4–6.
- Produces: action `RunAll` (cmd-shift-r, Query menu); `views::query_tab::RunScope { Statement, Script }`, with `QueryTab::{start(String, RunScope, RunHandle, cx), follow(String, RunScope, Receiver<RunEvent>, cx), views, view_id, select_view(&str), select_view_at(usize), show_statement(usize)}`; `views::results::{ResultView { id, title, render }, ResultViewRegistry::{standard, views, position}, ResultsDelegate::show, statement_tab_id(usize)}`.

- [ ] **Step 1: Write the failing tests**

Create `crates/app/tests/app/main.rs`:
```rust
//! The app in headless windows: the workspace, its tabs, and its runs.
//!
//! One test binary, so the helpers in [`support`] are shared.

mod runs;
mod support;
mod tabs;
mod workspace;
```

Create `crates/app/tests/app/runs.rs`:
```rust
//! Run all, the Results and Messages views, and runs queued across tabs.

use gpui_kit::{TestAppContext, test::TestWindowExt};
use quarry::{
    model::{messages, statements::Phase},
    views::results::statement_tab_id,
};
use quarry_engine::error::EngineError;

use super::support::{drawn, open, phase, run_with, settle, write};

const MIXED: &str = "CREATE TABLE notes (body TEXT);
INSERT INTO notes VALUES ('a'), ('b');
SELECT body FROM notes;
SELECT count(*) FROM astronauts;";

#[gpui_kit::test]
fn run_all_gives_each_row_returning_statement_a_result_tab(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, MIXED);
    run_with(&workspace, cx, "cmd-shift-r");
    assert_eq!(phase(&workspace, cx), Phase::Finished);
    assert!(!drawn(cx, statement_tab_id(0)));
    assert!(!drawn(cx, statement_tab_id(1)));
    assert!(drawn(cx, statement_tab_id(2)));
    assert!(drawn(cx, statement_tab_id(3)));
    let shown = |cx: &mut gpui_kit::VisualTestContext| {
        cx.update(|_, cx| {
            let tab = workspace.read(cx).active_tab().read(cx);
            let grid = tab.results().read(cx);
            (tab.view_id(), grid.delegate().shown())
        })
    };
    assert_eq!(shown(cx), ("results", 2));
    cx.update(|window, cx| window.click(statement_tab_id(3), cx));
    assert_eq!(shown(cx), ("results", 3));
}

#[gpui_kit::test]
fn every_statement_gets_a_message(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, MIXED);
    run_with(&workspace, cx, "cmd-shift-r");
    let lines = cx.update(|_, cx| messages::lines(workspace.read(cx).run_output(cx)));
    let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    assert_eq!(texts.len(), 4);
    assert!(texts[0].starts_with("Done · "), "{texts:?}");
    assert!(texts[1].starts_with("2 rows affected · "), "{texts:?}");
    assert!(texts[2].starts_with("2 rows · "), "{texts:?}");
    assert!(texts[3].starts_with("1 row · "), "{texts:?}");
}

#[gpui_kit::test]
fn a_script_without_rows_opens_on_its_messages(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(
        &workspace,
        cx,
        "CREATE TABLE a (x); INSERT INTO a VALUES (1);",
    );
    run_with(&workspace, cx, "cmd-shift-r");
    cx.update(|_, cx| {
        assert_eq!(
            workspace.read(cx).active_tab().read(cx).view_id(),
            "messages"
        );
    });
    assert!(drawn(cx, "messages"));
}

#[gpui_kit::test]
fn run_all_stops_at_the_first_error(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT 1; SELECT * FROM nowhere; SELECT 2;");
    run_with(&workspace, cx, "cmd-shift-r");
    let error = EngineError::Sql("no such table: nowhere".to_owned());
    assert_eq!(phase(&workspace, cx), Phase::Failed(error));
    let lines = cx.update(|_, cx| messages::lines(workspace.read(cx).run_output(cx)));
    assert_eq!(lines.len(), 2);
    assert!(!lines[0].is_error && lines[1].is_error);
    assert!(drawn(cx, "run-failure"));
}

#[gpui_kit::test]
fn run_all_on_an_empty_script_does_nothing(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "-- nothing to run");
    cx.simulate_keystrokes("cmd-shift-r");
    assert_eq!(phase(&workspace, cx), Phase::Idle);
}

#[gpui_kit::test]
fn the_view_switch_shows_messages(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    assert!(drawn(cx, ("segment", 1_usize)));
    cx.update(|window, cx| window.click(("segment", 1_usize), cx));
    cx.update(|_, cx| {
        assert_eq!(
            workspace.read(cx).active_tab().read(cx).view_id(),
            "messages"
        );
    });
}

#[gpui_kit::test]
fn a_run_in_another_tab_waits_for_the_running_one(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(
        &workspace,
        cx,
        "SELECT count(*) FROM telemetry a, telemetry b",
    );
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| {
        workspace.read(cx).run_output(cx).phase() == &Phase::Running
    });
    cx.simulate_keystrokes("cmd-t");
    cx.simulate_input("SELECT 1");
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, _| true);
    assert_eq!(phase(&workspace, cx), Phase::Waiting);
    cx.simulate_keystrokes("cmd-{ cmd-.");
    cx.simulate_keystrokes("cmd-}");
    settle(cx, |_, cx| {
        workspace.read(cx).run_output(cx).phase() == &Phase::Finished
    });
    assert_eq!(phase(&workspace, cx), Phase::Finished);
}
```

Create `crates/app/tests/app/support.rs`:
```rust
//! Opening a workspace in a headless window and waiting for its runs.

use std::{sync::Arc, time::Duration};

use gpui_kit::{App, Entity, TestAppContext, VisualTestContext, Window, test::TestWindowExt};
use quarry::{
    model::statements::Phase,
    sample,
    shell::{actions, store::Forgetful},
    views::workspace::{POLL, SavedTabs, Workspace},
};
use quarry_engine::error::EngineError;
use quarry_store::drafts::DraftStore;
use quarry_ui_kit::{Aurora, Preference};

/// How long a test waits for the session thread before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

/// Initializes GPUI, Quarry's actions and the Dark theme.
pub fn install(cx: &mut TestAppContext) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
}

/// A workspace on a fresh sample database that keeps no drafts.
pub fn open(
    cx: &mut TestAppContext,
) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
    install(cx);
    open_with(cx, Arc::new(Forgetful))
}

/// A workspace on a fresh sample database saving to `store`, with its
/// editor focused. Call [`install`] first.
pub fn open_with(
    cx: &mut TestAppContext,
    store: Arc<dyn DraftStore>,
) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
    let session = sample::open()?;
    let saved = SavedTabs {
        store,
        problem: None,
    };
    let (workspace, cx) =
        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, saved, window, cx));
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| state.focus(window, cx));
    });
    settle(cx, |_, _| true);
    Ok((workspace, cx))
}

/// Replaces the editor's text and puts the cursor at the end.
pub fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &str) {
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| {
            state.set_value(sql.to_owned(), window, cx);
            state.set_selected_range(sql.len()..sql.len(), cx);
        });
    });
}

/// Pumps the executor until `done` holds or [`PATIENCE`] runs out; the
/// session thread delivers events in real time.
pub fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
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

/// Presses `keys` and waits for the active tab's run to end.
pub fn run_with(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    settle(cx, |_, cx| {
        let output = workspace.read(cx).run_output(cx);
        !output.is_running() && output.phase() != &Phase::Idle
    });
}

/// Runs the editor's statement with ⌘↵ and waits for the run to end.
pub fn run(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    run_with(workspace, cx, "cmd-enter");
}

/// The active tab's run phase.
pub fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
    cx.update(|_, cx| workspace.read(cx).run_output(cx).phase().clone())
}

/// Whether the element `id` is drawn, after a fresh draw.
pub fn drawn(cx: &mut VisualTestContext, id: impl Into<gpui_kit::ElementId>) -> bool {
    let id = id.into();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(id).is_some()
    })
}
```

Move `crates/app/tests/tabs.rs` to `crates/app/tests/app/tabs.rs` and change it by applying this patch with `git apply` (the patch does the move itself; don't `git mv` first):
```diff
diff --git a/crates/app/tests/tabs.rs b/crates/app/tests/app/tabs.rs
similarity index 81%
rename from crates/app/tests/tabs.rs
rename to crates/app/tests/app/tabs.rs
--- a/crates/app/tests/tabs.rs
+++ b/crates/app/tests/app/tabs.rs
@@ -1,31 +1,23 @@
 //! Query tabs: opening, closing and switching them, and keeping their
 //! drafts across restarts.
 
-use std::{
-    sync::{
-        Arc,
-        atomic::{AtomicBool, Ordering},
-    },
-    time::Duration,
+use std::sync::{
+    Arc,
+    atomic::{AtomicBool, Ordering},
 };
 
-use gpui_kit::{App, Entity, TestAppContext, VisualTestContext, Window, test::TestWindowExt};
+use gpui_kit::{Entity, TestAppContext, VisualTestContext, test::TestWindowExt};
 use quarry::{
     model::statements::Phase,
-    sample,
-    shell::actions,
-    views::workspace::{AUTOSAVE, POLL, SavedTabs, Workspace, tab_element_id},
+    views::workspace::{AUTOSAVE, Workspace, tab_element_id},
 };
-use quarry_engine::error::EngineError;
 use quarry_store::{
     drafts::{Draft, DraftStore, Drafts},
     error::StoreError,
     sqlite::SqliteStore,
 };
-use quarry_ui_kit::{Aurora, Preference};
 
-/// How long a test waits for the session thread before giving up.
-const PATIENCE: Duration = Duration::from_secs(10);
+use super::support::{drawn, install, open_with as open, settle};
 
 /// A store whose every read and write fails.
 struct Broken;
@@ -40,47 +32,6 @@ impl DraftStore for Broken {
     }
 }
 
-fn install(cx: &mut TestAppContext) {
-    let installed = cx.update(|cx| {
-        gpui_kit::init(cx);
-        actions::install(cx);
-        Aurora::install(Preference::Dark, cx)
-    });
-    assert_eq!(installed, Ok(()));
-}
-
-/// A workspace on a fresh sample database, saving to `store`.
-fn open(
-    cx: &mut TestAppContext,
-    store: Arc<dyn DraftStore>,
-) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
-    let session = sample::open()?;
-    let saved = SavedTabs {
-        store,
-        problem: None,
-    };
-    let (workspace, cx) =
-        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, saved, window, cx));
-    cx.update(|window, cx| {
-        let editor = workspace.read(cx).editor(cx);
-        editor.update(cx, |state, cx| state.focus(window, cx));
-    });
-    settle(cx, |_, _| true);
-    Ok((workspace, cx))
-}
-
-fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
-    let deadline = std::time::Instant::now() + PATIENCE;
-    while std::time::Instant::now() < deadline {
-        cx.executor().advance_clock(POLL);
-        cx.run_until_parked();
-        if cx.update(|window, cx| done(window, cx)) {
-            return;
-        }
-        std::thread::sleep(Duration::from_millis(5));
-    }
-}
-
 fn titles(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<String> {
     cx.update(|_, cx| {
         let tabs = workspace.read(cx).tabs().tabs();
@@ -245,11 +196,10 @@ fn an_edit_is_saved_only_after_a_pause(cx: &mut TestAppContext) {
 fn a_failing_store_shows_a_warning(cx: &mut TestAppContext) {
     install(cx);
     let (workspace, cx) = open(cx, Arc::new(Broken)).expect("the sample database opens");
-    cx.update(|window, cx| {
+    cx.update(|_, cx| {
         assert_eq!(workspace.read(cx).drafts_problem(), Some("disk I/O error"));
-        window.draw(cx).clear(cx);
-        assert!(window.try_find("drafts-warning").is_some());
     });
+    assert!(drawn(cx, "drafts-warning"));
 }
 
 /// A store whose load fails but whose save would succeed, noting any save.
```

Move `crates/app/tests/workspace.rs` to `crates/app/tests/app/workspace.rs` and change it by applying this patch with `git apply` (the patch does the move itself; don't `git mv` first):
```diff
diff --git a/crates/app/tests/workspace.rs b/crates/app/tests/app/workspace.rs
similarity index 73%
rename from crates/app/tests/workspace.rs
rename to crates/app/tests/app/workspace.rs
--- a/crates/app/tests/workspace.rs
+++ b/crates/app/tests/app/workspace.rs
@@ -1,89 +1,18 @@
 //! The workspace in a headless window: running, cancelling, the schema tree
 //! and the window commands.
 
-use std::{sync::Arc, time::Duration};
-
 use gpui_kit::{
-    App, Entity, Focusable, TestAppContext, VisualTestContext, Window,
+    Focusable, TestAppContext,
     test::{TestEventExt, TestWindowExt},
 };
 use quarry::{
     model::statements::Phase,
-    sample,
-    shell::{actions, store::Forgetful},
-    views::{
-        schema::table_row_id,
-        workspace::{POLL, SavedTabs, Workspace},
-    },
+    views::{query_tab::RunScope, schema::table_row_id},
 };
 use quarry_engine::{error::EngineError, session::RunEvent};
 use quarry_ui_kit::{Aurora, Preference};
 
-/// How long a test waits for the session thread before giving up.
-const PATIENCE: Duration = Duration::from_secs(10);
-
-/// A workspace on a fresh sample database, with its editor focused.
-fn open(
-    cx: &mut TestAppContext,
-) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
-    let installed = cx.update(|cx| {
-        gpui_kit::init(cx);
-        actions::install(cx);
-        Aurora::install(Preference::Dark, cx)
-    });
-    assert_eq!(installed, Ok(()));
-    let session = sample::open()?;
-    let saved = SavedTabs {
-        store: Arc::new(Forgetful),
-        problem: None,
-    };
-    let (workspace, cx) =
-        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, saved, window, cx));
-    cx.update(|window, cx| {
-        let editor = workspace.read(cx).editor(cx);
-        editor.update(cx, |state, cx| state.focus(window, cx));
-    });
-    settle(cx, |_, _| true);
-    Ok((workspace, cx))
-}
-
-/// Replaces the editor's text and puts the cursor at the end.
-fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &str) {
-    cx.update(|window, cx| {
-        let editor = workspace.read(cx).editor(cx);
-        editor.update(cx, |state, cx| {
-            state.set_value(sql.to_owned(), window, cx);
-            state.set_selected_range(sql.len()..sql.len(), cx);
-        });
-    });
-}
-
-/// Pumps the executor until `done` holds or [`PATIENCE`] runs out; the
-/// session thread delivers events in real time.
-fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
-    let deadline = std::time::Instant::now() + PATIENCE;
-    while std::time::Instant::now() < deadline {
-        cx.executor().advance_clock(POLL);
-        cx.run_until_parked();
-        if cx.update(|window, cx| done(window, cx)) {
-            return;
-        }
-        std::thread::sleep(Duration::from_millis(5));
-    }
-}
-
-/// Runs the editor's statement with ⌘↵ and waits for the run to end.
-fn run(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
-    cx.simulate_keystrokes("cmd-enter");
-    settle(cx, |_, cx| {
-        let output = workspace.read(cx).run_output(cx);
-        !output.is_running() && output.phase() != &Phase::Idle
-    });
-}
-
-fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
-    cx.update(|_, cx| workspace.read(cx).run_output(cx).phase().clone())
-}
+use super::support::{drawn, open, phase, run, settle, write};
 
 #[gpui_kit::test]
 fn the_editor_never_has_a_completion_provider(cx: &mut TestAppContext) {
@@ -143,10 +72,7 @@ fn a_bad_statement_shows_the_database_error(cx: &mut TestAppContext) {
         phase(&workspace, cx),
         Phase::Failed(EngineError::Sql("no such table: nowhere".to_owned()))
     );
-    cx.update(|window, cx| {
-        window.draw(cx).clear(cx);
-        assert!(window.try_find("run-failure").is_some());
-    });
+    assert!(drawn(cx, "run-failure"));
 }
 
 #[gpui_kit::test]
@@ -282,17 +208,16 @@ fn a_run_whose_channel_closes_without_done_reports_a_lost_worker(cx: &mut TestAp
     drop(sender);
     cx.update(|_, cx| {
         let tab = workspace.read(cx).active_tab().clone();
-        tab.update(cx, |tab, cx| tab.follow("SELECT 1".to_owned(), events, cx));
+        tab.update(cx, |tab, cx| {
+            tab.follow("SELECT 1".to_owned(), RunScope::Statement, events, cx);
+        });
     });
     settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
     assert_eq!(
         phase(&workspace, cx),
         Phase::Failed(EngineError::WorkerGone)
     );
-    cx.update(|window, cx| {
-        window.draw(cx).clear(cx);
-        assert!(window.try_find("run-failure").is_some());
-    });
+    assert!(drawn(cx, "run-failure"));
 }
 
 #[gpui_kit::test]
```

Change `crates/app/tests/shell.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/shell.rs b/crates/app/tests/shell.rs
--- a/crates/app/tests/shell.rs
+++ b/crates/app/tests/shell.rs
@@ -49,7 +49,7 @@ fn every_menu_command_has_a_key_binding() {
             _ => None,
         })
         .collect();
-    assert_eq!(commands.len(), 11);
+    assert_eq!(commands.len(), 12);
     for command in commands {
         assert!(bound.contains(&command), "{command} has no key binding");
     }
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry`
Expected: `unresolved import quarry::views::results::statement_tab_id`.

- [ ] **Step 3: Implement**

Change `crates/app/src/shell/actions.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/shell/actions.rs b/crates/app/src/shell/actions.rs
--- a/crates/app/src/shell/actions.rs
+++ b/crates/app/src/shell/actions.rs
@@ -10,6 +10,8 @@ actions!(
     [
         /// Runs the selection, or the statement under the cursor.
         RunStatement,
+        /// Runs the whole script, statement by statement.
+        RunAll,
         /// Cancels the running query.
         CancelRun,
         /// Opens a new query tab.
@@ -45,6 +47,7 @@ pub fn key_bindings() -> Vec<KeyBinding> {
     vec![
         KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE)),
         KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
+        KeyBinding::new("cmd-shift-r", RunAll, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
         KeyBinding::new("cmd-t", NewTab, None),
@@ -69,6 +72,7 @@ pub fn menus() -> Vec<Menu> {
         ]),
         Menu::new("Query").items([
             MenuItem::action("Run Statement", RunStatement),
+            MenuItem::action("Run All", RunAll),
             MenuItem::action("Cancel", CancelRun),
         ]),
         Menu::new("View").items([
```

Change `crates/app/src/views/query_tab/layout.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/query_tab/layout.rs b/crates/app/src/views/query_tab/layout.rs
--- a/crates/app/src/views/query_tab/layout.rs
+++ b/crates/app/src/views/query_tab/layout.rs
@@ -6,13 +6,12 @@ use gpui_kit::{
     component::{
         input::Editor,
         resizable::{resizable_panel, v_resizable},
-        table::DataTable,
     },
     div, px,
 };
 use quarry_ui_kit::{
     Aurora, Role,
-    components::surface::glass_panel,
+    components::{segmented::SegmentedControl, surface::glass_panel},
     tokens::{space, text},
 };
 
@@ -23,7 +22,7 @@ use crate::model::status::{self, Failure};
 pub const EDITOR_HEIGHT: f32 = 220.0;
 
 impl Render for QueryTab {
-    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
+    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
         let colors = Aurora::colors(cx);
         let editor = glass_panel(&colors)
             .size_full()
@@ -31,14 +30,15 @@ impl Render for QueryTab {
             .child(Editor::new(&self.editor).appearance(false).h_full());
         v_resizable("editor-results")
             .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
-            .child(resizable_panel().child(self.results_panel(cx)))
+            .child(resizable_panel().child(self.results_panel(window, cx)))
     }
 }
 
 impl QueryTab {
-    fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
+    fn results_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
         let colors = Aurora::colors(cx);
         let failure = status::failure(self.output(cx));
+        let view = self.views.views()[self.view].render(self, window, cx);
         glass_panel(&colors)
             .id("results")
             .size_full()
@@ -46,12 +46,28 @@ impl QueryTab {
             .flex()
             .flex_col()
             .overflow_hidden()
+            .child(self.view_switch(cx))
             .children(failure.map(|failure| failure_banner(failure, cx)))
-            .child(
-                div()
-                    .flex_1()
-                    .child(DataTable::new(&self.results).bordered(false)),
-            )
+            .child(div().flex_1().child(view))
+            .into_any_element()
+    }
+
+    /// Results | Messages | Plan.
+    fn view_switch(&self, cx: &mut Context<Self>) -> AnyElement {
+        let titles = self.views.views().iter().map(|view| view.title());
+        let this = cx.entity().downgrade();
+        let switch = SegmentedControl::new("result-views", titles)
+            .selected(self.view)
+            .on_change(move |index, _, cx| {
+                let _ = this.update(cx, |tab, cx| tab.select_view_at(index, cx));
+            });
+        div()
+            .flex()
+            .px(px(space::SM))
+            .py(px(space::XS))
+            .border_b_1()
+            .border_color(Aurora::colors(cx).get(Role::Border))
+            .child(switch)
             .into_any_element()
     }
 }
```

Change `crates/app/src/views/query_tab/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/query_tab/mod.rs b/crates/app/src/views/query_tab/mod.rs
--- a/crates/app/src/views/query_tab/mod.rs
+++ b/crates/app/src/views/query_tab/mod.rs
@@ -17,7 +17,10 @@ use gpui_kit::{
 };
 use quarry_engine::session::{RunEvent, RunHandle};
 
-use super::{editor, results::ResultsDelegate};
+use super::{
+    editor,
+    results::{ResultViewRegistry, ResultsDelegate},
+};
 use crate::model::{statements::RunOutput, target};
 
 /// What a tab tells the workspace.
@@ -29,10 +32,22 @@ pub enum QueryTabEvent {
     Edited,
 }
 
+/// What a run was asked to run.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum RunScope {
+    /// The selection, or the statement under the cursor (⌘↵).
+    Statement,
+    /// The whole editor (⌘⇧R).
+    Script,
+}
+
 /// A query tab.
 pub struct QueryTab {
     editor: Entity<EditorState>,
     results: Entity<TableState<ResultsDelegate>>,
+    views: ResultViewRegistry,
+    view: usize,
+    scope: RunScope,
     run: Option<RunHandle>,
     pump: Option<Task<()>>,
     _edits: Subscription,
@@ -56,6 +71,9 @@ impl QueryTab {
         Self {
             editor,
             results,
+            views: ResultViewRegistry::standard(),
+            view: 0,
+            scope: RunScope::Statement,
             run: None,
             pump: None,
             _edits: edits,
@@ -91,14 +109,29 @@ impl QueryTab {
     }
 
     /// Shows the run of `script` that `handle` follows, replacing the last.
-    pub fn start(&mut self, script: String, handle: RunHandle, cx: &mut Context<Self>) {
-        self.follow(script, handle.events.clone(), cx);
+    pub fn start(
+        &mut self,
+        script: String,
+        scope: RunScope,
+        handle: RunHandle,
+        cx: &mut Context<Self>,
+    ) {
+        self.follow(script, scope, handle.events.clone(), cx);
         self.run = Some(handle);
     }
 
-    /// Shows the run of `script` whose events arrive on `events`.
-    pub fn follow(&mut self, script: String, events: Receiver<RunEvent>, cx: &mut Context<Self>) {
+    /// Shows the run of `script` whose events arrive on `events`, in the
+    /// Results view.
+    pub fn follow(
+        &mut self,
+        script: String,
+        scope: RunScope,
+        events: Receiver<RunEvent>,
+        cx: &mut Context<Self>,
+    ) {
         self.run = None;
+        self.scope = scope;
+        self.select_view("results", cx);
         self.results.update(cx, |table, cx| {
             table.delegate_mut().reset(RunOutput::submitted(script));
             table.clear_selection(cx);
@@ -109,6 +142,41 @@ impl QueryTab {
         cx.notify();
     }
 
+    /// The views under the editor.
+    pub fn views(&self) -> &ResultViewRegistry {
+        &self.views
+    }
+
+    /// The id of the view shown under the editor.
+    pub fn view_id(&self) -> &'static str {
+        self.views.views()[self.view].id()
+    }
+
+    /// Shows the view named `id`; an unknown id changes nothing.
+    pub fn select_view(&mut self, id: &str, cx: &mut Context<Self>) {
+        if let Some(position) = self.views.position(id) {
+            self.select_view_at(position, cx);
+        }
+    }
+
+    /// Shows the view at `position` in the switch; past the end changes
+    /// nothing.
+    pub fn select_view_at(&mut self, position: usize, cx: &mut Context<Self>) {
+        if position < self.views.views().len() {
+            self.view = position;
+            cx.notify();
+        }
+    }
+
+    /// Shows statement `index`'s rows in the grid.
+    pub fn show_statement(&mut self, index: usize, cx: &mut Context<Self>) {
+        self.results.update(cx, |table, cx| {
+            table.delegate_mut().show(index);
+            table.refresh(cx);
+            cx.notify();
+        });
+    }
+
     /// Cancels the tab's run, if one is going.
     pub fn cancel(&self) {
         if let Some(handle) = &self.run {
```

Change `crates/app/src/views/query_tab/pump.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/query_tab/pump.rs b/crates/app/src/views/query_tab/pump.rs
--- a/crates/app/src/views/query_tab/pump.rs
+++ b/crates/app/src/views/query_tab/pump.rs
@@ -10,9 +10,12 @@ use async_channel::Receiver;
 use gpui_kit::{Context, Task};
 use quarry_engine::session::RunEvent;
 
-use super::{QueryTab, QueryTabEvent};
+use super::{QueryTab, QueryTabEvent, RunScope};
 use crate::{
-    model::drain::{Drained, drain},
+    model::{
+        drain::{Drained, drain},
+        statements::StatementOutput,
+    },
     views::workspace::POLL,
 };
 
@@ -38,13 +41,26 @@ impl QueryTab {
         let ended = drained.closed || done;
         self.apply(drained.items, vanished, cx);
         if ended {
-            self.run = None;
-            cx.emit(QueryTabEvent::RunEnded);
+            self.run_ended(cx);
         }
         cx.notify();
         !ended
     }
 
+    /// A script whose statements returned no rows opens on its messages.
+    fn run_ended(&mut self, cx: &mut Context<Self>) {
+        self.run = None;
+        let rows = self
+            .output(cx)
+            .statements()
+            .iter()
+            .any(StatementOutput::returns_rows);
+        if self.scope == RunScope::Script && !rows {
+            self.select_view("messages", cx);
+        }
+        cx.emit(QueryTabEvent::RunEnded);
+    }
+
     /// Folds the events in; a channel that closed without `Done` ends the run
     /// as a vanished worker, unless an earlier event already failed it.
     fn apply(&mut self, events: Vec<RunEvent>, vanished: bool, cx: &mut Context<Self>) {
```

Move `crates/app/src/views/results.rs` to `crates/app/src/views/results/grid.rs` and change it by applying this patch with `git apply` (the patch does the move itself; don't `git mv` first):
```diff
diff --git a/crates/app/src/views/results.rs b/crates/app/src/views/results/grid.rs
similarity index 86%
rename from crates/app/src/views/results.rs
rename to crates/app/src/views/results/grid.rs
--- a/crates/app/src/views/results.rs
+++ b/crates/app/src/views/results/grid.rs
@@ -1,4 +1,4 @@
-//! The results grid: one statement's rows in a virtualized table.
+//! The grid: one statement's rows in a virtualized table.
 
 use gpui_kit::{
     AnyElement, App, Context, IntoElement, ParentElement, SharedString, Styled, Window,
@@ -54,18 +54,25 @@ impl ResultsDelegate {
         self.output.end_closed();
     }
 
-    /// Folds `event` in, showing each statement as it starts; returns
-    /// whether the table's columns changed.
+    /// Folds `event` in; returns whether the table's columns changed.
+    ///
+    /// The grid shows the first statement that returns rows: until one
+    /// does, each statement's columns take over the grid as they arrive.
     pub fn apply(&mut self, event: RunEvent, now: Instant) -> bool {
-        if let RunEvent::Started { index, .. } = &event {
-            self.shown = *index;
-        }
-        let reshapes = matches!(event, RunEvent::Started { .. } | RunEvent::Columns(_));
+        let columns = matches!(event, RunEvent::Columns(_));
         self.output.apply(event, now);
-        if reshapes {
-            self.columns = self.shown_columns();
+        if !columns || !self.columns.is_empty() {
+            return false;
         }
-        reshapes
+        self.shown = self.output.statements().len().saturating_sub(1);
+        self.columns = self.shown_columns();
+        true
+    }
+
+    /// Shows statement `index`.
+    pub fn show(&mut self, index: usize) {
+        self.shown = index;
+        self.columns = self.shown_columns();
     }
 
     fn shown_columns(&self) -> Vec<Column> {
```

Create `crates/app/src/views/results/messages.rs`:
```rust
//! The Messages view: what each statement of the run did.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{MONO_FONT, space, text},
};

use super::ResultView;
use crate::{
    model::messages::{self, MessageLine},
    views::query_tab::QueryTab,
};

/// The width of the statement labels' column.
pub const LABEL_WIDTH: f32 = 240.0;

/// One line per statement: rows, rows affected, or the error.
pub struct MessagesView;

impl ResultView for MessagesView {
    fn id(&self) -> &'static str {
        "messages"
    }

    fn title(&self) -> &'static str {
        "Messages"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let lines = messages::lines(tab.output(cx));
        let body = if lines.is_empty() {
            vec![
                div()
                    .text_color(colors.get(Role::TextMuted))
                    .child("Run a script with ⌘⇧R to see what each statement did.")
                    .into_any_element(),
            ]
        } else {
            lines.into_iter().map(|line| message(line, cx)).collect()
        };
        div()
            .id("messages")
            .size_full()
            .overflow_y_scroll()
            .p(px(space::MD))
            .flex()
            .flex_col()
            .gap(px(space::XS))
            .text_size(px(text::BODY))
            .children(body)
            .test_support()
            .into_any_element()
    }
}

/// "2 · INSERT INTO crew…   3 rows affected · 1.2 ms", the error in red.
fn message(line: MessageLine, cx: &Context<QueryTab>) -> AnyElement {
    let colors = Aurora::colors(cx);
    let role = if line.is_error {
        Role::Danger
    } else {
        Role::Text
    };
    div()
        .flex()
        .gap(px(space::MD))
        .child(
            div()
                .w(px(LABEL_WIDTH))
                .flex_none()
                .font_family(MONO_FONT)
                .text_color(colors.get(Role::TextMuted))
                .child(line.label),
        )
        .child(div().text_color(colors.get(role)).child(line.text))
        .into_any_element()
}
```

Create `crates/app/src/views/results/mod.rs`:
```rust
//! The views under the editor: Results, Messages and Plan.
//!
//! Each is a [`ResultView`] in the [`ResultViewRegistry`]; adding a view is
//! a new module and one line in [`ResultViewRegistry::standard`].

mod grid;
mod messages;
mod table;

use gpui_kit::{AnyElement, Context, Window};

pub use self::{
    grid::{COLUMN_WIDTH, ResultsDelegate},
    table::statement_tab_id,
};
use super::query_tab::QueryTab;

/// One view of a tab's run.
pub trait ResultView {
    /// A stable name, such as "messages".
    fn id(&self) -> &'static str;

    /// The label on the view switch.
    fn title(&self) -> &'static str;

    /// Draws the view for `tab`.
    fn render(&self, tab: &QueryTab, window: &mut Window, cx: &mut Context<QueryTab>)
    -> AnyElement;
}

/// The views a tab can switch between, in switch order.
pub struct ResultViewRegistry {
    views: Vec<Box<dyn ResultView>>,
}

impl ResultViewRegistry {
    /// Results, then Messages.
    pub fn standard() -> Self {
        Self {
            views: vec![
                Box::new(table::ResultsView),
                Box::new(messages::MessagesView),
            ],
        }
    }

    /// Every view, in switch order.
    pub fn views(&self) -> &[Box<dyn ResultView>] {
        &self.views
    }

    /// The position of the view named `id`.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.views.iter().position(|view| view.id() == id)
    }
}
```

Create `crates/app/src/views/results/table.rs`:
```rust
//! The Results view: a strip of statement tabs over the grid.

use gpui_kit::{
    AnyElement, Context, ElementId, IntoElement, ParentElement, Styled, Window,
    component::table::DataTable, div,
};
use quarry_ui_kit::components::tabs::{Tab, tab_bar};

use super::ResultView;
use crate::views::query_tab::QueryTab;

/// The grid of the statement chosen in the strip above it.
pub struct ResultsView;

/// The element id of statement `index`'s result tab, as in "statement-2".
pub fn statement_tab_id(index: usize) -> ElementId {
    ElementId::Name(format!("statement-{index}").into())
}

impl ResultView for ResultsView {
    fn id(&self) -> &'static str {
        "results"
    }

    fn title(&self) -> &'static str {
        "Results"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .children(statement_strip(tab, cx))
            .child(
                div()
                    .flex_1()
                    .child(DataTable::new(tab.results()).bordered(false)),
            )
            .into_any_element()
    }
}

/// One tab per statement that returns rows, when there is more than one.
fn statement_strip(tab: &QueryTab, cx: &mut Context<QueryTab>) -> Option<AnyElement> {
    let shown = tab.results().read(cx).delegate().shown();
    let tabs: Vec<Tab> = tab
        .output(cx)
        .statements()
        .iter()
        .filter(|statement| statement.returns_rows())
        .map(|statement| {
            let index = statement.index();
            Tab::new(statement_tab_id(index), statement.label().to_owned())
                .selected(index == shown)
                .on_click(cx.listener(move |tab, _, _, cx| tab.show_statement(index, cx)))
        })
        .collect();
    (tabs.len() > 1).then(|| tab_bar().children(tabs).into_any_element())
}
```

Change `crates/app/src/views/workspace/layout.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/layout.rs b/crates/app/src/views/workspace/layout.rs
--- a/crates/app/src/views/workspace/layout.rs
+++ b/crates/app/src/views/workspace/layout.rs
@@ -33,6 +33,7 @@ impl Render for Workspace {
             .key_context(WORKSPACE)
             .track_focus(&self.focus)
             .on_action(cx.listener(Self::run_statement))
+            .on_action(cx.listener(Self::run_all))
             .on_action(cx.listener(Self::cancel))
             .on_action(cx.listener(Self::new_tab))
             .on_action(cx.listener(Self::close_tab))
```

Change `crates/app/src/views/workspace/runs.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/runs.rs b/crates/app/src/views/workspace/runs.rs
--- a/crates/app/src/views/workspace/runs.rs
+++ b/crates/app/src/views/workspace/runs.rs
@@ -4,12 +4,13 @@
 //! [`crate::model::drain`]).
 
 use gpui_kit::{Context, Window};
-use quarry_engine::{error::EngineError, schema::Schema};
+use quarry_engine::{error::EngineError, schema::Schema, splitter};
 
 use super::{POLL, Workspace};
 use crate::{
     model::drain::{Drained, drain},
-    shell::actions::{CancelRun, RunStatement},
+    shell::actions::{CancelRun, RunAll, RunStatement},
+    views::query_tab::RunScope,
 };
 
 impl Workspace {
@@ -24,7 +25,23 @@ impl Workspace {
             return;
         };
         let handle = self.session.run(sql.clone());
-        tab.update(cx, |tab, cx| tab.start(sql, handle, cx));
+        tab.update(cx, |tab, cx| {
+            tab.start(sql, RunScope::Statement, handle, cx);
+        });
+        cx.notify();
+    }
+
+    /// Runs the active tab's whole script, statement by statement.
+    pub(super) fn run_all(&mut self, _: &RunAll, _: &mut Window, cx: &mut Context<Self>) {
+        let tab = self.active_tab().clone();
+        let script = tab.read(cx).text(cx);
+        if splitter::split(&script).is_empty() {
+            return;
+        }
+        let handle = self.session.run(script.clone());
+        tab.update(cx, |tab, cx| {
+            tab.start(script, RunScope::Script, handle, cx);
+        });
         cx.notify();
     }
 
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every app test passes; `tests/app` runs 33 tests.

Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add -A crates/app/src/shell/actions.rs crates/app/src/views/query_tab/layout.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/query_tab/pump.rs crates/app/src/views/results.rs crates/app/src/views/results/grid.rs crates/app/src/views/results/messages.rs crates/app/src/views/results/mod.rs crates/app/src/views/results/table.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/runs.rs crates/app/tests/app/main.rs crates/app/tests/app/runs.rs crates/app/tests/app/support.rs crates/app/tests/app/tabs.rs crates/app/tests/app/workspace.rs crates/app/tests/shell.rs crates/app/tests/tabs.rs crates/app/tests/workspace.rs
git commit -m "feat(app): run whole scripts with a result tab per statement and a Messages view"
```


---

### Task 8: Explain and the Plan view

⌘E asks the session for the plan of the selection or the statement under the cursor and shows it in the Plan view: one step per line, indented by nesting, in Menlo. A plan error, or a driver without plans, shows in the view instead of the run banner. Explain runs nothing. The schema read and Explain share `drain::reply`, which turns a drained one-off reply into its answer or `WorkerGone`.

**Files:**
- Modify: `crates/app/src/model/drain.rs`
- Modify: `crates/app/src/model/plan_tree.rs`
- Modify: `crates/app/src/shell/actions.rs`
- Create: `crates/app/src/views/query_tab/explain.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Modify: `crates/app/src/views/results/mod.rs`
- Create: `crates/app/src/views/results/plan.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/runs.rs`
- Modify: `crates/app/tests/app/runs.rs`
- Modify: `crates/app/tests/shell.rs`

**Interfaces:**
- Consumes: Tasks 2, 4, 7.
- Produces: action `Explain` (cmd-e, Query menu); `model::plan_tree::PlanState { Idle, Planning, Ready(Vec<PlanRow>), Failed(String) }` with `PlanState::answered`; `model::drain::reply`; `QueryTab::{explain(Receiver<Result<Plan, EngineError>>, cx), plan}`; the Plan view (`views::results::plan`, id "plan").

- [ ] **Step 1: Write the failing tests**

Change `crates/app/tests/app/runs.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/app/runs.rs b/crates/app/tests/app/runs.rs
--- a/crates/app/tests/app/runs.rs
+++ b/crates/app/tests/app/runs.rs
@@ -1,8 +1,12 @@
-//! Run all, the Results and Messages views, and runs queued across tabs.
+//! Run all, Explain, the result views, and runs queued across tabs.
 
 use gpui_kit::{TestAppContext, test::TestWindowExt};
 use quarry::{
-    model::{messages, statements::Phase},
+    model::{
+        messages,
+        plan_tree::{PlanRow, PlanState},
+        statements::Phase,
+    },
     views::results::statement_tab_id,
 };
 use quarry_engine::error::EngineError;
@@ -126,3 +130,50 @@ fn a_run_in_another_tab_waits_for_the_running_one(cx: &mut TestAppContext) {
     });
     assert_eq!(phase(&workspace, cx), Phase::Finished);
 }
+
+#[gpui_kit::test]
+fn command_e_shows_the_statement_plan(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    write(&workspace, cx, "SELECT name FROM astronauts WHERE id = 3");
+    cx.simulate_keystrokes("cmd-e");
+    let plan = |cx: &mut gpui_kit::VisualTestContext| {
+        cx.update(|_, cx| workspace.read(cx).active_tab().read(cx).plan().clone())
+    };
+    assert_eq!(plan(cx), PlanState::Planning);
+    settle(cx, |_, cx| {
+        matches!(
+            workspace.read(cx).active_tab().read(cx).plan(),
+            PlanState::Ready(_)
+        )
+    });
+    let rows = vec![PlanRow {
+        depth: 0,
+        label: "SEARCH astronauts USING INTEGER PRIMARY KEY (rowid=?)".to_owned(),
+    }];
+    assert_eq!(plan(cx), PlanState::Ready(rows));
+    cx.update(|_, cx| {
+        assert_eq!(workspace.read(cx).active_tab().read(cx).view_id(), "plan");
+    });
+    assert!(drawn(cx, "plan"));
+}
+
+#[gpui_kit::test]
+fn a_statement_that_cannot_be_planned_says_why(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    write(&workspace, cx, "SELECT * FROM nowhere");
+    cx.simulate_keystrokes("cmd-e");
+    settle(cx, |_, cx| {
+        matches!(
+            workspace.read(cx).active_tab().read(cx).plan(),
+            PlanState::Failed(_)
+        )
+    });
+    cx.update(|_, cx| {
+        let tab = workspace.read(cx).active_tab().read(cx);
+        assert_eq!(
+            tab.plan(),
+            &PlanState::Failed("no such table: nowhere".to_owned())
+        );
+        assert_eq!(tab.output(cx).phase(), &Phase::Idle, "explain runs nothing");
+    });
+}
```

Change `crates/app/tests/shell.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/shell.rs b/crates/app/tests/shell.rs
--- a/crates/app/tests/shell.rs
+++ b/crates/app/tests/shell.rs
@@ -49,7 +49,7 @@ fn every_menu_command_has_a_key_binding() {
             _ => None,
         })
         .collect();
-    assert_eq!(commands.len(), 12);
+    assert_eq!(commands.len(), 13);
     for command in commands {
         assert!(bound.contains(&command), "{command} has no key binding");
     }
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry`
Expected: `no method named plan` / `unresolved import quarry::model::plan_tree::PlanState`.

- [ ] **Step 3: Implement**

Change `crates/app/src/model/drain.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/model/drain.rs b/crates/app/src/model/drain.rs
--- a/crates/app/src/model/drain.rs
+++ b/crates/app/src/model/drain.rs
@@ -6,6 +6,7 @@
 //! redraw.
 
 use async_channel::{Receiver, TryRecvError};
+use quarry_engine::error::EngineError;
 
 /// What one poll found.
 #[derive(Debug, PartialEq)]
@@ -38,9 +39,20 @@ pub fn drain<T>(receiver: &Receiver<T>) -> Drained<T> {
     }
 }
 
+/// A one-off reply (a schema or a plan), once it has come: the reply
+/// itself, or `WorkerGone` when the channel closed without one.
+pub fn reply<T>(drained: Drained<Result<T, EngineError>>) -> Option<Result<T, EngineError>> {
+    let Drained { mut items, closed } = drained;
+    items
+        .pop()
+        .or_else(|| closed.then_some(Err(EngineError::WorkerGone)))
+}
+
 #[cfg(test)]
 mod tests {
-    use super::{Drained, drain};
+    use quarry_engine::error::EngineError;
+
+    use super::{Drained, drain, reply};
 
     #[test]
     fn takes_everything_waiting_and_stays_open() {
@@ -76,4 +88,23 @@ mod tests {
             }
         );
     }
+
+    #[test]
+    fn a_reply_is_its_last_item_or_a_lost_worker() {
+        let waiting: Drained<Result<u8, EngineError>> = Drained {
+            items: vec![],
+            closed: false,
+        };
+        assert_eq!(reply(waiting), None);
+        let came = Drained {
+            items: vec![Ok(1), Ok(2)],
+            closed: true,
+        };
+        assert_eq!(reply(came), Some(Ok(2)));
+        let gone: Drained<Result<u8, EngineError>> = Drained {
+            items: vec![],
+            closed: true,
+        };
+        assert_eq!(reply(gone), Some(Err(EngineError::WorkerGone)));
+    }
 }
```

Change `crates/app/src/model/plan_tree.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/model/plan_tree.rs b/crates/app/src/model/plan_tree.rs
--- a/crates/app/src/model/plan_tree.rs
+++ b/crates/app/src/model/plan_tree.rs
@@ -1,6 +1,9 @@
 //! The Plan view's rows: a query plan flattened with each step's depth.
 
-use quarry_engine::plan::{Plan, PlanNode};
+use quarry_engine::{
+    error::EngineError,
+    plan::{Plan, PlanNode},
+};
 
 /// One step of a plan, indented by `depth`.
 #[derive(Debug, Clone, PartialEq, Eq)]
@@ -11,6 +14,30 @@ pub struct PlanRow {
     pub label: String,
 }
 
+/// What the Plan view shows.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub enum PlanState {
+    /// Nothing has been explained yet.
+    #[default]
+    Idle,
+    /// The plan has been asked for.
+    Planning,
+    /// The plan's steps.
+    Ready(Vec<PlanRow>),
+    /// Why there is no plan.
+    Failed(String),
+}
+
+impl PlanState {
+    /// The state for the session's answer.
+    pub fn answered(answer: Result<Plan, EngineError>) -> Self {
+        match answer {
+            Ok(plan) => Self::Ready(rows(&plan)),
+            Err(error) => Self::Failed(error.to_string()),
+        }
+    }
+}
+
 /// Every step, depth first, in the database's order.
 pub fn rows(plan: &Plan) -> Vec<PlanRow> {
     let mut rows = Vec::new();
@@ -30,9 +57,12 @@ fn push(nodes: &[PlanNode], depth: usize, rows: &mut Vec<PlanRow>) {
 
 #[cfg(test)]
 mod tests {
-    use quarry_engine::plan::{Plan, PlanNode};
+    use quarry_engine::{
+        error::EngineError,
+        plan::{Plan, PlanNode},
+    };
 
-    use super::{PlanRow, rows};
+    use super::{PlanRow, PlanState, rows};
 
     fn node(label: &str, children: Vec<PlanNode>) -> PlanNode {
         PlanNode {
@@ -74,4 +104,20 @@ mod tests {
     fn an_empty_plan_has_no_rows() {
         assert!(rows(&Plan::default()).is_empty());
     }
+
+    #[test]
+    fn an_answer_becomes_rows_or_a_reason() {
+        let plan = Plan {
+            nodes: vec![node("SCAN t", vec![])],
+        };
+        assert_eq!(
+            PlanState::answered(Ok(plan)),
+            PlanState::Ready(vec![row(0, "SCAN t")])
+        );
+        assert_eq!(
+            PlanState::answered(Err(EngineError::PlanUnsupported)),
+            PlanState::Failed("This database can't show query plans.".to_owned())
+        );
+        assert_eq!(PlanState::default(), PlanState::Idle);
+    }
 }
```

Change `crates/app/src/shell/actions.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/shell/actions.rs b/crates/app/src/shell/actions.rs
--- a/crates/app/src/shell/actions.rs
+++ b/crates/app/src/shell/actions.rs
@@ -12,6 +12,8 @@ actions!(
         RunStatement,
         /// Runs the whole script, statement by statement.
         RunAll,
+        /// Shows how the database would run the current statement.
+        Explain,
         /// Cancels the running query.
         CancelRun,
         /// Opens a new query tab.
@@ -48,6 +50,7 @@ pub fn key_bindings() -> Vec<KeyBinding> {
         KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE)),
         KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
         KeyBinding::new("cmd-shift-r", RunAll, Some(WORKSPACE)),
+        KeyBinding::new("cmd-e", Explain, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
         KeyBinding::new("cmd-t", NewTab, None),
@@ -73,6 +76,7 @@ pub fn menus() -> Vec<Menu> {
         Menu::new("Query").items([
             MenuItem::action("Run Statement", RunStatement),
             MenuItem::action("Run All", RunAll),
+            MenuItem::action("Explain", Explain),
             MenuItem::action("Cancel", CancelRun),
         ]),
         Menu::new("View").items([
```

Create `crates/app/src/views/query_tab/explain.rs`:
```rust
//! Explain: asks the session for a statement's plan and shows it in the
//! Plan view. The reply is polled once per [`POLL`], like a run's events.

use async_channel::Receiver;
use gpui_kit::Context;
use quarry_engine::{error::EngineError, plan::Plan};

use super::QueryTab;
use crate::{
    model::{drain, plan_tree::PlanState},
    views::workspace::POLL,
};

impl QueryTab {
    /// Shows the Plan view, and the plan once `reply` brings it.
    pub fn explain(&mut self, reply: Receiver<Result<Plan, EngineError>>, cx: &mut Context<Self>) {
        self.plan = PlanState::Planning;
        self.select_view("plan", cx);
        self.planning = Some(cx.spawn(async move |this, cx| {
            let mut answer = None;
            while answer.is_none() {
                cx.background_executor().timer(POLL).await;
                answer = drain::reply(drain::drain(&reply));
            }
            if let Some(answer) = answer {
                let _ = this.update(cx, |tab, cx| tab.planned(answer, cx));
            }
        }));
    }

    fn planned(&mut self, answer: Result<Plan, EngineError>, cx: &mut Context<Self>) {
        self.plan = PlanState::answered(answer);
        cx.notify();
    }

    /// What the Plan view shows.
    pub fn plan(&self) -> &PlanState {
        &self.plan
    }
}
```

Change `crates/app/src/views/query_tab/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/query_tab/mod.rs b/crates/app/src/views/query_tab/mod.rs
--- a/crates/app/src/views/query_tab/mod.rs
+++ b/crates/app/src/views/query_tab/mod.rs
@@ -4,6 +4,7 @@
 //! runs; a tab follows its run's events (see [`pump`]) and tells the
 //! workspace when the run ends.
 
+mod explain;
 mod layout;
 mod pump;
 
@@ -21,7 +22,7 @@ use super::{
     editor,
     results::{ResultViewRegistry, ResultsDelegate},
 };
-use crate::model::{statements::RunOutput, target};
+use crate::model::{plan_tree::PlanState, statements::RunOutput, target};
 
 /// What a tab tells the workspace.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
@@ -50,6 +51,8 @@ pub struct QueryTab {
     scope: RunScope,
     run: Option<RunHandle>,
     pump: Option<Task<()>>,
+    plan: PlanState,
+    planning: Option<Task<()>>,
     _edits: Subscription,
 }
 
@@ -76,6 +79,8 @@ impl QueryTab {
             scope: RunScope::Statement,
             run: None,
             pump: None,
+            plan: PlanState::Idle,
+            planning: None,
             _edits: edits,
         }
     }
```

Change `crates/app/src/views/results/mod.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/results/mod.rs b/crates/app/src/views/results/mod.rs
--- a/crates/app/src/views/results/mod.rs
+++ b/crates/app/src/views/results/mod.rs
@@ -5,6 +5,7 @@
 
 mod grid;
 mod messages;
+mod plan;
 mod table;
 
 use gpui_kit::{AnyElement, Context, Window};
@@ -34,12 +35,13 @@ pub struct ResultViewRegistry {
 }
 
 impl ResultViewRegistry {
-    /// Results, then Messages.
+    /// Results, Messages, Plan.
     pub fn standard() -> Self {
         Self {
             views: vec![
                 Box::new(table::ResultsView),
                 Box::new(messages::MessagesView),
+                Box::new(plan::PlanView),
             ],
         }
     }
```

Create `crates/app/src/views/results/plan.rs`:
```rust
//! The Plan view: the steps the database would take, indented by nesting.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{MONO_FONT, space, text},
};

use super::ResultView;
use crate::{
    model::plan_tree::{PlanRow, PlanState},
    views::query_tab::QueryTab,
};

/// How far each level of nesting is indented.
pub const INDENT: f32 = 18.0;

/// The plan of the last statement explained with ⌘E.
pub struct PlanView;

impl ResultView for PlanView {
    fn id(&self) -> &'static str {
        "plan"
    }

    fn title(&self) -> &'static str {
        "Plan"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let note = |message: String, role: Role| {
            vec![
                div()
                    .text_color(colors.get(role))
                    .child(message)
                    .into_any_element(),
            ]
        };
        let body = match tab.plan() {
            PlanState::Idle => note(
                "Explain a statement with ⌘E to see its plan.".to_owned(),
                Role::TextMuted,
            ),
            PlanState::Planning => note("Planning…".to_owned(), Role::TextMuted),
            PlanState::Failed(reason) => note(reason.clone(), Role::Danger),
            PlanState::Ready(rows) => rows.iter().map(step).collect(),
        };
        div()
            .id("plan")
            .size_full()
            .overflow_y_scroll()
            .p(px(space::MD))
            .flex()
            .flex_col()
            .gap(px(space::XS))
            .text_size(px(text::BODY))
            .children(body)
            .test_support()
            .into_any_element()
    }
}

/// One step, indented by its depth, in Menlo.
fn step(row: &PlanRow) -> AnyElement {
    let depth = u16::try_from(row.depth).unwrap_or(u16::MAX);
    div()
        .pl(px(INDENT * f32::from(depth)))
        .font_family(MONO_FONT)
        .child(row.label.clone())
        .into_any_element()
}
```

Change `crates/app/src/views/workspace/layout.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/layout.rs b/crates/app/src/views/workspace/layout.rs
--- a/crates/app/src/views/workspace/layout.rs
+++ b/crates/app/src/views/workspace/layout.rs
@@ -34,6 +34,7 @@ impl Render for Workspace {
             .track_focus(&self.focus)
             .on_action(cx.listener(Self::run_statement))
             .on_action(cx.listener(Self::run_all))
+            .on_action(cx.listener(Self::explain))
             .on_action(cx.listener(Self::cancel))
             .on_action(cx.listener(Self::new_tab))
             .on_action(cx.listener(Self::close_tab))
```

Change `crates/app/src/views/workspace/runs.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/views/workspace/runs.rs b/crates/app/src/views/workspace/runs.rs
--- a/crates/app/src/views/workspace/runs.rs
+++ b/crates/app/src/views/workspace/runs.rs
@@ -1,15 +1,16 @@
-//! Submits the active tab's runs to the session, and reads the schema.
+//! Submits the active tab's runs and plans to the session, and reads the
+//! schema.
 //!
 //! The schema reply is polled once per [`POLL`] rather than awaited (see
 //! [`crate::model::drain`]).
 
 use gpui_kit::{Context, Window};
-use quarry_engine::{error::EngineError, schema::Schema, splitter};
+use quarry_engine::splitter;
 
 use super::{POLL, Workspace};
 use crate::{
-    model::drain::{Drained, drain},
-    shell::actions::{CancelRun, RunAll, RunStatement},
+    model::drain,
+    shell::actions::{CancelRun, Explain, RunAll, RunStatement},
     views::query_tab::RunScope,
 };
 
@@ -45,6 +46,16 @@ impl Workspace {
         cx.notify();
     }
 
+    /// Shows the plan of the active tab's selection or current statement.
+    pub(super) fn explain(&mut self, _: &Explain, _: &mut Window, cx: &mut Context<Self>) {
+        let tab = self.active_tab().clone();
+        let Some(sql) = tab.read(cx).target_sql(cx) else {
+            return;
+        };
+        let reply = self.session.plan(sql);
+        tab.update(cx, |tab, cx| tab.explain(reply, cx));
+    }
+
     pub(super) fn cancel(&mut self, _: &CancelRun, _: &mut Window, cx: &mut Context<Self>) {
         self.active_tab().read(cx).cancel();
     }
@@ -57,7 +68,7 @@ impl Workspace {
             let mut answer = None;
             while answer.is_none() {
                 cx.background_executor().timer(POLL).await;
-                answer = schema_answer(drain(&reply));
+                answer = drain::reply(drain::drain(&reply));
             }
             if let Some(schema) = answer {
                 tree.update(cx, |tree, cx| tree.set(schema, cx));
@@ -65,14 +76,3 @@ impl Workspace {
         }));
     }
 }
-
-/// The schema reply, once it has come: the reply itself, or `WorkerGone`
-/// when the channel closed without one.
-fn schema_answer(
-    drained: Drained<Result<Schema, EngineError>>,
-) -> Option<Result<Schema, EngineError>> {
-    let Drained { mut items, closed } = drained;
-    items
-        .pop()
-        .or_else(|| closed.then_some(Err(EngineError::WorkerGone)))
-}
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry`
Expected: every app test passes; `tests/app` runs 35 tests.

Run: `just check`
Expected: exit 0.

Run: `cargo mutants -p quarry --in-diff <(git diff main)`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

- [ ] **Step 5: Commit**

```bash
git add -A crates/app/src/model/drain.rs crates/app/src/model/plan_tree.rs crates/app/src/shell/actions.rs crates/app/src/views/query_tab/explain.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/results/mod.rs crates/app/src/views/results/plan.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/runs.rs crates/app/tests/app/runs.rs crates/app/tests/shell.rs
git commit -m "feat(app): explain a statement's plan with Cmd-E"
```


---

### Task 9: Quit with the last window, and the docs

Closing the only window quits Quarry instead of leaving it running with no window (M13). `on_last_window_closed` takes the action to run so a test can observe it (GPUI's test platform ignores `quit`). ADR 0010 records the decisions; ADR 0009, the README and the design spec's §4.2 are brought up to date.

**Files:**
- Modify: `README.md`
- Modify: `crates/app/src/main.rs`
- Modify: `crates/app/src/shell/window.rs`
- Modify: `crates/app/tests/shell.rs`
- Modify: `docs/adr/0009-app-shell.md`
- Create: `docs/adr/0010-tabs-and-drafts.md`
- Modify: `docs/superpowers/specs/2026-11-04-quarry-design.md`
- Modify: `docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md`

**Interfaces:**
- Produces: `shell::window::on_last_window_closed(&mut App, impl Fn(&mut App) + 'static)`.

- [ ] **Step 1: Write the failing tests**

Change `crates/app/tests/shell.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/tests/shell.rs b/crates/app/tests/shell.rs
--- a/crates/app/tests/shell.rs
+++ b/crates/app/tests/shell.rs
@@ -1,7 +1,7 @@
 //! Start-up: the main window opens on the sample database, and every command
 //! has a key binding and a menu item.
 
-use std::path::Path;
+use std::{cell::Cell, path::Path, rc::Rc};
 
 use gpui_kit::{MenuItem, TestAppContext, px};
 use quarry::shell::{actions, store, window};
@@ -88,3 +88,23 @@ fn a_store_that_opens_has_no_problem() {
     let saved = store::saved_tabs(Some(&dir.path().join("quarry.db")));
     assert_eq!(saved.problem, None);
 }
+
+#[gpui_kit::test]
+fn closing_the_last_window_is_noticed(cx: &mut TestAppContext) {
+    let closed = Rc::new(Cell::new(false));
+    let seen = Rc::clone(&closed);
+    cx.update(|cx| {
+        gpui_kit::init(cx);
+        actions::install(cx);
+        assert_eq!(Aurora::install(Preference::Dark, cx), Ok(()));
+        window::on_last_window_closed(cx, move |_| seen.set(true));
+        assert_eq!(window::open_main(store::saved_tabs(None), cx), Ok(()));
+    });
+    let windows = cx.windows();
+    assert!(!closed.get());
+    for window in windows {
+        let _ = window.update(cx, |_, window, _| window.remove_window());
+    }
+    cx.run_until_parked();
+    assert!(closed.get());
+}
```


- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry --test shell`
Expected: `cannot find function on_last_window_closed`.

- [ ] **Step 3: Implement**

Change `README.md` (apply with `git apply`, or edit by hand):
```diff
diff --git a/README.md b/README.md
--- a/README.md
+++ b/README.md
@@ -2,7 +2,7 @@
 
 A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.
 
-> **Status:** Phase 1C. You can write and run SQL against a built-in sample database. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
+> **Status:** Phase 1D. You can write and run SQL against a built-in sample database in query tabs that survive restarts, run whole scripts, and explain a statement's plan. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
 
 - Built-in SQLite playground with story-driven SQL challenge tracks
 - PostgreSQL and MySQL connections for everyday work
```

Change `crates/app/src/main.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/main.rs b/crates/app/src/main.rs
--- a/crates/app/src/main.rs
+++ b/crates/app/src/main.rs
@@ -10,6 +10,7 @@ fn main() {
         .run(|cx: &mut App| {
             gpui_kit::init(cx);
             actions::install(cx);
+            window::on_last_window_closed(cx, |cx| cx.quit());
             let started = Aurora::install(Preference::Dark, cx)
                 .map_err(|error| error.to_string())
                 .and_then(|()| {
```

Change `crates/app/src/shell/window.rs` (apply with `git apply`, or edit by hand):
```diff
diff --git a/crates/app/src/shell/window.rs b/crates/app/src/shell/window.rs
--- a/crates/app/src/shell/window.rs
+++ b/crates/app/src/shell/window.rs
@@ -58,3 +58,14 @@ pub fn open_main(saved: SavedTabs, cx: &mut App) -> Result<(), EngineError> {
         message: error.to_string(),
     })
 }
+
+/// Runs `then` when the last window closes. Quarry quits then, so it never
+/// lingers with no window and no way to open one.
+pub fn on_last_window_closed(cx: &mut App, then: impl Fn(&mut App) + 'static) {
+    cx.on_window_closed(move |cx, _| {
+        if cx.windows().is_empty() {
+            then(cx);
+        }
+    })
+    .detach();
+}
```

Change `docs/adr/0009-app-shell.md` (apply with `git apply`, or edit by hand):
```diff
diff --git a/docs/adr/0009-app-shell.md b/docs/adr/0009-app-shell.md
--- a/docs/adr/0009-app-shell.md
+++ b/docs/adr/0009-app-shell.md
@@ -1,6 +1,6 @@
 # ADR 0009: App shell structure
 
-- **Status:** Accepted
+- **Status:** Accepted; tabs, per-statement results and run driving are updated by ADR 0010
 - **Date:** 2026-11-09
 
 ## Context
```

Create `docs/adr/0010-tabs-and-drafts.md`:
```markdown
# ADR 0010: Query tabs, drafts and result views

- **Status:** Accepted
- **Date:** 2026-11-09

## Context
Phase 1D turns the single editor of Phase 1C (ADR 0009) into query tabs whose drafts survive restarts. It also adds Run all with a result per statement, the Messages and Plan views, and Explain. All tabs share one database session.

## Decision
- **One `QueryTab` entity per tab.** Each tab owns its editor, its run and its results, so switching tabs keeps undo history, scroll position and output. The workspace keeps the session, the sidebar, the `TabList` model and the status bar, and submits runs on the active tab's behalf.
- **Runs queue on the one session.** A tab whose run is queued behind another tab's reads "Waiting for the previous run…" until its first statement starts. ⌘. cancels only the active tab's run.
- **Per-statement output.** `model::statements::RunOutput` keeps every statement's columns, rows (up to 100,000), and summary or error. The grid shows the first statement that returns rows. With more than one, a strip of result tabs picks between them, and Messages lists every statement. The status bar times the whole run, from its first statement starting until `Done`, and leaves out time spent queued behind another tab. Events are timestamped when the UI polls them (every 33 ms), so the final figure is the larger of that wall time and the statements' own engine times added up; a fast script reads its real time, and the figure never jumps backwards. Each statement's own engine time is in Messages.
- **Result views are a registry.** `ResultView` and `ResultViewRegistry` hold Results, Messages and Plan; a new view is a module and one line. A segmented control switches between them.
- **Plans come from the driver.** `Driver::plan` defaults to `PlanUnsupported`. SQLite builds a tree from `EXPLAIN QUERY PLAN`, and `Session::plan` runs it on the session thread after any queued run, like `schema()`.
- **Drafts live in `quarry-store`.** `DraftStore` loads and saves `Drafts` (tabs and the active index); `SqliteStore` keeps them in `~/Library/Application Support/Quarry/quarry.db`, migrated with `PRAGMA user_version`. A file from a newer Quarry is refused, never changed. If the store can't be opened, `shell::store` falls back to a store that keeps nothing, and the status bar says drafts aren't being saved. If it opens but its drafts can't be read, the workspace falls back the same way, so a save never writes a fresh tab over drafts that are still on disk.
- **When drafts are saved.** One second after the last edit (each edit replaces the pending timer), at once when tabs open, close or switch, and synchronously when the window closes or Quarry quits. Saves run on the background executor one after another, so an older snapshot never lands after a newer one.
- **Streaming throughput.** The session buffers 128 batches of 500 rows per run instead of 16. With the UI polling every 33 ms, the 200,000-row sample table streams in about 0.16 s instead of about 1 s; the engine's own time drops from about 0.93 s to 0.12 s, because it no longer waits on the reader.
- **App lifecycle.** The shell follows the system appearance and quits when the last window closes.

## Consequences
- A run's results live only in its tab and are gone after a restart; a draft's cursor and selection are not saved.
- A long run in one tab holds up runs in every other tab until it ends or is cancelled.
- Up to 64,000 rows per run can wait in the channel, which costs memory only while the UI falls behind.
```

Change `docs/superpowers/specs/2026-11-04-quarry-design.md` (apply with `git apply`, or edit by hand):
```diff
diff --git a/docs/superpowers/specs/2026-11-04-quarry-design.md b/docs/superpowers/specs/2026-11-04-quarry-design.md
--- a/docs/superpowers/specs/2026-11-04-quarry-design.md
+++ b/docs/superpowers/specs/2026-11-04-quarry-design.md
@@ -151,10 +151,10 @@ Reference mockups are in `docs/design/mockups/` (`visual-direction.html`, `light
    - The schema tree: tables with row counts, and columns with their types. Clicking a table opens it in a browse tab. Dragging a name into the editor inserts it as text, which is a manual action and not autocomplete.
    - Query history.
 2. **Center**
-   - **Tabs** (⌘T opens one, ⌘W closes one). Each challenge tab keeps its own draft, and drafts survive restarts. Scratch tabs are freeform.
+   - **Tabs** (⌘T opens one, ⌘W closes one, ⌘⇧] and ⌘⇧[ switch). Each challenge tab keeps its own draft, and drafts survive restarts. Scratch tabs are freeform.
    - **Editor:** syntax highlighting, line numbers, bracket matching and multiple cursors. **Every completion and suggestion feature is disabled.**
    - A draggable divider.
-   - **Results views:** Results, Messages and Plan (`EXPLAIN` / `EXPLAIN QUERY PLAN`).
+   - **Results views:** Results, Messages and Plan. Explain (⌘E) fills the Plan view from the driver (`EXPLAIN QUERY PLAN` on SQLite).
    - **Actions:** Run (⌘↵) runs the statement under the cursor or the selection. Run all (⌘⇧R) runs the whole script, with one results tab per statement. Check answer (⌘⇧↵). Cancel (⌘.).
    - **Results grid:** virtualized and streaming ("1,000 of ~2.3M rows · fetching…"), resizable columns, NULLs shown in dim italic, and copy as TSV, CSV or JSON.
    - **Status bar:** which database you're connected to, row count, how long the query took, cursor position, a live timer while running, and a transaction indicator.
```

Change `docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md` (apply with `git apply`, or edit by hand):
```diff
diff --git a/docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md b/docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md
--- a/docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md
+++ b/docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md
@@ -63,7 +63,7 @@ impl SqliteStore {
   - A file with a newer `user_version` than this build knows is refused with `StoreError::TooNew { found, supported }` and never modified.
 - **Schema v1:**
   - `drafts(id INTEGER PRIMARY KEY, position INTEGER NOT NULL, title TEXT NOT NULL, text TEXT NOT NULL)`
-  - `state(key TEXT PRIMARY KEY, value TEXT NOT NULL)`, where the `active` key holds the active tab's index.
+  - `state(key TEXT PRIMARY KEY, value INTEGER NOT NULL)`, where the `active` key holds the active tab's index.
 - **`save`** replaces every row in one transaction.
 - **`load`** of an empty store returns no tabs and `active` 0. An `active` index past the last tab is clamped to the last tab.
 - **`StoreError`** is a `thiserror` enum with readable messages: `Open`, `TooNew`, `Sql(String)`.
@@ -93,11 +93,11 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
 | Module | Responsibility |
 |---|---|
 | `tabs.rs` | `TabList`: the ordered tab ids, the active index, and a "Query N" name taking the next N after the highest one in use. Also the `open`, `close` (closing the last tab opens a fresh one), `next` and `previous` (both wrap) and `activate` rules. |
-| `autosave.rs` | `Autosave`: `edited(now)`, `due(now) -> bool` (1 s after the last edit) and `saved()`. Pure timing, with no I/O. |
-| `statements.rs` | `RunOutput`: splits one run's `RunEvent`s into a `StatementOutput` per statement. Each holds the index, the range, a label (number plus first words, 24 characters at most, whitespace collapsed), its columns, its rows (`ROW_LIMIT` cap and a truncated count), and either a `Summary` or an `EngineError`. Absorbs today's per-statement handling in `RunModel`. |
+| `statements.rs` | `RunOutput`: splits one run's `RunEvent`s into a `StatementOutput` per statement. Each holds the index, a label (number plus first words, 24 characters at most, whitespace collapsed), its columns, its rows (`ROW_LIMIT` cap and a truncated count), and either a `Summary` or an `EngineError`. Replaces 1C's `RunModel`. |
 | `messages.rs` | One `MessageLine { statement, text, is_error }` per statement: "N rows · t", "N rows affected · t", "Done · t", the error text, or "Cancelled". |
-| `plan_tree.rs` | Flattens a `Plan` into `PlanRow { depth, label }`, depth first. |
-| `status.rs` (changed) | Adds the waiting state ("Waiting for the previous run…") and the whole-run time (M7). `failure` returns `Failure { message, cancelled }` (M14). |
+| `plan_tree.rs` | Flattens a `Plan` into `PlanRow { depth, label }`, depth first; `PlanState` (Idle, Planning, Ready, Failed) is what the Plan view shows. |
+| `status.rs` (changed) | Adds the waiting state ("Waiting for the previous run…") and "N statements · t" for a finished multi-statement run (M7). `failure` returns `Failure { message, cancelled }` (M14). |
+| `drain.rs` (changed) | `reply` turns a drained one-off reply (a schema or a plan) into its answer, or `WorkerGone`. |
 | `format.rs` (changed) | `cell_text` borrows `&Value`, truncates before normalising, and maps `\t` and other control characters to spaces (M4). |
 
 ### 3.4 App: `views/`
@@ -105,7 +105,7 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
 - **`query_tab.rs`:** a `QueryTab` entity. It holds the editor, the current `RunHandle` (`run`), the pump task (`pump`), a `RunOutput`, the selected result view, and the selected statement tab.
   - It takes its events through an injected `Receiver<RunEvent>`, so tests can supply any channel.
   - Most of 1C's `workspace/runner.rs` moves here.
-- **`tab_bar.rs`:** the query tab strip, using ui-kit `Tab`, with a "+" button.
+- **`workspace/tab_strip.rs`:** the query tab strip in the title bar, using ui-kit `Tab`, with a "+" button.
 - **`results/`:**
   - The `ResultView` trait: `id`, `title`, `render(&QueryTab, …)`.
   - `ResultViewRegistry`, holding Results, Messages and Plan in that order. Adding a view is a new module plus one registration line.
@@ -116,7 +116,7 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
   - the schema sidebar;
   - the `TabList` and its `Entity<QueryTab>`s;
   - the status bar, which reads the active tab;
-  - `Autosave` and the `DraftStore`, as `Box<dyn DraftStore>`.
+  - the `DraftStore`, as `Arc<dyn DraftStore>`, and the pending autosave timer.
 
 ### 3.5 App: `shell/`
 
@@ -126,8 +126,8 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
   - Run and Cancel also bind in the `Workspace > Input` context, as in 1C.
 - **Menus:** File gets New Tab and Close Tab. Query gets Run All and Explain. View gets Next Tab and Previous Tab.
 - **Follow the system appearance** from `open_main`, not from `Workspace` (M12).
-- **Quit when the last window closes** (M13).
-- **Opening the store:** if it fails, the workspace gets a store that keeps drafts in memory, and the warning reason is shown (§5).
+- **Quit when the last window closes** (M13): `on_last_window_closed` takes the action to run, so tests can observe it.
+- **Opening the store (`shell::store`):** if it fails, or the drafts can't be loaded from it, the workspace gets `Forgetful`, a store that keeps nothing, and the warning reason is shown (§5). The file is never written over, and the tabs themselves stay open for the session.
 
 ## 4. Behaviour
 
@@ -138,7 +138,7 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
 - **Cancel:** ⌘. cancels the active tab's run.
 - **Run (⌘↵)** runs the selection or the statement under the cursor, using 1C's `run_target`, and selects the Results view.
 - **Run all (⌘⇧R)** sends the whole editor text.
-  - Each statement that returns rows gets a result tab, and the first such tab is selected.
+  - Each statement that returns rows gets a result tab (the strip appears once there are two), and the first such tab is selected.
   - If no statement returns rows, the Messages view is selected.
 - **Messages** lists every statement that started.
 - **Failure:**
@@ -146,8 +146,8 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
   - The red banner (1C's `failure_banner`) shows above whichever view is open.
 - **Re-running a tab** replaces that tab's output, clears the grid's selection and scrolls it to the top (M3). Other tabs are unaffected.
 - **Schema refresh:** after every run, the schema sidebar refreshes, as in 1C.
-- **Status-bar time:** the timer measures from submission to `Done`, both while running and at the end (M7). Each statement's own time appears in Messages.
-- **Throughput (M2):** when a drain empties a full `EVENT_BUFFER`, the pump drains again straight away, up to 8 times per tick. The 200k-row stream time is measured before and after and recorded in ADR 0010.
+- **Status-bar time (M7):** the timer measures from the first statement's `Started` to `Done`, both while running and at the end; the final figure is never less than the statements' own times added up, because polled timestamps are only as precise as the 33 ms poll. Time spent queued behind another tab's run isn't counted. Each statement's own time appears in Messages.
+- **Throughput (M2):** the session's `EVENT_BUFFER` grows from 16 to 128 batches. Measured with a 33 ms poll, the 200k-row stream drops from about 1.0 s to 0.16 s (recorded in ADR 0010). Re-polling within a tick was considered, but the worker refills too slowly between immediate re-polls for it to help.
 
 ### 4.2 Explain (⌘E)
 
@@ -162,9 +162,8 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
 - **⌘T** opens an empty "Query N" after the active tab and focuses its editor.
 - **⌘W** closes the active tab without a prompt and activates its right neighbour, or its left neighbour if it was last. Closing the only tab replaces it with a fresh empty "Query N".
 - **Saving:**
-  - Edits call `Autosave::edited`.
-  - A save happens 1 s after the last edit.
-  - It also happens straight away on ⌘T, ⌘W, a tab switch and app quit.
+  - Each edit (the editor's change event) restarts a 1 s timer (`AUTOSAVE`); when it fires, the drafts are saved.
+  - It also happens straight away on ⌘T, ⌘W, a tab switch, closing the window and app quit.
   - Saves run on GPUI's background executor.
 - **What is saved:** titles, texts, order and the active index. Results, cursors and selections are not saved.
 
@@ -172,7 +171,7 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
 
 | Situation | Behaviour |
 |---|---|
-| Store can't be opened (permissions, corrupt file) | Drafts are kept in memory. The status bar shows "Drafts aren't being saved", with the error as its tooltip. |
+| Store can't be opened or its drafts can't be loaded (permissions, corrupt file, locked) | Drafts are kept in memory. The status bar shows "Drafts aren't being saved", with the error as its tooltip. |
 | Store file is from a newer Quarry | The same warning, with "Saved by a newer Quarry (version N)". The file is never modified. |
 | A save fails | The same warning. Saving is retried on the next edit, and the warning clears after a successful save. |
 | Plan unsupported or failed | A message inside the Plan view. |
@@ -192,7 +191,7 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
   - the default `plan` returns `PlanUnsupported`;
   - `Session::plan` after a queued run.
 - **Models:** every module in §3.3 has unit tests for each rule listed there.
-- **Views (GPUI tests):**
+- **Views (GPUI tests, one `tests/app` binary sharing `tests/app/support.rs`):**
   - ⌘T, ⌘W and switching, with each tab keeping its output;
   - closing the last tab;
   - Run all producing the right result tabs and Messages lines;
@@ -226,12 +225,12 @@ pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>
 
 1. `quarry-store`: `DraftStore`, `SqliteStore`, migrations, `StoreError`.
 2. Engine: `Plan`, `Driver::plan`, `EngineError::PlanUnsupported`, the SQLite plan, `Session::plan`.
-3. Model: `tabs`, `autosave`.
-4. Model: `statements`, `messages`, `plan_tree`; M4 (formatting), M6, M7, M14.
-5. Move the editor and run into `QueryTab` with no change in behaviour; M2, M3, M4 (view), M9, M11, the vanished-worker test.
-6. Tab bar, ⌘T, ⌘W, ⌘⇧] and ⌘⇧[; draft persistence and the warning.
+3. Model: `tabs`.
+4. Model: `statements`, `messages`, `plan_tree`; M4 (formatting), M6, M7, M14; the views switch to `RunOutput`.
+5. Move the editor and run into `QueryTab` with no change in behaviour; M2, M3, M4 (view), M9, M11, M12, the vanished-worker test.
+6. Tab bar, ⌘T, ⌘W, ⌘⇧] and ⌘⇧[; draft persistence, `shell::store` and the warning.
 7. `ResultViewRegistry`, the Results/Messages switch, per-statement result tabs, Run all.
 8. Plan view and Explain (⌘E).
-9. Shell (M12, M13, the store path, menus), ADR 0010 "Tabs and drafts", README, and a parent-spec §4.2 note.
+9. M13, ADR 0010 "Query tabs, drafts and result views", README, and a parent-spec §4.2 note.
 
 As in 1C, a verified prototype is built first, and the plan is generated from it.
```


- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry --test shell`
Expected: 7 tests pass.

Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add -A README.md crates/app/src/main.rs crates/app/src/shell/window.rs crates/app/tests/shell.rs docs/adr/0009-app-shell.md docs/adr/0010-tabs-and-drafts.md docs/superpowers/specs/2026-11-04-quarry-design.md docs/superpowers/specs/2026-11-09-phase-1d-tabs-runs-design.md
git commit -m "feat(app): quit when the last window closes, and document tabs and drafts"
```


---
