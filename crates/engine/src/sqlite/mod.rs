//! The built-in SQLite driver (bundled `SQLite`, no system library needed).

mod convert;
mod introspect;
mod run;

use std::path::Path;

use rusqlite::Connection;

use crate::{
    driver::{Canceller, Driver},
    error::EngineError,
    result::{RowSink, Summary},
    schema::Schema,
};

/// How many virtual-machine steps run between cancellation checks.
const CANCEL_CHECK_STEPS: i32 = 1_000;

/// A connection to one `SQLite` database file or in-memory database.
pub struct SqliteDriver {
    connection: Connection,
    canceller: Canceller,
}

impl SqliteDriver {
    /// Opens (or creates) the database file at `path`.
    ///
    /// # Errors
    /// Returns [`EngineError::Open`] when the file cannot be opened.
    pub fn open(path: &Path) -> Result<Self, EngineError> {
        let target = path.display().to_string();
        Self::from_connection(Connection::open(path), &target)
    }

    /// Opens a fresh, empty in-memory database.
    ///
    /// # Errors
    /// Returns [`EngineError::Open`] if `SQLite` cannot allocate it.
    pub fn open_in_memory() -> Result<Self, EngineError> {
        Self::from_connection(Connection::open_in_memory(), ":memory:")
    }

    /// Runs a whole script with no results, e.g. to load a schema and seed data.
    ///
    /// # Errors
    /// Returns [`EngineError::Sql`] for the first failing statement.
    pub fn execute_script(&mut self, sql: &str) -> Result<(), EngineError> {
        self.connection.execute_batch(sql).map_err(convert::error)
    }

    fn from_connection(
        opened: rusqlite::Result<Connection>,
        target: &str,
    ) -> Result<Self, EngineError> {
        let open_error = |error: rusqlite::Error| EngineError::Open {
            target: target.to_owned(),
            message: error.to_string(),
        };
        let mut connection = opened.map_err(open_error)?;
        let canceller = Canceller::default();
        let flag = canceller.clone();
        let handler = Some(move || flag.is_cancelled());
        connection
            .progress_handler(CANCEL_CHECK_STEPS, handler)
            .map_err(open_error)?;
        Ok(Self {
            connection,
            canceller,
        })
    }
}

impl Driver for SqliteDriver {
    fn execute(&mut self, sql: &str, sink: &mut dyn RowSink) -> Result<Summary, EngineError> {
        run::execute(&self.connection, sql, sink)
    }

    fn schema(&mut self) -> Result<Schema, EngineError> {
        introspect::schema(&self.connection)
    }

    fn canceller(&self) -> Canceller {
        self.canceller.clone()
    }
}

/// Registry entry point: `target` is a file path or `:memory:`.
///
/// # Errors
/// Returns [`EngineError::Open`] when the database cannot be opened.
pub fn connect(target: &str) -> Result<Box<dyn Driver>, EngineError> {
    if target.is_empty() {
        return Err(EngineError::Open {
            target: String::new(),
            message: "no database path given".to_owned(),
        });
    }
    let driver = if target == ":memory:" {
        SqliteDriver::open_in_memory()?
    } else {
        SqliteDriver::open(Path::new(target))?
    };
    Ok(Box::new(driver))
}
