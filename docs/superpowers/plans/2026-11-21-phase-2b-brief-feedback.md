# Phase 2B: The Brief, Feedback and the Aurora Pulse — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Give challenges their face: a right-hand inspector with the challenge brief (⌘⌥I), a feedback card that lists where every grading rule stands and offers the next challenge, a short teal glow on a solve, and Prev/Next across the whole catalog. Attempts are kept for the session until 2C.

**Architecture:**
- **`quarry-challenges`:**
  - `rules::grade` reports every rule as passed, failed or not checked (`Grading`), and `Verdict` carries it.
  - `ChallengeKind::expected` and `check::expect` give the solution's column names and row count.
  - `Catalog::neighbours` and `Catalog::position` place a challenge in the catalog.
  - `run::{Stop, Runner}` let a newer check stop an older one.
- **`quarry-store`:** `Drafts::inspector_hidden`, a key in the `state` table (no migration).
- **`quarry-ui-kit`:** `Pulse`, `motion::PULSE` and `Role::Pulse`.
- **`app`:**
  - GPUI-free models (100% covered, mutation-tested): `model::attempts`, `model::brief`, and `model::verdict::card`.
  - `Workspace::new` takes `Databases`, so tests inject tracks. `Tracks` caches each challenge's expectation.
  - `views::inspector`: `InspectorRegistry` with `BriefView` and `EmptyView`, in a fourth column.
  - The Check view draws the feedback card. `PreviousChallenge`, `NextChallenge` and `ToggleInspector` are new actions.
  - On a solve, unless Reduce Motion is on, the workspace draws the pulse as an inset glow at the window's edges.
- **Status:** every task's code was built and gated in a scratch clone first, one commit per task. The tasks were then replayed in order from the spec commit, and each commit passed fmt, clippy, the quality checks, typos, complexity and the whole workspace test suite. The suite grew from 486 tests after Task 1 to 521 after Task 7. `just check` passed at the end, and mutation testing over the whole diff missed nothing. Each task's code below is taken from those commits.

**Tech Stack:** Rust 1.98.1, `gpui-kit` =0.9.1, `rusqlite` 0.41.1. No new dependencies.

**Spec:** `docs/superpowers/specs/2026-11-19-phase-2b-brief-feedback-design.md` (approved 2026-11-21). Parent: `docs/superpowers/specs/2026-11-04-quarry-design.md` §3.3, §4.2, §4.3 and §5.4. ADR 0013 (written in Task 8).

## Global Constraints

- **Branch and commits:** work on `phase-2b-brief-feedback`, which already holds the spec and this plan. Use Conventional Commits, with no trailers.
- **No completion or assistance of any kind:**
  - The editor gets no completion, hover, definition or code-action providers, no inline suggestions and no hints. The existing test keeps asserting it.
  - The palette navigates and runs commands; it never offers SQL.
  - A wrong answer says what is wrong, never how to fix it. Neither the brief nor the card ever shows expected rows, values or the solution; the brief shows only the expected column names and row count.
- **Extension points:** new kinds of things go through traits and registries: `ChallengeKind`/`KindRegistry`, `CheckRule` (the ordered pipeline), `PaletteSource`/`PaletteSourceRegistry`, `ResultViewRegistry` and, new here, `InspectorView`/`InspectorRegistry`. No `match` on a registry-owned enum outside its module. A new track is a folder and nothing else.
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
- **Polling and background work:** GPUI code never awaits a session channel. It polls every `POLL` (33 ms). Grading and expectations use GPUI's background executor, so tests stay deterministic.
- **Motion:** every animation respects Reduce Motion through `cx.reduce_motion()`.
- **Contrast:** every new color goes through a `Role` and is covered by `tests/contrast.rs`. 2B adds `Role::Pulse`, checked at 3:1 as a non-text color on every surface.
- **Content:** all data is fictional and written for Quarry, with no real people, artists or labels.
- **Exact versions:** `gpui-kit` =0.9.1, `rusqlite` 0.41.1 (the workspace dependency).
- **Headless testing:**
  - Tests find elements by id, so elements they look for call `.test_support()`.
  - `settle()` advances the clock by `POLL`, pumps the executor, and returns whether its condition held. Every caller asserts it.
  - Tests click elements with `window.click(id, cx)` from `gpui_kit::test::TestWindowExt`.
- **Every task ends with its gates passing.** Don't commit anything under `.superpowers/`.
- **Merging:** only with the user's explicit go-ahead, after the user's visual review.

## Where the plan departs from the spec

The prototype found these. Each is small, and all of them are in the code below.

1. **Spec tasks 1 and 2 are one task.** Both change `check.rs`, so Task 1 here covers the full report, expectations and neighbours, and the plan has eight tasks instead of nine.
2. **The pulse is a glow at the window's edges.** A wash over the whole window failed the contrast gate: in Dawn, body text already sits at 4.51:1. `Role::Pulse` is an opaque teal (`#5eead4` in Aurora, `#0c8a7f` in Dawn) drawn as an inset shadow (64 px blur, 60% alpha at the peak) and checked at 3:1 as a non-text color. The user chose this on 2026-11-21, and the spec was updated to match.
3. **A stop, not a canceller per check.** A check makes its own sandbox copies inside `challenges`, so the app can't reach their cancellers. `run::Stop` collects the canceller of every run it watches and cancels them all, and any run that starts after it was stopped. `run::Runner` pairs a stop with the `Limits` and replaces the `limits` argument of `ChallengeKind` and `check_within` (now `check_with`). A stopped run reports as timed out to a caller that no longer listens.
4. **`Workspace::new` takes `Databases`.** Adding the `Tracks` argument would have taken it past five arguments, so the sample session, its name and the tracks travel together.
5. **Two hooks on the extension points.** `CheckRule::applies` lets the pipeline mark Order as not checked when order doesn't matter. Every rule implements it, with no default, because a default `true` body leaves a mutant that can't be caught. `ChallengeKind::expected` gives the solution's result, so `check::expect` works for any kind and Mutation can define its own in 2D.
6. **The expectation is worked out when a challenge tab opens** (or is restored), rather than when its brief is first shown, which keeps the work out of rendering. It runs once per challenge per session.
7. **The saved flag is `inspector_hidden`,** not `inspector_open`, so that a store with no saved value, including the in-memory `Forgetful` store, shows the inspector by default.
8. **The card's details:**
   - "Not quite yet" and errors share an orange (`Danger`) "!" header; 2A's Check view used amber for a wrong answer.
   - The checklist's rule names live in `model::verdict`. A rule it doesn't know reads as "This check", so a new rule still gets a line.
   - With no neighbour, Prev or Next is a muted label that does nothing, because ui-kit's `Button` has no disabled state.
9. **One store test for coverage.** The `state` helpers' error paths pushed `store` regions just under 90%. A test that loads a file whose `inspector_hidden` value isn't a number covers one of them.

## File Structure

```
crates/challenges/src/
  rules/mod.rs, rules/order.rs                  RuleStatus, RuleOutcome, Grading, grade, CheckRule::applies
  check.rs (+ check/tests.rs)                   Verdict(Grading), Expectation, expect, check_with
  kind/{mod,query}.rs                           ChallengeKind::expected, Runner arguments
  run.rs (+ run/tests.rs)                       Stop, Runner
  catalog.rs                                    Neighbours, Position
crates/challenges/tests/content.rs              every challenge has an expectation
crates/store/src/{drafts.rs, sqlite.rs}         Drafts::inspector_hidden, state helpers
crates/ui-kit/src/{pulse.rs, palette.rs, tokens.rs, lib.rs}   Pulse, Role::Pulse, motion::PULSE
crates/ui-kit/tests/contrast.rs                 the pulse at 3:1
crates/app/src/
  model/attempts.rs                             Attempts, Tally
  model/brief.rs (+ brief/tests.rs)             Brief, Expected, brief
  model/verdict.rs (+ verdict/tests.rs)         Card, Line, Mark, Footer, Solve, card
  tracks.rs                                     the expectation cache
  shell/{actions.rs, window.rs}                 ToggleInspector, PreviousChallenge, NextChallenge; Databases
  views/inspector/{mod,brief,empty}.rs          InspectorRegistry, BriefView, EmptyView
  views/query_tab/{mod,check}.rs                Checked/NextChallenge events, Stop, Solve, Stay here
  views/results/check.rs                        the feedback card
  views/workspace/{mod,challenges,layout,tabs,drafts}.rs   inspector column, attempts, Prev/Next, the pulse
crates/app/tests/app/{brief.rs, check.rs, challenges.rs, support.rs}, crates/app/tests/shell.rs
docs/adr/0013-brief-feedback-and-pulse.md
```

---

### Task 1: Challenges: every rule's outcome, expectations and neighbours

The card lists where every rule stands, the brief shows the expected columns and row count, and Prev/Next walk the catalog. `rules::grade` replaces `first_failure` and reports each rule as passed, failed or not checked; `CheckRule::applies`, which every rule implements, lets Order say it wasn't checked when order doesn't matter. `Verdict` carries the `Grading`. `ChallengeKind::expected` gives the solution's result, which both grading and `check::expect` use. `Catalog` learns `neighbours` and `position`. The app's verdict model and tests change only to compile; its sentences are the same.

**Files:**
- Modify: `crates/app/src/model/verdict.rs`
- Modify: `crates/app/src/model/verdict/tests.rs`
- Modify: `crates/app/tests/app/check.rs`
- Modify: `crates/challenges/src/catalog.rs`
- Modify: `crates/challenges/src/check.rs`
- Modify: `crates/challenges/src/check/tests.rs`
- Modify: `crates/challenges/src/kind/mod.rs`
- Modify: `crates/challenges/src/kind/query.rs`
- Modify: `crates/challenges/src/rules/columns.rs`
- Modify: `crates/challenges/src/rules/contents.rs`
- Modify: `crates/challenges/src/rules/count.rs`
- Modify: `crates/challenges/src/rules/mod.rs`
- Modify: `crates/challenges/src/rules/order.rs`
- Modify: `crates/challenges/tests/content.rs`

**Interfaces:**
- Consumes: `rules::{CheckRule, Comparison, pipeline}`, `kind::{ChallengeKind, KindRegistry}`, `catalog::Catalog`.
- Produces: `rules::{RuleStatus, RuleOutcome, Grading, grade}`, `CheckRule::applies`, `Verdict::{Solved(Grading), Wrong(Grading)}` with `of`, `grading`, `failure` and `is_solved`; `ChallengeKind::expected`; `check::{Expectation, expect}`; `catalog::{Neighbours, Position}`, `Catalog::neighbours` and `Catalog::position`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/src/model/verdict/tests.rs`:
```diff
@@ -1,13 +1,17 @@
 use quarry_challenges::{
     check::{CheckError, Verdict},
-    rules::CheckFailure,
+    rules::{CheckFailure, Grading, RuleOutcome, RuleStatus},
 };
 use quarry_engine::error::EngineError;
 
 use super::{CheckState, Report, Tone, report};
 
 fn wrong(failure: CheckFailure) -> Report {
-    report(&Ok(Verdict::Wrong(failure)))
+    let outcome = RuleOutcome {
+        rule: "columns",
+        status: RuleStatus::Failed(failure),
+    };
+    report(&Ok(Verdict::of(Grading::new(vec![outcome]))))
 }
 
 fn detail(failure: CheckFailure) -> Option<String> {
@@ -22,7 +26,7 @@
 #[test]
 fn a_solved_answer_says_so() {
     assert_eq!(
-        report(&Ok(Verdict::Solved)),
+        report(&Ok(Verdict::of(Grading::new(Vec::new())))),
         Report {
             tone: Tone::Solved,
             headline: "Solved.".to_owned(),
```

Modify `crates/app/tests/app/check.rs`:
```diff
@@ -9,11 +9,7 @@
         workspace::{NOT_A_CHALLENGE, Workspace},
     },
 };
-use quarry_challenges::{
-    catalog::ChallengeId,
-    check::{CheckError, Verdict},
-    rules::CheckFailure,
-};
+use quarry_challenges::{catalog::ChallengeId, check::CheckError, rules::CheckFailure};
 
 use super::{
     challenges::open_challenge,
@@ -31,6 +27,11 @@
     })
 }
 
+/// Whether `state` is a finished check that solved the challenge.
+fn solved(state: &CheckState) -> bool {
+    matches!(state, CheckState::Done(Ok(verdict)) if verdict.is_solved())
+}
+
 /// Writes `sql` in a new First spin tab, checks it with ⌘⇧↵, and waits.
 fn check_first_spin(
     workspace: &Entity<Workspace>,
@@ -55,7 +56,7 @@
 fn the_solution_is_solved_and_shown_in_the_check_view(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
     let checked = check_first_spin(&workspace, cx, "SELECT title, year FROM albums;");
-    assert_eq!(checked, CheckState::Done(Ok(Verdict::Solved)));
+    assert!(solved(&checked));
     let view = cx.update(|_, cx| workspace.read(cx).active_tab().read(cx).view_id());
     assert_eq!(view, CHECK);
     assert!(drawn(cx, CHECK));
@@ -69,7 +70,9 @@
         expected: 2,
         got: 1,
     };
-    assert_eq!(checked, CheckState::Done(Ok(Verdict::Wrong(failure))));
+    assert!(
+        matches!(&checked, CheckState::Done(Ok(verdict)) if verdict.failure() == Some(&failure))
+    );
 }
 
 #[gpui_kit::test]
@@ -91,7 +94,7 @@
     write(&workspace, cx, "DELETE FROM albums");
     run(&workspace, cx);
     let checked = check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
-    assert_eq!(checked, CheckState::Done(Ok(Verdict::Solved)));
+    assert!(solved(&checked));
 }
 
 #[gpui_kit::test]
```

Modify `crates/challenges/src/check/tests.rs`:
```diff
@@ -2,7 +2,7 @@
 
 use quarry_engine::error::EngineError;
 
-use super::{CheckError, Verdict, check, check_within};
+use super::{CheckError, Expectation, Verdict, check, check_within, expect};
 use crate::{
     rules::CheckFailure,
     run::{LIMITS, Limits, RunError},
@@ -35,25 +35,24 @@
 fn a_matching_answer_is_solved() {
     let challenge = challenge("SELECT title FROM albums WHERE year < 1965", false);
     let answer = "select TITLE from albums where year = 1959";
-    assert_eq!(
-        check(&template().unwrap(), &challenge, answer),
-        Ok(Verdict::Solved)
-    );
+    let verdict = check(&template().unwrap(), &challenge, answer).unwrap();
+    assert!(verdict.is_solved());
+    assert_eq!(verdict.failure(), None);
+    assert!(matches!(verdict, Verdict::Solved(_)));
 }
 
 #[test]
 fn a_wrong_answer_reports_the_first_failure() {
     let ordered = challenge("SELECT title FROM albums ORDER BY year DESC", true);
     let answer = "SELECT title FROM albums ORDER BY year";
-    assert_eq!(
-        check(&template().unwrap(), &ordered, answer),
-        Ok(Verdict::Wrong(CheckFailure::Order { row: 0 }))
-    );
+    let wrong = check(&template().unwrap(), &ordered, answer).unwrap();
+    assert!(!wrong.is_solved());
+    assert!(matches!(wrong, Verdict::Wrong(_)));
+    assert_eq!(wrong.failure(), Some(&CheckFailure::Order { row: 0 }));
+    assert_eq!(wrong.grading().outcomes().len(), 4);
     let unordered = challenge("SELECT title FROM albums ORDER BY year DESC", false);
-    assert_eq!(
-        check(&template().unwrap(), &unordered, answer),
-        Ok(Verdict::Solved)
-    );
+    let verdict = check(&template().unwrap(), &unordered, answer).unwrap();
+    assert!(verdict.is_solved());
 }
 
 #[test]
@@ -94,3 +93,33 @@
         CheckError::TooManyRows
     );
 }
+
+#[test]
+fn the_expectation_is_the_solutions_columns_and_row_count() {
+    let challenge = challenge(
+        "SELECT title, year AS released FROM albums WHERE year > 1960",
+        false,
+    );
+    assert_eq!(
+        expect(&template().unwrap(), &challenge),
+        Ok(Expectation {
+            columns: vec!["title".to_owned(), "released".to_owned()],
+            rows: 1,
+        })
+    );
+}
+
+#[test]
+fn a_broken_solution_has_no_expectation() {
+    let broken = challenge("SELECT nope FROM albums", false);
+    assert_eq!(
+        expect(&template().unwrap(), &broken),
+        Err(CheckError::Content("no such column: nope".to_owned()))
+    );
+    let mut mutation = challenge("SELECT 1", false);
+    mutation.kind = "mutation".to_owned();
+    assert_eq!(
+        expect(&template().unwrap(), &mutation),
+        Err(CheckError::Content("unknown kind \"mutation\"".to_owned()))
+    );
+}
```

Modify `crates/challenges/tests/content.rs`:
```diff
@@ -3,7 +3,7 @@
 
 use quarry_challenges::{
     catalog::{Catalog, ChallengeId},
-    check::{CheckError, Verdict, check},
+    check::{CheckError, Verdict, check, expect},
     rules::CheckFailure,
     run::{LIMITS, run},
     sandbox::Template,
@@ -21,14 +21,15 @@
     Template::new(&track.schema, &track.seed)
 }
 
