//! Start-up: the main window opens on the sample database, and every command
//! has a key binding and a menu item.

use gpui_kit::{MenuItem, TestAppContext, px};
use quarry::shell::{actions, window};
use quarry_ui_kit::{Aurora, Preference};

#[gpui_kit::test]
fn the_main_window_opens_on_the_sample(cx: &mut TestAppContext) {
    let opened = cx.update(|cx| {
        gpui_kit::init(cx);
        actions::install(cx);
        assert_eq!(Aurora::install(Preference::Dark, cx), Ok(()));
        window::open_main(cx)
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
fn every_menu_command_has_a_key_binding() {
    let bound: Vec<&str> = actions::key_bindings()
        .iter()
        .map(|binding| binding.action().name())
        .collect();
    let menus = actions::menus();
    let commands: Vec<&str> = menus
        .iter()
        .flat_map(|menu| &menu.items)
        .filter_map(|item| match item {
            MenuItem::Action { action, .. } => Some(action.name()),
            _ => None,
        })
        .collect();
    assert_eq!(commands.len(), 7);
    for command in commands {
        assert!(bound.contains(&command), "{command} has no key binding");
    }
}
