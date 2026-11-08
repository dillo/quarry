//! Keyboard-shortcut hint chips, e.g. "⌘↵".

use gpui_kit::{
    App, Div, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px,
};

use crate::{
    Aurora,
    color::hsla,
    palette::{Colors, Role},
    tokens::{MONO_FONT, radius, text},
};

/// A small monospace chip showing a shortcut. Colors are applied at render
/// time from the current theme by [`Kbd`].
pub fn kbd(keys: SharedString) -> Kbd {
    Kbd {
        keys,
        on_accent: false,
    }
}

/// A keyboard-shortcut chip.
#[derive(IntoElement)]
pub struct Kbd {
    keys: SharedString,
    on_accent: bool,
}

impl Kbd {
    /// Styles the chip for sitting on the accent gradient.
    #[must_use]
    pub fn on_accent(mut self) -> Self {
        self.on_accent = true;
        self
    }
}

impl RenderOnce for Kbd {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let (text, background, border) = chip_colors(self.on_accent, &colors);
        chip(self.keys)
            .text_color(text)
            .bg(background)
            .border_1()
            .border_color(border)
    }
}

/// The (text, background, border) colors of a chip.
fn chip_colors(on_accent: bool, colors: &Colors) -> (Hsla, Hsla, Hsla) {
    if on_accent {
        let white = colors.get(Role::OnAccent);
        // A dark wash deepens the accent, keeping white keys readable.
        (white, hsla(0x0000_002e), white.opacity(0.28))
    } else {
        (
            colors.get(Role::TextSecondary),
            colors.get(Role::Hover),
            colors.get(Role::Border),
        )
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

#[cfg(test)]
mod tests {
    use super::chip_colors;
    use crate::color::hsla;
    use crate::palette::{Palette, Role};

    #[test]
    fn plain_chip_uses_secondary_text_on_a_hover_wash() {
        let colors = Palette::aurora().colors();
        let (text, background, border) = chip_colors(false, &colors);
        assert_eq!(text, colors.get(Role::TextSecondary));
        assert_eq!(background, colors.get(Role::Hover));
        assert_eq!(border, colors.get(Role::Border));
    }

    #[test]
    fn accent_chip_is_white_on_a_dark_wash() {
        let colors = Palette::dawn().colors();
        let white = colors.get(Role::OnAccent);
        let (text, background, border) = chip_colors(true, &colors);
        assert_eq!(text, white);
        assert_eq!(background, hsla(0x0000_002e));
        assert_eq!(border, white.opacity(0.28));
    }
}
