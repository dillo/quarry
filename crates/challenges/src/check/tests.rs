use std::time::Duration;

use quarry_engine::error::EngineError;

use super::{CheckError, Expectation, Verdict, check, check_within, expect};
use crate::{
    rules::CheckFailure,
    run::{LIMITS, Limits, RunError},
    sandbox::Template,
    track::Challenge,
};

fn template() -> Result<Template, EngineError> {
    Template::new(
        "CREATE TABLE albums (title TEXT, year INTEGER);",
        "INSERT INTO albums VALUES ('Blue', 1971), ('Kind', 1959), ('Ok', NULL);",
    )
}

fn challenge(solution: &str, ordered: bool) -> Challenge {
    Challenge {
        id: "c".to_owned(),
        title: "C".to_owned(),
        prompt: "P".to_owned(),
        kind: "query".to_owned(),
        ordered,
        concepts: Vec::new(),
        solution: solution.to_owned(),
        hints: Vec::new(),
        skeleton: String::new(),
    }
}

#[test]
fn a_matching_answer_is_solved() {
    let challenge = challenge("SELECT title FROM albums WHERE year < 1965", false);
    let answer = "select TITLE from albums where year = 1959";
    let verdict = check(&template().unwrap(), &challenge, answer).unwrap();
    assert!(verdict.is_solved());
    assert_eq!(verdict.failure(), None);
    assert!(matches!(verdict, Verdict::Solved(_)));
}

#[test]
fn a_wrong_answer_reports_the_first_failure() {
    let ordered = challenge("SELECT title FROM albums ORDER BY year DESC", true);
    let answer = "SELECT title FROM albums ORDER BY year";
    let wrong = check(&template().unwrap(), &ordered, answer).unwrap();
    assert!(!wrong.is_solved());
    assert!(matches!(wrong, Verdict::Wrong(_)));
    assert_eq!(wrong.failure(), Some(&CheckFailure::Order { row: 0 }));
    assert_eq!(wrong.grading().outcomes().len(), 4);
    let unordered = challenge("SELECT title FROM albums ORDER BY year DESC", false);
    let verdict = check(&template().unwrap(), &unordered, answer).unwrap();
    assert!(verdict.is_solved());
}

#[test]
fn an_unknown_kind_is_a_content_error() {
    let mut mutation = challenge("SELECT 1", false);
    mutation.kind = "mutation".to_owned();
    assert_eq!(
        check(&template().unwrap(), &mutation, "SELECT 1"),
        Err(CheckError::Content("unknown kind \"mutation\"".to_owned()))
    );
}

#[test]
fn the_limits_apply_to_the_answer() {
    let endless =
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
    let limits = Limits {
        rows: LIMITS.rows,
        time: Duration::from_millis(50),
    };
    let challenge = challenge("SELECT 1", false);
    assert_eq!(
        check_within(&template().unwrap(), &challenge, endless, limits),
        Err(CheckError::TimedOut)
    );
}

#[test]
fn run_errors_become_check_errors() {
    let sql = RunError::Sql(EngineError::Sql("near \"FORM\": syntax error".to_owned()));
    assert_eq!(
        CheckError::from(sql),
        CheckError::Sql("near \"FORM\": syntax error".to_owned())
    );
    assert_eq!(CheckError::from(RunError::TimedOut), CheckError::TimedOut);
    assert_eq!(
        CheckError::from(RunError::TooManyRows),
        CheckError::TooManyRows
    );
}

#[test]
fn the_expectation_is_the_solutions_columns_and_row_count() {
    let challenge = challenge(
        "SELECT title, year AS released FROM albums WHERE year > 1960",
        false,
    );
    assert_eq!(
        expect(&template().unwrap(), &challenge),
        Ok(Expectation {
            columns: vec!["title".to_owned(), "released".to_owned()],
            rows: 1,
        })
    );
}

#[test]
fn a_broken_solution_has_no_expectation() {
    let broken = challenge("SELECT nope FROM albums", false);
    assert_eq!(
        expect(&template().unwrap(), &broken),
        Err(CheckError::Content("no such column: nope".to_owned()))
    );
    let mut mutation = challenge("SELECT 1", false);
    mutation.kind = "mutation".to_owned();
    assert_eq!(
        expect(&template().unwrap(), &mutation),
        Err(CheckError::Content("unknown kind \"mutation\"".to_owned()))
    );
}
