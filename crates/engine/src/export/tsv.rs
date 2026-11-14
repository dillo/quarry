//! Tab-separated values: what spreadsheets paste.

use super::{Exporter, delimited, plain, quoted};
use crate::{
    result::{Column, Row},
    value::Value,
};

/// One line per row, fields separated by tabs, lines ending in `\n`.
/// A field holding a tab, a line break or a quote is quoted CSV-style.
pub struct Tsv;

/// `text`, quoted only when a tab, line break or quote would break the line.
fn field(text: &str) -> String {
    if text.contains(['\t', '\n', '\r', '"']) {
        quoted(text)
    } else {
        text.to_owned()
    }
}

impl Exporter for Tsv {
    fn id(&self) -> &'static str {
        "tsv"
    }

    fn title(&self) -> &'static str {
        "TSV"
    }

    fn rows(&self, columns: &[Column], rows: &[Row]) -> String {
        delimited(columns, rows, field, "\t", "\n")
    }

    fn value(&self, value: &Value) -> String {
        plain(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Tsv;
    use crate::{export::Exporter, result::Column, value::Value};

    fn columns(names: &[&str]) -> Vec<Column> {
        names
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect()
    }

    #[test]
    fn writes_a_header_and_one_line_per_row() {
        let rows = vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned()), Value::Null],
            vec![
                Value::Float(2.5),
                Value::Bytes(vec![1, 2]),
                Value::Float(f64::NAN),
            ],
        ];
        assert_eq!(
            Tsv.rows(&columns(&["id", "name", "note"]), &rows),
            "id\tname\tnote\n1\tAda\t\n2.5\t\\x0102\tNaN\n"
        );
    }

    #[test]
    fn quotes_fields_that_would_break_the_line() {
        let rows = vec![vec![
            Value::Text("a\tb".to_owned()),
            Value::Text("c\nd".to_owned()),
            Value::Text("e\rf".to_owned()),
            Value::Text("say \"hi\"".to_owned()),
            Value::Text("x, y".to_owned()),
        ]];
        assert_eq!(
            Tsv.rows(&columns(&["t\tab", "b", "c", "d", "e"]), &rows),
            "\"t\tab\"\tb\tc\td\te\n\"a\tb\"\t\"c\nd\"\t\"e\rf\"\t\"say \"\"hi\"\"\"\tx, y\n"
        );
    }

    #[test]
    fn no_rows_is_the_header_alone() {
        assert_eq!(Tsv.rows(&columns(&["a", "b"]), &[]), "a\tb\n");
    }

    #[test]
    fn a_single_value_is_its_raw_text() {
        assert_eq!(
            Tsv.value(&Value::Text("a\tb \"c\"".to_owned())),
            "a\tb \"c\""
        );
        assert_eq!(Tsv.value(&Value::Null), "");
        assert_eq!(Tsv.value(&Value::Int(7)), "7");
    }
}
