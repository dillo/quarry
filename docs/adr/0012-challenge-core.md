# ADR 0012: Challenge core

- **Status:** Accepted
- **Date:** 2026-11-16

## Context
Phase 2 ("Challenges") is split into five slices (`docs/superpowers/specs/2026-11-16-phase-2a-challenge-core-design.md`). Phase 2A builds the core the others stand on: the content format and loader, sandboxes, the Query kind and the grading pipeline, with a thin playable path in the app. The brief panel, the feedback card, hints, progress, Mutation challenges and the on-disk template all come later, so 2A has to leave room for them without building them.

## Decision
- **Tracks are folders, embedded at compile time.** The `challenges` crate's build script turns every folder in `content/tracks/` into a `RawTrack` of `include_str!`s, so adding a track needs no code. `track::parse` checks a folder and reports a `ContentError` naming the file and field; unknown fields, ids that aren't lowercase letters, digits and hyphens, a blank solution, a hint count other than three, a repeated id, an unknown kind, and `verify` (until Mutation challenges exist) are all refused. `Catalog::load` leaves a bad track out and returns its error beside the catalog; the content tests make sure none ships.
- **Templates stay in memory until 2D.** A track's `Template` is an in-memory database built from its schema and seed the first time one of its challenges is opened, then kept. `Template::fresh` copies it with SQLite's backup API (`SqliteDriver::copy_of`). Query challenges never change data, so nothing is lost; the on-disk template, the persistent working copy and "Reset track database" (parent spec §5.2) arrive with Mutation challenges.
- **Grading is synchronous and GPUI-free.** `check` runs the reference solution and the answer, each on its own fresh copy, through `run::run`: every statement runs, the last one's result is the answer, a statement returning more than 10,000 rows stops the run, and a watchdog thread trips the driver's canceller after 5 seconds. The app calls `check` on GPUI's background executor, so it never touches a session channel.
- **Kinds and rules are registries.** `ChallengeKind` turns an answer into the expected and actual results; `KindRegistry::standard()` holds Query. `CheckRule` is one step of the ordered pipeline (columns, row count, contents, order), and `rules::first_failure` stops at the first `CheckFailure`, which carries counts and names, never the expected rows. Cells compare with `rules::value::same`: `NULL` equals `NULL`, an integer equals the same float, floats match within a 1e-9 relative tolerance, and text and bytes match exactly.
- **A challenge tab is a query tab with a sandbox.** The workspace keeps a session per challenge tab, on a fresh copy of its track, beside the shared sample session. Runs, Explain, cancel, the schema tree and the status bar's database label follow the active tab. Picking a challenge in the palette focuses its tab or opens one holding the skeleton. Schema v3 stores each tab's challenge id, so challenge tabs and their drafts survive restarts; a tab whose challenge is gone comes back plain, keeping its text.
- **The Check view stands in for the feedback card.** A challenge tab's `ResultViewRegistry` adds a Check view. ⌘⇧↵ grades the whole editor text and shows "Solved.", "Not quite yet." with what differs, or why nothing was checked; `model::verdict` writes those sentences. A newer check replaces one still running. In a plain tab, ⌘⇧↵ says checking needs a challenge tab.

## Consequences
- A new track is a folder; a new kind or rule is a module and one registration line.
- Expected results are computed at every check, never stored, so content changes can't leave stale answers behind.
- Every check copies the template twice; with 2A's hand-written seed that takes about a millisecond.
- Templates are rebuilt on each launch until 2D stores them on disk.
- The Check view is plain text; 2B replaces it with the feedback card without changing the grader.
