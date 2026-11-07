//! Integration tests for the session thread: events, errors, cancellation.

use quarry_engine::{
    error::EngineError,
    session::{RunEvent, RunHandle, Session},
    sqlite::SqliteDriver,
    value::Value,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const ENDLESS: &str =
    "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";

fn session() -> Result<Session, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script("CREATE TABLE t (x INTEGER); INSERT INTO t VALUES (1), (2);")?;
    Ok(Session::start(Box::new(driver)))
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
    let busy = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 300000) SELECT count(*) FROM n";
    let first = session.run(busy);
    let second = session.run("SELECT 1");
    assert!(matches!(
        first.events.recv_blocking()?,
        RunEvent::Started { .. }
    ));
    second.cancel();
    assert_eq!(
        kinds(&drain(&first)?),
        ["columns", "rows", "finished", "done"]
    );
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
