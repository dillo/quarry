//! Query tabs: opening, closing and switching them, and keeping their
//! drafts across restarts.

use std::sync::{
    Arc, Mutex,
    atomic::{AtomicBool, AtomicUsize, Ordering},
};

use gpui_kit::{
    Entity, TestAppContext, VisualTestContext, component::WindowExt, test::TestWindowExt,
};
use quarry::{
    model::{save_order::SaveOrder, statements::Phase},
    views::workspace::{AUTOSAVE, Workspace, save_in_order, tab_element_id},
};
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};

use super::support::{drawn, install, open_with as open, settle, write};

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
fn command_w_activates_the_right_hand_neighbour(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t cmd-t cmd-{");
    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2", "Query 3"]);
    assert_eq!(active(&workspace, cx), 1);
    cx.simulate_keystrokes("cmd-w");
    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 3"]);
    assert_eq!(active(&workspace, cx), 1);
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
        assert!(settle(cx, |_, cx| {
            workspace.read(cx).run_output(cx).phase() == &Phase::Finished
        }));
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
    assert!(settle(cx, |_, _| true));
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
fn the_closed_tabs_are_kept_through_a_save(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let closed = vec![draft(0, "Query 4", "SELECT 4")];
    let saved = Drafts {
        tabs: vec![draft(1, "Query 1", "SELECT 1")],
        active: 0,
        closed: closed.clone(),
    };
    store.save(&saved).expect("a first save");
    let (_workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    assert!(settle(cx, |_, _| true));
    let loaded = store.load().expect("the drafts load");
    assert_eq!(loaded.tabs.len(), 2);
    assert_eq!(loaded.closed, closed);
}

#[gpui_kit::test]
fn an_edit_is_saved_only_after_a_pause(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    store
        .save(&Drafts {
            tabs: vec![draft(1, "Query 1", "")],
            active: 0,
            closed: Vec::new(),
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
    assert!(settle(cx, |_, _| true));
    cx.simulate_keystrokes("cmd-t");
    assert!(settle(cx, |_, _| true));
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
    assert!(settle(cx, |_, _| true));
    assert!(drawn(cx, "drafts-warning"));
    cx.simulate_input(" + 1");
    cx.executor().advance_clock(AUTOSAVE);
    assert!(settle(cx, |_, _| true));
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

#[test]
fn a_delayed_older_snapshot_is_skipped() {
    let store = SqliteStore::in_memory().expect("an in-memory store");
    let saves = Mutex::new(SaveOrder::default());
    let (older, newer) = {
        let mut order = saves.lock().expect("an unpoisoned lock");
        (order.issue(), order.issue())
    };
    let snapshot = |text: &str| Drafts {
        tabs: vec![draft(1, "Query 1", text)],
        active: 0,
        closed: Vec::new(),
    };
    assert_eq!(
        save_in_order(&store, &saves, newer, &snapshot("newer")),
        Some(Ok(()))
    );
    assert_eq!(
        save_in_order(&store, &saves, older, &snapshot("older")),
        None
    );
    assert_eq!(store.load(), Ok(snapshot("newer")));
}

fn closed_titles(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<String> {
    cx.update(|_, cx| {
        let closed = workspace.read(cx).closed_tabs();
        closed.iter().map(|draft| draft.title.clone()).collect()
    })
}

/// Opens the palette, types `query` and picks the first match.
fn pick(cx: &mut VisualTestContext, query: &str) {
    cx.simulate_keystrokes("cmd-k");
    cx.simulate_input(query);
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
}

#[gpui_kit::test]
fn a_closed_tab_reopens_from_the_palette_and_saves_at_once(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t cmd-t cmd-{");
    write(&workspace, cx, "\n  SELECT 42\nFROM astronauts");
    cx.simulate_keystrokes("cmd-w cmd-{");
    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 3"]);
    assert_eq!(closed_titles(&workspace, cx), ["Query 2"]);
    pick(cx, "query 2");
    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2", "Query 3"]);
    assert_eq!(active(&workspace, cx), 1);
    assert_eq!(text(&workspace, cx), "\n  SELECT 42\nFROM astronauts");
    assert!(closed_titles(&workspace, cx).is_empty());
    let saved = store.load().expect("the drafts load");
    assert_eq!((saved.tabs.len(), saved.closed.len()), (3, 0));
}

#[gpui_kit::test]
fn the_palette_shows_a_closed_tab_with_its_first_line(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    write(&workspace, cx, "\n  SELECT 42\nFROM astronauts");
    cx.simulate_keystrokes("cmd-w cmd-k");
    cx.run_until_parked();
    let group = cx.update(|_, cx| {
        let palette = workspace.read(cx).palette().expect("the palette is open");
        palette.read(cx).groups()[1].clone()
    });
    assert_eq!(group.title, "Recently closed");
    assert_eq!(group.items[0].label, "Query 2");
    assert_eq!(group.items[0].detail, "SELECT 42");
}

#[gpui_kit::test]
fn closing_the_only_tab_remembers_it(cx: &mut TestAppContext) {
    install(cx);
    let store = Arc::new(SqliteStore::in_memory().expect("an in-memory store"));
    let (workspace, cx) = open(cx, store.clone()).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-w");
    assert_eq!(titles(&workspace, cx), ["Query 1"]);
    let closed = cx.update(|_, cx| workspace.read(cx).closed_tabs().to_vec());
    assert_eq!(closed.len(), 1);
    assert!(closed[0].text.starts_with("-- Welcome"));
    assert!(settle(cx, |_, _| true));
    assert_eq!(store.load().expect("the drafts load").closed.len(), 1);
}

#[gpui_kit::test]
fn reopening_works_while_drafts_are_not_saved(cx: &mut TestAppContext) {
    let (workspace, cx) = super::support::open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    write(&workspace, cx, "SELECT 7");
    cx.simulate_keystrokes("cmd-w");
    pick(cx, "query 2");
    assert_eq!(titles(&workspace, cx), ["Query 1", "Query 2"]);
    assert_eq!(text(&workspace, cx), "SELECT 7");
}
