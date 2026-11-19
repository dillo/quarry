//! One track: its story, its database and its challenges, parsed from a
//! track folder and checked.

use serde::{Deserialize, de::DeserializeOwned};

use crate::{embedded::RawTrack, error::ContentError};

/// How many hints every challenge has (parent spec §5.5).
pub const HINT_COUNT: usize = 3;

/// A track, ready to play.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Track {
    /// The folder's name, such as "vinyl-vault".
    pub slug: String,
    /// Its title, such as "Vinyl Vault".
    pub title: String,
    /// Its emoji.
    pub emoji: String,
    /// The story that opens it.
    pub intro: String,
    /// The concepts it teaches.
    pub concepts: Vec<String>,
    /// Its level, such as "Beginner".
    pub difficulty: String,
    /// Where it sits among the tracks; lower comes first.
    pub order: u32,
    /// The DDL that creates its database.
    pub schema: String,
    /// The script that fills its database.
    pub seed: String,
    /// Its challenges, in play order.
    pub challenges: Vec<Challenge>,
}

/// One challenge in a track.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Challenge {
    /// Its id within the track, such as "first-spin".
    pub id: String,
    /// Its title.
    pub title: String,
    /// The story prompt that says what to find.
    pub prompt: String,
    /// The id of its `ChallengeKind`, such as "query".
    pub kind: String,
    /// Whether the rows must come back in the solution's order.
    pub ordered: bool,
    /// The concepts it practises.
    pub concepts: Vec<String>,
    /// The reference solution.
    pub solution: String,
    /// The three hints, from a nudge to a skeleton with blanks.
    pub hints: Vec<String>,
    /// The editor's text when the challenge is first opened.
    pub skeleton: String,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct TrackFile {
    title: String,
    emoji: String,
    intro: String,
    concepts: Vec<String>,
    difficulty: String,
    order: u32,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct ChallengesFile {
    challenge: Vec<Entry>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct Entry {
    id: String,
    title: String,
    prompt: String,
    kind: String,
    #[serde(default)]
    ordered: bool,
    #[serde(default)]
    concepts: Vec<String>,
    solution: String,
    hints: Vec<String>,
    #[serde(default)]
    skeleton: String,
    verify: Option<String>,
}

/// Parses and checks the track in `raw`.
///
/// # Errors
/// Returns the first [`ContentError`] found, naming its file.
pub fn parse(raw: &RawTrack) -> Result<Track, ContentError> {
    let info: TrackFile = from_toml(raw.slug, "track.toml", raw.track)?;
    let file: ChallengesFile = from_toml(raw.slug, "challenges.toml", raw.challenges)?;
    Ok(Track {
        slug: raw.slug.to_owned(),
        title: info.title,
        emoji: info.emoji,
        intro: info.intro,
        concepts: info.concepts,
        difficulty: info.difficulty,
        order: info.order,
        schema: raw.schema.to_owned(),
        seed: raw.seed.to_owned(),
        challenges: challenges(raw.slug, file.challenge)?,
    })
}

/// `text`, the file `name` of track `slug`, as a `T`.
fn from_toml<T: DeserializeOwned>(slug: &str, name: &str, text: &str) -> Result<T, ContentError> {
    toml::from_str(text).map_err(|error| ContentError::Parse {
        file: format!("{slug}/{name}"),
        message: error.message().to_owned(),
    })
}

/// The checked challenges of track `slug`, in file order.
fn challenges(slug: &str, entries: Vec<Entry>) -> Result<Vec<Challenge>, ContentError> {
    let file = format!("{slug}/challenges.toml");
    let mut checked: Vec<Challenge> = Vec::with_capacity(entries.len());
    for entry in entries {
        check(&file, &entry)?;
        if checked.iter().any(|challenge| challenge.id == entry.id) {
            return Err(ContentError::Duplicate {
                file,
                challenge: entry.id,
            });
        }
        checked.push(entry.into_challenge());
    }
    Ok(checked)
}

/// The first field of `entry` that is not allowed, as an error.
fn check(file: &str, entry: &Entry) -> Result<(), ContentError> {
    let problem = if !is_id(&entry.id) {
        Some(("id", "must be lowercase letters, digits and hyphens"))
    } else if entry.solution.trim().is_empty() {
        Some(("solution", "must not be empty"))
    } else if entry.hints.len() != HINT_COUNT {
        Some(("hints", "must hold exactly 3 hints"))
    } else if entry.verify.is_some() {
        Some((
            "verify",
            "is only for mutation challenges, which aren't supported yet",
        ))
    } else {
        None
    };
    problem.map_or(Ok(()), |(field, problem)| {
        Err(ContentError::Field {
            file: file.to_owned(),
            challenge: entry.id.clone(),
            field,
            problem,
        })
    })
}

/// Whether `id` is lowercase letters, digits and hyphens, and not empty.
fn is_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .bytes()
            .all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-')
}

impl Entry {
    fn into_challenge(self) -> Challenge {
        Challenge {
            id: self.id,
            title: self.title,
            prompt: self.prompt,
            kind: self.kind,
            ordered: self.ordered,
            concepts: self.concepts,
            solution: self.solution,
            hints: self.hints,
            skeleton: self.skeleton,
        }
    }
}

#[cfg(test)]
mod tests;
