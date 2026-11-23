//! The grading pipeline (parent spec §5.4): columns, row count, contents,
//! then order. It stops at the first rule that fails, and reports where
//! every rule stands.
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

    /// Whether this rule has anything to check in `comparison`; a rule that
    /// doesn't apply is reported as not checked.
    fn applies(&self, comparison: &Comparison<'_>) -> bool;
}

/// Where one rule stands after grading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuleStatus {
    /// The rule found nothing wrong.
    Passed,
    /// The rule found this wrong.
    Failed(CheckFailure),
    /// An earlier rule failed, or this one doesn't apply.
    NotChecked,
}

/// One rule's outcome.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuleOutcome {
    /// The rule's [`CheckRule::id`].
    pub rule: &'static str,
    /// Where it stands.
    pub status: RuleStatus,
}

/// Every rule's outcome, in pipeline order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Grading {
    outcomes: Vec<RuleOutcome>,
}

impl Grading {
    /// A grading made of `outcomes`, in pipeline order.
    pub fn new(outcomes: Vec<RuleOutcome>) -> Self {
        Self { outcomes }
    }

    /// Every rule's outcome, in pipeline order.
    pub fn outcomes(&self) -> &[RuleOutcome] {
        &self.outcomes
    }

    /// The failure of the rule that failed, if one did.
    pub fn failure(&self) -> Option<&CheckFailure> {
        self.outcomes
            .iter()
            .find_map(|outcome| match &outcome.status {
                RuleStatus::Failed(failure) => Some(failure),
                _ => None,
            })
    }
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

/// Runs the pipeline on `comparison`. Every rule after the first failure,
/// and every rule that doesn't apply, is not checked.
pub fn grade(comparison: &Comparison<'_>) -> Grading {
    let mut failed = false;
    let outcomes = pipeline()
        .iter()
        .map(|rule| {
            let status = if failed || !rule.applies(comparison) {
                RuleStatus::NotChecked
            } else {
                rule.check(comparison)
                    .map_or(RuleStatus::Passed, RuleStatus::Failed)
            };
            failed |= matches!(status, RuleStatus::Failed(_));
            RuleOutcome {
                rule: rule.id(),
                status,
            }
        })
        .collect();
    Grading::new(outcomes)
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
        CheckFailure, Comparison, RuleStatus, grade, pipeline,
        testing::{ints, result},
    };

    fn statuses(expected: &[i64], got: &[i64], names: &[&str], ordered: bool) -> Vec<RuleStatus> {
        let expected = result(&["n"], ints(expected));
        let got = result(names, ints(got));
        let comparison = Comparison {
            expected: &expected,
            got: &got,
            ordered,
        };
        let grading = grade(&comparison);
        grading
            .outcomes()
            .iter()
            .map(|outcome| outcome.status.clone())
            .collect()
    }

    #[test]
    fn the_rules_run_columns_count_contents_then_order() {
        let ids: Vec<&str> = pipeline().iter().map(|rule| rule.id()).collect();
        assert_eq!(ids, ["columns", "row count", "contents", "order"]);
    }

    #[test]
    fn every_rule_after_the_first_failure_is_not_checked() {
        let failure = CheckFailure::RowCount {
            expected: 2,
            got: 1,
        };
        assert_eq!(
            statuses(&[1, 2], &[1], &["n"], true),
            [
                RuleStatus::Passed,
                RuleStatus::Failed(failure),
                RuleStatus::NotChecked,
                RuleStatus::NotChecked,
            ]
        );
    }

    #[test]
    fn a_failure_at_the_last_rule_follows_three_passes() {
        let statuses = statuses(&[1, 2], &[2, 1], &["n"], true);
        assert_eq!(statuses[..3], [const { RuleStatus::Passed }; 3]);
        assert_eq!(
            statuses[3],
            RuleStatus::Failed(CheckFailure::Order { row: 0 })
        );
    }

    #[test]
    fn a_matching_answer_passes_every_rule_that_applies() {
        assert_eq!(
            statuses(&[1, 2], &[1, 2], &["N"], true),
            [const { RuleStatus::Passed }; 4]
        );
        let unordered = statuses(&[1, 2], &[2, 1], &["n"], false);
        assert_eq!(unordered[..3], [const { RuleStatus::Passed }; 3]);
        assert_eq!(unordered[3], RuleStatus::NotChecked);
    }

    #[test]
    fn the_failure_is_the_rule_that_failed() {
        let expected = result(&["n"], ints(&[1, 2]));
        let wrong_columns = result(&["m", "n"], ints(&[3]));
        let failed = grade(&Comparison {
            expected: &expected,
            got: &wrong_columns,
            ordered: true,
        });
        assert_eq!(
            failed.failure(),
            Some(&CheckFailure::ColumnCount {
                expected: 1,
                got: 2
            })
        );
        assert_eq!(failed.outcomes()[0].rule, "columns");
        let solved = grade(&Comparison {
            expected: &expected,
            got: &expected,
            ordered: true,
        });
        assert_eq!(solved.failure(), None);
    }
}
