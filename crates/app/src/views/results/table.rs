//! The Results view: a strip of statement tabs over the grid.

use gpui_kit::{
    AnyElement, Context, ElementId, IntoElement, ParentElement, Styled, Window,
    component::table::DataTable, div,
};
use quarry_ui_kit::components::tabs::{Tab, tab_bar};

use super::ResultView;
use crate::views::query_tab::QueryTab;

/// The grid of the statement chosen in the strip above it.
pub struct ResultsView;

/// The element id of statement `index`'s result tab, as in "statement-2".
pub fn statement_tab_id(index: usize) -> ElementId {
    ElementId::Name(format!("statement-{index}").into())
}

impl ResultView for ResultsView {
    fn id(&self) -> &'static str {
        "results"
    }

    fn title(&self) -> &'static str {
        "Results"
    }

    fn render(&self, tab: &QueryTab, _: &mut Window, cx: &mut Context<QueryTab>) -> AnyElement {
        div()
            .size_full()
            .flex()
            .flex_col()
            .children(statement_strip(tab, cx))
            .child(
                div()
                    .flex_1()
                    .child(DataTable::new(tab.results()).bordered(false)),
            )
            .into_any_element()
    }
}

/// One tab per statement that returns rows, when there is more than one.
fn statement_strip(tab: &QueryTab, cx: &mut Context<QueryTab>) -> Option<AnyElement> {
    let shown = tab.results().read(cx).delegate().shown();
    let tabs: Vec<Tab> = tab
        .output(cx)
        .statements()
        .iter()
        .filter(|statement| statement.returns_rows())
        .map(|statement| {
            let index = statement.index();
            Tab::new(statement_tab_id(index), statement.label().to_owned())
                .selected(index == shown)
                .on_click(cx.listener(move |tab, _, _, cx| tab.show_statement(index, cx)))
        })
        .collect();
    (tabs.len() > 1).then(|| tab_bar().children(tabs).into_any_element())
}
