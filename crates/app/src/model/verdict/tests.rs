use quarry_challenges::{
    check::{CheckError, Verdict},
    rules::{CheckFailure, Grading, RuleOutcome, RuleStatus},
};
use quarry_engine::error::EngineError;

use super::{CheckState, Report, Tone, report};

fn wrong(failure: CheckFailure) -> Report {
    let outcome = RuleOutcome {
        rule: "columns",
        status: RuleStatus::Failed(failure),
    };
    report(&Ok(Verdict::of(Grading::new(vec![outcome]))))
}

fn detail(failure: CheckFailure) -> Option<String> {
    wrong(failure).detail
}

#[test]
fn nothing_is_checked_at_first() {
    assert_eq!(CheckState::default(), CheckState::Idle);
}

#[test]
fn a_solved_answer_says_so() {
    assert_eq!(
        report(&Ok(Verdict::of(Grading::new(Vec::new())))),
        Report {
            tone: Tone::Solved,
            headline: "Solved.".to_owned(),
            detail: None,
        }
    );
}

#[test]
fn a_wrong_answer_is_not_quite_yet() {
    let report = wrong(CheckFailure::RowCount {
        expected: 7,
        got: 8,
    });
    assert_eq!(report.tone, Tone::Wrong);
    assert_eq!(report.headline, "Not quite yet.");
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
    let failed = report(&Err(CheckError::Sql("no such column: nope".to_owned())));
    assert_eq!(
        (
            failed.tone,
            failed.headline.as_str(),
            failed.detail.as_deref()
        ),
        (
            Tone::Failed,
            "Your query failed.",
            Some("no such column: nope")
        )
    );
    let headline = |error| report(&Err(error)).headline;
    assert_eq!(headline(CheckError::TimedOut), "Stopped after 5 seconds.");
    assert_eq!(
        headline(CheckError::TooManyRows),
        "Returned more than 10,000 rows."
    );
    let content = report(&Err(CheckError::Content("no such table".to_owned())));
    assert_eq!(
        (content.headline.as_str(), content.detail),
        (
            "This challenge has a problem; your answer wasn't checked.",
            None
        )
    );
}

#[test]
fn a_failed_copy_gives_the_engine_error() {
    let copy = report(&Err(CheckError::Sandbox(EngineError::Cancelled)));
    assert_eq!(copy.tone, Tone::Failed);
    assert_eq!(
        copy.headline,
        "The challenge's database couldn't be copied."
    );
    assert_eq!(copy.detail.as_deref(), Some("Query cancelled."));
}
