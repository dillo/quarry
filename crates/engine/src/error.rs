//! Errors a user can see, each with a readable message.

/// Why a database operation failed.
///
/// Every variant renders as a sentence the UI can show as is. Errors are
/// plain data (`Clone`) so they can travel from the session thread.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum EngineError {
    /// The database rejected the SQL; the message is the database's own.
    #[error("{0}")]
    Sql(String),
    /// The user cancelled the query.
    #[error("Query cancelled.")]
    Cancelled,
    /// A database could not be opened.
    #[error("Cannot open {target}: {message}")]
    Open {
        /// What was being opened (a path or `:memory:`).
        target: String,
        /// Why it failed.
        message: String,
    },
    /// No driver handles this connection string.
    #[error("Unsupported connection: {0}")]
    Unsupported(String),
    /// The driver cannot show query plans.
    #[error("This database can't show query plans.")]
    PlanUnsupported,
    /// The session thread has stopped.
    #[error("The query worker stopped unexpectedly.")]
    WorkerGone,
}

#[cfg(test)]
mod tests {
    use super::EngineError;

    #[test]
    fn messages_are_readable_sentences() {
        let open = EngineError::Open {
            target: "/tmp/x.db".to_owned(),
            message: "unable to open database file".to_owned(),
        };
        assert_eq!(
            open.to_string(),
            "Cannot open /tmp/x.db: unable to open database file"
        );
        assert_eq!(
            EngineError::Sql("no such table: t".to_owned()).to_string(),
            "no such table: t"
        );
        assert_eq!(EngineError::Cancelled.to_string(), "Query cancelled.");
        assert_eq!(
            EngineError::Unsupported("x:y".to_owned()).to_string(),
            "Unsupported connection: x:y"
        );
        assert_eq!(
            EngineError::PlanUnsupported.to_string(),
            "This database can't show query plans."
        );
        assert_eq!(
            EngineError::WorkerGone.to_string(),
            "The query worker stopped unexpectedly."
        );
    }
}