-/// Checks `sql` against challenge `id` of Vinyl Vault.
-fn vinyl(id: &str, sql: &str) -> Result<Verdict, CheckError> {
+/// What is wrong with `sql` as an answer to challenge `id` of Vinyl Vault;
+/// `None` when it solves it.
+fn vinyl(id: &str, sql: &str) -> Result<Option<CheckFailure>, CheckError> {
     let catalog = catalog();
     let Some((track, challenge)) = catalog.challenge(&ChallengeId::new("vinyl-vault", id)) else {
         return Err(CheckError::Content(format!("no challenge {id}")));
     };
     let template = template(track).map_err(CheckError::Sandbox)?;
-    check(&template, challenge, sql)
+    check(&template, challenge, sql).map(|verdict| verdict.failure().cloned())
 }
 
 #[test]
@@ -57,8 +58,8 @@
         for challenge in &track.challenges {
             let verdict = check(&template, challenge, &challenge.solution);
             assert_eq!(
-                verdict,
-                Ok(Verdict::Solved),
+                verdict.as_ref().map(Verdict::is_solved),
+                Ok(true),
                 "{}/{}",
                 track.slug,
                 challenge.id
@@ -77,6 +78,20 @@
     }
 }
 
+#[test]
+fn every_challenge_has_an_expectation_with_rows() {
+    for track in catalog().tracks() {
+        let template = template(track).unwrap();
+        for challenge in &track.challenges {
+            let expectation = expect(&template, challenge);
+            let name = format!("{}/{}", track.slug, challenge.id);
+            let expectation = expectation.unwrap_or_else(|error| panic!("{name}: {error:?}"));
+            assert!(!expectation.columns.is_empty(), "{name} has no columns");
+            assert!(expectation.rows > 0, "{name} has no rows");
+        }
+    }
+}
+
 #[test]
 fn no_skeleton_is_already_an_answer() {
     for track in catalog().tracks() {
@@ -84,8 +99,8 @@
         for challenge in &track.challenges {
             let verdict = check(&template, challenge, &challenge.skeleton);
             assert_ne!(
-                verdict,
-                Ok(Verdict::Solved),
+                verdict.as_ref().map(Verdict::is_solved),
+                Ok(true),
                 "{}/{}",
                 track.slug,
                 challenge.id
@@ -98,14 +113,14 @@
 fn wrong_columns_are_caught() {
     assert_eq!(
         vinyl("first-spin", "SELECT title FROM albums"),
-        Ok(Verdict::Wrong(CheckFailure::ColumnCount {
+        Ok(Some(CheckFailure::ColumnCount {
             expected: 2,
             got: 1
         }))
     );
     assert!(matches!(
         vinyl("first-spin", "SELECT title, year AS released FROM albums"),
-        Ok(Verdict::Wrong(CheckFailure::ColumnNames { .. }))
+        Ok(Some(CheckFailure::ColumnNames { .. }))
     ));
 }
 
@@ -116,7 +131,7 @@
             "crate-digging",
             "SELECT title, year FROM albums WHERE year <= 1975"
         ),
-        Ok(Verdict::Wrong(CheckFailure::RowCount {
+        Ok(Some(CheckFailure::RowCount {
             expected: 7,
             got: 8
         }))
@@ -126,14 +141,14 @@
             "unknown-origins",
             "SELECT name FROM artists WHERE country = NULL"
         ),
-        Ok(Verdict::Wrong(CheckFailure::RowCount {
+        Ok(Some(CheckFailure::RowCount {
             expected: 2,
             got: 0
         }))
     );
     assert_eq!(
         vinyl("every-genre", "SELECT genre FROM albums"),
-        Ok(Verdict::Wrong(CheckFailure::RowCount {
+        Ok(Some(CheckFailure::RowCount {
             expected: 8,
             got: 18
         }))
@@ -147,7 +162,7 @@
             "new-arrivals",
             "SELECT title, year FROM albums ORDER BY year LIMIT 5"
         ),
-        Ok(Verdict::Wrong(CheckFailure::Contents {
+        Ok(Some(CheckFailure::Contents {
             missing: 5,
             unexpected: 5
         }))
@@ -157,6 +172,6 @@
             "new-arrivals",
             "SELECT title, year FROM albums WHERE year >= 1985 ORDER BY year"
         ),
-        Ok(Verdict::Wrong(CheckFailure::Order { row: 0 }))
+        Ok(Some(CheckFailure::Order { row: 0 }))
     );
 }
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo nextest run -p quarry-challenges`
Expected: compile errors or failures for the names and behavior above.

- [ ] **Step 3: Implement**

Modify `crates/app/src/model/verdict.rs`:
```diff
@@ -57,11 +57,9 @@
 
 /// The report for a finished check.
 pub fn report(outcome: &Result<Verdict, CheckError>) -> Report {
-    match outcome {
-        Ok(Verdict::Solved) => Report::new(Tone::Solved, "Solved.", None),
-        Ok(Verdict::Wrong(failure)) => {
-            Report::new(Tone::Wrong, "Not quite yet.", Some(difference(failure)))
-        }
+    match outcome.as_ref().map(Verdict::failure) {
+        Ok(None) => Report::new(Tone::Solved, "Solved.", None),
+        Ok(Some(failure)) => Report::new(Tone::Wrong, "Not quite yet.", Some(difference(failure))),
         Err(error) => unchecked(error),
     }
 }
```

Modify `crates/challenges/src/catalog.rs`:
```diff
@@ -50,6 +50,25 @@
     }
 }
 
+/// The challenges either side of one, across every track in order.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct Neighbours {
+    /// The challenge before, if there is one.
+    pub prev: Option<ChallengeId>,
+    /// The challenge after, if there is one.
+    pub next: Option<ChallengeId>,
+}
+
+/// Where a challenge sits in its track: challenge `index` of `of`,
+/// counting from 1.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct Position {
+    /// Its place, counting from 1.
+    pub index: usize,
+    /// How many challenges its track has.
+    pub of: usize,
+}
+
 /// The tracks that loaded, by `order`.
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
 pub struct Catalog {
@@ -97,6 +116,48 @@
             .find(|challenge| challenge.id == id.challenge())?;
         Some((track, challenge))
     }
+
+    /// The challenges before and after `id`: within its track, then on
+    /// into the track before or after. An unknown id has neither.
+    pub fn neighbours(&self, id: &ChallengeId) -> Neighbours {
+        let ids = self.ids();
+        let Some(at) = ids.iter().position(|each| each == id) else {
+            return Neighbours::default();
+        };
+        Neighbours {
+            prev: at
+                .checked_sub(1)
+                .and_then(|before| ids.get(before))
+                .cloned(),
+            next: ids.get(at + 1).cloned(),
+        }
+    }
+
+    /// Where `id` sits in its track.
+    pub fn position(&self, id: &ChallengeId) -> Option<Position> {
+        let track = self.track(id.track())?;
+        let index = track
+            .challenges
+            .iter()
+            .position(|challenge| challenge.id == id.challenge())?;
+        Some(Position {
+            index: index + 1,
+            of: track.challenges.len(),
+        })
+    }
+
+    /// Every challenge's id, track by track.
+    fn ids(&self) -> Vec<ChallengeId> {
+        self.tracks
+            .iter()
+            .flat_map(|track| {
+                track
+                    .challenges
+                    .iter()
+                    .map(|challenge| ChallengeId::new(&track.slug, &challenge.id))
+            })
+            .collect()
+    }
 }
 
 /// `track`, if every challenge's kind is in `kinds`.
@@ -118,7 +179,7 @@
 
 #[cfg(test)]
 mod tests {
-    use super::{Catalog, ChallengeId};
+    use super::{Catalog, ChallengeId, Neighbours, Position};
     use crate::{embedded::RawTrack, error::ContentError};
 
     fn raw(slug: &'static str, order: u32, challenge: &'static str) -> RawTrack {
@@ -135,10 +196,12 @@
             format!("title = \"{slug}\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = {order}")
                 .into_boxed_str(),
         );
-        let challenges: &'static str = Box::leak(
-            format!("[[challenge]]\nid = \"{challenge}\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"{kind}\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]")
-                .into_boxed_str(),
-        );
+        let challenges = challenge
+            .split(',')
+            .map(|id| format!("[[challenge]]\nid = \"{id}\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"{kind}\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]"))
+            .collect::<Vec<_>>()
+            .join("\n");
+        let challenges: &'static str = Box::leak(challenges.into_boxed_str());
         RawTrack {
             slug,
             track,
@@ -207,6 +270,48 @@
         );
     }
 
+    fn id(text: &str) -> Option<ChallengeId> {
+        ChallengeId::parse(text)
+    }
+
+    #[test]
+    fn neighbours_run_through_a_track_then_on_to_the_next() {
+        let (catalog, _) = Catalog::load(&[raw("two", 2, "c"), raw("one", 1, "a,b")]);
+        let around = |text: &str| catalog.neighbours(&id(text).unwrap());
+        assert_eq!(
+            around("one/a"),
+            Neighbours {
+                prev: None,
+                next: id("one/b")
+            }
+        );
+        assert_eq!(
+            around("one/b"),
+            Neighbours {
+                prev: id("one/a"),
+                next: id("two/c")
+            }
+        );
+        assert_eq!(
+            around("two/c"),
+            Neighbours {
+                prev: id("one/b"),
+                next: None
+            }
+        );
+        assert_eq!(around("two/z"), Neighbours::default());
+    }
+
+    #[test]
+    fn a_position_counts_from_one_within_its_track() {
+        let (catalog, _) = Catalog::load(&[raw("one", 1, "a,b,c"), raw("two", 2, "d")]);
+        let at = |text: &str| catalog.position(&id(text).unwrap());
+        assert_eq!(at("one/b"), Some(Position { index: 2, of: 3 }));
+        assert_eq!(at("two/d"), Some(Position { index: 1, of: 1 }));
+        assert_eq!(at("one/z"), None);
+        assert_eq!(at("three/a"), None);
+    }
+
     #[test]
     fn nothing_embedded_is_broken() {
         let (_, errors) = Catalog::embedded();
```

Modify `crates/challenges/src/check.rs`:
```diff
@@ -4,20 +4,48 @@
 use quarry_engine::error::EngineError;
 
 use crate::{
-    kind::KindRegistry,
-    rules::{self, CheckFailure, Comparison},
+    kind::{ChallengeKind, KindRegistry},
+    rules::{self, CheckFailure, Comparison, Grading},
     run::{LIMITS, Limits, RunError},
     sandbox::Template,
     track::Challenge,
 };
 
-/// What a check decided.
+/// What a check decided, with where every rule stands.
 #[derive(Debug, Clone, PartialEq, Eq)]
 pub enum Verdict {
     /// The answer matches the solution.
-    Solved,
-    /// The answer is wrong; the failure says how.
-    Wrong(CheckFailure),
+    Solved(Grading),
+    /// The answer is wrong; the grading says how.
+    Wrong(Grading),
+}
+
+impl Verdict {
+    /// The verdict `grading` gives: wrong when a rule failed.
+    pub fn of(grading: Grading) -> Self {
+        if grading.failure().is_some() {
+            Self::Wrong(grading)
+        } else {
+            Self::Solved(grading)
+        }
+    }
+
+    /// Every rule's outcome.
+    pub fn grading(&self) -> &Grading {
+        match self {
+            Self::Solved(grading) | Self::Wrong(grading) => grading,
+        }
+    }
+
+    /// What was wrong, for a wrong answer.
+    pub fn failure(&self) -> Option<&CheckFailure> {
+        self.grading().failure()
+    }
+
+    /// Whether the answer is right.
+    pub fn is_solved(&self) -> bool {
+        matches!(self, Self::Solved(_))
+    }
 }
 
 /// Why an answer could not be checked.
@@ -67,16 +95,52 @@
     limits: Limits,
 ) -> Result<Verdict, CheckError> {
     let kinds = KindRegistry::standard();
-    let kind = kinds
-        .get(&challenge.kind)
-        .ok_or_else(|| CheckError::Content(format!("unknown kind \"{}\"", challenge.kind)))?;
-    let answers = kind.answers(challenge, sql, template, limits)?;
+    let answers = kind_of(&kinds, challenge)?.answers(challenge, sql, template, limits)?;
     let comparison = Comparison {
         expected: &answers.expected,
         got: &answers.got,
         ordered: challenge.ordered,
     };
-    Ok(rules::first_failure(&comparison).map_or(Verdict::Solved, Verdict::Wrong))
+    Ok(Verdict::of(rules::grade(&comparison)))
+}
+
+/// What a right answer to a challenge looks like, without its rows: the
+/// brief shows it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Expectation {
+    /// The column names, in order.
+    pub columns: Vec<String>,
+    /// How many rows.
+    pub rows: usize,
+}
+
+/// What a right answer to `challenge` gives, worked out on a copy of
+/// `template` within the grading [`LIMITS`].
+///
+/// # Errors
+/// Returns [`CheckError::Content`] when the solution gives no result, or
+/// [`CheckError::Sandbox`] when no copy can be made.
+pub fn expect(template: &Template, challenge: &Challenge) -> Result<Expectation, CheckError> {
+    let kinds = KindRegistry::standard();
+    let expected = kind_of(&kinds, challenge)?.expected(challenge, template, LIMITS)?;
+    Ok(Expectation {
+        columns: expected
+            .columns
+            .into_iter()
+            .map(|column| column.name)
+            .collect(),
+        rows: expected.rows.len(),
+    })
+}
+
+/// `challenge`'s kind in `kinds`.
+fn kind_of<'a>(
+    kinds: &'a KindRegistry,
+    challenge: &Challenge,
+) -> Result<&'a dyn ChallengeKind, CheckError> {
+    kinds
+        .get(&challenge.kind)
+        .ok_or_else(|| CheckError::Content(format!("unknown kind \"{}\"", challenge.kind)))
 }
 
 #[cfg(test)]
```

Modify `crates/challenges/src/kind/mod.rs`:
```diff
@@ -24,6 +24,19 @@
     /// Its id, as `kind` in `challenges.toml`, such as "query".
     fn id(&self) -> &'static str;
 
+    /// What a right answer to `challenge` gives, run on a fresh copy of
+    /// `template` within `limits`.
+    ///
+    /// # Errors
+    /// Returns [`CheckError::Content`] when the solution gives no result, or
+    /// [`CheckError::Sandbox`] when no copy can be made.
+    fn expected(
+        &self,
+        challenge: &Challenge,
+        template: &Template,
+        limits: Limits,
+    ) -> Result<QueryResult, CheckError>;
+
     /// Runs `sql` and `challenge`'s solution, each on a fresh copy of
     /// `template`, within `limits`.
     ///
```

Modify `crates/challenges/src/kind/query.rs`:
```diff
@@ -1,7 +1,7 @@
 //! Query challenges: the user's result is compared with the reference
 //! solution's, each run on its own fresh copy.
 
-use quarry_engine::sqlite::SqliteDriver;
+use quarry_engine::{result::QueryResult, sqlite::SqliteDriver};
 
 use super::{Answers, ChallengeKind};
 use crate::{
@@ -19,6 +19,16 @@
         "query"
     }
 
+    fn expected(
+        &self,
+        challenge: &Challenge,
+        template: &Template,
+        limits: Limits,
+    ) -> Result<QueryResult, CheckError> {
+        run::run(&mut copy(template)?, &challenge.solution, limits)
+            .map_err(|error| CheckError::Content(describe(&error)))
+    }
+
     fn answers(
         &self,
         challenge: &Challenge,
@@ -26,8 +36,7 @@
         template: &Template,
         limits: Limits,
     ) -> Result<Answers, CheckError> {
-        let expected = run::run(&mut copy(template)?, &challenge.solution, limits)
-            .map_err(|error| CheckError::Content(describe(&error)))?;
+        let expected = self.expected(challenge, template, limits)?;
         let got = run::run(&mut copy(template)?, sql, limits).map_err(CheckError::from)?;
         Ok(Answers { expected, got })
     }
```

Modify `crates/challenges/src/rules/columns.rs`:
```diff
@@ -12,6 +12,10 @@
         "columns"
     }
 
+    fn applies(&self, _: &Comparison<'_>) -> bool {
+        true
+    }
+
     fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
         let expected = names(comparison.expected);
         let got = names(comparison.got);
```

Modify `crates/challenges/src/rules/contents.rs`:
```diff
@@ -15,6 +15,10 @@
         "contents"
     }
 
+    fn applies(&self, _: &Comparison<'_>) -> bool {
+        true
+    }
+
     fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
         let expected = sorted(&comparison.expected.rows);
         let got = sorted(&comparison.got.rows);
```

Modify `crates/challenges/src/rules/count.rs`:
```diff
@@ -10,6 +10,10 @@
         "row count"
     }
 
+    fn applies(&self, _: &Comparison<'_>) -> bool {
+        true
+    }
+
     fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
         let expected = comparison.expected.rows.len();
         let got = comparison.got.rows.len();
```

Modify `crates/challenges/src/rules/mod.rs`:
```diff
@@ -1,5 +1,6 @@
 //! The grading pipeline (parent spec §5.4): columns, row count, contents,
-//! then order. It stops at the first rule that fails.
+//! then order. It stops at the first rule that fails, and reports where
+//! every rule stands.
 //!
 //! Each rule is a [`CheckRule`]; adding one is a new module and one line in
 //! [`pipeline`].
@@ -70,6 +71,58 @@
     /// What this rule finds wrong, if anything. It may assume every earlier
     /// rule passed.
     fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure>;
+
+    /// Whether this rule has anything to check in `comparison`; a rule that
+    /// doesn't apply is reported as not checked.
+    fn applies(&self, comparison: &Comparison<'_>) -> bool;
+}
+
+/// Where one rule stands after grading.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub enum RuleStatus {
+    /// The rule found nothing wrong.
+    Passed,
+    /// The rule found this wrong.
+    Failed(CheckFailure),
+    /// An earlier rule failed, or this one doesn't apply.
+    NotChecked,
+}
+
+/// One rule's outcome.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct RuleOutcome {
+    /// The rule's [`CheckRule::id`].
+    pub rule: &'static str,
+    /// Where it stands.
+    pub status: RuleStatus,
+}
+
+/// Every rule's outcome, in pipeline order.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Grading {
+    outcomes: Vec<RuleOutcome>,
+}
+
+impl Grading {
+    /// A grading made of `outcomes`, in pipeline order.
+    pub fn new(outcomes: Vec<RuleOutcome>) -> Self {
+        Self { outcomes }
+    }
+
+    /// Every rule's outcome, in pipeline order.
+    pub fn outcomes(&self) -> &[RuleOutcome] {
+        &self.outcomes
+    }
+
+    /// The failure of the rule that failed, if one did.
+    pub fn failure(&self) -> Option<&CheckFailure> {
+        self.outcomes
+            .iter()
+            .find_map(|outcome| match &outcome.status {
+                RuleStatus::Failed(failure) => Some(failure),
+                _ => None,
+            })
+    }
 }
 
 /// The rules in the order they run.
@@ -82,9 +135,27 @@
     ]
 }
 
-/// The first rule's failure, or `None` when every rule passes.
-pub fn first_failure(comparison: &Comparison<'_>) -> Option<CheckFailure> {
-    pipeline().iter().find_map(|rule| rule.check(comparison))
+/// Runs the pipeline on `comparison`. Every rule after the first failure,
+/// and every rule that doesn't apply, is not checked.
+pub fn grade(comparison: &Comparison<'_>) -> Grading {
+    let mut failed = false;
+    let outcomes = pipeline()
+        .iter()
+        .map(|rule| {
+            let status = if failed || !rule.applies(comparison) {
+                RuleStatus::NotChecked
+            } else {
+                rule.check(comparison)
+                    .map_or(RuleStatus::Passed, RuleStatus::Failed)
+            };
+            failed |= matches!(status, RuleStatus::Failed(_));
+            RuleOutcome {
+                rule: rule.id(),
+                status,
+            }
+        })
+        .collect();
+    Grading::new(outcomes)
 }
 
 #[cfg(test)]
@@ -117,10 +188,26 @@
 #[cfg(test)]
 mod tests {
     use super::{
-        CheckFailure, Comparison, first_failure, pipeline,
+        CheckFailure, Comparison, RuleStatus, grade, pipeline,
         testing::{ints, result},
     };
 
+    fn statuses(expected: &[i64], got: &[i64], names: &[&str], ordered: bool) -> Vec<RuleStatus> {
+        let expected = result(&["n"], ints(expected));
+        let got = result(names, ints(got));
+        let comparison = Comparison {
+            expected: &expected,
+            got: &got,
+            ordered,
+        };
+        let grading = grade(&comparison);
+        grading
+            .outcomes()
+            .iter()
+            .map(|outcome| outcome.status.clone())
+            .collect()
+    }
+
     #[test]
     fn the_rules_run_columns_count_contents_then_order() {
         let ids: Vec<&str> = pipeline().iter().map(|rule| rule.id()).collect();
@@ -128,32 +215,65 @@
     }
 
     #[test]
-    fn the_first_failing_rule_is_reported() {
+    fn every_rule_after_the_first_failure_is_not_checked() {
+        let failure = CheckFailure::RowCount {
+            expected: 2,
+            got: 1,
+        };
+        assert_eq!(
+            statuses(&[1, 2], &[1], &["n"], true),
+            [
+                RuleStatus::Passed,
+                RuleStatus::Failed(failure),
+                RuleStatus::NotChecked,
+                RuleStatus::NotChecked,
+            ]
+        );
+    }
+
+    #[test]
+    fn a_failure_at_the_last_rule_follows_three_passes() {
+        let statuses = statuses(&[1, 2], &[2, 1], &["n"], true);
+        assert_eq!(statuses[..3], [const { RuleStatus::Passed }; 3]);
+        assert_eq!(
+            statuses[3],
+            RuleStatus::Failed(CheckFailure::Order { row: 0 })
+        );
+    }
+
+    #[test]
+    fn a_matching_answer_passes_every_rule_that_applies() {
+        assert_eq!(
+            statuses(&[1, 2], &[1, 2], &["N"], true),
+            [const { RuleStatus::Passed }; 4]
+        );
+        let unordered = statuses(&[1, 2], &[2, 1], &["n"], false);
+        assert_eq!(unordered[..3], [const { RuleStatus::Passed }; 3]);
+        assert_eq!(unordered[3], RuleStatus::NotChecked);
+    }
+
+    #[test]
+    fn the_failure_is_the_rule_that_failed() {
         let expected = result(&["n"], ints(&[1, 2]));
         let wrong_columns = result(&["m", "n"], ints(&[3]));
-        let comparison = Comparison {
+        let failed = grade(&Comparison {
             expected: &expected,
             got: &wrong_columns,
             ordered: true,
-        };
+        });
         assert_eq!(
-            first_failure(&comparison),
-            Some(CheckFailure::ColumnCount {
+            failed.failure(),
+            Some(&CheckFailure::ColumnCount {
                 expected: 1,
                 got: 2
             })
         );
-    }
-
-    #[test]
-    fn a_matching_answer_passes_every_rule() {
-        let expected = result(&["n"], ints(&[1, 2]));
-        let got = result(&["N"], ints(&[1, 2]));
-        let comparison = Comparison {
+        assert_eq!(failed.outcomes()[0].rule, "columns");
+        let solved = grade(&Comparison {
             expected: &expected,
-            got: &got,
+            got: &expected,
             ordered: true,
-        };
-        assert_eq!(first_failure(&comparison), None);
+        });
+        assert_eq!(solved.failure(), None);
     }
 }
```

Modify `crates/challenges/src/rules/order.rs`:
```diff
@@ -12,7 +12,7 @@
     }
 
     fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
-        if !comparison.ordered {
+        if !self.applies(comparison) {
             return None;
         }
         comparison
@@ -23,6 +23,10 @@
             .position(|(a, b)| !value::same_row(a, b))
             .map(|row| CheckFailure::Order { row })
     }
+
+    fn applies(&self, comparison: &Comparison<'_>) -> bool {
+        comparison.ordered
+    }
 }
 
 #[cfg(test)]
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p quarry-challenges`
Expected: all pass.

- [ ] **Step 5: Run the gates**

Run: `just fmt lint quality typos complexity && cargo nextest run --workspace`
Expected: no violations, and `486` tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/model/verdict.rs crates/app/src/model/verdict/tests.rs crates/app/tests/app/check.rs crates/challenges/src/catalog.rs crates/challenges/src/check.rs crates/challenges/src/check/tests.rs crates/challenges/src/kind/mod.rs crates/challenges/src/kind/query.rs crates/challenges/src/rules/columns.rs crates/challenges/src/rules/contents.rs crates/challenges/src/rules/count.rs crates/challenges/src/rules/mod.rs crates/challenges/src/rules/order.rs crates/challenges/tests/content.rs
git commit -m "feat(challenges): grade every rule, work out expectations, find neighbours"
```

---

### Task 2: App: inject the tracks, and stop a superseded check

Two follow-ups from 2A. `Workspace::new` takes a `Databases` bundle (the sample session, its name and the `Tracks`), keeping it within five arguments, so tests can open a workspace on broken tracks. `run::Stop` holds the cancellers of the runs it watches, and `run::Runner` registers each run with its stop; `check_within` becomes `check_with(.., &Runner)`. A tab stops its previous check before starting a new one.

**Files:**
- Modify: `crates/app/src/shell/window.rs`
- Modify: `crates/app/src/views/query_tab/check.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/tests/app/challenges.rs`
- Modify: `crates/app/tests/app/check.rs`
- Modify: `crates/app/tests/app/support.rs`
- Modify: `crates/challenges/src/check.rs`
- Modify: `crates/challenges/src/check/tests.rs`
- Modify: `crates/challenges/src/kind/mod.rs`
- Modify: `crates/challenges/src/kind/query.rs`
- Modify: `crates/challenges/src/run.rs`
- Modify: `crates/challenges/src/run/tests.rs`

**Interfaces:**
- Consumes: `run::run`, `Canceller`, `Workspace::new`, `QueryTab::check`.
- Produces: `run::{Stop, Runner}`, `check::check_with`, `views::workspace::Databases`, `QueryTab::check_stop`; test helpers `support::{open_on, broken_tracks}`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/challenges.rs`:
```diff
@@ -8,12 +8,12 @@
 use quarry_challenges::catalog::ChallengeId;
 use quarry_engine::value::Value;
 use quarry_store::{
-    drafts::{Draft, DraftStore, Drafts},
+    drafts::{Draft, DraftStore, Drafts, Forgetful},
     error::StoreError,
     sqlite::SqliteStore,
 };
 
-use super::support::{install, open, open_with, phase, run, settle, write};
+use super::support::{broken_tracks, install, open, open_on, open_with, phase, run, settle, write};
 
 fn first_spin() -> ChallengeId {
     ChallengeId::new("vinyl-vault", "first-spin")
@@ -67,6 +67,18 @@
     assert_eq!(active, Some(first_spin()));
 }
 
+#[gpui_kit::test]
+fn a_challenge_whose_database_cant_be_built_opens_nothing(cx: &mut TestAppContext) {
+    install(cx);
+    let tracks = broken_tracks("CREATE TABLE", "SELECT 1");
+    let (workspace, cx) =
+        open_on(cx, Arc::new(Forgetful), tracks).expect("the sample database opens");
+    open_challenge(&workspace, cx, &ChallengeId::new("broken", "a"));
+    assert_eq!(titles(&workspace, cx), ["Query 1"]);
+    let note = cx.update(|_, cx| workspace.read(cx).note().map(str::to_owned));
+    assert!(note.is_some_and(|note| note.starts_with("Couldn't open Cracked: ")));
+}
+
 #[gpui_kit::test]
 fn opening_a_challenge_again_returns_to_its_tab(cx: &mut TestAppContext) {
     let (workspace, cx) = open(cx).expect("the sample database opens");
```

Modify `crates/app/tests/app/check.rs`:
```diff
@@ -134,3 +134,24 @@
     assert_eq!(note.as_deref(), Some(NOT_A_CHALLENGE));
     assert_eq!(state(&workspace, cx), CheckState::Idle);
 }
+
+#[gpui_kit::test]
+fn a_newer_check_stops_the_older_one(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    check_first_spin(&workspace, cx, "SELECT title FROM albums");
+    let stop = |cx: &mut VisualTestContext| {
+        cx.update(|_, cx| {
+            workspace
+                .read(cx)
+                .active_tab()
+                .read(cx)
+                .check_stop()
+                .clone()
+        })
+    };
+    let first = stop(cx);
+    assert!(!first.is_stopped());
+    cx.simulate_keystrokes("cmd-shift-enter");
+    assert!(first.is_stopped());
+    assert!(!stop(cx).is_stopped());
+}
```

Modify `crates/app/tests/app/support.rs`:
```diff
@@ -10,8 +10,10 @@
     model::statements::Phase,
     sample,
     shell::actions,
-    views::workspace::{POLL, SavedTabs, Workspace},
+    tracks::Tracks,
+    views::workspace::{Databases, POLL, SavedTabs, Workspace},
 };
+use quarry_challenges::{catalog::Catalog, embedded::RawTrack};
 use quarry_engine::error::EngineError;
 use quarry_store::drafts::{DraftStore, Forgetful};
 use quarry_ui_kit::{Aurora, Preference};
@@ -38,20 +40,34 @@
 }
 
 /// A workspace on a fresh sample database saving to `store`, with its
-/// editor focused. Like the real window, it sits in gpui-kit's `Root`,
-/// which draws dialogs. Call [`install`] first.
+/// editor focused. Call [`install`] first.
 pub fn open_with(
     cx: &mut TestAppContext,
     store: Arc<dyn DraftStore>,
 ) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
-    let session = sample::open()?;
+    open_on(cx, store, Tracks::embedded())
+}
+
+/// A workspace on a fresh sample database and `tracks`, saving to
+/// `store`, with its editor focused. Like the real window, it sits in
+/// gpui-kit's `Root`, which draws dialogs. Call [`install`] first.
+pub fn open_on(
+    cx: &mut TestAppContext,
+    store: Arc<dyn DraftStore>,
+    tracks: Tracks,
+) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
+    let databases = Databases {
+        session: sample::open()?,
+        name: sample::NAME.to_owned(),
+        tracks,
+    };
     let saved = SavedTabs {
         store,
         problem: None,
     };
     let mut opened = None;
     let handle = cx.add_window(|window, cx| {
-        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, saved, window, cx));
+        let workspace = cx.new(|cx| Workspace::new(databases, saved, window, cx));
         opened = Some(workspace.clone());
         Root::new(workspace, window, cx)
     });
