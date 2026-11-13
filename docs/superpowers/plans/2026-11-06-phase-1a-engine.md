# Phase 1A: Engine Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `quarry-engine`, the database layer the UI will use to run SQL. That means the shared value and result model, the `Driver` trait and its registry, a SQLite driver that streams rows in batches and can be cancelled, a SQL statement splitter, and a `Session` that runs a driver on its own thread and streams results back as events.

**Architecture:**
- **Drivers are synchronous.** A `Session` owns one driver on a dedicated thread. It splits each submitted script into statements, runs them in order, and sends `RunEvent`s over a bounded `async-channel`. The UI can `await` events without blocking, and a slow reader applies backpressure.
- **Cancellation is per run.** A shared `Canceller` flag is checked by SQLite's progress handler, so a cancel that arrives just before a statement starts is never lost.
- **No GPUI.** This is pure Rust and test-first.

**Tech Stack:** Rust 1.98.1, `rusqlite` 0.41.1 (features `bundled`, `hooks`, `column_decltype`), `async-channel` 2.5.0, `thiserror` 2. Tests: nextest, `tempfile`.

**Spec:** `docs/superpowers/specs/2026-11-04-quarry-design.md` §3.1–3.4 and §6 (driver model). This plan departs from §3.4 in one place, the synchronous `Driver` trait; ADR 0007 (Task 7) records why.

## Global Constraints

- Branch: `phase-1a-engine`, which already exists and holds this plan. Use Conventional Commits.
- `quarry-engine` depends on **no workspace crate**; `check-deps` enforces this. Its only dependencies are `thiserror`, `rusqlite`, `async-channel` and `tempfile` (dev).
- Coverage for `engine` must be at least **95% lines / 90% regions**. Mutation score on `quarry-engine` must be at least **90%**, and the plan's tests are written to kill every viable mutant.
- Per function: cyclomatic ≤ 10, cognitive ≤ 15, ≤ 15 statements, ≤ 40 lines, ≤ 5 arguments, nesting ≤ 4. **These limits apply to tests too.**
- Banned outside tests: `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`. Tests return `Result<(), Box<dyn std::error::Error>>` and use `?`.
- No lint suppression attributes. All of this code passes without any.
- The app's own SQL (catalog introspection) uses **bound parameters**, never string formatting.
- Every `EngineError` renders as a sentence a user can read.
- **Every task ends with `just check` passing.** (`just check` runs fmt, clippy, the xtask gates, deny, machete, typos, complexity and coverage.)
- Don't commit anything under `.superpowers/`.

---

## File Structure

```
crates/engine/
├─ Cargo.toml                 deps grow per task (Tasks 1, 4, 6)
├─ src/lib.rs                 module list (grows per task)
├─ src/value.rs               Value enum + Display                     (Task 1)
├─ src/error.rs               EngineError                               (Task 1)
├─ src/result.rs              Column, Row, Summary, RowSink, QueryResult (Task 1)
├─ src/schema.rs              Schema, Table, TableKind, ColumnInfo      (Task 1)
├─ src/driver.rs              Driver trait, Canceller                   (Task 2)
├─ src/splitter.rs            split, statement_at                       (Task 3)
├─ src/sqlite/mod.rs          SqliteDriver, connect                     (Task 4)
├─ src/sqlite/convert.rs      rusqlite → Value / EngineError            (Task 4)
├─ src/sqlite/run.rs          execute one statement, batch streaming    (Task 4)
├─ src/sqlite/introspect.rs   catalog → Schema                          (Task 4)
├─ src/registry.rs            DriverRegistry (driver extension point)   (Task 5)
├─ src/session.rs             Session, RunHandle, RunEvent              (Task 6)
└─ tests/sqlite.rs registry.rs session.rs                               (Tasks 4–6)
docs/adr/0007-synchronous-drivers-on-a-session-thread.md                (Task 7)
```

---

### Task 1: Value, error, result and schema model

**Files:**
- Modify: `crates/engine/Cargo.toml`, `crates/engine/src/lib.rs`
- Create: `crates/engine/src/value.rs`, `error.rs`, `result.rs`, `schema.rs` (unit tests are inside each module)

**Interfaces (produced, used by every later task):**
- `Value { Null, Int(i64), Float(f64), Text(String), Bytes(Vec<u8>) }` with `is_null()` and `Display` (shows `NULL` for null, and `<n bytes>` for bytes)
- `EngineError { Sql(String), Cancelled, Open { target, message }, Unsupported(String), WorkerGone }`, which is `Clone + PartialEq`
- `Column { name, type_name }`
- `type Row = Vec<Value>`
- `Summary { rows: u64, rows_affected: Option<u64>, elapsed: Duration }`
- `trait RowSink { fn columns(&mut self, Vec<Column>); fn rows(&mut self, Vec<Row>); }`
- `QueryResult { columns, rows }`, which implements `RowSink`
- `Schema { tables: Vec<Table> }`, `Table { name, kind: TableKind, columns: Vec<ColumnInfo> }`, `TableKind { Table, View }`, `ColumnInfo { name, type_name, primary_key }`

- [ ] **Step 1: Switch to the branch.** It already exists and holds this plan.

```bash
git checkout phase-1a-engine && git pull --ff-only 2>/dev/null; git log --oneline -1
```

- [ ] **Step 2: Add `thiserror` to the engine.** In `crates/engine/Cargo.toml`, insert before `[lints]`:

```toml
[dependencies]
thiserror.workspace = true

```

- [ ] **Step 3: Write the modules, unit tests first.** Each file below contains its tests in a `#[cfg(test)] mod tests`. Create them in this order: `value.rs` with its tests, then run the tests and see them fail, then add the implementation. The same goes for `error.rs` and `result.rs`. `schema.rs` is plain data with no logic; it's covered by Task 4's tests.

`crates/engine/src/value.rs`:
```rust
//! The database-independent cell value.

use std::fmt;

/// One cell of a result set, independent of the database that produced it.
///
/// Only the variants a current driver produces exist; the Postgres and MySQL
/// drivers (Phase 3) add their own types here.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// SQL `NULL`.
    Null,
    /// A 64-bit integer.
    Int(i64),
    /// A double-precision float.
    Float(f64),
    /// Text.
    Text(String),
    /// Raw bytes (a BLOB).
    Bytes(Vec<u8>),
}

impl Value {
    /// Whether this is SQL `NULL`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("NULL"),
            Self::Int(n) => write!(f, "{n}"),
            Self::Float(x) => write!(f, "{x}"),
            Self::Text(s) => f.write_str(s),
            Self::Bytes(b) => write!(f, "<{} bytes>", b.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn displays_each_variant_for_the_grid() {
        let shown: Vec<String> = [
            Value::Null,
            Value::Int(-42),
            Value::Float(2.5),
            Value::Text("Orbital".to_owned()),
            Value::Bytes(vec![1, 2, 3]),
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(shown, ["NULL", "-42", "2.5", "Orbital", "<3 bytes>"]);
    }

    #[test]
    fn only_null_is_null() {
        assert!(Value::Null.is_null());
        assert!(!Value::Int(0).is_null());
        assert!(!Value::Text(String::new()).is_null());
    }
}
```

