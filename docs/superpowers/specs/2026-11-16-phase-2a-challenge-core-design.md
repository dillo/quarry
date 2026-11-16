# Phase 2A: the challenge core

Status: approved in brainstorming on 2026-11-16. Parent spec: `2026-11-04-quarry-design.md` (§3.3, §4.2, §5, §9). Previous phase: `2026-11-13-phase-1e-palette-copy-design.md`.

## 1. Scope

Phase 2 ("Challenges") is split into five slices:

| Slice | Scope |
|---|---|
| **2A** (this spec) | The `challenges` crate core: content format and loader, in-memory sandboxes, the Query kind and the grading pipeline. A thin end-to-end path in the app, plus a first part of Vinyl Vault |
| 2B | The brief inspector, the feedback card, the aurora pulse, Prev/Next |
| 2C | Progress and streaks in `store`, progressive hints (`HintPolicy`), Reveal solution |
| 2D | The Mutation kind, the on-disk template with a persistent working copy and "Reset track database", `cargo xtask gen-seed`, the Home screen |
| 2E | The rest of Vinyl Vault, Orbital Agency, Metro Lines and `content/COVERAGE.md` |

**In 2A:**
- Tracks embedded at compile time from `content/tracks/`; adding a track needs no code.
- A loader that validates every track and reports bad content precisely.
- In-memory sandboxes: one template per track and a fresh copy for every run that needs one.
- The **Query** challenge kind and the four-rule grading pipeline (§5.3–§5.4 of the parent spec).
- A thin playable path in the app:
  - challenges in the ⌘K palette;
  - a challenge tab per challenge, on its own sandbox, kept across restarts;
  - **Check answer (⌘⇧↵)** with a plain-text **Check** result view.
- Vinyl Vault's first five challenges on a small hand-written seed.

**Not in 2A:** everything listed for 2B–2E above. In particular there is no brief panel, no hint is shown, nothing records progress, Mutation challenges are rejected by the loader, and nothing is written to disk apart from drafts.

## 2. Global constraints

These come from the parent spec and bind every task.

- **No completion or assistance of any kind.**
  - The editor gets no completion, hover, definition or code-action providers, no inline suggestions and no hints.
  - The palette navigates and runs commands; it never offers SQL.
  - A wrong answer says what is wrong, never how to fix it.
- **Function size:** at most 15 statements and 40 lines per function, cyclomatic ≤ 10, cognitive ≤ 15, ≤ 5 arguments and nesting ≤ 4, tests included. Each unit has a single responsibility.
- **Extension points:** new kinds of things go through traits and registries: `ChallengeKind`/`KindRegistry`, `CheckRule` (the ordered pipeline), `PaletteSource`/`PaletteSourceRegistry` and `ResultViewRegistry`. No `match` on a registry-owned enum outside its module. A new track is a folder and nothing else.
- **Dependency direction:**
  - `engine` depends on no workspace crate.
  - `challenges` depends only on `engine`.
  - `ui-kit` depends only on GPUI.
  - `store` depends on no workspace crate apart from `engine`.
  - `cargo xtask check-deps` enforces all of these.
- **Coverage and mutation testing:**
  - `engine`, `challenges` and `store` need at least 95% lines and 90% regions.
  - The app needs at least 80% lines and 70% regions; every file in `crates/app/src/model` stays at 100% lines.
  - Mutation testing finds 0 missed in `engine`, `challenges`, `store`, `crates/app/src/model` and `crates/app/src/sample`.
- **No `unwrap`/`expect`/`panic!`/`unreachable!`/`todo!`/`unimplemented!`/`dbg!` outside `#[test]` functions**, test helpers included. No lint suppressions and no `as` casts.
- **Polling:** GPUI code never awaits a session channel. It polls every `POLL` (33 ms). Background work, including grading, uses GPUI's executor, so tests stay deterministic.
- **Contrast:** every new color goes through a `Role` and is covered by `tests/contrast.rs`. 2A adds no color.
- **Content:** all data is fictional and generated or hand-written for Quarry, with no real people, artists or labels.
- **Exact versions:** gpui-kit =0.9.1 and rusqlite 0.41.1 (the workspace dependency). New: the `backup` feature on rusqlite, and `serde` and `toml` (already workspace dependencies) in `challenges`.
- **Merging:** only with the user's explicit go-ahead, after the user's visual review.

