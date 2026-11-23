//! The inspector, the workspace's right column: the challenge brief in a
//! challenge tab, and an empty state elsewhere.
//!
//! Each is an [`InspectorView`] in the [`InspectorRegistry`]; adding one is
//! a new module and one line in [`InspectorRegistry::standard`].

mod brief;
mod empty;

use gpui_kit::{AnyElement, Context};

use super::workspace::Workspace;

/// The id of the brief.
pub const BRIEF: &str = "brief";
/// The id of the empty state.
pub const EMPTY: &str = "inspector-empty";

/// One thing the inspector can show.
pub trait InspectorView {
    /// A stable name, such as "brief"; its root element has this id.
    fn id(&self) -> &'static str;

    /// Whether it has something to show for the workspace's active tab.
    fn applies(&self, workspace: &Workspace) -> bool;

    /// Draws it for the workspace's active tab.
    fn render(&self, workspace: &Workspace, cx: &Context<Workspace>) -> AnyElement;
}

/// The inspector's views, most specific first.
pub struct InspectorRegistry {
    views: Vec<Box<dyn InspectorView>>,
}

impl InspectorRegistry {
    /// The brief, then the empty state.
    pub fn standard() -> Self {
        Self {
            views: vec![Box::new(brief::BriefView), Box::new(empty::EmptyView)],
        }
    }

    /// The first view that applies to the workspace's active tab.
    pub fn pick(&self, workspace: &Workspace) -> Option<&dyn InspectorView> {
        self.views
            .iter()
            .find(|view| view.applies(workspace))
            .map(AsRef::as_ref)
    }
}
