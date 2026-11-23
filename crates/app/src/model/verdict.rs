//! What the feedback card says about a tab's latest check.
//!
//! A wrong answer says what differs, rule by rule, never how to fix it,
//! and never shows the expected rows.

use quarry_challenges::{
    check::{CheckError, Verdict},
    rules::{CheckFailure, Grading, RuleOutcome, RuleStatus},
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

/// How a card reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// The answer is right.
    Solved,
    /// The answer is wrong.
    Wrong,
    /// The answer couldn't be checked.
    Failed,
}

/// Where one rule stands on the card.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// ✓: it passed.
    Pass,
    /// ✗: it failed.
    Fail,
    /// –: it wasn't checked.
    Skip,
}

impl Mark {
    /// The mark's glyph.
    pub fn glyph(self) -> &'static str {
        match self {
            Self::Pass => "✓",
            Self::Fail => "✗",
            Self::Skip => "–",
        }
    }
}

/// One rule's line in the checklist.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    /// Whether it passed, failed or wasn't checked.
    pub mark: Mark,
    /// What it says, such as "Columns match".
    pub text: String,
}

/// What follows a solve.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Footer {
    /// "Next up: <title>", with Next challenge and Stay here.
    Next(String),
    /// "Last challenge": there is no next one.
    Last,
}

/// What the card needs to know beyond the check: the challenge's title,
/// which attempt this was, and the next challenge's title.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Solve {
    /// The challenge's title.
    pub title: String,
    /// Which attempt the check was, counting from 1.
    pub attempt: u32,
    /// The next challenge's title, if there is one.
    pub next: Option<String>,
}

/// A finished check, as the feedback card shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Card {
    /// How it reads.
    pub tone: Tone,
    /// The first line, such as "Not quite yet".
    pub headline: String,
    /// The line under it.
    pub subline: Option<String>,
    /// Each rule's line, for a wrong answer.
    pub lines: Vec<Line>,
    /// What follows a solve.
    pub footer: Option<Footer>,
}

impl Card {
    fn new(tone: Tone, headline: &str, subline: Option<String>) -> Self {
        Self {
            tone,
            headline: headline.to_owned(),
            subline,
            lines: Vec::new(),
            footer: None,
        }
    }
}

/// The card for a finished check of the challenge `solve` describes.
pub fn card(outcome: &Result<Verdict, CheckError>, solve: &Solve) -> Card {
    match outcome {
        Ok(verdict) if verdict.is_solved() => solved(solve),
        Ok(verdict) => wrong(verdict.grading()),
        Err(error) => unchecked(error),
    }
}

/// "First spin: solved", the attempt, and what comes next.
fn solved(solve: &Solve) -> Card {
    let mut card = Card::new(
        Tone::Solved,
        &format!("{}: solved", solve.title),
        Some(format!("Attempt {}", solve.attempt)),
    );
    card.footer = Some(solve.next.clone().map_or(Footer::Last, Footer::Next));
    card
}

/// "Not quite yet", and every rule's line.
fn wrong(grading: &Grading) -> Card {
    let mut card = Card::new(
        Tone::Wrong,
        "Not quite yet",
        Some("Here's what's different. Your query is unchanged.".to_owned()),
    );
    let blocker = grading
        .outcomes()
        .iter()
        .find(|outcome| matches!(outcome.status, RuleStatus::Failed(_)))
        .map(|outcome| outcome.rule);
    card.lines = grading
        .outcomes()
        .iter()
        .map(|outcome| line(outcome, blocker))
        .collect();
    card
}

/// `outcome`'s line; `blocker` is the rule that failed, if one did.
fn line(outcome: &RuleOutcome, blocker: Option<&str>) -> Line {
    let name = rule_name(outcome.rule);
    let (mark, text) = match (&outcome.status, blocker) {
        (RuleStatus::Passed, _) => (Mark::Pass, name.agrees()),
        (RuleStatus::Failed(failure), _) => (Mark::Fail, difference(failure)),
        (RuleStatus::NotChecked, Some(blocker)) => (
            Mark::Skip,
            format!(
                "{} not checked until {}",
                name.label,
                rule_name(blocker).condition()
            ),
        ),
        (RuleStatus::NotChecked, None) => {
            (Mark::Skip, format!("{} doesn't matter here", name.label))
        }
    };
    Line { mark, text }
}

/// How a rule is named on the card.
struct RuleName {
    label: &'static str,
    plural: bool,
}

impl RuleName {
    /// "Columns match", "Row count matches".
    fn agrees(&self) -> String {
        format!("{} {}", self.label, self.verb())
    }

    /// "the columns match", "the row count matches".
    fn condition(&self) -> String {
        format!("the {} {}", self.label.to_lowercase(), self.verb())
    }

    fn verb(&self) -> &'static str {
        if self.plural { "match" } else { "matches" }
    }
}

/// The name of the rule with id `rule`.
fn rule_name(rule: &str) -> RuleName {
    let (label, plural) = match rule {
        "columns" => ("Columns", true),
        "row count" => ("Row count", false),
        "contents" => ("Rows", true),
        "order" => ("Order", false),
        _ => ("This check", false),
    };
    RuleName { label, plural }
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
fn unchecked(error: &CheckError) -> Card {
    match error {
        CheckError::Sql(message) => {
            Card::new(Tone::Failed, "Your query failed.", Some(message.clone()))
        }
        CheckError::TimedOut => Card::new(
            Tone::Failed,
            &format!("Stopped after {} seconds.", LIMITS.time.as_secs()),
            None,
        ),
        CheckError::TooManyRows => Card::new(
            Tone::Failed,
            &format!("Returned more than {} rows.", count(LIMITS.rows)),
            None,
        ),
        CheckError::Content(_) => Card::new(
            Tone::Failed,
            "This challenge has a problem; your answer wasn't checked.",
            None,
        ),
        CheckError::Sandbox(error) => Card::new(
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
