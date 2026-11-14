//! Tables and views, in sidebar order; picking one reveals it in the
//! sidebar.

use gpui_kit::{App, Context, Window};

use super::PaletteSource;
use crate::{
    model::{palette::PaletteItem, schema_tree},
    views::workspace::Workspace,
};

/// The tables and views of the database.
pub struct Tables;

impl PaletteSource for Tables {
    fn id(&self) -> &'static str {
        "tables"
    }

    fn title(&self) -> &'static str {
        "Tables & views"
    }

    fn items(&self, workspace: &Workspace, cx: &App) -> Vec<PaletteItem> {
        workspace
            .schema()
            .read(cx)
            .tables()
            .iter()
            .map(|table| PaletteItem {
                source: self.id(),
                key: table.name.clone(),
                label: table.name.clone(),
                detail: schema_tree::summary(table),
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
        workspace.reveal_table(key, window, cx);
    }
}