@@ -123,3 +139,19 @@
         window.try_find(id).is_some()
     })
 }
+
+/// One track, "Broken", whose challenge `broken/a` has `solution` and whose
+/// database is built from `schema`.
+pub fn broken_tracks(schema: &'static str, solution: &str) -> Tracks {
+    let challenges = format!(
+        "[[challenge]]\nid = \"a\"\ntitle = \"Cracked\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"{solution}\"\nhints = [\"1\", \"2\", \"3\"]"
+    );
+    let raw = RawTrack {
+        slug: "broken",
+        track: "title = \"Broken\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = 1",
+        schema,
+        seed: "",
+        challenges: Box::leak(challenges.into_boxed_str()),
+    };
+    Tracks::new(Catalog::load(&[raw]).0)
+}
```

Modify `crates/challenges/src/check/tests.rs`:
```diff
@@ -2,10 +2,10 @@
 
 use quarry_engine::error::EngineError;
 
-use super::{CheckError, Expectation, Verdict, check, check_within, expect};
+use super::{CheckError, Expectation, Verdict, check, check_with, expect};
 use crate::{
     rules::CheckFailure,
-    run::{LIMITS, Limits, RunError},
+    run::{LIMITS, Limits, RunError, Runner, Stop},
     sandbox::Template,
     track::Challenge,
 };
@@ -75,7 +75,12 @@
     };
     let challenge = challenge("SELECT 1", false);
     assert_eq!(
-        check_within(&template().unwrap(), &challenge, endless, limits),
+        check_with(
+            &template().unwrap(),
+            &challenge,
+            endless,
+            &Runner::new(limits)
+        ),
         Err(CheckError::TimedOut)
     );
 }
@@ -123,3 +128,19 @@
         Err(CheckError::Content("unknown kind \"mutation\"".to_owned()))
     );
 }
+
+#[test]
+fn a_stopped_check_ends_without_waiting_for_its_limits() {
+    let endless =
+        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
+    let stop = Stop::default();
+    stop.stop();
+    let runner = Runner::stoppable(LIMITS, stop);
+    let started = std::time::Instant::now();
+    let challenge = challenge("SELECT 1", false);
+    assert_eq!(
+        check_with(&template().unwrap(), &challenge, endless, &runner),
+        Err(CheckError::TimedOut)
+    );
+    assert!(started.elapsed() < LIMITS.time);
+}
```

Modify `crates/challenges/src/run/tests.rs`:
```diff
@@ -2,7 +2,7 @@
 
 use quarry_engine::{error::EngineError, sqlite::SqliteDriver, value::Value};
 
-use super::{LIMITS, Limits, RunError, run};
+use super::{LIMITS, Limits, RunError, Runner, Stop, run};
 
 const ENDLESS: &str =
     "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
@@ -93,3 +93,29 @@
     std::thread::sleep(Duration::from_millis(60));
     assert!(!quarry_engine::driver::Driver::canceller(&driver).is_cancelled());
 }
+
+#[test]
+fn stopping_cancels_a_run_in_flight() {
+    let stop = Stop::default();
+    let runner = Runner::stoppable(LIMITS, stop.clone());
+    let started = std::time::Instant::now();
+    let running = std::thread::spawn(move || {
+        let mut driver = database().unwrap();
+        runner.run(&mut driver, ENDLESS)
+    });
+    std::thread::sleep(Duration::from_millis(50));
+    assert!(!stop.is_stopped());
+    stop.stop();
+    assert!(stop.is_stopped());
+    assert_eq!(running.join().unwrap(), Err(RunError::TimedOut));
+    assert!(started.elapsed() < LIMITS.time);
+}
+
+#[test]
+fn a_runner_that_nothing_stops_runs_to_the_end() {
+    let mut driver = database().unwrap();
+    let result = Runner::new(LIMITS)
+        .run(&mut driver, "SELECT n FROM t")
+        .unwrap();
+    assert_eq!(result.rows, [[Value::Int(1)], [Value::Int(2)]]);
+}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo nextest run -p quarry-challenges -p quarry`
Expected: compile errors or failures for the names and behavior above.

- [ ] **Step 3: Implement**

Modify `crates/app/src/shell/window.rs`:
```diff
@@ -9,7 +9,8 @@
 
 use crate::{
     sample,
-    views::workspace::{SavedTabs, Workspace},
+    tracks::Tracks,
+    views::workspace::{Databases, SavedTabs, Workspace},
 };
 
 /// The main window's initial size.
@@ -43,12 +44,16 @@
 /// # Errors
 /// Returns the engine's error when the sample database cannot be opened.
 pub fn open_main(saved: SavedTabs, cx: &mut App) -> Result<(), EngineError> {
-    let session = sample::open()?;
+    let databases = Databases {
+        session: sample::open()?,
+        name: sample::NAME.to_owned(),
+        tracks: Tracks::embedded(),
+    };
     let opened = gpui_kit::open_window(options(cx), cx, |window, cx| {
         window
             .observe_window_appearance(|_, cx| Aurora::system_appearance_changed(cx))
             .detach();
-        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, saved, window, cx));
+        let workspace = cx.new(|cx| Workspace::new(databases, saved, window, cx));
         let editor = workspace.read(cx).editor(cx);
         editor.update(cx, |state, cx| state.focus(window, cx));
         workspace
```

Modify `crates/app/src/views/query_tab/check.rs`:
```diff
@@ -4,7 +4,12 @@
 use std::sync::Arc;
 
 use gpui_kit::{AppContext, Context};
-use quarry_challenges::{check, sandbox::Template, track::Challenge};
+use quarry_challenges::{
+    check,
+    run::{LIMITS, Runner, Stop},
+    sandbox::Template,
+    track::Challenge,
+};
 
 use super::QueryTab;
 use crate::{model::verdict::CheckState, views::results::CHECK};
@@ -17,15 +22,18 @@
     }
 
     /// Grades the editor's text as an answer to `challenge`, on copies of
-    /// `template`, and shows the Check view. A newer check replaces one
-    /// still running.
+    /// `template`, and shows the Check view. A newer check stops one still
+    /// running and replaces it.
     pub fn check(&mut self, template: Arc<Template>, challenge: Challenge, cx: &mut Context<Self>) {
         let sql = self.text(cx);
+        let runner = self.restart_check();
         self.check = CheckState::Checking;
         self.select_view(CHECK, cx);
         self.checking = Some(cx.spawn(async move |this, cx| {
             let outcome = cx
-                .background_spawn(async move { check::check(&template, &challenge, &sql) })
+                .background_spawn(
+                    async move { check::check_with(&template, &challenge, &sql, &runner) },
+                )
                 .await;
             let _ = this.update(cx, |tab, cx| {
                 tab.check = CheckState::Done(outcome);
@@ -35,6 +43,18 @@
         cx.notify();
     }
 
+    /// Stops the latest check, and returns a runner for the next one.
+    fn restart_check(&mut self) -> Runner {
+        self.check_stop.stop();
+        self.check_stop = Stop::default();
+        Runner::stoppable(LIMITS, self.check_stop.clone())
+    }
+
+    /// What stops the tab's latest check.
+    pub fn check_stop(&self) -> &Stop {
+        &self.check_stop
+    }
+
     /// Where the tab's latest check is.
     pub fn check_state(&self) -> &CheckState {
         &self.check
```

Modify `crates/app/src/views/query_tab/mod.rs`:
```diff
@@ -18,6 +18,7 @@
         table::TableState,
     },
 };
+use quarry_challenges::run::Stop;
 use quarry_engine::session::{RunEvent, RunHandle};
 
 use super::{
@@ -64,6 +65,7 @@
     planning: Option<Task<()>>,
     check: CheckState,
     checking: Option<Task<()>>,
+    check_stop: Stop,
     _edits: Subscription,
 }
 
@@ -94,6 +96,7 @@
             planning: None,
             check: CheckState::Idle,
             checking: None,
+            check_stop: Stop::default(),
             _edits: edits,
         }
     }
```

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -33,10 +33,10 @@
         Some(challenge.title.clone())
     }
 
-    /// The embedded tracks, with no challenge tab open.
-    pub(super) fn embedded() -> Self {
+    /// `tracks`, with no challenge tab open.
+    pub(super) fn new(tracks: Tracks) -> Self {
         Self {
-            tracks: Tracks::embedded(),
+            tracks,
             sandboxes: BTreeMap::new(),
         }
     }
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -49,6 +49,7 @@
         tabs::{Tab, TabId, TabList},
     },
     shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
+    tracks::Tracks,
 };
 
 /// How often the workspace polls the session: about 30 times a second.
@@ -57,6 +58,17 @@
 /// Width of the open sidebar.
 pub const SIDEBAR_WIDTH: f32 = 240.0;
 
+/// What a workspace runs on: a session for plain tabs, named in the
+/// status bar, and the challenge tracks.
+pub struct Databases {
+    /// The session plain tabs share.
+    pub session: Session,
+    /// What the status bar calls it.
+    pub name: String,
+    /// The challenge tracks.
+    pub tracks: Tracks,
+}
+
 /// The main window's content.
 ///
 /// Every tab in `tabs` has a view in `views`; challenge tabs also have a
@@ -87,20 +99,18 @@
 }
 
 impl Workspace {
-    /// A workspace on `session`, labelled `database` in the status bar,
-    /// with the tabs saved in `saved`.
+    /// A workspace on `databases`, with the tabs saved in `saved`.
     pub fn new(
-        session: Session,
-        database: &str,
+        databases: Databases,
         saved: SavedTabs,
         window: &mut Window,
         cx: &mut Context<Self>,
     ) -> Self {
         let (store, drafts, problem) = saved.open();
-        let challenges = Challenges::embedded();
+        let challenges = Challenges::new(databases.tracks);
         let mut workspace = Self {
-            session,
-            database: database.to_owned(),
+            session: databases.session,
+            database: databases.name,
             tabs: restored_tabs(&drafts, &challenges),
             views: BTreeMap::new(),
             challenges,
```

Modify `crates/challenges/src/check.rs`:
```diff
@@ -6,7 +6,7 @@
 use crate::{
     kind::{ChallengeKind, KindRegistry},
     rules::{self, CheckFailure, Comparison, Grading},
-    run::{LIMITS, Limits, RunError},
+    run::{LIMITS, RunError, Runner},
     sandbox::Template,
     track::Challenge,
 };
@@ -81,21 +81,21 @@
 /// Returns a [`CheckError`] when the answer or the solution gives no
 /// result.
 pub fn check(template: &Template, challenge: &Challenge, sql: &str) -> Result<Verdict, CheckError> {
-    check_within(template, challenge, sql, LIMITS)
+    check_with(template, challenge, sql, &Runner::new(LIMITS))
 }
 
-/// [`check`] within `limits`.
+/// [`check`], running both sides with `runner`.
 ///
 /// # Errors
 /// As for [`check`].
-pub fn check_within(
+pub fn check_with(
     template: &Template,
     challenge: &Challenge,
     sql: &str,
-    limits: Limits,
+    runner: &Runner,
 ) -> Result<Verdict, CheckError> {
     let kinds = KindRegistry::standard();
-    let answers = kind_of(&kinds, challenge)?.answers(challenge, sql, template, limits)?;
+    let answers = kind_of(&kinds, challenge)?.answers(challenge, sql, template, runner)?;
     let comparison = Comparison {
         expected: &answers.expected,
         got: &answers.got,
@@ -122,7 +122,8 @@
 /// [`CheckError::Sandbox`] when no copy can be made.
 pub fn expect(template: &Template, challenge: &Challenge) -> Result<Expectation, CheckError> {
     let kinds = KindRegistry::standard();
-    let expected = kind_of(&kinds, challenge)?.expected(challenge, template, LIMITS)?;
+    let expected =
+        kind_of(&kinds, challenge)?.expected(challenge, template, &Runner::new(LIMITS))?;
     Ok(Expectation {
         columns: expected
             .columns
```

Modify `crates/challenges/src/kind/mod.rs`:
```diff
@@ -8,7 +8,7 @@
 
 use quarry_engine::result::QueryResult;
 
-use crate::{check::CheckError, run::Limits, sandbox::Template, track::Challenge};
+use crate::{check::CheckError, run::Runner, sandbox::Template, track::Challenge};
 
 /// The user's result and the expected one, for the rules to compare.
 #[derive(Debug, Clone, PartialEq)]
@@ -24,8 +24,8 @@
     /// Its id, as `kind` in `challenges.toml`, such as "query".
     fn id(&self) -> &'static str;
 
-    /// What a right answer to `challenge` gives, run on a fresh copy of
-    /// `template` within `limits`.
+    /// What a right answer to `challenge` gives, run by `runner` on a fresh
+    /// copy of `template`.
     ///
     /// # Errors
     /// Returns [`CheckError::Content`] when the solution gives no result, or
@@ -34,11 +34,11 @@
         &self,
         challenge: &Challenge,
         template: &Template,
-        limits: Limits,
+        runner: &Runner,
     ) -> Result<QueryResult, CheckError>;
 
-    /// Runs `sql` and `challenge`'s solution, each on a fresh copy of
-    /// `template`, within `limits`.
+    /// Runs `sql` and `challenge`'s solution with `runner`, each on a
+    /// fresh copy of `template`.
     ///
     /// # Errors
     /// Returns a [`CheckError`] when either cannot give a result.
@@ -47,7 +47,7 @@
         challenge: &Challenge,
         sql: &str,
         template: &Template,
-        limits: Limits,
+        runner: &Runner,
     ) -> Result<Answers, CheckError>;
 }
```

Modify `crates/challenges/src/kind/query.rs`:
```diff
@@ -6,7 +6,7 @@
 use super::{Answers, ChallengeKind};
 use crate::{
     check::CheckError,
-    run::{self, Limits, RunError},
+    run::{RunError, Runner},
     sandbox::Template,
     track::Challenge,
 };
@@ -23,9 +23,10 @@
         &self,
         challenge: &Challenge,
         template: &Template,
-        limits: Limits,
+        runner: &Runner,
     ) -> Result<QueryResult, CheckError> {
-        run::run(&mut copy(template)?, &challenge.solution, limits)
+        runner
+            .run(&mut copy(template)?, &challenge.solution)
             .map_err(|error| CheckError::Content(describe(&error)))
     }
 
@@ -34,10 +35,12 @@
         challenge: &Challenge,
         sql: &str,
         template: &Template,
-        limits: Limits,
+        runner: &Runner,
     ) -> Result<Answers, CheckError> {
-        let expected = self.expected(challenge, template, limits)?;
-        let got = run::run(&mut copy(template)?, sql, limits).map_err(CheckError::from)?;
+        let expected = self.expected(challenge, template, runner)?;
+        let got = runner
+            .run(&mut copy(template)?, sql)
+            .map_err(CheckError::from)?;
         Ok(Answers { expected, got })
     }
 }
@@ -64,7 +67,7 @@
     use crate::{
         check::CheckError,
         kind::ChallengeKind,
-        run::{LIMITS, Limits, RunError},
+        run::{LIMITS, Limits, RunError, Runner},
         sandbox::Template,
         track::Challenge,
     };
@@ -98,7 +101,7 @@
                 &challenge("SELECT count(*) AS n FROM t"),
                 sql,
                 &template().unwrap(),
-                LIMITS,
+                &Runner::new(LIMITS),
             )
             .unwrap();
         assert_eq!(answers.expected.rows, [[Value::Int(2)]]);
@@ -109,13 +112,23 @@
     fn the_users_errors_keep_their_kind() {
         let template = template().unwrap();
         let solution = challenge("SELECT n FROM t");
-        let failed = Query.answers(&solution, "SELECT x FROM t", &template, LIMITS);
+        let failed = Query.answers(
+            &solution,
+            "SELECT x FROM t",
+            &template,
+            &Runner::new(LIMITS),
+        );
         assert_eq!(failed, Err(CheckError::Sql("no such column: x".to_owned())));
         let tight = Limits {
             rows: 1,
             time: LIMITS.time,
         };
-        let too_many = Query.answers(&challenge("SELECT 1"), "SELECT n FROM t", &template, tight);
+        let too_many = Query.answers(
+            &challenge("SELECT 1"),
+            "SELECT n FROM t",
+            &template,
+            &Runner::new(tight),
+        );
         assert_eq!(too_many, Err(CheckError::TooManyRows));
     }
 
@@ -125,7 +138,7 @@
             &challenge("SELECT nope FROM t"),
             "SELECT 1",
             &template().unwrap(),
-            LIMITS,
+            &Runner::new(LIMITS),
         );
         assert_eq!(
             broken,
```

Modify `crates/challenges/src/run.rs`:
```diff
@@ -2,7 +2,10 @@
 //! time limit.
 
 use std::{
-    sync::mpsc::{self, RecvTimeoutError},
+    sync::{
+        Arc, Mutex, PoisonError,
+        mpsc::{self, RecvTimeoutError},
+    },
     thread,
     time::Duration,
 };
@@ -40,6 +43,75 @@
     TooManyRows,
 }
 
+/// Stops graded runs from outside, such as a check that a newer one
+/// replaced. Clones share one stop.
+#[derive(Debug, Clone, Default)]
+pub struct Stop(Arc<Mutex<Stopping>>);
+
+#[derive(Debug, Default)]
+struct Stopping {
+    stopped: bool,
+    runs: Vec<Canceller>,
+}
+
+impl Stop {
+    /// Cancels every run it watches, and every run it watches later.
+    pub fn stop(&self) {
+        let mut stopping = self.lock();
+        stopping.stopped = true;
+        stopping.runs.iter().for_each(Canceller::cancel);
+    }
+
+    /// Whether [`Stop::stop`] was called.
+    pub fn is_stopped(&self) -> bool {
+        self.lock().stopped
+    }
+
+    /// Cancels `canceller` when the stop is called, or now if it was.
+    fn watch(&self, canceller: Canceller) {
+        let mut stopping = self.lock();
+        if stopping.stopped {
+            canceller.cancel();
+        }
+        stopping.runs.push(canceller);
+    }
+
+    fn lock(&self) -> std::sync::MutexGuard<'_, Stopping> {
+        // Every update leaves the state whole, so a panic elsewhere while
+        // the lock was held leaves nothing half-done.
+        self.0.lock().unwrap_or_else(PoisonError::into_inner)
+    }
+}
+
+/// Runs scripts for grading within its limits, until its stop is called.
+#[derive(Debug, Clone)]
+pub struct Runner {
+    limits: Limits,
+    stop: Stop,
+}
+
+impl Runner {
+    /// A runner within `limits` that nothing stops.
+    pub fn new(limits: Limits) -> Self {
+        Self::stoppable(limits, Stop::default())
+    }
+
+    /// A runner within `limits` that `stop` can stop.
+    pub fn stoppable(limits: Limits, stop: Stop) -> Self {
+        Self { limits, stop }
+    }
+
+    /// [`run`]s `script` on `driver`. A run cut short by the stop fails as
+    /// [`RunError::TimedOut`]; whoever stopped it has stopped listening.
+    ///
+    /// # Errors
+    /// As for [`run`].
+    pub fn run(&self, driver: &mut dyn Driver, script: &str) -> Result<QueryResult, RunError> {
+        self.stop.watch(driver.canceller());
+        run(driver, script, self.limits)
+    }
+}
+
 /// Runs every statement of `script` on `driver` within `limits`, and
 /// returns the last statement's result. A last statement that returns no
 /// rows has no columns.
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p quarry-challenges -p quarry`
Expected: all pass.

- [ ] **Step 5: Run the gates**

Run: `just fmt lint quality typos complexity && cargo nextest run --workspace`
Expected: no violations, and `491` tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/shell/window.rs crates/app/src/views/query_tab/check.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/mod.rs crates/app/tests/app/challenges.rs crates/app/tests/app/check.rs crates/app/tests/app/support.rs crates/challenges/src/check.rs crates/challenges/src/check/tests.rs crates/challenges/src/kind/mod.rs crates/challenges/src/kind/query.rs crates/challenges/src/run.rs crates/challenges/src/run/tests.rs
git commit -m "feat(app): inject the tracks and stop a superseded check"
```

---

### Task 3: App: attempts and each challenge's expected result

`model::attempts` counts each check that gives a verdict, per challenge, for the session. `model::brief` turns a challenge, its position, its expectation and its tally into the brief's lines. `Tracks` caches each challenge's `Expected` (asked for once), worked out on the background executor when its tab opens. A tab emits `QueryTabEvent::Checked`, and the workspace records the attempt.

**Files:**
- Create: `crates/app/src/model/attempts.rs`
- Create: `crates/app/src/model/brief.rs`
- Create: `crates/app/src/model/brief/tests.rs`
- Modify: `crates/app/src/model/mod.rs`
- Modify: `crates/app/src/tracks.rs`
- Modify: `crates/app/src/views/query_tab/check.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/tabs.rs`
- Create: `crates/app/tests/app/brief.rs`
- Modify: `crates/app/tests/app/check.rs`
- Modify: `crates/app/tests/app/main.rs`

**Interfaces:**
- Consumes: `check::expect`, `Catalog::position`, `Verdict::is_solved`.
- Produces: `model::attempts::{Attempts, Tally}`, `model::brief::{Brief, Expected, brief}`, `Tracks::{expected, start_expecting, set_expected}`, `QueryTabEvent::Checked`, `Workspace::attempts`.

- [ ] **Step 1: Write the failing tests**

Create `crates/app/src/model/brief/tests.rs`:
```diff
@@ -0,0 +1,115 @@
+use quarry_challenges::{
+    catalog::Position,
+    check::{CheckError, Expectation},
+    track::{Challenge, Track},
+};
+
+use super::{Brief, Expected, brief};
+use crate::model::attempts::Tally;
+
+fn track() -> Track {
+    Track {
+        slug: "vinyl-vault".to_owned(),
+        title: "Vinyl Vault".to_owned(),
+        emoji: "E".to_owned(),
+        intro: "I".to_owned(),
+        concepts: Vec::new(),
+        difficulty: "D".to_owned(),
+        order: 1,
+        schema: String::new(),
+        seed: String::new(),
+        challenges: Vec::new(),
+    }
+}
+
+fn challenge(ordered: bool) -> Challenge {
+    Challenge {
+        id: "first-spin".to_owned(),
+        title: "First spin".to_owned(),
+        prompt: "List every album.".to_owned(),
+        kind: "query".to_owned(),
+        ordered,
+        concepts: vec!["SELECT".to_owned(), "columns".to_owned()],
+        solution: String::new(),
+        hints: Vec::new(),
+        skeleton: String::new(),
+    }
+}
+
+fn ready(columns: &[&str], rows: usize) -> Expected {
+    Expected::Ready(Expectation {
+        columns: columns.iter().map(|name| (*name).to_owned()).collect(),
+        rows,
+    })
+}
+
+fn of(expected: &Expected, ordered: bool, checks: u32) -> Brief {
+    let tally = Tally {
+        checks,
+        solved: false,
+    };
+    brief(
+        &track(),
+        &challenge(ordered),
+        Position { index: 2, of: 5 },
+        expected,
+        tally,
+    )
+}
+
+#[test]
+fn the_brief_places_and_tells_the_challenge() {
+    let brief = of(&ready(&["title", "year"], 1_200), false, 2);
+    assert_eq!(
+        brief,
+        Brief {
+            place: "Vinyl Vault · Challenge 2 of 5".to_owned(),
+            title: "First spin".to_owned(),
+            prompt: "List every album.".to_owned(),
+            expected: vec![
+                "Expected columns: title, year".to_owned(),
+                "1,200 rows".to_owned()
+            ],
+            order: "Any order",
+            concepts: vec!["SELECT".to_owned(), "columns".to_owned()],
+            attempts: "Attempts: 2".to_owned(),
+        }
+    );
+}
+
+#[test]
+fn one_row_is_singular_and_order_can_matter() {
+    let brief = of(&ready(&["n"], 1), true, 0);
+    assert_eq!(brief.expected[1], "1 row");
+    assert_eq!(brief.order, "Order matters");
+    assert_eq!(brief.attempts, "Attempts: 0");
+}
+
+#[test]
+fn an_unknown_expectation_says_why() {
+    assert_eq!(
+        of(&Expected::Pending, false, 0).expected,
+        ["Working out the expected result…"]
+    );
+    assert_eq!(
+        of(&Expected::Unavailable, false, 0).expected,
+        ["Expected result unavailable"]
+    );
+    assert_eq!(Expected::default(), Expected::Pending);
+}
+
+#[test]
+fn a_result_becomes_ready_or_unavailable() {
+    let expectation = Expectation {
+        columns: vec!["n".to_owned()],
+        rows: 3,
+    };
+    assert_eq!(
+        Expected::from(Ok(expectation.clone())),
+        Expected::Ready(expectation)
+    );
+    assert_eq!(
+        Expected::from(Err(CheckError::TimedOut)),
+        Expected::Unavailable
+    );
+}
```

Create `crates/app/tests/app/brief.rs`:
```diff
@@ -0,0 +1,78 @@
+//! The brief's data: each challenge's expected result, worked out in the
+//! background, and this session's attempts.
+
+use std::sync::Arc;
+
+use gpui_kit::{Entity, TestAppContext, VisualTestContext};
+use quarry::{
+    model::{attempts::Tally, brief::Expected},
+    views::workspace::Workspace,
+};
+use quarry_challenges::{catalog::ChallengeId, check::Expectation};
+use quarry_store::drafts::Forgetful;
+
+use super::{
+    challenges::open_challenge,
+    check::check_first_spin,
+    support::{broken_tracks, install, open, open_on, settle},
+};
+
+fn first_spin() -> ChallengeId {
+    ChallengeId::new("vinyl-vault", "first-spin")
+}
+
+/// Waits until `id`'s expectation is no longer pending, and returns it.
+fn expected(
+    workspace: &Entity<Workspace>,
+    cx: &mut VisualTestContext,
+    id: &ChallengeId,
+) -> Expected {
+    let known = |cx: &mut gpui_kit::App| workspace.read(cx).tracks().expected(id);
+    assert!(settle(cx, |_, cx| known(cx) != Expected::Pending));
+    cx.update(|_, cx| known(cx))
+}
+
+fn tally(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Tally {
+    cx.update(|_, cx| workspace.read(cx).attempts().of(&first_spin()))
+}
+
+#[gpui_kit::test]
+fn opening_a_challenge_works_out_its_expected_result(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    open_challenge(&workspace, cx, &first_spin());
+    assert_eq!(
+        expected(&workspace, cx, &first_spin()),
+        Expected::Ready(Expectation {
+            columns: vec!["title".to_owned(), "year".to_owned()],
+            rows: 18,
+        })
+    );
+}
+
+#[gpui_kit::test]
+fn a_broken_solution_leaves_the_expectation_unavailable(cx: &mut TestAppContext) {
+    install(cx);
+    let tracks = broken_tracks("CREATE TABLE t (n INTEGER);", "SELECT nope FROM t");
+    let (workspace, cx) =
+        open_on(cx, Arc::new(Forgetful), tracks).expect("the sample database opens");
+    let id = ChallengeId::new("broken", "a");
+    open_challenge(&workspace, cx, &id);
+    assert_eq!(expected(&workspace, cx, &id), Expected::Unavailable);
+}
+
+#[gpui_kit::test]
+fn verdicts_count_as_attempts_and_errors_dont(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    assert_eq!(tally(&workspace, cx), Tally::default());
+    check_first_spin(&workspace, cx, "SELECT nope FROM albums");
+    assert_eq!(tally(&workspace, cx), Tally::default());
+    check_first_spin(&workspace, cx, "SELECT title FROM albums");
+    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
+    assert_eq!(
+        tally(&workspace, cx),
+        Tally {
+            checks: 2,
+            solved: true
+        }
+    );
+}
```

Modify `crates/app/tests/app/check.rs`:
```diff
@@ -33,7 +33,7 @@
 }
 
 /// Writes `sql` in a new First spin tab, checks it with ⌘⇧↵, and waits.
-fn check_first_spin(
+pub fn check_first_spin(
     workspace: &Entity<Workspace>,
     cx: &mut VisualTestContext,
     sql: &str,
```

Modify `crates/app/tests/app/main.rs`:
```diff
@@ -2,6 +2,7 @@
 //!
 //! One test binary, so the helpers in [`support`] are shared.
 
+mod brief;
 mod challenges;
 mod check;
 mod copy;
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo nextest run -p quarry`
Expected: compile errors or failures for the names and behavior above.

- [ ] **Step 3: Implement**

Create `crates/app/src/model/attempts.rs`:
```diff
@@ -0,0 +1,100 @@
+//! How many times each challenge has been checked this session, and
+//! whether it was solved. Phase 2C keeps them across restarts.
+
+use std::collections::BTreeMap;
+
+use quarry_challenges::{
+    catalog::ChallengeId,
+    check::{CheckError, Verdict},
+};
+
+/// One challenge's checks so far.
+#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
+pub struct Tally {
+    /// Checks that gave a verdict, right or wrong.
+    pub checks: u32,
+    /// Whether one of them solved it.
+    pub solved: bool,
+}
+
+/// Every challenge's [`Tally`] this session.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct Attempts {
+    tallies: BTreeMap<ChallengeId, Tally>,
+}
+
+impl Attempts {
+    /// Counts a check of `id` that ended in `outcome`. A check that gave no
+    /// verdict, such as a failing query, doesn't count.
+    pub fn record(&mut self, id: &ChallengeId, outcome: &Result<Verdict, CheckError>) {
+        let Ok(verdict) = outcome else {
+            return;
+        };
+        let tally = self.tallies.entry(id.clone()).or_default();
+        tally.checks = tally.checks.saturating_add(1);
+        tally.solved |= verdict.is_solved();
+    }
+
+    /// `id`'s tally; nothing yet for a challenge never checked.
+    pub fn of(&self, id: &ChallengeId) -> Tally {
+        self.tallies.get(id).copied().unwrap_or_default()
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use quarry_challenges::{
+        catalog::ChallengeId,
+        check::{CheckError, Verdict},
+        rules::{CheckFailure, Grading, RuleOutcome, RuleStatus},
+    };
+
+    use super::{Attempts, Tally};
+
+    fn solved() -> Verdict {
+        Verdict::of(Grading::new(Vec::new()))
+    }
+
+    fn wrong() -> Verdict {
+        let outcome = RuleOutcome {
+            rule: "order",
+            status: RuleStatus::Failed(CheckFailure::Order { row: 0 }),
+        };
+        Verdict::of(Grading::new(vec![outcome]))
+    }
+
+    #[test]
+    fn verdicts_count_and_a_solve_is_kept() {
+        let id = ChallengeId::new("t", "a");
+        let mut attempts = Attempts::default();
+        assert_eq!(attempts.of(&id), Tally::default());
+        attempts.record(&id, &Ok(wrong()));
+        assert_eq!(
+            attempts.of(&id),
+            Tally {
+                checks: 1,
+                solved: false
+            }
+        );
+        attempts.record(&id, &Ok(solved()));
+        attempts.record(&id, &Ok(wrong()));
+        assert_eq!(
+            attempts.of(&id),
+            Tally {
+                checks: 3,
+                solved: true
+            }
+        );
+    }
+
+    #[test]
+    fn errors_dont_count_and_challenges_are_counted_apart() {
+        let (a, b) = (ChallengeId::new("t", "a"), ChallengeId::new("t", "b"));
+        let mut attempts = Attempts::default();
+        attempts.record(&a, &Err(CheckError::TimedOut));
+        assert_eq!(attempts.of(&a), Tally::default());
+        attempts.record(&b, &Ok(wrong()));
+        assert_eq!(attempts.of(&a).checks, 0);
+        assert_eq!(attempts.of(&b).checks, 1);
+    }
+}
```

Create `crates/app/src/model/brief.rs`:
```diff
@@ -0,0 +1,99 @@
+//! What the inspector's brief says about a challenge: where it sits, its
+//! story, what a right answer looks like, and the attempts so far.
+//!
+//! It never shows the expected rows; only their columns and count.
+
+use quarry_challenges::{
+    catalog::Position,
+    check::{CheckError, Expectation},
+    track::{Challenge, Track},
+};
+
+use super::{attempts::Tally, format};
+
+/// What a right answer looks like, as far as it is known.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub enum Expected {
+    /// Still being worked out.
+    #[default]
+    Pending,
+    /// Worked out.
+    Ready(Expectation),
+    /// The solution or its database failed.
+    Unavailable,
+}
+
+impl From<Result<Expectation, CheckError>> for Expected {
+    fn from(result: Result<Expectation, CheckError>) -> Self {
+        result.map_or(Self::Unavailable, Self::Ready)
+    }
+}
+
+/// The brief, line by line.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Brief {
+    /// "Vinyl Vault · Challenge 2 of 5".
+    pub place: String,
+    /// The challenge's title.
+    pub title: String,
+    /// Its story.
+    pub prompt: String,
+    /// The expected columns and row count, or why they aren't shown.
+    pub expected: Vec<String>,
+    /// "Order matters" or "Any order".
+    pub order: &'static str,
+    /// The concepts it practises.
+    pub concepts: Vec<String>,
+    /// "Attempts: 2".
+    pub attempts: String,
+}
+
+/// The brief for `challenge`, challenge `position` of `track`.
+pub fn brief(
+    track: &Track,
+    challenge: &Challenge,
+    position: Position,
+    expected: &Expected,
+    tally: Tally,
+) -> Brief {
+    Brief {
+        place: format!(
+            "{} · Challenge {} of {}",
+            track.title, position.index, position.of
+        ),
+        title: challenge.title.clone(),
+        prompt: challenge.prompt.clone(),
+        expected: expected_lines(expected),
+        order: if challenge.ordered {
+            "Order matters"
+        } else {
+            "Any order"
+        },
+        concepts: challenge.concepts.clone(),
+        attempts: format!("Attempts: {}", tally.checks),
+    }
+}
+
+/// "Expected columns: name, year" and "12 rows".
+fn expected_lines(expected: &Expected) -> Vec<String> {
+    match expected {
+        Expected::Pending => vec!["Working out the expected result…".to_owned()],
+        Expected::Unavailable => vec!["Expected result unavailable".to_owned()],
+        Expected::Ready(expectation) => vec![
+            format!("Expected columns: {}", expectation.columns.join(", ")),
+            rows(expectation.rows),
+        ],
+    }
+}
+
+/// "1 row", "1,200 rows".
+fn rows(n: usize) -> String {
+    let s = if n == 1 { "" } else { "s" };
+    format!(
+        "{} row{s}",
+        format::count(u64::try_from(n).unwrap_or(u64::MAX))
+    )
+}
+
+#[cfg(test)]
+mod tests;
```

Modify `crates/app/src/model/mod.rs`:
```diff
@@ -3,6 +3,8 @@
 //! Nothing here depends on GPUI, so every rule is unit-tested and
 //! mutation-tested like the core crates.
 
+pub mod attempts;
+pub mod brief;
 pub mod closed_tabs;
 pub mod copy;
 pub mod drain;
```

Modify `crates/app/src/tracks.rs`:
```diff
@@ -13,10 +13,14 @@
 };
 use quarry_engine::{error::EngineError, session::Session};
 
-/// The tracks embedded in Quarry, and the templates built so far.
+use crate::model::brief::Expected;
+
+/// The tracks embedded in Quarry, the templates built so far, and what
+/// is known of each challenge's expected result.
 pub struct Tracks {
     catalog: Catalog,
     templates: BTreeMap<String, Arc<Template>>,
+    expectations: BTreeMap<ChallengeId, Expected>,
 }
 
 impl Tracks {
@@ -25,6 +29,7 @@
         Self {
             catalog,
             templates: BTreeMap::new(),
+            expectations: BTreeMap::new(),
         }
     }
 
@@ -64,6 +69,26 @@
         Ok(template)
     }
 
