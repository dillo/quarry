//! Where Quarry keeps its local database, and what happens when it can't.

use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use quarry_store::{drafts::Forgetful, sqlite::SqliteStore};

use crate::views::workspace::SavedTabs;

/// The store's folder under the home folder.
pub const FOLDER: &str = "Library/Application Support/Quarry";

/// The store's file name.
pub const FILE: &str = "quarry.db";

/// `~/Library/Application Support/Quarry/quarry.db`, or `None` without a
/// home folder.
pub fn default_path() -> Option<PathBuf> {
    std::env::var_os("HOME").map(|home| PathBuf::from(home).join(FOLDER).join(FILE))
}

/// Opens the store at `path`. When it can't be opened, drafts live only
/// as long as Quarry runs, and the reason says why.
pub fn saved_tabs(path: Option<&Path>) -> SavedTabs {
    let Some(path) = path else {
        return forgetful("There is no home folder to save drafts in.".to_owned());
    };
    match SqliteStore::open(path) {
        Ok(store) => SavedTabs {
            store: Arc::new(store),
            problem: None,
        },
        Err(error) => forgetful(error.to_string()),
    }
}

fn forgetful(problem: String) -> SavedTabs {
    SavedTabs {
        store: Arc::new(Forgetful),
        problem: Some(problem),
    }
}
