//! Query tabs: opening, closing and switching them, and keeping their
//! drafts across restarts.

use std::{sync::Arc, time::Duration};

use gpui_kit::{App, Entity, TestAppContext, VisualTestContext, Window, test::TestWindowExt};
use quarry::{
    model::statements::Phase,
    sample,
    shell::actions,
    views::workspace::{AUTOSAVE, POLL, SavedTabs, Workspace, tab_element_id},
};
use quarry_engine::error::EngineError;
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};
use quarry_ui_kit::{Aurora, Preference};

/// How long a test waits for the session thread before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

/// A store whose every read and write fails.
struct Broken;

impl DraftStore for Broken {
    fn load(&self) -> Result<Drafts, StoreError> {
        Err(StoreError::Sql("disk I/O error".to_owned()))
    }

    fn save(&self, _: &Drafts) -> Result<(), StoreError> {
        Err(StoreError::Sql("disk I/O error".to_owned()))
    }
}

fn install(cx: &mut TestAppContext) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
}

/// A workspace on a fresh sample database, saving to `store`.
fn open(
    cx: &mut TestAppContext,
    store: Arc<dyn DraftStore>,
) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
    let session = sample::open()?;
    let saved = SavedTabs {
        store,
        problem: None,
    };
    let (workspace, cx) =
        cx.add_window_view(|window, cx| Workspace::new(session, sample::NAME, saved, window, cx));
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| state.focus(window, cx));
    });
    settle(cx, |_, _| true);
    Ok((workspace, cx))
}

fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
    let deadline = std::time::Instant::now() + PATIENCE;
    while std::time::Instant::now() < deadline {
        cx.executor().advance_clock(POLL);
        cx.run_until_parked();
        if cx.update(|window, cx| done(window, cx)) {
            return;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
}

fn titles(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| {
        let tabs = workspace.read(cx).tabs().tabs();
        tabs.iter().map(|tab| tab.title.clone()).collect()
    })
}

fn active(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> usize {
    cx.update(|_, cx| workspace.read(cx).tabs().active())
}

fn text(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> String {
    cx.update(|_, cx| workspace.read(cx).editor(cx).read(cx).value().to_string())
}

fn draft(id: i64, title: &str, text: &str) -> Draft {
    Draft {
        id,
        title: title.to_owned(),
        text: text.to_owned(),
    }
}

#[gpui_kit::test]
fn a_fresh_start_opens_query_one_with_the_welcome_query(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
    assert!(text(&workspace, cx).starts_with("-- Welcome to Quarry."));
}

#[gpui_kit::test]
fn command_t_opens_an_empty_tab_and_command_w_closes_it(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2"]);
    assert_eq!(active(&workspace, cx), 1);
    assert_eq!(text(&workspace, cx), "");
    cx.simulate_keystrokes("cmd-w");
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
    assert!(text(&workspace, cx).starts_with("-- Welcome"));
}

#[gpui_kit::test]
fn closing_the_only_tab_opens_a_fresh_one(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-w");
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
    assert_eq!(text(&workspace, cx), "");
}

#[gpui_kit::test]
fn next_and_previous_switch_tabs_and_wrap(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t cmd-t");
    assert_eq!(active(&workspace, cx), 2);
    cx.simulate_keystrokes("cmd-}");
    assert_eq!(active(&workspace, cx), 0);
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(active(&workspace, cx), 2);
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(active(&workspace, cx), 1);
}

#[gpui_kit::test]
fn clicking_a_tab_activates_it(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    let first = cx.update(|_, cx| workspace.read(cx).tabs().tabs()[0].id);
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.click(tab_element_id(first), cx);
    });
    assert_eq!(active(&workspace, cx), 0);
}

#[gpui_kit::test]
fn each_tab_keeps_its_own_results(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-enter");
    let finished = |cx: &mut VisualTestContext| {
        settle(cx, |_, cx| {
            workspace.read(cx).run_output(cx).phase() == &Phase::Finished
        });
    };
    finished(cx);
    cx.simulate_keystrokes("cmd-t");
    cx.simulate_input("SELECT 1");
    cx.simulate_keystrokes("cmd-enter");
    finished(cx);
    let rows = |cx: &mut VisualTestContext| {
        cx.update(|_, cx| {
            workspace.read(cx).run_output(cx).statements()[0]
                .rows()
                .len()
        })
    };
    assert_eq!(rows(cx), 1);
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(rows(cx), 12);
}

#[gpui_kit::test]
fn drafts_come_back_in_a_new_workspace(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    cx.simulate_input("SELECT 42");
    cx.executor().advance_clock(AUTOSAVE);
    settle(cx, |_, _| true);
    let saved = store.load().expect("the drafts load");
    assert_eq!(
        (saved.active, saved.tabs[1].text.as_str()),
        (1, "SELECT 42")
    );
    drop(workspace);
    let mut again = TestAppContext::single();
    install(&mut again);
    let (restored, again) = open(&mut again, store).expect("the sample database opens");
    assert_eq!(titles(&restored, again), ["Query 1", "Query 2"]);
    assert_eq!(
        (active(&restored, again), text(&restored, again)),
        (1, "SELECT 42".to_owned())
    );
}

#[gpui_kit::test]
fn an_edit_is_saved_only_after_a_pause(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    store
        .save(&Drafts {
            tabs: vec![draft(1, "Query 1", "")],
            active: 0,
        })
        .expect("a first save");
    let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_input("SELECT 7");
    cx.run_until_parked();
    assert_eq!(store.load().expect("the drafts load").tabs[0].text, "");
    cx.executor().advance_clock(AUTOSAVE);
    cx.run_until_parked();
    assert_eq!(
        store.load().expect("the drafts load").tabs[0].text,
        "SELECT 7"
    );
}

#[gpui_kit::test]
fn a_failing_store_shows_a_warning(cx: &mut TestAppContext) {
    install(cx);
    let (workspace, cx) = open(cx, Arc::new(Broken)).expect("the sample database opens");
    cx.update(|window, cx| {
        assert_eq!(workspace.read(cx).drafts_problem(), Some("disk I/O error"));
        window.draw(cx).clear(cx);
        assert!(window.try_find("drafts-warning").is_some());
    });
}
