//! One run's output, statement by statement, built from the engine's
//! [`RunEvent`]s.

use std::time::{Duration, Instant};

use quarry_engine::{
    error::EngineError,
    result::{Column, Row, Summary},
    session::RunEvent,
};

/// The most rows of one statement Quarry keeps; the rest are only counted.
pub const ROW_LIMIT: usize = 100_000;

/// The most characters of SQL a statement's label shows.
pub const LABEL_CHARS: usize = 24;

/// Where a run is in its life.
#[derive(Debug, Clone, PartialEq)]
pub enum Phase {
    /// Nothing has run yet.
    Idle,
    /// Submitted, but queued behind another run.
    Waiting,
    /// Statements are running; rows may still arrive.
    Running,
    /// Every statement finished.
    Finished,
    /// A statement failed or the run was cancelled.
    Failed(EngineError),
}

/// What one statement of a run produced.
#[derive(Debug, PartialEq)]
pub struct StatementOutput {
    index: usize,
    label: String,
    columns: Vec<Column>,
    rows: Vec<Row>,
    received: u64,
    outcome: Option<Result<Summary, EngineError>>,
}

impl StatementOutput {
    fn new(index: usize, sql: &str) -> Self {
        Self {
            index,
            label: label(index, sql),
            columns: Vec::new(),
            rows: Vec::new(),
            received: 0,
            outcome: None,
        }
    }

    /// The statement's 0-based position in the script.
    pub fn index(&self) -> usize {
        self.index
    }

    /// Its number and first words, as in "3 · SELECT name FROM…".
    pub fn label(&self) -> &str {
        &self.label
    }

    /// Its columns (empty for a statement that returns no rows).
    pub fn columns(&self) -> &[Column] {
        &self.columns
    }

    /// The rows kept so far: at most [`ROW_LIMIT`].
    pub fn rows(&self) -> &[Row] {
        &self.rows
    }

    /// Every row the statement has streamed, kept or not.
    pub fn received(&self) -> u64 {
        self.received
    }

    /// Whether the statement streamed more rows than Quarry keeps.
    pub fn truncated(&self) -> bool {
        self.received > u64::try_from(self.rows.len()).unwrap_or(u64::MAX)
    }

    /// Whether the statement returns rows, so it gets a result tab.
    pub fn returns_rows(&self) -> bool {
        !self.columns.is_empty()
    }

    /// How it ended, once it has.
    pub fn outcome(&self) -> Option<&Result<Summary, EngineError>> {
        self.outcome.as_ref()
    }

    /// Keeps the batch's rows up to [`ROW_LIMIT`] and counts all of them.
    fn take(&mut self, batch: Vec<Row>) {
        let arrived = u64::try_from(batch.len()).unwrap_or(u64::MAX);
        self.received = self.received.saturating_add(arrived);
        let room = ROW_LIMIT.saturating_sub(self.rows.len());
        self.rows.extend(batch.into_iter().take(room));
    }
}

/// "3 · SELECT name FROM…": the 1-based number and the statement's first
/// [`LABEL_CHARS`] characters, with runs of whitespace made single spaces.
pub fn label(index: usize, sql: &str) -> String {
    let words = sql.split_whitespace().collect::<Vec<_>>().join(" ");
    let mut short: String = words.chars().take(LABEL_CHARS).collect();
    if words.chars().nth(LABEL_CHARS).is_some() {
        short.push('…');
    }
    format!("{} · {short}", index + 1)
}

/// One run: its phase and every statement that has started.
#[derive(Debug, PartialEq)]
pub struct RunOutput {
    script: String,
    phase: Phase,
    statements: Vec<StatementOutput>,
    started_at: Option<Instant>,
    ended_at: Option<Instant>,
}

impl Default for RunOutput {
    fn default() -> Self {
        Self {
            script: String::new(),
            phase: Phase::Idle,
            statements: Vec::new(),
            started_at: None,
            ended_at: None,
        }
    }
}

impl RunOutput {
    /// A run of `script` that is queued and has not started.
    pub fn submitted(script: impl Into<String>) -> Self {
        Self {
            script: script.into(),
            phase: Phase::Waiting,
            ..Self::default()
        }
    }