`crates/engine/src/error.rs`:
```rust
//! Errors a user can see, each with a readable message.

/// Why a database operation failed.
///
/// Every variant renders as a sentence the UI can show as is. Errors are
/// plain data (`Clone`) so they can travel from the session thread.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    /// The database rejected the SQL; the message is the database's own.
    #[error("{0}")]
    Sql(String),
    /// The user cancelled the query.
    #[error("Query cancelled.")]
    Cancelled,
    /// A database could not be opened.
    #[error("Cannot open {target}: {message}")]
    Open {
        /// What was being opened (a path or `:memory:`).
        target: String,
        /// Why it failed.
        message: String,
    },
    /// No driver handles this connection string.
    #[error("Unsupported connection: {0}")]
    Unsupported(String),
    /// The session thread has stopped.
    #[error("The query worker stopped unexpectedly.")]
    WorkerGone,
}

#[cfg(test)]
mod tests {
    use super::EngineError;

    #[test]
    fn messages_are_readable_sentences() {
        let open = EngineError::Open {
            target: "/tmp/x.db".to_owned(),
            message: "unable to open database file".to_owned(),
        };
        assert_eq!(
            open.to_string(),
            "Cannot open /tmp/x.db: unable to open database file"
        );
        assert_eq!(
            EngineError::Sql("no such table: t".to_owned()).to_string(),
            "no such table: t"
        );
        assert_eq!(EngineError::Cancelled.to_string(), "Query cancelled.");
        assert_eq!(
            EngineError::Unsupported("x:y".to_owned()).to_string(),
            "Unsupported connection: x:y"
        );
        assert_eq!(
            EngineError::WorkerGone.to_string(),
            "The query worker stopped unexpectedly."
        );
    }
}
```

`crates/engine/src/result.rs`:
```rust
//! Result-set shapes shared by every driver.

use std::time::Duration;

use crate::value::Value;

/// A result column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Column name as the query labels it.
    pub name: String,
    /// Declared type, or empty when the database doesn't know (expressions).
    pub type_name: String,
}

/// One result row, in column order.
pub type Row = Vec<Value>;

/// What one statement did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// Rows returned (0 for statements that return none).
    pub rows: u64,
    /// Rows changed, for statements that return no rows.
    pub rows_affected: Option<u64>,
    /// Wall-clock time the statement took.
    pub elapsed: Duration,
}

/// Receives a statement's results as they are produced.
pub trait RowSink {
    /// Called once, before any rows, for statements that return rows.
    fn columns(&mut self, columns: Vec<Column>);
    /// Called with each batch of rows, in order.
    fn rows(&mut self, batch: Vec<Row>);
}

/// A sink that keeps everything in memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueryResult {
    /// The result columns.
    pub columns: Vec<Column>,
    /// Every row received.
    pub rows: Vec<Row>,
}

impl RowSink for QueryResult {
    fn columns(&mut self, columns: Vec<Column>) {
        self.columns = columns;
    }

    fn rows(&mut self, batch: Vec<Row>) {
        self.rows.extend(batch);
    }
}

#[cfg(test)]
mod tests {
    use super::{Column, QueryResult, RowSink};
    use crate::value::Value;

    #[test]
    fn query_result_keeps_columns_and_appends_batches() {
        let mut result = QueryResult::default();
        let column = Column {
            name: "n".to_owned(),
            type_name: "INTEGER".to_owned(),
        };
        result.columns(vec![column.clone()]);
        result.rows(vec![vec![Value::Int(1)]]);
        result.rows(vec![vec![Value::Int(2)], vec![Value::Int(3)]]);
        assert_eq!(result.columns, [column]);
        assert_eq!(
            result.rows,
            [[Value::Int(1)], [Value::Int(2)], [Value::Int(3)]]
        );
    }
}
```

`crates/engine/src/schema.rs`:
```rust
//! What a database contains, for the schema tree.

/// Every table and view in a database, sorted by name.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Schema {
    /// Tables and views.
    pub tables: Vec<Table>,
}

/// Whether an entry is a stored table or a view.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableKind {
    /// A stored table.
    Table,
    /// A view.
    View,
}

/// One table or view.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Table {
    /// Table name.
    pub name: String,
    /// Table or view.
    pub kind: TableKind,
    /// Columns in declaration order.
    pub columns: Vec<ColumnInfo>,
}

/// One column of a table.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ColumnInfo {
    /// Column name.
    pub name: String,
    /// Declared type (may be empty).
    pub type_name: String,
    /// Whether the column is part of the primary key.
    pub primary_key: bool,
}
```

- [ ] **Step 4: Declare the modules.** Replace `crates/engine/src/lib.rs` with:

```rust
//! Database layer: the `Driver` trait, the shared value model, and drivers.
//!
//! The UI talks to a database only through a `session::Session`, which runs
//! a `driver::Driver` on its own thread and streams results back as events.

pub mod error;
pub mod result;
pub mod schema;
pub mod value;
```

- [ ] **Step 5: Verify**

Run: `cargo nextest run -p quarry-engine`
Expected: PASS, with 4 tests (2 in `value`, 1 in `error`, 1 in `result`).
Run: `just check`
Expected: exit 0.

- [ ] **Step 6: Commit**

```bash
git add crates/engine Cargo.lock
git commit -m "feat(engine): add value, error, result and schema model"
```

---

### Task 2: Driver trait and canceller

**Files:**
- Create: `crates/engine/src/driver.rs`
- Modify: `crates/engine/src/lib.rs` (add `pub mod driver;`, keeping the list alphabetical)

**Interfaces:**
- Consumes: `EngineError`, `RowSink`, `Summary` and `Schema` (Task 1).
- Produces:
  - `trait Driver: Send { fn execute(&mut self, sql: &str, sink: &mut dyn RowSink) -> Result<Summary, EngineError>; fn schema(&mut self) -> Result<Schema, EngineError>; fn canceller(&self) -> Canceller; }`
  - `Canceller` (`Clone + Default`; clones share one `Arc<AtomicBool>`) with `cancel()`, `reset()` and `is_cancelled()`

- [ ] **Step 1: Write `driver.rs` with its unit test first**, run `cargo nextest run -p quarry-engine driver` and see it fail, then add the implementation. The final file:

```rust
//! The contract every database driver implements.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{
    error::EngineError,
    result::{RowSink, Summary},
    schema::Schema,
};

/// A connection to one database.
///
/// Drivers are synchronous; a [`crate::session::Session`] runs them on their
/// own thread so the UI never blocks.
pub trait Driver: Send {
    /// Runs one SQL statement, streaming any rows into `sink`.
    ///
    /// # Errors
    /// Returns [`EngineError::Sql`] when the database rejects the statement and
    /// [`EngineError::Cancelled`] when the driver's [`Canceller`] fires.
    fn execute(&mut self, sql: &str, sink: &mut dyn RowSink) -> Result<Summary, EngineError>;

    /// Reads every table and view with its columns.
    ///
    /// # Errors
    /// Returns an error when the catalog cannot be read.
    fn schema(&mut self) -> Result<Schema, EngineError>;

    /// The flag that interrupts whatever this driver is running.
    fn canceller(&self) -> Canceller;
}

/// A shared cancel flag; cloning shares the same flag.
#[derive(Debug, Clone, Default)]
pub struct Canceller(Arc<AtomicBool>);

impl Canceller {
    /// Requests cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Clears a previous request.
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::Canceller;

    #[test]
    fn clones_share_one_flag_that_can_be_reset() {
        let canceller = Canceller::default();
        let clone = canceller.clone();
        assert!(!clone.is_cancelled());
        canceller.cancel();
        assert!(clone.is_cancelled());
        clone.reset();
        assert!(!canceller.is_cancelled());
    }
}
```

- [ ] **Step 2: Verify**

Run: `cargo nextest run -p quarry-engine` (5 tests pass), then `just check` (exit 0).

