//! Turns result rows into text to copy: the exporter extension point.
//!
//! Each format is an [`Exporter`] in its own module; adding one means a new
//! module and one line in [`ExporterRegistry::standard`].

mod csv;
mod json;
mod tsv;

use std::fmt::Write;

use crate::{
    result::{Column, Row},
    value::Value,
};

/// One text format for rows and single values.
pub trait Exporter: Send + Sync {
    /// A stable name, such as "csv".
    fn id(&self) -> &'static str;

    /// The name people see, such as "CSV".
    fn title(&self) -> &'static str;

    /// `rows` under a header of `columns`, in this format.
    fn rows(&self, columns: &[Column], rows: &[Row]) -> String;

    /// One cell on its own, in this format.
    fn value(&self, value: &Value) -> String;
}

/// The formats Quarry copies to, in menu order.
pub struct ExporterRegistry {
    exporters: Vec<Box<dyn Exporter>>,
}

impl ExporterRegistry {
    /// TSV, CSV and JSON, in that order.
    pub fn standard() -> Self {
        Self {
            exporters: vec![Box::new(tsv::Tsv), Box::new(csv::Csv), Box::new(json::Json)],
        }
    }

    /// The registered ids, in order.
    pub fn ids(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.exporters.iter().map(|exporter| exporter.id())
    }

    /// The exporter named `id`.
    pub fn get(&self, id: &str) -> Option<&dyn Exporter> {
        self.exporters
            .iter()
            .find(|exporter| exporter.id() == id)
            .map(AsRef::as_ref)
    }
}

/// A value's full text, as the delimited formats write it: `NULL` is
/// empty, numbers are the engine's text, and bytes are `\x` and hex.
fn plain(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Bytes(bytes) => hex(bytes),
        other => other.to_string(),
    }
}

/// `\x` followed by the bytes in lowercase hex, as in `\x00ff`.
fn hex(bytes: &[u8]) -> String {
    bytes.iter().fold(String::from("\\x"), |mut text, byte| {
        let _ = write!(text, "{byte:02x}");
        text
    })
}

/// `text` in double quotes, with each quote inside doubled.
fn quoted(text: &str) -> String {
    format!("\"{}\"", text.replace('"', "\"\""))
}

/// One line per row under a header line, each field made by `field`,
/// separated by `separator` and ended by `end`.
fn delimited(
    columns: &[Column],
    rows: &[Row],
    field: impl Fn(&str) -> String,
    separator: &str,
    end: &str,
) -> String {
    let line = |fields: Vec<String>| fields.join(separator) + end;
    let header = line(columns.iter().map(|column| field(&column.name)).collect());
    let body = rows
        .iter()
        .map(|row| line(row.iter().map(|value| field(&plain(value))).collect()));
    std::iter::once(header).chain(body).collect()
}

#[cfg(test)]
mod tests {
    use super::{ExporterRegistry, hex, plain, quoted};
    use crate::value::Value;

    #[test]
    fn the_standard_formats_are_tsv_csv_and_json() {
        let registry = ExporterRegistry::standard();
        assert_eq!(registry.ids().collect::<Vec<_>>(), ["tsv", "csv", "json"]);
        let titles: Vec<_> = registry
            .ids()
            .filter_map(|id| registry.get(id))
            .map(super::Exporter::title)
            .collect();
        assert_eq!(titles, ["TSV", "CSV", "JSON"]);
        assert!(registry.get("xml").is_none());
    }

    #[test]
    fn plain_text_is_the_full_value() {
        assert_eq!(plain(&Value::Null), "");
        assert_eq!(plain(&Value::Int(-3)), "-3");
        assert_eq!(plain(&Value::Float(2.5)), "2.5");
        assert_eq!(plain(&Value::Text("x".repeat(300))), "x".repeat(300));
        assert_eq!(plain(&Value::Bytes(vec![0, 171, 255])), "\\x00abff");
    }

    #[test]
    fn hex_and_quotes() {
        assert_eq!(hex(&[]), "\\x");
        assert_eq!(quoted("say \"hi\""), "\"say \"\"hi\"\"\"");
    }
}
