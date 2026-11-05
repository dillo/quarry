//! Requires every crate to opt in to the workspace lint table.

use serde::Deserialize;

use crate::violation::Violation;

#[derive(Deserialize)]
struct Manifest {
    lints: Option<LintsTable>,
}

#[derive(Deserialize)]
struct LintsTable {
    workspace: Option<bool>,
}

/// Checks `(path, contents)` pairs; flags manifests without `[lints] workspace = true`.
pub fn check(manifests: &[(String, String)]) -> Vec<Violation> {
    manifests
        .iter()
        .filter(|(_, source)| !inherits_workspace_lints(source))
        .map(|(path, _)| {
            Violation::new(
                path,
                "must contain `[lints] workspace = true` so the workspace lints apply",
            )
        })
        .collect()
}

fn inherits_workspace_lints(source: &str) -> bool {
    toml::from_str::<Manifest>(source)
        .ok()
        .and_then(|manifest| manifest.lints)
        .and_then(|lints| lints.workspace)
        == Some(true)
}

#[cfg(test)]
mod tests {
    use super::check;

    fn one(source: &str) -> Vec<(String, String)> {
        vec![("crates/engine/Cargo.toml".to_owned(), source.to_owned())]
    }

    #[test]
    fn accepts_manifest_inheriting_workspace_lints() {
        assert!(
            check(&one(
                "[package]\nname = \"a\"\n\n[lints]\nworkspace = true\n"
            ))
            .is_empty()
        );
    }

    #[test]
    fn flags_manifest_without_lints_table() {
        let found = check(&one("[package]\nname = \"a\"\n"));
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, "crates/engine/Cargo.toml");
    }

    #[test]
    fn flags_manifest_opting_out_or_invalid() {
        assert_eq!(check(&one("[lints]\nworkspace = false\n")).len(), 1);
        assert_eq!(check(&one("not = [valid")).len(), 1);
    }
}
