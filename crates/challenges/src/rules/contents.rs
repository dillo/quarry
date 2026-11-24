//! Contents: the same rows as a multiset, ignoring order and counting
//! duplicates, with cells compared by [`value::same`].

use std::cmp::Ordering;

use quarry_engine::result::Row;

use super::{CheckFailure, CheckRule, Comparison, value};

/// The third rule.
pub struct Contents;

impl CheckRule for Contents {
    fn id(&self) -> &'static str {
        "contents"
    }

    fn applies(&self, _: &Comparison<'_>) -> bool {
        true
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        let expected = sorted(&comparison.expected.rows);
        let got = sorted(&comparison.got.rows);
        let (missing, unexpected) = differences(&expected, &got);
        (missing + unexpected > 0).then_some(CheckFailure::Contents {
            missing,
            unexpected,
        })
    }
}

fn sorted(rows: &[Row]) -> Vec<&Row> {
    let mut sorted: Vec<&Row> = rows.iter().collect();
    sorted.sort_by(|a, b| value::compare_rows(a, b));
    sorted
}

/// How many of `expected`'s rows `got` lacks, and how many of `got`'s rows
/// `expected` lacks.
fn differences(expected: &[&Row], got: &[&Row]) -> (usize, usize) {
    let matched = matched(expected, got);
    (expected.len() - matched, got.len() - matched)
}

/// How many rows the two sorted lists share, walking them together.
fn matched(expected: &[&Row], got: &[&Row]) -> usize {
    let (mut i, mut j, mut matched) = (0, 0, 0);
    while i < expected.len() && j < got.len() {
        if value::same_row(expected[i], got[j]) {
            matched += 1;
            i += 1;
            j += 1;
        } else if value::compare_rows(expected[i], got[j]) == Ordering::Less {
            i += 1;
        } else {
            j += 1;
        }
    }
    matched
}

#[cfg(test)]
mod tests {
    use quarry_engine::value::Value;

    use super::Contents;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    fn check(expected: &[i64], got: &[i64]) -> Option<CheckFailure> {
        let expected = result(&["n"], ints(expected));
        let got = result(&["n"], ints(got));
        Contents.check(&Comparison {
            expected: &expected,
            got: &got,
            ordered: false,
        })
    }

    #[test]
    fn order_is_ignored() {
        assert_eq!(check(&[1, 2, 3], &[3, 1, 2]), None);
    }

    #[test]
    fn duplicates_are_counted() {
        assert_eq!(
            check(&[1, 1, 2], &[1, 2, 2]),
            Some(CheckFailure::Contents {
                missing: 1,
                unexpected: 1
            })
        );
    }

    #[test]
    fn missing_and_unexpected_rows_are_counted_at_either_end() {
        assert_eq!(
            check(&[1, 2, 3, 4], &[0, 2, 3, 9]),
            Some(CheckFailure::Contents {
                missing: 2,
                unexpected: 2
            })
        );
        assert_eq!(
            check(&[5, 6], &[]),
            Some(CheckFailure::Contents {
                missing: 2,
                unexpected: 0
            })
        );
        assert_eq!(
            check(&[], &[5]),
            Some(CheckFailure::Contents {
                missing: 0,
                unexpected: 1
            })
        );
    }

    #[test]
    fn cells_compare_with_nulls_and_numbers_as_numbers() {
        let expected = result(&["a", "b"], vec![vec![Value::Null, Value::Int(4)]]);
        let got = result(&["a", "b"], vec![vec![Value::Null, Value::Float(4.0)]]);
        let comparison = Comparison {
            expected: &expected,
            got: &got,
            ordered: false,
        };
        assert_eq!(Contents.check(&comparison), None);
    }
}
