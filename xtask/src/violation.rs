//! The single result type every gate reports.

use std::fmt;

/// One broken rule, with where it happened and what is wrong.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    /// Where the rule was broken: a path, `path:line`, or a crate name.
    pub location: String,
    /// What is wrong, phrased so the reader knows what to change.
    pub message: String,
}

impl Violation {
    /// Creates a violation at `location` with `message`.
    pub fn new(location: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            location: location.into(),
            message: message.into(),
        }
    }
}

impl fmt::Display for Violation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}: {}", self.location, self.message)
    }
}

#[cfg(test)]
mod tests {
    use super::Violation;

    #[test]
    fn displays_location_then_message() {
        let violation = Violation::new("crates/engine/src/utils.rs", "rename it");
        assert_eq!(
            violation.to_string(),
            "crates/engine/src/utils.rs: rename it"
        );
    }
}
