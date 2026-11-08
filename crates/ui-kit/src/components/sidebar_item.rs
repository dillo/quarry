//! A sidebar row: label, optional trailing text, selectable.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use super::ClickHandler;
use crate::{
    Aurora,
    palette::Role,
    tokens::{radius, space, text},
};

/// One row in the sidebar.
#[derive(IntoElement)]
pub struct SidebarItem {
    id: ElementId,
    label: SharedString,
    trailing: Option<SharedString>,
    selected: bool,
    on_click: Option<ClickHandler>,
}

impl SidebarItem {
    /// A row with an id and label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            trailing: None,
            selected: false,
            on_click: None,
        }
    }

    /// Right-aligned secondary text, e.g. a row count.
    #[must_use]
    pub fn trailing(mut self, trailing: impl Into<SharedString>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Highlights the row with the selection wash and border.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Runs `handler` when the row is clicked.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SidebarItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let hover = colors.get(Role::Hover);
        let mut row = div()
            .id(self.id)
            .flex()
            .justify_between()
            .items_center()
            .px(px(space::SM))
            .py(px(5.))
            .rounded(px(radius::SM))
            .text_size(px(text::SMALL))
            .text_color(colors.get(Role::Text))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .child(self.label)
            .children(
                self.trailing
                    .map(|t| div().text_color(colors.get(Role::TextMuted)).child(t)),
            );
        if self.selected {
            row = row
                .bg(colors.get(Role::Selection))
                .border_1()
                .border_color(colors.get(Role::SelectionBorder));
        }
        if let Some(handler) = self.on_click {
            row = row.on_click(move |event, window, cx| handler(event, window, cx));
        }
        row.test_support()
    }
}
