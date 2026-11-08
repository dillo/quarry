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
        segmented::SegmentedControl,
        sidebar_item::SidebarItem,
        status_bar::StatusBar,
        surface::{glass_panel, sidebar_surface, window_background},
        tabs::{Tab, tab_bar},
    },
};

/// Records clicks and the last segment chosen.
#[derive(Default)]
struct Probe {
    clicks: Cell<usize>,
    segment: Cell<Option<usize>>,
}

struct Gallery {
    probe: Rc<Probe>,
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let (clicks, segment) = (Rc::clone(&self.probe), Rc::clone(&self.probe));
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(window_background(&colors))
            .child(
                sidebar_surface(&colors).child(
                    SidebarItem::new("item", "astronauts")
                        .trailing("42")
                        .selected(true),
                ),
            )
            .child(
                tab_bar()
                    .child(Tab::new("tab", "Scratch").selected(true))
                    .child(Tab::new("tab-2", "Query 2")),
            )
            .child(glass_panel(&colors).child(Badge::new("JOIN", Tone::Accent)))
            .child(
                Button::new("run", "Run")
                    .variant(ButtonVariant::Primary)
                    .shortcut("⌘↵")
                    .on_click(move |_, _, _| clicks.clicks.set(clicks.clicks.get() + 1)),
            )
            .child(
                SegmentedControl::new("mode", ["Playground", "Connections"])
                    .on_change(move |index, _, _| segment.segment.set(Some(index))),
            )
            .child(
                StatusBar::new()
                    .left("● Playground")
                    .right("4 rows · 2.1 ms"),
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
fn every_component_lays_out_with_a_visible_size(cx: &mut TestAppContext) {
    let (cx, _) = open(cx);
    cx.update(|window, _| {
        for id in ["item", "tab", "tab-2", "run", "mode"] {
            let bounds = window.find(id).bounds();
            assert!(
                bounds.size.width > px(0.) && bounds.size.height > px(0.),
                "{id}: {bounds:?}"
            );
        }
    });
}

#[gpui_kit::test]
fn clicking_a_button_runs_its_handler(cx: &mut TestAppContext) {
    let (cx, probe) = open(cx);
    cx.update(|window, cx| window.click("run", cx));
    assert_eq!(probe.clicks.get(), 1);
}

#[gpui_kit::test]
fn a_segment_reports_its_index(cx: &mut TestAppContext) {
    let (cx, probe) = open(cx);
    cx.update(|window, cx| window.within("mode").click(("segment", 1_usize), cx));
    assert_eq!(probe.segment.get(), Some(1));
    cx.update(|window, cx| window.within("mode").click(("segment", 0_usize), cx));
    assert_eq!(probe.segment.get(), Some(0));
}

#[gpui_kit::test]
fn the_selected_tab_sits_left_of_the_next(cx: &mut TestAppContext) {
    let (cx, _) = open(cx);
    cx.update(|window, _| {
        let (first, second) = (window.find("tab").bounds(), window.find("tab-2").bounds());
        assert!(first.origin.x < second.origin.x);
        assert_eq!(first.origin.y, second.origin.y);
    });
}
