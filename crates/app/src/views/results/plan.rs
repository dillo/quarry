//! The Plan view: the steps the database would take, indented by nesting.

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
    model::plan_tree::{PlanRow, PlanState},
    views::query_tab::QueryTab,
};

/// How far each level of nesting is indented.
pub const INDENT: f32 = 18.0;

/// The plan of the last statement explained with ⌘E.
pub struct PlanView;

impl ResultView for PlanView {
    fn id(&self) -> &'static str {
        "plan"
    }

    fn title(&self) -> &'static str {
        "Plan"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let note = |message: String, role: Role| {
            vec![
                div()
                    .text_color(colors.get(role))
                    .child(message)
                    .into_any_element(),
            ]
        };
        let body = match tab.plan() {
            PlanState::Idle => note(
                "Explain a statement with ⌘E to see its plan.".to_owned(),
                Role::TextMuted,
            ),
            PlanState::Planning => note("Planning…".to_owned(), Role::TextMuted),
            PlanState::Failed(reason) => note(reason.clone(), Role::Danger),
            PlanState::Ready(rows) => rows.iter().map(step).collect(),
        };
        div()
            .id("plan")
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

/// One step, indented by its depth, in Menlo.
fn step(row: &PlanRow) -> AnyElement {
    let depth = u16::try_from(row.depth).unwrap_or(u16::MAX);
    div()
        .pl(px(INDENT * f32::from(depth)))
        .font_family(MONO_FONT)
        .child(row.label.clone())
        .into_any_element()
}
