//! Finding the workspace files the gates inspect.

use std::path::{Path, PathBuf};

use walkdir::WalkDir;

use crate::error::XtaskError;

/// Top-level directories that hold workspace source code.
pub const SOURCE_ROOTS: [&str; 2] = ["crates", "xtask"];

/// Returns the source roots that exist under `root`, relative to it.
pub fn source_dirs(root: &Path) -> Vec<PathBuf> {
    SOURCE_ROOTS
        .iter()
        .map(PathBuf::from)
        .filter(|dir| root.join(dir).is_dir())
        .collect()
}

/// Lists every `.rs` file under the source roots, relative to `root`, sorted.
///
/// # Errors
/// Returns an error when a directory cannot be read.
pub fn rust_files(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    let mut found = Vec::new();
    for dir in source_dirs(root) {
        for entry in WalkDir::new(root.join(&dir)) {
            let entry = entry.map_err(|error| XtaskError::Walk {
                path: dir.display().to_string(),
                detail: error.to_string(),
            })?;
            if entry.path().extension().is_some_and(|ext| ext == "rs") {
                found.push(relative(root, entry.path()));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Lists `xtask/Cargo.toml` and every `crates/*/Cargo.toml`, relative to `root`.
///
/// # Errors
/// Returns an error when the `crates` directory cannot be read.
pub fn member_manifests(root: &Path) -> Result<Vec<PathBuf>, XtaskError> {
    let mut found = Vec::new();
    if root.join("xtask/Cargo.toml").is_file() {
        found.push(PathBuf::from("xtask/Cargo.toml"));
    }
    let crates = root.join("crates");
    if crates.is_dir() {
        for entry in std::fs::read_dir(&crates).map_err(|source| io_error(&crates, source))? {
            let manifest = entry
                .map_err(|source| io_error(&crates, source))?
                .path()
                .join("Cargo.toml");
            if manifest.is_file() {
                found.push(relative(root, &manifest));
            }
        }
    }
    found.sort();
    Ok(found)
}

/// Reads a whole file as UTF-8.
///
/// # Errors
/// Returns an error when the file is missing or unreadable.
pub fn read(path: &Path) -> Result<String, XtaskError> {
    std::fs::read_to_string(path).map_err(|source| io_error(path, source))
}

/// Renders a path with forward slashes, the form every gate reports.
pub fn display(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn relative(root: &Path, path: &Path) -> PathBuf {
    path.strip_prefix(root).unwrap_or(path).to_path_buf()
}

fn io_error(path: &Path, source: std::io::Error) -> XtaskError {
    XtaskError::Io {
        path: display(path),
        source,
    }
}
