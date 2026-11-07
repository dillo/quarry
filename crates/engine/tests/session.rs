//! Integration tests for the session thread: events, errors, cancellation.

use quarry_engine::{
    driver::{Canceller, Driver},
    error::EngineError,
    result::{RowSink, Summary},
    schema::Schema,
    session::{RunEvent, RunHandle, Session},
    sqlite::SqliteDriver,
    value::Value,
};

use std::{
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ENDLESS: &str =
    "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";

fn session() -> Result<Session, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1), (2);")?;
    Ok(Session::start(Box::new(driver)))
}

/// A driver whose every statement runs until its cancel flag is set, and
/// whose schema read fails while the flag is still set.
#[derive(Default)]
struct FlagDriver {
    canceller: Canceller,
}

impl Driver for FlagDriver {
    fn execute(&mut self, _sql: &str, _sink: &mut dyn RowSink) -> Result<Summary, EngineError> {
        while !self.canceller.is_cancelled() {
            std::thread::yield_now();
        }
        Err(EngineError::Cancelled)
    }

    fn schema(&mut self) -> Result<Schema, EngineError> {
        if self.canceller.is_cancelled() {
            return Err(EngineError::Cancelled);
        }
        Ok(Schema::default())
    }

    fn canceller(&self) -> Canceller {
        self.canceller.clone()
    }
}

/// A driver whose statements succeed at once and whose schema read waits
/// for `release`, failing if its cancel flag was set by then.
#[derive(Default)]
struct GateDriver {
    canceller: Canceller,
    in_schema: Arc<AtomicBool>,
    release: Arc<AtomicBool>,
}

impl Driver for GateDriver {
    fn execute(&mut self, _sql: &str, _sink: &mut dyn RowSink) -> Result<Summary, EngineError> {
        Ok(Summary {
            rows: 0,
            rows_affected: Some(0),
            elapsed: Duration::ZERO,
        })
    }

    fn schema(&mut self) -> Result<Schema, EngineError> {
        self.in_schema.store(true, Ordering::SeqCst);
        while !self.release.load(Ordering::SeqCst) {
            std::thread::yield_now();
        }
        if self.canceller.is_cancelled() {
            return Err(EngineError::Cancelled);
        }
        Ok(Schema::default())
    }

    fn canceller(&self) -> Canceller {
        self.canceller.clone()
    }
}

/// Reads events until `Done`, which is included.
fn drain(handle: &RunHandle) -> Result<Vec<RunEvent>, async_channel::RecvError> {
    let mut events = Vec::new();
    loop {
        let event = handle.events.recv_blocking()?;
        let done = event == RunEvent::Done;
        events.push(event);
        if done {
            return Ok(events);
        }
    }
}

fn kinds(events: &[RunEvent]) -> Vec<&'static str> {
    events
        .iter()
        .map(|event| match event {
            RunEvent::Started { .. } => "started",
            RunEvent::Columns(_) => "columns",
            RunEvent::Rows(_) => "rows",
            RunEvent::Finished(_) => "finished",
            RunEvent::Failed(_) => "failed",
            RunEvent::Done => "done",
        })
        .collect()
}

#[test]
fn runs_each_statement_in_order() -> TestResult {
    let session = session()?;
    let events = drain(&session.run("UPDATE t SET x = x * 10; SELECT x FROM t ORDER BY x"))?;
    assert_eq!(
        kinds(&events),
        [
            "started", "finished", "started", "columns", "rows", "finished", "done"
        ]
    );
    assert_eq!(
        events[0],
        RunEvent::Started {
            index: 0,
            range: 0..23
        }
    );
    assert_eq!(
        events[2],
        RunEvent::Started {
            index: 1,
            range: 25..51
        }
    );
    assert_eq!(
        events[4],
        RunEvent::Rows(vec![vec![Value::Int(10)], vec![Value::Int(20)]])
    );
    Ok(())
}

#[test]
fn stops_at_the_first_error() -> TestResult {
    let session = session()?;
    let events = drain(&session.run("SELECT nope FROM t; SELECT 1"))?;
    assert_eq!(kinds(&events), ["started", "failed", "done"]);
    assert_eq!(
        events[1],
        RunEvent::Failed(EngineError::Sql("no such column: nope".to_owned()))
    );
    Ok(())
}

#[test]
fn an_empty_script_is_just_done() -> TestResult {
    assert_eq!(drain(&session()?.run("  -- nothing\n"))?, [RunEvent::Done]);
    Ok(())
}

