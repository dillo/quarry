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
