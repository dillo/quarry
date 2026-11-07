//! Runs a driver on its own thread and streams results back as events.
//!
//! The UI submits scripts with [`Session::run`] and reads [`RunEvent`]s from
//! the returned [`RunHandle`] without ever blocking. Runs execute one at a
//! time, in submission order. Dropping a handle stops its run, and dropping
//! the session stops whichever run is executing.
//!
//! A cancel affects only the run it belongs to; the lock makes check-and-set
//! atomic with run transitions.

use std::{
    ops::Range,
    sync::{
        Arc, Mutex, PoisonError,
        atomic::{AtomicBool, AtomicU64, Ordering},
        mpsc,
    },
    thread,
};

use async_channel::{Receiver, Sender};

use crate::{
    driver::{Canceller, Driver},
    error::EngineError,
    result::{Column, Row, RowSink, Summary},
    schema::Schema,
    splitter,
};

/// Batches buffered per run before the session thread waits for the reader.
const EVENT_BUFFER: usize = 16;

/// Something that happened while running a script.
#[derive(Debug, Clone, PartialEq)]
pub enum RunEvent {
    /// Statement `index` (0-based) started; `range` locates it in the script.
    Started {
        /// Position of the statement in the script.
        index: usize,
        /// Byte range of the statement within the script.
        range: Range<usize>,
    },
    /// The current statement's columns (only for statements returning rows).
    Columns(Vec<Column>),
    /// A batch of the current statement's rows.
    Rows(Vec<Row>),
    /// The current statement finished.
    Finished(Summary),
    /// The current statement failed; later statements do not run.
    Failed(EngineError),
    /// The run is over. Always the last event.
    Done,
}

/// A database connection running on its own thread.
pub struct Session {
    jobs: mpsc::Sender<Job>,
    shared: Arc<Shared>,
    next_id: AtomicU64,
}

/// The receiving end of one run.
pub struct RunHandle {
    /// Events for this run, ending with [`RunEvent::Done`].
    pub events: Receiver<RunEvent>,
    run: Arc<RunState>,
    shared: Arc<Shared>,
}

/// State the session thread shares with every handle.
///
/// A cancel affects only the run it belongs to; the lock makes check-and-set
/// atomic with run transitions. The lock guards only those flag transitions:
/// it is never held while a statement runs or an event is sent.
struct Shared {
    driver_canceller: Canceller,
    /// The executing run's id, or 0 when none is executing.
    current_run: Mutex<u64>,
}

struct RunState {
    id: u64,
    cancelled: AtomicBool,
}

enum Job {
    Run {
        script: String,
        run: Arc<RunState>,
        events: Sender<RunEvent>,
    },
    Schema(Sender<Result<Schema, EngineError>>),
}

impl Session {
    /// Moves `driver` onto a new session thread.
    pub fn start(driver: Box<dyn Driver>) -> Self {
        let shared = Arc::new(Shared {
            driver_canceller: driver.canceller(),
            current_run: Mutex::new(0),
        });
        let (jobs, inbox) = mpsc::channel();
        let worker_shared = Arc::clone(&shared);
        thread::spawn(move || work(driver, &inbox, &worker_shared));
        Self {
            jobs,
            shared,
            next_id: AtomicU64::new(1),
        }
    }

    /// Queues `script`; its statements run in order, stopping at the first error.
    pub fn run(&self, script: impl Into<String>) -> RunHandle {
        let id = self.next_id.fetch_add(1, Ordering::SeqCst);
        let run = Arc::new(RunState {
            id,
            cancelled: AtomicBool::new(false),
        });
        let (events, receiver) = async_channel::bounded(EVENT_BUFFER);
        let job = Job::Run {
            script: script.into(),
            run: Arc::clone(&run),
            events: events.clone(),
        };
        if self.jobs.send(job).is_err() {
            let _ = events.try_send(RunEvent::Failed(EngineError::WorkerGone));
            let _ = events.try_send(RunEvent::Done);
        }
        RunHandle {
            events: receiver,
            run,
            shared: Arc::clone(&self.shared),
        }
    }

    /// Reads the schema on the session thread, after any queued runs.
    pub fn schema(&self) -> Receiver<Result<Schema, EngineError>> {
        let (reply, receiver) = async_channel::bounded(1);
        if let Err(mpsc::SendError(Job::Schema(reply))) = self.jobs.send(Job::Schema(reply)) {
            let _ = reply.try_send(Err(EngineError::WorkerGone));
        }
        receiver
    }
}

