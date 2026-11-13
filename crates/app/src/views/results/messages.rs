//! The Messages view: what each statement of the run did.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{MONO_FONT, space, text},
};

use super::ResultView;
use crate::{
    model::messages::{self, MessageLine},
    views::query_tab::QueryTab,
};

/// The width of the statement labels' column.
pub const LABEL_WIDTH: f32 = 240.0;

/// One line per statement: rows, rows affected, or the error.
pub struct MessagesView;

impl ResultView for MessagesView {
    fn id(&self) -> &'static str {
        "messages"
    }

    fn title(&self) -> &'static str {
        "Messages"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let lines = messages::lines(tab.output(cx));
        let body = if lines.is_empty() {
            vec![
                div()
                    .text_color(colors.get(Role::TextMuted))
                    .child("Run a script with ⌘⇧R to see what each statement did.")
                    .into_any_element(),
            ]
        } else {
            lines.into_iter().map(|line| message(line, cx)).collect()
        };
        div()
            .id("messages")
            .size_full()
            .overflow_y_scroll()
            .p(px(space::MD))
            .flex()
            .flex_col()
            .gap(px(space::XS))
            .text_size(px(text::BODY))
            .children(body)
            .test_support()
            .into_any_element()
    }
}

/// "2 · INSERT INTO crew…   3 rows affected · 1.2 ms", the error in red.
fn message(line: MessageLine, cx: &Context<QueryTab>) -> AnyElement {
    let colors = Aurora::colors(cx);
    let role = if line.is_error {
        Role::Danger
    } else {
        Role::Text
    };
    div()
        .flex()
        .gap(px(space::MD))
        .child(
            div()
                .w(px(LABEL_WIDTH))
                .flex_none()
                .font_family(MONO_FONT)
                .text_color(colors.get(Role::TextMuted))
                .child(line.label),
        )
        .child(div().text_color(colors.get(role)).child(line.text))
        .into_any_element()
}
