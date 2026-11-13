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
