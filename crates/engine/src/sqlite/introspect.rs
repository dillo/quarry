//! Reading tables, views and columns from the `SQLite` catalog.

use rusqlite::Connection;

use super::convert;
use crate::{
    error::EngineError,
    schema::{ColumnInfo, Schema, Table, TableKind},
};

const TABLES: &str = "SELECT name, type FROM sqlite_schema \
    WHERE type IN ('table', 'view') AND substr(name, 1, 7) <> 'sqlite_' ORDER BY name";

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
        .collect();
    Ok(Schema { tables })
}

/// A table or view whose columns cannot be read (e.g. a view over a dropped
/// table) is still listed, with no columns.
fn table(connection: &Connection, name: String, kind: &str) -> Table {
    let columns = columns(connection, &name).unwrap_or_default();
    let kind = if kind == "view" {
        TableKind::View
    } else {
        TableKind::Table
    };
    Table {
        name,
        kind,
        columns,
    }
}

fn columns(connection: &Connection, table: &str) -> rusqlite::Result<Vec<ColumnInfo>> {
    let mut statement = connection.prepare(COLUMNS)?;
    let columns = statement.query_map([table], |row| {
        Ok(ColumnInfo {
            name: row.get(0)?,
            type_name: row.get(1)?,
            primary_key: row.get::<_, i64>(2)? > 0,
        })
    })?;
    columns.collect()
}