+    /// What a right answer to `id` looks like, as far as it is known.
+    pub fn expected(&self, id: &ChallengeId) -> Expected {
+        self.expectations.get(id).cloned().unwrap_or_default()
+    }
+
+    /// Whether `id`'s expectation still has to be worked out. It is then
+    /// marked as being worked out, so it is asked for once.
+    pub fn start_expecting(&mut self, id: &ChallengeId) -> bool {
+        if self.expectations.contains_key(id) {
+            return false;
+        }
+        self.expectations.insert(id.clone(), Expected::Pending);
+        true
+    }
+
+    /// Records what is known of `id`'s expectation.
+    pub fn set_expected(&mut self, id: &ChallengeId, expected: Expected) {
+        self.expectations.insert(id.clone(), expected);
+    }
+
     /// A session on a fresh copy of the template of `id`'s track.
     ///
     /// # Errors
@@ -73,3 +98,30 @@
         Ok(Session::start(Box::new(copy)))
     }
 }
+
+#[cfg(test)]
+mod tests {
+    use quarry_challenges::{
+        catalog::{Catalog, ChallengeId},
+        check::Expectation,
+    };
+
+    use super::Tracks;
+    use crate::model::brief::Expected;
+
+    #[test]
+    fn an_expectation_is_asked_for_once_and_kept() {
+        let mut tracks = Tracks::new(Catalog::default());
+        let id = ChallengeId::new("t", "a");
+        assert_eq!(tracks.expected(&id), Expected::Pending);
+        assert!(tracks.start_expecting(&id));
+        assert!(!tracks.start_expecting(&id));
+        let ready = Expected::Ready(Expectation {
+            columns: vec!["n".to_owned()],
+            rows: 1,
+        });
+        tracks.set_expected(&id, ready.clone());
+        assert_eq!(tracks.expected(&id), ready);
+        assert!(!tracks.start_expecting(&id));
+    }
+}
```

Modify `crates/app/src/views/query_tab/check.rs`:
```diff
@@ -11,7 +11,7 @@
     track::Challenge,
 };
 
-use super::QueryTab;
+use super::{QueryTab, QueryTabEvent};
 use crate::{model::verdict::CheckState, views::results::CHECK};
 
 impl QueryTab {
@@ -36,6 +36,7 @@
                 )
                 .await;
             let _ = this.update(cx, |tab, cx| {
+                cx.emit(QueryTabEvent::Checked(outcome.clone()));
                 tab.check = CheckState::Done(outcome);
                 cx.notify();
             });
```

Modify `crates/app/src/views/query_tab/mod.rs`:
```diff
@@ -18,7 +18,10 @@
         table::TableState,
     },
 };
-use quarry_challenges::run::Stop;
+use quarry_challenges::{
+    check::{CheckError, Verdict},
+    run::Stop,
+};
 use quarry_engine::session::{RunEvent, RunHandle};
 
 use super::{
@@ -41,6 +44,8 @@
     Edited,
     /// A copy finished; the note says what it copied.
     Copied(String),
+    /// A check finished with this outcome.
+    Checked(Result<Verdict, CheckError>),
 }
 
 /// What a run was asked to run.
```

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -4,24 +4,32 @@
 
 use std::collections::BTreeMap;
 
-use gpui_kit::{Context, Window};
-use quarry_challenges::catalog::ChallengeId;
-use quarry_challenges::track::Challenge;
+use gpui_kit::{AppContext, Context, Window};
+use quarry_challenges::{
+    catalog::ChallengeId,
+    check::{self, CheckError, Verdict},
+    track::Challenge,
+};
 use quarry_engine::session::Session;
 use quarry_store::drafts::Draft;
 
 use super::Workspace;
 use crate::{
-    model::tabs::TabId, shell::actions::CheckAnswer, tracks::Tracks, views::query_tab::QueryTab,
+    model::{attempts::Attempts, brief::Expected, tabs::TabId},
+    shell::actions::CheckAnswer,
+    tracks::Tracks,
+    views::query_tab::QueryTab,
 };
 
 /// What ⌘⇧↵ says in a tab that isn't a challenge tab.
 pub const NOT_A_CHALLENGE: &str = "Check answer works in challenge tabs";
 
-/// The tracks, and the sandbox of every open challenge tab.
+/// The tracks, the sandbox of every open challenge tab, and this
+/// session's attempts.
 pub(super) struct Challenges {
     tracks: Tracks,
     sandboxes: BTreeMap<TabId, Sandbox>,
+    attempts: Attempts,
 }
 
 impl Challenges {
@@ -38,6 +46,7 @@
         Self {
             tracks,
             sandboxes: BTreeMap::new(),
+            attempts: Attempts::default(),
         }
     }
 }
@@ -151,19 +160,71 @@
 
     /// The active challenge's track slug and challenge.
     fn active_challenge_details(&self) -> Option<(String, Challenge)> {
-        let id = self.active_challenge()?;
+        self.details(self.active_challenge()?)
+    }
+
+    /// Challenge `id`'s track slug, and the challenge.
+    fn details(&self, id: &ChallengeId) -> Option<(String, Challenge)> {
         let (track, challenge) = self.challenges.tracks.challenge(id)?;
         Some((track.slug.clone(), challenge.clone()))
     }
 
-    /// Makes `tab` a challenge tab on `sandbox`, with the Check view.
+    /// Makes `tab` a challenge tab on `sandbox`, with the Check view, and
+    /// starts working out its challenge's expected result.
     fn attach(&mut self, tab: TabId, sandbox: Sandbox, cx: &mut Context<Self>) {
+        self.work_out_expectation(&sandbox.challenge, cx);
         self.challenges.sandboxes.insert(tab, sandbox);
         if let Some(view) = self.views.get(&tab) {
             view.update(cx, QueryTab::play_challenge);
         }
     }
 
+    /// Works out what a right answer to `id` looks like on the background
+    /// executor, unless that has already started.
+    fn work_out_expectation(&mut self, id: &ChallengeId, cx: &mut Context<Self>) {
+        if !self.challenges.tracks.start_expecting(id) {
+            return;
+        }
+        let Some((slug, challenge)) = self.details(id) else {
+            return;
+        };
+        let Ok(template) = self.challenges.tracks.template(&slug) else {
+            self.challenges
+                .tracks
+                .set_expected(id, Expected::Unavailable);
+            return;
+        };
+        let id = id.clone();
+        cx.spawn(async move |this, cx| {
+            let result = cx
+                .background_spawn(async move { check::expect(&template, &challenge) })
+                .await;
+            let _ = this.update(cx, |workspace, cx| {
+                workspace.challenges.tracks.set_expected(&id, result.into());
+                cx.notify();
+            });
+        })
+        .detach();
+    }
+
+    /// Counts tab `tab`'s finished check as an attempt at its challenge.
+    pub(super) fn checked(
+        &mut self,
+        tab: TabId,
+        outcome: &Result<Verdict, CheckError>,
+        cx: &mut Context<Self>,
+    ) {
+        if let Some(id) = self.challenge(tab).cloned() {
+            self.challenges.attempts.record(&id, outcome);
+            cx.notify();
+        }
+    }
+
+    /// This session's attempts.
+    pub fn attempts(&self) -> &Attempts {
+        &self.challenges.attempts
+    }
+
     /// The open tab playing challenge `id`.
     pub(super) fn challenge_tab(&self, id: &ChallengeId) -> Option<TabId> {
         self.challenges
```

Modify `crates/app/src/views/workspace/tabs.rs`:
```diff
@@ -21,10 +21,11 @@
         cx: &mut Context<Self>,
     ) {
         let view = cx.new(|cx| QueryTab::new(text, window, cx));
-        cx.subscribe(&view, |workspace, _, event, cx| match event {
+        cx.subscribe(&view, move |workspace, _, event, cx| match event {
             QueryTabEvent::RunEnded => workspace.read_schema(cx),
             QueryTabEvent::Edited => workspace.schedule_save(cx),
             QueryTabEvent::Copied(note) => workspace.show_note(note.clone(), cx),
+            QueryTabEvent::Checked(outcome) => workspace.checked(id, outcome, cx),
         })
         .detach();
         self.views.insert(id, view);
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p quarry`
Expected: all pass.

- [ ] **Step 5: Run the gates**

Run: `just fmt lint quality typos complexity && cargo nextest run --workspace`
Expected: no violations, and `501` tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/model/attempts.rs crates/app/src/model/brief.rs crates/app/src/model/brief/tests.rs crates/app/src/model/mod.rs crates/app/src/tracks.rs crates/app/src/views/query_tab/check.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/app/brief.rs crates/app/tests/app/check.rs crates/app/tests/app/main.rs
git commit -m "feat(app): count attempts and work out each challenge's expected result"
```

---

### Task 4: App: the inspector, with the brief and ⌘⌥I

`InspectorRegistry` picks the first `InspectorView` that applies: `BriefView` in a challenge tab, `EmptyView` elsewhere. The workspace gains a fourth column, `ToggleInspector` on ⌘⌥I, and `Drafts::inspector_hidden`, saved in the store's `state` table with no migration.

**Files:**
- Modify: `crates/app/src/shell/actions.rs`
- Create: `crates/app/src/views/inspector/brief.rs`
- Create: `crates/app/src/views/inspector/empty.rs`
- Create: `crates/app/src/views/inspector/mod.rs`
- Modify: `crates/app/src/views/mod.rs`
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/drafts.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/tests/app/brief.rs`
- Modify: `crates/app/tests/app/challenges.rs`
- Modify: `crates/app/tests/app/tabs.rs`
- Modify: `crates/app/tests/shell.rs`
- Modify: `crates/store/src/drafts.rs`
- Modify: `crates/store/src/sqlite.rs`
- Modify: `crates/store/tests/drafts.rs`

**Interfaces:**
- Consumes: `Workspace::active_challenge`, `model::brief`, `Drafts`.
- Produces: `views::inspector::{InspectorView, InspectorRegistry, BRIEF, EMPTY}`, `Workspace::{active_brief, inspector_open}`, `INSPECTOR_WIDTH`, `ToggleInspector`, `Drafts::inspector_hidden`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/brief.rs`:
```diff
@@ -6,15 +6,18 @@
 use gpui_kit::{Entity, TestAppContext, VisualTestContext};
 use quarry::{
     model::{attempts::Tally, brief::Expected},
-    views::workspace::Workspace,
+    views::{inspector, workspace::Workspace},
 };
 use quarry_challenges::{catalog::ChallengeId, check::Expectation};
-use quarry_store::drafts::Forgetful;
+use quarry_store::{
+    drafts::{DraftStore, Drafts, Forgetful},
+    sqlite::SqliteStore,
+};
 
 use super::{
     challenges::open_challenge,
     check::check_first_spin,
-    support::{broken_tracks, install, open, open_on, settle},
+    support::{broken_tracks, drawn, install, open, open_on, open_with, settle},
 };
 
 fn first_spin() -> ChallengeId {
@@ -76,3 +79,66 @@
         }
     );
 }
