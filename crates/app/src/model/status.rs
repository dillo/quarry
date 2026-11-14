//! The run summary the status bar shows, and the failure banner's message.

use std::time::Duration;

use quarry_engine::{error::EngineError, result::Summary};

use super::{
    format,
    statements::{Phase, RunOutput, StatementOutput},
};

/// The message above the results when a run failed.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Failure {
    /// The database's own words, or Quarry's for a cancel or a lost worker.
    pub message: String,
    /// Whether it was only a cancel, shown quietly rather than as an error.
    pub cancelled: bool,
}

/// What the status bar says about a run, given its elapsed time (see
/// [`RunOutput::elapsed`]).
pub fn run_summary(output: &RunOutput, elapsed: Duration) -> String {
    match output.phase() {
        Phase::Idle => "Ready".to_owned(),
        Phase::Waiting => "Waiting for the previous run…".to_owned(),
        Phase::Running => running(output, elapsed),
        Phase::Finished => finished(output, elapsed),
        Phase::Failed(error) if is_cancel(error) => "Cancelled".to_owned(),
        Phase::Failed(_) => "Error".to_owned(),
    }
}

/// The failure banner's message, for a failed run.
pub fn failure(output: &RunOutput) -> Option<Failure> {
    match output.phase() {
        Phase::Failed(error) => Some(Failure {
            message: error.to_string(),
            cancelled: is_cancel(error),
        }),
        _ => None,
    }
}

/// How a finished statement reads: "4 rows · 3.0 ms",
/// "2 rows affected · 1.0 ms" or "Done · 1.0 ms".
pub fn outcome_text(statement: &StatementOutput, summary: &Summary) -> String {
    let took = format::duration(summary.elapsed);
    match summary.rows_affected {
        Some(0) => format!("Done · {took}"),
        Some(1) => format!("1 row affected · {took}"),
        Some(n) => format!("{} rows affected · {took}", format::count(n)),
        None => format!("{} · {took}", rows_text(statement, summary.rows)),
    }
}

fn is_cancel(error: &EngineError) -> bool {
    *error == EngineError::Cancelled
}

fn running(output: &RunOutput, elapsed: Duration) -> String {
    let rows = output
        .statements()
        .last()
        .map(|statement| rows_text(statement, statement.received()))
        .unwrap_or_default();
    format!("Running · {} · {rows}", format::duration(elapsed))
}

/// One statement reads as its outcome; several as a count and their total.
fn finished(output: &RunOutput, elapsed: Duration) -> String {
    if let [statement] = output.statements()
        && let Some(Ok(summary)) = statement.outcome()
    {
        return outcome_text(statement, summary);
    }
    let count = u64::try_from(output.statements().len()).unwrap_or(u64::MAX);
    format!(
        "{} statements · {}",
        format::count(count),
        format::duration(elapsed)
    )
}

