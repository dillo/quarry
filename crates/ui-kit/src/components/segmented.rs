//! A segmented control: one of a few options (e.g. Playground / Connections).

use std::rc::Rc;

use gpui_kit::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
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

fn segment(
    index: usize,
    label: SharedString,
    selected: bool,
    colors: &Colors,
    handler: Option<ChangeHandler>,
) -> impl IntoElement + use<> {
    let (background, ink) = if selected {
        (colors.get(Role::Selection), colors.get(Role::Text))
    } else {
        (gpui_kit::transparent_black(), colors.get(Role::TextMuted))
    };
    div()
        .id(("segment", index))
        .flex_1()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(radius::SM))
        .text_size(px(text::CAPTION))
        .bg(background)
        .text_color(ink)
        .cursor_pointer()
        .child(label)
        .on_click(move |_, window, cx| {
            if let Some(handler) = &handler {
                handler(index, window, cx);
            }
        })
        .test_support()
}
