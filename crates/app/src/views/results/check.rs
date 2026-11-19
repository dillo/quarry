//! The Check view: whether the latest check solved the challenge, and if
//! not, what differs. Phase 2B replaces it with the feedback card.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{space, text},
};

use super::{CHECK, ResultView};
use crate::{
    model::verdict::{self, CheckState, Tone},
    views::query_tab::QueryTab,
};

/// What the latest Check answer said.
pub struct CheckView;

impl ResultView for CheckView {
    fn id(&self) -> &'static str {
        CHECK
    }

    fn title(&self) -> &'static str {
        "Check"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let line = |message: String, role: Role| {
            div()
                .text_color(colors.get(role))
                .child(message)
                .into_any_element()
        };
        let body = match tab.check_state() {
            CheckState::Idle => vec![line(
                "Check your answer with ⌘⇧↵.".to_owned(),
                Role::TextMuted,
            )],
            CheckState::Checking => vec![line("Checking…".to_owned(), Role::TextMuted)],
            CheckState::Done(outcome) => {
                let report = verdict::report(outcome);
                let mut lines = vec![line(report.headline, role(report.tone))];
                lines.extend(
                    report
                        .detail
                        .map(|detail| line(detail, Role::TextSecondary)),
                );
                lines
            }
        };
        div()
            .id(CHECK)
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

/// The headline's color.
fn role(tone: Tone) -> Role {
    match tone {
        Tone::Solved => Role::Success,
        Tone::Wrong => Role::Warning,
        Tone::Failed => Role::Danger,
    }
}
