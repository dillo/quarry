//! Run all, Explain, the result views, and runs queued across tabs.

use gpui_kit::{TestAppContext, test::TestWindowExt};
use quarry::{
    model::{
        messages,
        plan_tree::{PlanRow, PlanState},
        statements::Phase,
    },
    views::results::statement_tab_id,
};
use quarry_engine::error::EngineError;

use super::support::{drawn, open, phase, run_with, settle, write};

const MIXED: &str = "CREATE TABLE notes (body TEXT);
INSERT INTO notes VALUES ('a'), ('b');
SELECT body FROM notes;
SELECT count(*) FROM astronauts;";

#[gpui_kit::test]
fn run_all_gives_each_row_returning_statement_a_result_tab(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, MIXED);
    run_with(&workspace, cx, "cmd-shift-r");
    assert_eq!(phase(&workspace, cx), Phase::Finished);
    assert!(!drawn(cx, statement_tab_id(0)));
    assert!(!drawn(cx, statement_tab_id(1)));
    assert!(drawn(cx, statement_tab_id(2)));
    assert!(drawn(cx, statement_tab_id(3)));
    let shown = |cx: &mut gpui_kit::VisualTestContext| {
        cx.update(|_, cx| {
            let tab = workspace.read(cx).active_tab().read(cx);
            let grid = tab.results().read(cx);
            (tab.view_id(), grid.delegate().shown())
        })
    };
    assert_eq!(shown(cx), ("results", 2));
    cx.update(|window, cx| window.click(statement_tab_id(3), cx));
    assert_eq!(shown(cx), ("results", 3));
}

#[gpui_kit::test]
fn every_statement_gets_a_message(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, MIXED);
    run_with(&workspace, cx, "cmd-shift-r");
    let lines = cx.update(|_, cx| messages::lines(workspace.read(cx).run_output(cx)));
    let texts: Vec<&str> = lines.iter().map(|line| line.text.as_str()).collect();
    assert_eq!(texts.len(), 4);
    assert!(texts[0].starts_with("Done · "), "{texts:?}");
    assert!(texts[1].starts_with("2 rows affected · "), "{texts:?}");
    assert!(texts[2].starts_with("2 rows · "), "{texts:?}");
    assert!(texts[3].starts_with("1 row · "), "{texts:?}");
}

#[gpui_kit::test]
fn a_script_without_rows_opens_on_its_messages(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(
        &workspace,
        cx,
        "CREATE TABLE a (x); INSERT INTO a VALUES (1);",
    );
    run_with(&workspace, cx, "cmd-shift-r");
    cx.update(|_, cx| {
        assert_eq!(
            workspace.read(cx).active_tab().read(cx).view_id(),
            "messages"
        );
    });
    assert!(drawn(cx, "messages"));
}

#[gpui_kit::test]
fn run_all_stops_at_the_first_error(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT 1; SELECT * FROM nowhere; SELECT 2;");
    run_with(&workspace, cx, "cmd-shift-r");
    let error = EngineError::Sql("no such table: nowhere".to_owned());
    assert_eq!(phase(&workspace, cx), Phase::Failed(error));
    let lines = cx.update(|_, cx| messages::lines(workspace.read(cx).run_output(cx)));
    assert_eq!(lines.len(), 2);
    assert!(!lines[0].is_error && lines[1].is_error);
    assert!(drawn(cx, "run-failure"));
}

#[gpui_kit::test]
fn run_all_on_an_empty_script_does_nothing(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "-- nothing to run");
    cx.simulate_keystrokes("cmd-shift-r");
    assert_eq!(phase(&workspace, cx), Phase::Idle);
}

#[gpui_kit::test]
fn the_view_switch_shows_messages(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    assert!(drawn(cx, ("segment", 1_usize)));
    cx.update(|window, cx| window.click(("segment", 1_usize), cx));
    cx.update(|_, cx| {
        assert_eq!(
            workspace.read(cx).active_tab().read(cx).view_id(),
            "messages"
        );
    });
}

#[gpui_kit::test]
fn a_run_in_another_tab_waits_for_the_running_one(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(
        &workspace,
        cx,
        "SELECT count(*) FROM telemetry a, telemetry b",
    );
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| {
        workspace.read(cx).run_output(cx).phase() == &Phase::Running
    });
    cx.simulate_keystrokes("cmd-t");
    cx.simulate_input("SELECT 1");
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, _| true);
    assert_eq!(phase(&workspace, cx), Phase::Waiting);
    cx.simulate_keystrokes("cmd-{ cmd-.");
    cx.simulate_keystrokes("cmd-}");
    settle(cx, |_, cx| {
        workspace.read(cx).run_output(cx).phase() == &Phase::Finished
    });
    assert_eq!(phase(&workspace, cx), Phase::Finished);
}

#[gpui_kit::test]
fn cancelling_a_queued_run_ends_it_while_the_other_tab_still_runs(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(
        &workspace,
        cx,
        "SELECT count(*) FROM telemetry a, telemetry b",
    );
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, cx| {
        workspace.read(cx).run_output(cx).phase() == &Phase::Running
    });
    cx.simulate_keystrokes("cmd-t");
    cx.simulate_input("SELECT 1");
    cx.simulate_keystrokes("cmd-enter");
    settle(cx, |_, _| true);
    assert_eq!(phase(&workspace, cx), Phase::Waiting);
    cx.simulate_keystrokes("cmd-.");
    assert_eq!(phase(&workspace, cx), Phase::Failed(EngineError::Cancelled));
    cx.simulate_keystrokes("cmd-{");
    assert_eq!(phase(&workspace, cx), Phase::Running);
}

#[gpui_kit::test]
fn command_e_shows_the_statement_plan(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT name FROM astronauts WHERE id = 3");
    cx.simulate_keystrokes("cmd-e");
    let plan = |cx: &mut gpui_kit::VisualTestContext| {
        cx.update(|_, cx| workspace.read(cx).active_tab().read(cx).plan().clone())
    };
    assert_eq!(plan(cx), PlanState::Planning);
    settle(cx, |_, cx| {
        matches!(
            workspace.read(cx).active_tab().read(cx).plan(),
            PlanState::Ready(_)
        )
    });
    let rows = vec![PlanRow {
        depth: 0,
        label: "SEARCH astronauts USING INTEGER PRIMARY KEY (rowid=?)".to_owned(),
    }];
    assert_eq!(plan(cx), PlanState::Ready(rows));
    cx.update(|_, cx| {
        assert_eq!(workspace.read(cx).active_tab().read(cx).view_id(), "plan");
    });
    assert!(drawn(cx, "plan"));
}

#[gpui_kit::test]
fn a_statement_that_cannot_be_planned_says_why(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    write(&workspace, cx, "SELECT * FROM nowhere");
    cx.simulate_keystrokes("cmd-e");
    settle(cx, |_, cx| {
        matches!(
            workspace.read(cx).active_tab().read(cx).plan(),
            PlanState::Failed(_)
        )
    });
    cx.update(|_, cx| {
        let tab = workspace.read(cx).active_tab().read(cx);
        assert_eq!(
            tab.plan(),
            &PlanState::Failed("no such table: nowhere".to_owned())
        );
        assert_eq!(tab.output(cx).phase(), &Phase::Idle, "explain runs nothing");
    });
}