## 3. Architecture

### 3.1 Content

```
content/tracks/vinyl-vault/
├─ track.toml        title, emoji, intro, concepts, difficulty, order
├─ schema.sql        DDL
├─ seed.sql          hand-written in 2A; `gen-seed` takes over in 2D
└─ challenges.toml   [[challenge]] entries
```

- A track's slug is its folder name. A challenge's full id is `<slug>/<id>`, for example `vinyl-vault/first-spin`.
- Each `[[challenge]]` has `id`, `title`, `prompt`, `kind`, `ordered`, `concepts`, `solution`, `hints` (exactly three) and `skeleton`. `verify` belongs to Mutation challenges and is rejected in 2A.
- Hints are parsed and validated but not shown until 2C.
- **Vinyl Vault in 2A:** a fictional record shop with tables such as `artists`, `albums` and `pressings` and a few dozen rows. Five challenges cover SELECT of named columns, WHERE, ORDER BY with `ordered = true`, DISTINCT and NULL handling, so that between them they can fail every rule of the pipeline.

### 3.2 `quarry-engine`

- `SqliteDriver::copy_of(&SqliteDriver) -> Result<SqliteDriver, EngineError>` returns a new in-memory database holding a copy of another, through SQLite's backup API (rusqlite's `backup` feature).

### 3.3 `quarry-challenges`

Each module has one job.

| Module | Responsibility |
|---|---|
| `build.rs` + `embedded` | Turns every folder in `content/tracks/` into a `&'static [RawTrack]` of `include_str!`s, and reruns when the content changes |
| `track` | `Track` and `Challenge`, and parsing and validating one track. A bad track is a `ContentError` naming the file and the field |
| `catalog` | `Catalog`: every track in `order`, and lookup of a challenge by its full id |
| `kind` | The `ChallengeKind` trait and `KindRegistry`; `query` is the only kind. Given two fresh copies, a kind returns the user's result and the expected result |
| `sandbox` | `Template`: a track's in-memory database built from its schema and seed. `Template::fresh()` returns a new copy |
| `run` | Runs SQL on a copy into a `QueryResult` capped at 10,000 rows, with a 5-second timeout that trips the driver's `Canceller`. The last statement's result is the answer |
| `rules` | The `CheckRule` trait, the four rules and the ordered pipeline that stops at the first `CheckFailure` |
| `check` | `check(&Template, &Challenge, sql) -> Result<Verdict, CheckError>` |

- `Verdict` is `Solved` or `Wrong(CheckFailure)`.
- `CheckError` is the user's SQL failing (with the engine's message), the timeout, the row cap, or `Content` when the reference solution fails.
- Expected results are computed at check time and never stored.
- `challenges` has no GPUI dependency and is synchronous; the app runs it on the background executor.

### 3.4 `quarry-store`: schema v3

- The migration adds a nullable `challenge TEXT` column to `tabs` and to `closed_tabs`.
- `Draft` gains `challenge: Option<String>`, the challenge's full id.
- A v2 file upgrades in place, and a file newer than v3 is still refused and left untouched.

### 3.5 App

- **`model/verdict`** (GPUI-free, 100% coverage, mutation-tested): turns a `Verdict` or `CheckError` into the Check view's sentences.
- **Tracks:** the app holds the `Catalog` and builds each track's `Template` the first time it is used, on the background executor, then caches it.
- **Databases per tab:**
  - A challenge tab owns its own `Session` on a fresh copy of its track's template.
  - Plain tabs keep sharing the sample session.
  - The workspace sends runs, Explain and cancel to the active tab's session. The status-bar database label and the schema tree follow the active tab, for example "Vinyl Vault · sandbox".
- **Palette:** a new `Challenges` source after Recently closed. A row reads "First spin" with the detail "Vinyl Vault · 1 of 5".
- **`QueryTab`** gains `challenge: Option<ChallengeId>`.
- **Check view:** a new result view registered in `ResultViewRegistry`. It shows the latest check's sentences.
- **Shell:** a `CheckAnswer` action bound to ⌘⇧↵ in the workspace, and Query → Check Answer in the command table.

