//! How many times each challenge has been checked this session, and
//! whether it was solved. Phase 2C keeps them across restarts.

use std::collections::BTreeMap;

use quarry_challenges::{
    catalog::ChallengeId,
    check::{CheckError, Verdict},
};

/// One challenge's checks so far.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct Tally {
    /// Checks that gave a verdict, right or wrong.
    pub checks: u32,
    /// Whether one of them solved it.
    pub solved: bool,
}

/// Every challenge's [`Tally`] this session.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Attempts {
    tallies: BTreeMap<ChallengeId, Tally>,
}

impl Attempts {
    /// Counts a check of `id` that ended in `outcome`. A check that gave no
    /// verdict, such as a failing query, doesn't count.
    pub fn record(&mut self, id: &ChallengeId, outcome: &Result<Verdict, CheckError>) {
        let Ok(verdict) = outcome else {
            return;
        };
        let tally = self.tallies.entry(id.clone()).or_default();
        tally.checks = tally.checks.saturating_add(1);
        tally.solved |= verdict.is_solved();
    }

    /// `id`'s tally; nothing yet for a challenge never checked.
    pub fn of(&self, id: &ChallengeId) -> Tally {
        self.tallies.get(id).copied().unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use quarry_challenges::{
        catalog::ChallengeId,
        check::{CheckError, Verdict},
        rules::{CheckFailure, Grading, RuleOutcome, RuleStatus},
    };

    use super::{Attempts, Tally};

    fn solved() -> Verdict {
        Verdict::of(Grading::new(Vec::new()))
    }

    fn wrong() -> Verdict {
        let outcome = RuleOutcome {
            rule: "order",
            status: RuleStatus::Failed(CheckFailure::Order { row: 0 }),
        };
        Verdict::of(Grading::new(vec![outcome]))
    }

    #[test]
    fn verdicts_count_and_a_solve_is_kept() {
        let id = ChallengeId::new("t", "a");
        let mut attempts = Attempts::default();
        assert_eq!(attempts.of(&id), Tally::default());
        attempts.record(&id, &Ok(wrong()));
        assert_eq!(
            attempts.of(&id),
            Tally {
                checks: 1,
                solved: false
            }
        );
        attempts.record(&id, &Ok(solved()));
        attempts.record(&id, &Ok(wrong()));
        assert_eq!(
            attempts.of(&id),
            Tally {
                checks: 3,
                solved: true
            }
        );
    }

    #[test]
    fn errors_dont_count_and_challenges_are_counted_apart() {
        let (a, b) = (ChallengeId::new("t", "a"), ChallengeId::new("t", "b"));
        let mut attempts = Attempts::default();
        attempts.record(&a, &Err(CheckError::TimedOut));
        assert_eq!(attempts.of(&a), Tally::default());
        attempts.record(&b, &Ok(wrong()));
        assert_eq!(attempts.of(&a).checks, 0);
        assert_eq!(attempts.of(&b).checks, 1);
    }
}
