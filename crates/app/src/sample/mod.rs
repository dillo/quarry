//! The built-in sample database every new workspace opens.

use quarry_engine::{error::EngineError, session::Session, sqlite::SqliteDriver};

/// The script that creates and fills the sample database.
pub const SEED: &str = include_str!("seed.sql");

/// What the status bar calls the sample database.
pub const NAME: &str = "Sample · in memory";

/// Opens a fresh in-memory copy of the sample database on its own session.
///
/// # Errors
/// Returns the engine's error if `SQLite` cannot open or seed the database.
pub fn open() -> Result<Session, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script(SEED)?;
    Ok(Session::start(Box::new(driver)))
}
