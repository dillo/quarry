//! Opening a workspace in a headless window and waiting for its runs.

use std::{sync::Arc, time::Duration};

use gpui_kit::{App, Entity, TestAppContext, VisualTestContext, Window, test::TestWindowExt};
use quarry::{
    model::statements::Phase,
    sample,
    shell::actions,
    views::workspace::{POLL, SavedTabs, Workspace},
};
use quarry_engine::error::EngineError;
use quarry_store::drafts::{DraftStore, Forgetful};
use quarry_ui_kit::{Aurora, Preference};

/// How long a test waits for the session thread before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

/// Initializes GPUI, Quarry's actions and the Dark theme.
pub fn install(cx: &mut TestAppContext) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
}

/// A workspace on a fresh sample database that keeps no drafts.
pub fn open(
    cx: &mut TestAppContext,
) -> Result<(Entity<Workspace>, &mut VisualTestContext), EngineError> {
    install(cx);
    open_with(cx, Arc::new(Forgetful))
}

/// A workspace on a fresh sample database saving to `store`, with its
/// editor focused. Call [`install`] first.
pub fn open_with(
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

/// Replaces the editor's text and puts the cursor at the end.
pub fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &str) {
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| {
            state.set_value(sql.to_owned(), window, cx);
            state.set_selected_range(sql.len()..sql.len(), cx);
        });
    });
}

/// Pumps the executor until `done` holds or [`PATIENCE`] runs out; the
/// session thread delivers events in real time.
pub fn settle(cx: &mut VisualTestContext, done: impl Fn(&mut Window, &mut App) -> bool) {
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

/// Presses `keys` and waits for the active tab's run to end.
pub fn run_with(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, keys: &str) {
    cx.simulate_keystrokes(keys);
    settle(cx, |_, cx| {
        let output = workspace.read(cx).run_output(cx);
        !output.is_running() && output.phase() != &Phase::Idle
    });
}

/// Runs the editor's statement with ⌘↵ and waits for the run to end.
pub fn run(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    run_with(workspace, cx, "cmd-enter");
}

/// The active tab's run phase.
pub fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
    cx.update(|_, cx| workspace.read(cx).run_output(cx).phase().clone())
}

/// Whether the element `id` is drawn, after a fresh draw.
pub fn drawn(cx: &mut VisualTestContext, id: impl Into<gpui_kit::ElementId>) -> bool {
    let id = id.into();
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(id).is_some()
    })
}
