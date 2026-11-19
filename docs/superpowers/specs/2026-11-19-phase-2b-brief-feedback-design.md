# Phase 2B: the brief, feedback and the aurora pulse

Status: approved in brainstorming on 2026-11-19. Parent spec: `2026-11-04-quarry-design.md` (§3.3, §4.2, §4.3, §5.4). Previous slice: `2026-11-16-phase-2a-challenge-core-design.md`; its §1 holds the table of all Phase 2 slices.

## 1. Scope

**In 2B:**
- A right **inspector** column (⌘⌥I) behind a new `InspectorRegistry`. It has two views:
  - the challenge **Brief**;
  - an empty state for plain tabs.
- The brief shows:
  - the track and the challenge's position;
  - the title and the prompt;
  - the expected column names and row count, computed from the solution;
  - whether order matters;
  - the concept tags;
  - the attempt count;
  - Prev/Next.
- The **feedback card**, which replaces 2A's plain-text Check view:
  - a wrong answer shows a checklist of every rule, each passed, failed or not checked;
  - a right answer shows "Next up: …" with Next challenge and Stay here buttons.
- The **aurora pulse:** about 600 ms of teal light over the window on a right answer. It's skipped under Reduce Motion.
- **Prev/Next** across the whole catalog: within a track, then on to the next track.
- Two follow-ups deferred from 2A:
  - `Tracks` is injected into `Workspace`;
  - a superseded check's background work is cancelled.

**Session-only for now:** the attempt count and whether a challenge is solved are kept in memory and lost on quit. 2C moves them into `store`.

**Not in 2B:**
- "N of M solved" and the glowing progress segment (2C, once progress persists).
- Show row diff and Reveal expected result. Both give away part of the answer, so they arrive in 2C under the hint policy.
- Hints and Reveal solution (2C).
- The row inspector (Phase 3).
- Home (2D).

## 2. Global constraints

Every constraint in §2 of the 2A spec still applies, without change:
- no completion or assistance of any kind;
- the function-size limits;
- extension points through registries;
- the direction of dependencies;
- the coverage and mutation thresholds;
- no `unwrap` or `as`;
- polling at `POLL`;
- every color through a `Role` covered by `tests/contrast.rs`;
- fictional content;
- exact versions;
- merging only after the user's visual review.

In addition:
- **No spoilers.** The brief shows the expected column names and row count, which the parent spec already allows. It never shows expected rows or values. The card never shows them either.
- **Motion:** every animation respects Reduce Motion, through `cx.reduce_motion()` as the theme cross-fade does.

## 3. Architecture

### 3.1 `quarry-challenges`

**The full pipeline report.**
- `rules::RuleOutcome { rule: &'static str, status: RuleStatus }`, where `rule` is the `CheckRule::id()`.
- `RuleStatus` is `Passed`, `Failed(CheckFailure)` or `NotChecked`.
- `rules::grade(&Comparison) -> Grading` runs the pipeline in order. `Grading` wraps `Vec<RuleOutcome>`.
  - Every rule after the first failure is `NotChecked`.
  - `Order` is `NotChecked` when `ordered = false`, even on a solve.
  - `Grading::failure()` returns the first `Failed`, if there is one.
- `first_failure` is removed; `Grading::failure()` replaces it.
- `Verdict` becomes `Solved(Grading)` or `Wrong(Grading)`.

**Expectations.**
- `expect(&Template, &Challenge) -> Result<Expectation, CheckError>` runs the solution on a fresh copy with the grading limits.
- It returns `Expectation { columns: Vec<String>, rows: usize }`.
- A failure is `CheckError::Content`, or `Sandbox` when no copy could be made.

**Navigation.** `Catalog` gains:
- `neighbours(&ChallengeId) -> Neighbours { prev: Option<ChallengeId>, next: Option<ChallengeId> }`, walking challenges within a track and then across tracks in `order`, over the tracks that loaded;
- `position(&ChallengeId) -> Option<Position { index: usize, of: usize }>`, the challenge's 1-based place within its track.

### 3.2 `quarry-ui-kit`