    /// Folds one event in. `now` starts the clock at the first statement
    /// and stops it at `Done`.
    pub fn apply(&mut self, event: RunEvent, now: Instant) {
        match event {
            RunEvent::Started { index, range } => self.begin(index, &range, now),
            RunEvent::Columns(columns) => self.current(|statement| statement.columns = columns),
            RunEvent::Rows(batch) => self.current(|statement| statement.take(batch)),
            RunEvent::Finished(summary) => {
                self.current(|statement| statement.outcome = Some(Ok(summary)));
            }
            RunEvent::Failed(error) => self.fail(error),
            RunEvent::Done => self.finish(now),
        }
    }

    /// The channel closed without `Done`: the worker vanished.
    /// A failure already recorded is kept; anything else becomes `WorkerGone`.
    pub fn end_closed(&mut self) {
        if !matches!(self.phase, Phase::Failed(_)) {
            self.phase = Phase::Failed(EngineError::WorkerGone);
        }
        self.fail_unfinished();
    }

    /// Where the run is.
    pub fn phase(&self) -> &Phase {
        &self.phase
    }

    /// Every statement that has started, in order.
    pub fn statements(&self) -> &[StatementOutput] {
        &self.statements
    }

    /// The statement at `index`, if it has started.
    pub fn statement(&self, index: usize) -> Option<&StatementOutput> {
        self.statements.get(index)
    }

    /// Whether the run is queued or rows may still arrive.
    pub fn is_running(&self) -> bool {
        matches!(self.phase, Phase::Waiting | Phase::Running)
    }

    /// The whole run's time. While it goes on: from the first statement
    /// starting until `now`. Once it has ended: the larger of the wall time
    /// until `Done` and the sum of the statements' own times, because the
    /// timestamps the view passes are only as precise as its poll, and a
    /// fast run can otherwise read zero. The live reading never exceeds the
    /// final one, so the time never jumps backwards. Zero before any
    /// statement starts, so time spent queued never counts.
    pub fn elapsed(&self, now: Instant) -> Duration {
        let Some(start) = self.started_at else {
            return Duration::ZERO;
        };
        match self.ended_at {
            Some(end) => end.saturating_duration_since(start).max(self.engine_time()),
            None => now.saturating_duration_since(start),
        }
    }

    /// The sum of the times the engine reported for finished statements.
    fn engine_time(&self) -> Duration {
        self.statements
            .iter()
            .filter_map(|statement| statement.outcome().and_then(|o| o.as_ref().ok()))
            .map(|summary| summary.elapsed)
            .sum()
    }

    fn begin(&mut self, index: usize, range: &std::ops::Range<usize>, now: Instant) {
        self.phase = Phase::Running;
        self.started_at.get_or_insert(now);
        let sql = self.script.get(range.clone()).unwrap_or_default();
        self.statements.push(StatementOutput::new(index, sql));
    }

    fn current(&mut self, change: impl FnOnce(&mut StatementOutput)) {
        if let Some(statement) = self.statements.last_mut() {
            change(statement);
        }
    }

    /// The running statement, if any, ends with `error`, and so does the run.
    fn fail(&mut self, error: EngineError) {
        if let Some(statement) = self.statements.last_mut()
            && statement.outcome.is_none()
        {
            statement.outcome = Some(Err(error.clone()));
        }
        self.phase = Phase::Failed(error);
    }

    /// `Done` stops the clock; with a statement still unfinished it means
    /// the worker vanished.
    fn finish(&mut self, now: Instant) {
        self.ended_at = Some(now);
        if matches!(self.phase, Phase::Failed(_)) {
            return;
        }
        let unfinished = self.statements.iter().any(|s| s.outcome.is_none());
        self.phase = if unfinished {
            Phase::Failed(EngineError::WorkerGone)
        } else {
            Phase::Finished
        };
        self.fail_unfinished();
    }

