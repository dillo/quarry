//! The workspace in a headless window: running, cancelling, the schema tree
//! and the window commands.

use std::time::Duration;

use gpui_kit::{
    App, Entity, Focusable, TestAppContext, VisualTestContext, Window,
    test::{TestEventExt, TestWindowExt},
};
use quarry::{
    model::statements::Phase,
    sample,
    shell::actions,
    views::{
        schema::table_row_id,
        workspace::{POLL, Workspace},
    },
};
use quarry_engine::error::EngineError;
use quarry_ui_kit::{Aurora, Preference};

/// How long a test waits for the session thread before giving up.
const PATIENCE: Duration = Duration::from_secs(10);

fn open(cx: &mut TestAppContext) -> (Entity<Workspace>, &mut VisualTestContext) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let (workspace, cx) = cx.add_window_view(|window, cx| {
        let session = sample::open()
            .unwrap_or_else(|error| unreachable!("the sample database opens: {error}"));
        Workspace::new(session, sample::NAME, window, cx)
    });
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| state.focus(window, cx));
    });
    settle(cx, |_, _| true);
    (workspace, cx)
}

/// Replaces the editor's text and puts the cursor at the end.
fn write(workspace: &Entity<Workspace>, cx: &mut VisualTestContext, sql: &str) {
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| {
            state.set_value(sql.to_owned(), window, cx);
            state.set_selected_range(sql.len()..sql.len(), cx);
        });
    });
}

/// Pumps the executor until `done` holds or [`PATIENCE`] runs out; the
/// session thread delivers events in real time.
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

/// Runs the editor's statement with ⌘↵ and waits for the run to end.
fn run(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| {
        let output = workspace.read(cx).run_output(cx);
        !output.is_running() && output.phase() != &Phase::Idle
    });
}

fn phase(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Phase {
    cx.update(|_, cx| workspace.read(cx).run_output(cx).phase().clone())
}

#[gpui_kit::test]
fn the_editor_never_has_a_completion_provider(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.update(|_, cx| {
        let editor = workspace.read(cx).editor().read(cx);
        let lsp = editor.lsp();
        assert!(lsp.completion_provider.is_none());
        assert!(lsp.hover_provider.is_none());
        assert!(lsp.definition_provider.is_none());
        assert!(lsp.code_action_providers.is_empty());
    });
}

#[gpui_kit::test]
fn result_columns_cannot_be_dragged_out_of_order(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT name, nationality FROM astronauts");
    run(&workspace, cx);
    cx.update(|_, cx| assert!(!workspace.read(cx).columns_movable(cx)));
}

#[gpui_kit::test]
fn command_return_in_the_editor_runs_the_statement(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT name FROM astronauts ORDER BY id");
    run(&workspace, cx);
    assert_eq!(phase(&workspace, cx), Phase::Finished);
    cx.update(|_, cx| {
        let statement = &workspace.read(cx).run_output(cx).statements()[0];
        assert_eq!(statement.rows().len(), 12);
        assert_eq!(statement.columns()[0].name, "name");
    });
}

#[gpui_kit::test]
fn command_return_leaves_the_editor_text_alone(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT 1");
    run(&workspace, cx);
    cx.update(|_, cx| {
        let text = workspace.read(cx).editor().read(cx).value();
        assert_eq!(text.as_ref(), "SELECT 1");
    });
}

#[gpui_kit::test]
fn a_bad_statement_shows_the_database_error(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "SELECT * FROM nowhere");
    run(&workspace, cx);
    assert_eq!(
        phase(&workspace, cx),
        Phase::Failed(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        assert!(window.try_find("run-failure").is_some());
    });
}

#[gpui_kit::test]
fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(
        &workspace,
        cx,
        "SELECT count(*) FROM telemetry a, telemetry b",
    );
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| workspace.read(cx).run_output(cx).is_running());
    cx.simulate_keystrokes("cmd-.");
    settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
    assert_eq!(phase(&workspace, cx), Phase::Failed(EngineError::Cancelled));
}

#[gpui_kit::test]
fn running_again_during_a_run_replaces_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(
        &workspace,
        cx,
        "SELECT count(*) FROM telemetry a, telemetry b",
    );
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| workspace.read(cx).run_output(cx).is_running());
    write(&workspace, cx, "SELECT 1");
    run(&workspace, cx);
    assert_eq!(phase(&workspace, cx), Phase::Finished);
    cx.update(|_, cx| {
        let output = workspace.read(cx).run_output(cx);
        assert_eq!(output.statements()[0].rows().len(), 1);
    });
}

#[gpui_kit::test]
fn creating_a_table_adds_it_to_the_schema_tree(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    write(&workspace, cx, "CREATE TABLE notes (body TEXT)");
    run(&workspace, cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("notes")).is_some()
    });
    cx.update(|window, _| assert!(window.try_find(table_row_id("notes")).is_some()));
}

#[gpui_kit::test]
fn clicking_a_table_expands_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("missions")).is_some()
    });
    cx.update(|window, cx| window.click(table_row_id("missions"), cx));
    cx.update(|_, cx| {
        let tree = workspace.read(cx).schema().read(cx);
        assert!(tree.expanded().contains("missions"));
    });
}

#[gpui_kit::test]
fn command_backslash_toggles_the_sidebar(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(!workspace.read(cx).sidebar_open()));
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(workspace.read(cx).sidebar_open()));
}

#[gpui_kit::test]
fn command_shift_t_cycles_the_theme(cx: &mut TestAppContext) {
    let (_, cx) = open(cx);
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::Light));
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::System));
}

#[gpui_kit::test]
fn command_one_focuses_the_first_table_and_return_expands_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        window.try_find(table_row_id("astronauts")).is_some()
    });
    cx.simulate_keystrokes("cmd-1");
    cx.update(|window, cx| {
        window.draw(cx).clear(cx);
        window.key_down("enter", false, cx);
        window.key_up("enter", cx);
    });
    cx.update(|_, cx| {
        let tree = workspace.read(cx).schema().read(cx);
        assert!(tree.expanded().contains("astronauts"));
    });
}

#[gpui_kit::test]
fn command_two_returns_focus_to_the_editor(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx);
    cx.simulate_keystrokes("cmd-1");
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        true
    });
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().read(cx);
        assert!(
            !editor.focus_handle(cx).is_focused(window),
            "cmd-1 left focus in the editor"
        );
    });
    cx.simulate_keystrokes("cmd-2");
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor().read(cx);
        assert!(editor.focus_handle(cx).is_focused(window));
    });
}