- [ ] **Step 3: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): add Driver trait and shared canceller"
```

---

### Task 3: SQL statement splitter

**Files:**
- Create: `crates/engine/src/splitter.rs`
- Modify: `crates/engine/src/lib.rs` (add `pub mod splitter;`)

**Interfaces:**
- Produces:
  - `splitter::split(script: &str) -> Vec<Range<usize>>`: byte ranges of statements, trimmed, without the `;`, with comment-only segments skipped
  - `splitter::statement_at(script: &str, offset: usize) -> Option<Range<usize>>`: the last statement starting at or before `offset`, else the first statement
- **Behavior pinned by tests:**
  - `'…'`, `"…"` and `` `…` `` are quoted, and a doubled quote is an escape.
  - `-- …` runs to the newline, and `/* … */` is a block comment.
  - An unterminated quote or comment runs to the end of the script.
  - Ranges are byte offsets, which stay safe for multibyte text because every delimiter is ASCII.
  - Documented limit: a `;` inside a `CREATE TRIGGER … BEGIN … END` body ends the statement early.

- [ ] **Step 1: Write the tests first.** Create `splitter.rs` with only the `#[cfg(test)] mod tests` block below, plus stub signatures returning `Vec::new()` and `None`. Run `cargo nextest run -p quarry-engine splitter`: almost every test fails, because the stubs return nothing.

- [ ] **Step 2: Implement.** The final `crates/engine/src/splitter.rs`:

```rust
//! Splits a SQL script into statements, respecting quotes and comments.
//!
//! Known limit: a `;` inside a `CREATE TRIGGER ... BEGIN ... END` body ends
//! the statement early. Quarry's content avoids trigger bodies.

use std::ops::Range;

/// Byte ranges of every statement in `script`, in order.
///
/// Each range excludes the terminating `;` and surrounding whitespace.
/// Segments holding only whitespace and comments are skipped.
pub fn split(script: &str) -> Vec<Range<usize>> {
    let bytes = script.as_bytes();
    let mut statements = Vec::new();
    let (mut start, mut pos) = (0, 0);
    while pos < bytes.len() {
        if bytes[pos] == b';' {
            push_statement(script, start..pos, &mut statements);
            start = pos + 1;
            pos += 1;
        } else {
            pos = skip_token(bytes, pos);
        }
    }
    push_statement(script, start..bytes.len(), &mut statements);
    statements
}

/// The statement to run for a cursor at byte `offset`: the last statement
/// starting at or before it, else the first statement.
pub fn statement_at(script: &str, offset: usize) -> Option<Range<usize>> {
    let statements = split(script);
    statements
        .iter()
        .rev()
        .find(|range| range.start <= offset)
        .or_else(|| statements.first())
        .cloned()
}

fn push_statement(script: &str, range: Range<usize>, out: &mut Vec<Range<usize>>) {
    let text = &script[range.clone()];
    let trimmed = text.trim();
    if trimmed.is_empty() || !has_code(trimmed.as_bytes()) {
        return;
    }
    let start = range.start + (text.len() - text.trim_start().len());
    out.push(start..start + trimmed.len());
}

fn has_code(bytes: &[u8]) -> bool {
    let mut pos = 0;
    while pos < bytes.len() {
        match (bytes[pos], bytes.get(pos + 1)) {
            (b'-', Some(b'-')) => pos = skip_line_comment(bytes, pos),
            (b'/', Some(b'*')) => pos = skip_block_comment(bytes, pos),
            (byte, _) if byte.is_ascii_whitespace() => pos += 1,
            _ => return true,
        }
    }
    false
}

fn skip_token(bytes: &[u8], pos: usize) -> usize {
    match (bytes[pos], bytes.get(pos + 1)) {
        (b'\'' | b'"' | b'`', _) => skip_quoted(bytes, pos),
        (b'-', Some(b'-')) => skip_line_comment(bytes, pos),
        (b'/', Some(b'*')) => skip_block_comment(bytes, pos),
        _ => pos + 1,
    }
}

/// Skips a quoted string or identifier; a doubled quote is an escape.
fn skip_quoted(bytes: &[u8], pos: usize) -> usize {
    let quote = bytes[pos];
    let mut i = pos + 1;
    while i < bytes.len() {
        if bytes[i] != quote {
            i += 1;
        } else if bytes.get(i + 1) == Some(&quote) {
            i += 2;
        } else {
            return i + 1;
        }
    }
    bytes.len()
}

/// Skips to the newline ending a `--` comment (the newline itself is kept).
fn skip_line_comment(bytes: &[u8], pos: usize) -> usize {
    bytes[pos..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |n| pos + n)
}

fn skip_block_comment(bytes: &[u8], pos: usize) -> usize {
    bytes[pos + 2..]
        .windows(2)
        .position(|w| w == b"*/")
        .map_or(bytes.len(), |n| pos + 2 + n + 2)
}

#[cfg(test)]
mod tests {
    use super::{split, statement_at};

    fn texts(script: &str) -> Vec<&str> {
        split(script).into_iter().map(|r| &script[r]).collect()
    }

    #[test]
    fn splits_on_semicolons_and_trims() {
        assert_eq!(
            texts("SELECT 1;\n  SELECT 2 ;SELECT 3"),
            ["SELECT 1", "SELECT 2", "SELECT 3"]
        );
    }

    #[test]
    fn ignores_semicolons_inside_quotes() {
        let script = "SELECT 'a;b', \"c;d\", `e;f`; SELECT 'it''s; fine'";
        assert_eq!(
            texts(script),
            ["SELECT 'a;b', \"c;d\", `e;f`", "SELECT 'it''s; fine'"]
        );
    }

    #[test]
    fn ignores_semicolons_inside_comments() {
        let script = "SELECT 1 -- not; here\n; /* nor; here */ SELECT 2";
        assert_eq!(
            texts(script),
            ["SELECT 1 -- not; here", "/* nor; here */ SELECT 2"]
        );
    }

    #[test]
    fn skips_empty_and_comment_only_segments() {
        assert_eq!(
            texts(";; -- only a comment\n; /* and this */ ;SELECT 1;"),
            ["SELECT 1"]
        );
        assert!(split("").is_empty());
        assert!(split("   \n\t ").is_empty());
    }

    #[test]
    fn comment_only_segments_may_contain_whitespace_between_comments() {
        assert_eq!(
            texts("/* a */  \n  -- b\n  /* c */; SELECT 1"),
            ["SELECT 1"]
        );
    }

    #[test]
    fn doubled_quotes_are_escapes_not_terminators() {
        assert_eq!(texts("SELECT '''' ; SELECT 2"), ["SELECT ''''", "SELECT 2"]);
        assert_eq!(
            texts("SELECT \"a\"\"b;\" ; SELECT 2"),
            ["SELECT \"a\"\"b;\"", "SELECT 2"]
        );
    }

    #[test]
    fn a_leading_line_comment_ends_at_its_newline() {
        let script = "-- first; comment\nSELECT 1; SELECT 2";
        assert_eq!(texts(script), ["-- first; comment\nSELECT 1", "SELECT 2"]);
    }

    #[test]
    fn unterminated_quote_or_comment_runs_to_the_end() {
        assert_eq!(
            texts("SELECT 'open; still open"),
            ["SELECT 'open; still open"]
        );
        assert_eq!(
            texts("SELECT 1 /* open; comment"),
            ["SELECT 1 /* open; comment"]
        );
        assert_eq!(texts("SELECT 1 -- trailing"), ["SELECT 1 -- trailing"]);
    }

    #[test]
    fn keeps_operators_that_look_like_comment_starts() {
        assert_eq!(
            texts("SELECT 4 - -1; SELECT 8 / 2"),
            ["SELECT 4 - -1", "SELECT 8 / 2"]
        );
    }

    #[test]
    fn ranges_are_byte_offsets_into_multibyte_text() {
        let script = "SELECT 'café'; SELECT 'ü'";
        assert_eq!(split(script), [0..14, 16..27]);
    }

