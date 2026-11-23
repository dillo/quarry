//! The brief's data: each challenge's expected result, worked out in the
//! background, and this session's attempts.

use std::sync::Arc;

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use quarry::{
    model::{attempts::Tally, brief::Expected},
    views::workspace::Workspace,
};
use quarry_challenges::{catalog::ChallengeId, check::Expectation};
use quarry_store::drafts::Forgetful;

use super::{
    challenges::open_challenge,
    check::check_first_spin,
    support::{broken_tracks, install, open, open_on, settle},
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
