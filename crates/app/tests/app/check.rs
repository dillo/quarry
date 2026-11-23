//! Check answer (⌘⇧↵): grading a challenge tab's answer and showing it in
//! the Check view.

use gpui_kit::{Entity, TestAppContext, VisualTestContext, test::TestWindowExt};
use quarry::{
    model::verdict::{self, Card, CheckState, Footer, Mark},
    views::{
        results::{self, CHECK},
        workspace::{NOT_A_CHALLENGE, Workspace},
    },
};
use quarry_challenges::{catalog::ChallengeId, check::CheckError, rules::CheckFailure};

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

/// Whether `state` is a finished check that solved the challenge.
fn solved(state: &CheckState) -> bool {
    matches!(state, CheckState::Done(Ok(verdict)) if verdict.is_solved())
}

/// Writes `sql` in a new First spin tab, checks it with ⌘⇧↵, and waits.
pub fn check_first_spin(
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
    assert!(solved(&checked));
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
    assert!(
        matches!(&checked, CheckState::Done(Ok(verdict)) if verdict.failure() == Some(&failure))
    );
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
    assert!(solved(&checked));
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

#[gpui_kit::test]
fn a_newer_check_stops_the_older_one(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    check_first_spin(&workspace, cx, "SELECT title FROM albums");
    let stop = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            workspace
                .read(cx)
                .active_tab()
                .read(cx)
                .check_stop()
                .clone()
        })
    };
    let first = stop(cx);
    assert!(!first.is_stopped());
    cx.simulate_keystrokes("cmd-shift-enter");
    assert!(first.is_stopped());
    assert!(!stop(cx).is_stopped());
}

/// The active tab's card, once its check is done.
fn card(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<Card> {
    cx.update(|_, cx| {
        let tab = workspace.read(cx).active_tab().read(cx);
        match tab.check_state() {
            CheckState::Done(outcome) => Some(verdict::card(outcome, tab.solve())),
            _ => None,
        }
    })
}

#[gpui_kit::test]
fn a_wrong_answer_shows_every_rule_on_the_card(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    check_first_spin(&workspace, cx, "SELECT title, year FROM albums LIMIT 3");
    let card = card(&workspace, cx).expect("a card");
    let marks: Vec<Mark> = card.lines.iter().map(|line| line.mark).collect();
    assert_eq!(marks, [Mark::Pass, Mark::Fail, Mark::Skip, Mark::Skip]);
    assert_eq!(card.footer, None);
    assert!(!drawn(cx, results::check::NEXT_CHALLENGE));
}

#[gpui_kit::test]
fn a_solve_offers_the_next_challenge(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    check_first_spin(&workspace, cx, "SELECT title FROM albums");
    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
    let card = card(&workspace, cx).expect("a card");
    assert_eq!(card.headline, "First spin: solved");
    assert_eq!(card.subline.as_deref(), Some("Attempt 2"));
    assert_eq!(card.footer, Some(Footer::Next("Crate digging".to_owned())));
    assert!(drawn(cx, results::check::STAY_HERE));
    cx.update(|window, cx| window.click(results::check::NEXT_CHALLENGE, cx));
    let active = cx.update(|_, cx| workspace.read(cx).active_challenge().cloned());
    assert_eq!(active, ChallengeId::parse("vinyl-vault/crate-digging"));
}

#[gpui_kit::test]
fn stay_here_hides_the_buttons_until_the_next_check(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
    cx.update(|window, cx| window.click(results::check::STAY_HERE, cx));
    assert!(!drawn(cx, results::check::NEXT_CHALLENGE));
    assert!(!drawn(cx, results::check::STAY_HERE));
    let active = cx.update(|_, cx| workspace.read(cx).active_challenge().cloned());
    assert_eq!(active, ChallengeId::parse("vinyl-vault/first-spin"));
    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
    assert!(drawn(cx, results::check::NEXT_CHALLENGE));
}

#[gpui_kit::test]
fn the_last_challenge_has_no_next(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let last = ChallengeId::new("vinyl-vault", "unknown-origins");
    open_challenge(&workspace, cx, &last);
    let solution = cx.update(|_, cx| {
        let (_, challenge) = workspace.read(cx).tracks().challenge(&last)?;
        Some(challenge.solution.clone())
    });
    write(&workspace, cx, &solution.expect("the last challenge"));
    cx.simulate_keystrokes("cmd-shift-enter");
    assert!(settle(cx, |_, cx| {
        let tab = workspace.read(cx).active_tab().read(cx);
        matches!(tab.check_state(), CheckState::Done(_))
    }));
    assert_eq!(
        card(&workspace, cx).and_then(|card| card.footer),
        Some(Footer::Last)
    );
}

fn pulsed(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> bool {
    cx.update(|_, cx| workspace.read(cx).pulse().is_some())
}

#[gpui_kit::test]
fn a_solve_sets_off_the_aurora_pulse_and_a_wrong_answer_doesnt(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    check_first_spin(&workspace, cx, "SELECT title FROM albums");
    assert!(!pulsed(&workspace, cx));
    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
    assert!(pulsed(&workspace, cx));
}

#[gpui_kit::test]
fn reduced_motion_solves_without_a_pulse(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.update(|_, cx| cx.set_reduce_motion(true));
    check_first_spin(&workspace, cx, "SELECT title, year FROM albums");
    assert!(!pulsed(&workspace, cx));
    let card = card(&workspace, cx).expect("a card");
    assert_eq!(card.headline, "First spin: solved");
}
