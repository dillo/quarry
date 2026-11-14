//! Comma-separated values, as RFC 4180 describes them.

use super::{Exporter, delimited, plain, quoted};
use crate::{
    result::{Column, Row},
    value::Value,
};

/// Comma-separated fields, lines ending in `\r\n`. A field holding a comma,
/// a line break or a quote is quoted.
pub struct Csv;

/// `text`, quoted when a comma, line break or quote would break the record.
fn field(text: &str) -> String {
    if text.contains([',', '\n', '\r', '"']) {
        quoted(text)
    } else {
        text.to_owned()
    }
}

impl Exporter for Csv {
    fn id(&self) -> &'static str {
        "csv"
    }

    fn title(&self) -> &'static str {
        "CSV"
    }

    fn rows(&self, columns: &[Column], rows: &[Row]) -> String {
        delimited(columns, rows, field, ",", "\r\n")
    }

    fn value(&self, value: &Value) -> String {
        field(&plain(value))
    }
}

#[cfg(test)]
mod tests {
    use super::Csv;
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
    fn writes_a_header_and_one_record_per_row() {
        let rows = vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned()), Value::Null],
            vec![
                Value::Float(1e300),
                Value::Bytes(vec![255]),
                Value::Float(f64::INFINITY),
            ],
        ];
        assert_eq!(
            Csv.rows(&columns(&["id", "name", "note"]), &rows),
            "id,name,note\r\n1,Ada,\r\n1e300,\\xff,inf\r\n"
        );
    }

    #[test]
    fn quotes_fields_that_would_break_the_record() {
        let rows = vec![vec![
            Value::Text("a,b".to_owned()),
            Value::Text("c\nd".to_owned()),
            Value::Text("e\rf".to_owned()),
            Value::Text("say \"hi\"".to_owned()),
            Value::Text("tab\there".to_owned()),
        ]];
        assert_eq!(
            Csv.rows(&columns(&["x,y", "b", "c", "d", "e"]), &rows),
            "\"x,y\",b,c,d,e\r\n\"a,b\",\"c\nd\",\"e\rf\",\"say \"\"hi\"\"\",tab\there\r\n"
        );
    }

    #[test]
    fn no_rows_is_the_header_alone() {
        assert_eq!(Csv.rows(&columns(&["a", "b"]), &[]), "a,b\r\n");
    }

    #[test]
    fn a_single_value_is_quoted_only_when_needed() {
        assert_eq!(Csv.value(&Value::Text("a,b".to_owned())), "\"a,b\"");
        assert_eq!(Csv.value(&Value::Text("plain".to_owned())), "plain");
        assert_eq!(Csv.value(&Value::Null), "");
    }
}
