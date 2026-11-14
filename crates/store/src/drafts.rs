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

/// A store that keeps nothing: it loads no tabs and every save succeeds.
///
/// Used when the real store cannot open, and by tests that need no storage.
pub struct Forgetful;

impl DraftStore for Forgetful {
    fn load(&self) -> Result<Drafts, StoreError> {
        Ok(Drafts::default())
    }

    fn save(&self, _: &Drafts) -> Result<(), StoreError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_forgetful_store_saves_without_keeping_anything() -> Result<(), StoreError> {
        let store = Forgetful;
        let drafts = Drafts {
            tabs: vec![Draft {
                id: 1,
                title: "Query 1".to_owned(),
                text: "SELECT 1".to_owned(),
            }],
            active: 0,
        };
        store.save(&drafts)?;
        assert_eq!(store.load()?, Drafts::default());
        Ok(())
    }
}
