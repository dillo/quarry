//! Every track that loaded, and finding a challenge by its full id.

use std::fmt;

use crate::{
    embedded::{RawTrack, TRACKS},
    error::ContentError,
    track::{self, Challenge, Track},
};

/// A challenge's full id: its track's slug and its own id, written
/// `vinyl-vault/first-spin`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ChallengeId {
    track: String,
    challenge: String,
}

impl ChallengeId {
    /// The id of challenge `challenge` in track `track`.
    pub fn new(track: &str, challenge: &str) -> Self {
        Self {
            track: track.to_owned(),
            challenge: challenge.to_owned(),
        }
    }

    /// Reads `track/challenge`; anything else is `None`.
    pub fn parse(text: &str) -> Option<Self> {
        let (track, challenge) = text.split_once('/')?;
        let ok = |part: &str| !part.is_empty() && !part.contains('/');
        (ok(track) && ok(challenge)).then(|| Self::new(track, challenge))
    }

    /// The track's slug.
    pub fn track(&self) -> &str {
        &self.track
    }

    /// The challenge's id within its track.
    pub fn challenge(&self) -> &str {
        &self.challenge
    }
}

impl fmt::Display for ChallengeId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}/{}", self.track, self.challenge)
    }
}

/// The tracks that loaded, by `order`.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Catalog {
    tracks: Vec<Track>,
}

impl Catalog {
    /// Loads every track in `raw`. A track with bad content is left out,
    /// and its error is returned beside the catalog.
    pub fn load(raw: &[RawTrack]) -> (Self, Vec<ContentError>) {
        let mut tracks = Vec::new();
        let mut errors = Vec::new();
        for raw in raw {
            match track::parse(raw) {
                Ok(track) => tracks.push(track),
                Err(error) => errors.push(error),
            }
        }
        tracks.sort_by_key(|track| track.order);
        (Self { tracks }, errors)
    }

    /// Loads the tracks embedded in the binary.
    pub fn embedded() -> (Self, Vec<ContentError>) {
        Self::load(TRACKS)
    }

    /// Every track, by `order`.
    pub fn tracks(&self) -> &[Track] {
        &self.tracks
    }

    /// The track with slug `slug`.
    pub fn track(&self, slug: &str) -> Option<&Track> {
        self.tracks.iter().find(|track| track.slug == slug)
    }

    /// The challenge `id` and its track.
    pub fn challenge(&self, id: &ChallengeId) -> Option<(&Track, &Challenge)> {
        let track = self.track(id.track())?;
        let challenge = track
            .challenges
            .iter()
            .find(|challenge| challenge.id == id.challenge())?;
        Some((track, challenge))
    }
}

#[cfg(test)]
mod tests {
    use super::{Catalog, ChallengeId};
    use crate::{embedded::RawTrack, error::ContentError};

    fn raw(slug: &'static str, order: u32, challenge: &'static str) -> RawTrack {
        let track: &'static str = Box::leak(
            format!("title = \"{slug}\"\nemoji = \"E\"\nintro = \"I\"\nconcepts = []\ndifficulty = \"D\"\norder = {order}")
                .into_boxed_str(),
        );
        let challenges: &'static str = Box::leak(
            format!("[[challenge]]\nid = \"{challenge}\"\ntitle = \"T\"\nprompt = \"P\"\nkind = \"query\"\nsolution = \"S\"\nhints = [\"1\", \"2\", \"3\"]")
                .into_boxed_str(),
        );
        RawTrack {
            slug,
            track,
            schema: "",
            seed: "",
            challenges,
        }
    }

    #[test]
    fn ids_read_and_print_as_track_slash_challenge() {
        let id = ChallengeId::parse("vinyl-vault/first-spin").unwrap();
        assert_eq!(id, ChallengeId::new("vinyl-vault", "first-spin"));
        assert_eq!(id.track(), "vinyl-vault");
        assert_eq!(id.challenge(), "first-spin");
        assert_eq!(id.to_string(), "vinyl-vault/first-spin");
        for bad in ["vinyl-vault", "/first-spin", "vinyl-vault/", "a/b/c", ""] {
            assert_eq!(ChallengeId::parse(bad), None, "{bad}");
        }
    }

    #[test]
    fn tracks_come_in_order_and_bad_ones_are_left_out() {
        let broken = RawTrack {
            slug: "broken",
            track: "title =",
            schema: "",
            seed: "",
            challenges: "",
        };
        let (catalog, errors) = Catalog::load(&[raw("late", 9, "a"), broken, raw("early", 1, "b")]);
        let slugs: Vec<&str> = catalog
            .tracks()
            .iter()
            .map(|track| track.slug.as_str())
            .collect();
        assert_eq!(slugs, ["early", "late"]);
        assert!(
            matches!(&errors[..], [ContentError::Parse { file, .. }] if file == "broken/track.toml")
        );
    }

    #[test]
    fn finds_a_challenge_and_its_track_by_id() {
        let (catalog, _) = Catalog::load(&[raw("one", 1, "a"), raw("two", 2, "b")]);
        let (track, challenge) = catalog.challenge(&ChallengeId::new("two", "b")).unwrap();
        assert_eq!((track.slug.as_str(), challenge.id.as_str()), ("two", "b"));
        assert_eq!(catalog.track("one").map(|track| track.order), Some(1));
        assert_eq!(catalog.track("three"), None);
        assert_eq!(catalog.challenge(&ChallengeId::new("two", "a")), None);
        assert_eq!(catalog.challenge(&ChallengeId::new("three", "b")), None);
    }

    #[test]
    fn nothing_embedded_is_broken() {
        let (_, errors) = Catalog::embedded();
        assert_eq!(errors, []);
    }
}
