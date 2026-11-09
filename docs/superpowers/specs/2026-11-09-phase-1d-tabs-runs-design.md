# Phase 1D: Tabs, drafts, Run all, Messages and Plan

Status: approved in brainstorming on 2026-11-09. Parent spec: `2026-11-04-quarry-design.md` (§3.3, §4.2, §7).

## 1. Scope

Phase 1 is split into 1C (app shell, merged in #6), **1D (this spec)** and 1E (the ⌘K palette and copy/export).

**In 1D:**
- Query tabs (⌘T opens one, ⌘W closes one, ⌘⇧] and ⌘⇧[ switch between them).
- Drafts that survive restarts.
- Run all (⌘⇧R), with a result tab for each statement that returns rows.
- Results, Messages and Plan views through `ResultViewRegistry`.
- Explain (⌘E).
- Every deferred minor from the 1C final review (§7).

**Not in 1D:** ⌘K, copy/export (1E); challenge tabs and Check answer (Phase 2); renaming tabs; restoring results, cursors or selections.

## 2. Global constraints

These come from the parent spec and bind every task.

- **No completion or assistance of any kind.** No completion, hover, definition or code-action providers, no inline suggestions, no hints in the editor.
- **Function size:** at most 15 statements and 40 lines per function. Each unit has a single responsibility.
- **Extension points:** new kinds of things go through traits and registries (§3.3 of the parent spec). There are no `match` statements on a registry-owned enum outside its module.
- **Dependency direction:**
  - `engine` depends on no workspace crate.
  - `ui-kit` depends only on GPUI.
  - `store` depends on no workspace crate apart from `engine`, and 1D needs none.
  - `xtask check-deps` enforces all three.
- **Coverage and mutation testing:**
  - `engine`, `store`, `crates/app/src/model` and `crates/app/src/sample` have 100% line coverage and are mutation-tested; the gate is at least 90% of mutants caught.
  - Views are covered by GPUI tests.
- **No `unwrap`/`expect`/`panic!`/`unreachable!` outside `#[test]` functions**, test helpers included.
- **Polling:** GPUI code never awaits a session channel. It polls with `model::drain` every `POLL` (33 ms). Background work uses GPUI's executor, so tests stay deterministic.
- **Row cap:** each statement keeps at most `ROW_LIMIT` = 100,000 rows and counts the rest.
- **Exact versions:** gpui-kit =0.9.0 and rusqlite 0.41.1 (the workspace dependency).
- **Merging:** only with the user's explicit go-ahead.

## 3. Architecture

### 3.1 `quarry-store` (new contents)

The store depends on rusqlite (the workspace dependency) and no workspace crate.

```rust
pub struct Draft { pub id: i64, pub title: String, pub text: String }
pub struct Drafts { pub tabs: Vec<Draft>, pub active: usize }

pub trait DraftStore {
    fn load(&self) -> Result<Drafts, StoreError>;
    fn save(&self, drafts: &Drafts) -> Result<(), StoreError>;
}

pub struct SqliteStore { /* rusqlite::Connection */ }
impl SqliteStore {
    pub fn open(path: &Path) -> Result<Self, StoreError>; // creates parent dirs, migrates
    pub fn in_memory() -> Result<Self, StoreError>;
}
```

- **Migrations:** an ordered list of SQL steps, applied in a transaction. The schema version is `PRAGMA user_version`.
  - A file with a newer `user_version` than this build knows is refused with `StoreError::TooNew { found, supported }` and never modified.
- **Schema v1:**
  - `drafts(id INTEGER PRIMARY KEY, position INTEGER NOT NULL, title TEXT NOT NULL, text TEXT NOT NULL)`
  - `state(key TEXT PRIMARY KEY, value TEXT NOT NULL)`, where the `active` key holds the active tab's index.
- **`save`** replaces every row in one transaction.
- **`load`** of an empty store returns no tabs and `active` 0. An `active` index past the last tab is clamped to the last tab.
- **`StoreError`** is a `thiserror` enum with readable messages: `Open`, `TooNew`, `Sql(String)`.
- **File location:** `shell` passes `~/Library/Application Support/Quarry/quarry.db`. The path is never hard-coded inside the store.

### 3.2 `quarry-engine` additions

```rust
pub struct Plan { pub nodes: Vec<PlanNode> }
pub struct PlanNode { pub label: String, pub children: Vec<PlanNode> }

// on Driver, with a default:
fn plan(&mut self, sql: &str) -> Result<Plan, EngineError> { Err(EngineError::PlanUnsupported) }

// on Session, mirroring schema():
pub fn plan(&self, sql: impl Into<String>) -> Receiver<Result<Plan, EngineError>>;
```

- **New variant:** `EngineError::PlanUnsupported`, whose message is "This database can't show query plans."
- **SQLite** runs `EXPLAIN QUERY PLAN <sql>` and builds the tree from `id`, `parent` and `detail`.
  - Rows whose parent is 0 are roots.
  - The order of the rows is kept.
- **`Session::plan`** runs after any queued run, like `schema()`.

### 3.3 App: `model/` (GPUI-free, 100% coverage, mutation-tested)

| Module | Responsibility |
|---|---|
| `tabs.rs` | `TabList`: the ordered tab ids, the active index, and a "Query N" name taking the next N after the highest one in use. Also the `open`, `close` (closing the last tab opens a fresh one), `next` and `previous` (both wrap) and `activate` rules. |
| `autosave.rs` | `Autosave`: `edited(now)`, `due(now) -> bool` (1 s after the last edit) and `saved()`. Pure timing, with no I/O. |
| `statements.rs` | `RunOutput`: splits one run's `RunEvent`s into a `StatementOutput` per statement. Each holds the index, the range, a label (number plus first words, 24 characters at most, whitespace collapsed), its columns, its rows (`ROW_LIMIT` cap and a truncated count), and either a `Summary` or an `EngineError`. Absorbs today's per-statement handling in `RunModel`. |
| `messages.rs` | One `MessageLine { statement, text, is_error }` per statement: "N rows · t", "N rows affected · t", "Done · t", the error text, or "Cancelled". |
| `plan_tree.rs` | Flattens a `Plan` into `PlanRow { depth, label }`, depth first. |
| `status.rs` (changed) | Adds the waiting state ("Waiting for the previous run…") and the whole-run time (M7). `failure` returns `Failure { message, cancelled }` (M14). |
| `format.rs` (changed) | `cell_text` borrows `&Value`, truncates before normalising, and maps `\t` and other control characters to spaces (M4). |

### 3.4 App: `views/`

- **`query_tab.rs`:** a `QueryTab` entity. It holds the editor, the current `RunHandle` (`run`), the pump task (`pump`), a `RunOutput`, the selected result view, and the selected statement tab.
  - It takes its events through an injected `Receiver<RunEvent>`, so tests can supply any channel.
  - Most of 1C's `workspace/runner.rs` moves here.
- **`tab_bar.rs`:** the query tab strip, using ui-kit `Tab`, with a "+" button.
- **`results/`:**
  - The `ResultView` trait: `id`, `title`, `render(&QueryTab, …)`.
  - `ResultViewRegistry`, holding Results, Messages and Plan in that order. Adding a view is a new module plus one registration line.
  - The views are picked with the ui-kit segmented control.
  - Results contains the per-statement tab strip and the grid (1C's `ResultsDelegate`).
- **`workspace/`** keeps:
  - the `Session`;
  - the schema sidebar;
  - the `TabList` and its `Entity<QueryTab>`s;
  - the status bar, which reads the active tab;
  - `Autosave` and the `DraftStore`, as `Box<dyn DraftStore>`.

### 3.5 App: `shell/`

- **New actions and bindings:**
  - NewTab ⌘T, CloseTab ⌘W, NextTab ⌘⇧], PreviousTab ⌘⇧[;
  - RunAll ⌘⇧R, Explain ⌘E.
  - Run and Cancel also bind in the `Workspace > Input` context, as in 1C.
- **Menus:** File gets New Tab and Close Tab. Query gets Run All and Explain. View gets Next Tab and Previous Tab.
- **Follow the system appearance** from `open_main`, not from `Workspace` (M12).
- **Quit when the last window closes** (M13).
- **Opening the store:** if it fails, the workspace gets a store that keeps drafts in memory, and the warning reason is shown (§5).

## 4. Behaviour

### 4.1 Runs

- **One `Session`, queued runs.** The session thread runs queued scripts in order.
- **Waiting status:** a tab whose run has had no `Started` yet shows "Waiting for the previous run…".
- **Cancel:** ⌘. cancels the active tab's run.
- **Run (⌘↵)** runs the selection or the statement under the cursor, using 1C's `run_target`, and selects the Results view.
- **Run all (⌘⇧R)** sends the whole editor text.
  - Each statement that returns rows gets a result tab, and the first such tab is selected.
  - If no statement returns rows, the Messages view is selected.
- **Messages** lists every statement that started.
- **Failure:**
  - The statements that finished keep their outputs, and the failing statement's line is marked as an error.
  - The red banner (1C's `failure_banner`) shows above whichever view is open.
- **Re-running a tab** replaces that tab's output, clears the grid's selection and scrolls it to the top (M3). Other tabs are unaffected.
- **Schema refresh:** after every run, the schema sidebar refreshes, as in 1C.
- **Status-bar time:** the timer measures from submission to `Done`, both while running and at the end (M7). Each statement's own time appears in Messages.
- **Throughput (M2):** when a drain empties a full `EVENT_BUFFER`, the pump drains again straight away, up to 8 times per tick. The 200k-row stream time is measured before and after and recorded in ADR 0010.

### 4.2 Explain (⌘E)

- ⌘E calls `session.plan` on the run target and selects the Plan view.
- The Plan view shows `plan_tree` rows indented by depth in Menlo.
- An error, including `PlanUnsupported`, is shown as a message inside the Plan view, not in the run banner.
- Explain does not change the tab's run output.

### 4.3 Tabs and drafts

- **On launch:** the saved tabs and the active tab are loaded. If nothing is saved, Quarry opens "Query 1" with 1C's `WELCOME` text.
- **⌘T** opens an empty "Query N" after the active tab and focuses its editor.
- **⌘W** closes the active tab without a prompt and activates its right neighbour, or its left neighbour if it was last. Closing the only tab replaces it with a fresh empty "Query N".
- **Saving:**
  - Edits call `Autosave::edited`.
  - A save happens 1 s after the last edit.
  - It also happens straight away on ⌘T, ⌘W, a tab switch and app quit.
  - Saves run on GPUI's background executor.
- **What is saved:** titles, texts, order and the active index. Results, cursors and selections are not saved.

## 5. Errors

| Situation | Behaviour |
|---|---|
| Store can't be opened (permissions, corrupt file) | Drafts are kept in memory. The status bar shows "Drafts aren't being saved", with the error as its tooltip. |
| Store file is from a newer Quarry | The same warning, with "Saved by a newer Quarry (version N)". The file is never modified. |
| A save fails | The same warning. Saving is retried on the next edit, and the warning clears after a successful save. |
| Plan unsupported or failed | A message inside the Plan view. |
| Run fails mid-script | Earlier statements keep their outputs. Messages marks the failing line. The banner is shown. |
| Session channel closes without `Done` | `WorkerGone`, as in 1C, now covered by a view test with an injected channel. |

## 6. Testing

- **Store:**
  - round-trip in memory and through a temporary file;
  - migrating an empty file;
  - refusing a `TooNew` file, which is left byte-identical;
  - saving with no tabs;
  - clamping `active`.
- **Engine:**
  - SQLite plans for a join, a correlated subquery and an index lookup, with the tree shape asserted;
  - the default `plan` returns `PlanUnsupported`;
  - `Session::plan` after a queued run.
- **Models:** every module in §3.3 has unit tests for each rule listed there.
- **Views (GPUI tests):**
  - ⌘T, ⌘W and switching, with each tab keeping its output;
  - closing the last tab;
  - Run all producing the right result tabs and Messages lines;
  - Run all stopping at an error;
  - the waiting status while another tab runs;
  - ⌘E filling the Plan view;
  - drafts restored by a second workspace on the same store;
  - a failing `DraftStore` fake showing the warning;
  - quitting on the last window closing;
  - the vanished-worker path;
  - the editor having no completion, hover, definition or code-action providers (M10, kept);
  - the grid resetting its selection and scroll on a re-run.
- **Visual review** by the user before the PR.

## 7. 1C minors folded in

- **M2:** pump re-polling (§4.1).
- **M3:** grid reset on re-run (§4.1).
- **M4:** borrowed values and truncating before normalising, plus `\t` and other control characters (§3.3).
- **M6:** drop `Clone` on `RunModel`; `statements()` is replaced by `RunOutput`.
- **M7:** whole-run time (§4.1).
- **M9:** the `pump` and `run` field names in `QueryTab`.
- **M11:** test `open()` returns `Option`.
- **M12:** follow the appearance from `shell`.
- **M13:** quit on the last window closing.
- **M14:** the `Failure` struct.
- **`drain.rs`:** re-wrap the doc comment.
- **Vanished-worker path:** a view test.

## 8. Tasks

1. `quarry-store`: `DraftStore`, `SqliteStore`, migrations, `StoreError`.
2. Engine: `Plan`, `Driver::plan`, `EngineError::PlanUnsupported`, the SQLite plan, `Session::plan`.
3. Model: `tabs`, `autosave`.
4. Model: `statements`, `messages`, `plan_tree`; M4 (formatting), M6, M7, M14.
5. Move the editor and run into `QueryTab` with no change in behaviour; M2, M3, M4 (view), M9, M11, the vanished-worker test.
6. Tab bar, ⌘T, ⌘W, ⌘⇧] and ⌘⇧[; draft persistence and the warning.
7. `ResultViewRegistry`, the Results/Messages switch, per-statement result tabs, Run all.
8. Plan view and Explain (⌘E).
9. Shell (M12, M13, the store path, menus), ADR 0010 "Tabs and drafts", README, and a parent-spec §4.2 note.

As in 1C, a verified prototype is built first, and the plan is generated from it.
