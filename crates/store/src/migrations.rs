//! Schema migrations, tracked with `PRAGMA user_version`.

use rusqlite::Connection;

use crate::error::StoreError;

/// Each step moves the schema up one version; step `n` makes version `n + 1`.
const STEPS: &[&str] = &["CREATE TABLE drafts (
        id INTEGER PRIMARY KEY,
        position INTEGER NOT NULL,
        title TEXT NOT NULL,
        text TEXT NOT NULL
    );
    CREATE TABLE state (key TEXT PRIMARY KEY, value INTEGER NOT NULL);"];

/// The schema version this build writes.
pub const VERSION: i64 = 1;

/// Brings the schema up to [`VERSION`] in one transaction.
///
/// # Errors
/// Returns [`StoreError::TooNew`] for a file from a newer build, without
/// changing it, or [`StoreError::Sql`] when a step fails.
pub fn migrate(connection: &mut Connection) -> Result<(), StoreError> {
    let found: i64 = connection.pragma_query_value(None, "user_version", |row| row.get(0))?;
    if found > VERSION {
        return Err(StoreError::TooNew {
            found,
            supported: VERSION,
        });
    }
    let transaction = connection.transaction()?;
    for step in STEPS.iter().skip(usize::try_from(found).unwrap_or(0)) {
        transaction.execute_batch(step)?;
    }
    transaction.pragma_update(None, "user_version", VERSION)?;
    transaction.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use rusqlite::Connection;

    use super::{STEPS, VERSION, migrate};
    use crate::error::StoreError;

    fn version(connection: &Connection) -> i64 {
        connection
            .pragma_query_value(None, "user_version", |row| row.get(0))
            .unwrap()
    }

    #[test]
    fn version_matches_the_number_of_steps() {
        assert_eq!(usize::try_from(VERSION).unwrap(), STEPS.len());
    }

    #[test]
    fn migrates_an_empty_database_and_is_idempotent() {
        let mut connection = Connection::open_in_memory().unwrap();
        migrate(&mut connection).unwrap();
        assert_eq!(version(&connection), VERSION);
        migrate(&mut connection).unwrap();
        let tables: i64 = connection
            .query_row(
                "SELECT count(*) FROM sqlite_schema WHERE name IN ('drafts', 'state')",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(tables, 2);
    }

    #[test]
    fn refuses_a_newer_schema() {
        let mut connection = Connection::open_in_memory().unwrap();
        connection.pragma_update(None, "user_version", 7).unwrap();
        assert_eq!(
            migrate(&mut connection),
            Err(StoreError::TooNew {
                found: 7,
                supported: VERSION
            })
        );
        assert_eq!(version(&connection), 7);
    }
}
