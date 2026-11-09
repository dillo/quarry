//! A segmented control: one of a few options (e.g. Playground / Connections).
//!
//! Keyboard focus and navigation are added with the workspace in Phase 1C.

use std::rc::Rc;

use gpui_kit::{
    App, ElementId, Hsla, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::{
    Aurora,
    palette::{Colors, Role},
    tokens::{radius, text},
};

/// Called with the index of the newly chosen option.
pub type ChangeHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A pill track with one highlighted option.
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    options: Vec<SharedString>,
    selected: usize,
    on_change: Option<ChangeHandler>,
}

impl SegmentedControl {
    /// A control with `options`; the first is selected.
    pub fn new(
        id: impl Into<ElementId>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let options = options.into_iter().map(Into::into).collect();
        Self {
            id: id.into(),
            options,
            selected: 0,
            on_change: None,
        }
    }

    /// Selects option `index`.
    #[must_use]
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Runs `handler` with the clicked option's index.
    #[must_use]
    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SegmentedControl {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let track = div()
            .id(self.id)
            .flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(radius::SM + 2.))
            .bg(colors.get(Role::Hover))
            .border_1()
            .border_color(colors.get(Role::Border));
        let (selected, handler) = (self.selected, self.on_change);
        let segments = self.options.into_iter().enumerate().map(|(index, label)| {
            segment(index, label, index == selected, &colors, handler.clone())
        });
        track.children(segments).test_support()
    }
}

/// Resolved colors for a segment.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SegmentStyle {
    /// Fill.
    pub background: Hsla,
    /// Label color.
    pub text: Hsla,
    /// Fill while hovered; `None` for the selected segment.
    pub hover: Option<Hsla>,
}

/// The colors a segment uses when `selected` or not.
pub fn segment_style(selected: bool, colors: &Colors) -> SegmentStyle {
    if selected {
        SegmentStyle {
            background: colors.get(Role::Selection),
            text: colors.get(Role::Text),
            hover: None,
        }
    } else {
        SegmentStyle {
            background: gpui_kit::transparent_black(),
            text: colors.get(Role::TextMuted),
            hover: Some(colors.get(Role::Hover)),
        }
    }
}

fn segment(
    index: usize,
    label: SharedString,
    selected: bool,
    colors: &Colors,
    handler: Option<ChangeHandler>,
) -> impl IntoElement + use<> {
    let style = segment_style(selected, colors);
    let mut segment = div()
        .id(("segment", index))
        .flex_1()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(radius::SM))
        .text_size(px(text::CAPTION))
        .bg(style.background)
        .text_color(style.text)
        .cursor_pointer()
        .child(label);
    if let Some(hover) = style.hover {
        segment = segment.hover(move |s| s.bg(hover));
    }
    segment
        .on_click(move |_, window, cx| {
            if let Some(handler) = &handler {
                handler(index, window, cx);
            }
        })
        .test_support()
}

#[cfg(test)]
mod tests {
    use super::segment_style;
    use crate::palette::{Palette, Role};

    #[test]
    fn selected_segment_is_washed_and_does_not_hover() {
        let colors = Palette::aurora().colors();
        let style = segment_style(true, &colors);
        assert_eq!(style.background, colors.get(Role::Selection));
        assert_eq!(style.text, colors.get(Role::Text));
        assert_eq!(style.hover, None);
    }

    #[test]
    fn unselected_segment_is_flat_and_hovers() {
        let colors = Palette::aurora().colors();
        let style = segment_style(false, &colors);
        assert_eq!(style.background, gpui_kit::transparent_black());
        assert_eq!(style.text, colors.get(Role::TextMuted));
        assert_eq!(style.hover, Some(colors.get(Role::Hover)));
    }
}
