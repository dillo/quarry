//! Result-set shapes shared by every driver.

use std::time::Duration;

use crate::value::Value;

/// A result column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Column {
    /// Column name as the query labels it.
    pub name: String,
    /// Declared type, or empty when the database doesn't know (expressions).
    pub type_name: String,
}

/// One result row, in column order.
pub type Row = Vec<Value>;

/// What one statement did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Summary {
    /// Rows returned (0 for statements that return none).
    pub rows: u64,
    /// Rows changed, for statements that return no rows.
    pub rows_affected: Option<u64>,
    /// Wall-clock time the statement took.
    pub elapsed: Duration,
}

/// Receives a statement's results as they are produced.
pub trait RowSink {
    /// Called once, before any rows, for statements that return rows.
    fn columns(&mut self, columns: Vec<Column>);
    /// Called with each batch of rows, in order.
    fn rows(&mut self, batch: Vec<Row>);
}

/// A sink that keeps everything in memory.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct QueryResult {
    /// The result columns.
    pub columns: Vec<Column>,
    /// Every row received.
    pub rows: Vec<Row>,
}

impl RowSink for QueryResult {
    fn columns(&mut self, columns: Vec<Column>) {
        self.columns = columns;
    }

    fn rows(&mut self, batch: Vec<Row>) {
        self.rows.extend(batch);
    }
}

#[cfg(test)]
mod tests {
    use super::{Column, QueryResult, RowSink};
    use crate::value::Value;

    #[test]
    fn query_result_keeps_columns_and_appends_batches() {
        let mut result = QueryResult::default();
        let column = Column {
            name: "n".to_owned(),
            type_name: "INTEGER".to_owned(),
        };
        result.columns(vec![column.clone()]);
        result.rows(vec![vec![Value::Int(1)]]);
        result.rows(vec![vec![Value::Int(2)], vec![Value::Int(3)]]);
        assert_eq!(result.columns, [column]);
        assert_eq!(
            result.rows,
            [[Value::Int(1)], [Value::Int(2)], [Value::Int(3)]]
        );
    }
}
