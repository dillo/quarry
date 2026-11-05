//! Rejects catch-all module names, which hide more than one responsibility.

use std::path::{Path, PathBuf};

use crate::{files, violation::Violation};

/// Module names that never describe a single responsibility.
pub const FORBIDDEN: [&str; 4] = ["utils", "helpers", "misc", "common"];

/// Returns a violation for every path whose file or directory name is forbidden.
pub fn check_paths(paths: &[PathBuf]) -> Vec<Violation> {
    paths
        .iter()
        .filter(|path| has_forbidden_part(path))
        .map(|path| {
            Violation::new(
                files::display(path),
                "catch-all module name; name the module after its single responsibility",
            )
        })
        .collect()
}

fn has_forbidden_part(path: &Path) -> bool {
    path.iter().any(|part| {
        let part = part.to_string_lossy();
        let stem = part.strip_suffix(".rs").unwrap_or(&part);
        FORBIDDEN.contains(&stem)
    })
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::check_paths;

    fn paths(list: &[&str]) -> Vec<PathBuf> {
        list.iter().map(PathBuf::from).collect()
    }

    #[test]
    fn flags_forbidden_file_names() {
        let found = check_paths(&paths(&["crates/engine/src/utils.rs", "xtask/src/misc.rs"]));
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].location, "crates/engine/src/utils.rs");
    }

    #[test]
    fn flags_forbidden_directory_names() {
        let found = check_paths(&paths(&["crates/store/src/helpers/mod.rs"]));
        assert_eq!(found.len(), 1);
    }

    #[test]
    fn accepts_names_that_merely_contain_a_forbidden_word() {
        let found = check_paths(&paths(&[
            "crates/engine/src/common_table.rs",
            "src/value.rs",
        ]));
        assert!(found.is_empty());
    }
}
