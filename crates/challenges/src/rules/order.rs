//! Order: when the challenge says order matters, the rows come in the
//! solution's order.

use super::{CheckFailure, CheckRule, Comparison, value};

/// The fourth rule; it checks nothing unless `ordered` is set.
pub struct Order;

impl CheckRule for Order {
    fn id(&self) -> &'static str {
        "order"
    }

    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure> {
        if !self.applies(comparison) {
            return None;
        }
        comparison
            .expected
            .rows
            .iter()
            .zip(&comparison.got.rows)
            .position(|(a, b)| !value::same_row(a, b))
            .map(|row| CheckFailure::Order { row })
    }

    fn applies(&self, comparison: &Comparison<'_>) -> bool {
        comparison.ordered
    }
}

#[cfg(test)]
mod tests {
    use super::Order;
    use crate::rules::{
        CheckFailure, CheckRule, Comparison,
        testing::{ints, result},
    };

    fn check(expected: &[i64], got: &[i64], ordered: bool) -> Option<CheckFailure> {
        let expected = result(&["n"], ints(expected));
        let got = result(&["n"], ints(got));
        Order.check(&Comparison {
            expected: &expected,
            got: &got,
            ordered,
        })
    }

    #[test]
    fn reports_the_first_row_out_of_place() {
        assert_eq!(
            check(&[1, 2, 3], &[1, 3, 2], true),
            Some(CheckFailure::Order { row: 1 })
        );
        assert_eq!(check(&[1, 2, 3], &[1, 2, 3], true), None);
    }

    #[test]
    fn checks_nothing_when_order_does_not_matter() {
        assert_eq!(check(&[1, 2, 3], &[3, 2, 1], false), None);
    }
}
