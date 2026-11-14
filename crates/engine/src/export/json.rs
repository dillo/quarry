//! JSON: an array of objects keyed by column name.

use std::{collections::HashMap, fmt::Write};

use super::{Exporter, hex};
use crate::{
    result::{Column, Row},
    value::Value,
};

/// An array of objects, one per row, keyed by column name in column order
/// and pretty-printed with two-space indents.
pub struct Json;

/// `value` as JSON: numbers stay numbers, a float that is not finite
/// becomes `null`, and bytes become their `\x…` hex as a string.
fn json(value: &Value) -> String {
    match value {
        Value::Int(n) => n.to_string(),
        Value::Float(x) if x.is_finite() => format!("{x:?}"),
        Value::Null | Value::Float(_) => "null".to_owned(),
        Value::Text(text) => string(text),
        Value::Bytes(bytes) => string(&hex(bytes)),
    }
}

/// `text` as a JSON string.
fn string(text: &str) -> String {
    let mut out = String::with_capacity(text.len() + 2);
    out.push('"');
    for c in text.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c < ' ' => {
                let _ = write!(out, "\\u{:04x}", u32::from(c));
            }
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// The object keys: the column names, with a repeated name numbered from
/// its second use, as in `name (2)`.
fn keys(columns: &[Column]) -> Vec<String> {
    let mut seen: HashMap<&str, usize> = HashMap::new();
    columns
        .iter()
        .map(|column| {
            let count = seen.entry(column.name.as_str()).or_insert(0);
            *count += 1;
            match *count {
                1 => column.name.clone(),
                n => format!("{} ({n})", column.name),
            }
        })
        .collect()
}

/// One row's object, indented as an array element.
fn object(keys: &[String], row: &Row) -> String {
    if keys.is_empty() {
        return "  {}".to_owned();
    }
    let fields: Vec<String> = keys
        .iter()
        .zip(row)
        .map(|(key, value)| format!("    {}: {}", string(key), json(value)))
        .collect();
    format!("  {{\n{}\n  }}", fields.join(",\n"))
}

impl Exporter for Json {
    fn id(&self) -> &'static str {
        "json"
    }

    fn title(&self) -> &'static str {
        "JSON"
    }

    fn rows(&self, columns: &[Column], rows: &[Row]) -> String {
        if rows.is_empty() {
            return "[]".to_owned();
        }
        let keys = keys(columns);
        let objects: Vec<String> = rows.iter().map(|row| object(&keys, row)).collect();
        format!("[\n{}\n]", objects.join(",\n"))
    }

    fn value(&self, value: &Value) -> String {
        json(value)
    }
}

#[cfg(test)]
mod tests {
    use super::Json;
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
    fn writes_an_array_of_objects_in_column_order() {
        let rows = vec![
            vec![Value::Int(1), Value::Text("Ada".to_owned()), Value::Null],
            vec![
                Value::Float(2.5),
                Value::Bytes(vec![1, 255]),
                Value::Float(1e300),
            ],
        ];
        assert_eq!(
            Json.rows(&columns(&["z", "a", "m"]), &rows),
            "[\n  {\n    \"z\": 1,\n    \"a\": \"Ada\",\n    \"m\": null\n  },\n  \
             {\n    \"z\": 2.5,\n    \"a\": \"\\\\x01ff\",\n    \"m\": 1e300\n  }\n]"
        );
    }

    #[test]
    fn floats_that_are_not_finite_are_null() {
        for x in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
            assert_eq!(Json.value(&Value::Float(x)), "null");
        }
        assert_eq!(Json.value(&Value::Float(-0.5)), "-0.5");
    }

    #[test]
    fn escapes_strings() {
        let text = "q\" b\\ n\n r\r t\t bell\u{7} é";
        assert_eq!(
            Json.value(&Value::Text(text.to_owned())),
            "\"q\\\" b\\\\ n\\n r\\r t\\t bell\\u0007 é\""
        );
        assert_eq!(Json.value(&Value::Text(" ~".to_owned())), "\" ~\"");
    }

    #[test]
    fn numbers_repeated_column_names() {
        let rows = vec![vec![
            Value::Int(1),
            Value::Int(2),
            Value::Int(3),
            Value::Int(4),
        ]];
        assert_eq!(
            Json.rows(&columns(&["id", "id", "name", "id"]), &rows),
            "[\n  {\n    \"id\": 1,\n    \"id (2)\": 2,\n    \"name\": 3,\n    \"id (3)\": 4\n  }\n]"
        );
    }

    #[test]
    fn no_rows_is_an_empty_array() {
        assert_eq!(Json.rows(&columns(&["a"]), &[]), "[]");
    }

    #[test]
    fn a_row_without_columns_is_an_empty_object() {
        assert_eq!(Json.rows(&[], &[vec![]]), "[\n  {}\n]");
    }

    #[test]
    fn a_single_value_is_the_json_value() {
        assert_eq!(Json.value(&Value::Null), "null");
        assert_eq!(Json.value(&Value::Int(-7)), "-7");
        assert_eq!(Json.value(&Value::Text("x".to_owned())), "\"x\"");
    }
}
