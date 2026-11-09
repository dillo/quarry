//! Buttons: the accent "primary", a glass "secondary", and a bare "ghost".

use std::rc::Rc;

use gpui_kit::{
    App, Background, BoxShadow, ClickEvent, Div, ElementId, FontWeight, Hsla, InteractiveElement,
    IntoElement, ParentElement, RenderOnce, SharedString, Stateful, StatefulInteractiveElement,
    Styled, TestSupportExt, Window, div, linear_color_stop, linear_gradient, point, px,
};

use super::{ClickHandler, kbd::kbd};
use crate::{
    Aurora,
    color::{hsla, mix},
    palette::{Colors, Role},
    tokens::{radius, space, text},
};

const BLACK: u32 = 0x0000_00ff;

/// Visual weight of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Accent gradient with a soft glow: the one main action (Run).
    Primary,
    /// Glass with a border: secondary actions (Check answer).
    Secondary,
    /// Text only until hovered: toolbar actions.
    Ghost,
}

/// Resolved colors for a variant.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonStyle {
    /// Fill.
    pub background: Option<Background>,
    /// Label color.
    pub text: Hsla,
    /// Border color, if any.
    pub border: Option<Hsla>,
    /// Glow color, if any.
    pub glow: Option<Hsla>,
    /// Fill while the pointer is over the button.
    pub hover: Background,
    /// Label size in points.
    pub size: f32,
    /// Label weight.
    pub weight: FontWeight,
}

impl ButtonVariant {
    /// The colors this variant uses with `colors`.
    pub fn style(self, colors: &Colors) -> ButtonStyle {
        match self {
            Self::Primary => ButtonStyle {
                background: Some(accent_gradient(colors)),
                text: colors.get(Role::OnAccent),
                border: None,
                glow: Some(colors.get(Role::AccentStart).opacity(0.45)),
                hover: accent_hover(colors),
                // 14 pt semibold is WCAG large text, so 3:1 suffices (ADR 0008).
                size: text::LARGE_LABEL,
                weight: FontWeight::SEMIBOLD,
            },
            Self::Secondary => ButtonStyle {
                background: Some(colors.get(Role::Hover).into()),
                text: colors.get(Role::Text),
                border: Some(colors.get(Role::Border)),
                glow: None,
                hover: colors.get(Role::Selection).into(),
                size: text::SMALL,
                weight: FontWeight::NORMAL,
            },
            Self::Ghost => ButtonStyle {
                background: None,
                text: colors.get(Role::TextSecondary),
                border: None,
                glow: None,
                hover: colors.get(Role::Hover).into(),
                size: text::SMALL,
                weight: FontWeight::NORMAL,
            },
        }
    }
}

/// The diagonal violet-to-blue accent gradient.
pub fn accent_gradient(colors: &Colors) -> Background {
    linear_gradient(
        135.,
        linear_color_stop(colors.get(Role::AccentStart), 0.),
        linear_color_stop(colors.get(Role::AccentEnd), 1.),
    )
}

/// How far hover deepens each accent stop toward black (10%).
pub const HOVER_DEEPEN: f32 = 0.10;

/// The accent stops deepened toward black, for hover. Deepening keeps the
/// white label above 3:1 (brightening it would not).
pub fn accent_hover_stops(colors: &Colors) -> [Hsla; 2] {
    [Role::AccentStart, Role::AccentEnd]
        .map(|role| mix(colors.get(role), hsla(BLACK), HOVER_DEEPEN))
}

/// The accent gradient deepened toward black, for hover.
pub fn accent_hover(colors: &Colors) -> Background {
    let [start, end] = accent_hover_stops(colors);
    linear_gradient(
        135.,
        linear_color_stop(start, 0.),
        linear_color_stop(end, 1.),
    )
}

/// A clickable button with an optional keyboard-shortcut hint.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    shortcut: Option<SharedString>,
    on_click: Option<ClickHandler>,
}

