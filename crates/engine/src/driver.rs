//! The contract every database driver implements.

use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use crate::{
    error::EngineError,
    plan::Plan,
    result::{RowSink, Summary},
    schema::Schema,
};

/// A connection to one database.
///
/// Drivers are synchronous; a [`crate::session::Session`] runs them on their
/// own thread so the UI never blocks.
pub trait Driver: Send {
    /// Runs one SQL statement, streaming any rows into `sink`.
    ///
    /// # Errors
    /// Returns [`EngineError::Sql`] when the database rejects the statement and
    /// [`EngineError::Cancelled`] when the driver's [`Canceller`] fires.
    fn execute(&mut self, sql: &str, sink: &mut dyn RowSink) -> Result<Summary, EngineError>;

    /// Reads every table and view with its columns.
    ///
    /// # Errors
    /// Returns an error when the catalog cannot be read.
    fn schema(&mut self) -> Result<Schema, EngineError>;

    /// Describes how the database would run the first statement in `sql`,
    /// without running it.
    ///
    /// # Errors
    /// Returns [`EngineError::PlanUnsupported`] unless the driver overrides
    /// it, or the database's error for SQL it cannot plan.
    fn plan(&mut self, sql: &str) -> Result<Plan, EngineError> {
        let _ = sql;
        Err(EngineError::PlanUnsupported)
    }

    /// The flag that interrupts whatever this driver is running.
    fn canceller(&self) -> Canceller;
}

/// A shared cancel flag; cloning shares the same flag.
#[derive(Debug, Clone, Default)]
pub struct Canceller(Arc<AtomicBool>);

impl Canceller {
    /// Requests cancellation.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Clears a previous request.
    pub fn reset(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    /// Whether cancellation has been requested.
    pub fn is_cancelled(&self) -> bool {
        self.0.load(Ordering::SeqCst)
    }
}

#[cfg(test)]
mod tests {
    use super::Canceller;

    #[test]
    fn clones_share_one_flag_that_can_be_reset() {
        let canceller = Canceller::default();
        let clone = canceller.clone();
        assert!(!clone.is_cancelled());
        canceller.cancel();
        assert!(clone.is_cancelled());
        clone.reset();
        assert!(!canceller.is_cancelled());
    }
}