    #[test]
    fn statement_at_picks_the_statement_under_or_before_the_cursor() {
        let script = "SELECT 1;\n\nSELECT 2;";
        assert_eq!(statement_at(script, 3), Some(0..8));
        assert_eq!(statement_at(script, 9), Some(0..8));
        assert_eq!(statement_at(script, 11), Some(11..19));
        assert_eq!(statement_at(script, 100), Some(11..19));
    }

    #[test]
    fn statement_at_before_the_first_statement_picks_the_first() {
        assert_eq!(statement_at("\n\n  SELECT 1", 0), Some(4..12));
        assert_eq!(statement_at("-- nothing", 0), None);
    }
}
```

- [ ] **Step 3: Verify**

Run: `cargo nextest run -p quarry-engine splitter`
Expected: all 12 splitter tests pass.
Run: `just check`
Expected: exit 0. Every scanner helper is a separate small function, so each stays within the complexity limits.

- [ ] **Step 4: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): split SQL scripts into statements"
```

---

### Task 4: SQLite driver with streaming, cancellation and introspection

**Files:**
- Modify: root `Cargo.toml` (`[workspace.dependencies]`), `crates/engine/Cargo.toml`, `crates/engine/src/lib.rs` (`pub mod sqlite;`)
- Create: `crates/engine/src/sqlite/{mod,convert,run,introspect}.rs`
- Test: `crates/engine/tests/sqlite.rs`

**Interfaces:**
- Consumes: Tasks 1–2.
- Produces:
  - `SqliteDriver::open(&Path)`, `SqliteDriver::open_in_memory()` and `SqliteDriver::execute_script(&mut self, &str) -> Result<(), EngineError>`
  - `impl Driver for SqliteDriver`
  - `sqlite::connect(target: &str) -> Result<Box<dyn Driver>, EngineError>`, where `target` is a path or `:memory:` (used by Task 5)
  - `sqlite::run::BATCH_SIZE = 500`
- **Facts (verified against rusqlite 0.41.1):**
  - `Connection::progress_handler(n, Some(FnMut() -> bool))` returns `Result<()>`, and returning `true` interrupts the statement with `ErrorCode::OperationInterrupted`.
  - Prepare errors arrive as `Error::SqlInputError { msg, .. }`, whose `Display` appends `in <sql> at offset N`. We use `msg` alone.
  - `Statement::columns()` returns an iterator, and `decl_type()` needs the `column_decltype` feature.
  - `rows_affected` is the change in `total_changes()`, so DDL reports `Some(0)`, not a stale count.

- [ ] **Step 1: Add the dependencies**

Root `Cargo.toml` `[workspace.dependencies]`, keeping alphabetical order:
```toml
rusqlite = { version = "0.41.1", features = ["bundled", "hooks", "column_decltype"] }
```

`crates/engine/Cargo.toml`: add `rusqlite.workspace = true` under `[dependencies]`, and before `[lints]` add:
```toml
[dev-dependencies]
tempfile.workspace = true

```

- [ ] **Step 2: Write the failing integration tests.** Create `crates/engine/tests/sqlite.rs`:

```rust
//! Integration tests for the `SQLite` driver.

use quarry_engine::{
    driver::Driver,
    error::EngineError,
    result::{Column, QueryResult},
    schema::{ColumnInfo, TableKind},
    sqlite::SqliteDriver,
    value::Value,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CREW: &str =
    "CREATE TABLE astronauts (id INTEGER PRIMARY KEY, name TEXT NOT NULL, photo BLOB);
    INSERT INTO astronauts (name, photo) VALUES ('Valentina', x'0102'), ('Kenji', NULL);
    CREATE VIEW named AS SELECT name FROM astronauts;";

fn crew() -> Result<SqliteDriver, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script(CREW)?;
    Ok(driver)
}

fn column(name: &str, type_name: &str) -> Column {
    Column {
        name: name.to_owned(),
        type_name: type_name.to_owned(),
    }
}

#[test]
fn select_returns_typed_columns_and_values() -> TestResult {
    let mut result = QueryResult::default();
    let summary = crew()?.execute(
        "SELECT id, name, photo, 1.5 AS ratio FROM astronauts",
        &mut result,
    )?;
    assert_eq!(
        result.columns,
        [
            column("id", "INTEGER"),
            column("name", "TEXT"),
            column("photo", "BLOB"),
            column("ratio", "")
        ]
    );
    let valentina = vec![
        Value::Int(1),
        Value::Text("Valentina".to_owned()),
        Value::Bytes(vec![1, 2]),
        Value::Float(1.5),
    ];
    assert_eq!(result.rows[0], valentina);
    assert_eq!(result.rows[1][2], Value::Null);
    assert_eq!((summary.rows, summary.rows_affected), (2, None));
    Ok(())
}

#[test]
fn statements_without_rows_report_rows_affected() -> TestResult {
    let mut driver = crew()?;
    let mut result = QueryResult::default();
    let update = driver.execute("UPDATE astronauts SET name = upper(name)", &mut result)?;
    assert_eq!((update.rows, update.rows_affected), (0, Some(2)));
    let ddl = driver.execute("CREATE TABLE missions (id INTEGER)", &mut result)?;
    assert_eq!(ddl.rows_affected, Some(0));
    assert!(result.columns.is_empty() && result.rows.is_empty());
    Ok(())
}

#[test]
fn rows_arrive_in_batches_of_500() -> TestResult {
    struct Batches(Vec<usize>);
    impl quarry_engine::result::RowSink for Batches {
        fn columns(&mut self, _: Vec<Column>) {}
        fn rows(&mut self, batch: Vec<Vec<Value>>) {
            self.0.push(batch.len());
        }
    }
    let mut sink = Batches(Vec::new());
    let sql = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 1203) SELECT x FROM n";
    let summary = SqliteDriver::open_in_memory()?.execute(sql, &mut sink)?;
    assert_eq!(sink.0, [500, 500, 203]);
    assert_eq!(summary.rows, 1203);
    Ok(())
}

#[test]
fn sql_errors_carry_the_database_message() -> TestResult {
    let error = crew()?.execute("SELECT * FROM missing", &mut QueryResult::default());
    assert_eq!(
        error,
        Err(EngineError::Sql("no such table: missing".to_owned()))
    );
    let script_error = crew()?.execute_script("INSERT INTO nowhere VALUES (1)");
    assert_eq!(
        script_error,
        Err(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    Ok(())
}

#[test]
fn a_cancelled_driver_interrupts_and_can_run_again() -> TestResult {
    let mut driver = crew()?;
    let canceller = driver.canceller();
    canceller.cancel();
    let endless =
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
    assert_eq!(
        driver.execute(endless, &mut QueryResult::default()),
        Err(EngineError::Cancelled)
    );
    canceller.reset();
    let summary = driver.execute("SELECT 1", &mut QueryResult::default())?;
    assert_eq!(summary.rows, 1);
    Ok(())
}

#[test]
fn schema_lists_tables_and_views_with_columns() -> TestResult {
    let schema = crew()?.schema()?;
    let names: Vec<(&str, TableKind)> = schema
        .tables
        .iter()
        .map(|t| (t.name.as_str(), t.kind))
        .collect();
    assert_eq!(
        names,
        [("astronauts", TableKind::Table), ("named", TableKind::View)]
    );
    let id = ColumnInfo {
        name: "id".to_owned(),
        type_name: "INTEGER".to_owned(),
        primary_key: true,
    };
    let name = ColumnInfo {
        name: "name".to_owned(),
        type_name: "TEXT".to_owned(),
        primary_key: false,
    };
    assert_eq!(schema.tables[0].columns[..2], [id, name]);
    Ok(())
}

#[test]
fn files_persist_and_bad_paths_fail_to_open() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("quarry.db");
    SqliteDriver::open(&path)?.execute_script("CREATE TABLE t (x); INSERT INTO t VALUES (7);")?;
    let mut result = QueryResult::default();
    SqliteDriver::open(&path)?.execute("SELECT x FROM t", &mut result)?;
    assert_eq!(result.rows, [[Value::Int(7)]]);
    let missing_dir = dir.path().join("no/such/dir/x.db");
    let error = SqliteDriver::open(&missing_dir).err();
    assert!(
        matches!(error, Some(EngineError::Open { ref target, .. }) if target.ends_with("x.db"))
    );
    Ok(())
}
```

