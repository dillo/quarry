//! The built-in sample database opens, seeds and answers queries.

use quarry::sample;
use quarry_engine::{schema::TableKind, session::RunEvent, value::Value};

fn first_cell(sql: &str) -> Option<Value> {
    let session = sample::open().ok()?;
    let handle = session.run(sql);
    while let Ok(event) = handle.events.recv_blocking() {
        if let RunEvent::Rows(rows) = event {
            return rows.into_iter().next()?.into_iter().next();
        }
    }
    None
}

#[test]
fn the_sample_has_a_large_table_to_stream() {
    assert_eq!(
        first_cell("SELECT count(*) FROM telemetry"),
        Some(Value::Int(200_000))
    );
}

#[test]
fn the_sample_has_nulls_to_show() {
    assert_eq!(
        first_cell("SELECT count(*) FROM astronauts WHERE call_sign IS NULL"),
        Some(Value::Int(5))
    );
}

#[test]
fn the_schema_lists_every_table_and_the_view() {
    let session = sample::open();
    let schema = session
        .as_ref()
        .ok()
        .and_then(|session| session.schema().recv_blocking().ok())
        .and_then(Result::ok)
        .unwrap_or_default();
    let names: Vec<(&str, TableKind)> = schema
        .tables
        .iter()
        .map(|table| (table.name.as_str(), table.kind))
        .collect();
    assert_eq!(
        names,
        [
            ("astronauts", TableKind::Table),
            ("crew_assignments", TableKind::Table),
            ("mission_crews", TableKind::View),
            ("missions", TableKind::Table),
            ("spacecraft", TableKind::Table),
            ("telemetry", TableKind::Table),
        ]
    );
}
