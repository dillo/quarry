//! A sidebar row: label, optional trailing text, selectable.
//!
//! A row with a click handler is a keyboard stop (see [`keyboard_focusable`]).
//! Give it a focus handle with [`SidebarItem::track_focus`] to focus it from
//! code.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, Div, ElementId, FocusHandle, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, Stateful, StatefulInteractiveElement, Styled,
    TestSupportExt, Window, div, px,
};

use super::{ClickHandler, keyboard_focusable};
use crate::{
    Aurora,
    palette::{Colors, Role},
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
    focus: Option<FocusHandle>,
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
            focus: None,
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

impl SidebarItem {
    /// Tracks `handle`, so focusing it focuses the row. A row that is a
    /// keyboard stop needs a handle that is a tab stop.
    #[must_use]
    pub fn track_focus(mut self, handle: FocusHandle) -> Self {
        self.focus = Some(handle);
        self
    }
}

/// Resolved colors for a row.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct RowStyle {
    /// Fill, if any.
    pub background: Option<Hsla>,
    /// Border color, if any.
    pub border: Option<Hsla>,
    /// Fill while hovered; `None` for the selected row.
    pub hover: Option<Hsla>,
}

/// The colors a row uses when `selected` or not.
pub fn row_style(selected: bool, colors: &Colors) -> RowStyle {
    if selected {
        RowStyle {
            background: Some(colors.get(Role::Selection)),
            border: Some(colors.get(Role::SelectionBorder)),
            hover: None,
        }
    } else {
        RowStyle {
            background: None,
            border: None,
            hover: Some(colors.get(Role::Hover)),
        }
    }
}

impl RenderOnce for SidebarItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let style = row_style(self.selected, &colors);
        let mut row = div()
            .id(self.id)
            .flex()
            .justify_between()
            .items_center()
            .px(px(space::SM))
            .py(px(5.))
            .rounded(px(radius::SM))
            .border_1()
            .border_color(gpui_kit::transparent_black())
            .text_size(px(text::SMALL))
            .text_color(colors.get(Role::Text))
            .cursor_pointer()
            .child(self.label)
            .children(
                self.trailing
                    .map(|t| div().text_color(colors.get(Role::TextMuted)).child(t)),
            );
        row = paint(row, style);
        if let Some(focus) = &self.focus {
            row = row.track_focus(focus);
        }
        if let Some(handler) = self.on_click {
            row = keyboard_focusable(row, &colors)
                .on_click(move |event, window, cx| handler(event, window, cx));
        }
        row.test_support()
    }
}

/// Applies a row's resolved colors.
fn paint(mut row: Stateful<Div>, style: RowStyle) -> Stateful<Div> {
    if let Some(background) = style.background {
        row = row.bg(background);
    }
    if let Some(border) = style.border {
        row = row.border_color(border);
    }
    if let Some(hover) = style.hover {
        row = row.hover(move |s| s.bg(hover));
    }
    row
}

#[cfg(test)]
mod tests {
    use super::row_style;
    use crate::palette::{Palette, Role};

    #[test]
    fn selected_row_keeps_its_wash_on_hover() {
        let colors = Palette::aurora().colors();
        let style = row_style(true, &colors);
        assert_eq!(style.background, Some(colors.get(Role::Selection)));
        assert_eq!(style.border, Some(colors.get(Role::SelectionBorder)));
        assert_eq!(style.hover, None);
    }

    #[test]
    fn unselected_row_is_bare_and_hovers() {
        let colors = Palette::aurora().colors();
        let style = row_style(false, &colors);
        assert_eq!(style.background, None);
        assert_eq!(style.border, None);
        assert_eq!(style.hover, Some(colors.get(Role::Hover)));
    }
}
