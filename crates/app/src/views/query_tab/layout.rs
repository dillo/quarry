//! Draws a query tab: the editor over its results, with a draggable divider.

use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestSupportExt, Window,
    component::{
        input::Editor,
        resizable::{resizable_panel, v_resizable},
    },
    div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::{segmented::SegmentedControl, surface::glass_panel},
    tokens::{space, text},
};

use super::QueryTab;
use crate::model::status::{self, Failure};

/// The editor panel's starting height; the divider below it drags.
pub const EDITOR_HEIGHT: f32 = 220.0;

impl Render for QueryTab {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let editor = glass_panel(&colors)
            .size_full()
            .p(px(space::SM))
            .child(Editor::new(&self.editor).appearance(false).h_full());
        v_resizable("editor-results")
            .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
            .child(resizable_panel().child(self.results_panel(window, cx)))
    }
}

impl QueryTab {
    fn results_panel(&self, window: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let failure = status::failure(self.output(cx));
        let view = self.views.views()[self.view].render(self, window, cx);
        glass_panel(&colors)
            .id("results")
            .size_full()
            .mt(px(space::SM))
            .flex()
            .flex_col()
            .overflow_hidden()
            .child(self.view_switch(cx))
            .children(failure.map(|failure| failure_banner(failure, cx)))
            .child(div().flex_1().child(view))
            .into_any_element()
    }

    /// Results | Messages | Plan.
    fn view_switch(&self, cx: &mut Context<Self>) -> AnyElement {
        let titles = self.views.views().iter().map(|view| view.title());
        let this = cx.entity().downgrade();
        let switch = SegmentedControl::new("result-views", titles)
            .selected(self.view)
            .on_change(move |index, _, cx| {
                let _ = this.update(cx, |tab, cx| tab.select_view_at(index, cx));
            });
        div()
            .flex()
            .px(px(space::SM))
            .py(px(space::XS))
            .border_b_1()
            .border_color(Aurora::colors(cx).get(Role::Border))
            .child(switch)
            .into_any_element()
    }
}

/// The run's error above the grid; a cancel reads quietly.
fn failure_banner(failure: Failure, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    let role = if failure.cancelled {
        Role::TextSecondary
    } else {
        Role::Danger
    };
    div()
        .id("run-failure")
        .px(px(space::MD))
        .py(px(space::SM))
        .border_b_1()
        .border_color(colors.get(Role::Border))
        .text_size(px(text::BODY))
        .text_color(colors.get(role))
        .child(failure.message)
        .test_support()
        .into_any_element()
}
