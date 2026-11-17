# Phase 2A: The Challenge Core — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build the core of Phase 2 ("Challenges"): tracks embedded from `content/tracks/`, in-memory sandboxes, the Query kind and the four-rule grading pipeline, with a thin playable path in the app (challenges in ⌘K, a challenge tab per challenge on its own sandbox, kept across restarts, and Check answer with ⌘⇧↵), plus Vinyl Vault's first five challenges.

**Architecture:**
- **`quarry-engine`:** `SqliteDriver::copy_of`, an in-memory copy through SQLite's backup API.
- **`quarry-challenges`** (depends only on `engine`, no GPUI, synchronous):
  - `build.rs` + `embedded`: every track folder as a `RawTrack` of `include_str!`s.
  - `track`, `error`, `catalog`: parsing and checking a track, `ContentError`, `Catalog` and `ChallengeId`.
  - `sandbox`: `Template`, an in-memory database built from a track's schema and seed, and `Template::fresh`.
  - `run`: a graded run with a 10,000-row cap and a 5-second watchdog.
  - `rules`: `CheckRule`, the pipeline (columns, row count, contents, order) and cell equality.
  - `kind`: `ChallengeKind` and `KindRegistry` with Query.
  - `check`: `check(&Template, &Challenge, sql) -> Result<Verdict, CheckError>`.
- **Content:** `content/tracks/vinyl-vault/` with a hand-written seed and five challenges, and content tests.
- **`quarry-store`:** schema v3 adds a nullable `challenge` column to `drafts` and `closed_tabs`; `Draft::challenge`.
- **`app`:**
  - `tracks::Tracks`: the catalog and the templates built so far.
  - The workspace keeps a sandbox session per challenge tab; runs, Explain, cancel, the schema tree and the status bar follow the active tab.
  - A Challenges palette source; challenge tabs saved, restored and reopened with their challenge.
  - `model::verdict` (GPUI-free, 100% covered, mutation-tested), the Check result view, the `CheckAnswer` action on ⌘⇧↵.
- **Status:** every task's code was built and gated in a scratch clone first, one commit per task. The tasks were then replayed in order from the spec commit, and each commit passed fmt, clippy, the quality checks, typos, complexity and the whole workspace test suite. The suite grew from 385 tests after Task 1 to 478 after Task 9. `just check` passed at the end, and mutation testing over the whole diff missed nothing. Each task's code below is taken from those commits.

**Tech Stack:** Rust 1.98.1, `gpui-kit` =0.9.1, `rusqlite` 0.41.1 (now with its `backup` feature), `serde` and `toml` (workspace dependencies), `thiserror`.

**Spec:** `docs/superpowers/specs/2026-11-16-phase-2a-challenge-core-design.md` (approved 2026-11-16). Parent: `docs/superpowers/specs/2026-11-04-quarry-design.md` §3.3, §4.2, §5 and §9. ADR 0012 (written in Task 10).

## Global Constraints

- **Branch and commits:** work on `phase-2a-challenge-core`, which already holds the spec and this plan. Use Conventional Commits, with no trailers.
- **No completion or assistance of any kind:**
  - The editor gets no completion, hover, definition or code-action providers, no inline suggestions and no hints. The existing test keeps asserting it.
  - The palette navigates and runs commands; it never offers SQL.
  - A wrong answer says what is wrong, never how to fix it, and never shows the expected rows or the solution.
- **Extension points:** new kinds of things go through traits and registries: `ChallengeKind`/`KindRegistry`, `CheckRule` (the ordered pipeline), `PaletteSource`/`PaletteSourceRegistry` and `ResultViewRegistry`. No `match` on a registry-owned enum outside its module. A new track is a folder and nothing else.
- **Dependency direction:**
  - `engine` depends on no workspace crate.
  - `challenges` depends only on `engine`.
  - `ui-kit` depends only on GPUI.
  - `store` depends on no workspace crate apart from `engine`.
  - `cargo xtask check-deps` enforces all of these.
- **Coverage:**
  - `engine`, `challenges` and `store` need at least 95% lines and 90% regions.
  - `app` needs at least 80% lines and 70% regions.
  - Every file in `crates/app/src/model` stays at 100% lines.
- **Mutation testing:** 0 missed in `engine`, `challenges`, `store`, `crates/app/src/model` and `crates/app/src/sample`.
- **Per function:** at most 15 statements and 40 lines, cyclomatic ≤ 10, cognitive ≤ 15, ≤ 5 arguments and nesting ≤ 4. This applies to tests too.
- **Banned outside `#[test]` functions:** `unwrap`, `expect`, `panic!`, `unreachable!`, `todo!`, `unimplemented!` and `dbg!`.
  - Test helpers that aren't `#[test]` functions count as outside: they return a `Result` or an `Option`, and the test unwraps it.
  - No lint suppression attributes, and no `as` casts.
- **Polling:** GPUI code never awaits a session channel. It polls every `POLL` (33 ms). Background work, including grading, uses GPUI's executor, so tests stay deterministic.
- **Contrast:** every new color goes through a `Role` and is covered by `tests/contrast.rs`. 2A adds no color; the Check view uses `Success`, `Warning`, `Danger`, `TextSecondary` and `TextMuted`.
- **Content:** all data is fictional and written for Quarry, with no real people, artists or labels.
- **Exact versions:** `gpui-kit` =0.9.1, `rusqlite` 0.41.1 (the workspace dependency).
- **Headless testing:**
  - Tests find elements by id, so elements they look for call `.test_support()`.
  - `settle()` advances the clock by `POLL`, pumps the executor, and returns whether its condition held. Every caller asserts it.
- **Every task ends with its gates passing.** Don't commit anything under `.superpowers/`.
- **Merging:** only with the user's explicit go-ahead, after the user's visual review.

## Where the plan departs from the spec

The prototype found these. Each is small, and all of them are in the code below.

1. **The tabs table is `drafts`.** Schema v3 adds `challenge` to `drafts` and `closed_tabs`; the spec called the first one `tabs`.
2. **A template is built on the main thread, the first time its track is used.** With 2A's hand-written seed this takes about a millisecond, and the tab needs its session at once. 2D's generated seed and on-disk template move the build off the main thread.
3. **A sandbox that can't be made opens nothing.** Opening or reopening a challenge whose template or copy fails shows "Couldn't open <title>: <error>" in the status bar instead of opening a tab that can't run; a restored tab in that state stays a plain tab and keeps its text. The content tests keep this from happening with shipped content.
4. **"Checking…" shows in the Check view, not the status bar.** ⌘⇧↵ switches the tab to the Check view at once, so the progress and the result are in one place.
5. **The workspace knows which tabs are challenge tabs.** It keeps a sandbox (challenge id and session) per challenge tab, keyed by tab id, instead of a `challenge` field on `QueryTab`. `QueryTab` gains only the Check view and the check itself (`play_challenge`, `check`, `check_state`).
6. **Titles and reopening:**
   - A challenge tab's title always comes from the catalog, so a renamed challenge shows its new title after a restart.
   - Reopening a closed challenge tab whose challenge is already open brings it back as a plain tab with its text, keeping one tab per challenge without losing the draft.
7. **Unknown kinds are refused by `Catalog::load`,** not by `track::parse`, because the kinds live in `KindRegistry`; the error is the same `ContentError::Field` naming `kind`.
8. **The row cap decides, not the cancellation.** A statement's last batch can push it past 10,000 rows after it has already finished, so `run` refuses any statement whose sink went over the cap, whether or not the cancellation stopped it.
9. **Small additions the behavior needs:**
   - `Catalog::load(&[RawTrack]) -> (Catalog, Vec<ContentError>)` and `ChallengeId`.
   - `run::{Limits, LIMITS, RunError}` and `check::check_within`, so tests can use small limits.
   - `rules::value::{same, same_row, compare, compare_rows}`.
   - `ResultViewRegistry::for_challenge`, `tracks::Tracks`, `NOT_A_CHALLENGE`.

## File Structure

```
crates/engine/src/sqlite/mod.rs                 SqliteDriver::copy_of
crates/challenges/build.rs                      embeds content/tracks/*
crates/challenges/src/
  embedded.rs       RawTrack, TRACKS
  error.rs          ContentError
  track.rs (+ track/tests.rs)                   Track, Challenge, parse
  catalog.rs        Catalog, ChallengeId
  sandbox.rs        Template
  run.rs (+ run/tests.rs)                       Limits, LIMITS, RunError, run
  rules/{mod,value,columns,count,contents,order}.rs
  kind/{mod,query}.rs                           ChallengeKind, KindRegistry, Query
  check.rs (+ check/tests.rs)                   Verdict, CheckError, check
crates/challenges/tests/content.rs              content tests
content/tracks/vinyl-vault/{track.toml, schema.sql, seed.sql, challenges.toml}
crates/store/src/{drafts.rs, migrations.rs, sqlite.rs}        schema v3
crates/app/src/tracks.rs                        Tracks
crates/app/src/model/verdict.rs (+ verdict/tests.rs)          CheckState, Report
crates/app/src/views/
  workspace/challenges.rs                       sandboxes, open_challenge, check_answer
  palette/challenges.rs                         the Challenges source
  query_tab/check.rs                            QueryTab::check
  results/check.rs                              the Check view
crates/app/tests/app/{challenges.rs, check.rs}  GPUI tests
docs/adr/0012-challenge-core.md
```

---

### Task 1: Engine: `SqliteDriver::copy_of`

Sandboxes copy a track's template for every run, so the engine learns to copy an in-memory database. `copy_of` opens a new in-memory database and copies the source into it with SQLite's backup API, which needs rusqlite's `backup` feature. The copy has its own canceller.

**Files:**
- Modify: `Cargo.toml`
- Modify: `crates/engine/src/sqlite/mod.rs`
- Modify: `crates/engine/tests/sqlite.rs`

**Interfaces:**
- Consumes: `quarry_engine::sqlite::SqliteDriver`.
- Produces: `SqliteDriver::copy_of(source: &SqliteDriver) -> Result<SqliteDriver, EngineError>`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/engine/tests/sqlite.rs`:
```diff
@@ -198,3 +198,41 @@ fn user_tables_that_resemble_internal_names_are_listed() -> TestResult {
     assert_eq!(names, ["SqliteSettings"]);
     Ok(())
 }
+
+#[test]
+fn a_copy_holds_the_same_data_and_changes_independently() -> TestResult {
+    let mut original = crew()?;
+    let mut copy = SqliteDriver::copy_of(&original)?;
+    copy.execute_script("DELETE FROM astronauts WHERE name = 'Kenji'")?;
+    original.execute_script("INSERT INTO astronauts (name) VALUES ('Amara')")?;
+    let names = |driver: &mut SqliteDriver| -> Result<Vec<Vec<Value>>, EngineError> {
+        let mut result = QueryResult::default();
+        driver.execute("SELECT name FROM named ORDER BY name", &mut result)?;
+        Ok(result.rows)
+    };
+    assert_eq!(names(&mut copy)?, [[Value::Text("Valentina".to_owned())]]);
+    assert_eq!(
+        names(&mut original)?,
+        [
+            [Value::Text("Amara".to_owned())],
+            [Value::Text("Kenji".to_owned())],
+            [Value::Text("Valentina".to_owned())]
+        ]
+    );
+    Ok(())
+}
+
+#[test]
+fn a_copy_can_be_cancelled_on_its_own() -> TestResult {
+    let original = crew()?;
+    let mut copy = SqliteDriver::copy_of(&original)?;
+    copy.canceller().cancel();
+    let endless =
+        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
+    assert_eq!(
+        copy.execute(endless, &mut QueryResult::default()),
+        Err(EngineError::Cancelled)
+    );
+    assert!(!original.canceller().is_cancelled());
+    Ok(())
+}
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry-engine --test sqlite`
Expected: FAIL to compile: `no associated function or constant named copy_of found for struct SqliteDriver`.

- [ ] **Step 3: Implement**

Modify `Cargo.toml`:
```diff
@@ -15,7 +15,7 @@ async-channel = "2.5.0"
 clap = { version = "4.6.7", features = ["derive"] }
 gpui-kit = { version = "=0.9.1", default-features = false, features = ["component"] }
 regex = "1.13.1"
-rusqlite = { version = "0.41.1", features = ["bundled", "hooks", "column_decltype"] }
+rusqlite = { version = "0.41.1", features = ["backup", "bundled", "hooks", "column_decltype"] }
 serde = { version = "1.0", features = ["derive"] }
 serde_json = "1.0.154"
 tempfile = "3.27.0"
```

Modify `crates/engine/src/sqlite/mod.rs`:
```diff
@@ -5,9 +5,9 @@ mod explain;
 mod introspect;
 mod run;
 
-use std::path::Path;
+use std::{path::Path, time::Duration};
 