## 4. Behaviour

### 4.1 Opening a challenge

- Picking a challenge in the palette focuses its tab if one is open. Otherwise it opens a new tab titled after the challenge, on a fresh sandbox, with the skeleton as its draft.
- Running, Run all and Explain in a challenge tab use that tab's sandbox, and its data never affects grading.
- Closing a challenge tab remembers it in Recently closed with its challenge; reopening it gives a fresh sandbox.
- On restart a challenge tab reopens on a fresh sandbox with its draft. If its id no longer exists, it becomes a plain tab and keeps its text.

### 4.2 Check answer

- ⌘⇧↵ in a challenge tab grades the whole editor text. The status bar shows "Checking…" while it runs, and the tab switches to the Check view when it finishes.
- A newer check replaces one still running in the same tab.
- The Check view says one of:
  - "Solved."
  - "Not quite yet: …" followed by the first failure, in plain words:
    - the columns: how many were expected and received, or which names differ;
    - the row count: expected and received;
    - the contents: how many rows are missing and how many are unexpected;
    - the order: the first row out of place.
  - "Your query failed: <the engine's message>".
  - "Stopped after 5 seconds."
  - "Returned more than 10,000 rows."
- It never shows the expected rows or the solution.
- In a plain tab, ⌘⇧↵ shows the status-bar note "Check answer works in challenge tabs".

### 4.3 The grading pipeline

Each step runs only if the previous one passed.

1. **Columns:** names compared ignoring case; their order must match.
2. **Row count.**
3. **Contents:** compared as a multiset (order ignored, duplicates counted).
   - `NULL` equals `NULL`; an integer equals the same value as a float (`4 == 4.0`); floats are equal within a 1e-9 relative tolerance; text and bytes must match exactly.
   - Both row lists are sorted by one total order, with numbers compared as numbers, then paired up with the tolerant equality.
4. **Order:** only when `ordered = true`.

The user's SQL and the reference solution each run on their own fresh copy, each under its own timeout and row cap.

## 5. Errors

- **Bad content** is caught by the content tests, so it never ships. If a track still fails to load at runtime, it is left out of the palette and the rest of the app works.
- **A template that can't be built:** a challenge tab on that track says so in its Messages view, and Check answer says so in the Check view.
- **A failing reference solution** is `CheckError::Content`, reported as "This challenge has a problem; your answer wasn't checked."
- **The store:** schema v3 follows the 1D rules. If the file can't be opened, drafts stay in memory and the status bar says so.

## 6. Testing

- **`challenges`:**
  - unit tests for each rule, for value equality and for the pipeline order;
  - the loader rejecting each kind of bad content with the right `ContentError`;
  - the timeout and the row cap.
- **Content tests:** every track loads, ids are unique, every challenge's solution passes its own check, and no skeleton passes.
- **`engine`:** `copy_of` gives an independent copy.
- **`store`:** v2 to v3 upgrade in place; challenge ids round-trip through tabs and closed tabs.
- **App (GPUI tests):**
  - the palette lists challenges, and picking one opens a single tab per challenge;
  - a solved check and a wrong check, and a newer check replacing an older one;
  - runs and the schema tree follow the active tab's database;
  - a challenge tab survives a restart, and an unknown id falls back to a plain tab;
  - ⌘⇧↵ in a plain tab shows the note.

## 7. Tasks

1. Engine: `SqliteDriver::copy_of`.
2. Challenges: content format, `build.rs`, `track` and `catalog`, with the loader's tests.
3. Challenges: `sandbox` and `run` (copies, row cap, timeout).
4. Challenges: `rules`, `kind` and `check`.
5. Content: Vinyl Vault's schema, seed and first five challenges, with the content tests.
6. Store: schema v3 and `Draft::challenge`.
7. App: tracks, templates and a session per challenge tab; runs, Explain, the status bar and the schema tree follow the active tab.
8. App: the Challenges palette source and challenge tabs kept across restarts.
9. App: `model/verdict`, the Check view and ⌘⇧↵.
10. ADR 0012 "Challenge core", the README and a note in §5.2 of the parent spec (the template stays in memory until 2D).

As in 1C–1E, a verified prototype is built first, and the plan is generated from it.
