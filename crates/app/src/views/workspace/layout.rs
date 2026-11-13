//! Draws the workspace: sidebar, the query tab, status bar.

use std::time::Instant;

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Render, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, component::tooltip::Tooltip, div,
    prelude::FluentBuilder, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::{
        status_bar::StatusBar,
        surface::{sidebar_surface, window_background},
    },
    tokens::{chrome, space, text},
};

use super::{SIDEBAR_WIDTH, Workspace};
use crate::{
    model::{format, status},
    shell::actions::WORKSPACE,
};

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
            .on_action(cx.listener(Self::new_tab))
            .on_action(cx.listener(Self::close_tab))
            .on_action(cx.listener(Self::next_tab))
            .on_action(cx.listener(Self::previous_tab))
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
        div()
            .flex_1()
            .h_full()
            .flex()
            .flex_col()
            .child(self.tab_strip(cx))
            .child(
                div()
                    .flex_1()
                    .px(px(space::MD))
                    .pb(px(space::SM))
                    .child(self.active_tab().clone()),
            )
            .child(self.status_bar(cx))
            .into_any_element()
    }

    fn status_bar(&self, cx: &Context<Self>) -> AnyElement {
        let tab = self.active_tab().read(cx);
        let output = tab.output(cx);
        let summary = status::run_summary(output, output.elapsed(Instant::now()));
        let cursor = tab.editor().read(cx).cursor_position();
        let place = format::position(cursor.line, cursor.character);
        let warning = self.drafts_problem().map(drafts_warning);
        div()
            .id("status")
            .child(
                StatusBar::new()
                    .left(format!("● {}", self.database))
                    .left(div().children(warning))
                    .right(format!("{summary} · {place}")),
            )
            .into_any_element()
    }
}

/// The quiet warning that drafts aren't being saved, with the reason on hover.
fn drafts_warning(problem: &str) -> AnyElement {
    let reason = SharedString::from(problem.to_owned());
    div()
        .id("drafts-warning")
        .child("Drafts aren't being saved")
        .tooltip(move |window, cx| Tooltip::new(reason.clone()).build(window, cx))
        .test_support()
        .into_any_element()
}
