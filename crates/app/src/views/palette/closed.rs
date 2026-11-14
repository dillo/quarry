//! Recently closed tabs, newest first; picking one reopens it.

use gpui_kit::{App, Context, Window};

use super::PaletteSource;
use crate::{
    model::{closed_tabs, palette::PaletteItem},
    views::workspace::Workspace,
};

/// The tabs closed with ⌘W, kept across restarts.
pub struct RecentlyClosed;

impl PaletteSource for RecentlyClosed {
    fn id(&self) -> &'static str {
        "closed"
    }

    fn title(&self) -> &'static str {
        "Recently closed"
    }

    fn items(&self, workspace: &Workspace, _: &App) -> Vec<PaletteItem> {
        workspace
            .closed_tabs()
            .iter()
            .enumerate()
            .map(|(index, draft)| PaletteItem {
                source: self.id(),
                key: index.to_string(),
                label: draft.title.clone(),
                detail: closed_tabs::first_line(&draft.text).to_owned(),
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
        if let Ok(index) = key.parse() {
            workspace.reopen_tab(index, window, cx);
        }
    }
}