-use rusqlite::Connection;
+use rusqlite::{Connection, backup::Backup};
 
 use crate::{
     driver::{Canceller, Driver},
@@ -20,6 +20,9 @@ use crate::{
 /// How many virtual-machine steps run between cancellation checks.
 const CANCEL_CHECK_STEPS: i32 = 1_000;
 
+/// How many pages a copy moves per backup step.
+const COPY_PAGES_PER_STEP: i32 = 256;
+
 /// A connection to one `SQLite` database file or in-memory database.
 pub struct SqliteDriver {
     connection: Connection,
@@ -44,6 +47,20 @@ impl SqliteDriver {
         Self::from_connection(Connection::open_in_memory(), ":memory:")
     }
 
+    /// A new in-memory database holding a copy of `source`, made with
+    /// `SQLite`'s backup API. The copy has its own [`Canceller`].
+    ///
+    /// # Errors
+    /// Returns [`EngineError::Open`] if the copy cannot be allocated, and
+    /// [`EngineError::Sql`] if the backup fails.
+    pub fn copy_of(source: &Self) -> Result<Self, EngineError> {
+        let mut copy = Self::open_in_memory()?;
+        Backup::new(&source.connection, &mut copy.connection)
+            .and_then(|backup| backup.run_to_completion(COPY_PAGES_PER_STEP, Duration::ZERO, None))
+            .map_err(convert::error)?;
+        Ok(copy)
+    }
+
     /// Runs a whole script with no results, e.g. to load a schema and seed data.
     ///
     /// # Errors
```

- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run -p quarry-engine`
Expected: every test passes, including `a_copy_holds_the_same_data_and_changes_independently` and `a_copy_can_be_cancelled_on_its_own`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml crates/engine/src/sqlite/mod.rs crates/engine/tests/sqlite.rs
git commit -m "feat(engine): copy an in-memory database with the backup API"
```

---

### Task 2: Challenges: content format, `build.rs`, `track` and `catalog`

The `challenges` crate learns to load tracks. Its build script embeds every folder in `content/tracks/` (none exists yet, so `TRACKS` is empty). `track::parse` reads `track.toml` and `challenges.toml` with `serde` and `toml`, refusing unknown fields, bad ids, a blank solution, a hint count other than three, `verify` and repeated ids, each as a `ContentError` naming the file. `Catalog` keeps the tracks that load, in `order`, and finds a challenge by its full id.

**Files:**
- Modify: `Cargo.lock`
- Modify: `crates/challenges/Cargo.toml`
- Create: `crates/challenges/build.rs`
- Create: `crates/challenges/src/catalog.rs`
- Create: `crates/challenges/src/embedded.rs`
- Create: `crates/challenges/src/error.rs`
- Modify: `crates/challenges/src/lib.rs`
- Create: `crates/challenges/src/track.rs`
- Create: `crates/challenges/src/track/tests.rs`

**Interfaces:**
- Consumes: nothing new (the crate already exists, empty).
- Produces:
  - `embedded::{RawTrack { slug, track, schema, seed, challenges: &'static str }, TRACKS: &[RawTrack]}`.
  - `error::ContentError::{Parse { file, message }, Field { file, challenge, field: &'static str, problem: &'static str }, Duplicate { file, challenge }}`.
  - `track::{Track { slug, title, emoji, intro, concepts, difficulty, order: u32, schema, seed, challenges: Vec<Challenge> }, Challenge { id, title, prompt, kind, ordered: bool, concepts, solution, hints: Vec<String>, skeleton }, HINT_COUNT, parse(&RawTrack) -> Result<Track, ContentError>}`.
  - `catalog::{ChallengeId::{new(&str, &str), parse(&str) -> Option<Self>, track(), challenge()}` (displays as `track/challenge`), `Catalog::{load(&[RawTrack]) -> (Self, Vec<ContentError>), embedded(), tracks(), track(&str), challenge(&ChallengeId) -> Option<(&Track, &Challenge)>}}`.

- [ ] **Step 1: Implement**

The unit tests sit beside their code: `src/track/tests.rs`, and test modules in `error.rs` and `catalog.rs`.

Modify `Cargo.lock`:
```diff
@@ -5613,6 +5613,12 @@ dependencies = [
 [[package]]
 name = "quarry-challenges"
 version = "0.0.0"
+dependencies = [
+ "quarry-engine",
+ "serde",
+ "thiserror 2.0.21",
+ "toml 1.2.1+spec-1.1.0",
+]
 
 [[package]]
 name = "quarry-engine"
```

Modify `crates/challenges/Cargo.toml`:
```diff
@@ -6,5 +6,11 @@ rust-version.workspace = true
 publish.workspace = true
 license.workspace = true
 
+[dependencies]
+quarry-engine = { path = "../engine" }
+serde.workspace = true
+thiserror.workspace = true
+toml.workspace = true
+
 [lints]
 workspace = true
```

Create `crates/challenges/build.rs`:
```rust
//! Embeds every track folder under `content/tracks/` in the binary.
//!
//! Each folder becomes one `RawTrack` of `include_str!`s, in folder-name
//! order, so adding a track needs no code. A folder missing one of its four
//! files fails the build, naming the file.

use std::{
    env,
    error::Error,
    fmt::Write as _,
    fs,
    path::{Path, PathBuf},
};

/// The files every track folder holds, as `RawTrack` field and file name.
const FILES: [(&str, &str); 4] = [
    ("track", "track.toml"),
    ("schema", "schema.sql"),
    ("seed", "seed.sql"),
    ("challenges", "challenges.toml"),
];

fn main() -> Result<(), Box<dyn Error>> {
    let root = Path::new(&env::var("CARGO_MANIFEST_DIR")?).join("../../content/tracks");
    println!("cargo::rerun-if-changed={}", root.display());
    let mut code = String::from("&[\n");
    for folder in track_folders(&root)? {
        println!("cargo::rerun-if-changed={}", folder.display());
        code.push_str(&raw_track(&folder)?);
    }
    code.push(']');
    let out = Path::new(&env::var("OUT_DIR")?).join("tracks.rs");
    fs::write(out, code)?;
    Ok(())
}

/// The track folders, sorted by name; none when `root` is missing.
fn track_folders(root: &Path) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    if !root.is_dir() {
        return Ok(Vec::new());
    }
    let mut folders = Vec::new();
    for entry in fs::read_dir(root)? {
        let path = entry?.path();
        if path.is_dir() {
            folders.push(path);
        }
    }
    folders.sort();
    Ok(folders)
}

/// One `RawTrack { .. }` expression for `folder`.
fn raw_track(folder: &Path) -> Result<String, Box<dyn Error>> {
    let slug = folder
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or("a track folder's name is not UTF-8")?;
    let mut code = format!("    RawTrack {{\n        slug: {slug:?},\n");
    for (field, file) in FILES {
        let path = folder.join(file);
        println!("cargo::rerun-if-changed={}", path.display());
        if !path.is_file() {
            return Err(format!("content/tracks/{slug}/{file} is missing").into());
        }
        let path = path.canonicalize()?;
        let path = path.to_str().ok_or("a track file's path is not UTF-8")?;
        writeln!(code, "        {field}: include_str!({path:?}),")?;
    }
    code.push_str("    },\n");
    Ok(code)
}
```

Create `crates/challenges/src/catalog.rs`:
```rust
//! Every track that loaded, and finding a challenge by its full id.

use std::fmt;

use crate::{
    embedded::{RawTrack, TRACKS},
    error::ContentError,
    track::{self, Challenge, Track},
};

/// A challenge's full id: its track's slug and its own id, written
/// `vinyl-vault/first-spin`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChallengeId {
    track: String,
    challenge: String,
}

impl ChallengeId {
    /// The id of challenge `challenge` in track `track`.
    pub fn new(track: &str, challenge: &str) -> Self {
        Self {
            track: track.to_owned(),
            challenge: challenge.to_owned(),
        }
    }

    /// Reads `track/challenge`; anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let (track, challenge) = text.split_once('/')?;
        let ok = |part: &str| !part.is_empty() && !part.contains('/');
        (ok(track) && ok(challenge)).then(|| Self::new(track, challenge))
    }

    /// The track's slug.
    pub fn track(&self) -> &str {
        &self.track
    }

    /// The challenge's id within its track.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

impl fmt::Display for ChallengeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.track, self.challenge)
    }
}

/// The tracks that loaded, by `order`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    tracks: Vec<Track>,
}

impl Catalog {
    /// Loads every track in `raw`. A track with bad content is left out,
    /// and its error is returned beside the catalog.
    pub fn load(raw: &[RawTrack]) -> (Self, Vec<ContentError>) {
        let mut tracks = Vec::new();
        let mut errors = Vec::new();
        for raw in raw {
            match track::parse(raw) {
                Ok(track) => tracks.push(track),
                Err(error) => errors.push(error),
            }
        }
        tracks.sort_by_key(|track| track.order);
        (Self { tracks }, errors)
    }

    /// Loads the tracks embedded in the binary.
    pub fn embedded() -> (Self, Vec<ContentError>) {
        Self::load(TRACKS)
    }

    /// Every track, by `order`.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// The track with slug `slug`.
    pub fn track(&self, slug: &str) -> Option<&Track> {
        self.tracks.iter().find(|track| track.slug == slug)
    }

    /// The challenge `id` and its track.
    pub fn challenge(&self, id: &ChallengeId) -> Option<(&Track, &Challenge)> {
        let track = self.track(id.track())?;
        let challenge = track
            .challenges
            .iter()
            .find(|challenge| challenge.id == id.challenge())?;
        Some((track, challenge))
    }
}

#[cfg(test)]
mod tests {
    use super::{Catalog, ChallengeId};
    use crate::{embedded::RawTrack, error::ContentError};

    fn raw(slug: &'static str, order: u32, challenge: &'static str) -> RawTrack {
        let track: &'static str = Box::leak(
            format!("title = \"{slug}\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = {order}")
                .into_boxed_str(),
        );
        let challenges: &'static str = Box::leak(
            format!("[[challenge]]\nid = \"{challenge}\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]")
                .into_boxed_str(),
        );
        RawTrack {
            slug,
            track,
            schema: "",
            seed: "",
            challenges,
        }
    }

    #[test]
    fn ids_read_and_print_as_track_slash_challenge() {
        let id = ChallengeId::parse("vinyl-vault/first-spin").unwrap();
        assert_eq!(id, ChallengeId::new("vinyl-vault", "first-spin"));
        assert_eq!(id.track(), "vinyl-vault");
        assert_eq!(id.challenge(), "first-spin");
        assert_eq!(id.to_string(), "vinyl-vault/first-spin");
        for bad in ["vinyl-vault", "/first-spin", "vinyl-vault/", "a/b/c", ""] {
            assert_eq!(ChallengeId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn tracks_come_in_order_and_bad_ones_are_left_out() {
        let broken = RawTrack {
            slug: "broken",
            track: "title =",
            schema: "",
            seed: "",
            challenges: "",
        };
        let (catalog, errors) = Catalog::load(&[raw("late", 9, "a"), broken, raw("early", 1, "b")]);
        let slugs: Vec<&str> = catalog
            .tracks()
            .iter()
            .map(|track| track.slug.as_str())
            .collect();
        assert_eq!(slugs, ["early", "late"]);
        assert!(
            matches!(&errors[..], [ContentError::Parse { file, .. }] if file == "broken/track.toml")
        );
    }

    #[test]
    fn finds_a_challenge_and_its_track_by_id() {
        let (catalog, _) = Catalog::load(&[raw("one", 1, "a"), raw("two", 2, "b")]);
        let (track, challenge) = catalog.challenge(&ChallengeId::new("two", "b")).unwrap();
        assert_eq!((track.slug.as_str(), challenge.id.as_str()), ("two", "b"));
        assert_eq!(catalog.track("one").map(|track| track.order), Some(1));
        assert_eq!(catalog.track("three"), None);
        assert_eq!(catalog.challenge(&ChallengeId::new("two", "a")), None);
        assert_eq!(catalog.challenge(&ChallengeId::new("three", "b")), None);
    }

    #[test]
    fn nothing_embedded_is_broken() {
        let (_, errors) = Catalog::embedded();
        assert_eq!(errors, []);
    }
}
```

Create `crates/challenges/src/embedded.rs`:
```rust
//! The track folders under `content/tracks/`, embedded at compile time by
//! the crate's build script.

/// One track folder's files, as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawTrack {
    /// The folder's name, which is the track's slug.
    pub slug: &'static str,
    /// `track.toml`.
    pub track: &'static str,
    /// `schema.sql`.
    pub schema: &'static str,
    /// `seed.sql`.
    pub seed: &'static str,
    /// `challenges.toml`.
    pub challenges: &'static str,
}

/// Every embedded track folder, in folder-name order.
pub static TRACKS: &[RawTrack] = include!(concat!(env!("OUT_DIR"), "/tracks.rs"));
```

Create `crates/challenges/src/error.rs`:
```rust
//! What can be wrong with a track's content.

/// Why a track could not be loaded. Each names the file at fault.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContentError {
    /// A TOML file is not valid TOML, or not the expected shape.
    #[error("{file}: {message}")]
    Parse {
        /// The file, such as `vinyl-vault/track.toml`.
        file: String,
        /// What the parser said.
        message: String,
    },
    /// A challenge's field has a value that is not allowed.
    #[error("{file}: challenge \"{challenge}\": {field} {problem}")]
    Field {
        /// The file, such as `vinyl-vault/challenges.toml`.
        file: String,
        /// The challenge's id as written.
        challenge: String,
        /// The field at fault.
        field: &'static str,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// Two challenges in one track share an id.
    #[error("{file}: challenge id \"{challenge}\" is used twice")]
    Duplicate {
        /// The file, such as `vinyl-vault/challenges.toml`.
        file: String,
        /// The repeated id.
        challenge: String,
    },
}

#[cfg(test)]
mod tests {
    use super::ContentError;

    #[test]
    fn messages_name_the_file_and_the_challenge() {
        let parse = ContentError::Parse {
            file: "vinyl-vault/track.toml".to_owned(),
            message: "missing field `title`".to_owned(),
        };
        assert_eq!(
            parse.to_string(),
            "vinyl-vault/track.toml: missing field `title`"
        );
        let field = ContentError::Field {
            file: "vinyl-vault/challenges.toml".to_owned(),
            challenge: "first-spin".to_owned(),
            field: "hints",
            problem: "must hold exactly 3 hints",
        };
        assert_eq!(
            field.to_string(),
            "vinyl-vault/challenges.toml: challenge \"first-spin\": hints must hold exactly 3 hints"
        );
        let duplicate = ContentError::Duplicate {
            file: "vinyl-vault/challenges.toml".to_owned(),
            challenge: "first-spin".to_owned(),
        };
        assert_eq!(
            duplicate.to_string(),
            "vinyl-vault/challenges.toml: challenge id \"first-spin\" is used twice"
        );
    }
}
```

Modify `crates/challenges/src/lib.rs`:
```diff
@@ -1 +1,6 @@
 //! SQL challenge tracks: loading, sandboxes, grading, and hints.
+
+pub mod catalog;
+pub mod embedded;
+pub mod error;
+pub mod track;
```

Create `crates/challenges/src/track.rs`:
```rust
//! One track: its story, its database and its challenges, parsed from a
//! track folder and checked.

use serde::{Deserialize, de::DeserializeOwned};

use crate::{embedded::RawTrack, error::ContentError};

/// How many hints every challenge has (parent spec §5.5).
pub const HINT_COUNT: usize = 3;

/// A track, ready to play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The folder's name, such as "vinyl-vault".
    pub slug: String,
    /// Its title, such as "Vinyl Vault".
    pub title: String,
    /// Its emoji.
    pub emoji: String,
    /// The story that opens it.
    pub intro: String,
    /// The concepts it teaches.
    pub concepts: Vec<String>,
    /// Its level, such as "Beginner".
    pub difficulty: String,
    /// Where it sits among the tracks; lower comes first.
    pub order: u32,
    /// The DDL that creates its database.
    pub schema: String,
    /// The script that fills its database.
    pub seed: String,
    /// Its challenges, in play order.
    pub challenges: Vec<Challenge>,
}

/// One challenge in a track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    /// Its id within the track, such as "first-spin".
    pub id: String,
    /// Its title.
    pub title: String,
    /// The story prompt that says what to find.
    pub prompt: String,
    /// The id of its `ChallengeKind`, such as "query".
    pub kind: String,
    /// Whether the rows must come back in the solution's order.
    pub ordered: bool,
    /// The concepts it practises.
    pub concepts: Vec<String>,
    /// The reference solution.
    pub solution: String,
    /// The three hints, from a nudge to a skeleton with blanks.
    pub hints: Vec<String>,
    /// The editor's text when the challenge is first opened.
    pub skeleton: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackFile {
    title: String,
    emoji: String,
    intro: String,
    concepts: Vec<String>,
    difficulty: String,
    order: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChallengesFile {
    challenge: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    title: String,
    prompt: String,
    kind: String,
    #[serde(default)]
    ordered: bool,
    #[serde(default)]
    concepts: Vec<String>,
    solution: String,
    hints: Vec<String>,
    #[serde(default)]
    skeleton: String,
    verify: Option<String>,
}

/// Parses and checks the track in `raw`.
///
/// # Errors
/// Returns the first [`ContentError`] found, naming its file.
pub fn parse(raw: &RawTrack) -> Result<Track, ContentError> {
    let info: TrackFile = from_toml(raw.slug, "track.toml", raw.track)?;
    let file: ChallengesFile = from_toml(raw.slug, "challenges.toml", raw.challenges)?;
    Ok(Track {
        slug: raw.slug.to_owned(),
        title: info.title,
        emoji: info.emoji,
        intro: info.intro,
        concepts: info.concepts,
        difficulty: info.difficulty,
        order: info.order,
        schema: raw.schema.to_owned(),
        seed: raw.seed.to_owned(),
        challenges: challenges(raw.slug, file.challenge)?,
    })
}

/// `text`, the file `name` of track `slug`, as a `T`.
fn from_toml<T: DeserializeOwned>(slug: &str, name: &str, text: &str) -> Result<T, ContentError> {
    toml::from_str(text).map_err(|error| ContentError::Parse {
        file: format!("{slug}/{name}"),
        message: error.message().to_owned(),
    })
}

/// The checked challenges of track `slug`, in file order.
fn challenges(slug: &str, entries: Vec<Entry>) -> Result<Vec<Challenge>, ContentError> {
    let file = format!("{slug}/challenges.toml");
    let mut checked: Vec<Challenge> = Vec::with_capacity(entries.len());
    for entry in entries {
        check(&file, &entry)?;
        if checked.iter().any(|challenge| challenge.id == entry.id) {
            return Err(ContentError::Duplicate {
                file,
                challenge: entry.id,
            });
        }
        checked.push(entry.into_challenge());
    }
    Ok(checked)
}

/// The first field of `entry` that is not allowed, as an error.
fn check(file: &str, entry: &Entry) -> Result<(), ContentError> {
    let problem = if !is_id(&entry.id) {
        Some(("id", "must be lowercase letters, digits and hyphens"))
    } else if entry.solution.trim().is_empty() {
        Some(("solution", "must not be empty"))
    } else if entry.hints.len() != HINT_COUNT {
        Some(("hints", "must hold exactly 3 hints"))
    } else if entry.verify.is_some() {
        Some((
            "verify",
            "is only for mutation challenges, which aren't supported yet",
        ))
    } else {
        None
    };
    problem.map_or(Ok(()), |(field, problem)| {
        Err(ContentError::Field {
            file: file.to_owned(),
            challenge: entry.id.clone(),
            field,
            problem,
        })
    })
}

/// Whether `id` is lowercase letters, digits and hyphens, and not empty.
fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

impl Entry {
    fn into_challenge(self) -> Challenge {
        Challenge {
            id: self.id,
            title: self.title,
            prompt: self.prompt,
            kind: self.kind,
            ordered: self.ordered,
            concepts: self.concepts,
            solution: self.solution,
            hints: self.hints,
            skeleton: self.skeleton,
        }
    }
}

#[cfg(test)]
mod tests;
```

Create `crates/challenges/src/track/tests.rs`:
```rust
use super::{Challenge, parse};
use crate::{embedded::RawTrack, error::ContentError};

const TRACK: &str = r#"
title = "Test Pressings"
emoji = "💿"
intro = "A shop that sells test pressings."
concepts = ["SELECT"]
difficulty = "Beginner"
order = 3
"#;

const CHALLENGES: &str = r#"
[[challenge]]
id = "first-spin"
title = "First spin"
prompt = "List every album."
kind = "query"
ordered = true
concepts = ["SELECT"]
solution = "SELECT title FROM albums"
hints = ["one", "two", "three"]
skeleton = "SELECT ___ FROM albums"

[[challenge]]
id = "b-side"
title = "B-side"
prompt = "List every artist."
kind = "query"
solution = "SELECT name FROM artists"
hints = ["one", "two", "three"]
"#;

fn raw(track: &'static str, challenges: &'static str) -> RawTrack {
    RawTrack {
        slug: "test-pressings",
        track,
        schema: "CREATE TABLE albums (title TEXT);",
        seed: "INSERT INTO albums VALUES ('Blue');",
        challenges,
    }
}

/// A challenges file holding one challenge with `field` set to `value`,
/// replacing that field's default.
fn one_with(field: &str, value: &str) -> String {
    let mut lines = vec![
        "[[challenge]]".to_owned(),
        "title = \"T\"".to_owned(),
        "prompt = \"P\"".to_owned(),
        "kind = \"query\"".to_owned(),
    ];
    let defaults = [
        ("id", "\"ok\""),
        ("solution", "\"SELECT 1\""),
        ("hints", "[\"a\", \"b\", \"c\"]"),
    ];
    for (name, default) in defaults {
        let shown = if name == field { value } else { default };
        lines.push(format!("{name} = {shown}"));
    }
    if field == "verify" {
        lines.push(format!("verify = {value}"));
    }
    lines.join("\n")
}

fn field_error(field: &str, value: &str) -> Result<ContentError, String> {
    let text: &'static str = Box::leak(one_with(field, value).into_boxed_str());
    parse(&raw(TRACK, text))
        .err()
        .ok_or(format!("{field} = {value} loaded"))
}

#[test]
fn parses_the_track() {
    let track = parse(&raw(TRACK, CHALLENGES)).unwrap();
    assert_eq!(track.slug, "test-pressings");
    assert_eq!(track.title, "Test Pressings");
    assert_eq!(track.emoji, "💿");
    assert_eq!(track.intro, "A shop that sells test pressings.");
    assert_eq!(track.concepts, ["SELECT"]);
    assert_eq!(track.difficulty, "Beginner");
    assert_eq!(track.order, 3);
    assert_eq!(track.schema, "CREATE TABLE albums (title TEXT);");
    assert_eq!(track.seed, "INSERT INTO albums VALUES ('Blue');");
    assert_eq!(track.challenges.len(), 2);
}

#[test]
fn parses_every_field_of_a_challenge() {
    let track = parse(&raw(TRACK, CHALLENGES)).unwrap();
    assert_eq!(
        track.challenges[0],
        Challenge {
            id: "first-spin".to_owned(),
            title: "First spin".to_owned(),
            prompt: "List every album.".to_owned(),
            kind: "query".to_owned(),
            ordered: true,
            concepts: vec!["SELECT".to_owned()],
            solution: "SELECT title FROM albums".to_owned(),
            hints: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            skeleton: "SELECT ___ FROM albums".to_owned(),
        }
    );
}

#[test]
fn ordered_concepts_and_skeleton_are_optional() {
    let track = parse(&raw(TRACK, CHALLENGES)).unwrap();
    let b_side = &track.challenges[1];
    assert_eq!(b_side.id, "b-side");
    assert!(!b_side.ordered);
    assert!(b_side.concepts.is_empty());
    assert_eq!(b_side.skeleton, "");
}

#[test]
fn bad_toml_names_the_file() {
    let error = parse(&raw("title = ", CHALLENGES)).unwrap_err();
    assert!(
        matches!(&error, ContentError::Parse { file, .. } if file == "test-pressings/track.toml")
    );
    let error = parse(&raw(TRACK, "[[challenge]]\nid = 1")).unwrap_err();
    assert!(
        matches!(&error, ContentError::Parse { file, .. } if file == "test-pressings/challenges.toml")
    );
}

#[test]
fn unknown_fields_are_refused() {
    let track = "title = \"T\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = 1\ncolour = \"red\"";
    let error = parse(&raw(track, CHALLENGES)).unwrap_err();
    assert!(matches!(error, ContentError::Parse { message, .. } if message.contains("colour")));
}

#[test]
fn ids_must_be_lowercase_letters_digits_and_hyphens() {
    for bad in ["\"\"", "\"First\"", "\"a b\"", "\"a/b\"", "\"a_b\""] {
        let error = field_error("id", bad).unwrap();
        assert!(
            matches!(error, ContentError::Field { field: "id", .. }),
            "{bad}"
        );
    }
    let loaded = field_error("id", "\"track-2-b\"");
    assert_eq!(loaded, Err("id = \"track-2-b\" loaded".to_owned()));
}

#[test]
fn a_solution_must_not_be_blank() {
    let error = field_error("solution", "\"  \"").unwrap();
    assert_eq!(
        error,
        ContentError::Field {
            file: "test-pressings/challenges.toml".to_owned(),
            challenge: "ok".to_owned(),
            field: "solution",
            problem: "must not be empty",
        }
    );
}

#[test]
fn there_must_be_exactly_three_hints() {
    for bad in ["[\"a\", \"b\"]", "[\"a\", \"b\", \"c\", \"d\"]"] {
        let error = field_error("hints", bad).unwrap();
        assert!(matches!(error, ContentError::Field { field: "hints", .. }));
    }
}

#[test]
fn verify_is_refused_until_mutation_challenges_exist() {
    let error = field_error("verify", "\"SELECT 1\"").unwrap();
    assert!(matches!(
        error,
        ContentError::Field {
            field: "verify",
            ..
        }
    ));
}

#[test]
fn a_repeated_id_is_refused() {
    let twice = "[[challenge]]\nid = \"a\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]\n\n[[challenge]]\nid = \"a\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]";
    let error = parse(&raw(TRACK, twice)).unwrap_err();
    assert_eq!(
        error,
        ContentError::Duplicate {
            file: "test-pressings/challenges.toml".to_owned(),
            challenge: "a".to_owned(),
        }
    );
}
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry-challenges`
Expected: 15 tests pass.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add Cargo.lock crates/challenges/Cargo.toml crates/challenges/build.rs crates/challenges/src/catalog.rs crates/challenges/src/embedded.rs crates/challenges/src/error.rs crates/challenges/src/lib.rs crates/challenges/src/track.rs crates/challenges/src/track/tests.rs
git commit -m "feat(challenges): load embedded tracks and find challenges by id"
```

---

### Task 3: Challenges: `sandbox` and `run`

A `Template` is a track's database as its schema and seed leave it, kept in memory behind a mutex so a shared template can be copied from any thread. `run::run` runs a script on a copy for grading: every statement runs, the last one's result is the answer, a statement whose rows pass the cap gives `TooManyRows`, and a watchdog thread trips the canceller when the time is up.

**Files:**
- Modify: `crates/challenges/src/lib.rs`
- Create: `crates/challenges/src/run.rs`
- Create: `crates/challenges/src/run/tests.rs`
- Create: `crates/challenges/src/sandbox.rs`

**Interfaces:**
- Consumes: `SqliteDriver::copy_of` (Task 1), `quarry_engine::splitter::split`.
- Produces:
  - `sandbox::Template::{new(schema: &str, seed: &str) -> Result<Self, EngineError>, fresh(&self) -> Result<SqliteDriver, EngineError>}`.
  - `run::{Limits { rows: usize, time: Duration }, LIMITS` (10,000 rows, 5 s), `RunError::{Sql(EngineError), TimedOut, TooManyRows}, run(&mut dyn Driver, &str, Limits) -> Result<QueryResult, RunError>}`.

- [ ] **Step 1: Implement**

Modify `crates/challenges/src/lib.rs`:
```diff
@@ -3,4 +3,6 @@
 pub mod catalog;
 pub mod embedded;
 pub mod error;
+pub mod run;
+pub mod sandbox;
 pub mod track;
```

Create `crates/challenges/src/run.rs`:
```rust
//! Running a script on a sandbox copy for grading, with a row cap and a
//! time limit.

use std::{
    sync::mpsc::{self, RecvTimeoutError},
    thread,
    time::Duration,
};

use quarry_engine::{
    driver::{Canceller, Driver},
    error::EngineError,
    result::{Column, QueryResult, Row, RowSink},
    splitter,
};

/// How much one graded run may return and how long it may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most rows a statement may return.
    pub rows: usize,
    /// The longest a whole run may take.
    pub time: Duration,
}

/// The limits for grading (parent spec §5.2): 10,000 rows and 5 seconds.
pub const LIMITS: Limits = Limits {
    rows: 10_000,
    time: Duration::from_secs(5),
};

/// Why a graded run gave no result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// The database rejected a statement.
    Sql(EngineError),
    /// The run took longer than its time limit.
    TimedOut,
    /// A statement returned more rows than the cap.
    TooManyRows,
}

/// Runs every statement of `script` on `driver` within `limits`, and
/// returns the last statement's result. A last statement that returns no
/// rows has no columns.
///
/// # Errors
/// Returns a [`RunError`] for the first statement that fails, the time
/// limit or the row cap.
pub fn run(driver: &mut dyn Driver, script: &str, limits: Limits) -> Result<QueryResult, RunError> {
    let _watchdog = Watchdog::start(driver.canceller(), limits.time);
    let mut last = QueryResult::default();
    for range in splitter::split(script) {
        let mut sink = Capped::new(limits.rows, driver.canceller());
        let outcome = driver.execute(&script[range], &mut sink);
        let failure = if sink.over {
            Some(RunError::TooManyRows)
        } else {
            outcome.err().map(stopped)
        };
        if let Some(failure) = failure {
            return Err(failure);
        }
        last = sink.result;
    }
    Ok(last)
}

/// What a failed statement within the row cap means: the clock cancelled
/// it, or the database refused it.
fn stopped(error: EngineError) -> RunError {
    match error {
        EngineError::Cancelled => RunError::TimedOut,
        other => RunError::Sql(other),
    }
}

/// Keeps a statement's rows up to a cap, and cancels the statement when
/// it returns more. Rows past the cap may arrive in the statement's last
/// batch, after it has finished, so `over` decides the outcome rather
/// than the cancellation.
struct Capped {
    result: QueryResult,
    cap: usize,
    over: bool,
    canceller: Canceller,
}

impl Capped {
    fn new(cap: usize, canceller: Canceller) -> Self {
        Self {
            result: QueryResult::default(),
            cap,
            over: false,
            canceller,
        }
    }
}

impl RowSink for Capped {
    fn columns(&mut self, columns: Vec<Column>) {
        self.result.columns = columns;
    }

    fn rows(&mut self, batch: Vec<Row>) {
        if self.result.rows.len() + batch.len() > self.cap {
            self.over = true;
            self.canceller.cancel();
            return;
        }
        self.result.rows.extend(batch);
    }
}

/// Cancels a run that is still going when its time is up. Dropping it
/// ends the watch: its thread wakes and leaves without cancelling.
struct Watchdog {
    _done: mpsc::Sender<()>,
}

impl Watchdog {
    fn start(canceller: Canceller, time: Duration) -> Self {
        let (done, finished) = mpsc::channel::<()>();
        thread::spawn(move || {
            if finished.recv_timeout(time) == Err(RecvTimeoutError::Timeout) {
                canceller.cancel();
            }
        });
        Self { _done: done }
    }
}

#[cfg(test)]
mod tests;
```

Create `crates/challenges/src/run/tests.rs`:
```rust
use std::time::Duration;

use quarry_engine::{error::EngineError, sqlite::SqliteDriver, value::Value};

use super::{LIMITS, Limits, RunError, run};

const ENDLESS: &str =
    "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";

/// `n` rows of one column, 1 to `n`.
fn numbers(n: usize) -> String {
    format!(
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < {n}) SELECT x FROM n"
    )
}

fn database() -> Result<SqliteDriver, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script("CREATE TABLE t (n INTEGER); INSERT INTO t VALUES (1), (2);")?;
    Ok(driver)
}

#[test]
fn grading_allows_ten_thousand_rows_and_five_seconds() {
    assert_eq!(LIMITS.rows, 10_000);
    assert_eq!(LIMITS.time, Duration::from_secs(5));
}

#[test]
fn the_last_statement_is_the_answer_and_earlier_ones_still_run() {
    let mut driver = database().unwrap();
    let script = "DELETE FROM t WHERE n = 1; SELECT 'x' AS tag; SELECT n FROM t";
    let result = run(&mut driver, script, LIMITS).unwrap();
    assert_eq!(result.columns[0].name, "n");
    assert_eq!(result.rows, [[Value::Int(2)]]);
}

#[test]
fn a_last_statement_without_rows_gives_no_columns() {
    let mut driver = database().unwrap();
    let result = run(&mut driver, "SELECT n FROM t; DELETE FROM t", LIMITS).unwrap();
    assert!(result.columns.is_empty());
    assert!(result.rows.is_empty());
    assert!(
        run(&mut driver, "  -- nothing\n", LIMITS)
            .unwrap()
            .columns
            .is_empty()
    );
}

#[test]
fn a_failing_statement_stops_the_run_with_its_error() {
    let mut driver = database().unwrap();
    let error = run(&mut driver, "SELECT nope FROM t; SELECT 1", LIMITS).unwrap_err();
    assert_eq!(
        error,
        RunError::Sql(EngineError::Sql("no such column: nope".to_owned()))
    );
}

#[test]
fn exactly_the_cap_is_allowed_and_one_more_is_too_many() {
    let mut driver = database().unwrap();
    let limits = Limits {
        rows: 600,
        time: LIMITS.time,
    };
    let result = run(&mut driver, &numbers(600), limits).unwrap();
    assert_eq!(result.rows.len(), 600);
    let error = run(&mut driver, &numbers(601), limits).unwrap_err();
    assert_eq!(error, RunError::TooManyRows);
}

#[test]
fn a_run_past_its_time_is_stopped() {
    let mut driver = database().unwrap();
    let limits = Limits {
        rows: LIMITS.rows,
        time: Duration::from_millis(50),
    };
    assert_eq!(run(&mut driver, ENDLESS, limits), Err(RunError::TimedOut));
}

#[test]
fn a_finished_run_is_not_cancelled_afterwards() {
    let mut driver = database().unwrap();
    let limits = Limits {
        rows: LIMITS.rows,
        time: Duration::from_millis(20),
    };
    run(&mut driver, "SELECT 1", limits).unwrap();
    std::thread::sleep(Duration::from_millis(60));
    assert!(!quarry_engine::driver::Driver::canceller(&driver).is_cancelled());
}
```

Create `crates/challenges/src/sandbox.rs`:
```rust
//! A track's database: built once in memory from its schema and seed, then
//! copied for every run that needs a clean one.

use std::sync::{Mutex, PoisonError};

use quarry_engine::{error::EngineError, sqlite::SqliteDriver};

/// A track's database as its schema and seed leave it. It is never run
/// against; [`Template::fresh`] gives copies to run on.
pub struct Template {
    database: Mutex<SqliteDriver>,
}

impl Template {
    /// Builds the template from a track's `schema` and `seed` scripts.
    ///
    /// # Errors
    /// Returns the engine's error for the first statement that fails.
    pub fn new(schema: &str, seed: &str) -> Result<Self, EngineError> {
        let mut database = SqliteDriver::open_in_memory()?;
        database.execute_script(schema)?;
        database.execute_script(seed)?;
        Ok(Self {
            database: Mutex::new(database),
        })
    }

    /// A new in-memory copy of the template, with its own canceller.
    ///
    /// # Errors
    /// Returns the engine's error if the copy cannot be made.
    pub fn fresh(&self) -> Result<SqliteDriver, EngineError> {
        // Copying only reads the template, so a panic elsewhere while the
        // lock was held leaves nothing half-done.
        let database = self.database.lock().unwrap_or_else(PoisonError::into_inner);
        SqliteDriver::copy_of(&database)
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::{driver::Driver, error::EngineError, result::QueryResult, value::Value};

    use super::Template;

    const SCHEMA: &str = "CREATE TABLE albums (title TEXT NOT NULL);";
    const SEED: &str = "INSERT INTO albums VALUES ('Blue'), ('Green');";

    fn count(driver: &mut impl Driver) -> Result<Vec<Vec<Value>>, EngineError> {
        let mut result = QueryResult::default();
        driver.execute("SELECT count(*) FROM albums", &mut result)?;
        Ok(result.rows)
    }

    #[test]
    fn every_copy_starts_from_the_seed() {
        let template = Template::new(SCHEMA, SEED).unwrap();
        let mut first = template.fresh().unwrap();
        first.execute_script("DELETE FROM albums").unwrap();
        assert_eq!(count(&mut first).unwrap(), [[Value::Int(0)]]);
        let mut second = template.fresh().unwrap();
        assert_eq!(count(&mut second).unwrap(), [[Value::Int(2)]]);
    }

    #[test]
    fn a_broken_schema_or_seed_is_the_engine_error() {
        let schema = Template::new("CREATE TABLE", SEED).err().unwrap();
        assert!(matches!(schema, EngineError::Sql(_)));
        let seed = Template::new(SCHEMA, "INSERT INTO nowhere VALUES (1)")
            .err()
            .unwrap();
        assert_eq!(seed, EngineError::Sql("no such table: nowhere".to_owned()));
    }
}
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry-challenges`
Expected: 24 tests pass.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/challenges/src/lib.rs crates/challenges/src/run.rs crates/challenges/src/run/tests.rs crates/challenges/src/sandbox.rs
git commit -m "feat(challenges): build sandboxes and run graded SQL within limits"
```

---

### Task 4: Challenges: `rules`, `kind` and `check`

The grading pipeline. Each rule is a `CheckRule`, run in order by `rules::first_failure`: columns (names ignoring case, in order), row count, contents (a multiset, sorted by one total order and walked together) and order (only when `ordered`). Cells compare with `rules::value::same`, which converts an integer to a float without a cast. The Query kind runs the solution and the answer on their own fresh copies, and `check` puts it together. `Catalog::load` now also refuses a challenge whose kind isn't registered.

**Files:**
- Modify: `crates/challenges/src/catalog.rs`
- Create: `crates/challenges/src/check.rs`
- Create: `crates/challenges/src/check/tests.rs`
- Create: `crates/challenges/src/kind/mod.rs`
- Create: `crates/challenges/src/kind/query.rs`
- Modify: `crates/challenges/src/lib.rs`
- Create: `crates/challenges/src/rules/columns.rs`
- Create: `crates/challenges/src/rules/contents.rs`
- Create: `crates/challenges/src/rules/count.rs`
- Create: `crates/challenges/src/rules/mod.rs`
- Create: `crates/challenges/src/rules/order.rs`
- Create: `crates/challenges/src/rules/value.rs`

**Interfaces:**
- Consumes: `Template`, `run` (Task 3), `Catalog` (Task 2).
- Produces:
  - `rules::{CheckFailure::{ColumnCount { expected, got }, ColumnNames { expected, got }, RowCount { expected, got }, Contents { missing, unexpected }, Order { row }}, Comparison { expected, got, ordered }, CheckRule, pipeline(), first_failure(&Comparison) -> Option<CheckFailure>}`.
  - `rules::value::{TOLERANCE, same, same_row, compare, compare_rows}`.
  - `kind::{Answers { expected, got }, ChallengeKind, KindRegistry::{standard(), get(&str)}}`.
  - `check::{Verdict::{Solved, Wrong(CheckFailure)}, CheckError::{Sql(String), TimedOut, TooManyRows, Content(String), Sandbox(EngineError)}, check(&Template, &Challenge, &str) -> Result<Verdict, CheckError>, check_within(.., Limits)}`.

- [ ] **Step 1: Implement**

Each rule's tests sit beside it; `rules::testing` builds results for them.

Modify `crates/challenges/src/catalog.rs`:
```diff
@@ -5,6 +5,7 @@ use std::fmt;
 use crate::{
     embedded::{RawTrack, TRACKS},
     error::ContentError,
+    kind::KindRegistry,
     track::{self, Challenge, Track},
 };
 
@@ -59,10 +60,11 @@ impl Catalog {
     /// Loads every track in `raw`. A track with bad content is left out,
     /// and its error is returned beside the catalog.
     pub fn load(raw: &[RawTrack]) -> (Self, Vec<ContentError>) {
+        let kinds = KindRegistry::standard();
         let mut tracks = Vec::new();
         let mut errors = Vec::new();
         for raw in raw {
-            match track::parse(raw) {
+            match track::parse(raw).and_then(|track| known_kinds(track, &kinds)) {
                 Ok(track) => tracks.push(track),
                 Err(error) => errors.push(error),
             }
@@ -97,18 +99,44 @@ impl Catalog {
     }
 }
 
+/// `track`, if every challenge's kind is in `kinds`.
+fn known_kinds(track: Track, kinds: &KindRegistry) -> Result<Track, ContentError> {
+    let unknown = track
+        .challenges
+        .iter()
+        .find(|challenge| kinds.get(&challenge.kind).is_none());
+    match unknown {
+        Some(challenge) => Err(ContentError::Field {
+            file: format!("{}/challenges.toml", track.slug),
+            challenge: challenge.id.clone(),
+            field: "kind",
+            problem: "names no kind Quarry knows",
+        }),
+        None => Ok(track),
+    }
+}
+
 #[cfg(test)]
 mod tests {
     use super::{Catalog, ChallengeId};
     use crate::{embedded::RawTrack, error::ContentError};
 
     fn raw(slug: &'static str, order: u32, challenge: &'static str) -> RawTrack {
+        raw_of_kind(slug, order, challenge, "query")
+    }
+
+    fn raw_of_kind(
+        slug: &'static str,
+        order: u32,
+        challenge: &'static str,
+        kind: &str,
+    ) -> RawTrack {
         let track: &'static str = Box::leak(
             format!("title = \"{slug}\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = {order}")
                 .into_boxed_str(),
         );
         let challenges: &'static str = Box::leak(
-            format!("[[challenge]]\nid = \"{challenge}\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]")
+            format!("[[challenge]]\nid = \"{challenge}\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"{kind}\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]")
                 .into_boxed_str(),
         );
         RawTrack {
@@ -164,6 +192,21 @@ mod tests {
         assert_eq!(catalog.challenge(&ChallengeId::new("three", "b")), None);
     }
 
+    #[test]
+    fn a_track_with_an_unknown_kind_is_left_out() {
+        let (catalog, errors) = Catalog::load(&[raw_of_kind("odd", 1, "a", "mutation")]);
+        assert!(catalog.tracks().is_empty());
+        assert_eq!(
+            errors,
+            [ContentError::Field {
+                file: "odd/challenges.toml".to_owned(),
+                challenge: "a".to_owned(),
+                field: "kind",
+                problem: "names no kind Quarry knows",
+            }]
+        );
+    }
+
     #[test]
     fn nothing_embedded_is_broken() {
         let (_, errors) = Catalog::embedded();
```

Create `crates/challenges/src/check.rs`:
```rust
//! Checking an answer: run it and the reference solution, then compare
//! their results with the grading pipeline.

use quarry_engine::error::EngineError;

use crate::{
    kind::KindRegistry,
    rules::{self, CheckFailure, Comparison},
    run::{LIMITS, Limits, RunError},
    sandbox::Template,
    track::Challenge,
};

/// What a check decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The answer matches the solution.
    Solved,
    /// The answer is wrong; the failure says how.
    Wrong(CheckFailure),
}

/// Why an answer could not be checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// The user's SQL failed; the database's message.
    Sql(String),
    /// The user's SQL ran out of time.
    TimedOut,
    /// The user's SQL returned more rows than the cap.
    TooManyRows,
    /// The challenge itself is broken: its solution failed, or its kind is
    /// unknown. Says why, for the content's authors.
    Content(String),
    /// The sandbox copy could not be made.
    Sandbox(EngineError),
}

impl From<RunError> for CheckError {
    fn from(error: RunError) -> Self {
        match error {
            RunError::Sql(error) => Self::Sql(error.to_string()),
            RunError::TimedOut => Self::TimedOut,
            RunError::TooManyRows => Self::TooManyRows,
        }
    }
}

/// Checks `sql` as an answer to `challenge`, on copies of `template`, within
/// the grading [`LIMITS`].
///
/// # Errors
/// Returns a [`CheckError`] when the answer or the solution gives no
/// result.
pub fn check(template: &Template, challenge: &Challenge, sql: &str) -> Result<Verdict, CheckError> {
    check_within(template, challenge, sql, LIMITS)
}

/// [`check`] within `limits`.
///
/// # Errors
/// As for [`check`].
pub fn check_within(
    template: &Template,
    challenge: &Challenge,
    sql: &str,
    limits: Limits,
) -> Result<Verdict, CheckError> {
    let kinds = KindRegistry::standard();
    let kind = kinds
        .get(&challenge.kind)
        .ok_or_else(|| CheckError::Content(format!("unknown kind \"{}\"", challenge.kind)))?;
    let answers = kind.answers(challenge, sql, template, limits)?;
    let comparison = Comparison {
        expected: &answers.expected,
        got: &answers.got,
        ordered: challenge.ordered,
    };
    Ok(rules::first_failure(&comparison).map_or(Verdict::Solved, Verdict::Wrong))
}

#[cfg(test)]
mod tests;
```

Create `crates/challenges/src/check/tests.rs`:
```rust
use std::time::Duration;

use quarry_engine::error::EngineError;

use super::{CheckError, Verdict, check, check_within};
use crate::{
    rules::CheckFailure,
    run::{LIMITS, Limits, RunError},
    sandbox::Template,
    track::Challenge,
};

fn template() -> Result<Template, EngineError> {
    Template::new(
        "CREATE TABLE albums (title TEXT, year INTEGER);",
        "INSERT INTO albums VALUES ('Blue', 1971), ('Kind', 1959), ('Ok', NULL);",
    )
}

fn challenge(solution: &str, ordered: bool) -> Challenge {
    Challenge {
        id: "c".to_owned(),
        title: "C".to_owned(),
        prompt: "P".to_owned(),
        kind: "query".to_owned(),
        ordered,
        concepts: Vec::new(),
        solution: solution.to_owned(),
        hints: Vec::new(),
        skeleton: String::new(),
    }
}

#[test]
fn a_matching_answer_is_solved() {
    let challenge = challenge("SELECT title FROM albums WHERE year < 1965", false);
    let answer = "select TITLE from albums where year = 1959";
    assert_eq!(
        check(&template().unwrap(), &challenge, answer),
        Ok(Verdict::Solved)
    );
}

#[test]
fn a_wrong_answer_reports_the_first_failure() {
    let ordered = challenge("SELECT title FROM albums ORDER BY year DESC", true);
    let answer = "SELECT title FROM albums ORDER BY year";
    assert_eq!(
        check(&template().unwrap(), &ordered, answer),
        Ok(Verdict::Wrong(CheckFailure::Order { row: 0 }))
    );
    let unordered = challenge("SELECT title FROM albums ORDER BY year DESC", false);
    assert_eq!(
        check(&template().unwrap(), &unordered, answer),
        Ok(Verdict::Solved)
    );
}

#[test]
fn an_unknown_kind_is_a_content_error() {
    let mut mutation = challenge("SELECT 1", false);
    mutation.kind = "mutation".to_owned();
    assert_eq!(
        check(&template().unwrap(), &mutation, "SELECT 1"),
        Err(CheckError::Content("unknown kind \"mutation\"".to_owned()))
    );
}

#[test]
fn the_limits_apply_to_the_answer() {
    let endless =
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
    let limits = Limits {
        rows: LIMITS.rows,
        time: Duration::from_millis(50),
    };
    let challenge = challenge("SELECT 1", false);
    assert_eq!(
        check_within(&template().unwrap(), &challenge, endless, limits),
        Err(CheckError::TimedOut)
    );
}

#[test]
fn run_errors_become_check_errors() {
    let sql = RunError::Sql(EngineError::Sql("near \"FORM\": syntax error".to_owned()));
    assert_eq!(
        CheckError::from(sql),
        CheckError::Sql("near \"FORM\": syntax error".to_owned())
    );
    assert_eq!(CheckError::from(RunError::TimedOut), CheckError::TimedOut);
    assert_eq!(
        CheckError::from(RunError::TooManyRows),
        CheckError::TooManyRows
    );
}
```

Create `crates/challenges/src/kind/mod.rs`:
```rust
//! Challenge kinds (parent spec §5.3): how a kind turns an answer into the
//! two results the rules compare.
//!
//! Each kind is a [`ChallengeKind`] in the [`KindRegistry`]; adding one is a
//! new module and one line in [`KindRegistry::standard`].

mod query;

use quarry_engine::result::QueryResult;

use crate::{check::CheckError, run::Limits, sandbox::Template, track::Challenge};

/// The user's result and the expected one, for the rules to compare.
#[derive(Debug, Clone, PartialEq)]
pub struct Answers {
    /// What the reference solution gives.
    pub expected: QueryResult,
    /// What the user's SQL gave.
    pub got: QueryResult,
}

/// One kind of challenge.
pub trait ChallengeKind: Send + Sync {
    /// Its id, as `kind` in `challenges.toml`, such as "query".
    fn id(&self) -> &'static str;

    /// Runs `sql` and `challenge`'s solution, each on a fresh copy of
    /// `template`, within `limits`.
    ///
    /// # Errors
    /// Returns a [`CheckError`] when either cannot give a result.
    fn answers(
        &self,
        challenge: &Challenge,
        sql: &str,
        template: &Template,
        limits: Limits,
    ) -> Result<Answers, CheckError>;
}

/// The kinds a challenge can be.
pub struct KindRegistry {
    kinds: Vec<Box<dyn ChallengeKind>>,
}

impl KindRegistry {
    /// Query; Mutation arrives in Phase 2D.
    pub fn standard() -> Self {
        Self {
            kinds: vec![Box::new(query::Query)],
        }
    }

    /// The kind with id `id`.
    pub fn get(&self, id: &str) -> Option<&dyn ChallengeKind> {
        self.kinds
            .iter()
            .find(|kind| kind.id() == id)
            .map(AsRef::as_ref)
    }
}

#[cfg(test)]
mod tests {
    use super::{ChallengeKind, KindRegistry};

    #[test]
    fn query_is_the_only_kind_so_far() {
        let kinds = KindRegistry::standard();
        assert_eq!(kinds.get("query").map(ChallengeKind::id), Some("query"));
        assert!(kinds.get("mutation").is_none());
    }
}
```

Create `crates/challenges/src/kind/query.rs`:
```rust
//! Query challenges: the user's result is compared with the reference
//! solution's, each run on its own fresh copy.

use quarry_engine::sqlite::SqliteDriver;

use super::{Answers, ChallengeKind};
use crate::{
    check::CheckError,
    run::{self, Limits, RunError},
    sandbox::Template,
    track::Challenge,
};

/// A challenge answered by a query.
pub struct Query;

impl ChallengeKind for Query {
    fn id(&self) -> &'static str {
        "query"
    }

    fn answers(
        &self,
        challenge: &Challenge,
        sql: &str,
        template: &Template,
        limits: Limits,
    ) -> Result<Answers, CheckError> {
        let expected = run::run(&mut copy(template)?, &challenge.solution, limits)
            .map_err(|error| CheckError::Content(describe(&error)))?;
        let got = run::run(&mut copy(template)?, sql, limits).map_err(CheckError::from)?;
        Ok(Answers { expected, got })
    }
}

/// A fresh copy of `template`.
fn copy(template: &Template) -> Result<SqliteDriver, CheckError> {
    template.fresh().map_err(CheckError::Sandbox)
}

/// Why the reference solution gave no result, for the content's authors.
fn describe(error: &RunError) -> String {
    match error {
        RunError::Sql(error) => error.to_string(),
        RunError::TimedOut => "the solution ran out of time".to_owned(),
        RunError::TooManyRows => "the solution returned too many rows".to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::{error::EngineError, value::Value};

    use super::{Query, describe};
    use crate::{
        check::CheckError,
        kind::ChallengeKind,
        run::{LIMITS, Limits, RunError},
        sandbox::Template,
        track::Challenge,
    };

    fn challenge(solution: &str) -> Challenge {
        Challenge {
            id: "c".to_owned(),
            title: "C".to_owned(),
            prompt: "P".to_owned(),
            kind: "query".to_owned(),
            ordered: false,
            concepts: Vec::new(),
            solution: solution.to_owned(),
            hints: Vec::new(),
            skeleton: String::new(),
        }
    }

    fn template() -> Result<Template, EngineError> {
        Template::new(
            "CREATE TABLE t (n INTEGER);",
            "INSERT INTO t VALUES (1), (2);",
        )
    }

    #[test]
    fn each_side_runs_on_its_own_fresh_copy() {
        let sql = "DELETE FROM t; SELECT count(*) AS n FROM t";
        let answers = Query
            .answers(
                &challenge("SELECT count(*) AS n FROM t"),
                sql,
                &template().unwrap(),
                LIMITS,
            )
            .unwrap();
        assert_eq!(answers.expected.rows, [[Value::Int(2)]]);
        assert_eq!(answers.got.rows, [[Value::Int(0)]]);
    }

    #[test]
    fn the_users_errors_keep_their_kind() {
        let template = template().unwrap();
        let solution = challenge("SELECT n FROM t");
        let failed = Query.answers(&solution, "SELECT x FROM t", &template, LIMITS);
        assert_eq!(failed, Err(CheckError::Sql("no such column: x".to_owned())));
        let tight = Limits {
            rows: 1,
            time: LIMITS.time,
        };
        let too_many = Query.answers(&challenge("SELECT 1"), "SELECT n FROM t", &template, tight);
        assert_eq!(too_many, Err(CheckError::TooManyRows));
    }

    #[test]
    fn a_failing_solution_is_a_content_error() {
        let broken = Query.answers(
            &challenge("SELECT nope FROM t"),
            "SELECT 1",
            &template().unwrap(),
            LIMITS,
        );
        assert_eq!(
            broken,
            Err(CheckError::Content("no such column: nope".to_owned()))
        );
    }

    #[test]
    fn content_errors_say_what_stopped_the_solution() {
        assert_eq!(
            describe(&RunError::TimedOut),
            "the solution ran out of time"
        );
        assert_eq!(
            describe(&RunError::TooManyRows),
            "the solution returned too many rows"
        );
    }
}
```

Modify `crates/challenges/src/lib.rs`:
```diff
@@ -1,8 +1,11 @@
 //! SQL challenge tracks: loading, sandboxes, grading, and hints.
 
 pub mod catalog;
+pub mod check;
 pub mod embedded;
 pub mod error;
+pub mod kind;
+pub mod rules;
 pub mod run;
 pub mod sandbox;
 pub mod track;
```

Create `crates/challenges/src/rules/columns.rs`:
```rust
//! Columns: the same number, with the same names in the same order,
//! ignoring case.

use super::{CheckFailure, CheckRule, Comparison};
use quarry_engine::result::QueryResult;

/// The first rule.
pub struct Columns;

impl CheckRule for Columns {
    fn id(&self) -> &'static str {
        "columns"
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        let expected = names(comparison.expected);
        let got = names(comparison.got);
        if expected.len() != got.len() {
            return Some(CheckFailure::ColumnCount {
                expected: expected.len(),
                got: got.len(),
            });
        }
        let differ = expected
            .iter()
            .zip(&got)
            .any(|(a, b)| a.to_lowercase() != b.to_lowercase());
        differ.then_some(CheckFailure::ColumnNames { expected, got })
    }
}

fn names(result: &QueryResult) -> Vec<String> {
    result
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Columns;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    fn check(expected: &[&str], got: &[&str]) -> Option<CheckFailure> {
        let expected = result(expected, ints(&[]));
        let got = result(got, ints(&[]));
        Columns.check(&Comparison {
            expected: &expected,
            got: &got,
            ordered: false,
        })
    }

    #[test]
    fn names_match_ignoring_case() {
        assert_eq!(check(&["title", "year"], &["Title", "YEAR"]), None);
    }

    #[test]
    fn a_different_count_is_reported_first() {
        assert_eq!(
            check(&["title", "year"], &["title"]),
            Some(CheckFailure::ColumnCount {
                expected: 2,
                got: 1
            })
        );
    }

    #[test]
    fn different_names_or_order_are_reported_with_both_lists() {
        assert_eq!(
            check(&["title", "year"], &["year", "title"]),
            Some(CheckFailure::ColumnNames {
                expected: vec!["title".to_owned(), "year".to_owned()],
                got: vec!["year".to_owned(), "title".to_owned()],
            })
        );
    }
}
```

Create `crates/challenges/src/rules/contents.rs`:
```rust
//! Contents: the same rows as a multiset, ignoring order and counting
//! duplicates, with cells compared by [`value::same`].

use std::cmp::Ordering;

use quarry_engine::result::Row;

use super::{CheckFailure, CheckRule, Comparison, value};

/// The third rule.
pub struct Contents;

impl CheckRule for Contents {
    fn id(&self) -> &'static str {
        "contents"
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        let expected = sorted(&comparison.expected.rows);
        let got = sorted(&comparison.got.rows);
        let (missing, unexpected) = differences(&expected, &got);
        (missing + unexpected > 0).then_some(CheckFailure::Contents {
            missing,
            unexpected,
        })
    }
}

fn sorted(rows: &[Row]) -> Vec<&Row> {
    let mut sorted: Vec<&Row> = rows.iter().collect();
    sorted.sort_by(|a, b| value::compare_rows(a, b));
    sorted
}

/// How many of `expected`'s rows `got` lacks, and how many of `got`'s rows
/// `expected` lacks.
fn differences(expected: &[&Row], got: &[&Row]) -> (usize, usize) {
    let matched = matched(expected, got);
    (expected.len() - matched, got.len() - matched)
}

/// How many rows the two sorted lists share, walking them together.
fn matched(expected: &[&Row], got: &[&Row]) -> usize {
    let (mut i, mut j, mut matched) = (0, 0, 0);
    while i < expected.len() && j < got.len() {
        if value::same_row(expected[i], got[j]) {
            matched += 1;
            i += 1;
            j += 1;
        } else if value::compare_rows(expected[i], got[j]) == Ordering::Less {
            i += 1;
        } else {
            j += 1;
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use quarry_engine::value::Value;

    use super::Contents;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    fn check(expected: &[i64], got: &[i64]) -> Option<CheckFailure> {
        let expected = result(&["n"], ints(expected));
        let got = result(&["n"], ints(got));
        Contents.check(&Comparison {
            expected: &expected,
            got: &got,
            ordered: false,
        })
    }

    #[test]
    fn order_is_ignored() {
        assert_eq!(check(&[1, 2, 3], &[3, 1, 2]), None);
    }

    #[test]
    fn duplicates_are_counted() {
        assert_eq!(
            check(&[1, 1, 2], &[1, 2, 2]),
            Some(CheckFailure::Contents {
                missing: 1,
                unexpected: 1
            })
        );
    }

    #[test]
    fn missing_and_unexpected_rows_are_counted_at_either_end() {
        assert_eq!(
            check(&[1, 2, 3, 4], &[0, 2, 3, 9]),
            Some(CheckFailure::Contents {
                missing: 2,
                unexpected: 2
            })
        );
        assert_eq!(
            check(&[5, 6], &[]),
            Some(CheckFailure::Contents {
                missing: 2,
                unexpected: 0
            })
        );
        assert_eq!(
            check(&[], &[5]),
            Some(CheckFailure::Contents {
                missing: 0,
                unexpected: 1
            })
        );
    }

    #[test]
    fn cells_compare_with_nulls_and_numbers_as_numbers() {
        let expected = result(&["a", "b"], vec![vec![Value::Null, Value::Int(4)]]);
        let got = result(&["a", "b"], vec![vec![Value::Null, Value::Float(4.0)]]);
        let comparison = Comparison {
            expected: &expected,
            got: &got,
            ordered: false,
        };
        assert_eq!(Contents.check(&comparison), None);
    }
}
```

Create `crates/challenges/src/rules/count.rs`:
```rust
//! Row count: the same number of rows.

use super::{CheckFailure, CheckRule, Comparison};

/// The second rule.
pub struct RowCount;

impl CheckRule for RowCount {
    fn id(&self) -> &'static str {
        "row count"
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        let expected = comparison.expected.rows.len();
        let got = comparison.got.rows.len();
        (expected != got).then_some(CheckFailure::RowCount { expected, got })
    }
}

#[cfg(test)]
mod tests {
    use super::RowCount;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    #[test]
    fn counts_rows_only() {
        let expected = result(&["n"], ints(&[1, 2, 3]));
        let same_count = result(&["n"], ints(&[7, 8, 9]));
        let fewer = result(&["n"], ints(&[1]));
        let check = |got| {
            RowCount.check(&Comparison {
                expected: &expected,
                got,
                ordered: false,
            })
        };
        assert_eq!(check(&same_count), None);
        assert_eq!(
            check(&fewer),
            Some(CheckFailure::RowCount {
                expected: 3,
                got: 1
            })
        );
    }
}
```

Create `crates/challenges/src/rules/mod.rs`:
```rust
//! The grading pipeline (parent spec §5.4): columns, row count, contents,
//! then order. It stops at the first rule that fails.
//!
//! Each rule is a [`CheckRule`]; adding one is a new module and one line in
//! [`pipeline`].

mod columns;
mod contents;
mod count;
mod order;
pub mod value;

use quarry_engine::result::QueryResult;

/// What was wrong with an answer. It says what differs, never the expected
/// rows themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckFailure {
    /// The answer has a different number of columns.
    ColumnCount {
        /// How many the solution returns.
        expected: usize,
        /// How many the answer returned.
        got: usize,
    },
    /// The columns' names differ, ignoring case.
    ColumnNames {
        /// The solution's column names, in order.
        expected: Vec<String>,
        /// The answer's column names, in order.
        got: Vec<String>,
    },
    /// The answer has a different number of rows.
    RowCount {
        /// How many the solution returns.
        expected: usize,
        /// How many the answer returned.
        got: usize,
    },
    /// The rows differ, ignoring their order.
    Contents {
        /// Rows the solution returns that the answer lacks.
        missing: usize,
        /// Rows the answer returned that the solution doesn't.
        unexpected: usize,
    },
    /// The rows are right but in the wrong order.
    Order {
        /// The first row out of place, counting from 0.
        row: usize,
    },
}

/// The two results a rule compares.
#[derive(Debug, Clone, Copy)]
pub struct Comparison<'a> {
    /// The reference solution's result.
    pub expected: &'a QueryResult,
    /// The user's result.
    pub got: &'a QueryResult,
    /// Whether the rows must come in the solution's order.
    pub ordered: bool,
}

/// One step of the pipeline.
pub trait CheckRule {
    /// A stable name, such as "columns".
    fn id(&self) -> &'static str;

    /// What this rule finds wrong, if anything. It may assume every earlier
    /// rule passed.
    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure>;
}

/// The rules in the order they run.
pub fn pipeline() -> Vec<Box<dyn CheckRule>> {
    vec![
        Box::new(columns::Columns),
        Box::new(count::RowCount),
        Box::new(contents::Contents),
        Box::new(order::Order),
    ]
}

/// The first rule's failure, or `None` when every rule passes.
pub fn first_failure(comparison: &Comparison<'_>) -> Option<CheckFailure> {
    pipeline().iter().find_map(|rule| rule.check(comparison))
}

#[cfg(test)]
pub(crate) mod testing {
    //! Building results for the rules' tests.

    use quarry_engine::{
        result::{Column, QueryResult},
        value::Value,
    };

    /// A result with columns `names` and `rows`.
    pub fn result(names: &[&str], rows: Vec<Vec<Value>>) -> QueryResult {
        let columns = names
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect();
        QueryResult { columns, rows }
    }

    /// One-column rows holding `numbers`.
    pub fn ints(numbers: &[i64]) -> Vec<Vec<Value>> {
        numbers.iter().map(|n| vec![Value::Int(*n)]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CheckFailure, Comparison, first_failure, pipeline,
        testing::{ints, result},
    };

    #[test]
    fn the_rules_run_columns_count_contents_then_order() {
        let ids: Vec<&str> = pipeline().iter().map(|rule| rule.id()).collect();
        assert_eq!(ids, ["columns", "row count", "contents", "order"]);
    }

    #[test]
    fn the_first_failing_rule_is_reported() {
        let expected = result(&["n"], ints(&[1, 2]));
        let wrong_columns = result(&["m", "n"], ints(&[3]));
        let comparison = Comparison {
            expected: &expected,
            got: &wrong_columns,
            ordered: true,
        };
        assert_eq!(
            first_failure(&comparison),
            Some(CheckFailure::ColumnCount {
                expected: 1,
                got: 2
            })
        );
    }

    #[test]
    fn a_matching_answer_passes_every_rule() {
        let expected = result(&["n"], ints(&[1, 2]));
        let got = result(&["N"], ints(&[1, 2]));
        let comparison = Comparison {
            expected: &expected,
            got: &got,
            ordered: true,
        };
        assert_eq!(first_failure(&comparison), None);
    }
}
```

Create `crates/challenges/src/rules/order.rs`:
```rust
//! Order: when the challenge says order matters, the rows come in the
//! solution's order.

use super::{CheckFailure, CheckRule, Comparison, value};

/// The fourth rule; it checks nothing unless `ordered` is set.
pub struct Order;

impl CheckRule for Order {
    fn id(&self) -> &'static str {
        "order"
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        if !comparison.ordered {
            return None;
        }
        comparison
            .expected
            .rows
            .iter()
            .zip(&comparison.got.rows)
            .position(|(a, b)| !value::same_row(a, b))
            .map(|row| CheckFailure::Order { row })
    }
}

#[cfg(test)]
mod tests {
    use super::Order;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    fn check(expected: &[i64], got: &[i64], ordered: bool) -> Option<CheckFailure> {
        let expected = result(&["n"], ints(expected));
        let got = result(&["n"], ints(got));
        Order.check(&Comparison {
            expected: &expected,
            got: &got,
            ordered,
        })
    }

    #[test]
    fn reports_the_first_row_out_of_place() {
        assert_eq!(
            check(&[1, 2, 3], &[1, 3, 2], true),
            Some(CheckFailure::Order { row: 1 })
        );
        assert_eq!(check(&[1, 2, 3], &[1, 2, 3], true), None);
    }

    #[test]
    fn checks_nothing_when_order_does_not_matter() {
        assert_eq!(check(&[1, 2, 3], &[3, 2, 1], false), None);
    }
}
```

Create `crates/challenges/src/rules/value.rs`:
```rust
//! When two cells count as equal, and one order for sorting rows.
//!
//! `NULL` equals `NULL`, an integer equals the same value as a float, floats
//! are equal within a relative tolerance, and text and bytes must match
//! exactly.

use std::cmp::Ordering;

use quarry_engine::value::Value;

/// How close two floats must be, relative to the larger.
pub const TOLERANCE: f64 = 1e-9;

/// Whether `a` and `b` count as the same cell.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        _ => match (number(a), number(b)) {
            (Some(x), Some(y)) => close(x, y),
            _ => a == b,
        },
    }
}

/// Whether two rows hold the same cells, column by column.
pub fn same_row(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
}

/// One total order over cells: `NULL`, then numbers by value, then text,
/// then bytes.
pub fn compare(a: &Value, b: &Value) -> Ordering {
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        _ => match (a, b) {
            (Value::Text(x), Value::Text(y)) => x.cmp(y),
            (Value::Bytes(x), Value::Bytes(y)) => x.cmp(y),
            _ => rank(a).cmp(&rank(b)),
        },
    }
}

/// Rows compared cell by cell with [`compare`], then by length.
pub fn compare_rows(a: &[Value], b: &[Value]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| compare(x, y))
        .find(|order| order.is_ne())
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

/// Where a cell's kind sorts.
fn rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Int(_) | Value::Float(_) => 1,
        Value::Text(_) => 2,
        Value::Bytes(_) => 3,
    }
}

/// A number's value as a float.
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(float(*n)),
        Value::Float(x) => Some(*x),
        _ => None,
    }
}

/// Whether `x` and `y` differ by at most [`TOLERANCE`] of the larger.
/// An infinity is close only to itself.
fn close(x: f64, y: f64) -> bool {
    let within = || (x - y).abs() <= TOLERANCE * x.abs().max(y.abs());
    x.total_cmp(&y).is_eq() || (x.is_finite() && y.is_finite() && within())
}

/// `n` as the nearest float, without a lossy cast: its high and low 32 bits
/// each convert exactly.
fn float(n: i64) -> f64 {
    let high = i32::try_from(n >> 32).unwrap_or(0);
    let low = u32::try_from(n & 0xFFFF_FFFF).unwrap_or(0);
    f64::from(high) * 4_294_967_296.0 + f64::from(low)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use quarry_engine::value::Value;

    use super::{compare, compare_rows, float, same, same_row};

    fn text(s: &str) -> Value {
        Value::Text(s.to_owned())
    }

    #[test]
    fn integers_convert_exactly() {
        let cases = [
            (0, 0.0),
            (7, 7.0),
            (-3, -3.0),
            (5_000_000_123, 5_000_000_123.0),
            (-5_000_000_123, -5_000_000_123.0),
        ];
        for (n, x) in cases {
            assert_eq!(float(n).to_bits(), f64::to_bits(x), "{n}");
        }
    }

    #[test]
    fn equal_infinities_match() {
        assert!(same(
            &Value::Float(f64::INFINITY),
            &Value::Float(f64::INFINITY)
        ));
        assert!(!same(
            &Value::Float(f64::INFINITY),
            &Value::Float(f64::NEG_INFINITY)
        ));
    }

    #[test]
    fn null_equals_null_and_nothing_else() {
        assert!(same(&Value::Null, &Value::Null));
        assert!(!same(&Value::Null, &Value::Int(0)));
        assert!(!same(&Value::Null, &text("")));
    }

    #[test]
    fn an_integer_equals_the_same_float() {
        assert!(same(&Value::Int(4), &Value::Float(4.0)));
        assert!(same(&Value::Float(4.0), &Value::Int(4)));
        assert!(!same(&Value::Int(4), &Value::Float(4.5)));
        assert!(!same(&Value::Int(4), &Value::Int(5)));
    }

    #[test]
    fn large_integers_compare_exactly_with_each_other() {
        let big = 9_007_199_254_740_993;
        assert!(!same(&Value::Int(big), &Value::Int(big - 1)));
    }

    #[test]
    fn floats_are_equal_within_a_relative_tolerance() {
        assert!(same(&Value::Float(0.1 + 0.2), &Value::Float(0.3)));
        assert!(same(&Value::Float(1e12), &Value::Float(1e12 + 1e-1)));
        assert!(!same(&Value::Float(1.0), &Value::Float(1.000_001)));
        assert!(!same(&Value::Float(1e-12), &Value::Float(2e-12)));
        assert!(same(&Value::Float(0.0), &Value::Float(-0.0)));
    }

    #[test]
    fn text_and_bytes_match_exactly() {
        assert!(same(&text("Blue"), &text("Blue")));
        assert!(!same(&text("Blue"), &text("blue")));
        assert!(!same(&text("4"), &Value::Int(4)));
        assert!(same(&Value::Bytes(vec![1]), &Value::Bytes(vec![1])));
        assert!(!same(&Value::Bytes(vec![1]), &Value::Bytes(vec![2])));
    }

    #[test]
    fn rows_match_cell_by_cell() {
        assert!(same_row(
            &[Value::Int(1), text("a")],
            &[Value::Float(1.0), text("a")]
        ));
        assert!(!same_row(
            &[Value::Int(1), text("a")],
            &[Value::Int(1), text("b")]
        ));
        assert!(!same_row(&[Value::Int(1)], &[Value::Int(1), Value::Null]));
    }

    #[test]
    fn cells_sort_null_then_numbers_then_text_then_bytes() {
        let ordered = [
            Value::Null,
            Value::Int(-2),
            Value::Float(1.5),
            Value::Int(2),
            text("a"),
            text("b"),
            Value::Bytes(vec![0]),
            Value::Bytes(vec![1]),
        ];
        for pair in ordered.windows(2) {
            assert_eq!(compare(&pair[0], &pair[1]), Ordering::Less, "{pair:?}");
            assert_eq!(compare(&pair[1], &pair[0]), Ordering::Greater, "{pair:?}");
        }
        assert_eq!(compare(&Value::Int(2), &Value::Float(2.0)), Ordering::Equal);
        assert_eq!(compare(&Value::Null, &Value::Null), Ordering::Equal);
    }

    #[test]
    fn rows_sort_by_their_first_differing_cell_then_length() {
        let short = [Value::Int(1)];
        let long = [Value::Int(1), Value::Int(0)];
        let later = [Value::Int(2)];
        assert_eq!(compare_rows(&short, &long), Ordering::Less);
        assert_eq!(compare_rows(&long, &later), Ordering::Less);
        assert_eq!(compare_rows(&later, &long), Ordering::Greater);
        assert_eq!(compare_rows(&short, &short), Ordering::Equal);
    }
}
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry-challenges`
Expected: 58 tests pass.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/challenges/src/catalog.rs crates/challenges/src/check.rs crates/challenges/src/check/tests.rs crates/challenges/src/kind/mod.rs crates/challenges/src/kind/query.rs crates/challenges/src/lib.rs crates/challenges/src/rules/columns.rs crates/challenges/src/rules/contents.rs crates/challenges/src/rules/count.rs crates/challenges/src/rules/mod.rs crates/challenges/src/rules/order.rs crates/challenges/src/rules/value.rs
git commit -m "feat(challenges): grade query answers through the check pipeline"
```

---

### Task 5: Content: Vinyl Vault's first five challenges

The first track: a fictional second-hand record shop with `artists`, `albums` and `pressings`. Five challenges cover SELECT, WHERE, ORDER BY with LIMIT (ordered), DISTINCT and NULL handling. The content tests check that every track loads, every solution solves its challenge and returns rows, no skeleton passes, and real wrong answers fail each of the four rules.

**Files:**
- Create: `content/tracks/vinyl-vault/challenges.toml`
- Create: `content/tracks/vinyl-vault/schema.sql`
- Create: `content/tracks/vinyl-vault/seed.sql`
- Create: `content/tracks/vinyl-vault/track.toml`
- Create: `crates/challenges/tests/content.rs`

**Interfaces:**
- Consumes: everything in `quarry-challenges`.
- Produces: `content/tracks/vinyl-vault/`, whose challenge ids are `first-spin`, `crate-digging`, `new-arrivals`, `every-genre` and `unknown-origins`. `albums` has 18 rows.

- [ ] **Step 1: Implement**

Create `content/tracks/vinyl-vault/challenges.toml`:
```toml
# Vinyl Vault's first challenges (Phase 2A). The rest arrive in 2E.

[[challenge]]
id = "first-spin"
title = "First spin"
prompt = """
On your first morning the owner hands you a clipboard: "Write down every \
album we've got, with the year it came out." List the title and year of \
every album.\
"""
kind = "query"
ordered = false
concepts = ["SELECT"]
solution = "SELECT title, year FROM albums;"
hints = [
    "Every album lives in one table. You only need two of its columns.",
    "SELECT names the columns you want, and FROM names the table.",
    "SELECT ___, ___ FROM ___;",
]
skeleton = "SELECT ___, ___\nFROM ___;"

[[challenge]]
id = "crate-digging"
title = "Crate digging"
prompt = """
A collector wants anything from before 1975. List the title and year of \
every album released before 1975.\
"""
kind = "query"
ordered = false
concepts = ["WHERE"]
solution = "SELECT title, year FROM albums WHERE year < 1975;"
hints = [
    "Keep only some of the rows: the ones with an early year.",
    "WHERE filters rows. Be careful whether 1975 itself counts.",
    "SELECT title, year FROM albums WHERE year ___ ___;",
]
skeleton = "SELECT title, year\nFROM albums\nWHERE ___;"

[[challenge]]
id = "new-arrivals"
title = "New arrivals"
prompt = """
The window display shows the newest records. List the title and year of \
the five most recent albums, newest first.\
"""
kind = "query"
ordered = true
concepts = ["ORDER BY", "LIMIT"]
solution = "SELECT title, year FROM albums ORDER BY year DESC LIMIT 5;"
hints = [
    "Sort the albums by year so the newest come first, then keep the top few.",
    "ORDER BY ... DESC sorts from highest to lowest, and LIMIT keeps the first rows.",
    "SELECT title, year FROM albums ORDER BY ___ ___ LIMIT ___;",
]
skeleton = "SELECT title, year\nFROM albums\nORDER BY ___\nLIMIT ___;"

[[challenge]]
id = "every-genre"
title = "Every genre"
prompt = """
The owner is making new divider cards for the racks, one per genre. List \
each genre once.\
"""
kind = "query"
ordered = false
concepts = ["DISTINCT"]
solution = "SELECT DISTINCT genre FROM albums;"
hints = [
    "Many albums share a genre, but each card should appear only once.",
    "DISTINCT removes repeated rows from a result.",
    "SELECT ___ genre FROM albums;",
]
skeleton = "SELECT ___\nFROM albums;"

[[challenge]]
id = "unknown-origins"
title = "Unknown origins"
prompt = """
Some artists arrived with no paperwork, so nobody knows where they're from. \
List the names of the artists whose country is unknown.\
"""
kind = "query"
ordered = false
concepts = ["NULL handling"]
solution = "SELECT name FROM artists WHERE country IS NULL;"
hints = [
    "An unknown country is stored as NULL, which isn't equal to anything, not even NULL.",
    "Test for NULL with IS NULL, not with =.",
    "SELECT name FROM artists WHERE country ___ ___;",
]
skeleton = "SELECT name\nFROM artists\nWHERE ___;"
```

Create `content/tracks/vinyl-vault/schema.sql`:
```
-- Vinyl Vault: a fictional second-hand record shop. Every artist, album
-- and pressing here is made up.

CREATE TABLE artists (
    id INTEGER PRIMARY KEY,
    name TEXT NOT NULL,
    country TEXT -- NULL when nobody knows where the artist is from
);

CREATE TABLE albums (
    id INTEGER PRIMARY KEY,
    artist_id INTEGER NOT NULL REFERENCES artists (id),
    title TEXT NOT NULL,
    year INTEGER NOT NULL,
    genre TEXT NOT NULL
);

CREATE TABLE pressings (
    id INTEGER PRIMARY KEY,
    album_id INTEGER NOT NULL REFERENCES albums (id),
    format TEXT NOT NULL,
    condition TEXT NOT NULL,
    price REAL NOT NULL,
    in_stock INTEGER NOT NULL
);
```

Create `content/tracks/vinyl-vault/seed.sql`:
```
-- Hand-written for Phase 2A; `cargo xtask gen-seed` replaces it in 2D.

INSERT INTO artists (id, name, country) VALUES
    (1, 'Static Orchard', 'UK'),
    (2, 'Marisol Okafor', 'Nigeria'),
    (3, 'Hollow Pine Collective', 'USA'),
    (4, 'Kestrel Tides', 'Ireland'),
    (5, 'Nadia Varga Quartet', 'Hungary'),
    (6, 'Low Orbit Choir', NULL),
    (7, 'Juniper Vale', 'Canada'),
    (8, 'Lantern Ferry', 'Japan'),
    (9, 'Sundial Sound System', NULL),
    (10, 'Ottoline Reyes', 'Mexico');

INSERT INTO albums (id, artist_id, title, year, genre) VALUES
    (1, 1, 'Copper Skies', 1968, 'Rock'),
    (2, 1, 'Night Bus Hymns', 1971, 'Rock'),
    (3, 2, 'Lagos After Rain', 1974, 'Afrobeat'),
    (4, 2, 'Brass Weather', 1979, 'Afrobeat'),
    (5, 3, 'Timber and Smoke', 1972, 'Folk'),
    (6, 3, 'Long Porch Evenings', 1975, 'Folk'),
    (7, 4, 'Saltwater Letters', 1981, 'Folk'),
    (8, 5, 'Blue Hour Variations', 1963, 'Jazz'),
    (9, 5, 'Late Set at the Kalina', 1966, 'Jazz'),
    (10, 6, 'Signal from Perigee', 1983, 'Electronic'),
    (11, 7, 'Maple Static', 1988, 'Pop'),
    (12, 7, 'Paper Planets', 1991, 'Pop'),
    (13, 8, 'Harbor Lights at Four', 1978, 'Jazz'),
    (14, 9, 'Equinox Dub', 1985, 'Reggae'),
    (15, 10, 'Cantos del Mercado', 1977, 'Latin'),
    (16, 10, 'Marigold Nights', 1986, 'Latin'),
    (17, 4, 'Gull Song', 1969, 'Folk'),
    (18, 6, 'Static Bloom', 1995, 'Electronic');

INSERT INTO pressings (id, album_id, format, condition, price, in_stock) VALUES
    (1, 1, 'LP', 'Very Good', 38.5, 1),
    (2, 1, 'LP', 'Near Mint', 64.0, 0),
    (3, 3, 'LP', 'Good', 22.0, 1),
    (4, 5, 'LP', 'Very Good', 27.5, 1),
    (5, 8, 'LP', 'Near Mint', 85.0, 1),
    (6, 9, 'LP', 'Fair', 12.0, 1),
    (7, 11, '7-inch', 'Very Good', 6.5, 1),
    (8, 12, 'LP', 'Very Good', 19.0, 0),
    (9, 13, 'LP', 'Good', 31.0, 1),
    (10, 14, '12-inch', 'Near Mint', 24.0, 1),
    (11, 16, 'LP', 'Very Good', 17.5, 1),
    (12, 18, 'LP', 'Near Mint', 29.0, 1);
```

Create `content/tracks/vinyl-vault/track.toml`:
```toml
title = "Vinyl Vault"
emoji = "🎧"
intro = """
You've just started at Vinyl Vault, a second-hand record shop down a side \
street. The owner keeps every artist, album and pressing in a small \
database, and customers ask questions all day. Answer them with SQL.\
"""
concepts = ["SELECT", "WHERE", "ORDER BY", "LIMIT", "DISTINCT", "CASE", "NULL handling", "basic functions"]
difficulty = "Beginner"
order = 1
```

Create `crates/challenges/tests/content.rs`:
```rust
//! Content tests (parent spec §7.3): every embedded track loads, and every
//! challenge can be solved by its own solution and not by its skeleton.

use quarry_challenges::{
    catalog::{Catalog, ChallengeId},
    check::{CheckError, Verdict, check},
    rules::CheckFailure,
    run::{LIMITS, run},
    sandbox::Template,
    track::Track,
};
use quarry_engine::error::EngineError;

fn catalog() -> Catalog {
    let (catalog, errors) = Catalog::embedded();
    assert_eq!(errors, []);
    catalog
}

fn template(track: &Track) -> Result<Template, EngineError> {
    Template::new(&track.schema, &track.seed)
}

/// Checks `sql` against challenge `id` of Vinyl Vault.
fn vinyl(id: &str, sql: &str) -> Result<Verdict, CheckError> {
    let catalog = catalog();
    let Some((track, challenge)) = catalog.challenge(&ChallengeId::new("vinyl-vault", id)) else {
        return Err(CheckError::Content(format!("no challenge {id}")));
    };
    let template = template(track).map_err(CheckError::Sandbox)?;
    check(&template, challenge, sql)
}

#[test]
fn every_track_loads_and_vinyl_vault_comes_first() {
    let catalog = catalog();
    let first = catalog.tracks().first().expect("at least one track");
    assert_eq!(first.slug, "vinyl-vault");
    assert_eq!(first.title, "Vinyl Vault");
    let ids: Vec<&str> = first.challenges.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "first-spin",
            "crate-digging",
            "new-arrivals",
            "every-genre",
            "unknown-origins"
        ]
    );
}

