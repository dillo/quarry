//! How values, counts, durations and positions read on screen.

use std::time::Duration;

use quarry_engine::value::Value;

/// The longest text a grid cell shows before it is cut with `…`.
pub const CELL_LIMIT: usize = 200;

/// A cell's text: one line, at most [`CELL_LIMIT`] characters.
pub fn cell_text(value: &Value) -> String {
    let text = value.to_string().replace(['\n', '\r'], " ");
    match text.char_indices().nth(CELL_LIMIT) {
        Some((cut, _)) => format!("{}…", &text[..cut]),
        None => text,
    }
}

/// Numbers line up on the right, by digit; everything else on the left.
pub fn aligns_right(value: &Value) -> bool {
    matches!(value, Value::Int(_) | Value::Float(_))
}

/// `1234567` reads as `1,234,567`.
pub fn count(n: u64) -> String {
    let digits = n.to_string();
    let mut grouped = String::with_capacity(digits.len() + digits.len() / 3);
    for (index, digit) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index).is_multiple_of(3) {
            grouped.push(',');
        }
        grouped.push(digit);
    }
    grouped
}

/// `"1 row"`, `"1,000 rows"`.
pub fn rows(n: u64) -> String {
    if n == 1 {
        "1 row".to_owned()
    } else {
        format!("{} rows", count(n))
    }
}

/// Milliseconds with one decimal under a second, seconds with two under a
/// minute, then minutes and seconds.
pub fn duration(elapsed: Duration) -> String {
    let millis = elapsed.as_secs_f64() * 1000.0;
    if millis < 1000.0 {
        format!("{millis:.1} ms")
    } else if elapsed.as_secs() < 60 {
        format!("{:.2} s", elapsed.as_secs_f64())
    } else {
        format!(
            "{} m {:02} s",
            elapsed.as_secs() / 60,
            elapsed.as_secs() % 60
        )
    }
}

/// A 0-based line and column, shown 1-based: `"Ln 6, Col 26"`.
pub fn position(line: u32, column: u32) -> String {
    format!(
        "Ln {}, Col {}",
        line.saturating_add(1),
        column.saturating_add(1)
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use quarry_engine::value::Value;

    use super::{CELL_LIMIT, aligns_right, cell_text, count, duration, position, rows};

    #[test]
    fn cells_are_one_line() {
        let value = Value::Text("two\nlines\r\n".to_owned());
        assert_eq!(cell_text(&value), "two lines  ");
    }

    #[test]
    fn long_cells_are_cut_on_a_character_boundary() {
        let exact = "é".repeat(CELL_LIMIT);
        assert_eq!(cell_text(&Value::Text(exact.clone())), exact);
        let long = "é".repeat(CELL_LIMIT + 1);
        assert_eq!(cell_text(&Value::Text(long)), format!("{exact}…"));
    }

    #[test]
    fn values_keep_their_engine_display() {
        assert_eq!(cell_text(&Value::Null), "NULL");
        assert_eq!(cell_text(&Value::Float(1.0)), "1.0");
    }

    #[test]
    fn only_numbers_align_right() {
        assert!(aligns_right(&Value::Int(1)));
        assert!(aligns_right(&Value::Float(1.5)));
        assert!(!aligns_right(&Value::Text("1".to_owned())));
        assert!(!aligns_right(&Value::Null));
        assert!(!aligns_right(&Value::Bytes(vec![1])));
    }

    #[test]
    fn counts_group_thousands() {
        assert_eq!(count(0), "0");
        assert_eq!(count(999), "999");
        assert_eq!(count(1000), "1,000");
        assert_eq!(count(123_456), "123,456");
        assert_eq!(count(1_234_567), "1,234,567");
    }

    #[test]
    fn row_counts_agree_in_number() {
        assert_eq!(rows(0), "0 rows");
        assert_eq!(rows(1), "1 row");
        assert_eq!(rows(2000), "2,000 rows");
    }

    #[test]
    fn durations_pick_a_readable_unit() {
        assert_eq!(duration(Duration::from_micros(2140)), "2.1 ms");
        assert_eq!(duration(Duration::from_micros(999_940)), "999.9 ms");
        assert_eq!(duration(Duration::from_millis(1000)), "1.00 s");
        assert_eq!(duration(Duration::from_millis(1240)), "1.24 s");
        assert_eq!(duration(Duration::from_secs(59)), "59.00 s");
        assert_eq!(duration(Duration::from_secs(60)), "1 m 00 s");
        assert_eq!(duration(Duration::from_secs(65)), "1 m 05 s");
    }

    #[test]
    fn positions_are_one_based() {
        assert_eq!(position(5, 25), "Ln 6, Col 26");
        assert_eq!(position(u32::MAX, 0), format!("Ln {}, Col 1", u32::MAX));
    }
}
