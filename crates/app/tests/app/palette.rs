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
fn an_empty_query_lists_open_tabs_then_commands(cx: &mut TestAppContext) {
    let (workspace, cx) = open(cx).expect("the sample database opens");
    cx.simulate_keystrokes("cmd-t");
    search(cx, "");
    let groups = groups(&workspace, cx);
    let headings: Vec<&str> = groups.iter().map(|(title, _)| title.as_str()).collect();
    assert_eq!(headings, ["Open tabs", "Commands"]);
    assert_eq!(groups[0].1, ["Query 1", "Query 2"]);
    assert_eq!(groups[1].1[..3], ["New Tab", "Close Tab", "Copy as TSV"]);
    assert!(!groups[1].1.contains(&"Quit Quarry".to_owned()));
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
