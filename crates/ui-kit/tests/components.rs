//! Components rendered in a headless window: layout, ids and click callbacks.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, px, test::TestWindowExt,
};
use quarry_ui_kit::{
    Aurora, Preference,
    components::{
        badge::{Badge, Tone},
        button::{Button, ButtonVariant},
        surface::{glass_panel, sidebar_surface, window_background},
    },
};

/// Records clicks.
#[derive(Default)]
struct Probe {
    clicks: Cell<usize>,
}

struct Gallery {
    probe: Rc<Probe>,
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let clicks = Rc::clone(&self.probe);
        div()
            .size_full()
            .bg(window_background(&colors))
            .child(sidebar_surface(&colors).child("sidebar"))
            .child(glass_panel(&colors).child(Badge::new("JOIN", Tone::Accent)))
            .child(
                Button::new("run", "Run")
                    .variant(ButtonVariant::Primary)
                    .shortcut("⌘↵")
                    .on_click(move |_, _, _| clicks.clicks.set(clicks.clicks.get() + 1)),
            )
    }
}

fn open(cx: &mut TestAppContext) -> (&mut VisualTestContext, Rc<Probe>) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let probe = Rc::new(Probe::default());
    let view_probe = Rc::clone(&probe);
    let (_, cx) = cx.add_window_view(move |_, _| Gallery { probe: view_probe });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (cx, probe)
}

#[gpui_kit::test]
fn the_button_lays_out_with_a_visible_size(cx: &mut TestAppContext) {
    let (cx, _) = open(cx);
    cx.update(|window, _| {
        let bounds = window.find("run").bounds();
        assert!(
            bounds.size.width > px(0.) && bounds.size.height > px(0.),
            "{bounds:?}"
        );
    });
}

#[gpui_kit::test]
fn clicking_a_button_runs_its_handler(cx: &mut TestAppContext) {
    let (cx, probe) = open(cx);
    cx.update(|window, cx| window.click("run", cx));
    assert_eq!(probe.clicks.get(), 1);
}
