//! Open tabs, in tab order; picking one activates it.

use gpui_kit::{App, Context, Window};

use super::PaletteSource;
use crate::{model::palette::PaletteItem, views::workspace::Workspace};

/// The open query tabs.
pub struct OpenTabs;

impl PaletteSource for OpenTabs {
    fn id(&self) -> &'static str {
        "tabs"
    }

    fn title(&self) -> &'static str {
        "Open tabs"
    }

    fn items(&self, workspace: &Workspace, _: &App) -> Vec<PaletteItem> {
        let active = workspace.tabs().active_id();
        workspace
            .tabs()
            .tabs()
            .iter()
            .map(|tab| PaletteItem {
                source: self.id(),
                key: tab.id.to_string(),
                label: tab.title.clone(),
                detail: if tab.id == active { "current" } else { "" }.to_owned(),
            })
            .collect()
    }

    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        if let Ok(id) = key.parse() {
            workspace.activate_tab(id, window, cx);
        }
    }
}
