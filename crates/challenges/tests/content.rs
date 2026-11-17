//! Content tests (parent spec §7.3): every embedded track loads, and every
//! challenge can be solved by its own solution and not by its skeleton.

use quarry_challenges::{
    catalog::{Catalog, ChallengeId},
    check::{CheckError, Verdict, check},
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

/// Checks `sql` against challenge `id` of Vinyl Vault.
fn vinyl(id: &str, sql: &str) -> Result<Verdict, CheckError> {
    let catalog = catalog();
    let Some((track, challenge)) = catalog.challenge(&ChallengeId::new("vinyl-vault", id)) else {
        return Err(CheckError::Content(format!("no challenge {id}")));
    };
    let template = template(track).map_err(CheckError::Sandbox)?;
    check(&template, challenge, sql)
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
                verdict,
                Ok(Verdict::Solved),
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
fn no_skeleton_is_already_an_answer() {
    for track in catalog().tracks() {
        let template = template(track).unwrap();
        for challenge in &track.challenges {
            let verdict = check(&template, challenge, &challenge.skeleton);
            assert_ne!(
                verdict,
                Ok(Verdict::Solved),
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
        Ok(Verdict::Wrong(CheckFailure::ColumnCount {
            expected: 2,
            got: 1
        }))
    );
    assert!(matches!(
        vinyl("first-spin", "SELECT title, year AS released FROM albums"),
        Ok(Verdict::Wrong(CheckFailure::ColumnNames { .. }))
    ));
}

#[test]
fn a_wrong_row_count_is_caught() {
    assert_eq!(
        vinyl(
            "crate-digging",
            "SELECT title, year FROM albums WHERE year <= 1975"
        ),
        Ok(Verdict::Wrong(CheckFailure::RowCount {
            expected: 7,
            got: 8
        }))
    );
    assert_eq!(
        vinyl(
            "unknown-origins",
            "SELECT name FROM artists WHERE country = NULL"
        ),
        Ok(Verdict::Wrong(CheckFailure::RowCount {
            expected: 2,
            got: 0
        }))
    );
    assert_eq!(
        vinyl("every-genre", "SELECT genre FROM albums"),
        Ok(Verdict::Wrong(CheckFailure::RowCount {
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
        Ok(Verdict::Wrong(CheckFailure::Contents {
            missing: 5,
            unexpected: 5
        }))
    );
    assert_eq!(
        vinyl(
            "new-arrivals",
            "SELECT title, year FROM albums WHERE year >= 1985 ORDER BY year"
        ),
        Ok(Verdict::Wrong(CheckFailure::Order { row: 0 }))
    );
}
