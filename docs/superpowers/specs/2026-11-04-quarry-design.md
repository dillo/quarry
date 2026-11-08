# Quarry — Design Spec

**Date:** 2026-11-04
**Status:** Approved in brainstorming; pending written-spec review
**Repo:** `github.com/dillo/quarry` · **Distribution:** `brew install --cask dillo/tap/quarry`

## 1. Product

Quarry is a native macOS (Apple Silicon) SQL client with two purposes:

1. **Learn and play.** It has a built-in SQLite database and story-driven SQL challenge tracks that cover a broad range of query techniques.
2. **Real work.** It connects to PostgreSQL and MySQL to write and run queries.

**Guiding goals**
- An interface beautiful enough that people want to keep opening it, and slicker than Beekeeper Studio.
- **No assistance while writing SQL: no autocomplete, suggestions or code insertion of any kind.** The only learning aids are the opt-in, conceptual hints described in §5.5.
- Native speed: about 200 ms to launch, 120 fps scrolling, and the UI thread never blocks.

**Non-goals for the MVP** (each must fit an existing extension point when it's added later): SSH tunnels (first after the MVP), inline editing of table data, a visual table designer, ER diagrams, SQL Server and other drivers, user-supplied tracks, cloud sync, AI features and automatic updates.

## 2. Technology decisions

| Concern | Choice | Reason |
|---|---|---|
| Language | Rust (stable, pinned via `rust-toolchain.toml`) | User goal; performance and safety |
| UI | **GPUI** via **`gpui-kit`** (bundles GPUI + `gpui-component`), pinned to an exact crates.io version | Pure Rust and drawn on the GPU with Metal; supports real frosted-glass windows; includes an editor and a virtualized table |
| Built-in database | **SQLite** via `rusqlite` (bundled) | Runs inside the app with nothing to install; the backup API resets sandboxes instantly |
| PostgreSQL | `tokio-postgres` + `rustls` | Streams rows; true query cancellation |
| MySQL | `mysql_async` + `rustls` | Streams rows; cancellation via `KILL QUERY` |
| Async runtime | Tokio on a dedicated background thread, connected to GPUI through channels | GPUI uses its own executor, so the UI never waits on I/O |
| Secrets | macOS Keychain via `keyring` | Passwords never stored on disk |
| Minimum OS | macOS 14 Sonoma, arm64 only | Target platform |

ADRs recorded in `docs/adr/`: 0001 GPUI over Tauri, 0002 SQLite as the built-in database, 0003 driver-native crates over sqlx, 0004 extension-point architecture, 0005 an unsigned but signing-ready release.

## 3. Architecture

### 3.1 Workspace layout

```
quarry/
├─ crates/
│  ├─ app/          GPUI binary: window, workspace, views, actions, registries wiring
│  ├─ ui-kit/       Aurora design system: tokens, themes, glass panel, button, tabs,
│  │                badge, progress ring, status bar, theme cross-fade
│  ├─ engine/       Driver trait, Value/Summary/RowSink model, drivers
│  │                (sqlite, postgres, mysql), statement splitter + classifier
│  ├─ challenges/   Track loader, sandbox manager, ChallengeKind, CheckRule pipeline,
│  │                HintPolicy
│  └─ store/        Settings, connections (metadata), history, saved queries,
│                   progress/streak — one SQLite file in
│                   ~/Library/Application Support/Quarry/quarry.db
├─ content/tracks/  vinyl-vault/, orbital-agency/, metro-lines/
├─ xtask/           gen-seed, complexity gate, exceptions check, bundle/dmg helpers
├─ quality/         exceptions.toml, complexity thresholds
├─ packaging/       Info.plist, icon, entitlements, cask template
└─ docs/            adr/, superpowers/specs/, superpowers/plans/
```

### 3.2 Dependency direction (enforced in CI)

```
app ──► ui-kit
app ──► challenges ──► engine
app ──► store ───────► engine (Value types only)
```

`engine` depends on no other workspace crate. `ui-kit` depends on no workspace crate apart from GPUI. Breaking either rule fails CI (`xtask check-deps`, which reads `cargo metadata`).

### 3.3 Extension points

New features are added through traits and registries, never by spreading `match` statements across the codebase. A CI check flags any `match` on a registry-owned enum outside the module that owns it.

| Extension point | Trait / registry | Adding one touches |
|---|---|---|
| Database driver | `engine::Driver` (incl. `schema()`), `DriverRegistry` | New module + 1 registration line |
| Challenge track | Folder in `content/tracks/` | 0 lines of code |
| Challenge kind | `challenges::ChallengeKind` | New module + 1 line |
| Answer-check rule | `challenges::CheckRule` (ordered pipeline) | New module + 1 line |
| Hint unlock rule | `challenges::HintPolicy` | New module + 1 line |
| Export format | `engine::Exporter`, `ExporterRegistry` | New module + 1 line |
| Theme | Token file in `ui-kit/themes/`, `ThemeRegistry` | New file + 1 line |
| Sidebar mode | `SidebarModeRegistry` | New view + 1 line |
| Results view (Results/Messages/Plan) | `ResultViewRegistry` | New view + 1 line |
| Right inspector (Brief/Row) | `InspectorRegistry` | New view + 1 line |
| Command palette source | `PaletteSourceRegistry` | New module + 1 line |

The PR template asks *"How many existing files did this feature modify, and why?"* If the answer is more than a few for a feature that has an extension point, the abstraction gets fixed.

### 3.4 Core data model (`engine`)

```rust
pub enum Value { Null, Bool(bool), Int(i64), Float(f64), Decimal(String),
                 Text(String), Bytes(Vec<u8>), Date(..), Time(..), Timestamp(..),
                 Json(String), Uuid([u8; 16]), Other(String) }

pub struct Column { pub name: String, pub type_name: String }

pub struct Summary { pub rows: u64, pub rows_affected: Option<u64>, pub elapsed: Duration }

/// Receives a statement's results as they are produced.
pub trait RowSink {
    fn columns(&mut self, columns: Vec<Column>);
    fn rows(&mut self, batch: Vec<Vec<Value>>);
}

/// The in-memory `RowSink`.
pub struct QueryResult { pub columns: Vec<Column>, pub rows: Vec<Vec<Value>> }

/// Drivers are synchronous; a `Session` runs them on their own thread.
pub trait Driver: Send {
    fn execute(&mut self, sql: &str, sink: &mut dyn RowSink) -> Result<Summary, EngineError>;
    fn schema(&mut self) -> Result<Schema, EngineError>;
    fn canceller(&self) -> Canceller;
}
```

- Results **stream** in batches through `RowSink`, so the grid fills in as rows arrive.
- `Canceller` (a shared flag) cancels a running statement. SQLite stops through its progress handler (ADR 0007). Postgres sends its cancel request and MySQL runs `KILL QUERY` (both Phase 3).
- Decimals are kept as exact strings and never rounded through a float.
- `EngineError` is a typed `thiserror` enum. Every variant has a readable message for the user.
- Drivers are synchronous and run on a `Session` thread; see ADR 0007.
- `Session` runs a `Driver` on its own thread and streams `RunEvent`s (`Started`, `Columns`, `Rows`, `Finished`, `Failed`, `Done`). See ADR 0007.

## 4. UI and interaction design

### 4.1 Visual language: Aurora

- **Aurora (dark) is the default.** A deep indigo-to-teal gradient behind native frosted-glass panels (GPUI's blurred window background), with floating panels that have 10 px rounded corners and 1 px light inner borders.
- **The accent** is a violet-to-blue gradient (`#8b6cff → #5b8cff`) with a soft glow. Success is teal (`#5eead4`), warnings amber (`#fbbf24`) and errors orange (`#fb923c`).
- **Syntax colors:** keywords violet, functions teal, strings amber, numbers pink, comments dim italic.
- **Aurora Dawn (light):** a pearly lavender-to-mint wash behind frosted white panels, with the same accents darkened enough to stay readable.
- **Typography:** SF Pro for the interface and Menlo for code and numbers (SF Mono isn't loadable by name; see ADR 0008). Numbers in the grid line up by digit.
- **Colors:** Colors are defined once per theme by semantic role in `ui-kit` and checked for WCAG contrast in CI.
- **Theme setting:** Dark / Light / Match System, in Settings and in View › Theme (⌘⇧T). Switching cross-fades over 250 ms.
- **Motion:** brief and purposeful. Every animation respects the macOS *Reduce Motion* setting.

Reference mockups are in `docs/design/mockups/` (`visual-direction.html`, `light-theme.html`, `layout.html` and `hints.html`). They are HTML fragments, so open them in a browser.

### 4.2 Screens

**Home (shown at launch)**
- A greeting based on the time of day, a progress nudge ("4 challenges away from finishing Orbital Agency") and a streak count (days with at least one solved challenge).
- Track cards with gradient progress bars and a Continue button, a Recent Connections card, ⌘K Search and New connection.

**Workspace (three columns)**

1. **Sidebar** (frosted glass, toggled with ⌘\\)
   - A Playground/Connections switch.
   - Tracks, each with a progress ring.
   - The schema tree: tables with row counts, and columns with their types. Clicking a table opens it in a browse tab. Dragging a name into the editor inserts it as text, which is a manual action and not autocomplete.
   - Query history.
2. **Center**
   - **Tabs** (⌘T opens one, ⌘W closes one). Each challenge tab keeps its own draft, and drafts survive restarts. Scratch tabs are freeform.
   - **Editor:** syntax highlighting, line numbers, bracket matching and multiple cursors. **Every completion and suggestion feature is disabled.**
   - A draggable divider.
   - **Results views:** Results, Messages and Plan (`EXPLAIN` / `EXPLAIN QUERY PLAN`).
   - **Actions:** Run (⌘↵) runs the statement under the cursor or the selection. Run all (⌘⇧R) runs the whole script, with one results tab per statement. Check answer (⌘⇧↵). Cancel (⌘.).
   - **Results grid:** virtualized and streaming ("1,000 of ~2.3M rows · fetching…"), resizable columns, NULLs shown in dim italic, and copy as TSV, CSV or JSON.
   - **Status bar:** which database you're connected to, row count, how long the query took, cursor position, a live timer while running, and a transaction indicator.
3. **Right inspector** (open by default, toggled with ⌘⌥I)
   - In Playground mode it shows the **challenge brief**: track and position ("Challenge 8 of 12 · Aggregates"), title, story prompt, expected columns, row count, whether order matters, concept tags, Prev/Next and the attempt count.
   - In Connections mode it shows a **row inspector**.

**Command palette (⌘K)**
- Jump to any track, challenge, table, tab, saved query, connection or setting.
- It's for navigating only and never suggests SQL.

### 4.3 Feedback on Check answer

- **Wrong:**
  - A "Not quite yet" card lists what's wrong: column mismatch, then row count, then row contents, then order.
  - It never says how to fix it.
  - "Show row diff" and "Reveal expected result" are opt-in buttons, so nothing gets spoiled by accident.
- **Right:**
  - An **aurora pulse**: about 600 ms of teal light across the window.
  - The progress segment glows, and the card reads "8 of 12 solved. Next up: …" with Next challenge and Stay here buttons.

## 5. Challenges

### 5.1 Content format

```
content/tracks/<slug>/
├─ track.toml        title, emoji, intro story, concept tags, difficulty, order
├─ schema.sql        DDL
├─ seed.sql          generated by `cargo xtask gen-seed` (deterministic, seeded)
└─ challenges.toml   [[challenge]]: id, title, prompt, kind, ordered, concepts,
                     solution, verify (mutation only), hints = [t1, t2, t3], skeleton
```

Tracks are embedded in the binary at compile time. All data is **fictional and generated**, with no real people in it.

### 5.2 Sandboxes

- On first launch, or when a track's content hash changes, each track is built into a **template DB** under Application Support.
- **Check answer** runs against a fresh in-memory copy of the template, loaded with SQLite's backup API.
- **Scratch tabs** use a persistent working copy. "Reset track database" restores it from the template.
- **Guardrails:** a 5-second timeout (via SQLite's progress handler, and cancellable) and a 10,000-row cap while grading.

### 5.3 Challenge kinds (`ChallengeKind`)

- **Query:** the user's result is compared with the result of running the reference `solution` on an identical fresh copy.
- **Mutation:** the user's SQL runs, then the hidden `verify` query runs. That result is compared with what `verify` returns after the reference `solution` runs on its own fresh copy.

Expected results are **computed at check time, never stored.**

### 5.4 Grading pipeline (`CheckRule`, stops at the first failure)

1. **Columns:** names compared case-insensitively; order must match.
2. **Row count.**
3. **Row contents:** compared as a multiset (order ignored, duplicates counted). `NULL` equals `NULL`, and an integer equals the same value as a float (`4 == 4.0`). Floats are equal within a 1e-9 relative tolerance.
4. **Row order:** checked only when `ordered = true`, and the brief says so.

Each rule returns a structured `CheckFailure` that the UI turns into plain language.

### 5.5 Progressive hints (`HintPolicy`)

- **Three tiers, written per challenge:**
  1. A nudge toward the concept.
  2. The clause or function to use.
  3. A read-only skeleton with blanks, which can't be pasted.
- **Default policy:** hint 1 unlocks after 2 wrong checks, then one more hint per additional wrong check. **Reveal solution** appears only after all three hints are open, and asks for confirmation.
- **Settings › Hints:** Progressive (default), Always available or Off.
- **Progress:** solving with hints still counts as solved and keeps the streak going. Hint use is marked with an amber dot, and a revealed solution with an outlined segment. Track cards offer "Re-solve N assisted challenges clean."

### 5.6 MVP content (about 36 challenges)

| Track | Level | Concepts |
|---|---|---|
| 🎧 Vinyl Vault | Beginner | SELECT, WHERE, ORDER BY, LIMIT, DISTINCT, CASE, NULL handling, basic functions |
| 🪐 Orbital Agency | Intermediate | inner, left, self and many-to-many joins; GROUP BY/HAVING; scalar, IN and correlated subqueries; EXISTS |
| 🚇 Metro Lines | Advanced | CTEs including recursive ones; window functions (ROW_NUMBER, RANK, LAG/LEAD, running totals); set operations; date/time; INSERT/UPDATE/DELETE; DDL |

A coverage matrix in `content/COVERAGE.md` maps every concept to at least two challenges.

## 6. External connections

- **Connection form:**
  - Name, driver, host, port, database and user.
  - Password stored in the Keychain.
  - Pasting a URL (`postgres://…`, `mysql://…`) fills in the form.
  - TLS mode: disable, prefer, require or verify-full.
  - Test connection, with readable errors.
  - Environment tag: dev, staging or prod, each with a color.
- **Schema introspection** comes from `pg_catalog` (Postgres) or `information_schema` (MySQL), and is loaded lazily as tree nodes are expanded.
- **Prod guardrails:**
  - A thin red aurora edge around the window.
  - Read-only sessions by default (`SET SESSION CHARACTERISTICS AS TRANSACTION READ ONLY` in Postgres, `SET SESSION TRANSACTION READ ONLY` in MySQL).
  - A statement classifier asks for confirmation before `DROP`, `TRUNCATE`, and `UPDATE`/`DELETE` without a `WHERE`.
- **Transactions:** auto-commit by default. A manual mode adds Commit and Rollback buttons, and the status bar shows when a transaction is open.
- **History and saved queries:**
  - History is kept automatically per connection and is searchable from ⌘K.
  - Saved queries are named and organized per connection.
- **Export:** CSV, TSV and JSON through `Exporter`.

## 7. Engineering standards

### 7.1 Design principles

1. Dependencies flow one way (§3.2), and CI enforces it.
2. Logic lives in library crates. GPUI views stay thin: they render state and send actions.
3. **Single responsibility, for every function and module.**
   - **Functions:** each one does one thing at one level of abstraction. Its name says exactly what it does. If the honest name needs "and" or "or" (`parse_and_validate`, `load_or_create`), split it. A function either decides (logic) or does (I/O, rendering), not both.
   - **Modules:** each one has one reason to change. For example, `checker/columns.rs` changes only when the column-matching rules change, and `sqlite/stream.rs` only when row streaming changes. When a module starts collecting unrelated helpers, split it. No catch-all modules such as `utils.rs` or `helpers.rs`; every module is named for its single responsibility.
   - **Crates:** one responsibility each, as described in §3.1.
   - **Enforcement:** reviewers check it, using the PR template question *"Does each new or changed function and module have a single responsibility? Name it in one sentence without 'and'."* The size and complexity limits in §7.2 back it up automatically, and `xtask` rejects files named `utils`, `helpers`, `misc` or `common`.
   - **Relationship to deep modules (principle 4):** single responsibility is about *cohesion*, meaning everything in a module serves one purpose. Deep modules are about the *interface*, meaning a small public API that hides a lot. A module should have both: one focused purpose, with plenty of logic hidden behind a narrow API.
4. Deep modules: a narrow, documented public API that hides internals.
5. Errors are values: `thiserror` in libraries, `anyhow` only in `app`. `unwrap`, `expect`, `panic!`, `todo!` and `dbg!` are denied outside tests.
6. `#![forbid(unsafe_code)]` in every workspace crate.
7. Tests come first for `engine`, `challenges` and `store`. Every bug fix starts with a test that reproduces it.
8. Extension points come first (§3.3). New features are additive.
9. Decisions are recorded as ADRs.
10. The app's own SQL (store, introspection) always uses bound parameters. User-typed SQL is the one deliberate exception.

### 7.2 Size and complexity limits (per function)

| Metric | Limit | Enforced by |
|---|---|---|
| Logical lines (statements) | ≤ 15 | `rust-code-analysis-cli` (pinned), via `xtask complexity` |
| Physical lines | ≤ 40 | Same, plus clippy `too-many-lines-threshold = 40` |
| Cyclomatic complexity | ≤ 10 | `rust-code-analysis-cli` |
| Cognitive complexity | ≤ 15 | `rust-code-analysis-cli`, plus clippy `cognitive-complexity-threshold = 15` (an early warning) |
| Nesting depth | ≤ 4 | clippy `excessive-nesting-threshold = 4` |
| Arguments | ≤ 5 | clippy `too-many-arguments-threshold = 5` |
| Type complexity | ≤ 250 | clippy `type-complexity-threshold = 250` |

**Exceptions:** each needs an `#[allow(..)]` with a written reason **and** an entry in `quality/exceptions.toml`. CI rejects any `allow` that isn't listed there.

### 7.3 Testing and coverage

| Layer | Tooling |
|---|---|
| Unit tests | `cargo nextest` |
| Grading pipeline and value comparison | `proptest` |
| Error messages and check feedback | `insta` snapshots |
| Postgres and MySQL drivers | Integration tests in Docker via `testcontainers` (always in CI; opt-in locally) |
| UI views | GPUI `TestAppContext`: user actions in, state changes out |
| Content | Every reference solution passes its own checker; every hint and skeleton is present; COVERAGE.md is checked |

| Coverage gate (`cargo llvm-cov`, per crate) | Lines | Regions |
|---|---|---|
| `engine`, `challenges`, `store` | ≥ 95% | ≥ 90% |
| `ui-kit`, `app` | ≥ 80% | ≥ 70% |

**Mutation testing** (`cargo-mutants`) runs on `engine`, `challenges` and `store`. PRs test only the changed code, and the whole codebase runs weekly. Gate: at least 90% of introduced bugs caught.

### 7.4 Linting and supply chain

- `rustfmt`.
- `clippy` with `-D warnings` and the pedantic group, plus the restriction lints listed in §7.1.
- `cargo-deny`: RustSec vulnerability advisories, a license allowlist, duplicate dependency versions and unknown sources.
- `cargo-machete` (unused dependencies), `typos` (spelling) and a `cargo-geiger` report (deferred: 0.14.0 fails on this workspace, see ADR 0004).

### 7.5 Workflow

- **`just check`** runs exactly the same steps as CI: fmt, clippy, deny, machete, typos, tests, coverage, complexity, the exceptions check, the dependency-direction check and the module-naming check.
- **`lefthook`:** fmt and clippy before each commit; `just check` before each push.
- **GitHub Actions on `macos-15` (arm64; `macos-14` was retired)** for every PR. `main` is protected and requires CI to pass.
- **Conventional Commits.** A PR template that includes the "files touched" question (§3.3) and the single-responsibility question (§7.1).

## 8. Release and distribution

- **The `.app`:**
  - Built by `xtask bundle` with `Info.plist`, the icon, bundle ID `io.github.dillo.quarry`, a hardened runtime, `LSMinimumSystemVersion` 14.0 and arm64 only.
  - Packaged as a DMG with `create-dmg`.
- **Signing:**
  - **Ad-hoc signature for now.** The release workflow has codesign and `notarytool` steps that switch on automatically once `APPLE_DEVELOPER_ID_*` secrets exist (ADR 0005).
- **Releases:**
  - `release-please` turns conventional commits into a changelog, a version tag and a GitHub Release with the DMG and its checksum attached.
- **Homebrew:**
  - The release workflow commits `Casks/quarry.rb` to **`dillo/homebrew-tap`**, using a fine-grained token that can only write to that repo. The existing `brewery.rb` is left untouched.
  - The cask contains `app "Quarry.app"`, `depends_on arch: :arm64`, `depends_on macos: ">= :sonoma"`, a postflight step that removes the quarantine flag (while the app is unsigned), and a `zap` stanza that removes `~/Library/Application Support/Quarry` and the app's preferences.
  - Install: `brew install --cask dillo/tap/quarry`.

## 9. Phases

Each phase gets its own implementation plan. Each one ends with a usable app and every gate passing.

| Phase | Scope | Exit criteria |
|---|---|---|
| **0: Foundations + spike** | Workspace, all quality gates, CI, ADRs, PR template. A throwaway GPUI spike testing (a) the frosted-glass window, (b) the `gpui-component` editor with completion fully removed, (c) a virtualized 1M-row table at 120 fps | `just check` and CI pass on an empty skeleton; the spike confirms all three or documents the custom work needed |
| **1: Shell + design system** | `ui-kit` (Aurora and Dawn, cross-fade, components), workspace layout, editor, grid, status bar, ⌘K, SQLite driver with streaming, cancel and multiple statements | You can write and run SQL against a sample database in the finished UI |
| **2: Challenges** | Track format and loader, sandboxes, both challenge kinds, grading pipeline, brief panel, hints, check feedback and aurora pulse, progress and streak, Home. Content: Vinyl Vault, then Orbital Agency and Metro Lines | All three tracks playable end to end; content tests passing |
| **3: Connections** | Postgres and MySQL drivers, Keychain, connection form and URL paste, introspection, prod guardrails, transactions, history, saved queries, export | Real work possible against Postgres and MySQL; integration tests passing in CI |
| **4: Ship (MVP)** | Bundling, DMG, release-please, the signing-ready workflow, cask publishing to `dillo/homebrew-tap` | `brew install --cask dillo/tap/quarry` installs and launches v0.1.0 |

## 10. Risks

| Risk | Mitigation |
|---|---|
| GPUI API changes and sparse documentation | Pin `gpui-kit` with `=`; upgrade on purpose; Phase 0 spike; keep GPUI-specific code in `app` and `ui-kit` |
| `gpui-component` editor completion can't be fully removed | The spike checks this first. Fallback: a thin editor of our own built on GPUI's text input primitives |
| `rust-code-analysis-cli` is pre-1.0 | Pin the version; clippy's limits still apply if it breaks |
| Users hit Gatekeeper warnings while unsigned | The cask's postflight removes the quarantine flag; signing switches on once Apple credentials are added |
| Strict gates slow down early work | Gates exist from day one, so there's no backlog to clear later; the exceptions file is the reviewed escape hatch |
