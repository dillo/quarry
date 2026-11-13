//! The views under the editor: Results, Messages and Plan.
//!
//! Each is a [`ResultView`] in the [`ResultViewRegistry`]; adding a view is
//! a new module and one line in [`ResultViewRegistry::standard`].

mod grid;
mod messages;
mod plan;
mod table;

use gpui_kit::{AnyElement, Context, Window};

pub use self::{
    grid::{COLUMN_WIDTH, ResultsDelegate},
    table::statement_tab_id,
};
use super::query_tab::QueryTab;

/// One view of a tab's run.
pub trait ResultView {
    /// A stable name, such as "messages".
    fn id(&self) -> &'static str;

    /// The label on the view switch.
    fn title(&self) -> &'static str;

    /// Draws the view for `tab`.
    fn render(&self, tab: &QueryTab, window: &mut Window, cx: &mut Context<QueryTab>)
    -> AnyElement;
}

/// The views a tab can switch between, in switch order.
pub struct ResultViewRegistry {
    views: Vec<Box<dyn ResultView>>,
}

impl ResultViewRegistry {
    /// Results, Messages, Plan.
    pub fn standard() -> Self {
        Self {
            views: vec![
                Box::new(table::ResultsView),
                Box::new(messages::MessagesView),
                Box::new(plan::PlanView),
            ],
        }
    }

    /// Every view, in switch order.
    pub fn views(&self) -> &[Box<dyn ResultView>] {
        &self.views
    }

    /// The position of the view named `id`.
    pub fn position(&self, id: &str) -> Option<usize> {
        self.views.iter().position(|view| view.id() == id)
    }
}
