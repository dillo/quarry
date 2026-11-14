//! Opening the ⌘K palette over the workspace.

use std::rc::Rc;

use gpui_kit::{Context, Entity, Window};

use super::Workspace;
use crate::{
    shell::actions::OpenPalette,
    views::palette::dialog::{self, Palette},
};

impl Workspace {
    pub(super) fn open_palette(
        &mut self,
        _: &OpenPalette,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let registry = Rc::clone(&self.palette);
        let palette = dialog::open(self, registry, window, cx);
        self.shown_palette = palette.downgrade();
    }

    /// The palette, while it is open.
    pub fn palette(&self) -> Option<Entity<Palette>> {
        self.shown_palette.upgrade()
    }
}
