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