- `motion::PULSE` = 600 ms.
- `Pulse`, modelled on `Transition`, is built from a start `Instant` and a duration.
  - `opacity(now) -> f32` rises from 0 to 1 at 25% of the duration, then falls back to 0 at the end, where it stays.
  - `is_done(now)` tells callers when to stop requesting frames.
- `Role::Pulse` is teal at a low alpha, in both themes. `tests/contrast.rs` checks that body text over the pulse at its peak still meets the text contrast threshold.

### 3.3 App models

These modules are free of GPUI, need 100% line coverage and are mutation-tested.

| Module | Responsibility |
|---|---|
| `model/attempts` | `Attempts`: a session-only map from `ChallengeId` to `{ checks: u32, solved: bool }`. `record(id, &Result<Verdict, CheckError>)` counts `Solved` and `Wrong` and ignores errors. A solve sets `solved` |
| `model/brief` | `brief(track, challenge, position, expectation, attempts) -> Brief`: the brief's lines (see §4.2), with the expectation as `Pending`, `Ready(Expectation)` or `Unavailable` |
| `model/verdict` (extended) | Turns an outcome into the card: header, subline, tone, checklist lines (`✓`/`✗`/`–` with text) and the solved footer ("Next up: <title>" or "Last challenge") |

### 3.4 App views and workspace

- **`Tracks`:**
  - `Workspace` receives it from its constructor instead of building it, so tests can inject broken tracks.
  - It also caches one `Expectation` result per challenge, computed on the background executor the first time that challenge's brief is shown.
- **`views/inspector`:**
  - `InspectorView` answers whether it applies to the active tab, and renders.
  - `InspectorRegistry` keeps the views in order and picks the first that applies. There are two views, `BriefView` (challenge tabs) and `EmptyView` (anything else).
  - Adding the row inspector is one new view and one line.
- **Layout:**
  - A fourth column with a fixed width, on a frosted panel like the sidebar.
  - `ToggleInspector` is bound to ⌘⌥I and listed as View → Toggle Inspector in the command table, so it also appears in the palette.
  - Whether the inspector is open is kept in the store's `state` table under `inspector_open`. It is open by default, and no migration is needed.
- **Navigation actions:**
  - `PreviousChallenge` (⌘⌥[) and `NextChallenge` (⌘⌥]) appear under Query in the command table, and do nothing outside challenge tabs.
  - They, the brief's buttons and the card's Next challenge all share the palette's open-or-focus path.
- **Check view:** the result view registered in 2A, rewritten to render the card.
- **Pulse:**
  - The workspace starts a `Pulse` when a check returns `Solved`, unless `cx.reduce_motion()` is on.
  - It draws a `Role::Pulse` overlay as the window's top layer, with no hit-testing, scaled by `opacity`, and requests frames until `is_done`.
- **Superseded checks:**
  - Each check runs with its own `Canceller`.
  - A newer check in the same tab trips the older one's canceller before starting.
  - A result is delivered to the tab that started it, even if another tab is active by then.

## 4. Behaviour

### 4.1 The inspector

- It is open by default, and ⌘⌥I hides and shows it. Its state survives a restart.
- In a plain tab it shows "No challenge in this tab" and "Press ⌘K to find one". The column keeps its width, so the layout doesn't shift between tabs.

### 4.2 The brief

From top to bottom:
1. "Vinyl Vault · Challenge 2 of 5".
2. The title, then the prompt.
3. **Expected:**
   - "Expected columns: name, year" and "12 rows";
   - "Working out the expected result…" while it is pending;
   - "Expected result unavailable" if the solution or the template fails.
4. "Order matters" or "Any order".
5. The concept tags.
6. ‹ Prev, "Attempts: N" and Next ›. A button is disabled when there is no neighbour.

### 4.3 The feedback card

- **Wrong:**
  - An orange "!" with "Not quite yet" and the subline "Here's what's different. Your query is unchanged."
  - Then one line per rule, in pipeline order:
    - `✓` (teal): "Columns match", "Row count matches", "Rows match", "Order matches".
    - `✗` (orange): the 2A sentence for that failure, for example "Row count: expected 4, got 7".
    - `–` (dim): "Row count not checked until the columns match", and so on for the rule that blocked it. When `ordered = false`, the order line reads "Order doesn't matter here".
