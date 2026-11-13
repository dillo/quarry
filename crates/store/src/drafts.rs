//! The query tabs' drafts, saved so they survive a restart.

use crate::error::StoreError;

/// One query tab's saved text.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Draft {
    /// The tab's identity, stable for the tab's lifetime.
    pub id: i64,
    /// The tab's title, such as "Query 3".
    pub title: String,
    /// The editor text.
    pub text: String,
}

/// Every open tab in order, and which one is active.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Drafts {
    /// The tabs, left to right.
    pub tabs: Vec<Draft>,
    /// The index of the active tab in `tabs`.
    pub active: usize,
}

/// Loads and saves the drafts.
pub trait DraftStore: Send + Sync {
    /// Reads the saved drafts; an empty store has no tabs.
    ///
    /// # Errors
    /// Returns a [`StoreError`] when the store cannot be read.
    fn load(&self) -> Result<Drafts, StoreError>;

    /// Replaces the saved drafts with `drafts`.
    ///
    /// # Errors
    /// Returns a [`StoreError`] when the store cannot be written.
    fn save(&self, drafts: &Drafts) -> Result<(), StoreError>;
}
