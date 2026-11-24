use quarry_challenges::{
    catalog::Position,
    check::{CheckError, Expectation},
    track::{Challenge, Track},
};

use super::{Brief, Expected, brief};
use crate::model::attempts::Tally;

fn track() -> Track {
    Track {
        slug: "vinyl-vault".to_owned(),
        title: "Vinyl Vault".to_owned(),
        emoji: "E".to_owned(),
        intro: "I".to_owned(),
        concepts: Vec::new(),
        difficulty: "D".to_owned(),
        order: 1,
        schema: String::new(),
        seed: String::new(),
        challenges: Vec::new(),
    }
}

fn challenge(ordered: bool) -> Challenge {
    Challenge {
        id: "first-spin".to_owned(),
        title: "First spin".to_owned(),
        prompt: "List every album.".to_owned(),
        kind: "query".to_owned(),
        ordered,
        concepts: vec!["SELECT".to_owned(), "columns".to_owned()],
        solution: String::new(),
        hints: Vec::new(),
        skeleton: String::new(),
    }
}

fn ready(columns: &[&str], rows: usize) -> Expected {
    Expected::Ready(Expectation {
        columns: columns.iter().map(|name| (*name).to_owned()).collect(),
        rows,
    })
}

fn of(expected: &Expected, ordered: bool, checks: u32) -> Brief {
    let tally = Tally {
        checks,
        solved: false,
    };
    brief(
        &track(),
        &challenge(ordered),
        Position { index: 2, of: 5 },
        expected,
        tally,
    )
}

#[test]
fn the_brief_places_and_tells_the_challenge() {
    let brief = of(&ready(&["title", "year"], 1_200), false, 2);
    assert_eq!(
        brief,
        Brief {
            place: "Vinyl Vault · Challenge 2 of 5".to_owned(),
            title: "First spin".to_owned(),
            prompt: "List every album.".to_owned(),
            expected: vec![
                "Expected columns: title, year".to_owned(),
                "1,200 rows".to_owned()
            ],
            order: "Any order",
            concepts: vec!["SELECT".to_owned(), "columns".to_owned()],
            attempts: "Attempts: 2".to_owned(),
        }
    );
}

#[test]
fn one_row_is_singular_and_order_can_matter() {
    let brief = of(&ready(&["n"], 1), true, 0);
    assert_eq!(brief.expected[1], "1 row");
    assert_eq!(brief.order, "Order matters");
    assert_eq!(brief.attempts, "Attempts: 0");
}

#[test]
fn an_unknown_expectation_says_why() {
    assert_eq!(
        of(&Expected::Pending, false, 0).expected,
        ["Working out the expected result…"]
    );
    assert_eq!(
        of(&Expected::Unavailable, false, 0).expected,
        ["Expected result unavailable"]
    );
    assert_eq!(Expected::default(), Expected::Pending);
}

#[test]
fn a_result_becomes_ready_or_unavailable() {
    let expectation = Expectation {
        columns: vec!["n".to_owned()],
        rows: 3,
    };
    assert_eq!(
        Expected::from(Ok(expectation.clone())),
        Expected::Ready(expectation)
    );
    assert_eq!(
        Expected::from(Err(CheckError::TimedOut)),
        Expected::Unavailable
    );
}