+
+#[gpui_kit::test]
+fn the_inspector_shows_the_brief_in_a_challenge_tab_and_a_note_elsewhere(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    assert!(drawn(cx, inspector::EMPTY));
+    assert!(!drawn(cx, inspector::BRIEF));
+    open_challenge(&workspace, cx, &first_spin());
+    assert!(drawn(cx, inspector::BRIEF));
+    assert!(!drawn(cx, inspector::EMPTY));
+    let brief = cx
+        .update(|_, cx| workspace.read(cx).active_brief())
+        .expect("a brief");
+    assert_eq!(brief.place, "Vinyl Vault · Challenge 1 of 5");
+    assert_eq!(brief.title, "First spin");
+}
+
+#[gpui_kit::test]
+fn the_brief_fills_in_the_expected_result_and_the_attempts(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    open_challenge(&workspace, cx, &first_spin());
+    expected(&workspace, cx, &first_spin());
+    check_first_spin(&workspace, cx, "SELECT title FROM albums");
+    let brief = cx
+        .update(|_, cx| workspace.read(cx).active_brief())
+        .expect("a brief");
+    assert_eq!(brief.expected, ["Expected columns: title, year", "18 rows"]);
+    assert_eq!(brief.attempts, "Attempts: 1");
+}
+
+#[gpui_kit::test]
+fn cmd_alt_i_hides_and_shows_the_inspector_and_the_choice_is_saved(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    let state = |cx: &mut VisualTestContext| {
+        cx.update(|_, cx| {
+            let workspace = workspace.read(cx);
+            (
+                workspace.inspector_open(),
+                workspace.snapshot(cx).inspector_hidden,
+            )
+        })
+    };
+    assert_eq!(state(cx), (true, false));
+    assert!(drawn(cx, "inspector"));
+    cx.simulate_keystrokes("cmd-alt-i");
+    assert_eq!(state(cx), (false, true));
+    assert!(!drawn(cx, "inspector"));
+    cx.simulate_keystrokes("cmd-alt-i");
+    assert_eq!(state(cx), (true, false));
+}
+
+#[gpui_kit::test]
+fn a_hidden_inspector_stays_hidden_after_a_restart(cx: &mut TestAppContext) {
+    install(cx);
+    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
+    let hidden = Drafts {
+        inspector_hidden: true,
+        ..Drafts::default()
+    };
+    store.save(&hidden).expect("a save");
+    let (workspace, cx) = open_with(cx, store).expect("the sample database opens");
+    assert!(!cx.update(|_, cx| workspace.read(cx).inspector_open()));
+    assert!(!drawn(cx, "inspector"));
+}
```

Modify `crates/app/tests/app/challenges.rs`:
```diff
@@ -163,6 +163,7 @@
         active: tabs.len() - 1,
         tabs,
         closed: Vec::new(),
+        inspector_hidden: false,
     };
     store.save(&drafts)?;
     Ok(store)
```

Modify `crates/app/tests/app/tabs.rs`:
```diff
@@ -195,6 +195,7 @@
         tabs: vec![draft(1, "Query 1", "SELECT 1")],
         active: 0,
         closed: closed.clone(),
+        inspector_hidden: false,
     };
     store.save(&saved).expect("a first save");
     let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
@@ -214,6 +215,7 @@
             tabs: vec![draft(1, "Query 1", "")],
             active: 0,
             closed: Vec::new(),
+            inspector_hidden: false,
         })
         .expect("a first save");
     let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
@@ -335,6 +337,7 @@
         tabs: vec![draft(1, "Query 1", text)],
         active: 0,
         closed: Vec::new(),
+        inspector_hidden: false,
     };
     assert_eq!(
         save_in_order(&store, &saves, newer, &snapshot("newer")),
```

Modify `crates/app/tests/shell.rs`:
```diff
@@ -42,7 +42,7 @@
         .collect();
     let unbound = ["quarry::CopyCsv", "quarry::CopyJson"];
     let commands = actions::commands();
-    assert_eq!(commands.len(), 18);
+    assert_eq!(commands.len(), 19);
     for command in commands {
         let name = (command.action)().name();
         assert!(
```

Modify `crates/store/tests/drafts.rs`:
```diff
@@ -33,6 +33,7 @@
             draft(0, "Notes", "-- newest"),
             draft(0, "Query 4", "SELECT 4;"),
         ],
+        inspector_hidden: true,
     }
 }
 
@@ -57,6 +58,7 @@
         tabs: vec![draft(9, "Query 5", "")],
         active: 0,
         closed: Vec::new(),
+        inspector_hidden: false,
     };
     store.save(&fewer).unwrap();
     assert_eq!(store.load().unwrap(), fewer);
@@ -165,6 +167,7 @@
             tabs: vec![draft(5, "Query 1", "SELECT 1;")],
             active: 0,
             closed: Vec::new(),
+            inspector_hidden: false,
         }
     );
 }
@@ -179,6 +182,7 @@
         ],
         active: 0,
         closed: vec![challenge_draft(0, "Every genre", "vinyl-vault/every-genre")],
+        inspector_hidden: false,
     };
     store.save(&drafts).unwrap();
     assert_eq!(store.load().unwrap(), drafts);
@@ -215,3 +219,19 @@
     store.save(&drafts).unwrap();
     assert_eq!(store.load().unwrap().active, 0);
 }
+
+#[test]
+fn a_state_value_that_isnt_a_number_is_reported() {
+    let dir = tempfile::tempdir().unwrap();
+    let path = dir.path().join("quarry.db");
+    SqliteStore::open(&path).unwrap().save(&sample()).unwrap();
+    rusqlite::Connection::open(&path)
+        .unwrap()
+        .execute(
+            "UPDATE state SET value = 'hidden' WHERE key = 'inspector_hidden'",
+            [],
+        )
+        .unwrap();
+    let loaded = SqliteStore::open(&path).unwrap().load();
+    assert!(matches!(loaded, Err(StoreError::Sql(_))), "{loaded:?}");
+}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo nextest run -p quarry-store -p quarry`
Expected: compile errors or failures for the names and behavior above.

- [ ] **Step 3: Implement**

Modify `crates/app/src/shell/actions.rs`:
```diff
@@ -29,6 +29,8 @@
         PreviousTab,
         /// Shows or hides the sidebar.
         ToggleSidebar,
+        /// Shows or hides the inspector.
+        ToggleInspector,
         /// Moves keyboard focus to the sidebar's first table.
         FocusSidebar,
         /// Moves keyboard focus to the editor.
@@ -78,6 +80,7 @@
         KeyBinding::new("cmd-}", NextTab, None),
         KeyBinding::new("cmd-{", PreviousTab, None),
         KeyBinding::new("cmd-\\", ToggleSidebar, None),
+        KeyBinding::new("cmd-alt-i", ToggleInspector, None),
         KeyBinding::new("cmd-1", FocusSidebar, None),
         KeyBinding::new("cmd-2", FocusEditor, None),
         KeyBinding::new("cmd-shift-t", CycleTheme, None),
@@ -125,6 +128,7 @@
         command("Next Tab", || Box::new(NextTab), "View"),
         command("Previous Tab", || Box::new(PreviousTab), "View"),
         command("Toggle Sidebar", || Box::new(ToggleSidebar), "View"),
+        command("Toggle Inspector", || Box::new(ToggleInspector), "View"),
         command("Focus Sidebar", || Box::new(FocusSidebar), "View"),
         command("Focus Editor", || Box::new(FocusEditor), "View"),
         command("Cycle Theme", || Box::new(CycleTheme), "View"),
```

Create `crates/app/src/views/inspector/brief.rs`:
```diff
@@ -0,0 +1,127 @@
+//! The challenge brief: where the challenge sits, its story, what a right
+//! answer looks like, and the attempts so far.
+
+use gpui_kit::{
+    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
+    StatefulInteractiveElement, Styled, TestSupportExt, div, px,
+};
+use quarry_ui_kit::{
+    Aurora, Role,
+    components::badge::{Badge, Tone},
+    palette::Colors,
+    tokens::{space, text},
+};
+
+use super::{BRIEF, InspectorView};
+use crate::{model::brief::Brief, views::workspace::Workspace};
+
+/// The id of the brief's title.
+pub const TITLE: &str = "brief-title";
+
+/// The active challenge's brief.
+pub struct BriefView;
+
+impl InspectorView for BriefView {
+    fn id(&self) -> &'static str {
+        BRIEF
+    }
+
+    fn applies(&self, workspace: &Workspace) -> bool {
+        workspace.active_challenge().is_some()
+    }
+
+    fn render(&self, workspace: &Workspace, cx: &Context<Workspace>) -> AnyElement {
+        let colors = Aurora::colors(cx);
+        let root = div()
+            .id(BRIEF)
+            .size_full()
+            .overflow_y_scroll()
+            .flex()
+            .flex_col()
+            .gap(px(space::MD))
+            .text_size(px(text::BODY));
+        let Some(brief) = workspace.active_brief() else {
+            return root.test_support().into_any_element();
+        };
+        root.child(story(&brief, &colors))
+            .child(expected(&brief, &colors))
+            .child(concepts(&brief))
+            .child(footer(&brief, &colors))
+            .test_support()
+            .into_any_element()
+    }
+}
+
+/// The place, the title and the prompt.
+fn story(brief: &Brief, colors: &Colors) -> AnyElement {
+    div()
+        .flex()
+        .flex_col()
+        .gap(px(space::XS))
+        .child(caption(brief.place.clone(), colors))
+        .child(
+            div()
+                .id(TITLE)
+                .text_size(px(text::TITLE))
+                .text_color(colors.get(Role::Text))
+                .child(brief.title.clone())
+                .test_support(),
+        )
+        .child(
+            div()
+                .text_color(colors.get(Role::TextSecondary))
+                .child(brief.prompt.clone()),
+        )
+        .into_any_element()
+}
+
+/// The expected columns and row count, and whether order matters.
+fn expected(brief: &Brief, colors: &Colors) -> AnyElement {
+    let line = |text: String| {
+        div()
+            .text_color(colors.get(Role::TextSecondary))
+            .child(text)
+    };
+    div()
+        .flex()
+        .flex_col()
+        .gap(px(space::XS))
+        .child(caption("EXPECTED".to_owned(), colors))
+        .children(brief.expected.iter().cloned().map(line))
+        .child(line(brief.order.to_owned()))
+        .into_any_element()
+}
+
+/// The concept tags.
+fn concepts(brief: &Brief) -> AnyElement {
+    div()
+        .flex()
+        .flex_wrap()
+        .gap(px(space::XS))
+        .children(
+            brief
+                .concepts
+                .iter()
+                .map(|concept| Badge::new(concept.clone(), Tone::Neutral)),
+        )
+        .into_any_element()
+}
+
+/// The attempts so far, at the bottom.
+fn footer(brief: &Brief, colors: &Colors) -> AnyElement {
+    div()
+        .mt_auto()
+        .text_size(px(text::SMALL))
+        .text_color(colors.get(Role::TextMuted))
+        .child(brief.attempts.clone())
+        .into_any_element()
+}
+
+/// A small upper-case section label.
+fn caption(label: String, colors: &Colors) -> AnyElement {
+    div()
+        .text_size(px(text::CAPTION))
+        .text_color(colors.get(Role::TextMuted))
+        .child(label)
+        .into_any_element()
+}
```

Create `crates/app/src/views/inspector/empty.rs`:
```diff
@@ -0,0 +1,51 @@
+//! What the inspector shows in a tab that isn't a challenge tab.
+
+use gpui_kit::{
+    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, TestSupportExt,
+    div, px,
+};
+use quarry_ui_kit::{
+    Aurora, Role,
+    tokens::{space, text},
+};
+
+use super::{EMPTY, InspectorView};
+use crate::views::workspace::Workspace;
+
+/// "No challenge in this tab".
+pub struct EmptyView;
+
+impl InspectorView for EmptyView {
+    fn id(&self) -> &'static str {
+        EMPTY
+    }
+
+    fn applies(&self, _: &Workspace) -> bool {
+        true
+    }
+
+    fn render(&self, _: &Workspace, cx: &Context<Workspace>) -> AnyElement {
+        let colors = Aurora::colors(cx);
+        div()
+            .id(EMPTY)
+            .size_full()
+            .flex()
+            .flex_col()
+            .items_center()
+            .justify_center()
+            .gap(px(space::XS))
+            .text_size(px(text::BODY))
+            .child(
+                div()
+                    .text_color(colors.get(Role::TextSecondary))
+                    .child("No challenge in this tab"),
+            )
+            .child(
+                div()
+                    .text_color(colors.get(Role::TextMuted))
+                    .child("Press ⌘K to find one"),
+            )
+            .test_support()
+            .into_any_element()
+    }
+}
```

Create `crates/app/src/views/inspector/mod.rs`:
```diff
@@ -0,0 +1,51 @@
+//! The inspector, the workspace's right column: the challenge brief in a
+//! challenge tab, and an empty state elsewhere.
+//!
+//! Each is an [`InspectorView`] in the [`InspectorRegistry`]; adding one is
+//! a new module and one line in [`InspectorRegistry::standard`].
+
+mod brief;
+mod empty;
+
+use gpui_kit::{AnyElement, Context};
+
+use super::workspace::Workspace;
+
+/// The id of the brief.
+pub const BRIEF: &str = "brief";
+/// The id of the empty state.
+pub const EMPTY: &str = "inspector-empty";
+
+/// One thing the inspector can show.
+pub trait InspectorView {
+    /// A stable name, such as "brief"; its root element has this id.
+    fn id(&self) -> &'static str;
+
+    /// Whether it has something to show for the workspace's active tab.
+    fn applies(&self, workspace: &Workspace) -> bool;
+
+    /// Draws it for the workspace's active tab.
+    fn render(&self, workspace: &Workspace, cx: &Context<Workspace>) -> AnyElement;
+}
+
+/// The inspector's views, most specific first.
+pub struct InspectorRegistry {
+    views: Vec<Box<dyn InspectorView>>,
+}
+
+impl InspectorRegistry {
+    /// The brief, then the empty state.
+    pub fn standard() -> Self {
+        Self {
+            views: vec![Box::new(brief::BriefView), Box::new(empty::EmptyView)],
+        }
+    }
+
+    /// The first view that applies to the workspace's active tab.
+    pub fn pick(&self, workspace: &Workspace) -> Option<&dyn InspectorView> {
+        self.views
+            .iter()
+            .find(|view| view.applies(workspace))
+            .map(AsRef::as_ref)
+    }
+}
```

Modify `crates/app/src/views/mod.rs`:
```diff
@@ -1,6 +1,7 @@
 //! GPUI views: each renders one region of the workspace.
 
 pub mod editor;
+pub mod inspector;
 pub mod palette;
 pub mod poll;
 pub mod query_tab;
```

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -15,7 +15,11 @@
 
 use super::Workspace;
 use crate::{
-    model::{attempts::Attempts, brief::Expected, tabs::TabId},
+    model::{
+        attempts::Attempts,
+        brief::{self, Brief, Expected},
+        tabs::TabId,
+    },
     shell::actions::CheckAnswer,
     tracks::Tracks,
     views::query_tab::QueryTab,
@@ -220,6 +224,17 @@
         }
     }
 
+    /// The active challenge's brief, if the active tab is a challenge tab.
+    pub fn active_brief(&self) -> Option<Brief> {
+        let id = self.active_challenge()?;
+        let tracks = &self.challenges.tracks;
+        let (track, challenge) = tracks.challenge(id)?;
+        let position = tracks.catalog().position(id)?;
+        let expected = tracks.expected(id);
+        let tally = self.challenges.attempts.of(id);
+        Some(brief::brief(track, challenge, position, &expected, tally))
+    }
+
     /// This session's attempts.
     pub fn attempts(&self) -> &Attempts {
         &self.challenges.attempts
```

Modify `crates/app/src/views/workspace/drafts.rs`:
```diff
@@ -85,6 +85,7 @@
             tabs,
             active: self.tabs.active(),
             closed: self.closed.clone(),
+            inspector_hidden: !self.inspector_open,
         }
     }
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -16,7 +16,7 @@
     tokens::{chrome, space, text},
 };
 
-use super::{SIDEBAR_WIDTH, Workspace};
+use super::{INSPECTOR_WIDTH, SIDEBAR_WIDTH, Workspace};
 use crate::{
     model::{format, status},
     shell::actions::WORKSPACE,
@@ -42,6 +42,7 @@
             .on_action(cx.listener(Self::next_tab))
             .on_action(cx.listener(Self::previous_tab))
             .on_action(cx.listener(Self::toggle_sidebar))
+            .on_action(cx.listener(Self::toggle_inspector))
             .on_action(cx.listener(Self::focus_sidebar))
             .on_action(cx.listener(Self::focus_editor))
             .on_action(cx.listener(Self::cycle_theme))
@@ -55,6 +56,7 @@
             .text_color(colors.get(Role::Text))
             .when(self.sidebar_open, |root| root.child(self.sidebar(cx)))
             .child(self.center(cx))
+            .when(self.inspector_open, |root| root.child(self.inspector(cx)))
     }
 }
 
@@ -83,6 +85,23 @@
             .into_any_element()
     }
 
+    /// The right column: the first inspector view that applies.
+    fn inspector(&self, cx: &Context<Self>) -> AnyElement {
+        let colors = Aurora::colors(cx);
+        let view = self.inspector.pick(self);
+        sidebar_surface(&colors)
+            .id("inspector")
+            .w(px(INSPECTOR_WIDTH))
+            .flex_none()
+            .h_full()
+            .pt(px(chrome::TITLEBAR_HEIGHT))
+            .px(px(space::MD))
+            .pb(px(space::MD))
+            .children(view.map(|view| view.render(self, cx)))
+            .test_support()
+            .into_any_element()
+    }
+
     fn center(&self, cx: &Context<Self>) -> AnyElement {
         div()
             .flex_1()
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -38,6 +38,7 @@
 };
 use super::{
     editor,
+    inspector::InspectorRegistry,
     palette::{PaletteSourceRegistry, dialog::Palette},
     query_tab::QueryTab,
     schema::SchemaTree,
@@ -48,7 +49,7 @@
         statements::RunOutput,
         tabs::{Tab, TabId, TabList},
     },
-    shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
+    shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleInspector, ToggleSidebar},
     tracks::Tracks,
 };
 
@@ -58,6 +59,9 @@
 /// Width of the open sidebar.
 pub const SIDEBAR_WIDTH: f32 = 240.0;
 
+/// Width of the open inspector.
+pub const INSPECTOR_WIDTH: f32 = 280.0;
+
 /// What a workspace runs on: a session for plain tabs, named in the
 /// status bar, and the challenge tracks.
 pub struct Databases {
@@ -87,6 +91,8 @@
     saves: Arc<Mutex<SaveOrder>>,
     schema: Entity<SchemaTree>,
     sidebar_open: bool,
+    inspector_open: bool,
+    inspector: Rc<InspectorRegistry>,
     focus: FocusHandle,
     sidebar_focus: FocusHandle,
     schema_read: Option<Task<()>>,
@@ -122,6 +128,8 @@
             saves: Arc::default(),
             schema: cx.new(|_| SchemaTree::default()),
             sidebar_open: true,
+            inspector_open: !drafts.inspector_hidden,
+            inspector: Rc::new(InspectorRegistry::standard()),
             focus: cx.focus_handle(),
             sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
             schema_read: None,
@@ -191,6 +199,18 @@
         cx.notify();
     }
 
+    /// Whether the inspector is shown.
+    pub fn inspector_open(&self) -> bool {
+        self.inspector_open
+    }
+
+    /// Shows or hides the inspector, and saves the choice.
+    fn toggle_inspector(&mut self, _: &ToggleInspector, _: &mut Window, cx: &mut Context<Self>) {
+        self.inspector_open = !self.inspector_open;
+        self.schedule_save(cx);
+        cx.notify();
+    }
+
     /// Focuses the sidebar's first table. A closed sidebar opens first, and
     /// focus moves once it has been drawn and its rows are tab stops.
     fn focus_sidebar(&mut self, _: &FocusSidebar, window: &mut Window, cx: &mut Context<Self>) {
```

Modify `crates/store/src/drafts.rs`:
```diff
@@ -16,8 +16,8 @@
     pub challenge: Option<String>,
 }
 
-/// Every open tab in order, which one is active, and the recently closed
-/// tabs.
+/// Every open tab in order, which one is active, the recently closed
+/// tabs, and whether the inspector was hidden.
 #[derive(Debug, Clone, Default, PartialEq, Eq)]
 pub struct Drafts {
     /// The tabs, left to right.
@@ -27,6 +27,8 @@
     /// Recently closed tabs, newest first. Their ids are not kept: each
     /// loads with id 0.
     pub closed: Vec<Draft>,
+    /// Whether the inspector was hidden; it shows by default.
+    pub inspector_hidden: bool,
 }
 
 /// Loads and saves the drafts.
@@ -75,6 +77,7 @@
             }],
             active: 0,
             closed: Vec::new(),
+            inspector_hidden: false,
         };
         store.save(&drafts)?;
         assert_eq!(store.load()?, Drafts::default());
```

Modify `crates/store/src/sqlite.rs`:
```diff
@@ -95,12 +95,27 @@
 
 /// The saved active index, or 0 when none was saved.
 fn load_active(connection: &Connection) -> Result<usize, StoreError> {
-    let active: Option<i64> = connection
-        .query_row("SELECT value FROM state WHERE key = 'active'", [], |row| {
+    let active = load_state(connection, "active")?;
+    Ok(usize::try_from(active.unwrap_or(0)).unwrap_or(0))
+}
+
+/// The value saved under `key` in `state`, if one was.
+fn load_state(connection: &Connection, key: &str) -> Result<Option<i64>, StoreError> {
+    let value = connection
+        .query_row("SELECT value FROM state WHERE key = ?1", [key], |row| {
             row.get(0)
         })
         .optional()?;
-    Ok(usize::try_from(active.unwrap_or(0)).unwrap_or(0))
+    Ok(value)
+}
+
+/// Saves `value` under `key` in `state`.
+fn save_state(connection: &Connection, key: &str, value: i64) -> Result<(), StoreError> {
+    connection.execute(
+        "INSERT OR REPLACE INTO state (key, value) VALUES (?1, ?2)",
+        params![key, value],
+    )?;
+    Ok(())
 }
 
 /// Replaces the open tabs.
@@ -144,6 +159,7 @@
             active: active.min(tabs.len().saturating_sub(1)),
             tabs,
             closed: load_closed(&connection)?,
+            inspector_hidden: load_state(&connection, "inspector_hidden")? == Some(1),
         })
     }
 
@@ -152,9 +168,11 @@
         let transaction = connection.transaction()?;
         save_tabs(&transaction, &drafts.tabs)?;
         save_closed(&transaction, &drafts.closed)?;
-        transaction.execute(
-            "INSERT OR REPLACE INTO state (key, value) VALUES ('active', ?1)",
-            params![index(drafts.active)],
+        save_state(&transaction, "active", index(drafts.active))?;
+        save_state(
+            &transaction,
+            "inspector_hidden",
+            i64::from(drafts.inspector_hidden),
         )?;
         transaction.commit()?;
         Ok(())
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p quarry-store -p quarry`
Expected: all pass.

- [ ] **Step 5: Run the gates**

Run: `just fmt lint quality typos complexity && cargo nextest run --workspace`
Expected: no violations, and `506` tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/shell/actions.rs crates/app/src/views/inspector/brief.rs crates/app/src/views/inspector/empty.rs crates/app/src/views/inspector/mod.rs crates/app/src/views/mod.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/drafts.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/tests/app/brief.rs crates/app/tests/app/challenges.rs crates/app/tests/app/tabs.rs crates/app/tests/shell.rs crates/store/src/drafts.rs crates/store/src/sqlite.rs crates/store/tests/drafts.rs
git commit -m "feat(app): the inspector, with the challenge brief and ⌘⌥I"
```

---

### Task 5: App: Prev and Next

`PreviousChallenge` (⌘⌥[) and `NextChallenge` (⌘⌥]) open or focus the neighbouring challenge's tab through `open_challenge`, across tracks. The brief's footer shows ‹ Prev, the attempts and Next ›; with nowhere to go, a button is a muted label.

**Files:**
- Modify: `crates/app/src/shell/actions.rs`
- Modify: `crates/app/src/views/inspector/brief.rs`
- Modify: `crates/app/src/views/inspector/mod.rs`
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/tests/app/brief.rs`
- Modify: `crates/app/tests/app/challenges.rs`
- Modify: `crates/app/tests/app/support.rs`
- Modify: `crates/app/tests/shell.rs`

