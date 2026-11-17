//! Row count: the same number of rows.

use super::{CheckFailure, CheckRule, Comparison};

/// The second rule.
pub struct RowCount;

impl CheckRule for RowCount {
    fn id(&self) -> &'static str {
        "row count"
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        let expected = comparison.expected.rows.len();
        let got = comparison.got.rows.len();
        (expected != got).then_some(CheckFailure::RowCount { expected, got })
    }
}

#[cfg(test)]
mod tests {
    use super::RowCount;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    #[test]
    fn counts_rows_only() {
        let expected = result(&["n"], ints(&[1, 2, 3]));
        let same_count = result(&["n"], ints(&[7, 8, 9]));
        let fewer = result(&["n"], ints(&[1]));
        let check = |got| {
            RowCount.check(&Comparison {
                expected: &expected,
                got,
                ordered: false,
            })
        };
        assert_eq!(check(&same_count), None);
        assert_eq!(
            check(&fewer),
            Some(CheckFailure::RowCount {
                expected: 3,
                got: 1
            })
        );
    }
}
