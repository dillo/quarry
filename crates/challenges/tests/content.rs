//! Content tests (parent spec §7.3): every embedded track loads, and every
//! challenge can be solved by its own solution and not by its skeleton.

use quarry_challenges::{
    catalog::{Catalog, ChallengeId},
    check::{CheckError, Verdict, check, expect},
    rules::CheckFailure,
    run::{LIMITS, run},
    sandbox::Template,
    track::Track,
};
use quarry_engine::error::EngineError;

fn catalog() -> Catalog {
    let (catalog, errors) = Catalog::embedded();
    assert_eq!(errors, []);
    catalog
}

fn template(track: &Track) -> Result<Template, EngineError> {
    Template::new(&track.schema, &track.seed)
}

/// What is wrong with `sql` as an answer to challenge `id` of Vinyl Vault;
/// `None` when it solves it.
fn vinyl(id: &str, sql: &str) -> Result<Option<CheckFailure>, CheckError> {
    let catalog = catalog();
    let Some((track, challenge)) = catalog.challenge(&ChallengeId::new("vinyl-vault", id)) else {
        return Err(CheckError::Content(format!("no challenge {id}")));
    };
    let template = template(track).map_err(CheckError::Sandbox)?;
    check(&template, challenge, sql).map(|verdict| verdict.failure().cloned())
}

#[test]
fn every_track_loads_and_vinyl_vault_comes_first() {
    let catalog = catalog();
    let first = catalog.tracks().first().expect("at least one track");
    assert_eq!(first.slug, "vinyl-vault");
    assert_eq!(first.title, "Vinyl Vault");
    let ids: Vec<&str> = first.challenges.iter().map(|c| c.id.as_str()).collect();
    assert_eq!(
        ids,
        [
            "first-spin",
            "crate-digging",
            "new-arrivals",
            "every-genre",
            "unknown-origins"
        ]
    );
}

#[test]
fn every_solution_solves_its_challenge_and_returns_rows() {
    for track in catalog().tracks() {
        let template = template(track).unwrap();
        for challenge in &track.challenges {
            let verdict = check(&template, challenge, &challenge.solution);
            assert_eq!(
                verdict.as_ref().map(Verdict::is_solved),
                Ok(true),
                "{}/{}",
                track.slug,
                challenge.id
            );
            let mut copy = template.fresh().expect("a copy");
            let rows = run(&mut copy, &challenge.solution, LIMITS)
                .expect("the solution runs")
                .rows;
            assert!(
                !rows.is_empty(),
                "{}/{} returns no rows",
                track.slug,
                challenge.id
            );
        }
    }
}

#[test]
fn every_challenge_has_an_expectation_with_rows() {
    for track in catalog().tracks() {
        let template = template(track).unwrap();
        for challenge in &track.challenges {
            let expectation = expect(&template, challenge);
            let name = format!("{}/{}", track.slug, challenge.id);
            let expectation = expectation.unwrap_or_else(|error| panic!("{name}: {error:?}"));
            assert!(!expectation.columns.is_empty(), "{name} has no columns");
            assert!(expectation.rows > 0, "{name} has no rows");
        }
    }
}

#[test]
fn no_skeleton_is_already_an_answer() {
    for track in catalog().tracks() {
        let template = template(track).unwrap();
        for challenge in &track.challenges {
            let verdict = check(&template, challenge, &challenge.skeleton);
            assert_ne!(
                verdict.as_ref().map(Verdict::is_solved),
                Ok(true),
                "{}/{}",
                track.slug,
                challenge.id
            );
        }
    }
}

#[test]
fn wrong_columns_are_caught() {
    assert_eq!(
        vinyl("first-spin", "SELECT title FROM albums"),
        Ok(Some(CheckFailure::ColumnCount {
            expected: 2,
            got: 1
        }))
    );
    assert!(matches!(
        vinyl("first-spin", "SELECT title, year AS released FROM albums"),
        Ok(Some(CheckFailure::ColumnNames { .. }))
    ));
}

#[test]
fn a_wrong_row_count_is_caught() {
    assert_eq!(
        vinyl(
            "crate-digging",
            "SELECT title, year FROM albums WHERE year <= 1975"
        ),
        Ok(Some(CheckFailure::RowCount {
            expected: 7,
            got: 8
        }))
    );
    assert_eq!(
        vinyl(
            "unknown-origins",
            "SELECT name FROM artists WHERE country = NULL"
        ),
        Ok(Some(CheckFailure::RowCount {
            expected: 2,
            got: 0
        }))
    );
    assert_eq!(
        vinyl("every-genre", "SELECT genre FROM albums"),
        Ok(Some(CheckFailure::RowCount {
            expected: 8,
            got: 18
        }))
    );
}

#[test]
fn wrong_rows_and_wrong_order_are_caught() {
    assert_eq!(
        vinyl(
            "new-arrivals",
            "SELECT title, year FROM albums ORDER BY year LIMIT 5"
        ),
        Ok(Some(CheckFailure::Contents {
            missing: 5,
            unexpected: 5
        }))
    );
    assert_eq!(
        vinyl(
            "new-arrivals",
            "SELECT title, year FROM albums WHERE year >= 1985 ORDER BY year"
        ),
        Ok(Some(CheckFailure::Order { row: 0 }))
    );
}
