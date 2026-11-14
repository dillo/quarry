//! Commands, in menu order with their shortcuts; picking one dispatches its
//! action, as its shortcut or menu item does.

use gpui_kit::{Action, App, Context, Window, component::kbd::Kbd};

use super::PaletteSource;
use crate::{
    model::palette::PaletteItem,
    shell::actions::{self, APP_MENU},
    views::workspace::Workspace,
};

/// Every command but those in the app menu.
pub struct Commands;

/// The shortcut that runs `action`, as in "⌘⇧R", or nothing.
fn shortcut(action: &dyn Action, cx: &App) -> String {
    let keymap = cx.key_bindings();
    let keymap = keymap.borrow();
    keymap
        .bindings_for_action(action)
        .last()
        .map(|binding| {
            let keys: Vec<String> = binding
                .keystrokes()
                .iter()
                .map(|key| Kbd::format(key.inner()))
                .collect();
            keys.join(" ")
        })
        .unwrap_or_default()
}

impl PaletteSource for Commands {
    fn id(&self) -> &'static str {
        "commands"
    }

    fn title(&self) -> &'static str {
        "Commands"
    }

    fn items(&self, _: &Workspace, cx: &App) -> Vec<PaletteItem> {
        actions::commands()
            .iter()
            .enumerate()
            .filter(|(_, command)| command.menu != APP_MENU)
            .map(|(index, command)| PaletteItem {
                source: self.id(),
                key: index.to_string(),
                label: command.label.to_owned(),
                detail: shortcut(&*(command.action)(), cx),
            })
            .collect()
    }

    fn pick(&self, key: &str, _: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let commands = actions::commands();
        let command = key
            .parse()
            .ok()
            .and_then(|index: usize| commands.get(index));
        if let Some(command) = command {
            window.dispatch_action((command.action)(), cx);
        }
    }
}