Run `cargo nextest run -p quarry-engine --test sqlite`. Expected: compile failure, `unresolved import quarry_engine::sqlite`.

- [ ] **Step 3: Implement the driver**

`crates/engine/src/sqlite/mod.rs`:
```rust
//! The built-in SQLite driver (bundled `SQLite`, no system library needed).

mod convert;
mod introspect;
mod run;

use std::path::Path;

use rusqlite::Connection;

use crate::{
    driver::{Canceller, Driver},
    error::EngineError,
    result::{RowSink, Summary},
    schema::Schema,
};

/// How many virtual-machine steps run between cancellation checks.
const CANCEL_CHECK_STEPS: i32 = 1_000;

/// A connection to one `SQLite` database file or in-memory database.
pub struct SqliteDriver {
    connection: Connection,
    canceller: Canceller,
}

impl SqliteDriver {
    /// Opens (or creates) the database file at `path`.
    ///
    /// # Errors
    /// Returns [`EngineError::Open`] when the file cannot be opened.
    pub fn open(path: &Path) -> Result<Self, EngineError> {
        let target = path.display().to_string();
        Self::from_connection(Connection::open(path), &target)
    }

    /// Opens a fresh, empty in-memory database.
    ///
    /// # Errors
    /// Returns [`EngineError::Open`] if `SQLite` cannot allocate it.
    pub fn open_in_memory() -> Result<Self, EngineError> {
        Self::from_connection(Connection::open_in_memory(), ":memory:")
    }

    /// Runs a whole script with no results, e.g. to load a schema and seed data.
    ///
    /// # Errors
    /// Returns [`EngineError::Sql`] for the first failing statement.
    pub fn execute_script(&mut self, sql: &str) -> Result<(), EngineError> {
        self.connection.execute_batch(sql).map_err(convert::error)
    }

    fn from_connection(
        opened: rusqlite::Result<Connection>,
        target: &str,
    ) -> Result<Self, EngineError> {
        let open_error = |error: rusqlite::Error| EngineError::Open {
            target: target.to_owned(),
            message: error.to_string(),
        };
        let mut connection = opened.map_err(open_error)?;
        let canceller = Canceller::default();
        let flag = canceller.clone();
        let handler = Some(move || flag.is_cancelled());
        connection
            .progress_handler(CANCEL_CHECK_STEPS, handler)
            .map_err(open_error)?;
        Ok(Self {
            connection,
            canceller,
        })
    }
}

impl Driver for SqliteDriver {
    fn execute(&mut self, sql: &str, sink: &mut dyn RowSink) -> Result<Summary, EngineError> {
        run::execute(&self.connection, sql, sink)
    }

    fn schema(&mut self) -> Result<Schema, EngineError> {
        introspect::schema(&self.connection)
    }

    fn canceller(&self) -> Canceller {
        self.canceller.clone()
    }
}

/// Registry entry point: `target` is a file path or `:memory:`.
///
/// # Errors
/// Returns [`EngineError::Open`] when the database cannot be opened.
pub fn connect(target: &str) -> Result<Box<dyn Driver>, EngineError> {
    let driver = if target == ":memory:" {
        SqliteDriver::open_in_memory()?
    } else {
        SqliteDriver::open(Path::new(target))?
    };
    Ok(Box::new(driver))
}
```

`crates/engine/src/sqlite/convert.rs`:
```rust
//! Converting `rusqlite` values and errors into engine types.

use rusqlite::{ErrorCode, types::ValueRef};

use crate::{error::EngineError, value::Value};

/// Converts one `SQLite` cell.
pub(super) fn value(cell: ValueRef<'_>) -> Value {
    match cell {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => Value::Int(n),
        ValueRef::Real(x) => Value::Float(x),
        ValueRef::Text(text) => Value::Text(String::from_utf8_lossy(text).into_owned()),
        ValueRef::Blob(bytes) => Value::Bytes(bytes.to_vec()),
    }
}

/// Converts a `rusqlite` error into a user-facing engine error.
pub(super) fn error(error: rusqlite::Error) -> EngineError {
    match error {
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::OperationInterrupted =>
        {
            EngineError::Cancelled
        }
        rusqlite::Error::SqliteFailure(_, Some(message))
        | rusqlite::Error::SqlInputError { msg: message, .. } => EngineError::Sql(message),
        other => EngineError::Sql(other.to_string()),
    }
}
```

`crates/engine/src/sqlite/run.rs`:
```rust
//! Executing one statement and streaming its rows in batches.

use std::time::Instant;

use rusqlite::{Connection, Statement};

use super::convert;
use crate::{
    error::EngineError,
    result::{Column, Row, RowSink, Summary},
};

/// Rows per batch handed to the sink.
pub const BATCH_SIZE: usize = 500;

/// Runs `sql` and streams its rows into `sink`.
pub(super) fn execute(
    connection: &Connection,
    sql: &str,
    sink: &mut dyn RowSink,
) -> Result<Summary, EngineError> {
    let started = Instant::now();
    let changes_before = connection.total_changes();
    let mut statement = connection.prepare(sql).map_err(convert::error)?;
    let columns = columns_of(&statement);
    if columns.is_empty() {
        statement.raw_execute().map_err(convert::error)?;
        let affected = connection.total_changes() - changes_before;
        return Ok(Summary {
            rows: 0,
            rows_affected: Some(affected),
            elapsed: started.elapsed(),
        });
    }
    let width = columns.len();
    sink.columns(columns);
    let rows = stream_rows(&mut statement, width, sink)?;
    Ok(Summary {
        rows,
        rows_affected: None,
        elapsed: started.elapsed(),
    })
}

fn columns_of(statement: &Statement<'_>) -> Vec<Column> {
    statement
        .columns()
        .map(|column| Column {
            name: column.name().to_owned(),
            type_name: column.decl_type().unwrap_or_default().to_owned(),
        })
        .collect()
}

fn stream_rows(
    statement: &mut Statement<'_>,
    width: usize,
    sink: &mut dyn RowSink,
) -> Result<u64, EngineError> {
    let mut rows = statement.raw_query();
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut total = 0;
    while let Some(row) = rows.next().map_err(convert::error)? {
        batch.push(read_row(row, width)?);
        total += 1;
        if batch.len() == BATCH_SIZE {
            sink.rows(std::mem::replace(
                &mut batch,
                Vec::with_capacity(BATCH_SIZE),
            ));
        }
    }
    if !batch.is_empty() {
        sink.rows(batch);
    }
    Ok(total)
}

fn read_row(row: &rusqlite::Row<'_>, width: usize) -> Result<Row, EngineError> {
    (0..width)
        .map(|index| {
            row.get_ref(index)
                .map(convert::value)
                .map_err(convert::error)
        })
        .collect()
}
```

