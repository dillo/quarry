//! Errors a user can see, each with a readable message.

/// Why the store could not be opened, read or written.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum StoreError {
    /// The store file could not be opened or created.
    #[error("Cannot open {path}: {message}")]
    Open {
        /// The file being opened.
        path: String,
        /// Why it failed.
        message: String,
    },
    /// The file was written by a newer Quarry; it is left untouched.
    #[error("Saved by a newer Quarry (version {found}; this one reads up to {supported}).")]
    TooNew {
        /// The file's schema version.
        found: i64,
        /// The newest schema version this build knows.
        supported: i64,
    },
    /// `SQLite` reported an error.
    #[error("{0}")]
    Sql(String),
}

impl From<rusqlite::Error> for StoreError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Sql(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::StoreError;

    #[test]
    fn messages_are_readable_sentences() {
        let open = StoreError::Open {
            path: "/x/quarry.db".to_owned(),
            message: "permission denied".to_owned(),
        };
        assert_eq!(
            open.to_string(),
            "Cannot open /x/quarry.db: permission denied"
        );
        let too_new = StoreError::TooNew {
            found: 4,
            supported: 1,
        };
        assert_eq!(
            too_new.to_string(),
            "Saved by a newer Quarry (version 4; this one reads up to 1)."
        );
        let sql = StoreError::from(rusqlite::Error::InvalidQuery);
        assert_eq!(
            sql,
            StoreError::Sql(rusqlite::Error::InvalidQuery.to_string())
        );
    }
}