/// "1,000 rows", or "100,000 of 250,000 rows" when Quarry keeps fewer
/// than the `total` the statement streamed.
fn rows_text(statement: &StatementOutput, total: u64) -> String {
    if statement.truncated() {
        let kept = u64::try_from(statement.rows().len()).unwrap_or(u64::MAX);
        format!("{} of {}", format::count(kept), format::rows(total))
    } else {
        format::rows(total)
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use quarry_engine::{error::EngineError, result::Summary, session::RunEvent, value::Value};

    use super::{Failure, failure, run_summary};
    use crate::model::statements::RunOutput;

    const MS: Duration = Duration::from_millis(3);

    fn started(index: usize) -> RunEvent {
        RunEvent::Started { index, range: 0..1 }
    }

    fn summary(rows: u64, rows_affected: Option<u64>) -> RunEvent {
        RunEvent::Finished(Summary {
            rows,
            rows_affected,
            elapsed: MS,
        })
    }

    fn output(events: Vec<RunEvent>) -> RunOutput {
        let mut output = RunOutput::submitted("x");
        for event in events {
            output.apply(event, Instant::now());
        }
        output
    }

    fn finished(rows: u64, rows_affected: Option<u64>) -> String {
        let output = output(vec![
            started(0),
            summary(rows, rows_affected),
            RunEvent::Done,
        ]);
        run_summary(&output, output.elapsed(Instant::now()))
    }

    #[test]
    fn only_a_failed_run_has_a_failure_message() {
        let mut run = output(vec![started(0)]);
        assert_eq!(failure(&run), None);
        run.apply(
            RunEvent::Failed(EngineError::Sql("no such table: t".to_owned())),
            Instant::now(),
        );
        assert_eq!(
            failure(&run),
            Some(Failure {
                message: "no such table: t".to_owned(),
                cancelled: false
            })
        );
        let cancelled = output(vec![RunEvent::Failed(EngineError::Cancelled)]);
        assert_eq!(
            failure(&cancelled),
            Some(Failure {
                message: "Query cancelled.".to_owned(),
                cancelled: true
            })
        );
    }

    #[test]
    fn idle_reads_ready_and_queued_reads_waiting() {
        assert_eq!(run_summary(&RunOutput::default(), MS), "Ready");
        assert_eq!(
            run_summary(&RunOutput::submitted("x"), MS),
            "Waiting for the previous run…"
        );
    }

    #[test]
    fn a_running_query_shows_the_live_timer_and_rows_so_far() {
        let run = output(vec![
            started(0),
            RunEvent::Rows(vec![vec![Value::Int(1)]; 1000]),
        ]);
        assert_eq!(
            run_summary(&run, Duration::from_millis(1500)),
            "Running · 1.50 s · 1,000 rows"
        );
    }

    #[test]
    fn a_truncated_run_says_how_many_rows_quarry_keeps() {
        let mut run = output(vec![
            started(0),
            RunEvent::Rows(vec![vec![Value::Int(1)]; 250_000]),
        ]);
        assert_eq!(
            run_summary(&run, Duration::from_millis(1500)),
            "Running · 1.50 s · 100,000 of 250,000 rows"
        );
        run.apply(summary(250_000, None), Instant::now());
        run.apply(RunEvent::Done, Instant::now());
        assert_eq!(run_summary(&run, MS), "100,000 of 250,000 rows · 3.0 ms");
    }

    #[test]
    fn a_query_reports_its_rows() {
        assert_eq!(finished(4, None), "4 rows · 3.0 ms");
        assert_eq!(finished(1, None), "1 row · 3.0 ms");
    }

    #[test]
    fn a_change_reports_rows_affected() {
        assert_eq!(finished(0, Some(1)), "1 row affected · 3.0 ms");
        assert_eq!(finished(0, Some(1200)), "1,200 rows affected · 3.0 ms");
    }

    #[test]
    fn a_statement_that_changes_nothing_just_reads_done() {
        assert_eq!(finished(0, Some(0)), "Done · 3.0 ms");
    }

    #[test]
    fn several_statements_read_as_a_count_and_the_run_time() {
        let run = output(vec![
            started(0),
            summary(1, None),
            started(1),
            summary(0, Some(2)),
            RunEvent::Done,
        ]);
        assert_eq!(
            run_summary(&run, Duration::from_millis(6)),
            "2 statements · 6.0 ms"
        );
    }

    #[test]
    fn a_script_with_no_statements_reads_done() {
        let run = output(vec![RunEvent::Done]);
        assert_eq!(run_summary(&run, Duration::ZERO), "0 statements · 0.0 ms");
    }

    #[test]
    fn a_running_run_before_any_rows_reads_zero_rows() {
        let run = output(vec![started(0)]);
        assert_eq!(run_summary(&run, MS), "Running · 3.0 ms · 0 rows");
    }

    #[test]
    fn failures_say_whether_they_were_cancelled() {
        let mut run = output(vec![started(0)]);
        run.apply(RunEvent::Failed(EngineError::Cancelled), Instant::now());
        assert_eq!(run_summary(&run, MS), "Cancelled");
        run.apply(
            RunEvent::Failed(EngineError::Sql("boom".to_owned())),
            Instant::now(),
        );
        assert_eq!(run_summary(&run, MS), "Error");
    }
}
