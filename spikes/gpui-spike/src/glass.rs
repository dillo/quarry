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
pub fn aurora_background() -> Background {
    linear_gradient(
        135.,
        linear_color_stop(rgba(0x1b1530d0), 0.),
        linear_color_stop(rgba(0x0b1b22d0), 1.),
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
