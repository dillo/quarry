//! The database-independent cell value.

use std::fmt;

/// One cell of a result set, independent of the database that produced it.
///
/// Only the variants a current driver produces exist; the Postgres and MySQL
/// drivers (Phase 3) add their own types here.
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    /// SQL `NULL`.
    Null,
    /// A 64-bit integer.
    Int(i64),
    /// A double-precision float.
    Float(f64),
    /// Text.
    Text(String),
    /// Raw bytes (a BLOB).
    Bytes(Vec<u8>),
}

impl Value {
    /// Whether this is SQL `NULL`.
    pub fn is_null(&self) -> bool {
        matches!(self, Self::Null)
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Null => f.write_str("NULL"),
            Self::Int(n) => write!(f, "{n}"),
            Self::Float(x) => write!(f, "{x}"),
            Self::Text(s) => f.write_str(s),
            Self::Bytes(b) => write!(f, "<{} bytes>", b.len()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Value;

    #[test]
    fn displays_each_variant_for_the_grid() {
        let shown: Vec<String> = [
            Value::Null,
            Value::Int(-42),
            Value::Float(2.5),
            Value::Text("Orbital".to_owned()),
            Value::Bytes(vec![1, 2, 3]),
        ]
        .iter()
        .map(ToString::to_string)
        .collect();
        assert_eq!(shown, ["NULL", "-42", "2.5", "Orbital", "<3 bytes>"]);
    }

    #[test]
    fn only_null_is_null() {
        assert!(Value::Null.is_null());
        assert!(!Value::Int(0).is_null());
        assert!(!Value::Text(String::new()).is_null());
    }
}
