//! A track's database: built once in memory from its schema and seed, then
//! copied for every run that needs a clean one.

use std::sync::{Mutex, PoisonError};

use quarry_engine::{error::EngineError, sqlite::SqliteDriver};

/// A track's database as its schema and seed leave it. It is never run
/// against; [`Template::fresh`] gives copies to run on.
pub struct Template {
    database: Mutex<SqliteDriver>,
}

impl Template {
    /// Builds the template from a track's `schema` and `seed` scripts.
    ///
    /// # Errors
    /// Returns the engine's error for the first statement that fails.
    pub fn new(schema: &str, seed: &str) -> Result<Self, EngineError> {
        let mut database = SqliteDriver::open_in_memory()?;
        database.execute_script(schema)?;
        database.execute_script(seed)?;
        Ok(Self {
            database: Mutex::new(database),
        })
    }

    /// A new in-memory copy of the template, with its own canceller.
    ///
    /// # Errors
    /// Returns the engine's error if the copy cannot be made.
    pub fn fresh(&self) -> Result<SqliteDriver, EngineError> {
        // Copying only reads the template, so a panic elsewhere while the
        // lock was held leaves nothing half-done.
        let database = self.database.lock().unwrap_or_else(PoisonError::into_inner);
        SqliteDriver::copy_of(&database)
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::{driver::Driver, error::EngineError, result::QueryResult, value::Value};

    use super::Template;

    const SCHEMA: &str = "CREATE TABLE albums (title TEXT NOT NULL);";
    const SEED: &str = "INSERT INTO albums VALUES ('Blue'), ('Green');";

    fn count(driver: &mut impl Driver) -> Result<Vec<Vec<Value>>, EngineError> {
        let mut result = QueryResult::default();
        driver.execute("SELECT count(*) FROM albums", &mut result)?;
        Ok(result.rows)
    }

    #[test]
    fn every_copy_starts_from_the_seed() {
        let template = Template::new(SCHEMA, SEED).unwrap();
        let mut first = template.fresh().unwrap();
        first.execute_script("DELETE FROM albums").unwrap();
        assert_eq!(count(&mut first).unwrap(), [[Value::Int(0)]]);
        let mut second = template.fresh().unwrap();
        assert_eq!(count(&mut second).unwrap(), [[Value::Int(2)]]);
    }

    #[test]
    fn a_broken_schema_or_seed_is_the_engine_error() {
        let schema = Template::new("CREATE TABLE", SEED).err().unwrap();
        assert!(matches!(schema, EngineError::Sql(_)));
        let seed = Template::new(SCHEMA, "INSERT INTO nowhere VALUES (1)")
            .err()
            .unwrap();
        assert_eq!(seed, EngineError::Sql("no such table: nowhere".to_owned()));
    }
}
