//! Integration tests for the `SQLite` driver.

use quarry_engine::{
    driver::Driver,
    error::EngineError,
    result::{Column, QueryResult},
    schema::{ColumnInfo, TableKind},
    sqlite::SqliteDriver,
    value::Value,
};

type TestResult = Result<(), Box<dyn std::error::Error>>;

const CREW: &str =
    "CREATE TABLE astronauts (id INTEGER PRIMARY KEY, name TEXT NOT NULL, photo BLOB);
    INSERT INTO astronauts (name, photo) VALUES ('Valentina', x'0102'), ('Kenji', NULL);
    CREATE VIEW named AS SELECT name FROM astronauts;";

fn crew() -> Result<SqliteDriver, EngineError> {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script(CREW)?;
    Ok(driver)
}

fn column(name: &str, type_name: &str) -> Column {
    Column {
        name: name.to_owned(),
        type_name: type_name.to_owned(),
    }
}

#[test]
fn select_returns_typed_columns_and_values() -> TestResult {
    let mut result = QueryResult::default();
    let summary = crew()?.execute(
        "SELECT id, name, photo, 1.5 AS ratio FROM astronauts",
        &mut result,
    )?;
    assert_eq!(
        result.columns,
        [
            column("id", "INTEGER"),
            column("name", "TEXT"),
            column("photo", "BLOB"),
            column("ratio", "")
        ]
    );
    let valentina = vec![
        Value::Int(1),
        Value::Text("Valentina".to_owned()),
        Value::Bytes(vec![1, 2]),
        Value::Float(1.5),
    ];
    assert_eq!(result.rows[0], valentina);
    assert_eq!(result.rows[1][2], Value::Null);
    assert_eq!((summary.rows, summary.rows_affected), (2, None));
    Ok(())
}

#[test]
fn statements_without_rows_report_rows_affected() -> TestResult {
    let mut driver = crew()?;
    let mut result = QueryResult::default();
    let update = driver.execute("UPDATE astronauts SET name = upper(name)", &mut result)?;
    assert_eq!((update.rows, update.rows_affected), (0, Some(2)));
    let ddl = driver.execute("CREATE TABLE missions (id INTEGER)", &mut result)?;
    assert_eq!(ddl.rows_affected, Some(0));
    assert!(result.columns.is_empty() && result.rows.is_empty());
    Ok(())
}

#[test]
fn rows_arrive_in_batches_of_500() -> TestResult {
    struct Batches(Vec<usize>);
    impl quarry_engine::result::RowSink for Batches {
        fn columns(&mut self, _: Vec<Column>) {}
        fn rows(&mut self, batch: Vec<Vec<Value>>) {
            self.0.push(batch.len());
        }
    }
    let mut sink = Batches(Vec::new());
    let sql = "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n WHERE x < 1203) SELECT x FROM n";
    let summary = SqliteDriver::open_in_memory()?.execute(sql, &mut sink)?;
    assert_eq!(sink.0, [500, 500, 203]);
    assert_eq!(summary.rows, 1203);
    Ok(())
}

#[test]
fn sql_errors_carry_the_database_message() -> TestResult {
    let error = crew()?.execute("SELECT * FROM missing", &mut QueryResult::default());
    assert_eq!(
        error,
        Err(EngineError::Sql("no such table: missing".to_owned()))
    );
    let script_error = crew()?.execute_script("INSERT INTO nowhere VALUES (1)");
    assert_eq!(
        script_error,
        Err(EngineError::Sql("no such table: nowhere".to_owned()))
    );
    Ok(())
}

#[test]
fn a_cancelled_driver_interrupts_and_can_run_again() -> TestResult {
    let mut driver = crew()?;
    let canceller = driver.canceller();
    canceller.cancel();
    let endless =
        "WITH RECURSIVE n(x) AS (SELECT 1 UNION ALL SELECT x + 1 FROM n) SELECT max(x) FROM n";
    assert_eq!(
        driver.execute(endless, &mut QueryResult::default()),
        Err(EngineError::Cancelled)
    );
    canceller.reset();
    let summary = driver.execute("SELECT 1", &mut QueryResult::default())?;
    assert_eq!(summary.rows, 1);
    Ok(())
}

#[test]
fn schema_lists_tables_and_views_with_columns() -> TestResult {
    let schema = crew()?.schema()?;
    let names: Vec<(&str, TableKind)> = schema
        .tables
        .iter()
        .map(|t| (t.name.as_str(), t.kind))
        .collect();
    assert_eq!(
        names,
        [("astronauts", TableKind::Table), ("named", TableKind::View)]
    );
    let id = ColumnInfo {
        name: "id".to_owned(),
        type_name: "INTEGER".to_owned(),
        primary_key: true,
    };
    let name = ColumnInfo {
        name: "name".to_owned(),
        type_name: "TEXT".to_owned(),
        primary_key: false,
    };
    assert_eq!(schema.tables[0].columns[..2], [id, name]);
    Ok(())
}

#[test]
fn files_persist_and_bad_paths_fail_to_open() -> TestResult {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("quarry.db");
    SqliteDriver::open(&path)?.execute_script("CREATE TABLE t (x); INSERT INTO t VALUES (7);")?;
    let mut result = QueryResult::default();
    SqliteDriver::open(&path)?.execute("SELECT x FROM t", &mut result)?;
    assert_eq!(result.rows, [[Value::Int(7)]]);
    let missing_dir = dir.path().join("no/such/dir/x.db");
    let error = SqliteDriver::open(&missing_dir).err();
    assert!(
        matches!(error, Some(EngineError::Open { ref target, .. }) if target.ends_with("x.db"))
    );
    Ok(())
}

#[test]
fn a_broken_view_does_not_blank_the_schema() -> TestResult {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script("CREATE TABLE a(x); CREATE VIEW v AS SELECT x FROM a; DROP TABLE a;")?;
    let schema = driver.schema()?;
    assert_eq!(schema.tables.len(), 1);
    assert_eq!(schema.tables[0].name, "v");
    assert_eq!(schema.tables[0].kind, TableKind::View);
    assert!(schema.tables[0].columns.is_empty());
    Ok(())
}

#[test]
fn rows_affected_excludes_trigger_changes() -> TestResult {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script(
        "CREATE TABLE t(x); CREATE TABLE log(y);
        CREATE TRIGGER u AFTER UPDATE ON t BEGIN INSERT INTO log VALUES (1); END;
        INSERT INTO t VALUES (1);",
    )?;
    let summary = driver.execute("UPDATE t SET x = 2", &mut QueryResult::default())?;
    assert_eq!(summary.rows_affected, Some(1));
    Ok(())
}

#[test]
fn user_tables_that_resemble_internal_names_are_listed() -> TestResult {
    let mut driver = SqliteDriver::open_in_memory()?;
    driver.execute_script("CREATE TABLE SqliteSettings(x);")?;
    let names: Vec<String> = driver
        .schema()?
        .tables
        .into_iter()
        .map(|t| t.name)
        .collect();
    assert_eq!(names, ["SqliteSettings"]);
    Ok(())
}
