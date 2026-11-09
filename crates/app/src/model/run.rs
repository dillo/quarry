//! The state of the latest run, built from the engine's [`RunEvent`]s.

use std::time::{Duration, Instant};

use quarry_engine::{
    error::EngineError,
    result::{Column, Row, Summary},
    session::RunEvent,
};

/// The most rows of one statement the grid keeps; the rest are only counted.
pub const ROW_LIMIT: usize = 100_000;

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
    received: u64,
    started_at: Option<Instant>,
    statements: usize,
}

impl Default for RunModel {
    fn default() -> Self {
        Self {
            phase: Phase::Idle,
            columns: Vec::new(),
            rows: Vec::new(),
            received: 0,
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
            RunEvent::Rows(batch) => self.take(batch),
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

    /// The current statement's rows kept so far: at most [`ROW_LIMIT`].
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Every row the current statement has streamed, kept or not.
    pub fn received(&self) -> u64 {
        self.received
    }

    /// Whether the statement streamed more rows than the grid keeps.
    pub fn truncated(&self) -> bool {
        self.received > u64::try_from(self.rows.len()).unwrap_or(u64::MAX)
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
        self.received = 0;
        self.statements += 1;
    }

    /// Keeps the batch's rows up to [`ROW_LIMIT`] and counts all of them.
    fn take(&mut self, batch: Vec<Row>) {
        let arrived = u64::try_from(batch.len()).unwrap_or(u64::MAX);
        self.received = self.received.saturating_add(arrived);
        let room = ROW_LIMIT.saturating_sub(self.rows.len());
        self.rows.extend(batch.into_iter().take(room));
    }

    /// `Done` without `Finished` or `Failed` means the worker vanished.
    fn finish(&mut self) {
        if self.phase == Phase::Running {
            self.phase = Phase::Failed(EngineError::WorkerGone);
        }
    }
}

/// A poll's events, made to end properly: a channel that closed without
/// `Done` means the worker vanished, whatever phase the run had reached.
pub fn end_if_closed(mut events: Vec<RunEvent>, closed: bool) -> Vec<RunEvent> {
    if closed && !events.contains(&RunEvent::Done) {
        events.push(RunEvent::Failed(EngineError::WorkerGone));
        events.push(RunEvent::Done);
    }
    events
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

    use super::{Phase, ROW_LIMIT, RunModel, end_if_closed};

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
        assert_eq!(model.received(), 0);
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

    fn rows(count: usize) -> RunEvent {
        RunEvent::Rows(vec![vec![Value::Int(1)]; count])
    }

    #[test]
    fn the_grid_keeps_only_the_first_rows_but_counts_them_all() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(rows(ROW_LIMIT - 1));
        assert!(!model.truncated());
        assert_eq!(model.received(), 99_999);
        model.apply(rows(1));
        assert!(!model.truncated(), "exactly the limit is not truncated");
        model.apply(rows(3));
        assert_eq!(model.rows().len(), ROW_LIMIT);
        assert_eq!(model.received(), 100_003);
        assert!(model.truncated());
        model.apply(rows(2));
        assert_eq!(model.rows().len(), ROW_LIMIT);
        assert_eq!(model.received(), 100_005);
    }

    #[test]
    fn a_new_statement_resets_the_count() {
        let mut model = RunModel::started(Instant::now());
        model.apply(started(0));
        model.apply(rows(ROW_LIMIT + 1));
        assert!(model.truncated());
        model.apply(started(1));
        assert_eq!(model.received(), 0);
        assert!(!model.truncated());
        model.apply(rows(2));
        assert_eq!(model.received(), 2);
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

    #[test]
    fn a_channel_closed_without_done_means_the_worker_is_gone() {
        let gone = vec![
            RunEvent::Finished(summary(1)),
            RunEvent::Failed(EngineError::WorkerGone),
            RunEvent::Done,
        ];
        assert_eq!(
            end_if_closed(vec![RunEvent::Finished(summary(1))], true),
            gone
        );
        let mut model = RunModel::started(Instant::now());
        for event in end_if_closed(vec![RunEvent::Finished(summary(1))], true) {
            model.apply(event);
        }
        assert_eq!(model.phase(), &Phase::Failed(EngineError::WorkerGone));
    }

    #[test]
    fn open_or_properly_ended_channels_are_left_alone() {
        let open = vec![RunEvent::Finished(summary(1))];
        assert_eq!(end_if_closed(open.clone(), false), open);
        let done = vec![RunEvent::Finished(summary(1)), RunEvent::Done];
        assert_eq!(end_if_closed(done.clone(), true), done);
    }
}
