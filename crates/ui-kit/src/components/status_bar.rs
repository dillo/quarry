//! The bottom status bar: left and right groups of small muted items.

use gpui_kit::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::{
    Aurora,
    palette::Role,
    tokens::{space, text},
};

/// A status bar with a left group and a right group.
#[derive(IntoElement, Default)]
pub struct StatusBar {
    left: Vec<AnyElement>,
    right: Vec<AnyElement>,
}

impl StatusBar {
    /// An empty status bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item to the left group.
    #[must_use]
    pub fn left(mut self, item: impl IntoElement) -> Self {
        self.left.push(item.into_any_element());
        self
    }

    /// Adds an item to the right group.
    #[must_use]
    pub fn right(mut self, item: impl IntoElement) -> Self {
        self.right.push(item.into_any_element());
        self
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let group = || div().flex().items_center().gap(px(space::MD));
        div()
            .flex()
            .justify_between()
            .items_center()
            .px(px(space::LG))
            .py(px(space::SM))
            .text_size(px(text::CAPTION))
            .text_color(colors.get(Role::TextMuted))
            .child(group().children(self.left))
            .child(group().children(self.right))
    }
}
