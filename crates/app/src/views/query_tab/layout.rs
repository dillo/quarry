//! Draws a query tab: the editor over its results, with a draggable divider.

use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render, Styled,
    TestSupportExt, Window,
    component::{
        input::Editor,
        resizable::{resizable_panel, v_resizable},
        table::DataTable,
    },
    div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::surface::glass_panel,
    tokens::{space, text},
};

use super::QueryTab;
use crate::model::status::{self, Failure};

/// The editor panel's starting height; the divider below it drags.
pub const EDITOR_HEIGHT: f32 = 220.0;

impl Render for QueryTab {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let editor = glass_panel(&colors)
            .size_full()
            .p(px(space::SM))
            .child(Editor::new(&self.editor).appearance(false).h_full());
        v_resizable("editor-results")
            .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
            .child(resizable_panel().child(self.results_panel(cx)))
    }
}

impl QueryTab {
    fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let failure = status::failure(self.output(cx));
        glass_panel(&colors)
            .id("results")
            .size_full()
            .mt(px(space::SM))
            .flex()
            .flex_col()
            .overflow_hidden()
            .children(failure.map(|failure| failure_banner(failure, cx)))
            .child(
                div()
                    .flex_1()
                    .child(DataTable::new(&self.results).bordered(false)),
            )
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