#[test]
fn every_solution_solves_its_challenge_and_returns_rows() {
    for track in catalog().tracks() {
        let template = template(track).unwrap();
        for challenge in &track.challenges {
            let verdict = check(&template, challenge, &challenge.solution);
            assert_eq!(
                verdict,
                Ok(Verdict::Solved),
                "{}/{}",
                track.slug,
                challenge.id
            );
            let mut copy = template.fresh().expect("a copy");
            let rows = run(&mut copy, &challenge.solution, LIMITS)
                .expect("the solution runs")
                .rows;
            assert!(
                !rows.is_empty(),
                "{}/{} returns no rows",
                track.slug,
                challenge.id
            );
        }
    }
}

#[test]
fn no_skeleton_is_already_an_answer() {
    for track in catalog().tracks() {
        let template = template(track).unwrap();
        for challenge in &track.challenges {
            let verdict = check(&template, challenge, &challenge.skeleton);
            assert_ne!(
                verdict,
                Ok(Verdict::Solved),
                "{}/{}",
                track.slug,
                challenge.id
            );
        }
    }
}

#[test]
fn wrong_columns_are_caught() {
    assert_eq!(
        vinyl("first-spin", "SELECT title FROM albums"),
        Ok(Verdict::Wrong(CheckFailure::ColumnCount {
            expected: 2,
            got: 1
        }))
    );
    assert!(matches!(
        vinyl("first-spin", "SELECT title, year AS released FROM albums"),
        Ok(Verdict::Wrong(CheckFailure::ColumnNames { .. }))
    ));
}

