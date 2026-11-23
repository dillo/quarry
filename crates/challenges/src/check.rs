//! Checking an answer: run it and the reference solution, then compare
//! their results with the grading pipeline.

use quarry_engine::error::EngineError;

use crate::{
    kind::{ChallengeKind, KindRegistry},
    rules::{self, CheckFailure, Comparison, Grading},
    run::{LIMITS, Limits, RunError},
    sandbox::Template,
    track::Challenge,
};

/// What a check decided, with where every rule stands.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// The answer matches the solution.
    Solved(Grading),
    /// The answer is wrong; the grading says how.
    Wrong(Grading),
}

impl Verdict {
    /// The verdict `grading` gives: wrong when a rule failed.
    pub fn of(grading: Grading) -> Self {
        if grading.failure().is_some() {
            Self::Wrong(grading)
        } else {
            Self::Solved(grading)
        }
    }

    /// Every rule's outcome.
    pub fn grading(&self) -> &Grading {
        match self {
            Self::Solved(grading) | Self::Wrong(grading) => grading,
        }
    }

    /// What was wrong, for a wrong answer.
    pub fn failure(&self) -> Option<&CheckFailure> {
        self.grading().failure()
    }

    /// Whether the answer is right.
    pub fn is_solved(&self) -> bool {
        matches!(self, Self::Solved(_))
    }
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
    let answers = kind_of(&kinds, challenge)?.answers(challenge, sql, template, limits)?;
    let comparison = Comparison {
        expected: &answers.expected,
        got: &answers.got,
        ordered: challenge.ordered,
    };
    Ok(Verdict::of(rules::grade(&comparison)))
}

/// What a right answer to a challenge looks like, without its rows: the
/// brief shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Expectation {
    /// The column names, in order.
    pub columns: Vec<String>,
    /// How many rows.
    pub rows: usize,
}

/// What a right answer to `challenge` gives, worked out on a copy of
/// `template` within the grading [`LIMITS`].
///
/// # Errors
/// Returns [`CheckError::Content`] when the solution gives no result, or
/// [`CheckError::Sandbox`] when no copy can be made.
pub fn expect(template: &Template, challenge: &Challenge) -> Result<Expectation, CheckError> {
    let kinds = KindRegistry::standard();
    let expected = kind_of(&kinds, challenge)?.expected(challenge, template, LIMITS)?;
    Ok(Expectation {
        columns: expected
            .columns
            .into_iter()
            .map(|column| column.name)
            .collect(),
        rows: expected.rows.len(),
    })
}

/// `challenge`'s kind in `kinds`.
fn kind_of<'a>(
    kinds: &'a KindRegistry,
    challenge: &Challenge,
) -> Result<&'a dyn ChallengeKind, CheckError> {
    kinds
        .get(&challenge.kind)
        .ok_or_else(|| CheckError::Content(format!("unknown kind \"{}\"", challenge.kind)))
}

#[cfg(test)]
mod tests;
