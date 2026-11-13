//! Keyboard stops: Tab reaches interactive components, Enter and Space click
//! them, and components without a handler are skipped.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, test::TestEventExt,
};
use quarry_ui_kit::{
    Aurora, Preference,
    components::{sidebar_item::SidebarItem, tabs::Tab},
};

#[derive(Default)]
struct Presses {
    row: Cell<usize>,
    tab: Cell<usize>,
}

struct Stops {
    presses: Rc<Presses>,
}

impl Render for Stops {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let (row, tab) = (Rc::clone(&self.presses), Rc::clone(&self.presses));
        div()
            .size_full()
            .child(SidebarItem::new("plain", "No handler"))
            .child(
                SidebarItem::new("row", "Row")
                    .on_click(move |_, _, _| row.row.set(row.row.get() + 1)),
            )
            .child(Tab::new("tab", "Tab").on_click(move |_, _, _| tab.tab.set(tab.tab.get() + 1)))
    }
}

fn open(cx: &mut TestAppContext) -> (&mut VisualTestContext, Rc<Presses>) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let presses = Rc::new(Presses::default());
    let view = Rc::clone(&presses);
    let (_, cx) = cx.add_window_view(move |_, _| Stops { presses: view });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (cx, presses)
}

/// Moves keyboard focus `stops` times, redrawing so the focused element
/// registers its key listeners.
fn tab(cx: &mut VisualTestContext, stops: usize) {
    cx.update(|window, cx| {
        for _ in 0..stops {
            window.focus_next(cx);
        }
        window.draw(cx).clear(cx);
    });
}

/// Presses and releases `key`; GPUI clicks the focused element on release.
fn press(cx: &mut VisualTestContext, key: &str) {
    cx.update(|window, cx| {
        window.key_down(key, false, cx);
        window.key_up(key, cx);
    });
}

#[gpui_kit::test]
fn the_first_stop_skips_a_row_without_a_handler(cx: &mut TestAppContext) {
    let (cx, presses) = open(cx);
    tab(cx, 1);
    press(cx, "enter");
    assert_eq!((presses.row.get(), presses.tab.get()), (1, 0));
}

#[gpui_kit::test]
fn space_clicks_and_the_next_stop_is_the_tab(cx: &mut TestAppContext) {
    let (cx, presses) = open(cx);
    tab(cx, 2);
    press(cx, "space");
    assert_eq!((presses.row.get(), presses.tab.get()), (0, 1));
}
