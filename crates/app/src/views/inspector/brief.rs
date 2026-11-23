//! The challenge brief: where the challenge sits, its story, what a right
//! answer looks like, and the attempts so far.

use gpui_kit::{
    AnyElement, Context, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::badge::{Badge, Tone},
    palette::Colors,
    tokens::{space, text},
};

use super::{BRIEF, InspectorView};
use crate::{model::brief::Brief, views::workspace::Workspace};

/// The id of the brief's title.
pub const TITLE: &str = "brief-title";

/// The active challenge's brief.
pub struct BriefView;

impl InspectorView for BriefView {
    fn id(&self) -> &'static str {
        BRIEF
    }

    fn applies(&self, workspace: &Workspace) -> bool {
        workspace.active_challenge().is_some()
    }

    fn render(&self, workspace: &Workspace, cx: &Context<Workspace>) -> AnyElement {
        let colors = Aurora::colors(cx);
        let root = div()
            .id(BRIEF)
            .size_full()
            .overflow_y_scroll()
            .flex()
            .flex_col()
            .gap(px(space::MD))
            .text_size(px(text::BODY));
        let Some(brief) = workspace.active_brief() else {
            return root.test_support().into_any_element();
        };
        root.child(story(&brief, &colors))
            .child(expected(&brief, &colors))
            .child(concepts(&brief))
            .child(footer(&brief, &colors))
            .test_support()
            .into_any_element()
    }
}

/// The place, the title and the prompt.
fn story(brief: &Brief, colors: &Colors) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .child(caption(brief.place.clone(), colors))
        .child(
            div()
                .id(TITLE)
                .text_size(px(text::TITLE))
                .text_color(colors.get(Role::Text))
                .child(brief.title.clone())
                .test_support(),
        )
        .child(
            div()
                .text_color(colors.get(Role::TextSecondary))
                .child(brief.prompt.clone()),
        )
        .into_any_element()
}

/// The expected columns and row count, and whether order matters.
fn expected(brief: &Brief, colors: &Colors) -> AnyElement {
    let line = |text: String| {
        div()
            .text_color(colors.get(Role::TextSecondary))
            .child(text)
    };
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .child(caption("EXPECTED".to_owned(), colors))
        .children(brief.expected.iter().cloned().map(line))
        .child(line(brief.order.to_owned()))
        .into_any_element()
}

/// The concept tags.
fn concepts(brief: &Brief) -> AnyElement {
    div()
        .flex()
        .flex_wrap()
        .gap(px(space::XS))
        .children(
            brief
                .concepts
                .iter()
                .map(|concept| Badge::new(concept.clone(), Tone::Neutral)),
        )
        .into_any_element()
}

/// The attempts so far, at the bottom.
fn footer(brief: &Brief, colors: &Colors) -> AnyElement {
    div()
        .mt_auto()
        .text_size(px(text::SMALL))
        .text_color(colors.get(Role::TextMuted))
        .child(brief.attempts.clone())
        .into_any_element()
}

/// A small upper-case section label.
fn caption(label: String, colors: &Colors) -> AnyElement {
    div()
        .text_size(px(text::CAPTION))
        .text_color(colors.get(Role::TextMuted))
        .child(label)
        .into_any_element()
}
