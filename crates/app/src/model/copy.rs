//! What a copy takes from the grid, and the note that says what it took.

use quarry_engine::{
    export::Exporter,
    result::{Column, Row},
    value::Value,
};

use super::format;

/// What is selected in the grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Selection {
    /// Nothing: a copy takes every kept row.
    None,
    /// One row.
    Row(usize),
    /// One column.
    Column(usize),
    /// One cell, as (row, column).
    Cell(usize, usize),
}

/// What a copy puts on the clipboard, before it is formatted.
#[derive(Debug, Clone, PartialEq)]
pub enum Copied {
    /// Rows under a header.
    Rows {
        /// The header.
        columns: Vec<Column>,
        /// The rows copied.
        rows: Vec<Row>,
        /// How many rows the copied rows stand for: more than `rows.len()`
        /// when the statement streamed more rows than Quarry keeps.
        of: u64,
    },
    /// A single value.
    Value(Value),
}

/// What copying `selection` takes from a statement's `columns` and kept
/// `rows`, of the `received` rows it streamed. `None` when there is
/// nothing to copy: no rows, or an index out of range.
pub fn copied(
    columns: &[Column],
    rows: &[Row],
    received: u64,
    selection: Selection,
) -> Option<Copied> {
    match selection {
        Selection::None => whole(columns, rows, received),
        Selection::Row(row) => Some(Copied::Rows {
            columns: columns.to_vec(),
            rows: vec![rows.get(row)?.clone()],
            of: 1,
        }),
        Selection::Column(column) => Some(Copied::Rows {
            columns: vec![columns.get(column)?.clone()],
            rows: column_values(rows, column)?,
            of: received,
        }),
        Selection::Cell(row, column) => Some(Copied::Value(rows.get(row)?.get(column)?.clone())),
    }
}

fn whole(columns: &[Column], rows: &[Row], received: u64) -> Option<Copied> {
    (!rows.is_empty()).then(|| Copied::Rows {
        columns: columns.to_vec(),
        rows: rows.to_vec(),
        of: received,
    })
}

/// Each row's value in `column`, as a one-value row; `None` without rows.
fn column_values(rows: &[Row], column: usize) -> Option<Vec<Row>> {
    let values: Option<Vec<Row>> = rows
        .iter()
        .map(|row| Some(vec![row.get(column)?.clone()]))
        .collect();
    values.filter(|values| !values.is_empty())
}

impl Copied {
    /// The text `exporter` makes of it.
    pub fn text(&self, exporter: &dyn Exporter) -> String {
        match self {
            Self::Rows { columns, rows, .. } => exporter.rows(columns, rows),
            Self::Value(value) => exporter.value(value),
        }
    }
}

/// The status-bar note after a copy as `format`, such as "Copied 12 rows
/// as CSV", or "Nothing to copy".
pub fn note(copied: Option<&Copied>, format: &str) -> String {
    match copied {
        None => "Nothing to copy".to_owned(),
        Some(Copied::Value(_)) => format!("Copied 1 value as {format}"),
        Some(Copied::Rows { rows, of, .. }) => {
            let kept = u64::try_from(rows.len()).unwrap_or(u64::MAX);
            if *of > kept {
                let (kept, of) = (format::count(kept), format::count(*of));
                format!("Copied {kept} of {of} rows as {format}")
            } else {
                format!("Copied {} as {format}", format::rows(kept))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::{
        export::ExporterRegistry,
        result::{Column, Row},
        value::Value,
    };

    use super::{Copied, Selection, copied, note};

    fn columns() -> Vec<Column> {
        ["id", "name"]
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect()
    }

    fn rows() -> Vec<Row> {
        vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned())],
            vec![Value::Int(2), Value::Null],
        ]
    }

    #[test]
    fn nothing_selected_copies_every_kept_row() {
        assert_eq!(
            copied(&columns(), &rows(), 2, Selection::None),
            Some(Copied::Rows {
                columns: columns(),
                rows: rows(),
                of: 2
            })
        );
    }

    #[test]
    fn a_row_copies_the_header_and_that_row() {
        assert_eq!(
            copied(&columns(), &rows(), 9, Selection::Row(1)),
            Some(Copied::Rows {
                columns: columns(),
                rows: vec![rows()[1].clone()],
                of: 1
            })
        );
    }

    #[test]
    fn a_column_copies_its_header_and_values() {
        assert_eq!(
            copied(&columns(), &rows(), 2, Selection::Column(1)),
            Some(Copied::Rows {
                columns: vec![columns()[1].clone()],
                rows: vec![vec![Value::Text("Ada".to_owned())], vec![Value::Null]],
                of: 2
            })
        );
    }

    #[test]
    fn a_cell_copies_its_value_alone() {
        assert_eq!(
            copied(&columns(), &rows(), 2, Selection::Cell(0, 1)),
            Some(Copied::Value(Value::Text("Ada".to_owned())))
        );
    }

    #[test]
    fn nothing_is_copied_without_rows_or_out_of_range() {
        let none = |selection| copied(&columns(), &rows(), 2, selection);
        assert_eq!(copied(&columns(), &[], 0, Selection::None), None);
        assert_eq!(copied(&columns(), &[], 0, Selection::Column(0)), None);
        assert_eq!(none(Selection::Row(2)), None);
        assert_eq!(none(Selection::Column(2)), None);
        assert_eq!(none(Selection::Cell(2, 0)), None);
        assert_eq!(none(Selection::Cell(0, 2)), None);
    }

    #[test]
    fn text_is_what_the_exporter_makes() {
        let registry = ExporterRegistry::standard();
        let tsv = registry.get("tsv").unwrap();
        let rows = copied(&columns(), &rows(), 2, Selection::None).unwrap();
        assert_eq!(rows.text(tsv), "id\tname\n1\tAda\n2\t\n");
        assert_eq!(Copied::Value(Value::Int(7)).text(tsv), "7");
    }

    #[test]
    fn notes_say_what_was_copied() {
        let rows_of = |kept: usize, of: u64| Copied::Rows {
            columns: columns(),
            rows: vec![vec![Value::Null]; kept],
            of,
        };
        assert_eq!(note(None, "TSV"), "Nothing to copy");
        assert_eq!(
            note(Some(&Copied::Value(Value::Null)), "JSON"),
            "Copied 1 value as JSON"
        );
        assert_eq!(note(Some(&rows_of(12, 12)), "CSV"), "Copied 12 rows as CSV");
        assert_eq!(note(Some(&rows_of(1, 1)), "CSV"), "Copied 1 row as CSV");
        assert_eq!(
            note(Some(&rows_of(1000, 2500)), "TSV"),
            "Copied 1,000 of 2,500 rows as TSV"
        );
    }
}
