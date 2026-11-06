//! Shared fixture helpers for the integration tests.

use std::{fs, path::Path};

/// Error type for fallible test helpers.
pub type TestError = Box<dyn std::error::Error>;

/// Writes `contents` to `root/relative`, creating parent directories.
///
/// # Errors
/// Returns an error when a directory or the file cannot be written.
pub fn write(root: &Path, relative: &str, contents: &str) -> Result<(), TestError> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, contents)?;
    Ok(())
}
