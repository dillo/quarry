//! The brief's data: each challenge's expected result, worked out in the
//! background, and this session's attempts.

use std::sync::Arc;

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use quarry::{
    model::{attempts::Tally, brief::Expected},
    views::{inspector, workspace::Workspace},
};
use quarry_challenges::{catalog::ChallengeId, check::Expectation};
use quarry_store::{
    drafts::{DraftStore, Drafts, Forgetful},
    sqlite::SqliteStore,
};

use super::{
    challenges::open_challenge,
    check::check_first_spin,
    support::{broken_tracks, drawn, install, open, open_on, open_with, settle},
};

fn first_spin() -> ChallengeId {
    ChallengeId::new("vinyl-vault", "first-spin")
}

/// Waits until `id`'s expectation is no longer pending, and returns it.
fn expected(
    workspace: &Entity<Workspace>,
    cx: &mut VisualTestContext,
    id: &ChallengeId,
) -> Expected {
    let known = |cx: &mut gpui_kit::App| workspace.read(cx).tracks().expected(id);
    assert!(settle(cx, |_, cx| known(cx) != Expected::Pending));
    cx.update(|_, cx| known(cx))
}

fn tally(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Tally {
    cx.update(|_, cx| workspace.read(cx).attempts().of(&first_spin()))
}

#[gpui_kit::test]
fn opening_a_challenge_works_out_its_expected_result(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    assert_eq!(
        expected(&workspace, cx, &first_spin()),
        Expected::Ready(Expectation {
            columns: vec!["title".to_owned(), "year".to_owned()],
            rows: 18,
        })
    );
}

#[gpui_kit::test]
fn a_broken_solution_leaves_the_expectation_unavailable(cx: &mut TestAppContext) {
    install(cx);
    let tracks = broken_tracks("CREATE TABLE t (n INTEGER);", "SELECT nope FROM t");
    let (workspace, cx) =
        open_on(cx, Arc::new(Forgetful), tracks).expect("the sample database opens");
    let id = ChallengeId::new("broken", "a");
    open_challenge(&workspace, cx, &id);
    assert_eq!(expected(&workspace, cx, &id), Expected::Unavailable);
}

#[gpui_kit::test]
fn verdicts_count_as_attempts_and_errors_dont(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    assert_eq!(tally(&workspace, cx), Tally::default());
    check_first_spin(&workspace, cx, "SELECT nope FROM albums");
    assert_eq!(tally(&workspace, cx), Tally::default());
    check_first_spin(&workspace, cx, "SELECT title FROM albums");
    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
    assert_eq!(
        tally(&workspace, cx),
        Tally {
            checks: 2,
            solved: true
        }
    );
}

#[gpui_kit::test]
fn the_inspector_shows_the_brief_in_a_challenge_tab_and_a_note_elsewhere(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    assert!(drawn(cx, inspector::EMPTY));
    assert!(!drawn(cx, inspector::BRIEF));
    open_challenge(&workspace, cx, &first_spin());
    assert!(drawn(cx, inspector::BRIEF));
    assert!(!drawn(cx, inspector::EMPTY));
    let brief = cx
        .update(|_, cx| workspace.read(cx).active_brief())
        .expect("a brief");
    assert_eq!(brief.place, "Vinyl Vault · Challenge 1 of 5");
    assert_eq!(brief.title, "First spin");
}

#[gpui_kit::test]
fn the_brief_fills_in_the_expected_result_and_the_attempts(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    expected(&workspace, cx, &first_spin());
    check_first_spin(&workspace, cx, "SELECT title FROM albums");
    let brief = cx
        .update(|_, cx| workspace.read(cx).active_brief())
        .expect("a brief");
    assert_eq!(brief.expected, ["Expected columns: title, year", "18 rows"]);
    assert_eq!(brief.attempts, "Attempts: 1");
}

#[gpui_kit::test]
fn cmd_alt_i_hides_and_shows_the_inspector_and_the_choice_is_saved(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let state = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            let workspace = workspace.read(cx);
            (
                workspace.inspector_open(),
                workspace.snapshot(cx).inspector_hidden,
            )
        })
    };
    assert_eq!(state(cx), (true, false));
    assert!(drawn(cx, "inspector"));
    cx.simulate_keystrokes("cmd-alt-i");
    assert_eq!(state(cx), (false, true));
    assert!(!drawn(cx, "inspector"));
    cx.simulate_keystrokes("cmd-alt-i");
    assert_eq!(state(cx), (true, false));
}

#[gpui_kit::test]
fn a_hidden_inspector_stays_hidden_after_a_restart(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let hidden = Drafts {
        inspector_hidden: true,
        ..Drafts::default()
    };
    store.save(&hidden).expect("a save");
    let (workspace, cx) = open_with(cx, store).expect("the sample database opens");
    assert!(!cx.update(|_, cx| workspace.read(cx).inspector_open()));
    assert!(!drawn(cx, "inspector"));
}
