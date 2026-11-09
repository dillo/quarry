//! Draws the workspace: sidebar, editor over results, status bar.

use std::time::Instant;

use gpui_kit::{
    AnyElement, App, Context, InteractiveElement, IntoElement, ParentElement, Render,
    StatefulInteractiveElement, Styled, TestSupportExt, Window,
    component::{
        input::Editor,
        resizable::{resizable_panel, v_resizable},
        table::DataTable,
    },
    div,
    prelude::FluentBuilder,
    px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::{
        status_bar::StatusBar,
        surface::{glass_panel, sidebar_surface, window_background},
    },
    tokens::{chrome, space, text},
};

use super::{SIDEBAR_WIDTH, Workspace};
use crate::{
    model::{format, status},
    shell::actions::WORKSPACE,
};

/// The editor panel's starting height; the divider below it drags.
pub const EDITOR_HEIGHT: f32 = 220.0;

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if Aurora::is_transitioning(cx) {
            window.request_animation_frame();
        }
        let colors = Aurora::colors(cx);
        div()
            .id("workspace")
            .key_context(WORKSPACE)
            .track_focus(&self.focus)
            .on_action(cx.listener(Self::run_statement))
            .on_action(cx.listener(Self::cancel))
            .on_action(cx.listener(Self::toggle_sidebar))
            .on_action(cx.listener(Self::focus_sidebar))
            .on_action(cx.listener(Self::focus_editor))
            .on_action(cx.listener(Self::cycle_theme))
            .size_full()
            .flex()
            .bg(window_background(&colors))
            .text_color(colors.get(Role::Text))
            .when(self.sidebar_open, |root| root.child(self.sidebar(cx)))
            .child(self.center(cx))
    }
}

impl Workspace {
    fn sidebar(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        sidebar_surface(&colors)
            .id("sidebar")
            .track_focus(&self.sidebar_focus)
            .tab_group()
            .w(px(SIDEBAR_WIDTH))
            .h_full()
            .pt(px(chrome::TITLEBAR_HEIGHT))
            .px(px(space::SM))
            .flex()
            .flex_col()
            .gap(px(space::SM))
            .overflow_y_scroll()
            .child(
                div()
                    .px(px(space::SM))
                    .text_size(px(text::CAPTION))
                    .text_color(colors.get(Role::TextMuted))
                    .child("TABLES"),
            )
            .child(self.schema.clone())
            .into_any_element()
    }

    fn center(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let editor = glass_panel(&colors)
            .size_full()
            .p(px(space::SM))
            .child(Editor::new(&self.editor).appearance(false).h_full());
        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .pt(px(chrome::TITLEBAR_HEIGHT))
            .child(
                div().flex_1().px(px(space::MD)).pb(px(space::SM)).child(
                    v_resizable("editor-results")
                        .child(resizable_panel().size(px(EDITOR_HEIGHT)).child(editor))
                        .child(resizable_panel().child(self.results_panel(cx))),
                ),
            )
            .child(self.status_bar(cx))
            .into_any_element()
    }

    fn results_panel(&self, cx: &Context<Self>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let failure = status::failure(self.run_model(cx));
        glass_panel(&colors)
            .id("results")
            .size_full()
            .mt(px(space::SM))
            .flex()
            .flex_col()
            .overflow_hidden()
            .children(failure.map(|(message, cancelled)| failure_banner(message, cancelled, cx)))
            .child(
                div()
                    .flex_1()
                    .child(DataTable::new(&self.results).bordered(false)),
            )
            .into_any_element()
    }

    fn status_bar(&self, cx: &Context<Self>) -> AnyElement {
        let model = self.run_model(cx);
        let summary = status::run_summary(model, model.elapsed(Instant::now()));
        let cursor = self.editor.read(cx).cursor_position();
        let place = format::position(cursor.line, cursor.character);
        div()
            .id("status")
            .child(
                StatusBar::new()
                    .left(format!("● {}", self.database))
                    .right(format!("{summary} · {place}")),
            )
            .into_any_element()
    }
}

/// The run's error above the grid; a cancel reads quietly.
fn failure_banner(message: String, cancelled: bool, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    let role = if cancelled {
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
        .child(message)
        .test_support()
        .into_any_element()
}
