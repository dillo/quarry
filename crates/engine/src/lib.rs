//! Database layer: the `Driver` trait, the shared value model, and drivers.
//!
//! The UI talks to a database only through a `session::Session`, which runs
//! a `driver::Driver` on its own thread and streams results back as events.

pub mod driver;
pub mod error;
pub mod result;
pub mod schema;
pub mod splitter;
pub mod value;
