//! Query plans from the `SQLite` driver and through a session.

use quarry_engine::{
    driver::{Canceller, Driver},
    error::EngineError,
    plan::PlanNode,
    result::{RowSink, Summary},
    schema::Schema,
    session::{RunEvent, Session},
    sqlite::SqliteDriver,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const SPACE: &str = "CREATE TABLE crew (id INTEGER PRIMARY KEY, name TEXT, ship INTEGER);
    CREATE TABLE ships (id INTEGER PRIMARY KEY, name TEXT);
    CREATE INDEX crew_name ON crew (name);";

fn driver() -> Result<SqliteDriver, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script(SPACE)?;
    Ok(driver)
}

fn labels(nodes: &[PlanNode]) -> Vec<&str> {
    nodes.iter().map(|node| node.label.as_str()).collect()
}

#[test]
fn an_index_lookup_is_one_search_step() -> TestResult {
    let plan = driver()?.plan("SELECT id FROM crew WHERE name = 'Ada'")?;
    assert_eq!(
        labels(&plan.nodes),
        ["SEARCH crew USING COVERING INDEX crew_name (name=?)"]
    );
    assert!(plan.nodes[0].children.is_empty());
    Ok(())
}

#[test]
fn a_join_has_one_step_per_table() -> TestResult {
    let plan = driver()?
        .plan("SELECT crew.name, ships.name FROM crew JOIN ships ON ships.id = crew.ship")?;
    assert_eq!(
        labels(&plan.nodes),
        [
            "SCAN crew",
            "SEARCH ships USING INTEGER PRIMARY KEY (rowid=?)"
        ]
    );
    Ok(())
}

#[test]
fn a_correlated_subquery_nests_its_steps() -> TestResult {
    let plan = driver()?
        .plan("SELECT name, (SELECT count(*) FROM crew WHERE crew.ship = ships.id) FROM ships")?;
    assert_eq!(
        labels(&plan.nodes),
        ["SCAN ships", "CORRELATED SCALAR SUBQUERY 1"]
    );
    assert_eq!(labels(&plan.nodes[1].children), ["SCAN crew"]);
    Ok(())
}

#[test]
fn sql_that_cannot_be_planned_is_a_sql_error() -> TestResult {
    let error = driver()?.plan("SELECT * FROM nowhere").err();
    assert_eq!(
        error,
        Some(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    Ok(())
}

#[test]
fn a_session_plans_after_queued_runs() -> TestResult {
    let session = Session::start(Box::new(driver()?));
    let run = session.run("CREATE TABLE late (x INTEGER)");
    let plan = session.plan("SELECT x FROM late").recv_blocking()??;
    assert_eq!(labels(&plan.nodes), ["SCAN late"]);
    while run.events.recv_blocking()? != RunEvent::Done {}
    Ok(())
}

#[test]
fn a_session_plans_only_the_first_statement() -> TestResult {
    let session = Session::start(Box::new(driver()?));
    let plan = session
        .plan("SELECT id FROM ships; SELECT * FROM nowhere")
        .recv_blocking()??;
    assert_eq!(labels(&plan.nodes), ["SCAN ships"]);
    Ok(())
}

#[test]
fn a_session_plans_text_without_statements_as_given() -> TestResult {
    let session = Session::start(Box::new(driver()?));
    let error = session.plan("-- nothing").recv_blocking()?.err();
    assert!(matches!(error, Some(EngineError::Sql(_))));
    Ok(())
}

/// A driver that keeps the default `plan`.
struct Plain;

impl Driver for Plain {
    fn execute(&mut self, _sql: &str, _sink: &mut dyn RowSink) -> Result<Summary, EngineError> {
        Err(EngineError::Cancelled)
    }

    fn schema(&mut self) -> Result<Schema, EngineError> {
        Ok(Schema::default())
    }

    fn canceller(&self) -> Canceller {
        Canceller::default()
    }
}

#[test]
fn a_driver_without_plans_says_so() -> TestResult {
    let session = Session::start(Box::new(Plain));
    assert_eq!(
        session.plan("SELECT 1").recv_blocking()?,
        Err(EngineError::PlanUnsupported)
    );
    Ok(())
}
