//! Every command, its key binding and its menu.
//!
//! Adding a command means declaring it in `actions!`, adding one row to
//! [`commands`], and a row to [`key_bindings`] if it has a key. The command
//! table makes the menus and the palette's Commands.

use gpui_kit::{Action, App, KeyBinding, Menu, MenuItem, actions};

actions!(
    quarry,
    [
        /// Runs the selection, or the statement under the cursor.
        RunStatement,
        /// Runs the whole script, statement by statement.
        RunAll,
        /// Shows how the database would run the current statement.
        Explain,
        /// Checks the challenge tab's answer.
        CheckAnswer,
        /// Cancels the running query.
        CancelRun,
        /// Opens the challenge before the active one.
        PreviousChallenge,
        /// Opens the challenge after the active one.
        NextChallenge,
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
        /// Shows or hides the inspector.
        ToggleInspector,
        /// Moves keyboard focus to the sidebar's first table.
        FocusSidebar,
        /// Moves keyboard focus to the editor.
        FocusEditor,
        /// Cycles the theme: Dark, Light, System.
        CycleTheme,
        /// Copies the grid's selection, or the whole result, as TSV.
        CopyTsv,
        /// Copies the grid's selection, or the whole result, as CSV.
        CopyCsv,
        /// Copies the grid's selection, or the whole result, as JSON.
        CopyJson,
        /// Opens the ⌘K palette, or closes it when it is open.
        OpenPalette,
        /// Quits Quarry.
        Quit,
    ]
);

/// The key context of the workspace's root element.
pub const WORKSPACE: &str = "Workspace";

/// The editor's own key context, inside the workspace. Bindings here win
/// over the editor's built-in ⌘↵ (newline) and ⌘. (code actions).
pub const WORKSPACE_EDITOR: &str = "Workspace > Input";

/// The palette's key context, where ⌘K closes it again.
pub const PALETTE: &str = "Palette";

/// The results grid's key context. ⌘C there copies as TSV; the editor
/// keeps its own ⌘C.
pub const GRID: &str = "DataTable";

/// Every key binding Quarry adds.
pub fn key_bindings() -> Vec<KeyBinding> {
    vec![
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE)),
        KeyBinding::new("cmd-enter", RunStatement, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-shift-r", RunAll, Some(WORKSPACE)),
        KeyBinding::new("cmd-shift-enter", CheckAnswer, Some(WORKSPACE)),
        KeyBinding::new("cmd-shift-enter", CheckAnswer, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-e", Explain, Some(WORKSPACE)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE)),
        KeyBinding::new("cmd-.", CancelRun, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-alt-[", PreviousChallenge, None),
        KeyBinding::new("cmd-alt-]", NextChallenge, None),
        KeyBinding::new("cmd-t", NewTab, None),
        KeyBinding::new("cmd-w", CloseTab, None),
        KeyBinding::new("cmd-}", NextTab, None),
        KeyBinding::new("cmd-{", PreviousTab, None),
        KeyBinding::new("cmd-\\", ToggleSidebar, None),
        KeyBinding::new("cmd-alt-i", ToggleInspector, None),
        KeyBinding::new("cmd-1", FocusSidebar, None),
        KeyBinding::new("cmd-2", FocusEditor, None),
        KeyBinding::new("cmd-shift-t", CycleTheme, None),
        KeyBinding::new("cmd-c", CopyTsv, Some(GRID)),
        KeyBinding::new("cmd-k", OpenPalette, Some(WORKSPACE)),
        KeyBinding::new("cmd-k", OpenPalette, Some(WORKSPACE_EDITOR)),
        KeyBinding::new("cmd-k", OpenPalette, Some(PALETTE)),
        KeyBinding::new("cmd-q", Quit, None),
    ]
}

/// The app menu, which holds Quit; the palette leaves it out.
pub const APP_MENU: &str = "Quarry";

/// One command: its menu item and palette label, its action, and its menu.
pub struct Command {
    /// The menu item's text, and the palette's.
    pub label: &'static str,
    /// Makes the action the command dispatches.
    pub action: fn() -> Box<dyn Action>,
    /// The menu the command is in.
    pub menu: &'static str,
}

/// Every command, menu by menu, in menu order.
pub fn commands() -> Vec<Command> {
    let command = |label, action, menu| Command {
        label,
        action,
        menu,
    };
    vec![
        command("Quit Quarry", || Box::new(Quit), APP_MENU),
        command("New Tab", || Box::new(NewTab), "File"),
        command("Close Tab", || Box::new(CloseTab), "File"),
        command("Copy as TSV", || Box::new(CopyTsv), "Edit"),
        command("Copy as CSV", || Box::new(CopyCsv), "Edit"),
        command("Copy as JSON", || Box::new(CopyJson), "Edit"),
        command("Run Statement", || Box::new(RunStatement), "Query"),
        command("Run All", || Box::new(RunAll), "Query"),
        command("Check Answer", || Box::new(CheckAnswer), "Query"),
        command("Explain", || Box::new(Explain), "Query"),
        command("Cancel", || Box::new(CancelRun), "Query"),
        command(
            "Previous Challenge",
            || Box::new(PreviousChallenge),
            "Query",
        ),
        command("Next Challenge", || Box::new(NextChallenge), "Query"),
        command("Open Palette", || Box::new(OpenPalette), "View"),
        command("Next Tab", || Box::new(NextTab), "View"),
        command("Previous Tab", || Box::new(PreviousTab), "View"),
        command("Toggle Sidebar", || Box::new(ToggleSidebar), "View"),
        command("Toggle Inspector", || Box::new(ToggleInspector), "View"),
        command("Focus Sidebar", || Box::new(FocusSidebar), "View"),
        command("Focus Editor", || Box::new(FocusEditor), "View"),
        command("Cycle Theme", || Box::new(CycleTheme), "View"),
    ]
}

/// The macOS menu bar, made from [`commands`].
pub fn menus() -> Vec<Menu> {
    let commands = commands();
    let mut names: Vec<&str> = Vec::new();
    for command in &commands {
        if !names.contains(&command.menu) {
            names.push(command.menu);
        }
    }
    names
        .into_iter()
        .map(|name| {
            let items = commands.iter().filter(|command| command.menu == name);
            Menu::new(name).items(items.map(menu_item))
        })
        .collect()
}

fn menu_item(command: &Command) -> MenuItem {
    MenuItem::Action {
        name: command.label.into(),
        action: (command.action)(),
        os_action: None,
        checked: false,
        disabled: false,
    }
}

/// Registers the bindings, the menu bar and the app-wide actions.
pub fn install(cx: &mut App) {
    cx.bind_keys(key_bindings());
    cx.set_menus(menus());
    cx.on_action(|_: &Quit, cx| cx.quit());
}
