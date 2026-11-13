//! The Plan view's rows: a query plan flattened with each step's depth.

use quarry_engine::{
    error::EngineError,
    plan::{Plan, PlanNode},
};

/// One step of a plan, indented by `depth`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanRow {
    /// How deeply the step is nested; top-level steps are 0.
    pub depth: usize,
    /// The database's description of the step.
    pub label: String,
}

/// What the Plan view shows.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum PlanState {
    /// Nothing has been explained yet.
    #[default]
    Idle,
    /// The plan has been asked for.
    Planning,
    /// The plan's steps.
    Ready(Vec<PlanRow>),
    /// Why there is no plan.
    Failed(String),
}

impl PlanState {
    /// The state for the session's answer.
    pub fn answered(answer: Result<Plan, EngineError>) -> Self {
        match answer {
            Ok(plan) => Self::Ready(rows(&plan)),
            Err(error) => Self::Failed(error.to_string()),
        }
    }
}

/// Every step, depth first, in the database's order.
pub fn rows(plan: &Plan) -> Vec<PlanRow> {
    let mut rows = Vec::new();
    push(&plan.nodes, 0, &mut rows);
    rows
}

fn push(nodes: &[PlanNode], depth: usize, rows: &mut Vec<PlanRow>) {
    for node in nodes {
        rows.push(PlanRow {
            depth,
            label: node.label.clone(),
        });
        push(&node.children, depth + 1, rows);
    }
}

#[cfg(test)]
mod tests {
    use quarry_engine::{
        error::EngineError,
        plan::{Plan, PlanNode},
    };

    use super::{PlanRow, PlanState, rows};

    fn node(label: &str, children: Vec<PlanNode>) -> PlanNode {
        PlanNode {
            label: label.to_owned(),
            children,
        }
    }

    fn row(depth: usize, label: &str) -> PlanRow {
        PlanRow {
            depth,
            label: label.to_owned(),
        }
    }

    #[test]
    fn flattens_depth_first_keeping_order() {
        let plan = Plan {
            nodes: vec![
                node(
                    "SCAN ships",
                    vec![node("SUBQUERY", vec![node("SCAN crew", vec![])])],
                ),
                node("USE TEMP B-TREE", vec![]),
            ],
        };
        assert_eq!(
            rows(&plan),
            [
                row(0, "SCAN ships"),
                row(1, "SUBQUERY"),
                row(2, "SCAN crew"),
                row(0, "USE TEMP B-TREE"),
            ]
        );
    }

    #[test]
    fn an_empty_plan_has_no_rows() {
        assert!(rows(&Plan::default()).is_empty());
    }

    #[test]
    fn an_answer_becomes_rows_or_a_reason() {
        let plan = Plan {
            nodes: vec![node("SCAN t", vec![])],
        };
        assert_eq!(
            PlanState::answered(Ok(plan)),
            PlanState::Ready(vec![row(0, "SCAN t")])
        );
        assert_eq!(
            PlanState::answered(Err(EngineError::PlanUnsupported)),
            PlanState::Failed("This database can't show query plans.".to_owned())
        );
        assert_eq!(PlanState::default(), PlanState::Idle);
    }
}