#[test]
fn cancel_interrupts_the_running_statement_and_skips_the_rest() -> TestResult {
    let session = session()?;
    let handle = session.run(format!("{ENDLESS}; SELECT 1"));
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Started { index: 0, .. }
    ));
    handle.cancel();
    let rest = drain(&handle)?;
    let ending = [RunEvent::Failed(EngineError::Cancelled), RunEvent::Done];
    assert!(rest.ends_with(&ending), "{rest:?}");
    let second_started = rest.iter().any(|e| matches!(e, RunEvent::Started { .. }));
    assert!(!second_started, "the second statement must not start");
    Ok(())
}

#[test]
fn cancelling_a_queued_run_leaves_the_running_one_alone() -> TestResult {
    let session = session()?;
    let many = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 100000) SELECT x FROM n";
    let first = session.run(many);
    let second = session.run("SELECT 1");
    // Nothing reads `first` yet, so the bounded channel holds the worker
    // inside it: `first` is still running when `second` is cancelled.
    second.cancel();
    let first_events = drain(&first)?;
    let ending = kinds(&first_events[first_events.len() - 2..]);
    assert_eq!(ending, ["finished", "done"], "{:?}", first_events.last());
    assert_eq!(
        drain(&second)?,
        [RunEvent::Failed(EngineError::Cancelled), RunEvent::Done]
    );
    Ok(())
}

#[test]
fn dropping_a_handle_stops_its_run_and_frees_the_session() -> TestResult {
    let session = session()?;
    let handle = session.run(ENDLESS);
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Started { .. }
    ));
    // After `Columns`, an aggregate sends nothing until it finishes, so only
    // the handle's `Drop` cancel can stop it.
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Columns(_)
    ));
    drop(handle);
    let next = drain(&session.run("SELECT 42"))?;
    assert_eq!(next[2], RunEvent::Rows(vec![vec![Value::Int(42)]]));
    Ok(())
}

#[test]
fn a_dropped_reader_of_a_large_result_does_not_block_the_session() -> TestResult {
    let session = session()?;
    let many = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 100000) SELECT x FROM n";
    drop(session.run(many));
    assert_eq!(
        kinds(&drain(&session.run("SELECT 1"))?),
        ["started", "columns", "rows", "finished", "done"]
    );
    Ok(())
}

#[test]
fn schema_is_read_on_the_session_thread() -> TestResult {
    let session = session()?;
    let schema = session.schema().recv_blocking()??;
    assert_eq!(schema.tables[0].name, "t");
    Ok(())
}

#[test]
fn schema_after_a_cancelled_run_is_not_cancelled() -> TestResult {
    let session = Session::start(Box::new(FlagDriver::default()));
    let handle = session.run("SELECT 1");
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Started { .. }
    ));
    handle.cancel();
    let rest = drain(&handle)?;
    assert_eq!(
        rest,
        [RunEvent::Failed(EngineError::Cancelled), RunEvent::Done]
    );
    assert_eq!(session.schema().recv_blocking()?, Ok(Schema::default()));
    Ok(())
}

#[test]
fn dropping_the_session_cancels_the_running_query() -> TestResult {
    let session = session()?;
    let handle = session.run(ENDLESS);
    assert!(matches!(
        handle.events.recv_blocking()?,
        RunEvent::Started { .. }
    ));
    drop(session);
    let rest = drain(&handle)?;
    let ending = [RunEvent::Failed(EngineError::Cancelled), RunEvent::Done];
    assert!(rest.ends_with(&ending), "{rest:?}");
    Ok(())
}

#[test]
fn a_schema_read_clears_a_stale_interrupt() -> TestResult {
    let canceller = Canceller::default();
    let driver = FlagDriver {
        canceller: canceller.clone(),
    };
    let session = Session::start(Box::new(driver));
    canceller.cancel();
    assert_eq!(session.schema().recv_blocking()?, Ok(Schema::default()));
    Ok(())
}

#[test]
fn a_finished_runs_late_cancel_does_not_interrupt_a_schema_read() -> TestResult {
    let driver = GateDriver::default();
    let in_schema = Arc::clone(&driver.in_schema);
    let release = Arc::clone(&driver.release);
    let session = Session::start(Box::new(driver));
    let handle = session.run("SELECT 1");
    assert_eq!(kinds(&drain(&handle)?), ["started", "finished", "done"]);
    let reply = session.schema();
    while !in_schema.load(Ordering::SeqCst) {
        std::thread::yield_now();
    }
    drop(handle);
    release.store(true, Ordering::SeqCst);
    assert_eq!(reply.recv_blocking()?, Ok(Schema::default()));
    Ok(())
}
