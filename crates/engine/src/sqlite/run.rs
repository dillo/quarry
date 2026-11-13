//! Executing one statement and streaming its rows in batches.

use std::time::Instant;

use rusqlite::{Connection, Statement};

use super::convert;
use crate::{
    error::EngineError,
    result::{Column, Row, RowSink, Summary},
};

/// Rows per batch handed to the sink.
pub const BATCH_SIZE: usize = 500;

/// Runs `sql` and streams its rows into `sink`.
pub(super) fn execute(
    connection: &Connection,
    sql: &str,
    sink: &mut dyn RowSink,
) -> Result<Summary, EngineError> {
    let started = Instant::now();
    let changes_before = connection.total_changes();
    let mut statement = connection.prepare(sql).map_err(convert::error)?;
    let columns = columns_of(&statement);
    if columns.is_empty() {
        statement.raw_execute().map_err(convert::error)?;
        // `changes()` is this statement's own count (triggers excluded) but is
        // stale after DDL, so only trust it when the total moved.
        let affected = if connection.total_changes() == changes_before {
            0
        } else {
            connection.changes()
        };
        return Ok(Summary {
            rows: 0,
            rows_affected: Some(affected),
            elapsed: started.elapsed(),
        });
    }
    let width = columns.len();
    sink.columns(columns);
    let rows = stream_rows(&mut statement, width, sink)?;
    Ok(Summary {
        rows,
        rows_affected: None,
        elapsed: started.elapsed(),
    })
}

fn columns_of(statement: &Statement<'_>) -> Vec<Column> {
    statement
        .columns()
        .map(|column| Column {
            name: column.name().to_owned(),
            type_name: column.decl_type().unwrap_or_default().to_owned(),
        })
        .collect()
}

fn stream_rows(
    statement: &mut Statement<'_>,
    width: usize,
    sink: &mut dyn RowSink,
) -> Result<u64, EngineError> {
    let mut rows = statement.raw_query();
    let mut batch = Vec::with_capacity(BATCH_SIZE);
    let mut total = 0;
    while let Some(row) = rows.next().map_err(convert::error)? {
        batch.push(read_row(row, width)?);
        total += 1;
        if batch.len() == BATCH_SIZE {
            sink.rows(std::mem::replace(
                &mut batch,
                Vec::with_capacity(BATCH_SIZE),
            ));
        }
    }
    if !batch.is_empty() {
        sink.rows(batch);
    }
    Ok(total)
}

fn read_row(row: &rusqlite::Row<'_>, width: usize) -> Result<Row, EngineError> {
    (0..width)
        .map(|index| {
            row.get_ref(index)
                .map(convert::value)
                .map_err(convert::error)
        })
        .collect()
}
