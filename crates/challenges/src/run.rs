//! Running a script on a sandbox copy for grading, with a row cap and a
//! time limit.

use std::{
    sync::{
        Arc, Mutex, PoisonError,
        mpsc::{self, RecvTimeoutError},
    },
    thread,
    time::Duration,
};

use quarry_engine::{
    driver::{Canceller, Driver},
    error::EngineError,
    result::{Column, QueryResult, Row, RowSink},
    splitter,
};

/// How much one graded run may return and how long it may take.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Limits {
    /// The most rows a statement may return.
    pub rows: usize,
    /// The longest a whole run may take.
    pub time: Duration,
}

/// The limits for grading (parent spec §5.2): 10,000 rows and 5 seconds.
pub const LIMITS: Limits = Limits {
    rows: 10_000,
    time: Duration::from_secs(5),
};

/// Why a graded run gave no result.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunError {
    /// The database rejected a statement.
    Sql(EngineError),
    /// The run took longer than its time limit.
    TimedOut,
    /// A statement returned more rows than the cap.
    TooManyRows,
}

/// Stops graded runs from outside, such as a check that a newer one
/// replaced. Clones share one stop.
#[derive(Debug, Clone, Default)]
pub struct Stop(Arc<Mutex<Stopping>>);

#[derive(Debug, Default)]
struct Stopping {
    stopped: bool,
    runs: Vec<Canceller>,
}

impl Stop {
    /// Cancels every run it watches, and every run it watches later.
    pub fn stop(&self) {
        let mut stopping = self.lock();
        stopping.stopped = true;
        stopping.runs.iter().for_each(Canceller::cancel);
    }

    /// Whether [`Stop::stop`] was called.
    pub fn is_stopped(&self) -> bool {
        self.lock().stopped
    }

    /// Cancels `canceller` when the stop is called, or now if it was.
    fn watch(&self, canceller: Canceller) {
        let mut stopping = self.lock();
        if stopping.stopped {
            canceller.cancel();
        }
        stopping.runs.push(canceller);
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, Stopping> {
        // Every update leaves the state whole, so a panic elsewhere while
        // the lock was held leaves nothing half-done.
        self.0.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// Runs scripts for grading within its limits, until its stop is called.
#[derive(Debug, Clone)]
pub struct Runner {
    limits: Limits,
    stop: Stop,
}

impl Runner {
    /// A runner within `limits` that nothing stops.
    pub fn new(limits: Limits) -> Self {
        Self::stoppable(limits, Stop::default())
    }

    /// A runner within `limits` that `stop` can stop.
    pub fn stoppable(limits: Limits, stop: Stop) -> Self {
        Self { limits, stop }
    }

    /// [`run`]s `script` on `driver`. A run cut short by the stop fails as
    /// [`RunError::TimedOut`]; whoever stopped it has stopped listening.
    ///
    /// # Errors
    /// As for [`run`].
    pub fn run(&self, driver: &mut dyn Driver, script: &str) -> Result<QueryResult, RunError> {
        self.stop.watch(driver.canceller());
        run(driver, script, self.limits)
    }
}

/// Runs every statement of `script` on `driver` within `limits`, and
/// returns the last statement's result. A last statement that returns no
/// rows has no columns.
///
/// # Errors
/// Returns a [`RunError`] for the first statement that fails, the time
/// limit or the row cap.
pub fn run(driver: &mut dyn Driver, script: &str, limits: Limits) -> Result<QueryResult, RunError> {
    let _watchdog = Watchdog::start(driver.canceller(), limits.time);
    let mut last = QueryResult::default();
    for range in splitter::split(script) {
        let mut sink = Capped::new(limits.rows, driver.canceller());
        let outcome = driver.execute(&script[range], &mut sink);
        let failure = if sink.over {
            Some(RunError::TooManyRows)
        } else {
            outcome.err().map(stopped)
        };
        if let Some(failure) = failure {
            return Err(failure);
        }
        last = sink.result;
    }
    Ok(last)
}

/// What a failed statement within the row cap means: the clock cancelled
/// it, or the database refused it.
fn stopped(error: EngineError) -> RunError {
    match error {
        EngineError::Cancelled => RunError::TimedOut,
        other => RunError::Sql(other),
    }
}

/// Keeps a statement's rows up to a cap, and cancels the statement when
/// it returns more. Rows past the cap may arrive in the statement's last
/// batch, after it has finished, so `over` decides the outcome rather
/// than the cancellation.
struct Capped {
    result: QueryResult,
    cap: usize,
    over: bool,
    canceller: Canceller,
}

impl Capped {
    fn new(cap: usize, canceller: Canceller) -> Self {
        Self {
            result: QueryResult::default(),
            cap,
            over: false,
            canceller,
        }
    }
}

impl RowSink for Capped {
    fn columns(&mut self, columns: Vec<Column>) {
        self.result.columns = columns;
    }

    fn rows(&mut self, batch: Vec<Row>) {
        if self.result.rows.len() + batch.len() > self.cap {
            self.over = true;
            self.canceller.cancel();
            return;
        }
        self.result.rows.extend(batch);
    }
}

/// Cancels a run that is still going when its time is up. Dropping it
/// ends the watch: its thread wakes and leaves without cancelling.
struct Watchdog {
    _done: mpsc::Sender<()>,
}

impl Watchdog {
    fn start(canceller: Canceller, time: Duration) -> Self {
        let (done, finished) = mpsc::channel::<()>();
        thread::spawn(move || {
            if finished.recv_timeout(time) == Err(RecvTimeoutError::Timeout) {
                canceller.cancel();
            }
        });
        Self { _done: done }
    }
}

#[cfg(test)]
mod tests;