`crates/engine/src/sqlite/introspect.rs`:
```rust
//! Reading tables, views and columns from the `SQLite` catalog.

use rusqlite::Connection;

use super::convert;
use crate::{
    error::EngineError,
    schema::{ColumnInfo, Schema, Table, TableKind},
};

const TABLES: &str = "SELECT name, type FROM sqlite_schema \
    WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name";

const COLUMNS: &str = "SELECT name, type, pk FROM pragma_table_info(?1) ORDER BY cid";

/// Reads every user table and view, sorted by name.
pub(super) fn schema(connection: &Connection) -> Result<Schema, EngineError> {
    let mut statement = connection.prepare(TABLES).map_err(convert::error)?;
    let entries = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(convert::error)?;
    let tables = entries
        .into_iter()
        .map(|(name, kind)| table(connection, name, &kind))
        .collect::<Result<_, _>>()?;
    Ok(Schema { tables })
}

fn table(connection: &Connection, name: String, kind: &str) -> Result<Table, EngineError> {
    let mut statement = connection.prepare(COLUMNS).map_err(convert::error)?;
    let columns = statement
        .query_map([&name], |row| {
            Ok(ColumnInfo {
                name: row.get(0)?,
                type_name: row.get(1)?,
                primary_key: row.get::<_, i64>(2)? > 0,
            })
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(convert::error)?;
    let kind = if kind == "view" {
        TableKind::View
    } else {
        TableKind::Table
    };
    Ok(Table {
        name,
        kind,
        columns,
    })
}
```

- [ ] **Step 4: Verify**

Run: `cargo nextest run -p quarry-engine --test sqlite`
Expected: all 7 tests pass. Each one is load-bearing:
- `rows_arrive_in_batches_of_500` pins the batch size.
- `a_cancelled_driver_interrupts_and_can_run_again` proves the progress-handler cancel works and that the driver recovers afterwards.
- `files_persist_and_bad_paths_fail_to_open` proves file paths really reach `Connection::open`.

Run: `just check`
Expected: exit 0. If `cargo deny` reports a duplicate dependency version brought in by rusqlite, add a `[[bans.skip]]` entry to `deny.toml` with the crate, the version and a one-line reason. Never relax a whole category.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/engine deny.toml
git commit -m "feat(engine): add SQLite driver with batched streaming and cancellation"
```

---

### Task 5: Driver registry

**Files:**
- Create: `crates/engine/src/registry.rs`
- Modify: `crates/engine/src/lib.rs` (`pub mod registry;`)
- Test: `crates/engine/tests/registry.rs`

**Interfaces:**
- Consumes: `sqlite::connect` (Task 4).
- Produces:
  - `type Connect = fn(&str) -> Result<Box<dyn Driver>, EngineError>`
  - `DriverRegistry::builtin()`, `schemes()` and `connect(url)`. The URL format is `scheme:target`: `sqlite::memory:` or `sqlite:/path/file.db`.
- **This is the driver extension point (spec §3.3).** Phase 3 adds Postgres and MySQL as one line each in `builtin()`.

- [ ] **Step 1: Write the failing tests.** Create `crates/engine/tests/registry.rs`:

```rust
//! Integration tests for connection strings and the driver registry.

use quarry_engine::{error::EngineError, registry::DriverRegistry, result::QueryResult};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn builtin_registry_knows_sqlite() {
    assert_eq!(
        DriverRegistry::builtin().schemes().collect::<Vec<_>>(),
        ["sqlite"]
    );
}

#[test]
fn connects_to_memory_and_file_databases() -> TestResult {
    let registry = DriverRegistry::builtin();
    let mut memory = registry.connect("sqlite::memory:")?;
    assert_eq!(
        memory
            .execute("SELECT 1", &mut QueryResult::default())?
            .rows,
        1
    );
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("f.db");
    let url = format!("sqlite:{}", path.display());
    registry
        .connect(&url)?
        .execute("CREATE TABLE t (x)", &mut QueryResult::default())?;
    assert!(path.exists(), "a file URL must create the file");
    assert_eq!(
        registry.connect(&url)?.schema()?.tables.len(),
        1,
        "the table must persist"
    );
    Ok(())
}

#[test]
fn rejects_unknown_or_malformed_urls() {
    let registry = DriverRegistry::builtin();
    for url in ["postgres://localhost/db", "no-scheme-here"] {
        let error = registry.connect(url).err();
        assert_eq!(
            error,
            Some(EngineError::Unsupported(url.to_owned())),
            "{url}"
        );
    }
}
```

Run `cargo nextest run -p quarry-engine --test registry`. Expected: compile failure, `unresolved import quarry_engine::registry`.

- [ ] **Step 2: Implement.** Create `crates/engine/src/registry.rs`:

```rust
//! Maps connection strings to drivers: the driver extension point.

use crate::{driver::Driver, error::EngineError, sqlite};

/// Opens a driver for the part of a connection string after `scheme:`.
pub type Connect = fn(&str) -> Result<Box<dyn Driver>, EngineError>;

/// The drivers Quarry knows, keyed by connection-string scheme.
///
/// Adding a database means adding one entry in [`DriverRegistry::builtin`].
pub struct DriverRegistry {
    schemes: Vec<(&'static str, Connect)>,
}

impl DriverRegistry {
    /// Every built-in driver.
    pub fn builtin() -> Self {
        Self {
            schemes: vec![("sqlite", sqlite::connect)],
        }
    }

    /// The registered schemes, in registration order.
    pub fn schemes(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.schemes.iter().map(|(scheme, _)| *scheme)
    }

    /// Opens `url`, e.g. `sqlite::memory:` or `sqlite:/path/to/file.db`.
    ///
    /// # Errors
    /// Returns [`EngineError::Unsupported`] for an unknown scheme, or the
    /// driver's own error when opening fails.
    pub fn connect(&self, url: &str) -> Result<Box<dyn Driver>, EngineError> {
        let unsupported = || EngineError::Unsupported(url.to_owned());
        let (scheme, target) = url.split_once(':').ok_or_else(unsupported)?;
        let (_, connect) = self
            .schemes
            .iter()
            .find(|(known, _)| *known == scheme)
            .ok_or_else(unsupported)?;
        connect(target)
    }
}
```

- [ ] **Step 3: Verify:** `cargo nextest run -p quarry-engine --test registry` (3 pass), then `just check` (exit 0).

- [ ] **Step 4: Commit**

```bash
git add crates/engine
git commit -m "feat(engine): add driver registry keyed by connection scheme"
```

---

### Task 6: Session thread with streamed events and per-run cancellation

**Files:**
- Modify: root `Cargo.toml` (`async-channel = "2.5.0"` in `[workspace.dependencies]`), `crates/engine/Cargo.toml` (`async-channel.workspace = true`), `crates/engine/src/lib.rs` (`pub mod session;`)
- Create: `crates/engine/src/session.rs`
- Test: `crates/engine/tests/session.rs`

**Interfaces:**
- Consumes: `Driver`, `Canceller`, `splitter::split` and the result types.
- Produces (1C builds the UI on these):
  - `Session::start(Box<dyn Driver>) -> Session`
  - `Session::run(&self, impl Into<String>) -> RunHandle`
  - `Session::schema(&self) -> async_channel::Receiver<Result<Schema, EngineError>>`
  - `RunHandle { pub events: async_channel::Receiver<RunEvent> }` with `cancel()`. Dropping a handle cancels its run.
  - `RunEvent { Started { index, range }, Columns(Vec<Column>), Rows(Vec<Row>), Finished(Summary), Failed(EngineError), Done }`. `Done` is always last, and the first `Failed` stops the script.
- **Concurrency design. Read this before coding:**
  - Runs execute one at a time, in submission order.
  - Each run has its own `cancelled` flag. `Shared.current_run` holds the id of the executing run.
  - `cancel()` sets the run's own flag, and **only** sets the driver's interrupt flag if this run is the one executing. So cancelling a queued run never interrupts the run in front of it.
  - Before each statement, the worker resets the driver flag and *then* re-checks the run flag. That ordering means a cancel arriving at any instant is either seen by the check or interrupts the statement.
  - The event channel is bounded (16), so a slow reader throttles the worker. A dropped reader makes sends fail, which `ChannelSink` turns into a cancel. The worker therefore never blocks forever, and the session keeps serving later runs.
  - `ActiveRun` bundles the per-run references so that no function takes more than 5 arguments.

- [ ] **Step 1: Add the dependency** as listed under Files.

- [ ] **Step 2: Write the failing tests.** Create `crates/engine/tests/session.rs`:

```rust
//! Integration tests for the session thread: events, errors, cancellation.

use quarry_engine::{
    error::EngineError,
    session::{RunEvent, RunHandle, Session},
    sqlite::SqliteDriver,
    value::Value,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ENDLESS: &str =
    "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";

fn session() -> Result<Session, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1), (2);")?;
    Ok(Session::start(Box::new(driver)))
}

