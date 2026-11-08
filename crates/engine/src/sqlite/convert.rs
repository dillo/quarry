//! Converting `rusqlite` values and errors into engine types.

use rusqlite::{ErrorCode, types::ValueRef};

use crate::{error::EngineError, value::Value};

/// Converts one `SQLite` cell.
pub(super) fn value(cell: ValueRef<'_>) -> Value {
    match cell {
        ValueRef::Null => Value::Null,
        ValueRef::Integer(n) => Value::Int(n),
        ValueRef::Real(x) => Value::Float(x),
        ValueRef::Text(text) => Value::Text(String::from_utf8_lossy(text).into_owned()),
        ValueRef::Blob(bytes) => Value::Bytes(bytes.to_vec()),
    }
}

/// Converts a `rusqlite` error into a user-facing engine error.
pub(super) fn error(error: rusqlite::Error) -> EngineError {
    match error {
        rusqlite::Error::SqliteFailure(failure, _)
            if failure.code == ErrorCode::OperationInterrupted =>
        {
            EngineError::Cancelled
        }
        rusqlite::Error::SqliteFailure(_, Some(message))
        | rusqlite::Error::SqlInputError { msg: message, .. } => EngineError::Sql(message),
        other => EngineError::Sql(other.to_string()),
    }
}
