use super::{Challenge, parse};
use crate::{embedded::RawTrack, error::ContentError};

const TRACK: &str = r#"
title = "Test Pressings"
emoji = "💿"
intro = "A shop that sells test pressings."
concepts = ["SELECT"]
difficulty = "Beginner"
order = 3
"#;

const CHALLENGES: &str = r#"
[[challenge]]
id = "first-spin"
title = "First spin"
prompt = "List every album."
kind = "query"
ordered = true
concepts = ["SELECT"]
solution = "SELECT title FROM albums"
hints = ["one", "two", "three"]
skeleton = "SELECT ___ FROM albums"

[[challenge]]
id = "b-side"
title = "B-side"
prompt = "List every artist."
kind = "query"
solution = "SELECT name FROM artists"
hints = ["one", "two", "three"]
"#;

fn raw(track: &'static str, challenges: &'static str) -> RawTrack {
    RawTrack {
        slug: "test-pressings",
        track,
        schema: "CREATE TABLE albums (title TEXT);",
        seed: "INSERT INTO albums VALUES ('Blue');",
        challenges,
    }
}

/// A challenges file holding one challenge with `field` set to `value`,
/// replacing that field's default.
fn one_with(field: &str, value: &str) -> String {
    let mut lines = vec![
        "[[challenge]]".to_owned(),
        "title = \"T\"".to_owned(),
        "prompt = \"P\"".to_owned(),
        "kind = \"query\"".to_owned(),
    ];
    let defaults = [
        ("id", "\"ok\""),
        ("solution", "\"SELECT 1\""),
        ("hints", "[\"a\", \"b\", \"c\"]"),
    ];
    for (name, default) in defaults {
        let shown = if name == field { value } else { default };
        lines.push(format!("{name} = {shown}"));
    }
    if field == "verify" {
        lines.push(format!("verify = {value}"));
    }
    lines.join("\n")
}

fn field_error(field: &str, value: &str) -> Result<ContentError, String> {
    let text: &'static str = Box::leak(one_with(field, value).into_boxed_str());
    parse(&raw(TRACK, text))
        .err()
        .ok_or(format!("{field} = {value} loaded"))
}

#[test]
fn parses_the_track() {
    let track = parse(&raw(TRACK, CHALLENGES)).unwrap();
    assert_eq!(track.slug, "test-pressings");
    assert_eq!(track.title, "Test Pressings");
    assert_eq!(track.emoji, "💿");
    assert_eq!(track.intro, "A shop that sells test pressings.");
    assert_eq!(track.concepts, ["SELECT"]);
    assert_eq!(track.difficulty, "Beginner");
    assert_eq!(track.order, 3);
    assert_eq!(track.schema, "CREATE TABLE albums (title TEXT);");
    assert_eq!(track.seed, "INSERT INTO albums VALUES ('Blue');");
    assert_eq!(track.challenges.len(), 2);
}

#[test]
fn parses_every_field_of_a_challenge() {
    let track = parse(&raw(TRACK, CHALLENGES)).unwrap();
    assert_eq!(
        track.challenges[0],
        Challenge {
            id: "first-spin".to_owned(),
            title: "First spin".to_owned(),
            prompt: "List every album.".to_owned(),
            kind: "query".to_owned(),
            ordered: true,
            concepts: vec!["SELECT".to_owned()],
            solution: "SELECT title FROM albums".to_owned(),
            hints: vec!["one".to_owned(), "two".to_owned(), "three".to_owned()],
            skeleton: "SELECT ___ FROM albums".to_owned(),
        }
    );
}

#[test]
fn ordered_concepts_and_skeleton_are_optional() {
    let track = parse(&raw(TRACK, CHALLENGES)).unwrap();
    let b_side = &track.challenges[1];
    assert_eq!(b_side.id, "b-side");
    assert!(!b_side.ordered);
    assert!(b_side.concepts.is_empty());
    assert_eq!(b_side.skeleton, "");
}

#[test]
fn bad_toml_names_the_file() {
    let error = parse(&raw("title = ", CHALLENGES)).unwrap_err();
    assert!(
        matches!(&error, ContentError::Parse { file, .. } if file == "test-pressings/track.toml")
    );
    let error = parse(&raw(TRACK, "[[challenge]]\nid = 1")).unwrap_err();
    assert!(
        matches!(&error, ContentError::Parse { file, .. } if file == "test-pressings/challenges.toml")
    );
}

#[test]
fn unknown_fields_are_refused() {
    let track = "title = \"T\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = 1\ncolour = \"red\"";
    let error = parse(&raw(track, CHALLENGES)).unwrap_err();
    assert!(matches!(error, ContentError::Parse { message, .. } if message.contains("colour")));
}

#[test]
fn ids_must_be_lowercase_letters_digits_and_hyphens() {
    for bad in ["\"\"", "\"First\"", "\"a b\"", "\"a/b\"", "\"a_b\""] {
        let error = field_error("id", bad).unwrap();
        assert!(
            matches!(error, ContentError::Field { field: "id", .. }),
            "{bad}"
        );
    }
    let loaded = field_error("id", "\"track-2-b\"");
    assert_eq!(loaded, Err("id = \"track-2-b\" loaded".to_owned()));
}

#[test]
fn a_solution_must_not_be_blank() {
    let error = field_error("solution", "\"  \"").unwrap();
    assert_eq!(
        error,
        ContentError::Field {
            file: "test-pressings/challenges.toml".to_owned(),
            challenge: "ok".to_owned(),
            field: "solution",
            problem: "must not be empty",
        }
    );
}

#[test]
fn there_must_be_exactly_three_hints() {
    for bad in ["[\"a\", \"b\"]", "[\"a\", \"b\", \"c\", \"d\"]"] {
        let error = field_error("hints", bad).unwrap();
        assert!(matches!(error, ContentError::Field { field: "hints", .. }));
    }
}

#[test]
fn verify_is_refused_until_mutation_challenges_exist() {
    let error = field_error("verify", "\"SELECT 1\"").unwrap();
    assert!(matches!(
        error,
        ContentError::Field {
            field: "verify",
            ..
        }
    ));
}

#[test]
fn a_repeated_id_is_refused() {
    let twice = "[[challenge]]\nid = \"a\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]\n\n[[challenge]]\nid = \"a\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]";
    let error = parse(&raw(TRACK, twice)).unwrap_err();
    assert_eq!(
        error,
        ContentError::Duplicate {
            file: "test-pressings/challenges.toml".to_owned(),
            challenge: "a".to_owned(),
        }
    );
}
