//! Errors that stop a gate from running at all (as opposed to violations).

/// A failure to gather a gate's inputs.
#[derive(Debug, thiserror::Error)]
pub enum XtaskError {
    /// A file could not be read.
    #[error("cannot read {path}: {source}")]
    Io {
        /// The file or directory involved.
        path: String,
        /// The underlying I/O error.
        #[source]
        source: std::io::Error,
    },
    /// A directory tree could not be walked.
    #[error("cannot walk {path}: {detail}")]
    Walk {
        /// The directory being walked.
        path: String,
        /// What went wrong.
        detail: String,
    },
    /// A tool printed JSON we could not parse.
    #[error("invalid JSON from {what}: {source}")]
    Json {
        /// Which tool or file produced the JSON.
        what: String,
        /// The parse error.
        #[source]
        source: serde_json::Error,
    },
    /// A TOML file could not be parsed.
    #[error("invalid TOML in {path}: {source}")]
    Toml {
        /// The file involved.
        path: String,
        /// The parse error.
        #[source]
        source: toml::de::Error,
    },
    /// An external command failed to start or exited unsuccessfully.
    #[error("command `{command}` failed: {detail}")]
    Command {
        /// The command line that was run.
        command: String,
        /// Its error output or start-up error.
        detail: String,
    },
    /// A built-in regular expression failed to compile.
    #[error("invalid built-in pattern: {0}")]
    Pattern(#[from] regex::Error),
}
