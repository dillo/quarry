//! The ⌘K palette: opening and closing it, filtering, and picking from
//! each source.

use gpui_kit::{Entity, Focusable, TestAppContext, VisualTestContext, component::WindowExt};
use quarry::{
    shell::actions::CloseTab,
    views::{palette::dialog::PaletteGroup, workspace::Workspace},
};

use super::support::{drawn, open, phase, settle};

/// Whether the editor of the active tab has focus.
fn editor_focused(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> bool {
    cx.update(|window, cx| {
        let editor = workspace.read(cx).editor(cx);
        editor
            .read(cx)
            .focus_handle(cx)
            .contains_focused(window, cx)
    })
}

fn is_open(cx: &mut VisualTestContext) -> bool {
    cx.update(WindowExt::has_active_dialog)
}

/// The palette's groups as (heading, labels).
fn groups(workspace: &Entity<Workspace>, cx: &mut VisualTestContext) -> Vec<(String, Vec<String>)> {
    cx.update(|_, cx| {
        let palette = workspace.read(cx).palette();
        let groups: Vec<PaletteGroup> = palette.map(|p| p.read(cx).groups()).unwrap_or_default();
        groups
            .into_iter()
            .map(|group| {
                let labels = group.items.into_iter().map(|item| item.label).collect();
                (group.title.to_owned(), labels)
            })
            .collect()
    })
}

/// Opens the palette with ⌘K and types `query`.
fn search(cx: &mut VisualTestContext, query: &str) {
    cx.simulate_keystrokes("cmd-k");
    cx.run_until_parked();
    if !query.is_empty() {
        cx.simulate_input(query);
        cx.run_until_parked();
    }
}

#[gpui_kit::test]
fn command_k_opens_the_palette_from_the_editor_and_escape_closes_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    assert!(editor_focused(&workspace, cx));
    search(cx, "");
    assert!(is_open(cx));
    assert!(drawn(cx, "palette"));
    assert!(!editor_focused(&workspace, cx));
    cx.simulate_keystrokes("escape");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    assert!(editor_focused(&workspace, cx));
}

#[gpui_kit::test]
fn command_k_again_closes_the_palette(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "");
    cx.simulate_keystrokes("cmd-k");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    assert!(editor_focused(&workspace, cx));
}

#[gpui_kit::test]
fn an_empty_query_lists_tabs_then_challenges_tables_and_commands(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    search(cx, "");
    let groups = groups(&workspace, cx);
    let headings: Vec<&str> = groups.iter().map(|(title, _)| title.as_str()).collect();
    assert_eq!(
        headings,
        ["Open tabs", "Challenges", "Tables & views", "Commands"]
    );
    assert_eq!(groups[0].1, ["Query 1", "Query 2"]);
    assert!(groups[2].1.contains(&"astronauts".to_owned()));
    assert_eq!(groups[3].1[..3], ["New Tab", "Close Tab", "Copy as TSV"]);
    assert!(!groups[3].1.contains(&"Quit Quarry".to_owned()));
    let current = cx.update(|_, cx| {
        let palette = workspace.read(cx).palette();
        palette.map(|p| p.read(cx).groups()[0].items[1].detail.clone())
    });
    assert_eq!(current.as_deref(), Some("current"));
}

#[gpui_kit::test]
fn typing_hides_groups_without_matches_and_says_when_nothing_matches(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "sidebar");
    let found = groups(&workspace, cx);
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].1, ["Toggle Sidebar", "Focus Sidebar"]);
    assert!(!drawn(cx, "palette-empty"));
    cx.simulate_input("zzz");
    cx.run_until_parked();
    assert!(groups(&workspace, cx).is_empty());
    assert!(drawn(cx, "palette-empty"));
}

#[gpui_kit::test]
fn picking_an_open_tab_activates_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    search(cx, "query 1");
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    cx.update(|_, cx| assert_eq!(workspace.read(cx).tabs().active(), 0));
    assert!(editor_focused(&workspace, cx));
}

