//! What the inspector's brief says about a challenge: where it sits, its
//! story, what a right answer looks like, and the attempts so far.
//!
//! It never shows the expected rows; only their columns and count.

use quarry_challenges::{
    catalog::Position,
    check::{CheckError, Expectation},
    track::{Challenge, Track},
};

use super::{attempts::Tally, format};

/// What a right answer looks like, as far as it is known.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum Expected {
    /// Still being worked out.
    #[default]
    Pending,
    /// Worked out.
    Ready(Expectation),
    /// The solution or its database failed.
    Unavailable,
}

impl From<Result<Expectation, CheckError>> for Expected {
    fn from(result: Result<Expectation, CheckError>) -> Self {
        result.map_or(Self::Unavailable, Self::Ready)
    }
}

/// The brief, line by line.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Brief {
    /// "Vinyl Vault · Challenge 2 of 5".
    pub place: String,
    /// The challenge's title.
    pub title: String,
    /// Its story.
    pub prompt: String,
    /// The expected columns and row count, or why they aren't shown.
    pub expected: Vec<String>,
    /// "Order matters" or "Any order".
    pub order: &'static str,
    /// The concepts it practises.
    pub concepts: Vec<String>,
    /// "Attempts: 2".
    pub attempts: String,
}

/// The brief for `challenge`, challenge `position` of `track`.
pub fn brief(
    track: &Track,
    challenge: &Challenge,
    position: Position,
    expected: &Expected,
    tally: Tally,
) -> Brief {
    Brief {
        place: format!(
            "{} · Challenge {} of {}",
            track.title, position.index, position.of
        ),
        title: challenge.title.clone(),
        prompt: challenge.prompt.clone(),
        expected: expected_lines(expected),
        order: if challenge.ordered {
            "Order matters"
        } else {
            "Any order"
        },
        concepts: challenge.concepts.clone(),
        attempts: format!("Attempts: {}", tally.checks),
    }
}

/// "Expected columns: name, year" and "12 rows".
fn expected_lines(expected: &Expected) -> Vec<String> {
    match expected {
        Expected::Pending => vec!["Working out the expected result…".to_owned()],
        Expected::Unavailable => vec!["Expected result unavailable".to_owned()],
        Expected::Ready(expectation) => vec![
            format!("Expected columns: {}", expectation.columns.join(", ")),
            rows(expectation.rows),
        ],
    }
}

/// "1 row", "1,200 rows".
fn rows(n: usize) -> String {
    let s = if n == 1 { "" } else { "s" };
    format!(
        "{} row{s}",
        format::count(u64::try_from(n).unwrap_or(u64::MAX))
    )
}

#[cfg(test)]
mod tests;
