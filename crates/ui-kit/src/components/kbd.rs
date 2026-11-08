//! Keyboard-shortcut hint chips, e.g. "⌘↵".

use gpui_kit::{App, Div, IntoElement, ParentElement, SharedString, Styled, div, px};

use crate::{
    Aurora,
    palette::Role,
    tokens::{MONO_FONT, radius, text},
};

/// A small monospace chip showing a shortcut. Colors are applied at render
/// time from the current theme by [`Kbd`].
pub fn kbd(keys: SharedString) -> Kbd {
    Kbd { keys }
}

/// A keyboard-shortcut chip.
#[derive(IntoElement)]
pub struct Kbd {
    keys: SharedString,
}

impl gpui_kit::RenderOnce for Kbd {
    fn render(self, _: &mut gpui_kit::Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        chip(self.keys)
            .text_color(colors.get(Role::TextSecondary))
            .bg(colors.get(Role::Hover))
            .border_1()
            .border_color(colors.get(Role::Border))
    }
}

fn chip(keys: SharedString) -> Div {
    div()
        .px(px(5.))
        .rounded(px(radius::SM - 2.))
        .font_family(MONO_FONT)
        .text_size(px(text::CAPTION))
        .child(keys)
}
