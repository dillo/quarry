//! Reading tables, views and columns from the `SQLite` catalog.

use rusqlite::Connection;

use super::convert;
use crate::{
    error::EngineError,
    schema::{ColumnInfo, Schema, Table, TableKind},
};

const TABLES: &str = "SELECT name, type FROM sqlite_schema \
    WHERE type IN ('table', 'view') AND name NOT LIKE 'sqlite_%' ORDER BY name";

const COLUMNS: &str = "SELECT name, type, pk FROM pragma_table_info(?1) ORDER BY cid";

/// Reads every user table and view, sorted by name.
pub(super) fn schema(connection: &Connection) -> Result<Schema, EngineError> {
    let mut statement = connection.prepare(TABLES).map_err(convert::error)?;
    let entries = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(convert::error)?;
    let tables = entries
        .into_iter()
        .map(|(name, kind)| table(connection, name, &kind))
        .collect::<Result<_, _>>()?;
    Ok(Schema { tables })
}

fn table(connection: &Connection, name: String, kind: &str) -> Result<Table, EngineError> {
    let mut statement = connection.prepare(COLUMNS).map_err(convert::error)?;
    let columns = statement
        .query_map([&name], |row| {
            Ok(ColumnInfo {
                name: row.get(0)?,
                type_name: row.get(1)?,
                primary_key: row.get::<_, i64>(2)? > 0,
            })
        })
        .and_then(Iterator::collect::<rusqlite::Result<Vec<_>>>)
        .map_err(convert::error)?;
    let kind = if kind == "view" {
        TableKind::View
    } else {
        TableKind::Table
    };
    Ok(Table {
        name,
        kind,
        columns,
    })
}
