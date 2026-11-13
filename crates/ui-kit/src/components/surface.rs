//! Window background and glass surfaces.

use gpui_kit::{Background, Div, Styled, div, linear_color_stop, linear_gradient, px};

use crate::{
    palette::{Colors, Role},
    tokens::radius,
};

/// The diagonal window gradient (opaque, so it works without system blur).
pub fn window_background(colors: &Colors) -> Background {
    linear_gradient(
        135.,
        linear_color_stop(colors.get(Role::BackgroundStart), 0.),
        linear_color_stop(colors.get(Role::BackgroundEnd), 1.),
    )
}

/// A floating glass panel: faint fill, 1px light border, medium radius.
pub fn glass_panel(colors: &Colors) -> Div {
    div()
        .rounded(px(radius::MD))
        .bg(colors.get(Role::Panel))
        .border_1()
        .border_color(colors.get(Role::Border))
}

/// The sidebar surface, a slightly stronger glass with a right border.
pub fn sidebar_surface(colors: &Colors) -> Div {
    div()
        .bg(colors.get(Role::Sidebar))
        .border_r_1()
        .border_color(colors.get(Role::Border))
}