impl RunHandle {
    /// Cancels this run: the running statement is interrupted and later
    /// statements are skipped. Has no effect once the run is done.
    ///
    /// After cancelling, keep reading until [`RunEvent::Done`] or drop the
    /// handle. A reader that stops reading without dropping the handle stalls
    /// the session, because the event channel is bounded.
    pub fn cancel(&self) {
        self.shared.cancel_run(&self.run);
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        self.shared.cancel_current();
    }
}

impl Shared {
    fn current(&self) -> std::sync::MutexGuard<'_, u64> {
        self.current_run
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
    }

    /// Marks `run` cancelled, interrupting the driver only if `run` is executing.
    fn cancel_run(&self, run: &RunState) {
        let current = self.current();
        run.cancelled.store(true, Ordering::SeqCst);
        if *current == run.id {
            self.driver_canceller.cancel();
        }
    }

    /// Interrupts whichever run is executing, if any.
    fn cancel_current(&self) {
        let current = self.current();
        if *current != 0 {
            self.driver_canceller.cancel();
        }
    }

    fn begin_run(&self, id: u64) {
        *self.current() = id;
    }

    /// Clears the driver flag for the next statement; returns whether `run`
    /// may go on (false once it has been cancelled).
    fn begin_statement(&self, run: &RunState) -> bool {
        let _current = self.current();
        self.driver_canceller.reset();
        !run.cancelled.load(Ordering::SeqCst)
    }

    fn end_run(&self) {
        let mut current = self.current();
        *current = 0;
        self.driver_canceller.reset();
    }

    /// Clears the driver flag before a schema read, outside of any run.
    fn begin_schema(&self) {
        let _current = self.current();
        self.driver_canceller.reset();
    }
}

impl Drop for RunHandle {
    fn drop(&mut self) {
        self.cancel();
    }
}

fn work(mut driver: Box<dyn Driver>, inbox: &mpsc::Receiver<Job>, shared: &Shared) {
    while let Ok(job) = inbox.recv() {
        match job {
            Job::Run {
                script,
                run,
                events,
            } => {
                let active = ActiveRun {
                    shared,
                    script: &script,
                    state: &run,
                    events: &events,
                };
                active.execute(driver.as_mut());
            }
            Job::Schema(reply) => {
                shared.begin_schema();
                let _ = reply.send_blocking(driver.schema());
            }
        }
    }
}

/// One run being executed on the session thread.
struct ActiveRun<'a> {
    shared: &'a Shared,
    script: &'a str,
    state: &'a RunState,
    events: &'a Sender<RunEvent>,
}

impl ActiveRun<'_> {
    fn execute(&self, driver: &mut dyn Driver) {
        self.shared.begin_run(self.state.id);
        for (index, range) in splitter::split(self.script).into_iter().enumerate() {
            if !self.statement(driver, index, range) {
                break;
            }
        }
        self.shared.end_run();
        let _ = self.events.send_blocking(RunEvent::Done);
    }

    /// Runs one statement; returns whether the script should continue.
    fn statement(&self, driver: &mut dyn Driver, index: usize, range: Range<usize>) -> bool {
        if !self.shared.begin_statement(self.state) {
            let _ = self
                .events
                .send_blocking(RunEvent::Failed(EngineError::Cancelled));
            return false;
        }
        let started = RunEvent::Started {
            index,
            range: range.clone(),
        };
        if self.events.send_blocking(started).is_err() {
            return false;
        }
        let canceller = &self.shared.driver_canceller;
        let mut sink = ChannelSink {
            events: self.events,
            canceller,
            open: true,
        };
        let outcome = driver.execute(&self.script[range], &mut sink);
        let keep_going = outcome.is_ok() && sink.open;
        let event = outcome.map_or_else(RunEvent::Failed, RunEvent::Finished);
        self.events.send_blocking(event).is_ok() && keep_going
    }
}

/// Forwards results to the run's channel; a closed channel cancels the run.
struct ChannelSink<'a> {
    events: &'a Sender<RunEvent>,
    canceller: &'a Canceller,
    open: bool,
}

impl ChannelSink<'_> {
    fn send(&mut self, event: RunEvent) {
        if self.open && self.events.send_blocking(event).is_err() {
            self.open = false;
            self.canceller.cancel();
        }
    }
}

impl RowSink for ChannelSink<'_> {
    fn columns(&mut self, columns: Vec<Column>) {
        self.send(RunEvent::Columns(columns));
    }

    fn rows(&mut self, batch: Vec<Row>) {
        self.send(RunEvent::Rows(batch));
    }
}
