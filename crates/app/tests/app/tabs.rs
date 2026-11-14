//! Query tabs: opening, closing and switching them, and keeping their
//! drafts across restarts.

use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use gpui_kit::{Entity, TestAppContext, VisualTestContext, test::TestWindowExt};
use quarry::{
    model::statements::Phase,
    views::workspace::{AUTOSAVE, Workspace, tab_element_id},
};
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};

use super::support::{drawn, install, open_with as open, settle};

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
    cx.update(|_, cx| {
        assert_eq!(workspace.read(cx).drafts_problem(), Some("disk I/O error"));
    });
    assert!(drawn(cx, "drafts-warning"));
}

/// A store whose load fails but whose save would succeed, noting any save.
struct Unreadable {
    saved: Arc<AtomicBool>,
}

impl DraftStore for Unreadable {
    fn load(&self) -> Result<Drafts, StoreError> {
        Err(StoreError::Sql("file is not a database".to_owned()))
    }

    fn save(&self, _: &Drafts) -> Result<(), StoreError> {
        self.saved.store(true, Ordering::SeqCst);
        Ok(())
    }
}

#[gpui_kit::test]
fn a_store_that_failed_to_load_is_never_saved_over(cx: &mut TestAppContext) {
    install(cx);
    let saved = Arc::new(AtomicBool::new(false));
    let store = Arc::new(Unreadable {
        saved: Arc::clone(&saved),
    });
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_input("SELECT 1");
    cx.executor().advance_clock(AUTOSAVE);
    settle(cx, |_, _| true);
    cx.simulate_keystrokes("cmd-t");
    settle(cx, |_, _| true);
    assert!(!saved.load(Ordering::SeqCst));
    cx.update(|_, cx| {
        assert_eq!(
            workspace.read(cx).drafts_problem(),
            Some("file is not a database")
        );
    });
}

/// A store whose first save fails and whose later saves succeed.
struct Flaky {
    saves: AtomicUsize,
}

impl DraftStore for Flaky {
    fn load(&self) -> Result<Drafts, StoreError> {
        Ok(Drafts::default())
    }

    fn save(&self, _: &Drafts) -> Result<(), StoreError> {
        match self.saves.fetch_add(1, Ordering::SeqCst) {
            0 => Err(StoreError::Sql("disk full".to_owned())),
            _ => Ok(()),
        }
    }
}

#[gpui_kit::test]
fn the_warning_clears_after_a_later_save_succeeds(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(Flaky {
        saves: AtomicUsize::new(0),
    });
    let (_workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_input("SELECT 1");
    cx.executor().advance_clock(AUTOSAVE);
    settle(cx, |_, _| true);
    assert!(drawn(cx, "drafts-warning"));
    cx.simulate_input(" + 1");
    cx.executor().advance_clock(AUTOSAVE);
    settle(cx, |_, _| true);
    assert!(!drawn(cx, "drafts-warning"));
}

#[gpui_kit::test]
fn closing_the_window_saves_the_last_edits(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_input("SELECT 9");
    cx.update(|window, _| window.remove_window());
    cx.run_until_parked();
    let loaded = store.load().expect("the drafts load");
    assert!(loaded.tabs[0].text.starts_with("SELECT 9"));
}
