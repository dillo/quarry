//! Editor tabs.
//!
//! A tab with a click handler is a keyboard stop (see [`keyboard_focusable`]).

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, Div, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement,
    RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use super::{ClickHandler, keyboard_focusable};
use crate::{
    Aurora,
    palette::{Colors, Role},
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

/// Resolved colors for a tab.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TabStyle {
    /// Fill.
    pub background: Hsla,
    /// Label color.
    pub text: Hsla,
    /// Fill while hovered; `None` for the selected tab.
    pub hover: Option<Hsla>,
}

/// The colors a tab uses when `selected` or not.
pub fn tab_style(selected: bool, colors: &Colors) -> TabStyle {
    if selected {
        TabStyle {
            background: colors.get(Role::Hover),
            text: colors.get(Role::Text),
            hover: None,
        }
    } else {
        TabStyle {
            background: gpui_kit::transparent_black(),
            text: colors.get(Role::TextSecondary),
            hover: Some(colors.get(Role::Hover)),
        }
    }
}

impl RenderOnce for Tab {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let style = tab_style(self.selected, &colors);
        let mut tab = div()
            .id(self.id)
            .px(px(space::MD + 2.))
            .py(px(space::SM - 1.))
            .rounded_t(px(radius::SM + 1.))
            .border_1()
            .border_color(gpui_kit::transparent_black())
            .text_size(px(text::SMALL))
            .bg(style.background)
            .text_color(style.text)
            .cursor_pointer()
            .child(self.label);
        if let Some(hover) = style.hover {
            tab = tab.hover(move |s| s.bg(hover));
        }
        if let Some(handler) = self.on_click {
            tab = keyboard_focusable(tab, &colors)
                .on_click(move |event, window, cx| handler(event, window, cx));
        }
        tab.test_support()
    }
}

/// A row of tabs aligned to the bottom of the title-bar strip.
pub fn tab_bar() -> Div {
    div().flex().items_end().gap(px(2.)).px(px(space::MD))
}

#[cfg(test)]
mod tests {
    use super::tab_style;
    use crate::palette::{Palette, Role};

    #[test]
    fn selected_tab_is_lifted_and_does_not_hover() {
        let colors = Palette::aurora().colors();
        let style = tab_style(true, &colors);
        assert_eq!(style.background, colors.get(Role::Hover));
        assert_eq!(style.text, colors.get(Role::Text));
        assert_eq!(style.hover, None);
    }

    #[test]
    fn unselected_tab_is_flat_and_hovers() {
        let colors = Palette::aurora().colors();
        let style = tab_style(false, &colors);
        assert_eq!(style.background, gpui_kit::transparent_black());
        assert_eq!(style.text, colors.get(Role::TextSecondary));
        assert_eq!(style.hover, Some(colors.get(Role::Hover)));
    }
}
