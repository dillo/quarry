//! The draft store round-trips tabs through memory and through a file.

use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    sqlite::SqliteStore,
};

fn draft(id: i64, title: &str, text: &str) -> Draft {
    Draft {
        id,
        title: title.to_owned(),
        text: text.to_owned(),
    }
}

fn sample() -> Drafts {
    Drafts {
        tabs: vec![
            draft(7, "Query 2", "SELECT 2;"),
            draft(3, "Query 1", "SELECT 1;\n-- é ✓"),
        ],
        active: 1,
    }
}

#[test]
fn an_empty_store_has_no_tabs() {
    let store = SqliteStore::in_memory().unwrap();
    assert_eq!(store.load().unwrap(), Drafts::default());
}

#[test]
fn round_trips_in_memory_keeping_order() {
    let store = SqliteStore::in_memory().unwrap();
    store.save(&sample()).unwrap();
    assert_eq!(store.load().unwrap(), sample());
}

#[test]
fn a_save_replaces_the_previous_one() {
    let store = SqliteStore::in_memory().unwrap();
    store.save(&sample()).unwrap();
    let fewer = Drafts {
        tabs: vec![draft(9, "Query 5", "")],
        active: 0,
    };
    store.save(&fewer).unwrap();
    assert_eq!(store.load().unwrap(), fewer);
    store.save(&Drafts::default()).unwrap();
    assert_eq!(store.load().unwrap(), Drafts::default());
}

#[test]
fn clamps_an_active_index_past_the_last_tab() {
    let store = SqliteStore::in_memory().unwrap();
    let mut drafts = sample();
    drafts.active = 9;
    store.save(&drafts).unwrap();
    assert_eq!(store.load().unwrap().active, 1);
}

#[test]
fn round_trips_through_a_file_in_a_new_directory() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("Quarry").join("quarry.db");
    SqliteStore::open(&path).unwrap().save(&sample()).unwrap();
    assert_eq!(SqliteStore::open(&path).unwrap().load().unwrap(), sample());
}

#[test]
fn leaves_a_newer_file_untouched() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quarry.db");
    let connection = rusqlite::Connection::open(&path).unwrap();
    connection.pragma_update(None, "user_version", 5).unwrap();
    drop(connection);
    let before = std::fs::read(&path).unwrap();
    let error = SqliteStore::open(&path).err();
    assert_eq!(
        error,
        Some(StoreError::TooNew {
            found: 5,
            supported: 1
        })
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn reports_a_path_that_cannot_be_created() {
    let dir = tempfile::tempdir().unwrap();
    let blocker = dir.path().join("file");
    std::fs::write(&blocker, "not a directory").unwrap();
    let error = SqliteStore::open(&blocker.join("quarry.db")).err();
    assert!(matches!(error, Some(StoreError::Open { .. })), "{error:?}");
}

#[test]
fn reports_a_file_that_is_not_a_database() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("quarry.db");
    std::fs::write(&path, vec![b'x'; 4096]).unwrap();
    let error = SqliteStore::open(&path).err();
    assert!(matches!(error, Some(StoreError::Sql(_))), "{error:?}");
}