**Interfaces:**
- Consumes: `Catalog::neighbours`, `Workspace::open_challenge`.
- Produces: `Workspace::{active_neighbours, open_previous_challenge, open_next_challenge}`, `PreviousChallenge`, `NextChallenge`, `inspector::{PREV, NEXT}`; test helpers `support::{raw_track, two_tracks}`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/brief.rs`:
```diff
@@ -3,7 +3,7 @@
 
 use std::sync::Arc;
 
-use gpui_kit::{Entity, TestAppContext, VisualTestContext};
+use gpui_kit::{Entity, TestAppContext, VisualTestContext, test::TestWindowExt};
 use quarry::{
     model::{attempts::Tally, brief::Expected},
     views::{inspector, workspace::Workspace},
@@ -17,7 +17,7 @@
 use super::{
     challenges::open_challenge,
     check::check_first_spin,
-    support::{broken_tracks, drawn, install, open, open_on, open_with, settle},
+    support::{broken_tracks, drawn, install, open, open_on, open_with, settle, two_tracks},
 };
 
 fn first_spin() -> ChallengeId {
@@ -142,3 +142,54 @@
     assert!(!cx.update(|_, cx| workspace.read(cx).inspector_open()));
     assert!(!drawn(cx, "inspector"));
 }
+
+fn active(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<ChallengeId> {
+    cx.update(|_, cx| workspace.read(cx).active_challenge().cloned())
+}
+
+fn tab_count(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> usize {
+    cx.update(|_, cx| workspace.read(cx).tabs().tabs().len())
+}
+
+#[gpui_kit::test]
+fn next_and_previous_run_through_a_track_and_on_to_the_next(cx: &mut TestAppContext) {
+    install(cx);
+    let (workspace, cx) =
+        open_on(cx, Arc::new(Forgetful), two_tracks()).expect("the sample database opens");
+    open_challenge(&workspace, cx, &ChallengeId::new("one", "a"));
+    cx.simulate_keystrokes("cmd-alt-[");
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("one/a"));
+    cx.simulate_keystrokes("cmd-alt-]");
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("one/b"));
+    cx.simulate_keystrokes("cmd-alt-]");
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("two/c"));
+    cx.simulate_keystrokes("cmd-alt-]");
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("two/c"));
+    assert_eq!(tab_count(&workspace, cx), 4);
+    cx.simulate_keystrokes("cmd-alt-[ cmd-alt-[");
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("one/a"));
+    assert_eq!(tab_count(&workspace, cx), 4);
+}
+
+#[gpui_kit::test]
+fn the_briefs_buttons_move_and_stop_at_the_ends(cx: &mut TestAppContext) {
+    install(cx);
+    let (workspace, cx) =
+        open_on(cx, Arc::new(Forgetful), two_tracks()).expect("the sample database opens");
+    open_challenge(&workspace, cx, &ChallengeId::new("one", "a"));
+    assert!(drawn(cx, inspector::PREV));
+    cx.update(|window, cx| window.click(inspector::PREV, cx));
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("one/a"));
+    cx.update(|window, cx| window.click(inspector::NEXT, cx));
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("one/b"));
+    cx.update(|window, cx| window.click(inspector::PREV, cx));
+    assert_eq!(active(&workspace, cx), ChallengeId::parse("one/a"));
+}
+
+#[gpui_kit::test]
+fn next_does_nothing_in_a_plain_tab(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    cx.simulate_keystrokes("cmd-alt-] cmd-alt-[");
+    assert_eq!(active(&workspace, cx), None);
+    assert_eq!(tab_count(&workspace, cx), 1);
+}
```

Modify `crates/app/tests/app/challenges.rs`:
```diff
@@ -76,7 +76,7 @@
     open_challenge(&workspace, cx, &ChallengeId::new("broken", "a"));
     assert_eq!(titles(&workspace, cx), ["Query 1"]);
     let note = cx.update(|_, cx| workspace.read(cx).note().map(str::to_owned));
-    assert!(note.is_some_and(|note| note.starts_with("Couldn't open Cracked: ")));
+    assert!(note.is_some_and(|note| note.starts_with("Couldn't open a: ")));
 }
 
 #[gpui_kit::test]
```

Modify `crates/app/tests/app/support.rs`:
```diff
@@ -140,18 +140,43 @@
     })
 }
 
