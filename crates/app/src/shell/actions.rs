//! Every command, its key binding and its menu.
//!
//! Adding a command means declaring it in `actions!`, adding one row to
//! [`key_bindings`] and one menu item to [`menus`].

use gpui_kit::{App, KeyBinding, Menu, MenuItem, actions};

actions!(
    quarry,
    [
        /// Runs the selection, or the statement under the cursor.
        RunStatement,
        /// Runs the whole script, statement by statement.
        RunAll,
        /// Shows how the database would run the current statement.
        Explain,
        /// Cancels the running query.
        CancelRun,
        /// Opens a new query tab.
        NewTab,
        /// Closes the active query tab.
        CloseTab,
        /// Activates the next query tab.
        NextTab,
        /// Activates the previous query tab.
        PreviousTab,
        /// Shows or hides the sidebar.
        ToggleSidebar,
        /// Moves keyboard focus to the sidebar's first table.
        FocusSidebar,
        /// Moves keyboard focus to the editor.
        FocusEditor,
        /// Cycles the theme: Dark, Light, System.
        CycleTheme,
        /// Quits Quarry.
        Quit,
    ]
);

/// The key context of the workspace's root element.
pub const WORKSPACE: &str = "Workspace";

/// The editor's own key context, inside the workspace. Bindings here win
/// over the editor's built-in ⌘↵ (newline) and ⌘. (code actions).
pub const WORKSPACE_EDITOR: &str = "Workspace > Input";

/// Every key binding Quarry adds.
pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE)),
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-shift-r", RunAll, Some(WORKSPACE)),
        KeyBinding::new("cmd-e", Explain, Some(WORKSPACE)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-t", NewTab, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-}", NextTab, None),
        KeyBinding::new("cmd-{", PreviousTab, None),
        KeyBinding::new("cmd-\\", ToggleSidebar, None),
        KeyBinding::new("cmd-1", FocusSidebar, None),
        KeyBinding::new("cmd-2", FocusEditor, None),
        KeyBinding::new("cmd-shift-t", CycleTheme, None),
        KeyBinding::new("cmd-q", Quit, None),
    ]
}

/// The macOS menu bar.
pub fn menus() -> Vec<Menu> {
    vec![
        Menu::new("Quarry").items([MenuItem::action("Quit Quarry", Quit)]),
        Menu::new("File").items([
            MenuItem::action("New Tab", NewTab),
            MenuItem::action("Close Tab", CloseTab),
        ]),
        Menu::new("Query").items([
            MenuItem::action("Run Statement", RunStatement),
            MenuItem::action("Run All", RunAll),
            MenuItem::action("Explain", Explain),
            MenuItem::action("Cancel", CancelRun),
        ]),
        Menu::new("View").items([
            MenuItem::action("Next Tab", NextTab),
            MenuItem::action("Previous Tab", PreviousTab),
            MenuItem::action("Toggle Sidebar", ToggleSidebar),
            MenuItem::action("Focus Sidebar", FocusSidebar),
            MenuItem::action("Focus Editor", FocusEditor),
            MenuItem::action("Cycle Theme", CycleTheme),
        ]),
    ]
}

/// Registers the bindings, the menu bar and the app-wide actions.
pub fn install(cx: &mut App) {
    cx.bind_keys(key_bindings());
    cx.set_menus(menus());
    cx.on_action(|_: &Quit, cx| cx.quit());
}
