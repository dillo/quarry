//! Reading a statement's plan with `EXPLAIN QUERY PLAN`.

use rusqlite::Connection;

use super::convert;
use crate::{
    error::EngineError,
    plan::{Plan, PlanStep},
};

/// Plans the first statement in `sql`.
pub(super) fn plan(connection: &Connection, sql: &str) -> Result<Plan, EngineError> {
    let mut statement = connection
        .prepare(&format!("EXPLAIN QUERY PLAN {sql}"))
        .map_err(convert::error)?;
    let steps = statement
        .query_map([], |row| {
            Ok(PlanStep {
                id: row.get(0)?,
                parent: row.get(1)?,
                label: row.get(3)?,
            })
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(convert::error)?;
    Ok(Plan::from_steps(&steps))
}
