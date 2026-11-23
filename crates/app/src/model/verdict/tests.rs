use quarry_challenges::{
    check::{CheckError, Verdict},
    rules::{CheckFailure, Grading, RuleOutcome, RuleStatus},
};
use quarry_engine::error::EngineError;

use super::{Card, CheckState, Footer, Line, Mark, Solve, Tone, card};

fn outcome(rule: &'static str, status: RuleStatus) -> RuleOutcome {
    RuleOutcome { rule, status }
}

fn solve(next: Option<&str>) -> Solve {
    Solve {
        title: "First spin".to_owned(),
        attempt: 3,
        next: next.map(str::to_owned),
    }
}

fn graded(outcomes: Vec<RuleOutcome>) -> Card {
    card(&Ok(Verdict::of(Grading::new(outcomes))), &solve(None))
}

fn errored(error: CheckError) -> Card {
    card(&Err(error), &solve(None))
}

/// The line a lone failure gives.
fn detail(failure: CheckFailure) -> Option<String> {
    let card = graded(vec![outcome("columns", RuleStatus::Failed(failure))]);
    card.lines.first().map(|line| line.text.clone())
}

fn lines(card: &Card) -> Vec<(Mark, &str)> {
    card.lines
        .iter()
        .map(|line| (line.mark, line.text.as_str()))
        .collect()
}

#[test]
fn nothing_is_checked_at_first() {
    assert_eq!(CheckState::default(), CheckState::Idle);
}

#[test]
fn a_solve_names_the_challenge_the_attempt_and_whats_next() {
    let solved = card(
        &Ok(Verdict::of(Grading::new(Vec::new()))),
        &solve(Some("Crate digging")),
    );
    assert_eq!(
        solved,
        Card {
            tone: Tone::Solved,
            headline: "First spin: solved".to_owned(),
            subline: Some("Attempt 3".to_owned()),
            lines: Vec::new(),
            footer: Some(Footer::Next("Crate digging".to_owned())),
        }
    );
    let last = graded(Vec::new());
    assert_eq!(last.footer, Some(Footer::Last));
}

#[test]
fn a_wrong_answer_lists_every_rule() {
    let failure = CheckFailure::RowCount {
        expected: 4,
        got: 7,
    };
    let wrong = graded(vec![
        outcome("columns", RuleStatus::Passed),
        outcome("row count", RuleStatus::Failed(failure)),
        outcome("contents", RuleStatus::NotChecked),
        outcome("order", RuleStatus::NotChecked),
    ]);
    assert_eq!(wrong.tone, Tone::Wrong);
    assert_eq!(wrong.headline, "Not quite yet");
    assert_eq!(
        wrong.subline.as_deref(),
        Some("Here's what's different. Your query is unchanged.")
    );
    assert_eq!(wrong.footer, None);
    assert_eq!(
        lines(&wrong),
        [
            (Mark::Pass, "Columns match"),
            (Mark::Fail, "Expected 4 rows, got 7."),
            (Mark::Skip, "Rows not checked until the row count matches"),
            (Mark::Skip, "Order not checked until the row count matches"),
        ]
    );
}

#[test]
fn every_rule_has_a_name() {
    let passed = graded(vec![
        outcome("row count", RuleStatus::Passed),
        outcome("contents", RuleStatus::Passed),
        outcome("order", RuleStatus::Passed),
        outcome("new", RuleStatus::Passed),
        outcome(
            "columns",
            RuleStatus::Failed(CheckFailure::Order { row: 0 }),
        ),
        outcome("new", RuleStatus::NotChecked),
    ]);
    assert_eq!(
        lines(&passed),
        [
            (Mark::Pass, "Row count matches"),
            (Mark::Pass, "Rows match"),
            (Mark::Pass, "Order matches"),
            (Mark::Pass, "This check matches"),
            (Mark::Fail, "The rows are right, but row 1 is out of order."),
            (Mark::Skip, "This check not checked until the columns match"),
        ]
    );
}

