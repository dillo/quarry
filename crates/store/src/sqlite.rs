//! The `SQLite` file behind every store trait.

use std::{
    path::Path,
    sync::{Mutex, MutexGuard, PoisonError},
};

use rusqlite::{Connection, OptionalExtension, params};

use crate::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
    migrations,
};

/// Quarry's local database: one file, migrated on open.
pub struct SqliteStore {
    connection: Mutex<Connection>,
}

impl SqliteStore {
    /// Opens (or creates) the store at `path`, creating its directory.
    ///
    /// # Errors
    /// Returns [`StoreError::Open`] when the file cannot be opened, or a
    /// migration error when the file is from a newer Quarry or is damaged.
    pub fn open(path: &Path) -> Result<Self, StoreError> {
        let open_error = |message: String| StoreError::Open {
            path: path.display().to_string(),
            message,
        };
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| open_error(error.to_string()))?;
        }
        let connection = Connection::open(path).map_err(|error| open_error(error.to_string()))?;
        Self::migrated(connection)
    }

    /// Opens a fresh store that lives only in memory.
    ///
    /// # Errors
    /// Returns [`StoreError::Sql`] if `SQLite` cannot allocate it.
    pub fn in_memory() -> Result<Self, StoreError> {
        Self::migrated(Connection::open_in_memory()?)
    }

    fn migrated(mut connection: Connection) -> Result<Self, StoreError> {
        migrations::migrate(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn connection(&self) -> MutexGuard<'_, Connection> {
        self.connection
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }
}

/// A position as `SQLite` stores it; no real tab list comes near `i64::MAX`.
fn index(position: usize) -> i64 {
    i64::try_from(position).unwrap_or(i64::MAX)
}

/// The open tabs, left to right.
fn load_tabs(connection: &Connection) -> Result<Vec<Draft>, StoreError> {
    let mut statement =
        connection.prepare("SELECT id, title, text FROM drafts ORDER BY position")?;
    let tabs = statement.query_map([], |row| {
        Ok(Draft {
            id: row.get(0)?,
            title: row.get(1)?,
            text: row.get(2)?,
        })
    })?;
    Ok(tabs.collect::<Result<Vec<_>, _>>()?)
}

/// The recently closed tabs, newest first, each with id 0.
fn load_closed(connection: &Connection) -> Result<Vec<Draft>, StoreError> {
    let mut statement =
        connection.prepare("SELECT title, text FROM closed_tabs ORDER BY position")?;
    let closed = statement.query_map([], |row| {
        Ok(Draft {
            id: 0,
            title: row.get(0)?,
            text: row.get(1)?,
        })
    })?;
    Ok(closed.collect::<Result<Vec<_>, _>>()?)
}

/// The saved active index, or 0 when none was saved.
fn load_active(connection: &Connection) -> Result<usize, StoreError> {
    let active: Option<i64> = connection
        .query_row("SELECT value FROM state WHERE key = 'active'", [], |row| {
            row.get(0)
        })
        .optional()?;
    Ok(usize::try_from(active.unwrap_or(0)).unwrap_or(0))
}

/// Replaces the open tabs.
fn save_tabs(connection: &Connection, tabs: &[Draft]) -> Result<(), StoreError> {
    connection.execute("DELETE FROM drafts", [])?;
    for (position, draft) in tabs.iter().enumerate() {
        connection.execute(
            "INSERT INTO drafts (id, position, title, text) VALUES (?1, ?2, ?3, ?4)",
            params![draft.id, index(position), draft.title, draft.text],
        )?;
    }
    Ok(())
}

/// Replaces the recently closed tabs.
fn save_closed(connection: &Connection, closed: &[Draft]) -> Result<(), StoreError> {
    connection.execute("DELETE FROM closed_tabs", [])?;
    for (position, draft) in closed.iter().enumerate() {
        connection.execute(
            "INSERT INTO closed_tabs (position, title, text) VALUES (?1, ?2, ?3)",
            params![index(position), draft.title, draft.text],
        )?;
    }
    Ok(())
}

impl DraftStore for SqliteStore {
    fn load(&self) -> Result<Drafts, StoreError> {
        let connection = self.connection();
        let tabs = load_tabs(&connection)?;
        let active = load_active(&connection)?;
        Ok(Drafts {
            active: active.min(tabs.len().saturating_sub(1)),
            tabs,
            closed: load_closed(&connection)?,
        })
    }

    fn save(&self, drafts: &Drafts) -> Result<(), StoreError> {
        let mut connection = self.connection();
        let transaction = connection.transaction()?;
        save_tabs(&transaction, &drafts.tabs)?;
        save_closed(&transaction, &drafts.closed)?;
        transaction.execute(
            "INSERT OR REPLACE INTO state (key, value) VALUES ('active', ?1)",
            params![index(drafts.active)],
        )?;
        transaction.commit()?;
        Ok(())
    }
}
