//! What the inspector shows in a tab that isn't a challenge tab.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement, Styled, TestSupportExt,
    div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{space, text},
};

use super::{EMPTY, InspectorView};
use crate::views::workspace::Workspace;

/// "No challenge in this tab".
pub struct EmptyView;

impl InspectorView for EmptyView {
    fn id(&self) -> &'static str {
        EMPTY
    }

    fn applies(&self, _: &Workspace) -> bool {
        true
    }

    fn render(&self, _: &Workspace, cx: &Context<Workspace>) -> AnyElement {
        let colors = Aurora::colors(cx);
        div()
            .id(EMPTY)
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(px(space::XS))
            .text_size(px(text::BODY))
            .child(
                div()
                    .text_color(colors.get(Role::TextSecondary))
                    .child("No challenge in this tab"),
            )
            .child(
                div()
                    .text_color(colors.get(Role::TextMuted))
                    .child("Press ⌘K to find one"),
            )
            .test_support()
            .into_any_element()
    }
}
