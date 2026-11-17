//! Challenge tabs: one per challenge, each on its own sandbox, with the
//! sidebar and the status bar following the active tab.

use std::sync::Arc;

use gpui_kit::{Entity, TestAppContext, VisualTestContext};
use quarry::{model::statements::Phase, sample, views::workspace::Workspace};
use quarry_challenges::catalog::ChallengeId;
use quarry_engine::value::Value;
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};

use super::support::{install, open, open_with, phase, run, settle, write};

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

fn draft(id: i64, title: &str, text: &str, challenge: Option<&str>) -> Draft {
    Draft {
        id,
        title: title.to_owned(),
        text: text.to_owned(),
        challenge: challenge.map(str::to_owned),
    }
}

/// A store holding `tabs`, the last one active.
fn store_with(tabs: Vec<Draft>) -> Result<Arc<SqliteStore>, StoreError> {
    let store = Arc::new(SqliteStore::in_memory()?);
    let drafts = Drafts {
        active: tabs.len() - 1,
        tabs,
        closed: Vec::new(),
    };
    store.save(&drafts)?;
    Ok(store)
}

/// Reopens the newest closed tab.
fn reopen_newest(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    cx.update(|window, cx| {
        workspace.update(cx, |workspace, cx| workspace.reopen_tab(0, window, cx));
    });
}

#[gpui_kit::test]
fn a_challenge_tab_and_its_draft_survive_a_restart(cx: &mut TestAppContext) {
    install(cx);
    let store = store_with(vec![
        draft(1, "Query 1", "SELECT 1", None),
        draft(
            2,
            "First spin",
            "SELECT title",
            Some("vinyl-vault/first-spin"),
        ),
    ])
    .expect("a saved store");
    let (workspace, cx) = open_with(cx, store.clone()).expect("the sample database opens");
    assert_eq!(text(&workspace, cx), "SELECT title");
    let active = cx.update(|_, cx| workspace.read(cx).active_challenge().cloned());
    assert_eq!(active, Some(first_spin()));
    assert!(sidebar_lists(&workspace, cx, "albums"));
    cx.simulate_keystrokes("cmd-t");
    assert!(settle(cx, |_, _| true));
    let saved = store.load().expect("the drafts load");
    assert_eq!(
        saved.tabs[1].challenge.as_deref(),
        Some("vinyl-vault/first-spin")
    );
    assert_eq!(saved.tabs[2].challenge, None);
}

#[gpui_kit::test]
fn a_restored_challenge_tab_takes_its_title_from_the_catalog(cx: &mut TestAppContext) {
    install(cx);
    let store = store_with(vec![draft(
        3,
        "Old name",
        "SELECT title",
        Some("vinyl-vault/first-spin"),
    )])
    .expect("a saved store");
    let (workspace, cx) = open_with(cx, store).expect("the sample database opens");
    assert_eq!(titles(&workspace, cx), ["First spin"]);
}

#[gpui_kit::test]
fn a_tab_whose_challenge_is_gone_comes_back_plain_with_its_text(cx: &mut TestAppContext) {
    install(cx);
    let store = store_with(vec![draft(
        4,
        "B-side",
        "SELECT 2",
        Some("vinyl-vault/b-side"),
    )])
    .expect("a saved store");
    let (workspace, cx) = open_with(cx, store).expect("the sample database opens");
    assert_eq!(titles(&workspace, cx), ["B-side"]);
    assert_eq!(text(&workspace, cx), "SELECT 2");
    let state = cx.update(|_, cx| {
        let workspace = workspace.read(cx);
        (workspace.active_challenge().cloned(), workspace.database())
    });
    assert_eq!(state, (None, sample::NAME.to_owned()));
}

#[gpui_kit::test]
fn a_closed_challenge_reopens_on_a_fresh_sandbox_with_its_text(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    write(
        &workspace,
        cx,
        "DELETE FROM albums; SELECT count(*) FROM albums",
    );
    cx.simulate_keystrokes("cmd-shift-r");
    assert!(settle(cx, |_, cx| !workspace
        .read(cx)
        .run_output(cx)
        .is_running()));
    cx.simulate_keystrokes("cmd-w");
    let closed = cx.update(|_, cx| workspace.read(cx).closed_tabs()[0].challenge.clone());
    assert_eq!(closed.as_deref(), Some("vinyl-vault/first-spin"));
    reopen_newest(&workspace, cx);
    assert_eq!(
        text(&workspace, cx),
        "DELETE FROM albums; SELECT count(*) FROM albums"
    );
    write(&workspace, cx, "SELECT count(*) FROM albums");
    run(&workspace, cx);
    assert_eq!(first_cell(&workspace, cx), Some(Value::Int(18)));
}

#[gpui_kit::test]
fn a_closed_challenge_that_is_open_again_reopens_as_a_plain_tab(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    open_challenge(&workspace, cx, &first_spin());
    write(&workspace, cx, "SELECT 'older'");
    cx.simulate_keystrokes("cmd-w");
    open_challenge(&workspace, cx, &first_spin());
    reopen_newest(&workspace, cx);
    assert_eq!(
        titles(&workspace, cx),
        ["Query 1", "First spin", "First spin"]
    );
    assert_eq!(text(&workspace, cx), "SELECT 'older'");
    assert_eq!(
        cx.update(|_, cx| workspace.read(cx).active_challenge().cloned()),
        None
    );
}