#[test]
fn a_wrong_row_count_is_caught() {
    assert_eq!(
        vinyl(
            "crate-digging",
            "SELECT title, year FROM albums WHERE year <= 1975"
        ),
        Ok(Verdict::Wrong(CheckFailure::RowCount {
            expected: 7,
            got: 8
        }))
    );
    assert_eq!(
        vinyl(
            "unknown-origins",
            "SELECT name FROM artists WHERE country = NULL"
        ),
        Ok(Verdict::Wrong(CheckFailure::RowCount {
            expected: 2,
            got: 0
        }))
    );
    assert_eq!(
        vinyl("every-genre", "SELECT genre FROM albums"),
        Ok(Verdict::Wrong(CheckFailure::RowCount {
            expected: 8,
            got: 18
        }))
    );
}

#[test]
fn wrong_rows_and_wrong_order_are_caught() {
    assert_eq!(
        vinyl(
            "new-arrivals",
            "SELECT title, year FROM albums ORDER BY year LIMIT 5"
        ),
        Ok(Verdict::Wrong(CheckFailure::Contents {
            missing: 5,
            unexpected: 5
        }))
    );
    assert_eq!(
        vinyl(
            "new-arrivals",
            "SELECT title, year FROM albums WHERE year >= 1985 ORDER BY year"
        ),
        Ok(Verdict::Wrong(CheckFailure::Order { row: 0 }))
    );
}
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry-challenges`
Expected: 64 tests pass, including the 6 in `tests/content.rs`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add content/tracks/vinyl-vault/challenges.toml content/tracks/vinyl-vault/schema.sql content/tracks/vinyl-vault/seed.sql content/tracks/vinyl-vault/track.toml crates/challenges/tests/content.rs
git commit -m "feat(content): add Vinyl Vault's first five challenges"
```

