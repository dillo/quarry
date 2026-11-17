//! What can be wrong with a track's content.

/// Why a track could not be loaded. Each names the file at fault.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ContentError {
    /// A TOML file is not valid TOML, or not the expected shape.
    #[error("{file}: {message}")]
    Parse {
        /// The file, such as `vinyl-vault/track.toml`.
        file: String,
        /// What the parser said.
        message: String,
    },
    /// A challenge's field has a value that is not allowed.
    #[error("{file}: challenge \"{challenge}\": {field} {problem}")]
    Field {
        /// The file, such as `vinyl-vault/challenges.toml`.
        file: String,
        /// The challenge's id as written.
        challenge: String,
        /// The field at fault.
        field: &'static str,
        /// What is wrong with it.
        problem: &'static str,
    },
    /// Two challenges in one track share an id.
    #[error("{file}: challenge id \"{challenge}\" is used twice")]
    Duplicate {
        /// The file, such as `vinyl-vault/challenges.toml`.
        file: String,
        /// The repeated id.
        challenge: String,
    },
}

#[cfg(test)]
mod tests {
    use super::ContentError;

    #[test]
    fn messages_name_the_file_and_the_challenge() {
        let parse = ContentError::Parse {
            file: "vinyl-vault/track.toml".to_owned(),
            message: "missing field `title`".to_owned(),
        };
        assert_eq!(
            parse.to_string(),
            "vinyl-vault/track.toml: missing field `title`"
        );
        let field = ContentError::Field {
            file: "vinyl-vault/challenges.toml".to_owned(),
            challenge: "first-spin".to_owned(),
            field: "hints",
            problem: "must hold exactly 3 hints",
        };
        assert_eq!(
            field.to_string(),
            "vinyl-vault/challenges.toml: challenge \"first-spin\": hints must hold exactly 3 hints"
        );
        let duplicate = ContentError::Duplicate {
            file: "vinyl-vault/challenges.toml".to_owned(),
            challenge: "first-spin".to_owned(),
        };
        assert_eq!(
            duplicate.to_string(),
            "vinyl-vault/challenges.toml: challenge id \"first-spin\" is used twice"
        );
    }
}
