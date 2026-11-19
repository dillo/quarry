use std::time::Duration;

use quarry_engine::{error::EngineError, sqlite::SqliteDriver, value::Value};

use super::{LIMITS, Limits, RunError, run};

const ENDLESS: &str =
    "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";

/// `n` rows of one column, 1 to `n`.
fn numbers(n: usize) -> String {
    format!(
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < {n}) SELECT x FROM n"
    )
}

fn database() -> Result<SqliteDriver, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script("CREATE TABLE t (n INTEGER); INSERT INTO t VALUES (1), (2);")?;
    Ok(driver)
}

#[test]
fn grading_allows_ten_thousand_rows_and_five_seconds() {
    assert_eq!(LIMITS.rows, 10_000);
    assert_eq!(LIMITS.time, Duration::from_secs(5));
}

#[test]
fn the_last_statement_is_the_answer_and_earlier_ones_still_run() {
    let mut driver = database().unwrap();
    let script = "DELETE FROM t WHERE n = 1; SELECT 'x' AS tag; SELECT n FROM t";
    let result = run(&mut driver, script, LIMITS).unwrap();
    assert_eq!(result.columns[0].name, "n");
    assert_eq!(result.rows, [[Value::Int(2)]]);
}

#[test]
fn a_last_statement_without_rows_gives_no_columns() {
    let mut driver = database().unwrap();
    let result = run(&mut driver, "SELECT n FROM t; DELETE FROM t", LIMITS).unwrap();
    assert!(result.columns.is_empty());
    assert!(result.rows.is_empty());
    assert!(
        run(&mut driver, "  -- nothing\n", LIMITS)
            .unwrap()
            .columns
            .is_empty()
    );
}

#[test]
fn a_failing_statement_stops_the_run_with_its_error() {
    let mut driver = database().unwrap();
    let error = run(&mut driver, "SELECT nope FROM t; SELECT 1", LIMITS).unwrap_err();
    assert_eq!(
        error,
        RunError::Sql(EngineError::Sql("no such column: nope".to_owned()))
    );
}

#[test]
fn exactly_the_cap_is_allowed_and_one_more_is_too_many() {
    let mut driver = database().unwrap();
    let limits = Limits {
        rows: 600,
        time: LIMITS.time,
    };
    let result = run(&mut driver, &numbers(600), limits).unwrap();
    assert_eq!(result.rows.len(), 600);
    let error = run(&mut driver, &numbers(601), limits).unwrap_err();
    assert_eq!(error, RunError::TooManyRows);
}

#[test]
fn a_run_past_its_time_is_stopped() {
    let mut driver = database().unwrap();
    let limits = Limits {
        rows: LIMITS.rows,
        time: Duration::from_millis(50),
    };
    assert_eq!(run(&mut driver, ENDLESS, limits), Err(RunError::TimedOut));
}

#[test]
fn a_finished_run_is_not_cancelled_afterwards() {
    let mut driver = database().unwrap();
    let limits = Limits {
        rows: LIMITS.rows,
        time: Duration::from_millis(20),
    };
    run(&mut driver, "SELECT 1", limits).unwrap();
    std::thread::sleep(Duration::from_millis(60));
    assert!(!quarry_engine::driver::Driver::canceller(&driver).is_cancelled());
}
