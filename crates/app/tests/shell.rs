//! Start-up: the main window opens on the sample database, the menus come
//! from the command table, and commands have key bindings.

use std::{cell::Cell, path::Path, rc::Rc};

use gpui_kit::{MenuItem, TestAppContext, px};
use quarry::shell::{actions, store, window};
use quarry_store::drafts::Drafts;
use quarry_ui_kit::{Aurora, Preference};

#[gpui_kit::test]
fn the_main_window_opens_on_the_sample(cx: &mut TestAppContext) {
    let opened = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        assert_eq!(Aurora::install(Preference::Dark, cx), Ok(()));
        window::open_main(store::saved_tabs(None), cx)
    });
    assert_eq!(opened, Ok(()));
    assert_eq!(cx.windows().len(), 1);
}

#[gpui_kit::test]
fn the_window_has_a_transparent_titlebar_and_the_default_size(cx: &mut TestAppContext) {
    let options = cx.update(|cx| window::options(cx));
    let titlebar = options.titlebar.unwrap_or_default();
    assert!(titlebar.appears_transparent);
    assert_eq!(titlebar.title.as_deref(), Some("Quarry"));
    let (width, height) = window::WINDOW_SIZE;
    let bounds = options.window_bounds.map(|b| b.get_bounds().size);
    assert_eq!(
        bounds.map(|s| (s.width, s.height)),
        Some((px(width), px(height)))
    );
}

#[test]
fn every_command_but_the_other_copy_formats_has_a_key_binding() {
    let bound: Vec<&str> = actions::key_bindings()
        .iter()
        .map(|binding| binding.action().name())
        .collect();
    let unbound = ["quarry::CopyCsv", "quarry::CopyJson"];
    let commands = actions::commands();
    assert_eq!(commands.len(), 19);
    for command in commands {
        let name = (command.action)().name();
        assert!(
            bound.contains(&name) != unbound.contains(&name),
            "{name} has no key binding"
        );
    }
}

#[test]
fn the_menus_are_the_command_table_in_order() {
    let menus = actions::menus();
    let names: Vec<&str> = menus.iter().map(|menu| menu.name.as_ref()).collect();
    assert_eq!(names, ["Quarry", "File", "Edit", "Query", "View"]);
    let items: Vec<String> = menus
        .iter()
        .flat_map(|menu| &menu.items)
        .filter_map(|item| match item {
            MenuItem::Action { name, .. } => Some(name.to_string()),
            _ => None,
        })
        .collect();
    let labels: Vec<&str> = actions::commands()
        .iter()
        .map(|command| command.label)
        .collect();
    assert_eq!(items, labels);
    assert_eq!(items[3..6], ["Copy as TSV", "Copy as CSV", "Copy as JSON"]);
    assert_eq!(
        items[6..11],
        [
            "Run Statement",
            "Run All",
            "Check Answer",
            "Explain",
            "Cancel"
        ]
    );
}

#[test]
fn the_store_lives_in_application_support() {
    let path = store::default_path();
    let path = path.as_deref().map(Path::to_string_lossy);
    assert!(
        path.as_deref()
            .is_some_and(|p| p.ends_with("Library/Application Support/Quarry/quarry.db")),
        "{path:?}"
    );
}

#[test]
fn a_store_that_cannot_open_keeps_nothing_and_says_why() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, "not a folder").expect("a file");
    let saved = store::saved_tabs(Some(&blocker.join("quarry.db")));
    assert!(saved.problem.is_some_and(|p| p.starts_with("Cannot open")));
    assert_eq!(saved.store.load(), Ok(Drafts::default()));
    assert_eq!(saved.store.save(&Drafts::default()), Ok(()));
    let none = store::saved_tabs(None);
    assert_eq!(
        none.problem.as_deref(),
        Some("There is no home folder to save drafts in.")
    );
}

#[test]
fn a_store_that_opens_has_no_problem() {
    let dir = tempfile::tempdir().expect("a temporary folder");
    let saved = store::saved_tabs(Some(&dir.path().join("quarry.db")));
    assert_eq!(saved.problem, None);
}

#[gpui_kit::test]
fn closing_the_last_window_is_noticed(cx: &mut TestAppContext) {
    let closed = Rc::new(Cell::new(false));
    let seen = Rc::clone(&closed);
    cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        assert_eq!(Aurora::install(Preference::Dark, cx), Ok(()));
        window::on_last_window_closed(cx, move |_| seen.set(true));
        assert_eq!(window::open_main(store::saved_tabs(None), cx), Ok(()));
    });
    let windows = cx.windows();
    assert!(!closed.get());
    for window in windows {
        assert!(
            window
                .update(cx, |_, window, _| window.remove_window())
                .is_ok()
        );
    }
    cx.run_until_parked();
    assert!(closed.get());
}
