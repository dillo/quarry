//! Columns: the same number, with the same names in the same order,
//! ignoring case.

use super::{CheckFailure, CheckRule, Comparison};
use quarry_engine::result::QueryResult;

/// The first rule.
pub struct Columns;

impl CheckRule for Columns {
    fn id(&self) -> &'static str {
        "columns"
    }

    fn applies(&self, _: &Comparison<'_>) -> bool {
        true
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        let expected = names(comparison.expected);
        let got = names(comparison.got);
        if expected.len() != got.len() {
            return Some(CheckFailure::ColumnCount {
                expected: expected.len(),
                got: got.len(),
            });
        }
        let differ = expected
            .iter()
            .zip(&got)
            .any(|(a, b)| a.to_lowercase() != b.to_lowercase());
        differ.then_some(CheckFailure::ColumnNames { expected, got })
    }
}

fn names(result: &QueryResult) -> Vec<String> {
    result
        .columns
        .iter()
        .map(|column| column.name.clone())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::Columns;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    fn check(expected: &[&str], got: &[&str]) -> Option<CheckFailure> {
        let expected = result(expected, ints(&[]));
        let got = result(got, ints(&[]));
        Columns.check(&Comparison {
            expected: &expected,
            got: &got,
            ordered: false,
        })
    }

    #[test]
    fn names_match_ignoring_case() {
        assert_eq!(check(&["title", "year"], &["Title", "YEAR"]), None);
    }

    #[test]
    fn a_different_count_is_reported_first() {
        assert_eq!(
            check(&["title", "year"], &["title"]),
            Some(CheckFailure::ColumnCount {
                expected: 2,
                got: 1
            })
        );
    }

    #[test]
    fn different_names_or_order_are_reported_with_both_lists() {
        assert_eq!(
            check(&["title", "year"], &["year", "title"]),
            Some(CheckFailure::ColumnNames {
                expected: vec!["title".to_owned(), "year".to_owned()],
                got: vec!["year".to_owned(), "title".to_owned()],
            })
        );
    }
}
