//! Local persistence: settings, connections, history, and progress.
//!
//! It stores the query tabs' drafts and the recently closed tabs. Each kind
//! of data gets a trait ([`drafts::DraftStore`]) so the app can swap in a
//! fake or a fallback.

pub mod drafts;
pub mod error;
mod migrations;
pub mod sqlite;