- **Solved:**
  - A teal "✓" with "<Title>: solved" and the subline "Attempt N".
  - Then "Next up: <next title>" with **Next challenge →** and **Stay here**.
  - On the catalog's last challenge it reads "Last challenge" instead, with no buttons.
  - Stay here hides both buttons and leaves the card in place.
- **Errors** (the query failing, the timeout, the row cap, a broken challenge): the 2A sentences under an orange "!" header, with no checklist.

### 4.4 The pulse

- On a solve, a teal wash over the whole window rises for 150 ms and fades out by 600 ms. Input keeps working during the pulse.
- Under Reduce Motion there is no pulse, and the card is the only feedback.

### 4.5 Attempts

- A check that returns `Solved` or `Wrong` counts as an attempt. Errors don't count.
- The count is per challenge, not per tab, and resets when Quarry quits (until 2C).

## 5. Errors

- **The expectation fails:** the brief says "Expected result unavailable" and the rest of the brief works. Check answer reports the problem as in 2A.
- **A track is left out because its content is bad:** Prev/Next and Next up skip over it, because `neighbours` walks only the tracks that loaded.
- **`inspector_open` can't be read or written:** the inspector is open, and the existing status-bar note about the store applies.

## 6. Testing

- **`challenges`:**
  - `grade` for a failure at each rule, with the statuses before and after it;
  - all `Passed` on a solve;
  - `Order` is `NotChecked` when not ordered;
  - `Grading::failure`;
  - `expect` returns the solution's columns and row count, and `Content` for a broken solution;
  - `neighbours` at the start, middle and end of a track, across tracks, and at both ends of the catalog;
  - `position`.
- **Content test:** `expect` succeeds for every challenge.
- **`ui-kit`:**
  - `Pulse` opacity at 0, the peak, the end and after the end;
  - `is_done`;
  - the contrast of `Role::Pulse`.
- **App models:** `attempts`, `brief` and `verdict` at 100% lines and 0 missed mutants.
- **App (GPUI tests):**
  - the brief appears for a challenge tab and the empty state for a plain tab;
  - ⌘⌥I toggles the inspector, and `inspector_open` survives a restart;
  - the expectation arrives and fills the brief, and a broken solution shows "unavailable" (with tracks injected);
  - a wrong check shows the checklist and counts an attempt;
  - a solve shows the card and starts the pulse, and a solve under Reduce Motion doesn't start it;
  - Next challenge, Prev/Next and ⌘⌥[ / ⌘⌥] open or focus the right tab, including across tracks and at the ends;
  - Stay here hides the buttons;
  - a newer check cancels the older one's work.

## 7. Tasks

1. Challenges: `RuleOutcome`, `RuleStatus`, `Grading` and `grade`; `Verdict` carries the `Grading`. Update `model/verdict` to compile with no change in behaviour.
2. Challenges: `expect`, `Catalog::neighbours` and `Catalog::position`. Add the content test for every expectation.
3. App: inject `Tracks` into `Workspace`, and cancel superseded checks.
4. App: `model/attempts` and `model/brief`, and the expectation cache in `Tracks`.
5. App: `InspectorRegistry`, `BriefView` and `EmptyView`, the fourth column, ⌘⌥I and `inspector_open`.
6. App: Prev/Next in the brief, and the `PreviousChallenge`/`NextChallenge` actions and commands.
7. App: the feedback card (`model/verdict` extended, and the Check view rewritten), with Next challenge and Stay here.
8. ui-kit and app: `motion::PULSE`, `Pulse`, `Role::Pulse` and its contrast test, and the overlay on a solve.
9. ADR 0013 "Brief, feedback and pulse", the README, and a note in the parent spec's §4.3 (attempts are session-only until 2C, and the progress glow and the opt-in reveals come in 2C).

As in 1C–2A, a verified prototype is built first, and the plan is generated from it.
