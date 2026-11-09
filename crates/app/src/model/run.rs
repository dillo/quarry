//! The state of the latest run, built from the engine's [`RunEvent`]s.

use std::time::{Duration, Instant};

use quarry_engine::{
    error::EngineError,
    result::{Column, Row, Summary},
    session::RunEvent,
};

/// Where a run is in its life.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Nothing has run yet.
    Idle,
    /// Statements are running; rows may still arrive.
    Running,
    /// Every statement finished; the summary is the last statement's.
    Finished(Summary),
    /// A statement failed or the run was cancelled.
    Failed(EngineError),
}

/// The latest run: its phase and the result of its most recent statement.
///
/// The grid shows one result. When a run has several statements, each
/// `Started` replaces the previous statement's columns and rows.
#[derive(Debug, Clone)]
pub struct RunModel {
    phase: Phase,
    columns: Vec<Column>,
    rows: Vec<Row>,
    started_at: Option<Instant>,
    statements: usize,
}

impl Default for RunModel {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            columns: Vec::new(),
            rows: Vec::new(),
            started_at: None,
            statements: 0,
        }
    }
}

impl RunModel {
    /// A run that began at `now`, with no results yet.
    pub fn started(now: Instant) -> Self {
        Self {
            phase: Phase::Running,
            started_at: Some(now),
            ..Self::default()
        }
    }

    /// Folds one event into the model.
    pub fn apply(&mut self, event: RunEvent) {
        match event {
            RunEvent::Started { .. } => self.begin_statement(),
            RunEvent::Columns(columns) => self.columns = columns,
            RunEvent::Rows(batch) => self.rows.extend(batch),
            RunEvent::Finished(summary) => self.phase = Phase::Finished(summary),
            RunEvent::Failed(error) => self.phase = Phase::Failed(error),
            RunEvent::Done => self.finish(),
        }
    }

    /// Where the run is.
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// The current statement's columns (empty for statements without rows).
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The current statement's rows received so far.
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Statements started so far in this run.
    pub fn statements(&self) -> usize {
        self.statements
    }

    /// Whether rows may still arrive.
    pub fn is_running(&self) -> bool {
        self.phase == Phase::Running
    }

    /// Time since the run began, for the live timer; zero before any run.
    pub fn elapsed(&self, now: Instant) -> Duration {
        self.started_at
            .map_or(Duration::ZERO, |start| now.saturating_duration_since(start))
    }

    fn begin_statement(&mut self) {
        self.phase = Phase::Running;
        self.columns.clear();
        self.rows.clear();
        self.statements += 1;
    }

    /// `Done` without `Finished` or `Failed` means the worker vanished.
    fn finish(&mut self) {
        if self.phase == Phase::Running {
            self.phase = Phase::Failed(EngineError::WorkerGone);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use quarry_engine::{
        error::EngineError,
        result::{Column, Summary},
        session::RunEvent,
        value::Value,
    };

    use super::{Phase, RunModel};

    fn column(name: &str) -> Column {
        Column {
            name: name.to_owned(),
            type_name: String::new(),
        }
    }

    fn summary(rows: u64) -> Summary {
        Summary {
            rows,
            rows_affected: None,
            elapsed: Duration::from_millis(2),
        }
    }

    fn started(index: usize) -> RunEvent {
        RunEvent::Started { index, range: 0..1 }
    }

    #[test]
    fn a_new_model_is_idle_and_empty() {
        let model = RunModel::default();
        assert_eq!(model.phase(), &Phase::Idle);
        assert!(model.columns().is_empty() && model.rows().is_empty());
        assert!(!model.is_running());
        assert_eq!(model.statements(), 0);
    }

    #[test]
    fn rows_stream_in_and_the_summary_finishes_the_run() {
        let mut model = RunModel::started(Instant::now());
        assert!(model.is_running());
        model.apply(started(0));
        model.apply(RunEvent::Columns(vec![column("n")]));
        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]]));
        model.apply(RunEvent::Rows(vec![vec![Value::Int(2)]]));
        assert_eq!(model.rows().len(), 2);
        assert!(model.is_running());
        model.apply(RunEvent::Finished(summary(2)));
        model.apply(RunEvent::Done);
        assert_eq!(model.phase(), &Phase::Finished(summary(2)));
        assert_eq!(model.columns(), [column("n")]);
        assert_eq!(model.statements(), 1);
    }

    #[test]
    fn each_statement_replaces_the_previous_result() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(RunEvent::Columns(vec![column("a")]));
        model.apply(RunEvent::Rows(vec![vec![Value::Int(1)]]));
        model.apply(RunEvent::Finished(summary(1)));
        model.apply(started(1));
        assert!(model.is_running());
        assert!(model.columns().is_empty() && model.rows().is_empty());
        assert_eq!(model.statements(), 2);
    }

    #[test]
    fn a_failure_keeps_its_error() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(RunEvent::Failed(EngineError::Cancelled));
        model.apply(RunEvent::Done);
        assert_eq!(model.phase(), &Phase::Failed(EngineError::Cancelled));
        assert!(!model.is_running());
    }

    #[test]
    fn done_while_still_running_means_the_worker_is_gone() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(RunEvent::Done);
        assert_eq!(model.phase(), &Phase::Failed(EngineError::WorkerGone));
    }

    #[test]
    fn elapsed_counts_from_the_start_and_is_zero_when_idle() {
        let start = Instant::now();
        let model = RunModel::started(start);
        let later = start + Duration::from_millis(1500);
        assert_eq!(model.elapsed(later), Duration::from_millis(1500));
        assert_eq!(RunModel::default().elapsed(later), Duration::ZERO);
    }
}
