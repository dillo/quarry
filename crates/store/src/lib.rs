//! Local persistence: settings, connections, history, and progress.
//!
//! Phase 1D stores the query tabs' drafts. Each kind of data gets a trait
//! ([`drafts::DraftStore`]) so the app can swap in a fake or a fallback.

pub mod drafts;
pub mod error;
mod migrations;
pub mod sqlite;
