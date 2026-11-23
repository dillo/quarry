//! What the Check view says about a tab's latest check.
//!
//! A wrong answer says what differs, never how to fix it, and never shows
//! the expected rows.

use quarry_challenges::{
    check::{CheckError, Verdict},
    rules::CheckFailure,
    run::LIMITS,
};

use super::format;

/// Where a challenge tab's latest check is.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum CheckState {
    /// Nothing has been checked yet.
    #[default]
    Idle,
    /// A check is running.
    Checking,
    /// The latest check finished.
    Done(Result<Verdict, CheckError>),
}

/// How a report reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The answer is right.
    Solved,
    /// The answer is wrong.
    Wrong,
    /// The answer couldn't be checked.
    Failed,
}

/// A finished check, in words.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// How it reads.
    pub tone: Tone,
    /// The first line, such as "Not quite yet."
    pub headline: String,
    /// What differs, or why it couldn't be checked.
    pub detail: Option<String>,
}

impl Report {
    fn new(tone: Tone, headline: &str, detail: Option<String>) -> Self {
        Self {
            tone,
            headline: headline.to_owned(),
            detail,
        }
    }
}

/// The report for a finished check.
pub fn report(outcome: &Result<Verdict, CheckError>) -> Report {
    match outcome.as_ref().map(Verdict::failure) {
        Ok(None) => Report::new(Tone::Solved, "Solved.", None),
        Ok(Some(failure)) => Report::new(Tone::Wrong, "Not quite yet.", Some(difference(failure))),
        Err(error) => unchecked(error),
    }
}

/// What differs, in one sentence.
fn difference(failure: &CheckFailure) -> String {
    match failure {
        CheckFailure::ColumnCount { expected, got } => {
            format!(
                "Expected {}, got {}.",
                plural(*expected, "column"),
                count(*got)
            )
        }
        CheckFailure::ColumnNames { expected, got } => format!(
            "Expected the columns {}, got {}.",
            expected.join(", "),
            got.join(", ")
        ),
        CheckFailure::RowCount { expected, got } => {
            format!(
                "Expected {}, got {}.",
                plural(*expected, "row"),
                count(*got)
            )
        }
        CheckFailure::Contents {
            missing,
            unexpected,
        } => contents(*missing, *unexpected),
        CheckFailure::Order { row } => {
            format!("The rows are right, but row {} is out of order.", row + 1)
        }
    }
}

/// "2 rows missing, 1 row unexpected."
fn contents(missing: usize, unexpected: usize) -> String {
    let mut parts = Vec::new();
    if missing > 0 {
        parts.push(format!("{} missing", plural(missing, "row")));
    }
    if unexpected > 0 {
        parts.push(format!("{} unexpected", plural(unexpected, "row")));
    }
    format!("{}.", parts.join(", "))
}

/// Why the answer couldn't be checked.
fn unchecked(error: &CheckError) -> Report {
    match error {
        CheckError::Sql(message) => {
            Report::new(Tone::Failed, "Your query failed.", Some(message.clone()))
        }
        CheckError::TimedOut => Report::new(
            Tone::Failed,
            &format!("Stopped after {} seconds.", LIMITS.time.as_secs()),
            None,
        ),
        CheckError::TooManyRows => Report::new(
            Tone::Failed,
            &format!("Returned more than {} rows.", count(LIMITS.rows)),
            None,
        ),
        CheckError::Content(_) => Report::new(
            Tone::Failed,
            "This challenge has a problem; your answer wasn't checked.",
            None,
        ),
        CheckError::Sandbox(error) => Report::new(
            Tone::Failed,
            "The challenge's database couldn't be copied.",
            Some(error.to_string()),
        ),
    }
}

/// `"1 column"`, `"2 columns"`.
fn plural(n: usize, noun: &str) -> String {
    let s = if n == 1 { "" } else { "s" };
    format!("{} {noun}{s}", count(n))
}

fn count(n: usize) -> String {
    format::count(u64::try_from(n).unwrap_or(u64::MAX))
}

#[cfg(test)]
mod tests;
