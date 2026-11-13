//! A query plan: how the database intends to run a statement.

/// The steps a database will take to run one statement, as a tree.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Plan {
    /// The top-level steps, in the order the database reports them.
    pub nodes: Vec<PlanNode>,
}

/// One step of a plan and the steps nested under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanNode {
    /// The database's description of the step.
    pub label: String,
    /// Nested steps, in order.
    pub children: Vec<PlanNode>,
}

/// One reported step: its id, its parent's id (0 for a top-level step), and
/// its description.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanStep {
    /// The step's id; ids are positive.
    pub id: i64,
    /// The parent step's id, or 0.
    pub parent: i64,
    /// The step's description.
    pub label: String,
}

impl Plan {
    /// Builds the tree from flat steps, keeping their order.
    ///
    /// A step is nested only under a parent with a smaller id, so a malformed
    /// list (a step naming itself or a later step) cannot loop; such a step
    /// is dropped.
    pub fn from_steps(steps: &[PlanStep]) -> Self {
        Self {
            nodes: children(steps, 0),
        }
    }
}

fn children(steps: &[PlanStep], parent: i64) -> Vec<PlanNode> {
    steps
        .iter()
        .filter(|step| step.parent == parent && step.id > parent)
        .map(|step| PlanNode {
            label: step.label.clone(),
            children: children(steps, step.id),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{Plan, PlanNode, PlanStep};

    fn step(id: i64, parent: i64, label: &str) -> PlanStep {
        PlanStep {
            id,
            parent,
            label: label.to_owned(),
        }
    }

    fn leaf(label: &str) -> PlanNode {
        PlanNode {
            label: label.to_owned(),
            children: Vec::new(),
        }
    }

    #[test]
    fn nests_steps_under_their_parents_in_order() {
        let plan = Plan::from_steps(&[
            step(2, 0, "SCAN a"),
            step(4, 0, "CORRELATED SUBQUERY 1"),
            step(7, 4, "SEARCH b"),
            step(9, 4, "USE TEMP B-TREE"),
        ]);
        assert_eq!(
            plan.nodes,
            vec![
                leaf("SCAN a"),
                PlanNode {
                    label: "CORRELATED SUBQUERY 1".to_owned(),
                    children: vec![leaf("SEARCH b"), leaf("USE TEMP B-TREE")],
                },
            ]
        );
    }

    #[test]
    fn drops_steps_that_would_loop() {
        let plan = Plan::from_steps(&[step(3, 0, "SCAN a"), step(5, 5, "self"), step(4, 6, "x")]);
        assert_eq!(plan.nodes, vec![leaf("SCAN a")]);
    }

    #[test]
    fn drops_a_reachable_step_that_names_itself_as_parent() {
        let plan = Plan::from_steps(&[step(5, 0, "SCAN a"), step(5, 5, "self")]);
        assert_eq!(plan.nodes, vec![leaf("SCAN a")]);
    }

    #[test]
    fn no_steps_is_an_empty_plan() {
        assert_eq!(Plan::from_steps(&[]), Plan::default());
    }
}
