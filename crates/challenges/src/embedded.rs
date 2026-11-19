//! The track folders under `content/tracks/`, embedded at compile time by
//! the crate's build script.

/// One track folder's files, as text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawTrack {
    /// The folder's name, which is the track's slug.
    pub slug: &'static str,
    /// `track.toml`.
    pub track: &'static str,
    /// `schema.sql`.
    pub schema: &'static str,
    /// `seed.sql`.
    pub seed: &'static str,
    /// `challenges.toml`.
    pub challenges: &'static str,
}

/// Every embedded track folder, in folder-name order.
pub static TRACKS: &[RawTrack] = include!(concat!(env!("OUT_DIR"), "/tracks.rs"));