---

### Task 6: Store: schema v3 and `Draft::challenge`

A tab remembers its challenge. Migration v3 adds a nullable `challenge` column to `drafts` and `closed_tabs`, and `Draft` gains `challenge: Option<String>`, the challenge's full id. A v2 file upgrades in place, its tabs plain. Every place that builds a `Draft` sets `challenge: None` for now; Task 8 fills it in.

**Files:**
- Modify: `crates/app/src/model/closed_tabs.rs`
- Modify: `crates/app/src/views/workspace/drafts.rs`
- Modify: `crates/app/src/views/workspace/tabs.rs`
- Modify: `crates/app/tests/app/tabs.rs`
- Modify: `crates/store/src/drafts.rs`
- Modify: `crates/store/src/migrations.rs`
- Modify: `crates/store/src/sqlite.rs`
- Modify: `crates/store/tests/drafts.rs`

**Interfaces:**
- Consumes: Phase 1E's `quarry_store`.
- Produces: `Draft { id, title, text, challenge: Option<String> }`; `migrations::VERSION == 3`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/tabs.rs`:
```diff
@@ -54,6 +54,7 @@ fn draft(id: i64, title: &str, text: &str) -> Draft {
         id,
         title: title.to_owned(),
         text: text.to_owned(),
+        challenge: None,
     }
 }
 
```

Modify `crates/store/tests/drafts.rs`:
```diff
@@ -11,6 +11,14 @@ fn draft(id: i64, title: &str, text: &str) -> Draft {
         id,
         title: title.to_owned(),
         text: text.to_owned(),
+        challenge: None,
+    }
+}
+
+fn challenge_draft(id: i64, title: &str, challenge: &str) -> Draft {
+    Draft {
+        challenge: Some(challenge.to_owned()),
+        ..draft(id, title, "SELECT ___;")
     }
 }
 
@@ -86,7 +94,7 @@ fn leaves_a_newer_file_untouched() {
         error,
         Some(StoreError::TooNew {
             found: 5,
-            supported: 2
+            supported: 3
         })
     );
     assert_eq!(std::fs::read(&path).unwrap(), before);
@@ -161,6 +169,44 @@ fn a_version_one_file_is_upgraded_and_keeps_its_tabs() {
     );
 }
 
+#[test]
+fn challenge_ids_round_trip_through_tabs_and_closed_tabs() {
+    let store = SqliteStore::in_memory().unwrap();
+    let drafts = Drafts {
+        tabs: vec![
+            challenge_draft(2, "First spin", "vinyl-vault/first-spin"),
+            draft(3, "Query 1", "SELECT 1;"),
+        ],
+        active: 0,
+        closed: vec![challenge_draft(0, "Every genre", "vinyl-vault/every-genre")],
+    };
+    store.save(&drafts).unwrap();
+    assert_eq!(store.load().unwrap(), drafts);
+}
+
+#[test]
+fn a_version_two_file_is_upgraded_and_its_tabs_are_not_challenges() {
+    let dir = tempfile::tempdir().unwrap();
+    let path = dir.path().join("quarry.db");
+    let connection = rusqlite::Connection::open(&path).unwrap();
+    connection
+        .execute_batch(
+            "CREATE TABLE drafts (id INTEGER PRIMARY KEY, position INTEGER NOT NULL,
+                 title TEXT NOT NULL, text TEXT NOT NULL);
+             CREATE TABLE state (key TEXT PRIMARY KEY, value INTEGER NOT NULL);
+             CREATE TABLE closed_tabs (position INTEGER NOT NULL, title TEXT NOT NULL,
+                 text TEXT NOT NULL);
+             INSERT INTO drafts VALUES (5, 0, 'Query 1', 'SELECT 1;');
+             INSERT INTO closed_tabs VALUES (0, 'Query 2', 'SELECT 2;');
+             PRAGMA user_version = 2;",
+        )
+        .unwrap();
+    drop(connection);
+    let loaded = SqliteStore::open(&path).unwrap().load().unwrap();
+    assert_eq!(loaded.tabs, [draft(5, "Query 1", "SELECT 1;")]);
+    assert_eq!(loaded.closed, [draft(0, "Query 2", "SELECT 2;")]);
+}
+
 #[test]
 fn keeps_the_first_tab_active() {
     let store = SqliteStore::in_memory().unwrap();
```

- [ ] **Step 2: Run them and watch them fail**

Run: `cargo nextest run -p quarry-store`
Expected: FAIL to compile: `struct Draft has no field named challenge`.

- [ ] **Step 3: Implement**

Modify `crates/app/src/model/closed_tabs.rs`:
```diff
@@ -43,6 +43,7 @@ mod tests {
             id: 0,
             title: title.to_owned(),
             text: text.to_owned(),
+            challenge: None,
         }
     }
 
```

Modify `crates/app/src/views/workspace/drafts.rs`:
```diff
@@ -77,6 +77,7 @@ impl Workspace {
                     id: tab.id,
                     title: tab.title.clone(),
                     text: self.views.get(&tab.id)?.read(cx).text(cx),
+                    challenge: None,
                 })
             })
             .collect();
```

Modify `crates/app/src/views/workspace/tabs.rs`:
```diff
@@ -67,6 +67,7 @@ impl Workspace {
             id,
             title: tab.title.clone(),
             text: view.read(cx).text(cx),
+            challenge: None,
         };
         closed_tabs::remember(&mut self.closed, draft);
     }
```

Modify `crates/store/src/drafts.rs`:
```diff
@@ -11,6 +11,9 @@ pub struct Draft {
     pub title: String,
     /// The editor text.
     pub text: String,
+    /// The full id of the challenge the tab plays, such as
+    /// `vinyl-vault/first-spin`; `None` for a plain query tab.
+    pub challenge: Option<String>,
 }
 
 /// Every open tab in order, which one is active, and the recently closed
@@ -68,6 +71,7 @@ mod tests {
                 id: 1,
                 title: "Query 1".to_owned(),
                 text: "SELECT 1".to_owned(),
+                challenge: None,
             }],
             active: 0,
             closed: Vec::new(),
```

Modify `crates/store/src/migrations.rs`:
```diff
@@ -18,10 +18,12 @@ const STEPS: &[&str] = &[
         title TEXT NOT NULL,
         text TEXT NOT NULL
     );",
+    "ALTER TABLE drafts ADD COLUMN challenge TEXT;
+    ALTER TABLE closed_tabs ADD COLUMN challenge TEXT;",
 ];
 
 /// The schema version this build writes.
-pub const VERSION: i64 = 2;
+pub const VERSION: i64 = 3;
 
 /// Brings the schema up to [`VERSION`] in one transaction.
 ///
@@ -91,7 +93,7 @@ mod tests {
             )
             .unwrap();
         migrate(&mut connection).unwrap();
-        assert_eq!(version(&connection), 2);
+        assert_eq!(version(&connection), VERSION);
         let kept: String = connection
             .query_row("SELECT text FROM drafts WHERE id = 4", [], |row| row.get(0))
             .unwrap();