    /// Every statement without an outcome was lost with the worker.
    fn fail_unfinished(&mut self) {
        for statement in &mut self.statements {
            statement
                .outcome
                .get_or_insert(Err(EngineError::WorkerGone));
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

    use super::{LABEL_CHARS, Phase, ROW_LIMIT, RunOutput, label};

    const SCRIPT: &str = "SELECT 1 AS n;\nCREATE TABLE t (x);";

    fn column(name: &str) -> Column {
        Column {
            name: name.to_owned(),
            type_name: String::new(),
        }
    }

    fn summary(millis: u64) -> Summary {
        Summary {
            rows: 1,
            rows_affected: None,
            elapsed: Duration::from_millis(millis),
        }
    }

    fn started(index: usize) -> RunEvent {
        let range = if index == 0 { 0..13 } else { 15..34 };
        RunEvent::Started { index, range }
    }

    fn rows(count: usize) -> RunEvent {
        RunEvent::Rows(vec![vec![Value::Int(1)]; count])
    }

    fn feed(output: &mut RunOutput, events: Vec<RunEvent>) {
        for event in events {
            output.apply(event, Instant::now());
        }
    }

    #[test]
    fn a_new_output_is_idle_and_empty() {
        let output = RunOutput::default();
        assert_eq!(output.phase(), &Phase::Idle);
        assert!(output.statements().is_empty());
        assert!(!output.is_running());
    }

    #[test]
    fn a_submitted_run_waits_until_its_first_statement_starts() {
        let mut output = RunOutput::submitted(SCRIPT);
        assert_eq!(output.phase(), &Phase::Waiting);
        assert!(output.is_running());
        feed(&mut output, vec![started(0)]);
        assert_eq!(output.phase(), &Phase::Running);
    }

    #[test]
    fn each_statement_keeps_its_own_columns_rows_and_outcome() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Columns(vec![column("n")]),
                rows(2),
                RunEvent::Finished(summary(2)),
                started(1),
                RunEvent::Finished(summary(3)),
                RunEvent::Done,
            ],
        );
        assert_eq!(output.phase(), &Phase::Finished);
        let [first, second] = output.statements() else {
            panic!("two statements");
        };
        assert_eq!(first.label(), "1 · SELECT 1 AS n");
        assert_eq!((first.index(), second.index()), (0, 1));
        assert_eq!(first.columns(), [column("n")]);
        assert_eq!(first.rows().len(), 2);
        assert!(first.returns_rows() && !second.returns_rows());
        assert_eq!(second.outcome(), Some(&Ok(summary(3))));
        assert_eq!(output.statement(1), Some(second));
        assert_eq!(output.statement(2), None);
    }

    #[test]
    fn a_statement_keeps_only_the_first_rows_but_counts_them_all() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(&mut output, vec![started(0), rows(ROW_LIMIT - 1)]);
        let statement = |output: &RunOutput| output.statements()[0].truncated();
        assert!(!statement(&output));
        feed(&mut output, vec![rows(1)]);
        assert!(!statement(&output), "exactly the limit is not truncated");
        feed(&mut output, vec![rows(3), rows(2)]);
        let first = &output.statements()[0];
        assert_eq!(first.rows().len(), ROW_LIMIT);
        assert_eq!(first.received(), 100_005);
        assert!(first.truncated());
    }

    #[test]
    fn a_failure_ends_the_running_statement_and_the_run() {
        let mut output = RunOutput::submitted(SCRIPT);
        let error = EngineError::Sql("boom".to_owned());
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Finished(summary(1)),
                started(1),
                RunEvent::Failed(error.clone()),
                RunEvent::Done,
            ],
        );
        assert_eq!(output.phase(), &Phase::Failed(error.clone()));
        assert_eq!(output.statements()[0].outcome(), Some(&Ok(summary(1))));
        assert_eq!(output.statements()[1].outcome(), Some(&Err(error)));
    }

    #[test]
    fn a_queued_run_can_fail_before_any_statement() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![RunEvent::Failed(EngineError::Cancelled), RunEvent::Done],
        );
        assert_eq!(output.phase(), &Phase::Failed(EngineError::Cancelled));
        assert!(output.statements().is_empty());
    }

    #[test]
    fn a_failure_after_a_finished_statement_leaves_its_outcome() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Finished(summary(1)),
                RunEvent::Failed(EngineError::Cancelled),
            ],
        );
        assert_eq!(output.statements()[0].outcome(), Some(&Ok(summary(1))));
        assert_eq!(output.phase(), &Phase::Failed(EngineError::Cancelled));
    }

    #[test]
    fn done_with_a_statement_unfinished_means_the_worker_is_gone() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(&mut output, vec![started(0), RunEvent::Done]);
        assert_eq!(output.phase(), &Phase::Failed(EngineError::WorkerGone));
    }

    #[test]
    fn done_with_no_statements_is_finished() {
        let mut output = RunOutput::submitted("-- nothing");
        feed(&mut output, vec![RunEvent::Done]);
        assert_eq!(output.phase(), &Phase::Finished);
    }

    #[test]
    fn a_channel_closed_after_finishing_means_the_worker_is_gone() {
        let mut output = RunOutput::submitted(SCRIPT);
        feed(
            &mut output,
            vec![started(0), RunEvent::Finished(summary(1))],
        );
        output.end_closed();
        assert_eq!(output.phase(), &Phase::Failed(EngineError::WorkerGone));
    }

    #[test]
    fn a_channel_closed_after_a_failure_keeps_the_real_error() {
        let mut output = RunOutput::submitted(SCRIPT);
        let error = EngineError::Sql("no such table".to_owned());
        feed(
            &mut output,
            vec![started(0), RunEvent::Failed(error.clone())],
        );
        output.end_closed();
        assert_eq!(output.phase(), &Phase::Failed(error));
    }

    #[test]
    fn elapsed_runs_from_the_first_start_until_done() {
        let start = Instant::now();
        let at = |millis| start + Duration::from_millis(millis);
        let mut output = RunOutput::submitted(SCRIPT);
        assert_eq!(output.elapsed(at(900)), Duration::ZERO, "queued time");
        output.apply(started(0), at(1000));
        output.apply(RunEvent::Finished(summary(2)), at(1200));
        output.apply(started(1), at(1300));
        assert_eq!(output.elapsed(at(1500)), Duration::from_millis(500));
        output.apply(RunEvent::Finished(summary(3)), at(1600));
        output.apply(RunEvent::Done, at(1700));
        assert_eq!(output.elapsed(at(9000)), Duration::from_millis(700));
    }

    #[test]
    fn a_vanished_worker_fails_the_statement_it_left_unfinished() {
        let mut done = RunOutput::submitted(SCRIPT);
        feed(&mut done, vec![started(0), RunEvent::Done]);
        let gone = Some(&Err(EngineError::WorkerGone));
        assert_eq!(done.statements()[0].outcome(), gone);
        let mut closed = RunOutput::submitted(SCRIPT);
        feed(&mut closed, vec![started(0)]);
        closed.end_closed();
        assert_eq!(closed.statements()[0].outcome(), gone);
    }

    #[test]
    fn a_closed_channel_keeps_earlier_outcomes() {
        let mut output = RunOutput::submitted(SCRIPT);
        let error = EngineError::Sql("boom".to_owned());
        feed(
            &mut output,
            vec![
                started(0),
                RunEvent::Finished(summary(1)),
                started(1),
                RunEvent::Failed(error.clone()),
            ],
        );
        output.end_closed();
        assert_eq!(output.statements()[0].outcome(), Some(&Ok(summary(1))));
        assert_eq!(output.statements()[1].outcome(), Some(&Err(error)));
    }

    #[test]
    fn a_finished_run_is_never_shorter_than_its_statements_own_time() {
        let now = Instant::now();
        let mut output = RunOutput::submitted(SCRIPT);
        output.apply(started(0), now);
        output.apply(RunEvent::Finished(summary(2)), now);
        output.apply(started(1), now);
        output.apply(RunEvent::Finished(summary(3)), now);
        assert_eq!(output.elapsed(now), Duration::ZERO, "live reading");
        output.apply(RunEvent::Done, now);
        assert_eq!(output.elapsed(now), Duration::from_millis(5));
    }

    #[test]
    fn a_failed_run_stops_the_clock_at_done() {
        let start = Instant::now();
        let mut output = RunOutput::submitted(SCRIPT);
        output.apply(started(0), start);
        output.apply(RunEvent::Failed(EngineError::Cancelled), start);
        output.apply(RunEvent::Done, start + Duration::from_millis(40));
        let later = start + Duration::from_secs(60);
        assert_eq!(output.elapsed(later), Duration::from_millis(40));
    }

    #[test]
    fn labels_collapse_whitespace_and_cut_long_sql() {
        assert_eq!(label(0, "SELECT\n\t  1"), "1 · SELECT 1");
        let exact = "x".repeat(LABEL_CHARS);
        assert_eq!(label(4, &exact), format!("5 · {exact}"));
        let long = "é".repeat(LABEL_CHARS + 1);
        assert_eq!(label(1, &long), format!("2 · {}…", "é".repeat(LABEL_CHARS)));
    }

    #[test]
    fn a_range_outside_the_script_gives_an_empty_label() {
        let mut output = RunOutput::submitted("SELECT 1");
        output.apply(
            RunEvent::Started {
                index: 0,
                range: 3..99,
            },
            Instant::now(),
        );
        assert_eq!(output.statements()[0].label(), "1 · ");
    }
}
