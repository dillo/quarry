//! The Check view: the feedback card for the latest check. A wrong answer
//! lists where every rule stands; a solve offers the next challenge.

use gpui_kit::{
    AnyElement, ClickEvent, Context, FontWeight, InteractiveElement, IntoElement, ParentElement,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    components::button::{Button, ButtonVariant},
    palette::Colors,
    tokens::{space, text},
};

use super::{CHECK, ResultView};
use crate::{
    model::verdict::{self, Card, CheckState, Footer, Line, Mark, Tone},
    views::query_tab::QueryTab,
};

/// The id of the card's Next challenge button.
pub const NEXT_CHALLENGE: &str = "card-next";
/// The id of the card's Stay here button.
pub const STAY_HERE: &str = "card-stay";

/// The feedback card for the latest Check answer.
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
        let root = div()
            .id(CHECK)
            .size_full()
            .overflow_y_scroll()
            .p(px(space::MD))
            .flex()
            .flex_col()
            .gap(px(space::MD))
            .text_size(px(text::BODY));
        let body = match tab.check_state() {
            CheckState::Idle => note("Check your answer with ⌘⇧↵.", &colors),
            CheckState::Checking => note("Checking…", &colors),
            CheckState::Done(outcome) => {
                let card = verdict::card(outcome, tab.solve());
                card_element(&card, tab.stayed(), cx)
            }
        };
        root.child(body).test_support().into_any_element()
    }
}

/// A quiet line, before there is a card.
fn note(message: &'static str, colors: &Colors) -> AnyElement {
    div()
        .text_color(colors.get(Role::TextMuted))
        .child(message)
        .into_any_element()
}

/// The card: its header, its checklist and, after a solve, what's next.
fn card_element(card: &Card, stayed: bool, cx: &Context<QueryTab>) -> AnyElement {
    let colors = Aurora::colors(cx);
    div()
        .flex()
        .flex_col()
        .gap(px(space::MD))
        .child(header(card, &colors))
        .children((!card.lines.is_empty()).then(|| checklist(&card.lines, &colors)))
        .children(
            card.footer
                .as_ref()
                .map(|footer| next_up(footer, stayed, cx)),
        )
        .into_any_element()
}

/// The mark, the headline and the subline.
fn header(card: &Card, colors: &Colors) -> AnyElement {
    let (glyph, role) = match card.tone {
        Tone::Solved => ("✓", Role::Success),
        Tone::Wrong | Tone::Failed => ("!", Role::Danger),
    };
    div()
        .flex()
        .items_start()
        .gap(px(space::SM))
        .child(
            div()
                .text_size(px(text::TITLE))
                .text_color(colors.get(role))
                .child(glyph),
        )
        .child(
            div()
                .flex()
                .flex_col()
                .child(
                    div()
                        .text_size(px(text::LARGE_LABEL))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.get(Role::Text))
                        .child(card.headline.clone()),
                )
                .children(card.subline.clone().map(|subline| {
                    div()
                        .text_color(colors.get(Role::TextSecondary))
                        .child(subline)
                })),
        )
        .into_any_element()
}

/// One line per rule, marked ✓, ✗ or –.
fn checklist(lines: &[Line], colors: &Colors) -> AnyElement {
    div()
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .children(lines.iter().map(|line| {
            let role = match line.mark {
                Mark::Pass => Role::Success,
                Mark::Fail => Role::Danger,
                Mark::Skip => Role::TextMuted,
            };
            let text = if line.mark == Mark::Skip {
                Role::TextMuted
            } else {
                Role::Text
            };
            div()
                .flex()
                .gap(px(space::SM))
                .child(div().text_color(colors.get(role)).child(line.mark.glyph()))
                .child(div().text_color(colors.get(text)).child(line.text.clone()))
        }))
        .into_any_element()
}

/// "Next up: …" with Next challenge and Stay here, or "Last challenge".
fn next_up(footer: &Footer, stayed: bool, cx: &Context<QueryTab>) -> AnyElement {
    let colors = Aurora::colors(cx);
    let Footer::Next(title) = footer else {
        return div()
            .text_color(colors.get(Role::TextSecondary))
            .child("Last challenge")
            .into_any_element();
    };
    let buttons = (!stayed).then(|| {
        div()
            .flex()
            .gap(px(space::SM))
            .child(
                Button::new(NEXT_CHALLENGE, "Next challenge →")
                    .variant(ButtonVariant::Primary)
                    .on_click(cx.listener(|tab, _: &ClickEvent, _, cx| tab.next_challenge(cx))),
            )
            .child(
                Button::new(STAY_HERE, "Stay here")
                    .variant(ButtonVariant::Ghost)
                    .on_click(cx.listener(|tab, _: &ClickEvent, _, cx| tab.stay(cx))),
            )
    });
    div()
        .flex()
        .flex_col()
        .gap(px(space::SM))
        .child(
            div()
                .text_color(colors.get(Role::TextSecondary))
                .child(format!("Next up: {title}")),
        )
        .children(buttons)
        .into_any_element()
}
