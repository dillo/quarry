//! Checking an answer: run it and the reference solution, then compare
//! their results with the grading pipeline.

use quarry_engine::error::EngineError;

use crate::{
    kind::KindRegistry,
    rules::{self, CheckFailure, Comparison},
    run::{LIMITS, Limits, RunError},
    sandbox::Template,
    track::Challenge,
};

/// What a check decided.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The answer matches the solution.
    Solved,
    /// The answer is wrong; the failure says how.
    Wrong(CheckFailure),
}

/// Why an answer could not be checked.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckError {
    /// The user's SQL failed; the database's message.
    Sql(String),
    /// The user's SQL ran out of time.
    TimedOut,
    /// The user's SQL returned more rows than the cap.
    TooManyRows,
    /// The challenge itself is broken: its solution failed, or its kind is
    /// unknown. Says why, for the content's authors.
    Content(String),
    /// The sandbox copy could not be made.
    Sandbox(EngineError),
}

impl From<RunError> for CheckError {
    fn from(error: RunError) -> Self {
        match error {
            RunError::Sql(error) => Self::Sql(error.to_string()),
            RunError::TimedOut => Self::TimedOut,
            RunError::TooManyRows => Self::TooManyRows,
        }
    }
}

/// Checks `sql` as an answer to `challenge`, on copies of `template`, within
/// the grading [`LIMITS`].
///
/// # Errors
/// Returns a [`CheckError`] when the answer or the solution gives no
/// result.
pub fn check(template: &Template, challenge: &Challenge, sql: &str) -> Result<Verdict, CheckError> {
    check_within(template, challenge, sql, LIMITS)
}

/// [`check`] within `limits`.
///
/// # Errors
/// As for [`check`].
pub fn check_within(
    template: &Template,
    challenge: &Challenge,
    sql: &str,
    limits: Limits,
) -> Result<Verdict, CheckError> {
    let kinds = KindRegistry::standard();
    let kind = kinds
        .get(&challenge.kind)
        .ok_or_else(|| CheckError::Content(format!("unknown kind \"{}\"", challenge.kind)))?;
    let answers = kind.answers(challenge, sql, template, limits)?;
    let comparison = Comparison {
        expected: &answers.expected,
        got: &answers.got,
        ordered: challenge.ordered,
    };
    Ok(rules::first_failure(&comparison).map_or(Verdict::Solved, Verdict::Wrong))
}

#[cfg(test)]
mod tests;