#[test]
fn a_rule_that_doesnt_apply_says_so_even_after_a_failure() {
    let unordered = graded(vec![
        outcome("columns", RuleStatus::Passed),
        outcome("row count", RuleStatus::Passed),
        outcome(
            "contents",
            RuleStatus::Failed(CheckFailure::Contents {
                missing: 1,
                unexpected: 0,
            }),
        ),
        outcome("order", RuleStatus::NotApplicable),
    ]);
    assert_eq!(
        unordered.lines[3],
        Line {
            mark: Mark::Skip,
            text: "Order doesn't matter here".to_owned()
        }
    );
}

#[test]
fn a_rule_not_checked_without_a_failure_just_says_so() {
    let line = super::line(&outcome("order", RuleStatus::NotChecked), None);
    assert_eq!(
        line,
        Line {
            mark: Mark::Skip,
            text: "Order not checked".to_owned()
        }
    );
}

#[test]
fn marks_have_glyphs() {
    assert_eq!(
        [Mark::Pass, Mark::Fail, Mark::Skip].map(Mark::glyph),
        ["✓", "✗", "–"]
    );
}

#[test]
fn column_differences_are_described() {
    assert_eq!(
        detail(CheckFailure::ColumnCount {
            expected: 2,
            got: 3
        })
        .as_deref(),
        Some("Expected 2 columns, got 3.")
    );
    assert_eq!(
        detail(CheckFailure::ColumnCount {
            expected: 1,
            got: 2
        })
        .as_deref(),
        Some("Expected 1 column, got 2.")
    );
    let names = CheckFailure::ColumnNames {
        expected: vec!["title".to_owned(), "year".to_owned()],
        got: vec!["title".to_owned(), "released".to_owned()],
    };
    assert_eq!(
        detail(names).as_deref(),
        Some("Expected the columns title, year, got title, released.")
    );
}

#[test]
fn row_count_and_contents_are_described() {
    assert_eq!(
        detail(CheckFailure::RowCount {
            expected: 1,
            got: 12_000
        })
        .as_deref(),
        Some("Expected 1 row, got 12,000.")
    );
    let contents = |missing, unexpected| {
        detail(CheckFailure::Contents {
            missing,
            unexpected,
        })
    };
    assert_eq!(
        contents(2, 1).as_deref(),
        Some("2 rows missing, 1 row unexpected.")
    );
    assert_eq!(contents(1, 0).as_deref(), Some("1 row missing."));
    assert_eq!(contents(0, 3).as_deref(), Some("3 rows unexpected."));
}

#[test]
fn order_names_the_first_row_counting_from_one() {
    assert_eq!(
        detail(CheckFailure::Order { row: 2 }).as_deref(),
        Some("The rows are right, but row 3 is out of order.")
    );
}

#[test]
fn errors_say_why_nothing_was_checked() {
    let failed = errored(CheckError::Sql("no such column: nope".to_owned()));
    assert_eq!(
        (
            failed.tone,
            failed.headline.as_str(),
            failed.subline.as_deref()
        ),
        (
            Tone::Failed,
            "Your query failed.",
            Some("no such column: nope")
        )
    );
    let headline = |error| errored(error).headline;
    assert_eq!(headline(CheckError::TimedOut), "Stopped after 5 seconds.");
    assert_eq!(
        headline(CheckError::TooManyRows),
        "Returned more than 10,000 rows."
    );
    let content = errored(CheckError::Content("no such table".to_owned()));
    assert_eq!(
        (content.headline.as_str(), content.subline),
        (
            "This challenge has a problem; your answer wasn't checked.",
            None
        )
    );
}

#[test]
fn a_failed_copy_gives_the_engine_error() {
    let copy = errored(CheckError::Sandbox(EngineError::Cancelled));
    assert_eq!(copy.tone, Tone::Failed);
    assert_eq!(
        copy.headline,
        "The challenge's database couldn't be copied."
    );
    assert_eq!(copy.subline.as_deref(), Some("Query cancelled."));
    assert!(copy.lines.is_empty());
}