```

Modify `crates/store/src/sqlite.rs`:
```diff
@@ -66,12 +66,13 @@ fn index(position: usize) -> i64 {
 /// The open tabs, left to right.
 fn load_tabs(connection: &Connection) -> Result<Vec<Draft>, StoreError> {
     let mut statement =
-        connection.prepare("SELECT id, title, text FROM drafts ORDER BY position")?;
+        connection.prepare("SELECT id, title, text, challenge FROM drafts ORDER BY position")?;
     let tabs = statement.query_map([], |row| {
         Ok(Draft {
             id: row.get(0)?,
             title: row.get(1)?,
             text: row.get(2)?,
+            challenge: row.get(3)?,
         })
     })?;
     Ok(tabs.collect::<Result<Vec<_>, _>>()?)
@@ -80,12 +81,13 @@ fn load_tabs(connection: &Connection) -> Result<Vec<Draft>, StoreError> {
 /// The recently closed tabs, newest first, each with id 0.
 fn load_closed(connection: &Connection) -> Result<Vec<Draft>, StoreError> {
     let mut statement =
-        connection.prepare("SELECT title, text FROM closed_tabs ORDER BY position")?;
+        connection.prepare("SELECT title, text, challenge FROM closed_tabs ORDER BY position")?;
     let closed = statement.query_map([], |row| {
         Ok(Draft {
             id: 0,
             title: row.get(0)?,
             text: row.get(1)?,
+            challenge: row.get(2)?,
         })
     })?;
     Ok(closed.collect::<Result<Vec<_>, _>>()?)
@@ -106,8 +108,15 @@ fn save_tabs(connection: &Connection, tabs: &[Draft]) -> Result<(), StoreError>
     connection.execute("DELETE FROM drafts", [])?;
     for (position, draft) in tabs.iter().enumerate() {
         connection.execute(
-            "INSERT INTO drafts (id, position, title, text) VALUES (?1, ?2, ?3, ?4)",
-            params![draft.id, index(position), draft.title, draft.text],
+            "INSERT INTO drafts (id, position, title, text, challenge)
+             VALUES (?1, ?2, ?3, ?4, ?5)",
+            params![
+                draft.id,
+                index(position),
+                draft.title,
+                draft.text,
+                draft.challenge
+            ],
         )?;
     }
     Ok(())
@@ -118,8 +127,9 @@ fn save_closed(connection: &Connection, closed: &[Draft]) -> Result<(), StoreErr
     connection.execute("DELETE FROM closed_tabs", [])?;
     for (position, draft) in closed.iter().enumerate() {
         connection.execute(
-            "INSERT INTO closed_tabs (position, title, text) VALUES (?1, ?2, ?3)",
-            params![index(position), draft.title, draft.text],
+            "INSERT INTO closed_tabs (position, title, text, challenge)
+             VALUES (?1, ?2, ?3, ?4)",
+            params![index(position), draft.title, draft.text, draft.challenge],
         )?;
     }
     Ok(())
```

- [ ] **Step 4: Run the tests and every gate**

Run: `cargo nextest run --workspace`
Expected: every test passes, including `challenge_ids_round_trip_through_tabs_and_closed_tabs` and `a_version_two_file_is_upgraded_and_its_tabs_are_not_challenges`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 5: Commit**

```bash
git add crates/app/src/model/closed_tabs.rs crates/app/src/views/workspace/drafts.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/app/tabs.rs crates/store/src/drafts.rs crates/store/src/migrations.rs crates/store/src/sqlite.rs crates/store/tests/drafts.rs
git commit -m "feat(store): keep each tab's challenge in schema v3"
```

---

### Task 7: App: a sandbox per challenge tab

The app learns about tracks. `Tracks` holds the catalog and builds each template on first use. The workspace keeps a sandbox (a challenge id and a session on a fresh copy) per challenge tab, and sends runs, Explain and cancel to the active tab's session. The schema tree is re-read whenever the tabs change, and the status bar's database label follows the active tab. `Workspace::open_challenge` focuses a challenge's tab or opens one holding its skeleton.

**Files:**
- Modify: `Cargo.lock`
- Modify: `crates/app/Cargo.toml`
- Modify: `crates/app/src/lib.rs`
- Create: `crates/app/src/tracks.rs`
- Create: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/src/views/workspace/runs.rs`
- Modify: `crates/app/src/views/workspace/tabs.rs`
- Create: `crates/app/tests/app/challenges.rs`
- Modify: `crates/app/tests/app/main.rs`

**Interfaces:**
- Consumes: `Catalog`, `ChallengeId`, `Template` (Tasks 2–4), the content (Task 5).
- Produces:
  - `quarry::tracks::Tracks::{new, embedded, catalog, challenge(&ChallengeId), template(&str) -> Result<Arc<Template>, EngineError>, sandbox(&ChallengeId) -> Result<Session, EngineError>}`.
  - `Workspace::{tracks(), open_challenge(&ChallengeId, window, cx), challenge(TabId) -> Option<&ChallengeId>, active_challenge(), database() -> String}`; private `open_sandbox_tab`, `challenge_tab`, `session`, `drop_sandbox`; `tabs_changed` is now `pub(super)` and re-reads the schema.
  - `tests/app/challenges.rs::open_challenge`, a test helper later tasks use.

- [ ] **Step 1: Implement**

Modify `Cargo.lock`:
```diff
@@ -5604,6 +5604,7 @@ version = "0.0.0"
 dependencies = [
  "async-channel",
  "gpui-kit",
+ "quarry-challenges",
  "quarry-engine",
  "quarry-store",
  "quarry-ui-kit",
```

Modify `crates/app/Cargo.toml`:
```diff
@@ -16,6 +16,7 @@ path = "src/main.rs"
 [dependencies]
 async-channel.workspace = true
 gpui-kit = { workspace = true, features = ["assets"] }
+quarry-challenges = { path = "../challenges" }
 quarry-engine = { path = "../engine" }
 quarry-store = { path = "../store" }
 quarry-ui-kit = { path = "../ui-kit" }
```

Modify `crates/app/src/lib.rs`:
```diff
@@ -1,9 +1,11 @@
 //! Quarry: a native macOS SQL client for learning and real work.
 //!
 //! [`model`] holds the GPUI-free view models; [`views`] renders them;
-//! [`shell`] wires actions, key bindings, menus and the window.
+//! [`shell`] wires actions, key bindings, menus and the window; [`tracks`]
+//! holds the challenge tracks and their sandboxes.
 
 pub mod model;
 pub mod sample;
 pub mod shell;
+pub mod tracks;
 pub mod views;
```

Create `crates/app/src/tracks.rs`:
```rust
//! The challenge tracks and their sandboxes.
//!
//! Each track's template database is built the first time one of its
//! challenges is opened, then kept; every challenge tab runs on a fresh
//! copy of it.

use std::{collections::BTreeMap, sync::Arc};

use quarry_challenges::{
    catalog::{Catalog, ChallengeId},
    sandbox::Template,
    track::{Challenge, Track},
};
use quarry_engine::{error::EngineError, session::Session};

/// The tracks embedded in Quarry, and the templates built so far.
pub struct Tracks {
    catalog: Catalog,
    templates: BTreeMap<String, Arc<Template>>,
}

impl Tracks {
    /// The tracks in `catalog`.
    pub fn new(catalog: Catalog) -> Self {
        Self {
            catalog,
            templates: BTreeMap::new(),
        }
    }

    /// The tracks embedded in the binary. A track with bad content is left
    /// out; the content tests keep that from shipping.
    pub fn embedded() -> Self {
        Self::new(Catalog::embedded().0)
    }

    /// Every track, in order.
    pub fn catalog(&self) -> &Catalog {
        &self.catalog
    }

    /// Challenge `id` and its track.
    pub fn challenge(&self, id: &ChallengeId) -> Option<(&Track, &Challenge)> {
        self.catalog.challenge(id)
    }

    /// The template of the track with slug `slug`, built now if this is
    /// its first use.
    ///
    /// # Errors
    /// Returns the engine's error when there is no such track or its
    /// database can't be built.
    pub fn template(&mut self, slug: &str) -> Result<Arc<Template>, EngineError> {
        if let Some(template) = self.templates.get(slug) {
            return Ok(Arc::clone(template));
        }
        let track = self.catalog.track(slug).ok_or_else(|| EngineError::Open {
            target: slug.to_owned(),
            message: "there is no such track".to_owned(),
        })?;
        let template = Arc::new(Template::new(&track.schema, &track.seed)?);
        self.templates
            .insert(slug.to_owned(), Arc::clone(&template));
        Ok(template)
    }

    /// A session on a fresh copy of the template of `id`'s track.
    ///
    /// # Errors
    /// As for [`Tracks::template`], or when the copy can't be made.
    pub fn sandbox(&mut self, id: &ChallengeId) -> Result<Session, EngineError> {
        let copy = self.template(id.track())?.fresh()?;
        Ok(Session::start(Box::new(copy)))
    }
}
```

Create `crates/app/src/views/workspace/challenges.rs`:
```rust
//! Challenge tabs: each plays one challenge on its own sandbox, a session
//! on a fresh copy of its track's database. Plain tabs share the sample
//! database.

use std::collections::BTreeMap;

use gpui_kit::{Context, Window};
use quarry_challenges::catalog::ChallengeId;
use quarry_engine::session::Session;

use super::Workspace;
use crate::{model::tabs::TabId, tracks::Tracks};

/// The tracks, and the sandbox of every open challenge tab.
pub(super) struct Challenges {
    tracks: Tracks,
    sandboxes: BTreeMap<TabId, Sandbox>,
}

impl Challenges {
    /// The embedded tracks, with no challenge tab open.
    pub(super) fn embedded() -> Self {
        Self {
            tracks: Tracks::embedded(),
            sandboxes: BTreeMap::new(),
        }
    }
}

/// What makes a tab a challenge tab.
struct Sandbox {
    challenge: ChallengeId,
    session: Session,
}

impl Workspace {
    /// The challenge tracks.
    pub fn tracks(&self) -> &Tracks {
        &self.challenges.tracks
    }

    /// Opens challenge `id`: activates its tab if one is open, or opens a
    /// new tab after the active one holding its skeleton. A challenge that
    /// doesn't exist does nothing.
    pub fn open_challenge(
        &mut self,
        id: &ChallengeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.challenge_tab(id) {
            self.activate_tab(tab, window, cx);
            return;
        }
        let Some((_, challenge)) = self.challenges.tracks.challenge(id) else {
            return;
        };
        let skeleton = challenge.skeleton.clone();
        if self.open_sandbox_tab(id, &skeleton, window, cx) {
            self.tabs_changed(window, cx);
        }
    }

    /// Opens a tab for challenge `id`, titled after it and holding `text`,
    /// after the active one, on a fresh sandbox. Returns `false`, opening
    /// nothing, when the challenge doesn't exist or its sandbox can't be
    /// made; the status bar then says why.
    pub(super) fn open_sandbox_tab(
        &mut self,
        id: &ChallengeId,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((_, challenge)) = self.challenges.tracks.challenge(id) else {
            return false;
        };
        let title = challenge.title.clone();
        match self.challenges.tracks.sandbox(id) {
            Ok(session) => {
                let tab = self.tabs.reopen(title);
                let challenge = id.clone();
                self.challenges
                    .sandboxes
                    .insert(tab, Sandbox { challenge, session });
                self.add_view(tab, text, window, cx);
                true
            }
            Err(error) => {
                self.show_note(format!("Couldn't open {title}: {error}"), cx);
                false
            }
        }
    }

    /// The open tab playing challenge `id`.
    fn challenge_tab(&self, id: &ChallengeId) -> Option<TabId> {
        self.challenges
            .sandboxes
            .iter()
            .find(|(_, sandbox)| &sandbox.challenge == id)
            .map(|(tab, _)| *tab)
    }

    /// The challenge tab `tab` plays, if it is a challenge tab.
    pub fn challenge(&self, tab: TabId) -> Option<&ChallengeId> {
        self.challenges
            .sandboxes
            .get(&tab)
            .map(|sandbox| &sandbox.challenge)
    }

    /// The challenge the active tab plays, if it is a challenge tab.
    pub fn active_challenge(&self) -> Option<&ChallengeId> {
        self.challenge(self.tabs.active_id())
    }

    /// The session the active tab runs on: its sandbox, or the sample
    /// database.
    pub(super) fn session(&self) -> &Session {
        self.challenges
            .sandboxes
            .get(&self.tabs.active_id())
            .map_or(&self.session, |sandbox| &sandbox.session)
    }

    /// What the status bar calls the active tab's database, such as
    /// "Vinyl Vault · sandbox".
    pub fn database(&self) -> String {
        let track = self
            .active_challenge()
            .and_then(|id| self.challenges.tracks.catalog().track(id.track()));
        track.map_or_else(
            || self.database.clone(),
            |track| format!("{} · sandbox", track.title),
        )
    }

    /// Forgets tab `tab`'s sandbox, ending its session.
    pub(super) fn drop_sandbox(&mut self, tab: TabId) {
        self.challenges.sandboxes.remove(&tab);
    }
}
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -106,7 +106,7 @@ impl Workspace {
         let summary = status::run_summary(output, output.elapsed(Instant::now()));
         let cursor = tab.editor().read(cx).cursor_position();
         let place = format::position(cursor.line, cursor.character);
-        let mut bar = StatusBar::new().left(format!("● {}", self.database));
+        let mut bar = StatusBar::new().left(format!("● {}", self.database()));
         if let Some(problem) = self.drafts_problem() {
             bar = bar.left(drafts_warning(problem));
         }
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -1,9 +1,11 @@
 //! The workspace: schema sidebar, query tabs, and the status bar.
 //!
 //! This module owns the workspace's state and its window commands;
-//! [`tabs`] opens, closes and switches tabs, [`drafts`] saves them,
-//! [`runs`] submits runs and reads the schema, and [`layout`] draws it.
+//! [`tabs`] opens, closes and switches tabs, [`challenges`] gives
+//! challenge tabs their sandboxes, [`drafts`] saves them, [`runs`] submits
+//! runs and reads the schema, and [`layout`] draws it.
 
+mod challenges;
 mod copy;
 mod drafts;
 mod layout;
@@ -27,6 +29,7 @@ use quarry_engine::session::Session;
 use quarry_store::drafts::{Draft, DraftStore, Drafts};
 use quarry_ui_kit::Aurora;
 
+use self::challenges::Challenges;
 pub use self::{
     copy::NOTE_TIME,
     drafts::{AUTOSAVE, SavedTabs, save_in_order},
@@ -55,12 +58,14 @@ pub const SIDEBAR_WIDTH: f32 = 240.0;
 
 /// The main window's content.
 ///
-/// Every tab in `tabs` has a view in `views`.
+/// Every tab in `tabs` has a view in `views`; challenge tabs also have a
+/// sandbox in `challenges`.
 pub struct Workspace {
     session: Session,
     database: String,
     tabs: TabList,
     views: BTreeMap<TabId, Entity<QueryTab>>,
+    challenges: Challenges,
     closed: Vec<Draft>,
     store: Arc<dyn DraftStore>,
     open_problem: Option<String>,
@@ -96,6 +101,7 @@ impl Workspace {
             database: database.to_owned(),
             tabs: restored_tabs(&drafts),
             views: BTreeMap::new(),
+            challenges: Challenges::embedded(),
             closed: drafts.closed.clone(),
             store,
             open_problem: problem,
```

Modify `crates/app/src/views/workspace/runs.rs`:
```diff
@@ -1,4 +1,4 @@
-//! Submits the active tab's runs and plans to the session, and reads the
+//! Submits the active tab's runs and plans to its session, and reads its
 //! schema.
 
 use gpui_kit::{Context, Window};
@@ -24,7 +24,7 @@ impl Workspace {
         let Some(sql) = tab.read(cx).target_sql(cx) else {
             return;
         };
-        let handle = self.session.run(sql.clone());
+        let handle = self.session().run(sql.clone());
         tab.update(cx, |tab, cx| {
             tab.start(sql, RunScope::Statement, handle, cx);
         });
@@ -38,7 +38,7 @@ impl Workspace {
         if splitter::split(&script).is_empty() {
             return;
         }
-        let handle = self.session.run(script.clone());
+        let handle = self.session().run(script.clone());
         tab.update(cx, |tab, cx| {
             tab.start(script, RunScope::Script, handle, cx);
         });
@@ -51,7 +51,7 @@ impl Workspace {
         let Some(sql) = tab.read(cx).target_sql(cx) else {
             return;
         };
-        let reply = self.session.plan(sql);
+        let reply = self.session().plan(sql);
         tab.update(cx, |tab, cx| tab.explain(reply, cx));
     }
 
@@ -59,9 +59,10 @@ impl Workspace {
         self.active_tab().clone().update(cx, QueryTab::cancel);
     }
 
-    /// Reads the schema after any queued runs and shows it in the sidebar.
+    /// Reads the active tab's schema after any queued runs and shows it in
+    /// the sidebar.
     pub(super) fn read_schema(&mut self, cx: &mut Context<Self>) {
-        let reply = self.session.schema();
+        let reply = self.session().schema();
         let tree = self.schema.clone();
         self.schema_read = Some(cx.spawn(async move |_, cx| {
             let schema = poll_reply(reply, cx.background_executor().clone()).await;
```

Modify `crates/app/src/views/workspace/tabs.rs`:
```diff
@@ -46,6 +46,7 @@ impl Workspace {
         let closing = self.tabs.active_id();
         self.remember(closing, cx);
         let opened = self.tabs.close(closing);
+        self.drop_sandbox(closing);
         let closed = self.views.remove(&closing);
         // A run that is cut off never emits `RunEnded`, so read the schema for it.
         if closed.is_some_and(|tab| tab.read(cx).output(cx).is_running()) {
@@ -111,9 +112,11 @@ impl Workspace {
         }
     }
 
-    /// Saves, focuses the active tab's editor, and redraws.
-    fn tabs_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
+    /// Saves, shows the active tab's schema, focuses its editor, and
+    /// redraws.
+    pub(super) fn tabs_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
         self.save_now(cx);
+        self.read_schema(cx);
         self.editor(cx)
             .update(cx, |state, cx| state.focus(window, cx));
         cx.notify();
```

Create `crates/app/tests/app/challenges.rs`:
```rust
//! Challenge tabs: one per challenge, each on its own sandbox, with the
//! sidebar and the status bar following the active tab.

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use quarry::{model::statements::Phase, sample, views::workspace::Workspace};
use quarry_challenges::catalog::ChallengeId;
use quarry_engine::value::Value;

use super::support::{open, phase, run, settle, write};

fn first_spin() -> ChallengeId {
    ChallengeId::new("vinyl-vault", "first-spin")
}

fn every_genre() -> ChallengeId {
    ChallengeId::new("vinyl-vault", "every-genre")
}

/// Opens challenge `id` in `workspace`.
pub fn open_challenge(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, id: &ChallengeId) {
    cx.update(|window, cx| {
        workspace.update(cx, |workspace, cx| workspace.open_challenge(id, window, cx));
    });
}

fn titles(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| {
        let tabs = workspace.read(cx).tabs().tabs();
        tabs.iter().map(|tab| tab.title.clone()).collect()
    })
}

fn text(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| workspace.read(cx).editor(cx).read(cx).value().to_string())
}

/// The first cell of the active tab's first result.
fn first_cell(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<Value> {
    cx.update(|_, cx| {
        let output = workspace.read(cx).run_output(cx);
        output.statement(0)?.rows().first()?.first().cloned()
    })
}

/// Whether the sidebar lists table `name`, once the schema has been read.
fn sidebar_lists(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, name: &str) -> bool {
    settle(cx, |_, cx| {
        let tree = workspace.read(cx).schema().read(cx);
        tree.tables().iter().any(|table| table.name == name)
    })
}

#[gpui_kit::test]
fn a_challenge_opens_in_a_tab_titled_after_it_with_its_skeleton(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    assert_eq!(titles(&workspace, cx), ["Query 1", "First spin"]);
    assert_eq!(text(&workspace, cx), "SELECT ___, ___\nFROM ___;");
    let active = cx.update(|_, cx| workspace.read(cx).active_challenge().cloned());
    assert_eq!(active, Some(first_spin()));
}

#[gpui_kit::test]
fn opening_a_challenge_again_returns_to_its_tab(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    cx.simulate_keystrokes("cmd-{");
    open_challenge(&workspace, cx, &first_spin());
    assert_eq!(titles(&workspace, cx), ["Query 1", "First spin"]);
    assert_eq!(cx.update(|_, cx| workspace.read(cx).tabs().active()), 1);
}

#[gpui_kit::test]
fn an_unknown_challenge_opens_nothing(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &ChallengeId::new("vinyl-vault", "b-side"));
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
}

#[gpui_kit::test]
fn a_challenge_tab_runs_on_its_track_and_a_plain_tab_on_the_sample(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    write(&workspace, cx, "SELECT count(*) FROM albums");
    run(&workspace, cx);
    assert_eq!(first_cell(&workspace, cx), Some(Value::Int(18)));
    cx.simulate_keystrokes("cmd-{");
    write(&workspace, cx, "SELECT count(*) FROM albums");
    run(&workspace, cx);
    assert!(matches!(phase(&workspace, cx), Phase::Failed { .. }));
}

#[gpui_kit::test]
fn each_challenge_tab_has_its_own_copy(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    write(&workspace, cx, "DELETE FROM albums");
    run(&workspace, cx);
    open_challenge(&workspace, cx, &every_genre());
    write(&workspace, cx, "SELECT count(*) FROM albums");
    run(&workspace, cx);
    assert_eq!(first_cell(&workspace, cx), Some(Value::Int(18)));
}

#[gpui_kit::test]
fn a_reopened_challenge_starts_on_a_fresh_copy(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    write(&workspace, cx, "DELETE FROM albums");
    run(&workspace, cx);
    cx.simulate_keystrokes("cmd-w");
    open_challenge(&workspace, cx, &first_spin());
    write(&workspace, cx, "SELECT count(*) FROM albums");
    run(&workspace, cx);
    assert_eq!(first_cell(&workspace, cx), Some(Value::Int(18)));
}

#[gpui_kit::test]
fn the_sidebar_and_status_bar_follow_the_active_tab(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let database = |cx: &mut VisualTestContext| cx.update(|_, cx| workspace.read(cx).database());
    assert_eq!(database(cx), sample::NAME);
    open_challenge(&workspace, cx, &first_spin());
    assert_eq!(database(cx), "Vinyl Vault · sandbox");
    assert!(sidebar_lists(&workspace, cx, "albums"));
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(database(cx), sample::NAME);
    assert!(sidebar_lists(&workspace, cx, "astronauts"));
}
```

Modify `crates/app/tests/app/main.rs`:
```diff
@@ -2,6 +2,7 @@
 //!
 //! One test binary, so the helpers in [`support`] are shared.
 
+mod challenges;
 mod copy;
 mod palette;
 mod runs;
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run -p quarry --test app challenges`
Expected: 7 tests pass.

Run: `cargo nextest run --workspace`
Expected: every test passes.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add Cargo.lock crates/app/Cargo.toml crates/app/src/lib.rs crates/app/src/tracks.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/runs.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/app/challenges.rs crates/app/tests/app/main.rs
git commit -m "feat(app): give each challenge tab its own sandbox"
```

---

### Task 8: App: the Challenges palette source and challenge tabs across restarts

Challenges appear in ⌘K after Recently closed, each as its title with "Vinyl Vault · 1 of 5" beside it; picking one opens it. Challenge tabs keep their challenge everywhere a draft goes: the snapshot saves it, a restored tab gets a fresh sandbox (or stays plain when its challenge is gone), and ⌘W remembers it so reopening brings it back on a fresh sandbox.

**Files:**
- Create: `crates/app/src/views/palette/challenges.rs`
- Modify: `crates/app/src/views/palette/mod.rs`
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/drafts.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/src/views/workspace/tabs.rs`
- Modify: `crates/app/tests/app/challenges.rs`
- Modify: `crates/app/tests/app/palette.rs`

**Interfaces:**
- Consumes: Task 7's workspace, `PaletteSource` (Phase 1E), `Draft::challenge` (Task 6).
- Produces: the `challenges` palette source (title "Challenges", key `track/challenge`); `Workspace::restore_sandbox`; `challenge_tab` is `pub(super)`; `reopen_tab` reopens challenge tabs.

- [ ] **Step 1: Implement**

Create `crates/app/src/views/palette/challenges.rs`:
```rust
//! Every challenge of every track; picking one opens it.

use gpui_kit::{App, Context, Window};
use quarry_challenges::catalog::ChallengeId;

use super::PaletteSource;
use crate::{model::palette::PaletteItem, views::workspace::Workspace};

/// The challenges, track by track, in play order.
pub struct Challenges;

impl PaletteSource for Challenges {
    fn id(&self) -> &'static str {
        "challenges"
    }

    fn title(&self) -> &'static str {
        "Challenges"
    }

    fn items(&self, workspace: &Workspace, _: &App) -> Vec<PaletteItem> {
        let mut items = Vec::new();
        for track in workspace.tracks().catalog().tracks() {
            let count = track.challenges.len();
            for (index, challenge) in track.challenges.iter().enumerate() {
                items.push(PaletteItem {
                    source: self.id(),
                    key: ChallengeId::new(&track.slug, &challenge.id).to_string(),
                    label: challenge.title.clone(),
                    detail: format!("{} · {} of {count}", track.title, index + 1),
                });
            }
        }
        items
    }

    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        if let Some(id) = ChallengeId::parse(key) {
            workspace.open_challenge(&id, window, cx);
        }
    }
}
```

Modify `crates/app/src/views/palette/mod.rs`:
```diff
@@ -1,9 +1,11 @@
-//! The ⌘K palette: open tabs, recently closed tabs, tables and commands.
+//! The ⌘K palette: open tabs, recently closed tabs, challenges, tables and
+//! commands.
 //!
 //! Each kind of item comes from a [`PaletteSource`] in the
 //! [`PaletteSourceRegistry`]; adding a source is a new module and one line
 //! in [`PaletteSourceRegistry::standard`]. [`dialog`] draws the palette.
 
+mod challenges;
 mod closed;
 mod commands;
 pub mod dialog;
@@ -43,12 +45,14 @@ pub struct PaletteSourceRegistry {
 }
 
 impl PaletteSourceRegistry {
-    /// Open tabs, recently closed tabs, tables and views, then commands.
+    /// Open tabs, recently closed tabs, challenges, tables and views, then
+    /// commands.
     pub fn standard() -> Self {
         Self {
             sources: vec![
                 Box::new(tabs::OpenTabs),
                 Box::new(closed::RecentlyClosed),
+                Box::new(challenges::Challenges),
                 Box::new(tables::Tables),
                 Box::new(commands::Commands),
             ],
```

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -7,6 +7,7 @@ use std::collections::BTreeMap;
 use gpui_kit::{Context, Window};
 use quarry_challenges::catalog::ChallengeId;
 use quarry_engine::session::Session;
+use quarry_store::drafts::Draft;
 
 use super::Workspace;
 use crate::{model::tabs::TabId, tracks::Tracks};
@@ -64,7 +65,7 @@ impl Workspace {
     /// Opens a tab for challenge `id`, titled after it and holding `text`,
     /// after the active one, on a fresh sandbox. Returns `false`, opening
     /// nothing, when the challenge doesn't exist or its sandbox can't be
-    /// made; the status bar then says why.
+    /// made.
     pub(super) fn open_sandbox_tab(
         &mut self,
         id: &ChallengeId,
@@ -76,25 +77,50 @@ impl Workspace {
             return false;
         };
         let title = challenge.title.clone();
+        let Some(sandbox) = self.sandbox(id, &title, cx) else {
+            return false;
+        };
+        let tab = self.tabs.reopen(title);
+        self.challenges.sandboxes.insert(tab, sandbox);
+        self.add_view(tab, text, window, cx);
+        true
+    }
+
+    /// Makes the restored tab `draft` a challenge tab again, when it played
+    /// a challenge that still exists. Otherwise it stays a plain tab and
+    /// keeps its text.
+    pub(super) fn restore_sandbox(&mut self, draft: &Draft, cx: &mut Context<Self>) {
+        let id = draft.challenge.as_deref().and_then(ChallengeId::parse);
+        let Some(id) = id.filter(|id| self.challenges.tracks.challenge(id).is_some()) else {
+            return;
+        };
+        if let Some(sandbox) = self.sandbox(&id, &draft.title, cx) {
+            self.challenges.sandboxes.insert(draft.id, sandbox);
+        }
+    }
+
+    /// A fresh sandbox for challenge `id`. When it can't be made, the status
+    /// bar says why, naming the tab `title`.
+    fn sandbox(
+        &mut self,
+        id: &ChallengeId,
+        title: &str,
+        cx: &mut Context<Self>,
+    ) -> Option<Sandbox> {
         match self.challenges.tracks.sandbox(id) {
-            Ok(session) => {
-                let tab = self.tabs.reopen(title);
-                let challenge = id.clone();
-                self.challenges
-                    .sandboxes
-                    .insert(tab, Sandbox { challenge, session });
-                self.add_view(tab, text, window, cx);
-                true
-            }
+            Ok(session) => Some(Sandbox {
+                challenge: id.clone(),
+                session,
+            }),
             Err(error) => {
                 self.show_note(format!("Couldn't open {title}: {error}"), cx);
-                false
+                None
             }
         }
     }
 
     /// The open tab playing challenge `id`.
-    fn challenge_tab(&self, id: &ChallengeId) -> Option<TabId> {
+    pub(super) fn challenge_tab(&self, id: &ChallengeId) -> Option<TabId> {
         self.challenges
             .sandboxes
             .iter()
```

Modify `crates/app/src/views/workspace/drafts.rs`:
```diff
@@ -77,7 +77,7 @@ impl Workspace {
                     id: tab.id,
                     title: tab.title.clone(),
                     text: self.views.get(&tab.id)?.read(cx).text(cx),
-                    challenge: None,
+                    challenge: self.challenge(tab.id).map(ToString::to_string),
                 })
             })
             .collect();
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -129,14 +129,17 @@ impl Workspace {
         workspace
     }
 
-    /// Creates a view per tab, with its saved text; a fresh start shows the
-    /// welcome query.
+    /// Creates a view per tab, with its saved text, and a sandbox per saved
+    /// challenge tab; a fresh start shows the welcome query.
     fn add_views(&mut self, drafts: &Drafts, window: &mut Window, cx: &mut Context<Self>) {
         let ids: Vec<TabId> = self.tabs.tabs().iter().map(|tab| tab.id).collect();
         for id in ids {
             let saved = drafts.tabs.iter().find(|draft| draft.id == id);
             let text = saved.map_or(editor::WELCOME, |draft| draft.text.as_str());
             self.add_view(id, text, window, cx);
+            if let Some(draft) = saved {
+                self.restore_sandbox(draft, cx);
+            }
         }
     }
 
```

Modify `crates/app/src/views/workspace/tabs.rs`:
```diff
@@ -1,6 +1,7 @@
 //! The workspace's query tabs: opening, closing, reopening and switching.
 
 use gpui_kit::{AppContext, Context, Entity, Window};
+use quarry_challenges::catalog::ChallengeId;
 use quarry_store::drafts::Draft;
 
 use super::Workspace;
@@ -68,7 +69,7 @@ impl Workspace {
             id,
             title: tab.title.clone(),
             text: view.read(cx).text(cx),
-            challenge: None,
+            challenge: self.challenge(id).map(ToString::to_string),
         };
         closed_tabs::remember(&mut self.closed, draft);
     }
@@ -79,14 +80,25 @@ impl Workspace {
     }
 
     /// Reopens closed tab `index` as a new tab after the active one, with
-    /// its title and text, and saves at once. An index past the list does
-    /// nothing.
+    /// its title and text, and saves at once. A challenge tab reopens on a
+    /// fresh sandbox; when its challenge is gone or already open, it comes
+    /// back as a plain tab so its text isn't lost. An index past the list
+    /// does nothing.
     pub fn reopen_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
         let Some(draft) = closed_tabs::take(&mut self.closed, index) else {
             return;
         };
-        let id = self.tabs.reopen(draft.title);
-        self.add_view(id, &draft.text, window, cx);
+        let challenge = draft
+            .challenge
+            .as_deref()
+            .and_then(ChallengeId::parse)
+            .filter(|id| self.challenge_tab(id).is_none());
+        let reopened =
+            challenge.is_some_and(|id| self.open_sandbox_tab(&id, &draft.text, window, cx));
+        if !reopened {
+            let id = self.tabs.reopen(draft.title);
+            self.add_view(id, &draft.text, window, cx);
+        }
         self.tabs_changed(window, cx);
     }
 
```

Modify `crates/app/tests/app/challenges.rs`:
```diff
@@ -1,12 +1,19 @@
 //! Challenge tabs: one per challenge, each on its own sandbox, with the
 //! sidebar and the status bar following the active tab.
 
+use std::sync::Arc;
+
 use gpui_kit::{Entity, TestAppContext, VisualTestContext};
 use quarry::{model::statements::Phase, sample, views::workspace::Workspace};
 use quarry_challenges::catalog::ChallengeId;
 use quarry_engine::value::Value;
+use quarry_store::{
+    drafts::{Draft, DraftStore, Drafts},
+    error::StoreError,
+    sqlite::SqliteStore,
+};
 
-use super::support::{open, phase, run, settle, write};
+use super::support::{install, open, open_with, phase, run, settle, write};
 
 fn first_spin() -> ChallengeId {
     ChallengeId::new("vinyl-vault", "first-spin")
@@ -127,3 +134,125 @@ fn the_sidebar_and_status_bar_follow_the_active_tab(cx: &mut TestAppContext) {
     assert_eq!(database(cx), sample::NAME);
     assert!(sidebar_lists(&workspace, cx, "astronauts"));
 }
+
+fn draft(id: i64, title: &str, text: &str, challenge: Option<&str>) -> Draft {
+    Draft {
+        id,
+        title: title.to_owned(),
+        text: text.to_owned(),
+        challenge: challenge.map(str::to_owned),
+    }
+}
+
+/// A store holding `tabs`, the last one active.
+fn store_with(tabs: Vec<Draft>) -> Result<Arc<SqliteStore>, StoreError> {
+    let store = Arc::new(SqliteStore::in_memory()?);
+    let drafts = Drafts {
+        active: tabs.len() - 1,
+        tabs,
+        closed: Vec::new(),
+    };
+    store.save(&drafts)?;
+    Ok(store)
+}
+
+/// Reopens the newest closed tab.
+fn reopen_newest(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
+    cx.update(|window, cx| {
+        workspace.update(cx, |workspace, cx| workspace.reopen_tab(0, window, cx));
+    });
+}
+
+#[gpui_kit::test]
+fn a_challenge_tab_and_its_draft_survive_a_restart(cx: &mut TestAppContext) {
+    install(cx);
+    let store = store_with(vec![
+        draft(1, "Query 1", "SELECT 1", None),
+        draft(
+            2,
+            "First spin",
+            "SELECT title",
+            Some("vinyl-vault/first-spin"),
+        ),
+    ])
+    .expect("a saved store");
+    let (workspace, cx) = open_with(cx, store.clone()).expect("the sample database opens");
+    assert_eq!(text(&workspace, cx), "SELECT title");
+    let active = cx.update(|_, cx| workspace.read(cx).active_challenge().cloned());
+    assert_eq!(active, Some(first_spin()));
+    assert!(sidebar_lists(&workspace, cx, "albums"));
+    cx.simulate_keystrokes("cmd-t");
+    assert!(settle(cx, |_, _| true));
+    let saved = store.load().expect("the drafts load");
+    assert_eq!(
+        saved.tabs[1].challenge.as_deref(),
+        Some("vinyl-vault/first-spin")
+    );
+    assert_eq!(saved.tabs[2].challenge, None);
+}
+
+#[gpui_kit::test]
+fn a_tab_whose_challenge_is_gone_comes_back_plain_with_its_text(cx: &mut TestAppContext) {
+    install(cx);
+    let store = store_with(vec![draft(
+        4,
+        "B-side",
+        "SELECT 2",
+        Some("vinyl-vault/b-side"),
+    )])
+    .expect("a saved store");
+    let (workspace, cx) = open_with(cx, store).expect("the sample database opens");
+    assert_eq!(titles(&workspace, cx), ["B-side"]);
+    assert_eq!(text(&workspace, cx), "SELECT 2");
+    let state = cx.update(|_, cx| {
+        let workspace = workspace.read(cx);
+        (workspace.active_challenge().cloned(), workspace.database())
+    });
+    assert_eq!(state, (None, sample::NAME.to_owned()));
+}
+
+#[gpui_kit::test]
+fn a_closed_challenge_reopens_on_a_fresh_sandbox_with_its_text(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    open_challenge(&workspace, cx, &first_spin());
+    write(
+        &workspace,
+        cx,
+        "DELETE FROM albums; SELECT count(*) FROM albums",
+    );
+    cx.simulate_keystrokes("cmd-shift-r");
+    assert!(settle(cx, |_, cx| !workspace
+        .read(cx)
+        .run_output(cx)
+        .is_running()));
+    cx.simulate_keystrokes("cmd-w");
+    let closed = cx.update(|_, cx| workspace.read(cx).closed_tabs()[0].challenge.clone());
+    assert_eq!(closed.as_deref(), Some("vinyl-vault/first-spin"));
+    reopen_newest(&workspace, cx);
+    assert_eq!(
+        text(&workspace, cx),
+        "DELETE FROM albums; SELECT count(*) FROM albums"
+    );
+    write(&workspace, cx, "SELECT count(*) FROM albums");
+    run(&workspace, cx);
+    assert_eq!(first_cell(&workspace, cx), Some(Value::Int(18)));
+}
+
+#[gpui_kit::test]
+fn a_closed_challenge_that_is_open_again_reopens_as_a_plain_tab(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    open_challenge(&workspace, cx, &first_spin());
+    write(&workspace, cx, "SELECT 'older'");
+    cx.simulate_keystrokes("cmd-w");
+    open_challenge(&workspace, cx, &first_spin());
+    reopen_newest(&workspace, cx);
+    assert_eq!(
+        titles(&workspace, cx),
+        ["Query 1", "First spin", "First spin"]
+    );
+    assert_eq!(text(&workspace, cx), "SELECT 'older'");
+    assert_eq!(
+        cx.update(|_, cx| workspace.read(cx).active_challenge().cloned()),
+        None
+    );
+}
```

Modify `crates/app/tests/app/palette.rs`:
```diff
@@ -72,17 +72,20 @@ fn command_k_again_closes_the_palette(cx: &mut TestAppContext) {
 }
 
 #[gpui_kit::test]
-fn an_empty_query_lists_tabs_then_tables_then_commands(cx: &mut TestAppContext) {
+fn an_empty_query_lists_tabs_then_challenges_tables_and_commands(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
     cx.simulate_keystrokes("cmd-t");
     search(cx, "");
     let groups = groups(&workspace, cx);
     let headings: Vec<&str> = groups.iter().map(|(title, _)| title.as_str()).collect();
-    assert_eq!(headings, ["Open tabs", "Tables & views", "Commands"]);
+    assert_eq!(
+        headings,
+        ["Open tabs", "Challenges", "Tables & views", "Commands"]
+    );
     assert_eq!(groups[0].1, ["Query 1", "Query 2"]);
-    assert!(groups[1].1.contains(&"astronauts".to_owned()));
-    assert_eq!(groups[2].1[..3], ["New Tab", "Close Tab", "Copy as TSV"]);
-    assert!(!groups[2].1.contains(&"Quit Quarry".to_owned()));
+    assert!(groups[2].1.contains(&"astronauts".to_owned()));
+    assert_eq!(groups[3].1[..3], ["New Tab", "Close Tab", "Copy as TSV"]);
+    assert!(!groups[3].1.contains(&"Quit Quarry".to_owned()));
     let current = cx.update(|_, cx| {
         let palette = workspace.read(cx).palette();
         palette.map(|p| p.read(cx).groups()[0].items[1].detail.clone())
@@ -194,3 +197,40 @@ fn a_table_that_is_gone_focuses_nothing(cx: &mut TestAppContext) {
         assert!(schema.expanded().is_empty());
     });
 }
+
+#[gpui_kit::test]
+fn challenges_are_listed_in_play_order_with_their_track_and_place(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    search(cx, "");
+    let challenges = cx.update(|_, cx| {
+        let palette = workspace.read(cx).palette();
+        let groups = palette.map(|p| p.read(cx).groups()).unwrap_or_default();
+        groups
+            .into_iter()
+            .find(|group| group.title == "Challenges")
+            .map(|group| group.items)
+            .unwrap_or_default()
+    });
+    let shown: Vec<(&str, &str)> = challenges
+        .iter()
+        .map(|item| (item.label.as_str(), item.detail.as_str()))
+        .collect();
+    assert_eq!(shown[0], ("First spin", "Vinyl Vault · 1 of 5"));
+    assert_eq!(shown[4], ("Unknown origins", "Vinyl Vault · 5 of 5"));
+}
+
+#[gpui_kit::test]
+fn picking_a_challenge_opens_it(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    search(cx, "crate digging");
+    cx.simulate_keystrokes("enter");
+    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
+    let opened = cx.update(|_, cx| {
+        workspace
+            .read(cx)
+            .active_challenge()
+            .map(ToString::to_string)
+    });
+    assert_eq!(opened.as_deref(), Some("vinyl-vault/crate-digging"));
+    assert!(editor_focused(&workspace, cx));
+}
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run --workspace`
Expected: every test passes, including the restart and reopen tests in `tests/app/challenges.rs` and the challenge tests in `tests/app/palette.rs`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/views/palette/challenges.rs crates/app/src/views/palette/mod.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/drafts.rs crates/app/src/views/workspace/mod.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/app/challenges.rs crates/app/tests/app/palette.rs
git commit -m "feat(app): list challenges in the palette and keep challenge tabs across restarts"
```

---

### Task 9: App: `model::verdict`, the Check view and ⌘⇧↵

Checking an answer. `model::verdict` turns a check's outcome into a headline and a detail line. A challenge tab gets the Check view through `ResultViewRegistry::for_challenge`. ⌘⇧↵ (`CheckAnswer`, Query → Check Answer) grades the whole editor text on the background executor and switches to the Check view; in a plain tab it shows the status-bar note "Check answer works in challenge tabs".

**Files:**
- Modify: `crates/app/src/model/mod.rs`
- Create: `crates/app/src/model/verdict.rs`
- Create: `crates/app/src/model/verdict/tests.rs`
- Modify: `crates/app/src/shell/actions.rs`
- Create: `crates/app/src/views/query_tab/check.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Create: `crates/app/src/views/results/check.rs`
- Modify: `crates/app/src/views/results/mod.rs`
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Create: `crates/app/tests/app/check.rs`
- Modify: `crates/app/tests/app/main.rs`
- Modify: `crates/app/tests/shell.rs`

**Interfaces:**
- Consumes: `check::check` (Task 4), the workspace's sandboxes (Tasks 7–8).
- Produces:
  - `model::verdict::{CheckState::{Idle, Checking, Done(Result<Verdict, CheckError>)}, Tone::{Solved, Wrong, Failed}, Report { tone, headline, detail }, report(&Result<Verdict, CheckError>) -> Report}`.
  - `results::{CHECK, ResultViewRegistry::for_challenge()}`; `QueryTab::{play_challenge, check(Arc<Template>, Challenge, cx), check_state()}`.
  - `shell::actions::CheckAnswer`, bound to `cmd-shift-enter` in `Workspace` and `Workspace > Input`; the command table has 18 rows.
  - `views::workspace::NOT_A_CHALLENGE`.

- [ ] **Step 1: Implement**

Modify `crates/app/src/model/mod.rs`:
```diff
@@ -16,3 +16,4 @@ pub mod statements;
 pub mod status;
 pub mod tabs;
 pub mod target;
+pub mod verdict;
```

Create `crates/app/src/model/verdict.rs`:
```rust
//! What the Check view says about a tab's latest check.
//!
//! A wrong answer says what differs, never how to fix it, and never shows
//! the expected rows.

use quarry_challenges::{
    check::{CheckError, Verdict},
    rules::CheckFailure,
    run::LIMITS,
};

use super::format;

/// Where a challenge tab's latest check is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CheckState {
    /// Nothing has been checked yet.
    #[default]
    Idle,
    /// A check is running.
    Checking,
    /// The latest check finished.
    Done(Result<Verdict, CheckError>),
}

/// How a report reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The answer is right.
    Solved,
    /// The answer is wrong.
    Wrong,
    /// The answer couldn't be checked.
    Failed,
}

/// A finished check, in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// How it reads.
    pub tone: Tone,
    /// The first line, such as "Not quite yet."
    pub headline: String,
    /// What differs, or why it couldn't be checked.
    pub detail: Option<String>,
}

impl Report {
    fn new(tone: Tone, headline: &str, detail: Option<String>) -> Self {
        Self {
            tone,
            headline: headline.to_owned(),
            detail,
        }
    }
}

/// The report for a finished check.
pub fn report(outcome: &Result<Verdict, CheckError>) -> Report {
    match outcome {
        Ok(Verdict::Solved) => Report::new(Tone::Solved, "Solved.", None),
        Ok(Verdict::Wrong(failure)) => {
            Report::new(Tone::Wrong, "Not quite yet.", Some(difference(failure)))
        }
        Err(error) => unchecked(error),
    }
}

/// What differs, in one sentence.
fn difference(failure: &CheckFailure) -> String {
    match failure {
        CheckFailure::ColumnCount { expected, got } => {
            format!(
                "Expected {}, got {}.",
                plural(*expected, "column"),
                count(*got)
            )
        }
        CheckFailure::ColumnNames { expected, got } => format!(
            "Expected the columns {}, got {}.",
            expected.join(", "),
            got.join(", ")
        ),
        CheckFailure::RowCount { expected, got } => {
            format!(
                "Expected {}, got {}.",
                plural(*expected, "row"),
                count(*got)
            )
        }
        CheckFailure::Contents {
            missing,
            unexpected,
        } => contents(*missing, *unexpected),
        CheckFailure::Order { row } => {
            format!("The rows are right, but row {} is out of order.", row + 1)
        }
    }
}

/// "2 rows missing, 1 row unexpected."
fn contents(missing: usize, unexpected: usize) -> String {
    let mut parts = Vec::new();
    if missing > 0 {
        parts.push(format!("{} missing", plural(missing, "row")));
    }
    if unexpected > 0 {
        parts.push(format!("{} unexpected", plural(unexpected, "row")));
    }
    format!("{}.", parts.join(", "))
}

/// Why the answer couldn't be checked.
fn unchecked(error: &CheckError) -> Report {
    match error {
        CheckError::Sql(message) => {
            Report::new(Tone::Failed, "Your query failed.", Some(message.clone()))
        }
        CheckError::TimedOut => Report::new(
            Tone::Failed,
            &format!("Stopped after {} seconds.", LIMITS.time.as_secs()),
            None,
        ),
        CheckError::TooManyRows => Report::new(
            Tone::Failed,
            &format!("Returned more than {} rows.", count(LIMITS.rows)),
            None,
        ),
        CheckError::Content(_) => Report::new(
            Tone::Failed,
            "This challenge has a problem; your answer wasn't checked.",
            None,
        ),
        CheckError::Sandbox(error) => Report::new(
            Tone::Failed,
            "The challenge's database couldn't be copied.",
            Some(error.to_string()),
        ),
    }
}

/// `"1 column"`, `"2 columns"`.
fn plural(n: usize, noun: &str) -> String {
    let s = if n == 1 { "" } else { "s" };
    format!("{} {noun}{s}", count(n))
}

fn count(n: usize) -> String {
    format::count(u64::try_from(n).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests;
```

Create `crates/app/src/model/verdict/tests.rs`:
```rust
use quarry_challenges::{
    check::{CheckError, Verdict},
    rules::CheckFailure,
};
use quarry_engine::error::EngineError;

use super::{CheckState, Report, Tone, report};

fn wrong(failure: CheckFailure) -> Report {
    report(&Ok(Verdict::Wrong(failure)))
}

fn detail(failure: CheckFailure) -> Option<String> {
    wrong(failure).detail
}

#[test]
fn nothing_is_checked_at_first() {
    assert_eq!(CheckState::default(), CheckState::Idle);
}

#[test]
fn a_solved_answer_says_so() {
    assert_eq!(
        report(&Ok(Verdict::Solved)),
        Report {
            tone: Tone::Solved,
            headline: "Solved.".to_owned(),
            detail: None,
        }
    );
}

#[test]
fn a_wrong_answer_is_not_quite_yet() {
    let report = wrong(CheckFailure::RowCount {
        expected: 7,
        got: 8,
    });
    assert_eq!(report.tone, Tone::Wrong);
    assert_eq!(report.headline, "Not quite yet.");
}

#[test]
fn column_differences_are_described() {
    assert_eq!(
        detail(CheckFailure::ColumnCount {
            expected: 2,
            got: 3
        })
        .as_deref(),
        Some("Expected 2 columns, got 3.")
    );
    assert_eq!(
        detail(CheckFailure::ColumnCount {
            expected: 1,
            got: 2
        })
        .as_deref(),
        Some("Expected 1 column, got 2.")
    );
    let names = CheckFailure::ColumnNames {
        expected: vec!["title".to_owned(), "year".to_owned()],
        got: vec!["title".to_owned(), "released".to_owned()],
    };
    assert_eq!(
        detail(names).as_deref(),
        Some("Expected the columns title, year, got title, released.")
    );
}

#[test]
fn row_count_and_contents_are_described() {
    assert_eq!(
        detail(CheckFailure::RowCount {
            expected: 1,
            got: 12_000
        })
        .as_deref(),
        Some("Expected 1 row, got 12,000.")
    );
    let contents = |missing, unexpected| {
        detail(CheckFailure::Contents {
            missing,
            unexpected,
        })
    };
    assert_eq!(
        contents(2, 1).as_deref(),
        Some("2 rows missing, 1 row unexpected.")
    );
    assert_eq!(contents(1, 0).as_deref(), Some("1 row missing."));
    assert_eq!(contents(0, 3).as_deref(), Some("3 rows unexpected."));
}

#[test]
fn order_names_the_first_row_counting_from_one() {
    assert_eq!(
        detail(CheckFailure::Order { row: 2 }).as_deref(),
        Some("The rows are right, but row 3 is out of order.")
    );
}

#[test]
fn errors_say_why_nothing_was_checked() {
    let failed = report(&Err(CheckError::Sql("no such column: nope".to_owned())));
    assert_eq!(
        (
            failed.tone,
            failed.headline.as_str(),
            failed.detail.as_deref()
        ),
        (
            Tone::Failed,
            "Your query failed.",
            Some("no such column: nope")
        )
    );
    let headline = |error| report(&Err(error)).headline;
    assert_eq!(headline(CheckError::TimedOut), "Stopped after 5 seconds.");
    assert_eq!(
        headline(CheckError::TooManyRows),
        "Returned more than 10,000 rows."
    );
    let content = report(&Err(CheckError::Content("no such table".to_owned())));
    assert_eq!(
        (content.headline.as_str(), content.detail),
        (
            "This challenge has a problem; your answer wasn't checked.",
            None
        )
    );
}

#[test]
fn a_failed_copy_gives_the_engine_error() {
    let copy = report(&Err(CheckError::Sandbox(EngineError::Cancelled)));
    assert_eq!(copy.tone, Tone::Failed);
    assert_eq!(
        copy.headline,
        "The challenge's database couldn't be copied."
    );
    assert_eq!(copy.detail.as_deref(), Some("Query cancelled."));
}
```

Modify `crates/app/src/shell/actions.rs`:
```diff
@@ -15,6 +15,8 @@ actions!(
         RunAll,
         /// Shows how the database would run the current statement.
         Explain,
+        /// Checks the challenge tab's answer.
+        CheckAnswer,
         /// Cancels the running query.
         CancelRun,
         /// Opens a new query tab.
@@ -66,6 +68,8 @@ pub fn key_bindings() -> Vec<KeyBinding> {
         KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE)),
         KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
         KeyBinding::new("cmd-shift-r", RunAll, Some(WORKSPACE)),
+        KeyBinding::new("cmd-shift-enter", CheckAnswer, Some(WORKSPACE)),
+        KeyBinding::new("cmd-shift-enter", CheckAnswer, Some(WORKSPACE_EDITOR)),
         KeyBinding::new("cmd-e", Explain, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
@@ -114,6 +118,7 @@ pub fn commands() -> Vec<Command> {
         command("Copy as JSON", || Box::new(CopyJson), "Edit"),
         command("Run Statement", || Box::new(RunStatement), "Query"),
         command("Run All", || Box::new(RunAll), "Query"),
+        command("Check Answer", || Box::new(CheckAnswer), "Query"),
         command("Explain", || Box::new(Explain), "Query"),
         command("Cancel", || Box::new(CancelRun), "Query"),
         command("Open Palette", || Box::new(OpenPalette), "View"),
```

Create `crates/app/src/views/query_tab/check.rs`:
```rust
//! Checking a challenge tab's answer: the whole editor text is graded on
//! the background executor, and the Check view shows the outcome.

use std::sync::Arc;

use gpui_kit::{AppContext, Context};
use quarry_challenges::{check, sandbox::Template, track::Challenge};

use super::QueryTab;
use crate::{model::verdict::CheckState, views::results::CHECK};

impl QueryTab {
    /// Gives the tab the Check view, for a challenge tab.
    pub fn play_challenge(&mut self, cx: &mut Context<Self>) {
        self.views = super::ResultViewRegistry::for_challenge();
        cx.notify();
    }

    /// Grades the editor's text as an answer to `challenge`, on copies of
    /// `template`, and shows the Check view. A newer check replaces one
    /// still running.
    pub fn check(&mut self, template: Arc<Template>, challenge: Challenge, cx: &mut Context<Self>) {
        let sql = self.text(cx);
        self.check = CheckState::Checking;
        self.select_view(CHECK, cx);
        self.checking = Some(cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { check::check(&template, &challenge, &sql) })
                .await;
            let _ = this.update(cx, |tab, cx| {
                tab.check = CheckState::Done(outcome);
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Where the tab's latest check is.
    pub fn check_state(&self) -> &CheckState {
        &self.check
    }
}
```

Modify `crates/app/src/views/query_tab/mod.rs`:
```diff
@@ -4,6 +4,7 @@
 //! runs; a tab follows its run's events (see [`pump`]) and tells the
 //! workspace when the run ends.
 
+mod check;
 mod copy;
 mod explain;
 mod layout;
@@ -27,6 +28,7 @@ use crate::model::{
     plan_tree::PlanState,
     statements::{Phase, RunOutput},
     target,
+    verdict::CheckState,
 };
 
 /// What a tab tells the workspace.
@@ -60,6 +62,8 @@ pub struct QueryTab {
     pump: Option<Task<()>>,
     plan: PlanState,
     planning: Option<Task<()>>,
+    check: CheckState,
+    checking: Option<Task<()>>,
     _edits: Subscription,
 }
 
@@ -88,6 +92,8 @@ impl QueryTab {
             pump: None,
             plan: PlanState::Idle,
             planning: None,
+            check: CheckState::Idle,
+            checking: None,
             _edits: edits,
         }
     }
```

Create `crates/app/src/views/results/check.rs`:
```rust
//! The Check view: whether the latest check solved the challenge, and if
//! not, what differs. Phase 2B replaces it with the feedback card.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{space, text},
};

use super::{CHECK, ResultView};
use crate::{
    model::verdict::{self, CheckState, Tone},
    views::query_tab::QueryTab,
};

/// What the latest Check answer said.
pub struct CheckView;

impl ResultView for CheckView {
    fn id(&self) -> &'static str {
        CHECK
    }

    fn title(&self) -> &'static str {
        "Check"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let line = |message: String, role: Role| {
            div()
                .text_color(colors.get(role))
                .child(message)
                .into_any_element()
        };
        let body = match tab.check_state() {
            CheckState::Idle => vec![line(
                "Check your answer with ⌘⇧↵.".to_owned(),
                Role::TextMuted,
            )],
            CheckState::Checking => vec![line("Checking…".to_owned(), Role::TextMuted)],
            CheckState::Done(outcome) => {
                let report = verdict::report(outcome);
                let mut lines = vec![line(report.headline, role(report.tone))];
                lines.extend(
                    report
                        .detail
                        .map(|detail| line(detail, Role::TextSecondary)),
                );
                lines
            }
        };
        div()
            .id(CHECK)
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

/// The headline's color.
fn role(tone: Tone) -> Role {
    match tone {
        Tone::Solved => Role::Success,
        Tone::Wrong => Role::Warning,
        Tone::Failed => Role::Danger,
    }
}
```

Modify `crates/app/src/views/results/mod.rs`:
```diff
@@ -1,8 +1,10 @@
-//! The views under the editor: Results, Messages and Plan.
+//! The views under the editor: Results, Messages and Plan, and Check in a
+//! challenge tab.
 //!
 //! Each is a [`ResultView`] in the [`ResultViewRegistry`]; adding a view is
 //! a new module and one line in [`ResultViewRegistry::standard`].
 
+mod check;
 mod grid;
 mod messages;
 mod plan;
@@ -22,6 +24,8 @@ pub const RESULTS: &str = "results";
 pub const MESSAGES: &str = "messages";
 /// The id of the Plan view.
 pub const PLAN: &str = "plan";
+/// The id of the Check view.
+pub const CHECK: &str = "check";
 
 /// One view of a tab's run.
 pub trait ResultView {
@@ -53,6 +57,13 @@ impl ResultViewRegistry {
         }
     }
 
+    /// Results, Messages, Plan and Check, for a challenge tab.
+    pub fn for_challenge() -> Self {
+        let mut registry = Self::standard();
+        registry.views.push(Box::new(check::CheckView));
+        registry
+    }
+
     /// Every view, in switch order.
     pub fn views(&self) -> &[Box<dyn ResultView>] {
         &self.views
```

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -6,11 +6,17 @@ use std::collections::BTreeMap;
 
 use gpui_kit::{Context, Window};
 use quarry_challenges::catalog::ChallengeId;
+use quarry_challenges::track::Challenge;
 use quarry_engine::session::Session;
 use quarry_store::drafts::Draft;
 
 use super::Workspace;
-use crate::{model::tabs::TabId, tracks::Tracks};
+use crate::{
+    model::tabs::TabId, shell::actions::CheckAnswer, tracks::Tracks, views::query_tab::QueryTab,
+};
+
+/// What ⌘⇧↵ says in a tab that isn't a challenge tab.
+pub const NOT_A_CHALLENGE: &str = "Check answer works in challenge tabs";
 
 /// The tracks, and the sandbox of every open challenge tab.
 pub(super) struct Challenges {
@@ -81,8 +87,8 @@ impl Workspace {
             return false;
         };
         let tab = self.tabs.reopen(title);
-        self.challenges.sandboxes.insert(tab, sandbox);
         self.add_view(tab, text, window, cx);
+        self.attach(tab, sandbox, cx);
         true
     }
 
@@ -95,7 +101,7 @@ impl Workspace {
             return;
         };
         if let Some(sandbox) = self.sandbox(&id, &draft.title, cx) {
-            self.challenges.sandboxes.insert(draft.id, sandbox);
+            self.attach(draft.id, sandbox, cx);
         }
     }
 
@@ -119,6 +125,37 @@ impl Workspace {
         }
     }
 
+    /// Checks the active challenge tab's answer; anywhere else, the status
+    /// bar says checking needs a challenge tab.
+    pub(super) fn check_answer(&mut self, _: &CheckAnswer, _: &mut Window, cx: &mut Context<Self>) {
+        let Some((slug, challenge)) = self.active_challenge_details() else {
+            self.show_note(NOT_A_CHALLENGE.to_owned(), cx);
+            return;
+        };
+        match self.challenges.tracks.template(&slug) {
+            Ok(template) => {
+                let tab = self.active_tab().clone();
+                tab.update(cx, |tab, cx| tab.check(template, challenge, cx));
+            }
+            Err(error) => self.show_note(error.to_string(), cx),
+        }
+    }
+
+    /// The active challenge's track slug and challenge.
+    fn active_challenge_details(&self) -> Option<(String, Challenge)> {
+        let id = self.active_challenge()?;
+        let (track, challenge) = self.challenges.tracks.challenge(id)?;
+        Some((track.slug.clone(), challenge.clone()))
+    }
+
+    /// Makes `tab` a challenge tab on `sandbox`, with the Check view.
+    fn attach(&mut self, tab: TabId, sandbox: Sandbox, cx: &mut Context<Self>) {
+        self.challenges.sandboxes.insert(tab, sandbox);
+        if let Some(view) = self.views.get(&tab) {
+            view.update(cx, QueryTab::play_challenge);
+        }
+    }
+
     /// The open tab playing challenge `id`.
     pub(super) fn challenge_tab(&self, id: &ChallengeId) -> Option<TabId> {
         self.challenges
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -35,6 +35,7 @@ impl Render for Workspace {
             .on_action(cx.listener(Self::run_statement))
             .on_action(cx.listener(Self::run_all))
             .on_action(cx.listener(Self::explain))
+            .on_action(cx.listener(Self::check_answer))
             .on_action(cx.listener(Self::cancel))
             .on_action(cx.listener(Self::new_tab))
             .on_action(cx.listener(Self::close_tab))
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -31,6 +31,7 @@ use quarry_ui_kit::Aurora;
 
 use self::challenges::Challenges;
 pub use self::{
+    challenges::NOT_A_CHALLENGE,
     copy::NOTE_TIME,
     drafts::{AUTOSAVE, SavedTabs, save_in_order},
     tab_strip::tab_element_id,
```

Create `crates/app/tests/app/check.rs`:
```rust
//! Check answer (⌘⇧↵): grading a challenge tab's answer and showing it in
//! the Check view.

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use quarry::{
    model::verdict::CheckState,
    views::{
        results::{self, CHECK},
        workspace::{NOT_A_CHALLENGE, Workspace},
    },
};
use quarry_challenges::{
    catalog::ChallengeId,
    check::{CheckError, Verdict},
    rules::CheckFailure,
};

use super::{
    challenges::open_challenge,
    support::{drawn, open, run, settle, write},
};

fn state(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> CheckState {
    cx.update(|_, cx| {
        workspace
            .read(cx)
            .active_tab()
            .read(cx)
            .check_state()
            .clone()
    })
}

/// Writes `sql` in a new First spin tab, checks it with ⌘⇧↵, and waits.
fn check_first_spin(
    workspace: &Entity<Workspace>,
    cx: &mut VisualTestContext,
    sql: &str,
) -> CheckState {
    open_challenge(
        workspace,
        cx,
        &ChallengeId::new("vinyl-vault", "first-spin"),
    );
    write(workspace, cx, sql);
    cx.simulate_keystrokes("cmd-shift-enter");
    assert!(settle(cx, |_, cx| {
        let tab = workspace.read(cx).active_tab().read(cx);
        matches!(tab.check_state(), CheckState::Done(_))
    }));
    state(workspace, cx)
}

#[gpui_kit::test]
fn the_solution_is_solved_and_shown_in_the_check_view(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let checked = check_first_spin(&workspace, cx, "SELECT title, year FROM albums;");
    assert_eq!(checked, CheckState::Done(Ok(Verdict::Solved)));
    let view = cx.update(|_, cx| workspace.read(cx).active_tab().read(cx).view_id());
    assert_eq!(view, CHECK);
    assert!(drawn(cx, CHECK));
}

#[gpui_kit::test]
fn a_wrong_answer_says_what_differs(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let checked = check_first_spin(&workspace, cx, "SELECT title FROM albums");
    let failure = CheckFailure::ColumnCount {
        expected: 2,
        got: 1,
    };
    assert_eq!(checked, CheckState::Done(Ok(Verdict::Wrong(failure))));
}

#[gpui_kit::test]
fn a_failing_answer_gives_the_database_message(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let checked = check_first_spin(&workspace, cx, "SELECT nope FROM albums");
    let error = CheckError::Sql("no such column: nope".to_owned());
    assert_eq!(checked, CheckState::Done(Err(error)));
}

#[gpui_kit::test]
fn changes_made_in_the_tab_never_reach_grading(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(
        &workspace,
        cx,
        &ChallengeId::new("vinyl-vault", "first-spin"),
    );
    write(&workspace, cx, "DELETE FROM albums");
    run(&workspace, cx);
    let checked = check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
    assert_eq!(checked, CheckState::Done(Ok(Verdict::Solved)));
}

#[gpui_kit::test]
fn only_challenge_tabs_have_the_check_view(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let ids = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            let tab = workspace.read(cx).active_tab().read(cx);
            tab.views()
                .views()
                .iter()
                .map(|view| view.id())
                .collect::<Vec<_>>()
        })
    };
    assert_eq!(
        ids(cx),
        [results::RESULTS, results::MESSAGES, results::PLAN]
    );
    open_challenge(
        &workspace,
        cx,
        &ChallengeId::new("vinyl-vault", "first-spin"),
    );
    assert_eq!(
        ids(cx),
        [results::RESULTS, results::MESSAGES, results::PLAN, CHECK]
    );
    assert_eq!(state(&workspace, cx), CheckState::Idle);
}

#[gpui_kit::test]
fn checking_a_plain_tab_says_it_needs_a_challenge(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-shift-enter");
    let note = cx.update(|_, cx| workspace.read(cx).note().map(str::to_owned));
    assert_eq!(note.as_deref(), Some(NOT_A_CHALLENGE));
    assert_eq!(state(&workspace, cx), CheckState::Idle);
}
```

Modify `crates/app/tests/app/main.rs`:
```diff
@@ -3,6 +3,7 @@
 //! One test binary, so the helpers in [`support`] are shared.
 
 mod challenges;
+mod check;
 mod copy;
 mod palette;
 mod runs;
```

Modify `crates/app/tests/shell.rs`:
```diff
@@ -42,7 +42,7 @@ fn every_command_but_the_other_copy_formats_has_a_key_binding() {
         .collect();
     let unbound = ["quarry::CopyCsv", "quarry::CopyJson"];
     let commands = actions::commands();
-    assert_eq!(commands.len(), 17);
+    assert_eq!(commands.len(), 18);
     for command in commands {
         let name = (command.action)().name();
         assert!(
@@ -71,6 +71,16 @@ fn the_menus_are_the_command_table_in_order() {
         .collect();
     assert_eq!(items, labels);
     assert_eq!(items[3..6], ["Copy as TSV", "Copy as CSV", "Copy as JSON"]);
+    assert_eq!(
+        items[6..11],
+        [
+            "Run Statement",
+            "Run All",
+            "Check Answer",
+            "Explain",
+            "Cancel"
+        ]
+    );
 }
 
 #[test]
```

- [ ] **Step 2: Run the tests and every gate**

Run: `cargo nextest run --workspace`
Expected: every test passes, including `tests/app/check.rs`, `model::verdict::tests` and the updated `tests/shell.rs`.

Run: `cargo mutants -p quarry -f crates/app/src/model/verdict.rs`
Expected: 0 missed. Then `rm -rf mutants.out mutants.out.old`.

Run: `just fmt lint quality typos complexity`
Expected: exit 0.

- [ ] **Step 3: Commit**

```bash
git add crates/app/src/model/mod.rs crates/app/src/model/verdict.rs crates/app/src/model/verdict/tests.rs crates/app/src/shell/actions.rs crates/app/src/views/query_tab/check.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/results/check.rs crates/app/src/views/results/mod.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/tests/app/check.rs crates/app/tests/app/main.rs crates/app/tests/shell.rs
git commit -m "feat(app): check a challenge tab's answer with ⌘⇧↵"
```

---

### Task 10: ADR 0012, the README, and a note in the parent spec

Record the decisions in ADR 0012 "Challenge core". Move the README's status to Phase 2A, and note in §5.2 of the parent spec that templates stay in memory until 2D.

**Files:**
- Modify: `README.md`
- Create: `docs/adr/0012-challenge-core.md`
- Modify: `docs/superpowers/specs/2026-11-04-quarry-design.md`

**Interfaces:**
- Consumes: everything above.
- Produces: documentation only.

- [ ] **Step 1: Implement**

Modify `README.md`:
```diff
@@ -2,7 +2,7 @@
 
 A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.
 
-> **Status:** Phase 1 complete. You can write and run SQL against a built-in sample database in query tabs that survive restarts, run whole scripts, explain a statement's plan, copy results as TSV, CSV or JSON, and jump to tabs, closed tabs, tables and commands with ⌘K. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
+> **Status:** Phase 2A. On top of Phase 1's query tabs, scripts, plans, copying and ⌘K palette, you can open Vinyl Vault's first five challenges from ⌘K, each in its own tab on a fresh copy of the track's database, and check your answer with ⌘⇧↵. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
 
 - Built-in SQLite playground with story-driven SQL challenge tracks
 - PostgreSQL and MySQL connections for everyday work
```

Create `docs/adr/0012-challenge-core.md`:
```markdown
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
```

Modify `docs/superpowers/specs/2026-11-04-quarry-design.md`:
```diff
@@ -198,6 +198,7 @@ Tracks are embedded in the binary at compile time. All data is **fictional and g
 - **Check answer** runs against a fresh in-memory copy of the template, loaded with SQLite's backup API.
 - **Scratch tabs** use a persistent working copy. "Reset track database" restores it from the template.
 - **Guardrails:** a 5-second timeout (via SQLite's progress handler, and cancellable) and a 10,000-row cap while grading.
+- Phase 2A keeps each track's template in memory, built on first use, because Query challenges never change data; the on-disk template, the working copy and the reset arrive with Mutation challenges in 2D (ADR 0012).
 
 ### 5.3 Challenge kinds (`ChallengeKind`)
 
```

- [ ] **Step 2: Run the tests and every gate**

Run: `just check`
Expected: exit 0 (fmt, lint, quality, deny, machete, typos, complexity, coverage).

Run: `git diff main > target/2a.diff && just mutants target/2a.diff`
Expected: the mutation gate passes with 0 missed. Then `rm -rf mutants.out mutants.out.old target/2a.diff`.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/adr/0012-challenge-core.md docs/superpowers/specs/2026-11-04-quarry-design.md
git commit -m "docs: record the challenge core (ADR 0012)"
```

---

## After the last task

- [ ] **Visual review.** Run `cargo run -p quarry` and check the following with the user:
  - Opening a challenge from ⌘K, and the Challenges group's details.
  - A challenge tab's schema tree and status-bar label, and switching back to a plain tab.
  - The Check view: solved, a wrong answer, a failing query and "Checking…", in the dark and light themes.
  - A challenge tab after quitting and relaunching.

  The spec makes this review a condition of the PR.
- [ ] **Open the PR** only after the review, using `.github/pull_request_template.md`. Merge only with the user's explicit go-ahead.

**Not covered by a test:**
- A newer check replacing one still running: the test executor runs tasks in order, so the older check always finishes first. The code drops the older task when a newer one starts.
- How the Check view looks.
