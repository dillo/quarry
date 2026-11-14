//! Copying results: ⌘C in the grid, Edit → Copy as …, and the note in the
//! status bar.

use gpui_kit::{Entity, Focusable, TestAppContext, VisualTestContext};
use quarry::{
    shell::actions::{CopyCsv, CopyJson, CopyTsv},
    views::workspace::{NOTE_TIME, Workspace},
};

use super::support::{drawn, open, run, settle, write};

const TWO_ROWS: &str = "SELECT 1 AS a, 'x' AS b UNION ALL SELECT 2, 'y'";

fn clipboard(cx: &mut VisualTestContext) -> Option<String> {
    cx.read_from_clipboard().and_then(|item| item.text())
}

fn note(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<String> {
    cx.update(|_, cx| workspace.read(cx).note().map(str::to_owned))
}

/// Runs [`TWO_ROWS`] and focuses the grid.
fn two_rows_in_the_grid(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) {
    write(workspace, cx, TWO_ROWS);
    run(workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        let handle = grid.read(cx).focus_handle(cx);
        window.focus(&handle, cx);
    });
}

/// Waits for the copy's note.
fn copied(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Option<String> {
    let shown = settle(cx, |_, cx| workspace.read(cx).note().is_some());
    shown.then(|| note(workspace, cx)).flatten()
}

#[gpui_kit::test]
fn command_c_in_the_grid_copies_every_row_as_tsv(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.simulate_keystrokes("cmd-c");
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 2 rows as TSV")
    );
    assert_eq!(clipboard(cx).as_deref(), Some("a\tb\n1\tx\n2\ty\n"));
    assert!(drawn(cx, "copy-note"));
}

#[gpui_kit::test]
fn copy_as_csv_copies_the_selected_row(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        grid.update(cx, |table, cx| table.set_selected_row(1, cx));
        window.dispatch_action(Box::new(CopyCsv), cx);
    });
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 1 row as CSV")
    );
    assert_eq!(clipboard(cx).as_deref(), Some("a,b\r\n2,y\r\n"));
}

#[gpui_kit::test]
fn copy_as_json_copies_the_selected_cell_from_the_editor(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        grid.update(cx, |table, cx| table.set_selected_cell(0, 1, cx));
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| state.focus(window, cx));
        window.dispatch_action(Box::new(CopyJson), cx);
    });
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 1 value as JSON")
    );
    assert_eq!(clipboard(cx).as_deref(), Some("\"x\""));
}

#[gpui_kit::test]
fn copy_as_json_copies_a_column(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| {
        let grid = workspace.read(cx).active_tab().read(cx).results().clone();
        grid.update(cx, |table, cx| table.set_selected_col(0, cx));
        window.dispatch_action(Box::new(CopyJson), cx);
    });
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 2 rows as JSON")
    );
    assert_eq!(
        clipboard(cx).as_deref(),
        Some("[\n  {\n    \"a\": 1\n  },\n  {\n    \"a\": 2\n  }\n]")
    );
}

#[gpui_kit::test]
fn nothing_to_copy_before_a_run_or_without_rows(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyTsv), cx));
    assert_eq!(copied(&workspace, cx).as_deref(), Some("Nothing to copy"));
    assert_eq!(clipboard(cx), None);
    write(&workspace, cx, "SELECT 1 WHERE 0");
    run(&workspace, cx);
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyTsv), cx));
    assert_eq!(copied(&workspace, cx).as_deref(), Some("Nothing to copy"));
    assert_eq!(clipboard(cx), None);
}

#[gpui_kit::test]
fn the_note_goes_after_three_seconds(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyTsv), cx));
    assert!(copied(&workspace, cx).is_some());
    cx.executor().advance_clock(NOTE_TIME);
    cx.run_until_parked();
    assert_eq!(note(&workspace, cx), None);
    assert!(!drawn(cx, "copy-note"));
}

#[gpui_kit::test]
fn a_newer_note_replaces_an_older_one_and_its_timer(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    two_rows_in_the_grid(&workspace, cx);
    cx.update(|window, cx| window.dispatch_action(Box::new(CopyCsv), cx));
    assert_eq!(
        copied(&workspace, cx).as_deref(),
        Some("Copied 2 rows as CSV")
    );
    cx.executor().advance_clock(NOTE_TIME / 2);
    cx.simulate_keystrokes("cmd-c");
    assert!(settle(cx, |_, cx| {
        workspace.read(cx).note() == Some("Copied 2 rows as TSV")
    }));
    cx.executor().advance_clock(NOTE_TIME / 2);
    cx.run_until_parked();
    assert_eq!(
        note(&workspace, cx).as_deref(),
        Some("Copied 2 rows as TSV")
    );
}

#[gpui_kit::test]
fn command_c_in_the_editor_copies_the_editor_text(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, TWO_ROWS);
    run(&workspace, cx);
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| {
            state.focus(window, cx);
            state.set_selected_range(0..6, cx);
        });
    });
    cx.simulate_keystrokes("cmd-c");
    cx.run_until_parked();
    assert_eq!(clipboard(cx).as_deref(), Some("SELECT"));
    assert_eq!(note(&workspace, cx), None);
}