/// Reads events until `Done`, which is included.
fn drain(handle: &RunHandle) -> Result<Vec<RunEvent>, async_channel::RecvError> {
    let mut events = Vec::new();
    loop {
        let event = handle.events.recv_blocking()?;
        let done = event == RunEvent::Done;
        events.push(event);
        if done {
            return Ok(events);
        }
    }
}

fn kinds(events: &[RunEvent]) -> Vec<&'static str> {
    events
        .iter()
        .map(|event| match event {
            RunEvent::Started { .. } => "started",
            RunEvent::Columns(_) => "columns",
            RunEvent::Rows(_) => "rows",
            RunEvent::Finished(_) => "finished",
            RunEvent::Failed(_) => "failed",
            RunEvent::Done => "done",
        })
        .collect()
}

#[test]
fn runs_each_statement_in_order() -> TestResult {
    let session = session()?;
    let events = drain(&session.run("UPDATE t SET x = x * 10; SELECT x FROM t ORDER BY x"))?;
    assert_eq!(
        kinds(&events),
        [
            "started", "finished", "started", "columns", "rows", "finished", "done"
        ]
    );
    assert_eq!(
        events[0],
        RunEvent::Started {
            index: 0,
            range: 0..23
        }
    );
    assert_eq!(
        events[2],
        RunEvent::Started {
            index: 1,
            range: 25..51
        }
    );
    assert_eq!(
        events[4],
        RunEvent::Rows(vec![vec![Value::Int(10)], vec![Value::Int(20)]])
    );
    Ok(())
}

#[test]
fn stops_at_the_first_error() -> TestResult {
    let session = session()?;
    let events = drain(&session.run("SELECT nope FROM t; SELECT 1"))?;
    assert_eq!(kinds(&events), ["started", "failed", "done"]);
    assert_eq!(
        events[1],
        RunEvent::Failed(EngineError::Sql("no such column: nope".to_owned()))
    );
    Ok(())
}

#[test]
fn an_empty_script_is_just_done() -> TestResult {
    assert_eq!(drain(&session()?.run("  -- nothing\n"))?, [RunEvent::Done]);
    Ok(())
}

#[test]
fn cancel_interrupts_the_running_statement_and_skips_the_rest() -> TestResult {
    let session = session()?;
    let handle = session.run(format!("{ENDLESS}; SELECT 1"));
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Started { index: 0, .. }
    ));
    handle.cancel();
    let rest = drain(&handle)?;
    let ending = [RunEvent::Failed(EngineError::Cancelled), RunEvent::Done];
    assert!(rest.ends_with(&ending), "{rest:?}");
    let second_started = rest.iter().any(|e| matches!(e, RunEvent::Started { .. }));
    assert!(!second_started, "the second statement must not start");
    Ok(())
}

#[test]
fn cancelling_a_queued_run_leaves_the_running_one_alone() -> TestResult {
    let session = session()?;
    let busy = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 300000) SELECT count(*) FROM n";
    let first = session.run(busy);
    let second = session.run("SELECT 1");
    assert!(matches!(
        first.events.recv_blocking()?,
        RunEvent::Started { .. }
    ));
    second.cancel();
    assert_eq!(
        kinds(&drain(&first)?),
        ["columns", "rows", "finished", "done"]
    );
    assert_eq!(
        drain(&second)?,
        [RunEvent::Failed(EngineError::Cancelled), RunEvent::Done]
    );
    Ok(())
}

#[test]
fn dropping_a_handle_stops_its_run_and_frees_the_session() -> TestResult {
    let session = session()?;
    let handle = session.run(ENDLESS);
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Started { .. }
    ));
    // After `Columns`, an aggregate sends nothing until it finishes, so only
    // the handle's `Drop` cancel can stop it.
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Columns(_)
    ));
    drop(handle);
    let next = drain(&session.run("SELECT 42"))?;
    assert_eq!(next[2], RunEvent::Rows(vec![vec![Value::Int(42)]]));
    Ok(())
}

#[test]
fn a_dropped_reader_of_a_large_result_does_not_block_the_session() -> TestResult {
    let session = session()?;
    let many = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 100000) SELECT x FROM n";
    drop(session.run(many));
    assert_eq!(
        kinds(&drain(&session.run("SELECT 1"))?),
        ["started", "columns", "rows", "finished", "done"]
    );
    Ok(())
}

#[test]
fn schema_is_read_on_the_session_thread() -> TestResult {
    let session = session()?;
    let schema = session.schema().recv_blocking()??;
    assert_eq!(schema.tables[0].name, "t");
    Ok(())
}
```

Run `cargo nextest run -p quarry-engine --test session`. Expected: compile failure, `unresolved import quarry_engine::session`.

- [ ] **Step 3: Implement.** Create `crates/engine/src/session.rs`:

```rust
//! Runs a driver on its own thread and streams results back as events.
//!
//! The UI submits scripts with [`Session::run`] and reads [`RunEvent`]s from
//! the returned [`RunHandle`] without ever blocking. Runs execute one at a
//! time, in submission order. Dropping a handle stops its run.

use std::{
    ops::Range,
    sync::{
        Arc,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
};

use async_channel::{Receiver, Sender};

use crate::{
    driver::{Canceller, Driver},
    error::EngineError,
    result::{Column, Row, RowSink, Summary},
    schema::Schema,
    splitter,
};

/// Batches buffered per run before the session thread waits for the reader.
const EVENT_BUFFER: usize = 16;

/// Something that happened while running a script.
#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    /// Statement `index` (0-based) started; `range` locates it in the script.
    Started {
        /// Position of the statement in the script.
        index: usize,
        /// Byte range of the statement within the script.
        range: Range<usize>,
    },
    /// The current statement's columns (only for statements returning rows).
    Columns(Vec<Column>),
    /// A batch of the current statement's rows.
    Rows(Vec<Row>),
    /// The current statement finished.
    Finished(Summary),
    /// The current statement failed; later statements do not run.
    Failed(EngineError),
    /// The run is over. Always the last event.
    Done,
}

/// A database connection running on its own thread.
pub struct Session {
    jobs: mpsc::Sender<Job>,
    shared: Arc<Shared>,
    next_id: AtomicU64,
}

/// The receiving end of one run.
pub struct RunHandle {
    /// Events for this run, ending with [`RunEvent::Done`].
    pub events: Receiver<RunEvent>,
    run: Arc<RunState>,
    shared: Arc<Shared>,
}

struct Shared {
    driver_canceller: Canceller,
    current_run: AtomicU64,
}

struct RunState {
    id: u64,
    cancelled: AtomicBool,
}

enum Job {
    Run {
        script: String,
        run: Arc<RunState>,
        events: Sender<RunEvent>,
    },
    Schema(Sender<Result<Schema, EngineError>>),
}

impl Session {
    /// Moves `driver` onto a new session thread.
    pub fn start(driver: Box<dyn Driver>) -> Self {
        let shared = Arc::new(Shared {
            driver_canceller: driver.canceller(),
            current_run: AtomicU64::new(0),
        });
        let (jobs, inbox) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        thread::spawn(move || work(driver, &inbox, &worker_shared));
        Self {
            jobs,
            shared,
            next_id: AtomicU64::new(1),
        }
    }

