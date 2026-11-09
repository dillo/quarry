//! The run summary the status bar shows.

use std::time::Duration;

use quarry_engine::{error::EngineError, result::Summary};

use super::{
    format,
    run::{Phase, RunModel},
};

/// What the status bar says about the latest run, given the live elapsed
/// time for a run still going.
pub fn run_summary(model: &RunModel, elapsed: Duration) -> String {
    match model.phase() {
        Phase::Idle => "Ready".to_owned(),
        Phase::Running => running(model, elapsed),
        Phase::Finished(summary) => finished(summary),
        Phase::Failed(EngineError::Cancelled) => "Cancelled".to_owned(),
        Phase::Failed(_) => "Error".to_owned(),
    }
}

/// The message above the grid when a run failed: the database's own words,
/// and whether it was only a cancel (shown quietly, not as an error).
pub fn failure(model: &RunModel) -> Option<(String, bool)> {
    match model.phase() {
        Phase::Failed(error) => Some((error.to_string(), *error == EngineError::Cancelled)),
        _ => None,
    }
}

fn running(model: &RunModel, elapsed: Duration) -> String {
    let received = u64::try_from(model.rows().len()).unwrap_or(u64::MAX);
    format!(
        "Running · {} · {}",
        format::duration(elapsed),
        format::rows(received)
    )
}

fn finished(summary: &Summary) -> String {
    let took = format::duration(summary.elapsed);
    match summary.rows_affected {
        Some(0) => format!("Done · {took}"),
        Some(1) => format!("1 row affected · {took}"),
        Some(n) => format!("{} rows affected · {took}", format::count(n)),
        None => format!("{} · {took}", format::rows(summary.rows)),
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use quarry_engine::{error::EngineError, result::Summary, session::RunEvent, value::Value};

    use super::{failure, run_summary};
    use crate::model::run::RunModel;

    const MS: Duration = Duration::from_millis(3);

    fn finished(rows: u64, rows_affected: Option<u64>) -> String {
        let mut model = RunModel::started(Instant::now());
        model.apply(RunEvent::Finished(Summary {
            rows,
            rows_affected,
            elapsed: MS,
        }));
        run_summary(&model, Duration::ZERO)
    }

    #[test]
    fn only_a_failed_run_has_a_failure_message() {
        let mut model = RunModel::started(Instant::now());
        assert_eq!(failure(&model), None);
        model.apply(RunEvent::Failed(EngineError::Sql(
            "no such table: t".to_owned(),
        )));
        assert_eq!(
            failure(&model),
            Some(("no such table: t".to_owned(), false))
        );
        model.apply(RunEvent::Failed(EngineError::Cancelled));
        assert_eq!(failure(&model), Some(("Query cancelled.".to_owned(), true)));
    }

    #[test]
    fn idle_reads_ready() {
        assert_eq!(run_summary(&RunModel::default(), MS), "Ready");
    }

    #[test]
    fn a_running_query_shows_the_live_timer_and_rows_so_far() {
        let mut model = RunModel::started(Instant::now());
        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]; 1000]));
        assert_eq!(
            run_summary(&model, Duration::from_millis(1500)),
            "Running · 1.50 s · 1,000 rows"
        );
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
    fn failures_say_whether_they_were_cancelled() {
        let mut model = RunModel::started(Instant::now());
        model.apply(RunEvent::Failed(EngineError::Cancelled));
        assert_eq!(run_summary(&model, MS), "Cancelled");
        model.apply(RunEvent::Failed(EngineError::Sql("boom".to_owned())));
        assert_eq!(run_summary(&model, MS), "Error");
    }
}
