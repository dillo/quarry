//! Loads `quality/thresholds.toml`, the single source of every numeric gate.

use std::{collections::BTreeMap, path::Path};

use serde::Deserialize;

use crate::{error::XtaskError, files};

/// Location of the thresholds file, relative to the workspace root.
pub const PATH: &str = "quality/thresholds.toml";

/// Maximum allowed value of each metric, per function.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct Limits {
    /// Cyclomatic complexity (independent paths).
    pub cyclomatic: f64,
    /// Cognitive complexity (how hard it is to follow).
    pub cognitive: f64,
    /// Logical lines, i.e. statements.
    pub logical_lines: f64,
    /// Physical lines that contain code.
    pub physical_lines: f64,
}

/// Minimum coverage percentages for one crate.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct CoverageTarget {
    /// Minimum line coverage, in percent.
    pub lines: f64,
    /// Minimum region coverage, in percent.
    pub regions: f64,
}

/// The minimum share of mutants the tests must catch.
#[derive(Debug, Clone, Copy, PartialEq, Deserialize)]
pub struct MutationTarget {
    /// Minimum score, in percent: (caught + timeout) / (caught + timeout + missed).
    pub min_score: f64,
}

/// Every numeric quality threshold.
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct Thresholds {
    /// Per-function size and complexity limits.
    pub complexity: Limits,
    /// Coverage targets keyed by crate directory name.
    pub coverage: BTreeMap<String, CoverageTarget>,
    /// Mutation testing target.
    pub mutation: MutationTarget,
}

impl Thresholds {
    /// Parses thresholds TOML; `origin` names the source in errors.
    ///
    /// # Errors
    /// Returns an error when the TOML is invalid or a section is missing.
    pub fn parse(source: &str, origin: &str) -> Result<Self, XtaskError> {
        toml::from_str(source).map_err(|source| XtaskError::Toml {
            path: origin.to_owned(),
            source,
        })
    }

    /// Loads the thresholds file of the workspace at `root`.
    ///
    /// # Errors
    /// Returns an error when the file is missing or invalid.
    pub fn load(root: &Path) -> Result<Self, XtaskError> {
        let path = root.join(PATH);
        Self::parse(&files::read(&path)?, &files::display(&path))
    }
}