#[gpui_kit::test]
fn picking_a_command_runs_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "run all");
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |_, cx| {
        let output = workspace.read(cx).run_output(cx);
        !output.is_running() && output.statements().len() == 1
    }));
    assert_eq!(
        phase(&workspace, cx),
        quarry::model::statements::Phase::Finished
    );
}

#[gpui_kit::test]
fn picking_a_tab_closed_while_the_palette_was_open_does_nothing(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    search(cx, "query 2");
    cx.update(|window, cx| {
        let focus = workspace.read(cx).focus_handle(cx);
        focus.dispatch_action(&CloseTab, window, cx);
    });
    cx.update(|_, cx| assert_eq!(workspace.read(cx).tabs().tabs().len(), 1));
    cx.update(|window, cx| {
        let palette = workspace.read(cx).palette().expect("the palette is open");
        let state = palette.read(cx).state().clone();
        state.update(cx, |state, cx| state.focus(window, cx));
    });
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    cx.update(|_, cx| assert_eq!(workspace.read(cx).tabs().tabs().len(), 1));
}

#[gpui_kit::test]
fn tables_and_views_show_their_kind_and_columns(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "mission_crews");
    let detail = cx.update(|_, cx| {
        let palette = workspace.read(cx).palette();
        palette.map(|p| p.read(cx).groups()[0].items[0].detail.clone())
    });
    assert!(detail.is_some_and(|d| d.starts_with("view ") && d.ends_with(" columns")));
}

#[gpui_kit::test]
fn picking_a_table_opens_the_sidebar_and_focuses_its_row(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-\\");
    cx.update(|_, cx| assert!(!workspace.read(cx).sidebar_open()));
    search(cx, "telemetry");
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |window, cx| {
        let schema = workspace.read(cx).schema().read(cx);
        schema.focused_table(window) == Some("telemetry")
    }));
    cx.update(|_, cx| {
        let workspace = workspace.read(cx);
        assert!(workspace.sidebar_open());
        assert!(workspace.schema().read(cx).expanded().contains("telemetry"));
    });
}

#[gpui_kit::test]
fn a_table_that_is_gone_focuses_nothing(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.update(|window, cx| {
        workspace.update(cx, |workspace, cx| {
            workspace.reveal_table("nowhere", window, cx);
        });
    });
    assert!(settle(cx, |_, _| true));
    cx.update(|window, cx| {
        let schema = workspace.read(cx).schema().read(cx);
        assert_eq!(schema.focused_table(window), None);
        assert!(schema.expanded().is_empty());
    });
}

#[gpui_kit::test]
fn challenges_are_listed_in_play_order_with_their_track_and_place(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "");
    let challenges = cx.update(|_, cx| {
        let palette = workspace.read(cx).palette();
        let groups = palette.map(|p| p.read(cx).groups()).unwrap_or_default();
        groups
            .into_iter()
            .find(|group| group.title == "Challenges")
            .map(|group| group.items)
            .unwrap_or_default()
    });
    let shown: Vec<(&str, &str)> = challenges
        .iter()
        .map(|item| (item.label.as_str(), item.detail.as_str()))
        .collect();
    assert_eq!(shown[0], ("First spin", "Vinyl Vault · 1 of 5"));
    assert_eq!(shown[4], ("Unknown origins", "Vinyl Vault · 5 of 5"));
}

#[gpui_kit::test]
fn picking_a_challenge_opens_it(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    search(cx, "crate digging");
    cx.simulate_keystrokes("enter");
    assert!(settle(cx, |window, cx| !window.has_active_dialog(cx)));
    let opened = cx.update(|_, cx| {
        workspace
            .read(cx)
            .active_challenge()
            .map(ToString::to_string)
    });
    assert_eq!(opened.as_deref(), Some("vinyl-vault/crate-digging"));
    assert!(editor_focused(&workspace, cx));
}