    /// Queues `script`; its statements run in order, stopping at the first error.
    pub fn run(&self, script: impl Into<String>) -> RunHandle {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let run = Arc::new(RunState {
            id,
            cancelled: AtomicBool::new(false),
        });
        let (events, receiver) = async_channel::bounded(EVENT_BUFFER);
        let job = Job::Run {
            script: script.into(),
            run: Arc::clone(&run),
            events: events.clone(),
        };
        if self.jobs.send(job).is_err() {
            let _ = events.try_send(RunEvent::Failed(EngineError::WorkerGone));
            let _ = events.try_send(RunEvent::Done);
        }
        RunHandle {
            events: receiver,
            run,
            shared: Arc::clone(&self.shared),
        }
    }

    /// Reads the schema on the session thread, after any queued runs.
    pub fn schema(&self) -> Receiver<Result<Schema, EngineError>> {
        let (reply, receiver) = async_channel::bounded(1);
        if let Err(mpsc::SendError(Job::Schema(reply))) = self.jobs.send(Job::Schema(reply)) {
            let _ = reply.try_send(Err(EngineError::WorkerGone));
        }
        receiver
    }
}

impl RunHandle {
    /// Cancels this run: the running statement is interrupted and later
    /// statements are skipped. Has no effect once the run is done.
    pub fn cancel(&self) {
        self.run.cancelled.store(true, Ordering::SeqCst);
        if self.shared.current_run.load(Ordering::SeqCst) == self.run.id {
            self.shared.driver_canceller.cancel();
        }
    }
}

impl Drop for RunHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn work(mut driver: Box<dyn Driver>, inbox: &mpsc::Receiver<Job>, shared: &Shared) {
    while let Ok(job) = inbox.recv() {
        match job {
            Job::Run {
                script,
                run,
                events,
            } => {
                let active = ActiveRun {
                    shared,
                    script: &script,
                    state: &run,
                    events: &events,
                };
                active.execute(driver.as_mut());
            }
            Job::Schema(reply) => {
                let _ = reply.send_blocking(driver.schema());
            }
        }
    }
}

/// One run being executed on the session thread.
struct ActiveRun<'a> {
    shared: &'a Shared,
    script: &'a str,
    state: &'a RunState,
    events: &'a Sender<RunEvent>,
}

impl ActiveRun<'_> {
    fn execute(&self, driver: &mut dyn Driver) {
        self.shared
            .current_run
            .store(self.state.id, Ordering::SeqCst);
        for (index, range) in splitter::split(self.script).into_iter().enumerate() {
            if !self.statement(driver, index, range) {
                break;
            }
        }
        self.shared.current_run.store(0, Ordering::SeqCst);
        let _ = self.events.send_blocking(RunEvent::Done);
    }

    /// Runs one statement; returns whether the script should continue.
    fn statement(&self, driver: &mut dyn Driver, index: usize, range: Range<usize>) -> bool {
        self.shared.driver_canceller.reset();
        if self.state.cancelled.load(Ordering::SeqCst) {
            let _ = self
                .events
                .send_blocking(RunEvent::Failed(EngineError::Cancelled));
            return false;
        }
        let started = RunEvent::Started {
            index,
            range: range.clone(),
        };
        if self.events.send_blocking(started).is_err() {
            return false;
        }
        let canceller = &self.shared.driver_canceller;
        let mut sink = ChannelSink {
            events: self.events,
            canceller,
            open: true,
        };
        let outcome = driver.execute(&self.script[range], &mut sink);
        let keep_going = outcome.is_ok() && sink.open;
        let event = outcome.map_or_else(RunEvent::Failed, RunEvent::Finished);
        self.events.send_blocking(event).is_ok() && keep_going
    }
}

/// Forwards results to the run's channel; a closed channel cancels the run.
struct ChannelSink<'a> {
    events: &'a Sender<RunEvent>,
    canceller: &'a Canceller,
    open: bool,
}

impl ChannelSink<'_> {
    fn send(&mut self, event: RunEvent) {
        if self.open && self.events.send_blocking(event).is_err() {
            self.open = false;
            self.canceller.cancel();
        }
    }
}

impl RowSink for ChannelSink<'_> {
    fn columns(&mut self, columns: Vec<Column>) {
        self.send(RunEvent::Columns(columns));
    }

    fn rows(&mut self, batch: Vec<Row>) {
        self.send(RunEvent::Rows(batch));
    }
}
```

- [ ] **Step 4: Verify, including for flakiness**

Run: `for i in 1 2 3 4 5; do cargo nextest run -p quarry-engine --test session || break; done`
Expected: all 8 tests pass on all 5 runs. The cancel tests synchronize on the `Started` event, not on timing, so they must never flake. If one does, report it as BLOCKED with the output; don't add sleeps.
Run: `just check`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock crates/engine
git commit -m "feat(engine): run drivers on a session thread with streamed, cancellable runs"
```

---

### Task 7: ADR 0007, mutation gate and the PR

**Files:**
- Create: `docs/adr/0007-synchronous-drivers-on-a-session-thread.md`
- Modify: `docs/superpowers/specs/2026-11-04-quarry-design.md` §3.4

- [ ] **Step 1: Write ADR 0007** using `docs/adr/0000-template.md`, with Status Accepted and Date `2026-11-06`:
- **Context:**
  - Spec §3.4 sketched an `async` `Driver` trait.
  - SQLite, the only Phase 1 driver, is synchronous.
  - GPUI has its own executor, so tying drivers to Tokio would push a runtime into the engine's API.
- **Decision:**
  - `Driver` is synchronous, and a `Session` runs it on a dedicated thread, streaming `RunEvent`s over a bounded `async-channel`. UI code can `await` them on any executor.
  - Cancellation is per run: a run flag plus a driver flag checked by SQLite's progress handler every 1,000 VM steps. Phase 3's async drivers will run their own runtime inside the session thread.
  - YAGNI cuts:
    - The destructive-statement classifier moves to Phase 3 (production guardrails).
    - Table row counts are deferred.
    - `Value` has only the variants SQLite produces.
    - Sample data lives in `app` (1C), not in the engine.
- **Consequences:**
  - The UI never blocks, and a slow reader throttles the worker.
  - One running query per session, which is fine for one tab per session.
  - Async drivers will need a small runtime inside the session thread.

- [ ] **Step 2: Update spec §3.4.** Replace the `#[async_trait] pub trait Driver …` code block with the synchronous trait from Task 2. After the bullet list in §3.4, add: "Drivers are synchronous and run on a `Session` thread; see ADR 0007."

- [ ] **Step 3: Run the mutation gate on the engine**

Run: `just mutants`
Expected: exit 0. Reference result from verifying this plan: **no missed mutants**. Timeouts are counted as caught, since they're mutants that disable cancellation and make tests hang.
If any mutant is missed, add a test that kills it rather than accepting it. The exception is a genuinely equivalent mutant, which you must explain in the PR description.

- [ ] **Step 4: Run the full gate and commit**

Run: `just check` (exit 0).
```bash
git add docs
git commit -m "docs: record synchronous drivers on a session thread (ADR 0007)"
```

- [ ] **Step 5: Open the PR and watch CI**

```bash
git push -u origin phase-1a-engine
gh pr create --base main --title "Phase 1A: engine (SQLite driver, splitter, session)" --body "Implements docs/superpowers/plans/2026-11-06-phase-1a-engine.md. ADR 0007 records the synchronous-driver decision."
gh pr checks --watch
```
Expected: `check` and `mutants` pass. Don't merge; the controller merges after the final review.
