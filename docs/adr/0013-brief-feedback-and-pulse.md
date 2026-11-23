# ADR 0013: Brief, feedback and pulse

- **Status:** Accepted
- **Date:** 2026-11-21

## Context
Phase 2B (`docs/superpowers/specs/2026-11-19-phase-2b-brief-feedback-design.md`) gives challenges their face: the challenge brief in a right-hand inspector, the "Not quite yet" and solved feedback card, the aurora pulse on a solve, and Prev/Next between challenges. Progress isn't stored until 2C, and the opt-in row diff and expected-result reveal wait for 2C's hint policy, so 2B has to show attempts and what comes next without either.

## Decision
- **The pipeline reports every rule.** `rules::grade` runs the pipeline and returns a `Grading`: one `RuleOutcome` per rule, each `Passed`, `Failed(CheckFailure)`, `NotChecked` or `NotApplicable`. A rule whose `CheckRule::applies` is false is `NotApplicable` (Order, when order doesn't matter, which the card words "Order doesn't matter here"); every other rule after the first failure is `NotChecked`. `Verdict` is `Solved(Grading)` or `Wrong(Grading)`, and `first_failure` is gone. The app turns the outcomes into the card's checklist without knowing the pipeline's order; a rule it has no name for still gets a line.
- **The brief's expectation is computed, never stored.** `check::expect` runs the solution through the kind's new `ChallengeKind::expected` and keeps only the column names and the row count. The workspace works it out on the background executor when a challenge tab opens, and `Tracks` caches it for the session. The brief never shows expected rows or values.
- **The inspector is a registry.** `InspectorRegistry` picks the first `InspectorView` that applies to the active tab: the brief for a challenge tab, an empty state otherwise. The column keeps its width between tabs. ⌘⌥I hides and shows it, and `Drafts::inspector_hidden` (a key in the store's `state` table, so no migration) keeps that across restarts. Phase 3's row inspector is one more view.
- **Attempts are session-only.** `model::attempts` counts each check that gives a verdict, right or wrong, per challenge; a failing query or a timeout doesn't count. 2C moves the counts into `store`, and with them the card's "N of M solved" and the glowing progress segment.
- **The feedback card is the Check view.** It renders `model::verdict::card`: "Not quite yet" with ✓, ✗ and – per rule, or "<Title>: solved" with the attempt, "Next up: …", **Next challenge** and **Stay here** (or "Last challenge"). The workspace hands the tab a `Solve` (title, attempt, next title) when the check finishes. A check that couldn't run shows its own headline (such as "Your query failed.") under an orange "!" header, with no checklist.
- **Prev/Next walk the whole catalog.** `Catalog::neighbours` moves within a track and on into the next one, over the tracks that loaded. The brief's buttons, ⌘⌥[ and ⌘⌥], and the card's Next challenge all open or focus a tab exactly as the palette does. With no neighbour, the brief shows the label disabled, in `Role::TextMuted` at 50% opacity. That one derived color sits outside the contrast test on purpose: WCAG exempts disabled controls, and a disabled label should read as fainter than the muted text around it.
- **The pulse is a glow at the window's edges.** A wash over the whole window failed the contrast gate: in the light theme, body text already sits at 4.51:1, so any tint over it falls below 4.5:1. Instead, `Pulse` drives an inset teal shadow over the window's edges for 600 ms (`motion::PULSE`), peaking at a quarter of the way through. `Role::Pulse` is checked as non-text at 3:1 on every surface. The glow has no hitbox, and it's skipped under Reduce Motion.
- **A newer check stops the older one.** `run::Stop` holds the cancellers of the runs it watches; `Runner` registers each run's driver with its stop. A tab stops its previous check's runs before starting a new one, so a superseded 5-second query no longer runs to its limit in the background. A stopped run reports as timed out to a caller that no longer listens.
- **Tracks are injected.** `Workspace::new` takes `Databases` (the sample session, its name and the `Tracks`), so tests can open a workspace on tracks whose database or solution is broken.

## Consequences
- A new rule needs a name in `model::verdict`, or it reads as "This check".
- Opening a challenge tab runs its solution once more, in the background, once per challenge per session.
- Attempts reset when Quarry quits until 2C.
- The pulse can't tint the window behind the text without lowering contrast; any stronger effect would need a palette with more headroom.
