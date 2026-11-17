//! When two cells count as equal, and one order for sorting rows.
//!
//! `NULL` equals `NULL`, an integer equals the same value as a float, floats
//! are equal within a relative tolerance, and text and bytes must match
//! exactly.

use std::cmp::Ordering;

use quarry_engine::value::Value;

/// How close two floats must be, relative to the larger.
pub const TOLERANCE: f64 = 1e-9;

/// Whether `a` and `b` count as the same cell.
pub fn same(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => x == y,
        _ => match (number(a), number(b)) {
            (Some(x), Some(y)) => close(x, y),
            _ => a == b,
        },
    }
}

/// Whether two rows hold the same cells, column by column.
pub fn same_row(a: &[Value], b: &[Value]) -> bool {
    a.len() == b.len() && a.iter().zip(b).all(|(x, y)| same(x, y))
}

/// One total order over cells: `NULL`, then numbers by value, then text,
/// then bytes.
pub fn compare(a: &Value, b: &Value) -> Ordering {
    match (number(a), number(b)) {
        (Some(x), Some(y)) => x.total_cmp(&y),
        _ => match (a, b) {
            (Value::Text(x), Value::Text(y)) => x.cmp(y),
            (Value::Bytes(x), Value::Bytes(y)) => x.cmp(y),
            _ => rank(a).cmp(&rank(b)),
        },
    }
}

/// Rows compared cell by cell with [`compare`], then by length.
pub fn compare_rows(a: &[Value], b: &[Value]) -> Ordering {
    a.iter()
        .zip(b)
        .map(|(x, y)| compare(x, y))
        .find(|order| order.is_ne())
        .unwrap_or_else(|| a.len().cmp(&b.len()))
}

/// Where a cell's kind sorts.
fn rank(value: &Value) -> u8 {
    match value {
        Value::Null => 0,
        Value::Int(_) | Value::Float(_) => 1,
        Value::Text(_) => 2,
        Value::Bytes(_) => 3,
    }
}

/// A number's value as a float.
fn number(value: &Value) -> Option<f64> {
    match value {
        Value::Int(n) => Some(float(*n)),
        Value::Float(x) => Some(*x),
        _ => None,
    }
}

/// Whether `x` and `y` differ by at most [`TOLERANCE`] of the larger.
/// An infinity is close only to itself.
fn close(x: f64, y: f64) -> bool {
    let within = || (x - y).abs() <= TOLERANCE * x.abs().max(y.abs());
    x.total_cmp(&y).is_eq() || (x.is_finite() && y.is_finite() && within())
}

/// `n` as the nearest float, without a lossy cast: its high and low 32 bits
/// each convert exactly.
fn float(n: i64) -> f64 {
    let high = i32::try_from(n >> 32).unwrap_or(0);
    let low = u32::try_from(n & 0xFFFF_FFFF).unwrap_or(0);
    f64::from(high) * 4_294_967_296.0 + f64::from(low)
}

#[cfg(test)]
mod tests {
    use std::cmp::Ordering;

    use quarry_engine::value::Value;

    use super::{compare, compare_rows, float, same, same_row};

    fn text(s: &str) -> Value {
        Value::Text(s.to_owned())
    }

    #[test]
    fn integers_convert_exactly() {
        let cases = [
            (0, 0.0),
            (7, 7.0),
            (-3, -3.0),
            (5_000_000_123, 5_000_000_123.0),
            (-5_000_000_123, -5_000_000_123.0),
        ];
        for (n, x) in cases {
            assert_eq!(float(n).to_bits(), f64::to_bits(x), "{n}");
        }
    }

    #[test]
    fn equal_infinities_match() {
        assert!(same(
            &Value::Float(f64::INFINITY),
            &Value::Float(f64::INFINITY)
        ));
        assert!(!same(
            &Value::Float(f64::INFINITY),
            &Value::Float(f64::NEG_INFINITY)
        ));
    }

    #[test]
    fn null_equals_null_and_nothing_else() {
        assert!(same(&Value::Null, &Value::Null));
        assert!(!same(&Value::Null, &Value::Int(0)));
        assert!(!same(&Value::Null, &text("")));
    }

    #[test]
    fn an_integer_equals_the_same_float() {
        assert!(same(&Value::Int(4), &Value::Float(4.0)));
        assert!(same(&Value::Float(4.0), &Value::Int(4)));
        assert!(!same(&Value::Int(4), &Value::Float(4.5)));
        assert!(!same(&Value::Int(4), &Value::Int(5)));
    }

    #[test]
    fn large_integers_compare_exactly_with_each_other() {
        let big = 9_007_199_254_740_993;
        assert!(!same(&Value::Int(big), &Value::Int(big - 1)));
    }

    #[test]
    fn floats_are_equal_within_a_relative_tolerance() {
        assert!(same(&Value::Float(0.1 + 0.2), &Value::Float(0.3)));
        assert!(same(&Value::Float(1e12), &Value::Float(1e12 + 1e-1)));
        assert!(!same(&Value::Float(1.0), &Value::Float(1.000_001)));
        assert!(!same(&Value::Float(1e-12), &Value::Float(2e-12)));
        assert!(same(&Value::Float(0.0), &Value::Float(-0.0)));
    }

    #[test]
    fn text_and_bytes_match_exactly() {
        assert!(same(&text("Blue"), &text("Blue")));
        assert!(!same(&text("Blue"), &text("blue")));
        assert!(!same(&text("4"), &Value::Int(4)));
        assert!(same(&Value::Bytes(vec![1]), &Value::Bytes(vec![1])));
        assert!(!same(&Value::Bytes(vec![1]), &Value::Bytes(vec![2])));
    }

    #[test]
    fn rows_match_cell_by_cell() {
        assert!(same_row(
            &[Value::Int(1), text("a")],
            &[Value::Float(1.0), text("a")]
        ));
        assert!(!same_row(
            &[Value::Int(1), text("a")],
            &[Value::Int(1), text("b")]
        ));
        assert!(!same_row(&[Value::Int(1)], &[Value::Int(1), Value::Null]));
    }

    #[test]
    fn cells_sort_null_then_numbers_then_text_then_bytes() {
        let ordered = [
            Value::Null,
            Value::Int(-2),
            Value::Float(1.5),
            Value::Int(2),
            text("a"),
            text("b"),
            Value::Bytes(vec![0]),
            Value::Bytes(vec![1]),
        ];
        for pair in ordered.windows(2) {
            assert_eq!(compare(&pair[0], &pair[1]), Ordering::Less, "{pair:?}");
            assert_eq!(compare(&pair[1], &pair[0]), Ordering::Greater, "{pair:?}");
        }
        assert_eq!(compare(&Value::Int(2), &Value::Float(2.0)), Ordering::Equal);
        assert_eq!(compare(&Value::Null, &Value::Null), Ordering::Equal);
    }

    #[test]
    fn rows_sort_by_their_first_differing_cell_then_length() {
        let short = [Value::Int(1)];
        let long = [Value::Int(1), Value::Int(0)];
        let later = [Value::Int(2)];
        assert_eq!(compare_rows(&short, &long), Ordering::Less);
        assert_eq!(compare_rows(&long, &later), Ordering::Less);
        assert_eq!(compare_rows(&later, &long), Ordering::Greater);
        assert_eq!(compare_rows(&short, &short), Ordering::Equal);
    }
}
