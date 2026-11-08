//! Maps connection strings to drivers: the driver extension point.

use crate::{driver::Driver, error::EngineError, sqlite};

/// Opens a driver for the part of a connection string after `scheme:`.
pub type Connect = fn(&str) -> Result<Box<dyn Driver>, EngineError>;

/// The drivers Quarry knows, keyed by connection-string scheme.
///
/// Adding a database means adding one entry in [`DriverRegistry::builtin`].
pub struct DriverRegistry {
    schemes: Vec<(&'static str, Connect)>,
}

impl DriverRegistry {
    /// Every built-in driver.
    pub fn builtin() -> Self {
        Self {
            schemes: vec![("sqlite", sqlite::connect)],
        }
    }

    /// The registered schemes, in registration order.
    pub fn schemes(&self) -> impl Iterator<Item = &'static str> + '_ {
        self.schemes.iter().map(|(scheme, _)| *scheme)
    }

    /// Opens `url`, e.g. `sqlite::memory:` or `sqlite:/path/to/file.db`.
    ///
    /// # Errors
    /// Returns [`EngineError::Unsupported`] for an unknown scheme, or the
    /// driver's own error when opening fails.
    pub fn connect(&self, url: &str) -> Result<Box<dyn Driver>, EngineError> {
        let unsupported = || EngineError::Unsupported(url.to_owned());
        let (scheme, target) = url.split_once(':').ok_or_else(unsupported)?;
        let (_, connect) = self
            .schemes
            .iter()
            .find(|(known, _)| *known == scheme)
            .ok_or_else(unsupported)?;
        connect(target)
    }
}
