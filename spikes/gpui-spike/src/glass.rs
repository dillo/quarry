//! Experiment (a): real frosted glass behind a translucent Aurora gradient.

use gpui_kit::*;

pub struct GlassSpike;

/// A floating Aurora panel: faint white fill, 1px light inner border.
pub fn panel() -> Div {
    div()
        .rounded(px(10.))
        .bg(rgba(0xffffff0a))
        .border_1()
        .border_color(rgba(0xffffff14))
        .p_4()
}

/// The Aurora background with partial alpha so the window blur shows through.
/// Alpha comes from `SPIKE_ALPHA` (hex byte, e.g. `80`), default `d0`.
pub fn aurora_background() -> Background {
    let alpha = std::env::var("SPIKE_ALPHA")
        .ok()
        .and_then(|a| u32::from_str_radix(&a, 16).ok())
        .unwrap_or(0xd0);
    linear_gradient(
        135.,
        linear_color_stop(rgba(0x1b1530_00 | alpha), 0.),
        linear_color_stop(rgba(0x0b1b22_00 | alpha), 1.),
    )
}

impl Render for GlassSpike {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .flex()
            .bg(aurora_background())
            .text_color(rgb(0xe6e4f0))
            .child(
                div()
                    .w(px(220.))
                    .h_full()
                    .pt(px(44.))
                    .px_3()
                    .bg(rgba(0xffffff09))
                    .child("Sidebar: the desktop should blur through here"),
            )
            .child(
                div()
                    .flex_1()
                    .p(px(44.))
                    .child(panel().child("A floating glass panel over the aurora gradient")),
            )
    }
}
