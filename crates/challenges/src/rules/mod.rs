//! The grading pipeline (parent spec §5.4): columns, row count, contents,
//! then order. It stops at the first rule that fails.
//!
//! Each rule is a [`CheckRule`]; adding one is a new module and one line in
//! [`pipeline`].

mod columns;
mod contents;
mod count;
mod order;
pub mod value;

use quarry_engine::result::QueryResult;

/// What was wrong with an answer. It says what differs, never the expected
/// rows themselves.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CheckFailure {
    /// The answer has a different number of columns.
    ColumnCount {
        /// How many the solution returns.
        expected: usize,
        /// How many the answer returned.
        got: usize,
    },
    /// The columns' names differ, ignoring case.
    ColumnNames {
        /// The solution's column names, in order.
        expected: Vec<String>,
        /// The answer's column names, in order.
        got: Vec<String>,
    },
    /// The answer has a different number of rows.
    RowCount {
        /// How many the solution returns.
        expected: usize,
        /// How many the answer returned.
        got: usize,
    },
    /// The rows differ, ignoring their order.
    Contents {
        /// Rows the solution returns that the answer lacks.
        missing: usize,
        /// Rows the answer returned that the solution doesn't.
        unexpected: usize,
    },
    /// The rows are right but in the wrong order.
    Order {
        /// The first row out of place, counting from 0.
        row: usize,
    },
}

/// The two results a rule compares.
#[derive(Debug, Clone, Copy)]
pub struct Comparison<'a> {
    /// The reference solution's result.
    pub expected: &'a QueryResult,
    /// The user's result.
    pub got: &'a QueryResult,
    /// Whether the rows must come in the solution's order.
    pub ordered: bool,
}

/// One step of the pipeline.
pub trait CheckRule {
    /// A stable name, such as "columns".
    fn id(&self) -> &'static str;

    /// What this rule finds wrong, if anything. It may assume every earlier
    /// rule passed.
    fn check(&self, comparison: &Comparison<'_>) -> Option<CheckFailure>;
}

/// The rules in the order they run.
pub fn pipeline() -> Vec<Box<dyn CheckRule>> {
    vec![
        Box::new(columns::Columns),
        Box::new(count::RowCount),
        Box::new(contents::Contents),
        Box::new(order::Order),
    ]
}

/// The first rule's failure, or `None` when every rule passes.
pub fn first_failure(comparison: &Comparison<'_>) -> Option<CheckFailure> {
    pipeline().iter().find_map(|rule| rule.check(comparison))
}

#[cfg(test)]
pub(crate) mod testing {
    //! Building results for the rules' tests.

    use quarry_engine::{
        result::{Column, QueryResult},
        value::Value,
    };

    /// A result with columns `names` and `rows`.
    pub fn result(names: &[&str], rows: Vec<Vec<Value>>) -> QueryResult {
        let columns = names
            .iter()
            .map(|name| Column {
                name: (*name).to_owned(),
                type_name: String::new(),
            })
            .collect();
        QueryResult { columns, rows }
    }

    /// One-column rows holding `numbers`.
    pub fn ints(numbers: &[i64]) -> Vec<Vec<Value>> {
        numbers.iter().map(|n| vec![Value::Int(*n)]).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CheckFailure, Comparison, first_failure, pipeline,
        testing::{ints, result},
    };

    #[test]
    fn the_rules_run_columns_count_contents_then_order() {
        let ids: Vec<&str> = pipeline().iter().map(|rule| rule.id()).collect();
        assert_eq!(ids, ["columns", "row count", "contents", "order"]);
    }

    #[test]
    fn the_first_failing_rule_is_reported() {
        let expected = result(&["n"], ints(&[1, 2]));
        let wrong_columns = result(&["m", "n"], ints(&[3]));
        let comparison = Comparison {
            expected: &expected,
            got: &wrong_columns,
            ordered: true,
        };
        assert_eq!(
            first_failure(&comparison),
            Some(CheckFailure::ColumnCount {
                expected: 1,
                got: 2
            })
        );
    }

    #[test]
    fn a_matching_answer_passes_every_rule() {
        let expected = result(&["n"], ints(&[1, 2]));
        let got = result(&["N"], ints(&[1, 2]));
        let comparison = Comparison {
            expected: &expected,
            got: &got,
            ordered: true,
        };
        assert_eq!(first_failure(&comparison), None);
    }
}
