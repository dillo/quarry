# ADR 0009: App shell structure

- **Status:** Accepted; tabs, per-statement results and run driving are updated by ADR 0010
- **Date:** 2026-11-09

## Context
Phase 1C builds the first usable Quarry window: the sample database, the schema sidebar, the SQL editor, Run and Cancel, a streaming results grid and the status bar. The `app` crate's shape is set here for every later screen.

## Decision
- **Three layers in `app`:**
  - `model` holds GPUI-free view models: the run target, the run state built from `RunEvent`s, display formatting, the status summary and the schema-tree rows. They are pure, fully unit-tested and mutation-tested like the core crates (`.cargo/mutants.toml` excludes only `views`, `shell` and `main.rs`).
  - `views` renders them. The workspace is split into its state and commands (`workspace/mod.rs`), run driving (`workspace/runner.rs`) and drawing (`workspace/layout.rs`).
  - `shell` owns the actions, key bindings, menus and the main window. Adding a command is one `actions!` entry, one key binding and one menu item.
- **Polling, not awaiting:** the workspace polls the session's channels every 33 ms (`POLL`) with `try_recv`, rather than awaiting them. A waker fired from the session thread wakes GPUI's executor from outside, which the test scheduler rejects as non-deterministic. Polling also batches many row events into one redraw, and the same poll drives the live timer.
- **One result at a time:** a Run sends the selection, or the statement under the cursor. With several statements, each `Started` replaces the grid's result. The Messages view and one tab per statement come in Phase 1D. The grid keeps the first 100,000 rows of a statement, and the status bar counts the rest.
- **Key bindings in the editor:** the editor binds ⌘↵ (newline) and ⌘. (code actions) itself. Quarry binds Run and Cancel again in the `Workspace > Input` context, which is deeper than the editor's own, so its bindings win.
- **Keyboard focus:** interactive sidebar rows, tabs and segments are tab stops (`keyboard_focusable`). Enter and Space click them, and a `FocusRing` border (3:1 on every surface) shows while they're focused. The editor keeps Tab for indentation, so ⌘1 focuses the sidebar's first table and ⌘2 returns to the editor. Components keep a transparent 1 px border so gaining focus never moves them.
- **Sample database:** `sample::open` seeds an in-memory SQLite database from `seed.sql`. It's a small space agency with 200,000 telemetry rows to stream and cancel. Phase 2's track sandboxes replace it.

## Consequences
- Logic that can be pure goes in `model` and carries the core crates' guarantees. GPUI code stays thin.
- Results reach the screen up to 33 ms late, which no one can see.
- ⌘1 on a closed sidebar focuses it on the next frame. GPUI's test platform doesn't run next-frame callbacks, so that path is checked by hand in the visual review.
