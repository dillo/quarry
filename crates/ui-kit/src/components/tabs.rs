//! Editor tabs.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use super::ClickHandler;
use crate::{
    Aurora,
    palette::Role,
    tokens::{radius, space, text},
};

/// One tab; the selected tab is lifted onto a faint glass background.
#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    label: SharedString,
    selected: bool,
    on_click: Option<ClickHandler>,
}

impl Tab {
    /// A tab with an id and label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected: false,
            on_click: None,
        }
    }

    /// Marks the tab as the selected one.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Runs `handler` when the tab is clicked.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let (background, ink) = if self.selected {
            (colors.get(Role::Hover), colors.get(Role::Text))
        } else {
            (
                gpui_kit::transparent_black(),
                colors.get(Role::TextSecondary),
            )
        };
        let mut tab = div()
            .id(self.id)
            .px(px(space::MD + 2.))
            .py(px(space::SM - 1.))
            .rounded_t(px(radius::SM + 1.))
            .text_size(px(text::SMALL))
            .bg(background)
            .text_color(ink)
            .cursor_pointer()
            .child(self.label);
        if let Some(handler) = self.on_click {
            tab = tab.on_click(move |event, window, cx| handler(event, window, cx));
        }
        tab.test_support()
    }
}

/// A row of tabs aligned to the bottom of the title-bar strip.
pub fn tab_bar() -> Div {
    div().flex().items_end().gap(px(2.)).px(px(space::MD))
}
