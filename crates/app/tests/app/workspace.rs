//! The workspace in a headless window: running, cancelling, the schema tree
//! and the window commands.

use gpui_kit::{
    Focusable, TestAppContext,
    test::{TestEventExt, TestWindowExt},
};
use quarry::{
    model::statements::Phase,
    views::{query_tab::RunScope, schema::table_row_id},
};
use quarry_engine::{error::EngineError, session::RunEvent};
use quarry_ui_kit::{Aurora, Preference};

use super::support::{drawn, open, phase, run, settle, write};

#[gpui_kit::test]
fn the_editor_never_has_a_completion_provider(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.update(|_, cx| {
        let editor = workspace.read(cx).editor(cx);
        let editor = editor.read(cx);
        let lsp = editor.lsp();
        assert!(lsp.completion_provider.is_none());
        assert!(lsp.hover_provider.is_none());
        assert!(lsp.definition_provider.is_none());
        assert!(lsp.code_action_providers.is_empty());
    });
}

#[gpui_kit::test]
fn result_columns_cannot_be_dragged_out_of_order(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT name, nationality FROM astronauts");
    run(&workspace, cx);
    cx.update(|_, cx| {
        let tab = workspace.read(cx).active_tab().read(cx);
        assert!(!tab.results().read(cx).col_movable);
    });
}

#[gpui_kit::test]
fn command_return_in_the_editor_runs_the_statement(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
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
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT 1");
    run(&workspace, cx);
    cx.update(|_, cx| {
        let text = workspace.read(cx).editor(cx).read(cx).value();
        assert_eq!(text.as_ref(), "SELECT 1");
    });
}

#[gpui_kit::test]
fn a_bad_statement_shows_the_database_error(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT * FROM nowhere");
    run(&workspace, cx);
    assert_eq!(
        phase(&workspace, cx),
        Phase::Failed(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    assert!(drawn(cx, "run-failure"));
}

#[gpui_kit::test]
fn command_period_cancels_a_long_query(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
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
    let (workspace, cx) = open(cx).expect("the sample database opens");
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
    let (workspace, cx) = open(cx).expect("the sample database opens");
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
    let (workspace, cx) = open(cx).expect("the sample database opens");
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
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(!workspace.read(cx).sidebar_open()));
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(workspace.read(cx).sidebar_open()));
}

#[gpui_kit::test]
fn command_shift_t_cycles_the_theme(cx: &mut TestAppContext) {
    let (_, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::Light));
    cx.simulate_keystrokes("cmd-shift-t");
    cx.update(|_, cx| assert_eq!(Aurora::preference(cx), Preference::System));
}

#[gpui_kit::test]
fn command_one_focuses_the_first_table_and_return_expands_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
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
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-1");
    settle(cx, |window, cx| {
        window.draw(cx).clear(cx);
        true
    });
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        let editor = editor.read(cx);
        assert!(
            !editor.focus_handle(cx).is_focused(window),
            "cmd-1 left focus in the editor"
        );
    });
    cx.simulate_keystrokes("cmd-2");
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        let editor = editor.read(cx);
        assert!(editor.focus_handle(cx).is_focused(window));
    });
}

#[gpui_kit::test]
fn a_run_whose_channel_closes_without_done_reports_a_lost_worker(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    let (sender, events) = async_channel::unbounded();
    let started = RunEvent::Started {
        index: 0,
        range: 0..8,
    };
    assert!(sender.try_send(started).is_ok());
    drop(sender);
    cx.update(|_, cx| {
        let tab = workspace.read(cx).active_tab().clone();
        tab.update(cx, |tab, cx| {
            tab.follow("SELECT 1".to_owned(), RunScope::Statement, events, cx);
        });
    });
    settle(cx, |_, cx| !workspace.read(cx).run_output(cx).is_running());
    assert_eq!(
        phase(&workspace, cx),
        Phase::Failed(EngineError::WorkerGone)
    );
    assert!(drawn(cx, "run-failure"));
}

#[gpui_kit::test]
fn a_new_run_clears_the_grid_selection(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT * FROM astronauts");
    run(&workspace, cx);
    let results = cx.update(|_, cx| workspace.read(cx).active_tab().read(cx).results().clone());
    cx.update(|_, cx| results.update(cx, |table, cx| table.set_selected_row(5, cx)));
    write(&workspace, cx, "SELECT 1");
    run(&workspace, cx);
    cx.update(|_, cx| assert_eq!(results.read(cx).selected_row(), None));
}