-/// One track, "Broken", whose challenge `broken/a` has `solution` and whose
-/// database is built from `schema`.
-pub fn broken_tracks(schema: &'static str, solution: &str) -> Tracks {
-    let challenges = format!(
-        "[[challenge]]\nid = \"a\"\ntitle = \"Cracked\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"{solution}\"\nhints = [\"1\", \"2\", \"3\"]"
+/// Track `slug`, titled after it, whose challenges `ids` (each titled
+/// after its id) share `solution` and whose database is built from
+/// `schema`.
+pub fn raw_track(
+    slug: &'static str,
+    order: u32,
+    ids: &[&str],
+    schema: &'static str,
+    solution: &str,
+) -> RawTrack {
+    let challenges = ids
+        .iter()
+        .map(|id| format!("[[challenge]]\nid = \"{id}\"\ntitle = \"{id}\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"{solution}\"\nhints = [\"1\", \"2\", \"3\"]"))
+        .collect::<Vec<_>>()
+        .join("\n");
+    let track = format!(
+        "title = \"{slug}\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = {order}"
     );
-    let raw = RawTrack {
-        slug: "broken",
-        track: "title = \"Broken\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = 1",
+    RawTrack {
+        slug,
+        track: Box::leak(track.into_boxed_str()),
         schema,
         seed: "",
         challenges: Box::leak(challenges.into_boxed_str()),
-    };
-    Tracks::new(Catalog::load(&[raw]).0)
+    }
+}
+
+/// One track, "broken", whose challenge `broken/a` (titled "a") has
+/// `solution` and whose database is built from `schema`.
+pub fn broken_tracks(schema: &'static str, solution: &str) -> Tracks {
+    Tracks::new(Catalog::load(&[raw_track("broken", 1, &["a"], schema, solution)]).0)
+}
+
+/// Two tracks: "one" with challenges a and b, then "two" with c.
+pub fn two_tracks() -> Tracks {
+    let schema = "CREATE TABLE t (n INTEGER); INSERT INTO t VALUES (1);";
+    let one = raw_track("one", 1, &["a", "b"], schema, "SELECT n FROM t");
+    let two = raw_track("two", 2, &["c"], schema, "SELECT n FROM t");
+    Tracks::new(Catalog::load(&[one, two]).0)
 }
```

Modify `crates/app/tests/shell.rs`:
```diff
@@ -42,7 +42,7 @@
         .collect();
     let unbound = ["quarry::CopyCsv", "quarry::CopyJson"];
     let commands = actions::commands();
-    assert_eq!(commands.len(), 19);
+    assert_eq!(commands.len(), 21);
     for command in commands {
         let name = (command.action)().name();
         assert!(
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo nextest run -p quarry`
Expected: compile errors or failures for the names and behavior above.

- [ ] **Step 3: Implement**

Modify `crates/app/src/shell/actions.rs`:
```diff
@@ -19,6 +19,10 @@
         CheckAnswer,
         /// Cancels the running query.
         CancelRun,
+        /// Opens the challenge before the active one.
+        PreviousChallenge,
+        /// Opens the challenge after the active one.
+        NextChallenge,
         /// Opens a new query tab.
         NewTab,
         /// Closes the active query tab.
@@ -75,6 +79,8 @@
         KeyBinding::new("cmd-e", Explain, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
         KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
+        KeyBinding::new("cmd-alt-[", PreviousChallenge, None),
+        KeyBinding::new("cmd-alt-]", NextChallenge, None),
         KeyBinding::new("cmd-t", NewTab, None),
         KeyBinding::new("cmd-w", CloseTab, None),
         KeyBinding::new("cmd-}", NextTab, None),
@@ -124,6 +130,12 @@
         command("Check Answer", || Box::new(CheckAnswer), "Query"),
         command("Explain", || Box::new(Explain), "Query"),
         command("Cancel", || Box::new(CancelRun), "Query"),
+        command(
+            "Previous Challenge",
+            || Box::new(PreviousChallenge),
+            "Query",
+        ),
+        command("Next Challenge", || Box::new(NextChallenge), "Query"),
         command("Open Palette", || Box::new(OpenPalette), "View"),
         command("Next Tab", || Box::new(NextTab), "View"),
         command("Previous Tab", || Box::new(PreviousTab), "View"),
```

Modify `crates/app/src/views/inspector/brief.rs`:
```diff
@@ -2,12 +2,15 @@
 //! answer looks like, and the attempts so far.
 
 use gpui_kit::{
-    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
-    StatefulInteractiveElement, Styled, TestSupportExt, div, px,
+    AnyElement, App, ClickEvent, Context, InteractiveElement, IntoElement, ParentElement,
+    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
 };
 use quarry_ui_kit::{
     Aurora, Role,
-    components::badge::{Badge, Tone},
+    components::{
+        badge::{Badge, Tone},
+        button::{Button, ButtonVariant},
+    },
     palette::Colors,
     tokens::{space, text},
 };
@@ -17,6 +20,10 @@
 
 /// The id of the brief's title.
 pub const TITLE: &str = "brief-title";
+/// The id of the brief's Prev button.
+pub const PREV: &str = "brief-prev";
+/// The id of the brief's Next button.
+pub const NEXT: &str = "brief-next";
 
 /// The active challenge's brief.
 pub struct BriefView;
@@ -46,7 +53,7 @@
         root.child(story(&brief, &colors))
             .child(expected(&brief, &colors))
             .child(concepts(&brief))
-            .child(footer(&brief, &colors))
+            .child(footer(&brief, workspace, cx))
             .test_support()
             .into_any_element()
     }
@@ -107,13 +114,62 @@
         .into_any_element()
 }
 
-/// The attempts so far, at the bottom.
-fn footer(brief: &Brief, colors: &Colors) -> AnyElement {
+/// Prev, the attempts so far, and Next, at the bottom.
+fn footer(brief: &Brief, workspace: &Workspace, cx: &Context<Workspace>) -> AnyElement {
+    let colors = Aurora::colors(cx);
+    let neighbours = workspace.active_neighbours();
+    let prev = cx.listener(|workspace, _: &ClickEvent, window, cx| {
+        workspace.open_previous_challenge(window, cx);
+    });
+    let next = cx.listener(|workspace, _: &ClickEvent, window, cx| {
+        workspace.open_next_challenge(window, cx);
+    });
     div()
         .mt_auto()
+        .flex()
+        .items_center()
+        .justify_between()
         .text_size(px(text::SMALL))
         .text_color(colors.get(Role::TextMuted))
+        .child(step(
+            PREV,
+            "‹ Prev",
+            neighbours.prev.is_some(),
+            prev,
+            &colors,
+        ))
         .child(brief.attempts.clone())
+        .child(step(
+            NEXT,
+            "Next ›",
+            neighbours.next.is_some(),
+            next,
+            &colors,
+        ))
+        .into_any_element()
+}
+
+/// A Prev or Next button; with nowhere to go, a muted label that does
+/// nothing.
+fn step(
+    id: &'static str,
+    label: &'static str,
+    enabled: bool,
+    click: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
+    colors: &Colors,
+) -> AnyElement {
+    if enabled {
+        return Button::new(id, label)
+            .variant(ButtonVariant::Ghost)
+            .on_click(click)
+            .into_any_element();
+    }
+    div()
+        .id(id)
+        .px(px(space::MD))
+        .text_color(colors.get(Role::TextMuted).opacity(0.5))
+        .child(label)
+        .test_support()
         .into_any_element()
 }
```

Modify `crates/app/src/views/inspector/mod.rs`:
```diff
@@ -7,6 +7,8 @@
 mod brief;
 mod empty;
 
+pub use self::brief::{NEXT, PREV, TITLE};
+
 use gpui_kit::{AnyElement, Context};
 
 use super::workspace::Workspace;
```

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -6,7 +6,7 @@
 
 use gpui_kit::{AppContext, Context, Window};
 use quarry_challenges::{
-    catalog::ChallengeId,
+    catalog::{ChallengeId, Neighbours},
     check::{self, CheckError, Verdict},
     track::Challenge,
 };
@@ -20,7 +20,7 @@
         brief::{self, Brief, Expected},
         tabs::TabId,
     },
-    shell::actions::CheckAnswer,
+    shell::actions::{CheckAnswer, NextChallenge, PreviousChallenge},
     tracks::Tracks,
     views::query_tab::QueryTab,
 };
@@ -235,6 +235,46 @@
         Some(brief::brief(track, challenge, position, &expected, tally))
     }
 
+    /// The challenges before and after the active one; none outside a
+    /// challenge tab.
+    pub fn active_neighbours(&self) -> Neighbours {
+        self.active_challenge()
+            .map(|id| self.challenges.tracks.catalog().neighbours(id))
+            .unwrap_or_default()
+    }
+
+    /// Opens the challenge before the active one, if there is one.
+    pub fn open_previous_challenge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
+        if let Some(id) = self.active_neighbours().prev {
+            self.open_challenge(&id, window, cx);
+        }
+    }
+
+    /// Opens the challenge after the active one, if there is one.
+    pub fn open_next_challenge(&mut self, window: &mut Window, cx: &mut Context<Self>) {
+        if let Some(id) = self.active_neighbours().next {
+            self.open_challenge(&id, window, cx);
+        }
+    }
+
+    pub(super) fn previous_challenge(
+        &mut self,
+        _: &PreviousChallenge,
+        window: &mut Window,
+        cx: &mut Context<Self>,
+    ) {
+        self.open_previous_challenge(window, cx);
+    }
+
+    pub(super) fn next_challenge(
+        &mut self,
+        _: &NextChallenge,
+        window: &mut Window,
+        cx: &mut Context<Self>,
+    ) {
+        self.open_next_challenge(window, cx);
+    }
+
     /// This session's attempts.
     pub fn attempts(&self) -> &Attempts {
         &self.challenges.attempts
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -36,6 +36,8 @@
             .on_action(cx.listener(Self::run_all))
             .on_action(cx.listener(Self::explain))
             .on_action(cx.listener(Self::check_answer))
+            .on_action(cx.listener(Self::previous_challenge))
+            .on_action(cx.listener(Self::next_challenge))
             .on_action(cx.listener(Self::cancel))
             .on_action(cx.listener(Self::new_tab))
             .on_action(cx.listener(Self::close_tab))
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p quarry`
Expected: all pass.

- [ ] **Step 5: Run the gates**

Run: `just fmt lint quality typos complexity && cargo nextest run --workspace`
Expected: no violations, and `509` tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/shell/actions.rs crates/app/src/views/inspector/brief.rs crates/app/src/views/inspector/mod.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/layout.rs crates/app/tests/app/brief.rs crates/app/tests/app/challenges.rs crates/app/tests/app/support.rs crates/app/tests/shell.rs
git commit -m "feat(app): move to the previous or next challenge"
```

---

### Task 6: App: the feedback card

`model::verdict::card` replaces `report`: a header, a subline, one checklist line per rule (✓, ✗ or –) for a wrong answer, and "Next up" or "Last challenge" after a solve. The workspace hands the tab a `Solve` when its check finishes. The Check view draws the card, with Next challenge (which the tab passes on as `QueryTabEvent::NextChallenge`) and Stay here.

**Files:**
- Modify: `crates/app/src/model/verdict.rs`
- Modify: `crates/app/src/model/verdict/tests.rs`
- Modify: `crates/app/src/views/query_tab/check.rs`
- Modify: `crates/app/src/views/query_tab/mod.rs`
- Modify: `crates/app/src/views/results/check.rs`
- Modify: `crates/app/src/views/results/mod.rs`
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/tabs.rs`
- Modify: `crates/app/tests/app/check.rs`

**Interfaces:**
- Consumes: `Grading`, `RuleOutcome`, `Attempts`, `Catalog::neighbours`.
- Produces: `model::verdict::{Card, Line, Mark, Footer, Solve, card}`, `QueryTab::{set_solve, solve, stay, stayed, next_challenge}`, `QueryTabEvent::NextChallenge`, `results::check::{NEXT_CHALLENGE, STAY_HERE}`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/src/model/verdict/tests.rs`:
```diff
@@ -4,18 +4,39 @@
 };
 use quarry_engine::error::EngineError;
 
-use super::{CheckState, Report, Tone, report};
+use super::{Card, CheckState, Footer, Line, Mark, Solve, Tone, card};
 
-fn wrong(failure: CheckFailure) -> Report {
-    let outcome = RuleOutcome {
-        rule: "columns",
-        status: RuleStatus::Failed(failure),
-    };
-    report(&Ok(Verdict::of(Grading::new(vec![outcome]))))
+fn outcome(rule: &'static str, status: RuleStatus) -> RuleOutcome {
+    RuleOutcome { rule, status }
+}
+
+fn solve(next: Option<&str>) -> Solve {
+    Solve {
+        title: "First spin".to_owned(),
+        attempt: 3,
+        next: next.map(str::to_owned),
+    }
+}
+
+fn graded(outcomes: Vec<RuleOutcome>) -> Card {
+    card(&Ok(Verdict::of(Grading::new(outcomes))), &solve(None))
+}
+
+fn errored(error: CheckError) -> Card {
+    card(&Err(error), &solve(None))
 }
 
+/// The line a lone failure gives.
 fn detail(failure: CheckFailure) -> Option<String> {
-    wrong(failure).detail
+    let card = graded(vec![outcome("columns", RuleStatus::Failed(failure))]);
+    card.lines.first().map(|line| line.text.clone())
+}
+
+fn lines(card: &Card) -> Vec<(Mark, &str)> {
+    card.lines
+        .iter()
+        .map(|line| (line.mark, line.text.as_str()))
+        .collect()
 }
 
 #[test]
@@ -24,25 +45,107 @@
 }
 
 #[test]
-fn a_solved_answer_says_so() {
+fn a_solve_names_the_challenge_the_attempt_and_whats_next() {
+    let solved = card(
+        &Ok(Verdict::of(Grading::new(Vec::new()))),
+        &solve(Some("Crate digging")),
+    );
     assert_eq!(
-        report(&Ok(Verdict::of(Grading::new(Vec::new())))),
-        Report {
+        solved,
+        Card {
             tone: Tone::Solved,
-            headline: "Solved.".to_owned(),
-            detail: None,
+            headline: "First spin: solved".to_owned(),
+            subline: Some("Attempt 3".to_owned()),
+            lines: Vec::new(),
+            footer: Some(Footer::Next("Crate digging".to_owned())),
+        }
+    );
+    let last = graded(Vec::new());
+    assert_eq!(last.footer, Some(Footer::Last));
+}
+
+#[test]
+fn a_wrong_answer_lists_every_rule() {
+    let failure = CheckFailure::RowCount {
+        expected: 4,
+        got: 7,
+    };
+    let wrong = graded(vec![
+        outcome("columns", RuleStatus::Passed),
+        outcome("row count", RuleStatus::Failed(failure)),
+        outcome("contents", RuleStatus::NotChecked),
+        outcome("order", RuleStatus::NotChecked),
+    ]);
+    assert_eq!(wrong.tone, Tone::Wrong);
+    assert_eq!(wrong.headline, "Not quite yet");
+    assert_eq!(
+        wrong.subline.as_deref(),
+        Some("Here's what's different. Your query is unchanged.")
+    );
+    assert_eq!(wrong.footer, None);
+    assert_eq!(
+        lines(&wrong),
+        [
+            (Mark::Pass, "Columns match"),
+            (Mark::Fail, "Expected 4 rows, got 7."),
+            (Mark::Skip, "Rows not checked until the row count matches"),
+            (Mark::Skip, "Order not checked until the row count matches"),
+        ]
+    );
+}
+
+#[test]
+fn every_rule_has_a_name() {
+    let passed = graded(vec![
+        outcome("row count", RuleStatus::Passed),
+        outcome("contents", RuleStatus::Passed),
+        outcome("order", RuleStatus::Passed),
+        outcome("new", RuleStatus::Passed),
+        outcome(
+            "columns",
+            RuleStatus::Failed(CheckFailure::Order { row: 0 }),
+        ),
+        outcome("new", RuleStatus::NotChecked),
+    ]);
+    assert_eq!(
+        lines(&passed),
+        [
+            (Mark::Pass, "Row count matches"),
+            (Mark::Pass, "Rows match"),
+            (Mark::Pass, "Order matches"),
+            (Mark::Pass, "This check matches"),
+            (Mark::Fail, "The rows are right, but row 1 is out of order."),
+            (Mark::Skip, "This check not checked until the columns match"),
+        ]
+    );
+}
+
+#[test]
+fn a_rule_that_doesnt_apply_says_so() {
+    let unordered = graded(vec![outcome(
+        "contents",
+        RuleStatus::Failed(CheckFailure::Contents {
+            missing: 1,
+            unexpected: 0,
+        }),
+    )]);
+    assert_eq!(unordered.lines[0].mark, Mark::Fail);
+    let line = super::line(&outcome("order", RuleStatus::NotChecked), None);
+    assert_eq!(
+        line,
+        Line {
+            mark: Mark::Skip,
+            text: "Order doesn't matter here".to_owned()
         }
     );
 }
 
 #[test]
-fn a_wrong_answer_is_not_quite_yet() {
-    let report = wrong(CheckFailure::RowCount {
-        expected: 7,
-        got: 8,
-    });
-    assert_eq!(report.tone, Tone::Wrong);
-    assert_eq!(report.headline, "Not quite yet.");
+fn marks_have_glyphs() {
+    assert_eq!(
+        [Mark::Pass, Mark::Fail, Mark::Skip].map(Mark::glyph),
+        ["✓", "✗", "–"]
+    );
 }
 
 #[test]
@@ -107,12 +210,12 @@
 
 #[test]
 fn errors_say_why_nothing_was_checked() {
-    let failed = report(&Err(CheckError::Sql("no such column: nope".to_owned())));
+    let failed = errored(CheckError::Sql("no such column: nope".to_owned()));
     assert_eq!(
         (
             failed.tone,
             failed.headline.as_str(),
-            failed.detail.as_deref()
+            failed.subline.as_deref()
         ),
         (
             Tone::Failed,
@@ -120,15 +223,15 @@
             Some("no such column: nope")
         )
     );
-    let headline = |error| report(&Err(error)).headline;
+    let headline = |error| errored(error).headline;
     assert_eq!(headline(CheckError::TimedOut), "Stopped after 5 seconds.");
     assert_eq!(
         headline(CheckError::TooManyRows),
         "Returned more than 10,000 rows."
     );
-    let content = report(&Err(CheckError::Content("no such table".to_owned())));
+    let content = errored(CheckError::Content("no such table".to_owned()));
     assert_eq!(
-        (content.headline.as_str(), content.detail),
+        (content.headline.as_str(), content.subline),
         (
             "This challenge has a problem; your answer wasn't checked.",
             None
@@ -138,11 +241,12 @@
 
 #[test]
 fn a_failed_copy_gives_the_engine_error() {
-    let copy = report(&Err(CheckError::Sandbox(EngineError::Cancelled)));
+    let copy = errored(CheckError::Sandbox(EngineError::Cancelled));
     assert_eq!(copy.tone, Tone::Failed);
     assert_eq!(
         copy.headline,
         "The challenge's database couldn't be copied."
     );
-    assert_eq!(copy.detail.as_deref(), Some("Query cancelled."));
+    assert_eq!(copy.subline.as_deref(), Some("Query cancelled."));
+    assert!(copy.lines.is_empty());
 }
```

Modify `crates/app/tests/app/check.rs`:
```diff
@@ -1,9 +1,9 @@
 //! Check answer (⌘⇧↵): grading a challenge tab's answer and showing it in
 //! the Check view.
 
-use gpui_kit::{Entity, TestAppContext, VisualTestContext};
+use gpui_kit::{Entity, TestAppContext, VisualTestContext, test::TestWindowExt};
 use quarry::{
-    model::verdict::CheckState,
+    model::verdict::{self, Card, CheckState, Footer, Mark},
     views::{
         results::{self, CHECK},
         workspace::{NOT_A_CHALLENGE, Workspace},
@@ -155,3 +155,74 @@
     assert!(first.is_stopped());
     assert!(!stop(cx).is_stopped());
 }
+
+/// The active tab's card, once its check is done.
+fn card(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<Card> {
+    cx.update(|_, cx| {
+        let tab = workspace.read(cx).active_tab().read(cx);
+        match tab.check_state() {
+            CheckState::Done(outcome) => Some(verdict::card(outcome, tab.solve())),
+            _ => None,
+        }
+    })
+}
+
+#[gpui_kit::test]
+fn a_wrong_answer_shows_every_rule_on_the_card(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    check_first_spin(&workspace, cx, "SELECT title, year FROM albums LIMIT 3");
+    let card = card(&workspace, cx).expect("a card");
+    let marks: Vec<Mark> = card.lines.iter().map(|line| line.mark).collect();
+    assert_eq!(marks, [Mark::Pass, Mark::Fail, Mark::Skip, Mark::Skip]);
+    assert_eq!(card.footer, None);
+    assert!(!drawn(cx, results::check::NEXT_CHALLENGE));
+}
+
+#[gpui_kit::test]
+fn a_solve_offers_the_next_challenge(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    check_first_spin(&workspace, cx, "SELECT title FROM albums");
+    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
+    let card = card(&workspace, cx).expect("a card");
+    assert_eq!(card.headline, "First spin: solved");
+    assert_eq!(card.subline.as_deref(), Some("Attempt 2"));
+    assert_eq!(card.footer, Some(Footer::Next("Crate digging".to_owned())));
+    assert!(drawn(cx, results::check::STAY_HERE));
+    cx.update(|window, cx| window.click(results::check::NEXT_CHALLENGE, cx));
+    let active = cx.update(|_, cx| workspace.read(cx).active_challenge().cloned());
+    assert_eq!(active, ChallengeId::parse("vinyl-vault/crate-digging"));
+}
+
+#[gpui_kit::test]
+fn stay_here_hides_the_buttons_until_the_next_check(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
+    cx.update(|window, cx| window.click(results::check::STAY_HERE, cx));
+    assert!(!drawn(cx, results::check::NEXT_CHALLENGE));
+    assert!(!drawn(cx, results::check::STAY_HERE));
+    let active = cx.update(|_, cx| workspace.read(cx).active_challenge().cloned());
+    assert_eq!(active, ChallengeId::parse("vinyl-vault/first-spin"));
+    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
+    assert!(drawn(cx, results::check::NEXT_CHALLENGE));
+}
+
+#[gpui_kit::test]
+fn the_last_challenge_has_no_next(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    let last = ChallengeId::new("vinyl-vault", "unknown-origins");
+    open_challenge(&workspace, cx, &last);
+    let solution = cx.update(|_, cx| {
+        let (_, challenge) = workspace.read(cx).tracks().challenge(&last)?;
+        Some(challenge.solution.clone())
+    });
+    write(&workspace, cx, &solution.expect("the last challenge"));
+    cx.simulate_keystrokes("cmd-shift-enter");
+    assert!(settle(cx, |_, cx| {
+        let tab = workspace.read(cx).active_tab().read(cx);
+        matches!(tab.check_state(), CheckState::Done(_))
+    }));
+    assert_eq!(
+        card(&workspace, cx).and_then(|card| card.footer),
+        Some(Footer::Last)
+    );
+}
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo nextest run -p quarry`
Expected: compile errors or failures for the names and behavior above.

- [ ] **Step 3: Implement**

Modify `crates/app/src/model/verdict.rs`:
```diff
@@ -1,11 +1,11 @@
-//! What the Check view says about a tab's latest check.
+//! What the feedback card says about a tab's latest check.
 //!
-//! A wrong answer says what differs, never how to fix it, and never shows
-//! the expected rows.
+//! A wrong answer says what differs, rule by rule, never how to fix it,
+//! and never shows the expected rows.
 
 use quarry_challenges::{
     check::{CheckError, Verdict},
-    rules::CheckFailure,
+    rules::{CheckFailure, Grading, RuleOutcome, RuleStatus},
     run::LIMITS,
 };
 
@@ -23,7 +23,7 @@
     Done(Result<Verdict, CheckError>),
 }
 
-/// How a report reads.
+/// How a card reads.
 #[derive(Debug, Clone, Copy, PartialEq, Eq)]
 pub enum Tone {
     /// The answer is right.
@@ -34,36 +34,180 @@
     Failed,
 }
 
-/// A finished check, in words.
+/// Where one rule stands on the card.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub enum Mark {
+    /// ✓: it passed.
+    Pass,
+    /// ✗: it failed.
+    Fail,
+    /// –: it wasn't checked.
+    Skip,
+}
+
+impl Mark {
+    /// The mark's glyph.
+    pub fn glyph(self) -> &'static str {
+        match self {
+            Self::Pass => "✓",
+            Self::Fail => "✗",
+            Self::Skip => "–",
+        }
+    }
+}
+
+/// One rule's line in the checklist.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Line {
+    /// Whether it passed, failed or wasn't checked.
+    pub mark: Mark,
+    /// What it says, such as "Columns match".
+    pub text: String,
+}
+
+/// What follows a solve.
 #[derive(Debug, Clone, PartialEq, Eq)]
-pub struct Report {
+pub enum Footer {
+    /// "Next up: <title>", with Next challenge and Stay here.
+    Next(String),
+    /// "Last challenge": there is no next one.
+    Last,
+}
+
+/// What the card needs to know beyond the check: the challenge's title,
+/// which attempt this was, and the next challenge's title.
+#[derive(Debug, Clone, Default, PartialEq, Eq)]
+pub struct Solve {
+    /// The challenge's title.
+    pub title: String,
+    /// Which attempt the check was, counting from 1.
+    pub attempt: u32,
+    /// The next challenge's title, if there is one.
+    pub next: Option<String>,
+}
+
+/// A finished check, as the feedback card shows it.
+#[derive(Debug, Clone, PartialEq, Eq)]
+pub struct Card {
     /// How it reads.
     pub tone: Tone,
-    /// The first line, such as "Not quite yet."
+    /// The first line, such as "Not quite yet".
     pub headline: String,
-    /// What differs, or why it couldn't be checked.
-    pub detail: Option<String>,
+    /// The line under it.
+    pub subline: Option<String>,
+    /// Each rule's line, for a wrong answer.
+    pub lines: Vec<Line>,
+    /// What follows a solve.
+    pub footer: Option<Footer>,
 }
 
-impl Report {
-    fn new(tone: Tone, headline: &str, detail: Option<String>) -> Self {
+impl Card {
+    fn new(tone: Tone, headline: &str, subline: Option<String>) -> Self {
         Self {
             tone,
             headline: headline.to_owned(),
-            detail,
+            subline,
+            lines: Vec::new(),
+            footer: None,
         }
     }
 }
 
-/// The report for a finished check.
-pub fn report(outcome: &Result<Verdict, CheckError>) -> Report {
-    match outcome.as_ref().map(Verdict::failure) {
-        Ok(None) => Report::new(Tone::Solved, "Solved.", None),
-        Ok(Some(failure)) => Report::new(Tone::Wrong, "Not quite yet.", Some(difference(failure))),
+/// The card for a finished check of the challenge `solve` describes.
+pub fn card(outcome: &Result<Verdict, CheckError>, solve: &Solve) -> Card {
+    match outcome {
+        Ok(verdict) if verdict.is_solved() => solved(solve),
+        Ok(verdict) => wrong(verdict.grading()),
         Err(error) => unchecked(error),
     }
 }
 
+/// "First spin: solved", the attempt, and what comes next.
+fn solved(solve: &Solve) -> Card {
+    let mut card = Card::new(
+        Tone::Solved,
+        &format!("{}: solved", solve.title),
+        Some(format!("Attempt {}", solve.attempt)),
+    );
+    card.footer = Some(solve.next.clone().map_or(Footer::Last, Footer::Next));
+    card
+}
+
+/// "Not quite yet", and every rule's line.
+fn wrong(grading: &Grading) -> Card {
+    let mut card = Card::new(
+        Tone::Wrong,
+        "Not quite yet",
+        Some("Here's what's different. Your query is unchanged.".to_owned()),
+    );
+    let blocker = grading
+        .outcomes()
+        .iter()
+        .find(|outcome| matches!(outcome.status, RuleStatus::Failed(_)))
+        .map(|outcome| outcome.rule);
+    card.lines = grading
+        .outcomes()
+        .iter()
+        .map(|outcome| line(outcome, blocker))
+        .collect();
+    card
+}
+
+/// `outcome`'s line; `blocker` is the rule that failed, if one did.
+fn line(outcome: &RuleOutcome, blocker: Option<&str>) -> Line {
+    let name = rule_name(outcome.rule);
+    let (mark, text) = match (&outcome.status, blocker) {
+        (RuleStatus::Passed, _) => (Mark::Pass, name.agrees()),
+        (RuleStatus::Failed(failure), _) => (Mark::Fail, difference(failure)),
+        (RuleStatus::NotChecked, Some(blocker)) => (
+            Mark::Skip,
+            format!(
+                "{} not checked until {}",
+                name.label,
+                rule_name(blocker).condition()
+            ),
+        ),
+        (RuleStatus::NotChecked, None) => {
+            (Mark::Skip, format!("{} doesn't matter here", name.label))
+        }
+    };
+    Line { mark, text }
+}
+
+/// How a rule is named on the card.
+struct RuleName {
+    label: &'static str,
+    plural: bool,
+}
+
+impl RuleName {
+    /// "Columns match", "Row count matches".
+    fn agrees(&self) -> String {
+        format!("{} {}", self.label, self.verb())
+    }
+
+    /// "the columns match", "the row count matches".
+    fn condition(&self) -> String {
+        format!("the {} {}", self.label.to_lowercase(), self.verb())
+    }
+
+    fn verb(&self) -> &'static str {
+        if self.plural { "match" } else { "matches" }
+    }
+}
+
+/// The name of the rule with id `rule`.
+fn rule_name(rule: &str) -> RuleName {
+    let (label, plural) = match rule {
+        "columns" => ("Columns", true),
+        "row count" => ("Row count", false),
+        "contents" => ("Rows", true),
+        "order" => ("Order", false),
+        _ => ("This check", false),
+    };
+    RuleName { label, plural }
+}
+
 /// What differs, in one sentence.
 fn difference(failure: &CheckFailure) -> String {
     match failure {
@@ -109,27 +253,27 @@
 }
 
 /// Why the answer couldn't be checked.
-fn unchecked(error: &CheckError) -> Report {
+fn unchecked(error: &CheckError) -> Card {
     match error {
         CheckError::Sql(message) => {
-            Report::new(Tone::Failed, "Your query failed.", Some(message.clone()))
+            Card::new(Tone::Failed, "Your query failed.", Some(message.clone()))
         }
-        CheckError::TimedOut => Report::new(
+        CheckError::TimedOut => Card::new(
             Tone::Failed,
             &format!("Stopped after {} seconds.", LIMITS.time.as_secs()),
             None,
         ),
-        CheckError::TooManyRows => Report::new(
+        CheckError::TooManyRows => Card::new(
             Tone::Failed,
             &format!("Returned more than {} rows.", count(LIMITS.rows)),
             None,
         ),
-        CheckError::Content(_) => Report::new(
+        CheckError::Content(_) => Card::new(
             Tone::Failed,
             "This challenge has a problem; your answer wasn't checked.",
             None,
         ),
-        CheckError::Sandbox(error) => Report::new(
+        CheckError::Sandbox(error) => Card::new(
             Tone::Failed,
             "The challenge's database couldn't be copied.",
             Some(error.to_string()),
```

Modify `crates/app/src/views/query_tab/check.rs`:
```diff
@@ -12,7 +12,10 @@
 };
 
 use super::{QueryTab, QueryTabEvent};
-use crate::{model::verdict::CheckState, views::results::CHECK};
+use crate::{
+    model::verdict::{CheckState, Solve},
+    views::results::CHECK,
+};
 
 impl QueryTab {
     /// Gives the tab the Check view, for a challenge tab.
@@ -44,13 +47,43 @@
         cx.notify();
     }
 
-    /// Stops the latest check, and returns a runner for the next one.
+    /// Stops the latest check, and returns a runner for the next one. The
+    /// next card shows its buttons again.
     fn restart_check(&mut self) -> Runner {
+        self.stayed = false;
         self.check_stop.stop();
         self.check_stop = Stop::default();
         Runner::stoppable(LIMITS, self.check_stop.clone())
     }
 
+    /// Tells the feedback card about the challenge the latest check was
+    /// for.
+    pub fn set_solve(&mut self, solve: Solve, cx: &mut Context<Self>) {
+        self.solve = solve;
+        cx.notify();
+    }
+
+    /// What the feedback card knows of the challenge.
+    pub fn solve(&self) -> &Solve {
+        &self.solve
+    }
+
+    /// Stay here: hides the solved card's buttons until the next check.
+    pub fn stay(&mut self, cx: &mut Context<Self>) {
+        self.stayed = true;
+        cx.notify();
+    }
+
+    /// Whether Stay here was clicked since the latest check.
+    pub fn stayed(&self) -> bool {
+        self.stayed
+    }
+
+    /// Next challenge: asks the workspace to open the next challenge.
+    pub fn next_challenge(&mut self, cx: &mut Context<Self>) {
+        cx.emit(QueryTabEvent::NextChallenge);
+    }
+
     /// What stops the tab's latest check.
     pub fn check_stop(&self) -> &Stop {
         &self.check_stop
```

Modify `crates/app/src/views/query_tab/mod.rs`:
```diff
@@ -32,7 +32,7 @@
     plan_tree::PlanState,
     statements::{Phase, RunOutput},
     target,
-    verdict::CheckState,
+    verdict::{CheckState, Solve},
 };
 
 /// What a tab tells the workspace.
@@ -46,6 +46,8 @@
     Copied(String),
     /// A check finished with this outcome.
     Checked(Result<Verdict, CheckError>),
+    /// The card's Next challenge was clicked.
+    NextChallenge,
 }
 
 /// What a run was asked to run.
@@ -71,6 +73,8 @@
     check: CheckState,
     checking: Option<Task<()>>,
     check_stop: Stop,
+    solve: Solve,
+    stayed: bool,
     _edits: Subscription,
 }
 
@@ -102,6 +106,8 @@
             check: CheckState::Idle,
             checking: None,
             check_stop: Stop::default(),
+            solve: Solve::default(),
+            stayed: false,
             _edits: edits,
         }
     }
```

Modify `crates/app/src/views/results/check.rs`:
```diff
@@ -1,22 +1,29 @@
-//! The Check view: whether the latest check solved the challenge, and if
-//! not, what differs. Phase 2B replaces it with the feedback card.
+//! The Check view: the feedback card for the latest check. A wrong answer
+//! lists where every rule stands; a solve offers the next challenge.
 
 use gpui_kit::{
-    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
+    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
     StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
 };
 use quarry_ui_kit::{
     Aurora, Role,
+    components::button::{Button, ButtonVariant},
+    palette::Colors,
     tokens::{space, text},
 };
 
 use super::{CHECK, ResultView};
 use crate::{
-    model::verdict::{self, CheckState, Tone},
+    model::verdict::{self, Card, CheckState, Footer, Line, Mark, Tone},
     views::query_tab::QueryTab,
 };
 
-/// What the latest Check answer said.
+/// The id of the card's Next challenge button.
+pub const NEXT_CHALLENGE: &str = "card-next";
+/// The id of the card's Stay here button.
+pub const STAY_HERE: &str = "card-stay";
+
+/// The feedback card for the latest Check answer.
 pub struct CheckView;
 
 impl ResultView for CheckView {
@@ -30,49 +37,147 @@
 
     fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
         let colors = Aurora::colors(cx);
-        let line = |message: String, role: Role| {
-            div()
-                .text_color(colors.get(role))
-                .child(message)
-                .into_any_element()
-        };
-        let body = match tab.check_state() {
-            CheckState::Idle => vec![line(
-                "Check your answer with ⌘⇧↵.".to_owned(),
-                Role::TextMuted,
-            )],
-            CheckState::Checking => vec![line("Checking…".to_owned(), Role::TextMuted)],
-            CheckState::Done(outcome) => {
-                let report = verdict::report(outcome);
-                let mut lines = vec![line(report.headline, role(report.tone))];
-                lines.extend(
-                    report
-                        .detail
-                        .map(|detail| line(detail, Role::TextSecondary)),
-                );
-                lines
-            }
-        };
-        div()
+        let root = div()
             .id(CHECK)
             .size_full()
             .overflow_y_scroll()
             .p(px(space::MD))
             .flex()
             .flex_col()
-            .gap(px(space::XS))
-            .text_size(px(text::BODY))
-            .children(body)
-            .test_support()
-            .into_any_element()
+            .gap(px(space::MD))
+            .text_size(px(text::BODY));
+        let body = match tab.check_state() {
+            CheckState::Idle => note("Check your answer with ⌘⇧↵.", &colors),
+            CheckState::Checking => note("Checking…", &colors),
+            CheckState::Done(outcome) => {
+                let card = verdict::card(outcome, tab.solve());
+                card_element(&card, tab.stayed(), cx)
+            }
+        };
+        root.child(body).test_support().into_any_element()
     }
 }
 
-/// The headline's color.
-fn role(tone: Tone) -> Role {
-    match tone {
-        Tone::Solved => Role::Success,
-        Tone::Wrong => Role::Warning,
-        Tone::Failed => Role::Danger,
-    }
+/// A quiet line, before there is a card.
+fn note(message: &'static str, colors: &Colors) -> AnyElement {
+    div()
+        .text_color(colors.get(Role::TextMuted))
+        .child(message)
+        .into_any_element()
+}
+
+/// The card: its header, its checklist and, after a solve, what's next.
+fn card_element(card: &Card, stayed: bool, cx: &Context<QueryTab>) -> AnyElement {
+    let colors = Aurora::colors(cx);
+    div()
+        .flex()
+        .flex_col()
+        .gap(px(space::MD))
+        .child(header(card, &colors))
+        .children((!card.lines.is_empty()).then(|| checklist(&card.lines, &colors)))
+        .children(
+            card.footer
+                .as_ref()
+                .map(|footer| next_up(footer, stayed, cx)),
+        )
+        .into_any_element()
+}
+
+/// The mark, the headline and the subline.
+fn header(card: &Card, colors: &Colors) -> AnyElement {
+    let (glyph, role) = match card.tone {
+        Tone::Solved => ("✓", Role::Success),
+        Tone::Wrong | Tone::Failed => ("!", Role::Danger),
+    };
+    div()
+        .flex()
+        .items_start()
+        .gap(px(space::SM))
+        .child(
+            div()
+                .text_size(px(text::TITLE))
+                .text_color(colors.get(role))
+                .child(glyph),
+        )
+        .child(
+            div()
+                .flex()
+                .flex_col()
+                .child(
+                    div()
+                        .text_size(px(text::LARGE_LABEL))
+                        .font_weight(FontWeight::SEMIBOLD)
+                        .text_color(colors.get(Role::Text))
+                        .child(card.headline.clone()),
+                )
+                .children(card.subline.clone().map(|subline| {
+                    div()
+                        .text_color(colors.get(Role::TextSecondary))
+                        .child(subline)
+                })),
+        )
+        .into_any_element()
+}
+
+/// One line per rule, marked ✓, ✗ or –.
+fn checklist(lines: &[Line], colors: &Colors) -> AnyElement {
+    div()
+        .flex()
+        .flex_col()
+        .gap(px(space::XS))
+        .children(lines.iter().map(|line| {
+            let role = match line.mark {
+                Mark::Pass => Role::Success,
+                Mark::Fail => Role::Danger,
+                Mark::Skip => Role::TextMuted,
+            };
+            let text = if line.mark == Mark::Skip {
+                Role::TextMuted
+            } else {
+                Role::Text
+            };
+            div()
+                .flex()
+                .gap(px(space::SM))
+                .child(div().text_color(colors.get(role)).child(line.mark.glyph()))
+                .child(div().text_color(colors.get(text)).child(line.text.clone()))
+        }))
+        .into_any_element()
+}
+
+/// "Next up: …" with Next challenge and Stay here, or "Last challenge".
+fn next_up(footer: &Footer, stayed: bool, cx: &Context<QueryTab>) -> AnyElement {
+    let colors = Aurora::colors(cx);
+    let Footer::Next(title) = footer else {
+        return div()
+            .text_color(colors.get(Role::TextSecondary))
+            .child("Last challenge")
+            .into_any_element();
+    };
+    let buttons = (!stayed).then(|| {
+        div()
+            .flex()
+            .gap(px(space::SM))
+            .child(
+                Button::new(NEXT_CHALLENGE, "Next challenge →")
+                    .variant(ButtonVariant::Primary)
+                    .on_click(cx.listener(|tab, _: &ClickEvent, _, cx| tab.next_challenge(cx))),
+            )
+            .child(
+                Button::new(STAY_HERE, "Stay here")
+                    .variant(ButtonVariant::Ghost)
+                    .on_click(cx.listener(|tab, _: &ClickEvent, _, cx| tab.stay(cx))),
+            )
+    });
+    div()
+        .flex()
+        .flex_col()
+        .gap(px(space::SM))
+        .child(
+            div()
+                .text_color(colors.get(Role::TextSecondary))
+                .child(format!("Next up: {title}")),
+        )
+        .children(buttons)
+        .into_any_element()
 }
```

Modify `crates/app/src/views/results/mod.rs`:
```diff
@@ -4,7 +4,7 @@
 //! Each is a [`ResultView`] in the [`ResultViewRegistry`]; adding a view is
 //! a new module and one line in [`ResultViewRegistry::standard`].
 
-mod check;
+pub mod check;
 mod grid;
 mod messages;
 mod plan;
```

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -19,6 +19,7 @@
         attempts::Attempts,
         brief::{self, Brief, Expected},
         tabs::TabId,
+        verdict::Solve,
     },
     shell::actions::{CheckAnswer, NextChallenge, PreviousChallenge},
     tracks::Tracks,
@@ -211,16 +212,38 @@
         .detach();
     }
 
-    /// Counts tab `tab`'s finished check as an attempt at its challenge.
+    /// Counts tab `tab`'s finished check as an attempt at its challenge,
+    /// and tells the tab's card about it.
     pub(super) fn checked(
         &mut self,
         tab: TabId,
         outcome: &Result<Verdict, CheckError>,
         cx: &mut Context<Self>,
     ) {
-        if let Some(id) = self.challenge(tab).cloned() {
-            self.challenges.attempts.record(&id, outcome);
-            cx.notify();
+        let Some(id) = self.challenge(tab).cloned() else {
+            return;
+        };
+        self.challenges.attempts.record(&id, outcome);
+        let solve = self.solve(&id);
+        if let Some(view) = self.views.get(&tab) {
+            view.update(cx, |tab, cx| tab.set_solve(solve, cx));
+        }
+        cx.notify();
+    }
+
+    /// What the card needs to know about challenge `id`.
+    fn solve(&self, id: &ChallengeId) -> Solve {
+        let tracks = &self.challenges.tracks;
+        let title = |id: &ChallengeId| tracks.challenge(id).map(|(_, c)| c.title.clone());
+        Solve {
+            title: title(id).unwrap_or_default(),
+            attempt: self.challenges.attempts.of(id).checks,
+            next: tracks
+                .catalog()
+                .neighbours(id)
+                .next
+                .as_ref()
+                .and_then(title),
         }
     }
```

Modify `crates/app/src/views/workspace/tabs.rs`:
```diff
@@ -21,12 +21,17 @@
         cx: &mut Context<Self>,
     ) {
         let view = cx.new(|cx| QueryTab::new(text, window, cx));
-        cx.subscribe(&view, move |workspace, _, event, cx| match event {
-            QueryTabEvent::RunEnded => workspace.read_schema(cx),
-            QueryTabEvent::Edited => workspace.schedule_save(cx),
-            QueryTabEvent::Copied(note) => workspace.show_note(note.clone(), cx),
-            QueryTabEvent::Checked(outcome) => workspace.checked(id, outcome, cx),
-        })
+        cx.subscribe_in(
+            &view,
+            window,
+            move |workspace, _, event, window, cx| match event {
+                QueryTabEvent::RunEnded => workspace.read_schema(cx),
+                QueryTabEvent::Edited => workspace.schedule_save(cx),
+                QueryTabEvent::Copied(note) => workspace.show_note(note.clone(), cx),
+                QueryTabEvent::Checked(outcome) => workspace.checked(id, outcome, cx),
+                QueryTabEvent::NextChallenge => workspace.open_next_challenge(window, cx),
+            },
+        )
         .detach();
         self.views.insert(id, view);
     }
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p quarry`
Expected: all pass.

- [ ] **Step 5: Run the gates**

Run: `just fmt lint quality typos complexity && cargo nextest run --workspace`
Expected: no violations, and `516` tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/model/verdict.rs crates/app/src/model/verdict/tests.rs crates/app/src/views/query_tab/check.rs crates/app/src/views/query_tab/mod.rs crates/app/src/views/results/check.rs crates/app/src/views/results/mod.rs crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/tabs.rs crates/app/tests/app/check.rs
git commit -m "feat(app): the feedback card, with Next challenge and Stay here"
```

---

### Task 7: ui-kit and app: the aurora pulse

`Pulse` rises to its peak a quarter of the way through `motion::PULSE` (600 ms) and fades out. On a solve, unless Reduce Motion is on, the workspace draws an inset `Role::Pulse` shadow over the window's edges, with no hitbox, and requests frames while it runs. `Role::Pulse` is checked at 3:1 as a non-text color.

**Files:**
- Modify: `crates/app/src/views/workspace/challenges.rs`
- Modify: `crates/app/src/views/workspace/layout.rs`
- Modify: `crates/app/src/views/workspace/mod.rs`
- Modify: `crates/app/tests/app/check.rs`
- Modify: `crates/ui-kit/src/lib.rs`
- Modify: `crates/ui-kit/src/palette.rs`
- Create: `crates/ui-kit/src/pulse.rs`
- Modify: `crates/ui-kit/src/tokens.rs`
- Modify: `crates/ui-kit/tests/contrast.rs`

**Interfaces:**
- Consumes: `Role`, `Palette`, `motion`, `cx.reduce_motion()`.
- Produces: `quarry_ui_kit::Pulse`, `motion::PULSE`, `Role::Pulse`, `Workspace::pulse`.

- [ ] **Step 1: Write the failing tests**

Modify `crates/app/tests/app/check.rs`:
```diff
@@ -226,3 +226,26 @@
         Some(Footer::Last)
     );
 }
+
+fn pulsed(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> bool {
+    cx.update(|_, cx| workspace.read(cx).pulse().is_some())
+}
+
+#[gpui_kit::test]
+fn a_solve_sets_off_the_aurora_pulse_and_a_wrong_answer_doesnt(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    check_first_spin(&workspace, cx, "SELECT title FROM albums");
+    assert!(!pulsed(&workspace, cx));
+    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
+    assert!(pulsed(&workspace, cx));
+}
+
+#[gpui_kit::test]
+fn reduced_motion_solves_without_a_pulse(cx: &mut TestAppContext) {
+    let (workspace, cx) = open(cx).expect("the sample database opens");
+    cx.update(|_, cx| cx.set_reduce_motion(true));
+    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
+    assert!(!pulsed(&workspace, cx));
+    let card = card(&workspace, cx).expect("a card");
+    assert_eq!(card.headline, "First spin: solved");
+}
```

Modify `crates/ui-kit/tests/contrast.rs`:
```diff
@@ -134,6 +134,16 @@
     assert_eq!(badge_failures(&Palette::dawn()), Vec::<String>::new());
 }
 
+#[test]
+fn the_pulse_glow_stands_out_on_every_surface() {
+    for palette in [Palette::aurora(), Palette::dawn()] {
+        assert_eq!(
+            failures(&palette, &[Role::Pulse], 3.0),
+            Vec::<String>::new()
+        );
+    }
+}
+
 #[test]
 fn the_focus_ring_stands_out_on_every_surface() {
     for palette in [Palette::aurora(), Palette::dawn()] {
```

- [ ] **Step 2: Run them and see them fail**

Run: `cargo nextest run -p quarry-ui-kit -p quarry`
Expected: compile errors or failures for the names and behavior above.

- [ ] **Step 3: Implement**

Modify `crates/app/src/views/workspace/challenges.rs`:
```diff
@@ -2,7 +2,7 @@
 //! on a fresh copy of its track's database. Plain tabs share the sample
 //! database.
 
-use std::collections::BTreeMap;
+use std::{collections::BTreeMap, time::Instant};
 
 use gpui_kit::{AppContext, Context, Window};
 use quarry_challenges::{
@@ -12,6 +12,7 @@
 };
 use quarry_engine::session::Session;
 use quarry_store::drafts::Draft;
+use quarry_ui_kit::{Pulse, tokens::motion};
 
 use super::Workspace;
 use crate::{
@@ -224,6 +225,9 @@
             return;
         };
         self.challenges.attempts.record(&id, outcome);
+        if outcome.as_ref().is_ok_and(Verdict::is_solved) {
+            self.start_pulse(cx);
+        }
         let solve = self.solve(&id);
         if let Some(view) = self.views.get(&tab) {
             view.update(cx, |tab, cx| tab.set_solve(solve, cx));
@@ -231,6 +235,18 @@
         cx.notify();
     }
 
+    /// Starts the aurora pulse, unless the system asks for reduced motion.
+    fn start_pulse(&mut self, cx: &mut Context<Self>) {
+        if !cx.reduce_motion() {
+            self.pulse = Some(Pulse::new(Instant::now(), motion::PULSE));
+        }
+    }
+
+    /// The latest aurora pulse, if a solve started one.
+    pub fn pulse(&self) -> Option<Pulse> {
+        self.pulse
+    }
+
     /// What the card needs to know about challenge `id`.
     fn solve(&self, id: &ChallengeId) -> Solve {
         let tracks = &self.challenges.tracks;
```

Modify `crates/app/src/views/workspace/layout.rs`:
```diff
@@ -3,12 +3,12 @@
 use std::time::Instant;
 
 use gpui_kit::{
-    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
-    StatefulInteractiveElement, Styled, TestSupportExt, Window, component::tooltip::Tooltip, div,
-    prelude::FluentBuilder, px,
+    AnyElement, BoxShadow, Context, InteractiveElement, IntoElement, ParentElement, Render,
+    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window,
+    component::tooltip::Tooltip, div, point, prelude::FluentBuilder, px,
 };
 use quarry_ui_kit::{
-    Aurora, Role,
+    Aurora, Colors, Pulse, Role,
     components::{
         status_bar::StatusBar,
         surface::{sidebar_surface, window_background},
@@ -24,9 +24,8 @@
 
 impl Render for Workspace {
     fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
-        if Aurora::is_transitioning(cx) {
-            window.request_animation_frame();
-        }
+        let now = Instant::now();
+        let pulsing = self.animate(now, window, cx);
         let colors = Aurora::colors(cx);
         div()
             .id("workspace")
@@ -53,16 +52,50 @@
             .on_action(cx.listener(Self::copy_json))
             .on_action(cx.listener(Self::open_palette))
             .size_full()
+            .relative()
             .flex()
             .bg(window_background(&colors))
             .text_color(colors.get(Role::Text))
             .when(self.sidebar_open, |root| root.child(self.sidebar(cx)))
             .child(self.center(cx))
             .when(self.inspector_open, |root| root.child(self.inspector(cx)))
+            .children(pulsing.map(|pulse| glow(pulse.opacity(now), &colors)))
     }
 }
 
+/// The aurora pulse: teal light glowing in from the window's edges, at
+/// `opacity` of its peak. It has no hitbox, so clicks pass through.
+fn glow(opacity: f32, colors: &Colors) -> AnyElement {
+    div()
+        .absolute()
+        .inset_0()
+        .shadow(vec![BoxShadow {
+            color: colors.get(Role::Pulse).opacity(opacity * GLOW_STRENGTH),
+            offset: point(px(0.), px(0.)),
+            blur_radius: px(GLOW_BLUR),
+            spread_radius: px(0.),
+            inset: true,
+        }])
+        .into_any_element()
+}
+
+/// How far the glow reaches into the window.
+const GLOW_BLUR: f32 = 64.0;
+
+/// The glow's alpha at its peak.
+const GLOW_STRENGTH: f32 = 0.6;
+
 impl Workspace {
+    /// The pulse still running at `now`. While it or the theme's
+    /// cross-fade runs, the window keeps drawing frames.
+    fn animate(&self, now: Instant, window: &mut Window, cx: &Context<Self>) -> Option<Pulse> {
+        let pulsing = self.pulse.filter(|pulse| !pulse.is_done(now));
+        if Aurora::is_transitioning(cx) || pulsing.is_some() {
+            window.request_animation_frame();
+        }
+        pulsing
+    }
+
     fn sidebar(&self, cx: &Context<Self>) -> AnyElement {
         let colors = Aurora::colors(cx);
         sidebar_surface(&colors)
```

Modify `crates/app/src/views/workspace/mod.rs`:
```diff
@@ -27,7 +27,7 @@
 };
 use quarry_engine::session::Session;
 use quarry_store::drafts::{Draft, DraftStore, Drafts};
-use quarry_ui_kit::Aurora;
+use quarry_ui_kit::{Aurora, Pulse};
 
 use self::challenges::Challenges;
 pub use self::{
@@ -93,6 +93,7 @@
     sidebar_open: bool,
     inspector_open: bool,
     inspector: Rc<InspectorRegistry>,
+    pulse: Option<Pulse>,
     focus: FocusHandle,
     sidebar_focus: FocusHandle,
     schema_read: Option<Task<()>>,
@@ -130,6 +131,7 @@
             sidebar_open: true,
             inspector_open: !drafts.inspector_hidden,
             inspector: Rc::new(InspectorRegistry::standard()),
+            pulse: None,
             focus: cx.focus_handle(),
             sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
             schema_read: None,
```

Modify `crates/ui-kit/src/lib.rs`:
```diff
@@ -11,9 +11,11 @@
 pub mod component_theme;
 pub mod components;
 pub mod palette;
+pub mod pulse;
 pub mod tokens;
 pub mod transition;
 
 pub use appearance::{Appearance, Preference};
 pub use aurora::Aurora;
 pub use palette::{Colors, Palette, Role};
+pub use pulse::Pulse;
```

Modify `crates/ui-kit/src/palette.rs`:
```diff
@@ -61,11 +61,14 @@
     ActiveLine,
     /// The keyboard focus ring (WCAG non-text contrast, 3:1).
     FocusRing,
+    /// The aurora pulse's glow at the window's edges, at its peak
+    /// (WCAG non-text contrast, 3:1).
+    Pulse,
 }
 
 impl Role {
     /// Every role, in declaration order.
-    pub const ALL: [Self; 27] = [
+    pub const ALL: [Self; 28] = [
         Self::BackgroundStart,
         Self::BackgroundMiddle,
         Self::BackgroundEnd,
@@ -93,6 +96,7 @@
         Self::LineNumber,
         Self::ActiveLine,
         Self::FocusRing,
+        Self::Pulse,
     ];
 
     fn index(self) -> usize {
@@ -100,7 +104,7 @@
     }
 }
 
-const AURORA: [(Role, u32); 27] = [
+const AURORA: [(Role, u32); 28] = [
     (Role::BackgroundStart, 0x1b15_30ff),
     (Role::BackgroundMiddle, 0x0e14_24ff),
     (Role::BackgroundEnd, 0x0b1b_22ff),
@@ -128,9 +132,10 @@
     (Role::LineNumber, 0xe6e4_f071),
     (Role::ActiveLine, 0xffff_ff0a),
     (Role::FocusRing, 0xcbb8_ffff),
+    (Role::Pulse, 0x5eea_d4ff),
 ];
 
-const DAWN: [(Role, u32); 27] = [
+const DAWN: [(Role, u32); 28] = [
     (Role::BackgroundStart, 0xf3ee_ffff),
     (Role::BackgroundMiddle, 0xeef3_ffff),
     (Role::BackgroundEnd, 0xe9f8_f6ff),
@@ -158,6 +163,7 @@
     (Role::LineNumber, 0x2420_3a82),
     (Role::ActiveLine, 0x8b6c_ff0f),
     (Role::FocusRing, 0x593f_bcff),
+    (Role::Pulse, 0x0c8a_7fff),
 ];
 
 /// A theme's colors as `0xRRGGBBAA` values, one per [`Role`].
```

Create `crates/ui-kit/src/pulse.rs`:
```diff
@@ -0,0 +1,79 @@
+//! The aurora pulse: a brief glow of light at the window's edges that
+//! rises quickly and fades.
+
+use std::time::{Duration, Instant};
+
+/// How far into the pulse it is brightest.
+const PEAK: f32 = 0.25;
+
+/// One pulse, from when it started.
+#[derive(Debug, Clone, Copy, PartialEq, Eq)]
+pub struct Pulse {
+    started: Instant,
+    duration: Duration,
+}
+
+impl Pulse {
+    /// A pulse starting at `started` and lasting `duration`.
+    pub fn new(started: Instant, duration: Duration) -> Self {
+        Self { started, duration }
+    }
+
+    /// How bright it is at `now`, from 0 to 1: rising to 1 a quarter of
+    /// the way through, then fading back to 0 at the end.
+    pub fn opacity(&self, now: Instant) -> f32 {
+        let t = self.progress(now);
+        if t < PEAK {
+            t / PEAK
+        } else {
+            (1.0 - t) / (1.0 - PEAK)
+        }
+    }
+
+    /// Whether the pulse has finished at `now`.
+    pub fn is_done(&self, now: Instant) -> bool {
+        self.progress(now) >= 1.0
+    }
+
+    fn progress(&self, now: Instant) -> f32 {
+        if self.duration.is_zero() {
+            return 1.0;
+        }
+        let elapsed = now.saturating_duration_since(self.started);
+        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
+    }
+}
+
+#[cfg(test)]
+mod tests {
+    use std::time::{Duration, Instant};
+
+    use super::Pulse;
+
+    fn at(pulse: &Pulse, start: Instant, millis: u64) -> f32 {
+        pulse.opacity(start + Duration::from_millis(millis))
+    }
+
+    #[test]
+    fn rises_to_its_peak_then_fades_to_nothing() {
+        let start = Instant::now();
+        let pulse = Pulse::new(start, Duration::from_millis(400));
+        assert!(at(&pulse, start, 0).abs() < 1e-6);
+        assert!((at(&pulse, start, 50) - 0.5).abs() < 1e-3);
+        assert!((at(&pulse, start, 100) - 1.0).abs() < 1e-3);
+        assert!((at(&pulse, start, 250) - 0.5).abs() < 1e-3);
+        assert!(at(&pulse, start, 400).abs() < 1e-6);
+        assert!(at(&pulse, start, 900).abs() < 1e-6);
+    }
+
+    #[test]
+    fn is_done_once_its_time_is_up() {
+        let start = Instant::now();
+        let pulse = Pulse::new(start, Duration::from_millis(400));
+        assert!(!pulse.is_done(start + Duration::from_millis(399)));
+        assert!(pulse.is_done(start + Duration::from_millis(400)));
+        let instant = Pulse::new(start, Duration::ZERO);
+        assert!(instant.is_done(start));
+        assert!(instant.opacity(start).abs() < 1e-6);
+    }
+}
```

Modify `crates/ui-kit/src/tokens.rs`:
```diff
@@ -67,4 +67,6 @@
     pub const FAST: Duration = Duration::from_millis(150);
     /// The theme cross-fade.
     pub const THEME: Duration = Duration::from_millis(250);
+    /// The aurora pulse on a solved challenge.
+    pub const PULSE: Duration = Duration::from_millis(600);
 }
```

- [ ] **Step 4: Run the tests**

Run: `cargo nextest run -p quarry-ui-kit -p quarry`
Expected: all pass.

- [ ] **Step 5: Run the gates**

Run: `just fmt lint quality typos complexity && cargo nextest run --workspace`
Expected: no violations, and `521` tests pass.

- [ ] **Step 6: Commit**

```bash
git add crates/app/src/views/workspace/challenges.rs crates/app/src/views/workspace/layout.rs crates/app/src/views/workspace/mod.rs crates/app/tests/app/check.rs crates/ui-kit/src/lib.rs crates/ui-kit/src/palette.rs crates/ui-kit/src/pulse.rs crates/ui-kit/src/tokens.rs crates/ui-kit/tests/contrast.rs
git commit -m "feat: the aurora pulse, a teal glow at the window's edges on a solve"
```

---

### Task 8: Docs: ADR 0013, the README and the parent spec

Record the decisions in ADR 0013 "Brief, feedback and pulse", move the README's status to Phase 2B, and note in §4.3 of the parent spec what 2B builds and what waits for 2C.

**Files:**
- Modify: `README.md`
- Create: `docs/adr/0013-brief-feedback-and-pulse.md`
- Modify: `docs/superpowers/specs/2026-11-04-quarry-design.md`

**Interfaces:**
- Consumes: —
- Produces: `docs/adr/0013-brief-feedback-and-pulse.md`.

- [ ] **Step 1: Write the docs**

Modify `README.md`:
```diff
@@ -2,7 +2,7 @@
 
 A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.
 
-> **Status:** Phase 2A. On top of Phase 1's query tabs, scripts, plans, copying and ⌘K palette, you can open Vinyl Vault's first five challenges from ⌘K, each in its own tab on a fresh copy of the track's database, and check your answer with ⌘⇧↵. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
+> **Status:** Phase 2B. On top of Phase 1's query tabs, scripts, plans, copying and ⌘K palette, you can open Vinyl Vault's first five challenges from ⌘K, each in its own tab on a fresh copy of the track's database, read its brief in the inspector (⌘⌥I), check your answer with ⌘⇧↵ and see what's different rule by rule, and move between challenges with ⌘⌥[ and ⌘⌥]. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).
 
 - Built-in SQLite playground with story-driven SQL challenge tracks
 - PostgreSQL and MySQL connections for everyday work
```

Create `docs/adr/0013-brief-feedback-and-pulse.md`:
```diff
@@ -0,0 +1,24 @@
+# ADR 0013: Brief, feedback and pulse
+
+- **Status:** Accepted
+- **Date:** 2026-11-21
+
+## Context
+Phase 2B (`docs/superpowers/specs/2026-11-19-phase-2b-brief-feedback-design.md`) gives challenges their face: the challenge brief in a right-hand inspector, the "Not quite yet" and solved feedback card, the aurora pulse on a solve, and Prev/Next between challenges. Progress isn't stored until 2C, and the opt-in row diff and expected-result reveal wait for 2C's hint policy, so 2B has to show attempts and what comes next without either.
+
+## Decision
+- **The pipeline reports every rule.** `rules::grade` runs the pipeline and returns a `Grading`: one `RuleOutcome` per rule, each `Passed`, `Failed(CheckFailure)` or `NotChecked`. Every rule after the first failure is not checked, and so is a rule whose `CheckRule::applies` is false (Order, when order doesn't matter). `Verdict` is `Solved(Grading)` or `Wrong(Grading)`, and `first_failure` is gone. The app turns the outcomes into the card's checklist without knowing the pipeline's order; a rule it has no name for still gets a line.
+- **The brief's expectation is computed, never stored.** `check::expect` runs the solution through the kind's new `ChallengeKind::expected` and keeps only the column names and the row count. The workspace works it out on the background executor when a challenge tab opens, and `Tracks` caches it for the session. The brief never shows expected rows or values.
+- **The inspector is a registry.** `InspectorRegistry` picks the first `InspectorView` that applies to the active tab: the brief for a challenge tab, an empty state otherwise. The column keeps its width between tabs. ⌘⌥I hides and shows it, and `Drafts::inspector_hidden` (a key in the store's `state` table, so no migration) keeps that across restarts. Phase 3's row inspector is one more view.
+- **Attempts are session-only.** `model::attempts` counts each check that gives a verdict, right or wrong, per challenge; a failing query or a timeout doesn't count. 2C moves the counts into `store`, and with them the card's "N of M solved" and the glowing progress segment.
+- **The feedback card is the Check view.** It renders `model::verdict::card`: "Not quite yet" with ✓, ✗ and – per rule, or "<Title>: solved" with the attempt, "Next up: …", **Next challenge** and **Stay here** (or "Last challenge"). The workspace hands the tab a `Solve` (title, attempt, next title) when the check finishes. Errors keep 2A's sentences under the same header.
+- **Prev/Next walk the whole catalog.** `Catalog::neighbours` moves within a track and on into the next one, over the tracks that loaded. The brief's buttons, ⌘⌥[ and ⌘⌥], and the card's Next challenge all open or focus a tab exactly as the palette does.
+- **The pulse is a glow at the window's edges.** A wash over the whole window failed the contrast gate: in the light theme, body text already sits at 4.51:1, so any tint over it falls below 4.5:1. Instead, `Pulse` drives an inset teal shadow over the window's edges for 600 ms (`motion::PULSE`), peaking at a quarter of the way through. `Role::Pulse` is checked as non-text at 3:1 on every surface. The glow has no hitbox, and it's skipped under Reduce Motion.
+- **A newer check stops the older one.** `run::Stop` holds the cancellers of the runs it watches; `Runner` registers each run's driver with its stop. A tab stops its previous check's runs before starting a new one, so a superseded 5-second query no longer runs to its limit in the background. A stopped run reports as timed out to a caller that no longer listens.
+- **Tracks are injected.** `Workspace::new` takes `Databases` (the sample session, its name and the `Tracks`), so tests can open a workspace on tracks whose database or solution is broken.
+
+## Consequences
+- A new rule needs a name in `model::verdict`, or it reads as "This check".
+- Opening a challenge tab runs its solution once more, in the background, once per challenge per session.
+- Attempts reset when Quarry quits until 2C.
+- The pulse can't tint the window behind the text without lowering contrast; any stronger effect would need a palette with more headroom.
```

Modify `docs/superpowers/specs/2026-11-04-quarry-design.md`:
```diff
@@ -176,6 +176,7 @@
 - **Right:**
   - An **aurora pulse**: about 600 ms of teal light across the window.
   - The progress segment glows, and the card reads "8 of 12 solved. Next up: …" with Next challenge and Stay here buttons.
+- Phase 2B builds the card, the brief and the pulse with attempts kept for the session only; the progress glow, "N of M solved", Show row diff and Reveal expected result come with 2C's progress and hint policy. The pulse is a teal glow at the window's edges rather than a wash across it, because a wash lowers text contrast below the gate (ADR 0013).
 
 ## 5. Challenges
```

- [ ] **Step 2: Check spelling**

Run: `just typos`
Expected: no output.

- [ ] **Step 3: Commit**

```bash
git add README.md docs/adr/0013-brief-feedback-and-pulse.md docs/superpowers/specs/2026-11-04-quarry-design.md
git commit -m "docs: ADR 0013 brief, feedback and pulse"
```