impl Button {
    /// A button with an id (for clicks and tests) and a label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Secondary,
            shortcut: None,
            on_click: None,
        }
    }

    /// Sets the visual weight.
    #[must_use]
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Shows a shortcut hint such as "⌘↵" after the label.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Runs `handler` when clicked.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let style = self.variant.style(&colors);
        let hover = style.hover;
        let primary = self.variant == ButtonVariant::Primary;
        let mut button = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(space::SM))
            .px(px(space::MD))
            .py(px(space::XS + 1.))
            .rounded(px(radius::SM))
            .text_size(px(style.size))
            .font_weight(style.weight)
            .text_color(style.text)
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .child(self.label)
            .children(self.shortcut.map(|keys| {
                let chip = kbd(keys);
                if primary { chip.on_accent() } else { chip }
            }));
        button = decorate(button, &style);
        if let Some(handler) = self.on_click {
            // Calls through the shared `Rc` handler.
            button = button.on_click(move |event, window, cx| handler(event, window, cx));
        }
        button.test_support()
    }
}

fn decorate(mut button: Stateful<Div>, style: &ButtonStyle) -> Stateful<Div> {
    if let Some(background) = style.background {
        button = button.bg(background);
    }
    if let Some(border) = style.border {
        button = button.border_1().border_color(border);
    }
    if let Some(glow) = style.glow {
        button = button.shadow(vec![glow_shadow(glow)]);
    }
    button
}

fn glow_shadow(color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(4.)),
        blur_radius: px(14.),
        spread_radius: px(0.),
        inset: false,
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::FontWeight;

    use super::{ButtonVariant, accent_gradient, accent_hover_stops};
    use crate::palette::{Palette, Role};

    #[test]
    fn each_variant_has_its_own_hover() {
        let colors = Palette::aurora().colors();
        let primary = ButtonVariant::Primary.style(&colors);
        assert_ne!(Some(primary.hover), primary.background);
        assert_ne!(primary.hover, colors.get(Role::Hover).into());
        let secondary = ButtonVariant::Secondary.style(&colors);
        assert_eq!(secondary.hover, colors.get(Role::Selection).into());
        assert_ne!(Some(secondary.hover), secondary.background);
        let ghost = ButtonVariant::Ghost.style(&colors);
        assert_eq!(ghost.hover, colors.get(Role::Hover).into());
    }

    #[test]
    fn primary_hover_is_darker_than_the_resting_accent() {
        for palette in [Palette::aurora(), Palette::dawn()] {
            let colors = palette.colors();
            let stops = accent_hover_stops(&colors);
            for (hover, role) in stops.into_iter().zip([Role::AccentStart, Role::AccentEnd]) {
                assert!(hover.l < colors.get(role).l, "{role:?}");
            }
        }
    }

    #[test]
    fn only_primary_is_large_semibold() {
        let colors = Palette::aurora().colors();
        let primary = ButtonVariant::Primary.style(&colors);
        assert_eq!((primary.size, primary.weight), (14., FontWeight::SEMIBOLD));
        for variant in [ButtonVariant::Secondary, ButtonVariant::Ghost] {
            let style = variant.style(&colors);
            assert_eq!((style.size, style.weight), (12., FontWeight::NORMAL));
        }
    }

    #[test]
    fn primary_uses_the_accent_gradient_and_glows() {
        let colors = Palette::aurora().colors();
        let style = ButtonVariant::Primary.style(&colors);
        assert_eq!(style.background, Some(accent_gradient(&colors)));
        assert_eq!(style.text, colors.get(Role::OnAccent));
        assert!(style.glow.is_some() && style.border.is_none());
    }

    #[test]
    fn secondary_is_bordered_glass_and_ghost_is_bare() {
        let colors = Palette::dawn().colors();
        let secondary = ButtonVariant::Secondary.style(&colors);
        assert_eq!(secondary.border, Some(colors.get(Role::Border)));
        assert_eq!(secondary.text, colors.get(Role::Text));
        let ghost = ButtonVariant::Ghost.style(&colors);
        assert!(ghost.background.is_none() && ghost.border.is_none() && ghost.glow.is_none());
        assert_eq!(ghost.text, colors.get(Role::TextSecondary));
    }
}
