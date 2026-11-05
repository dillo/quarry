//! Minimum mutation score from a `cargo mutants` run.

use serde::Deserialize;

use crate::{error::XtaskError, thresholds::MutationTarget, violation::Violation};

#[derive(Deserialize)]
struct Outcomes {
    caught: u32,
    missed: u32,
    timeout: u32,
}

/// Checks the contents of `mutants.out/outcomes.json` against `target`.
/// Unviable mutants (ones that do not compile) are not counted.
///
/// # Errors
/// Returns an error when the file is not valid JSON of the expected shape.
pub fn check_outcomes(json: &str, target: &MutationTarget) -> Result<Vec<Violation>, XtaskError> {
    let outcomes: Outcomes = serde_json::from_str(json).map_err(|source| XtaskError::Json {
        what: "cargo mutants outcomes".to_owned(),
        source,
    })?;
    let detected = outcomes.caught.saturating_add(outcomes.timeout);
    let total = detected.saturating_add(outcomes.missed);
    if total == 0 {
        return Ok(Vec::new());
    }
    let score = f64::from(detected) * 100.0 / f64::from(total);
    if score >= target.min_score {
        return Ok(Vec::new());
    }
    let message = format!(
        "mutation score {score:.1}% is below {}% ({} mutants missed)",
        target.min_score, outcomes.missed
    );
    Ok(vec![Violation::new("mutants.out/outcomes.json", message)])
}

#[cfg(test)]
mod tests {
    use super::check_outcomes;
    use crate::thresholds::MutationTarget;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const TARGET: MutationTarget = MutationTarget { min_score: 90.0 };

    fn outcomes(caught: u32, missed: u32, timeout: u32) -> String {
        format!(
            r#"{{"caught":{caught},"missed":{missed},"timeout":{timeout},"unviable":7,"total_mutants":0}}"#
        )
    }

    #[test]
    fn passes_at_or_above_target() -> TestResult {
        assert!(check_outcomes(&outcomes(8, 1, 1), &TARGET)?.is_empty());
        Ok(())
    }

    #[test]
    fn fails_below_target() -> TestResult {
        let found = check_outcomes(&outcomes(8, 2, 0), &TARGET)?;
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].message,
            "mutation score 80.0% is below 90% (2 mutants missed)"
        );
        Ok(())
    }

    #[test]
    fn passes_when_there_is_nothing_to_mutate() -> TestResult {
        assert!(check_outcomes(&outcomes(0, 0, 0), &TARGET)?.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_malformed_outcomes() {
        assert!(check_outcomes("{}", &TARGET).is_err());
    }
}
