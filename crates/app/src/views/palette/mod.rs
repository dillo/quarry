//! The ⌘K palette: open tabs, recently closed tabs, challenges, tables and
//! commands.
//!
//! Each kind of item comes from a [`PaletteSource`] in the
//! [`PaletteSourceRegistry`]; adding a source is a new module and one line
//! in [`PaletteSourceRegistry::standard`]. [`dialog`] draws the palette.

mod challenges;
mod closed;
mod commands;
pub mod dialog;
mod tables;
mod tabs;

use gpui_kit::{App, Context, Window};

use super::workspace::Workspace;
use crate::model::palette::PaletteItem;

/// One kind of thing the palette lists.
pub trait PaletteSource {
    /// A stable name, such as "tabs".
    fn id(&self) -> &'static str;

    /// The heading over its items.
    fn title(&self) -> &'static str;

    /// Its items now, in the order they are listed.
    fn items(&self, workspace: &Workspace, cx: &App) -> Vec<PaletteItem>;

    /// Acts on the item `key` was listed with. A key that no longer names
    /// anything does nothing.
    fn pick(
        &self,
        key: &str,
        workspace: &mut Workspace,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    );
}

/// The palette's sources, in the order their groups appear.
pub struct PaletteSourceRegistry {
    sources: Vec<Box<dyn PaletteSource>>,
}

impl PaletteSourceRegistry {
    /// Open tabs, recently closed tabs, challenges, tables and views, then
    /// commands.
    pub fn standard() -> Self {
        Self {
            sources: vec![
                Box::new(tabs::OpenTabs),
                Box::new(closed::RecentlyClosed),
                Box::new(challenges::Challenges),
                Box::new(tables::Tables),
                Box::new(commands::Commands),
            ],
        }
    }

    /// Every source, in order.
    pub fn sources(&self) -> &[Box<dyn PaletteSource>] {
        &self.sources
    }

    /// The source named `id`.
    pub fn get(&self, id: &str) -> Option<&dyn PaletteSource> {
        self.sources
            .iter()
            .find(|source| source.id() == id)
            .map(AsRef::as_ref)
    }

    /// Every source's items, source by source.
    pub fn items(&self, workspace: &Workspace, cx: &App) -> Vec<PaletteItem> {
        self.sources
            .iter()
            .flat_map(|source| source.items(workspace, cx))
            .collect()
    }
}
