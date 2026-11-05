//! Requires every crate to opt in to the workspace lint table.

use serde::Deserialize;

use crate::{error::XtaskError, violation::Violation};

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

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    manifest_path: String,
}

/// Flags workspace members whose manifest is outside every directory in
/// `roots`, since the other gates only scan those. `json` is
/// `cargo metadata --format-version 1 --no-deps` output and `root` the
/// workspace root.
///
/// # Errors
/// Returns an error when the metadata is not valid JSON of the expected shape.
pub fn check_members(json: &str, root: &str, roots: &[&str]) -> Result<Vec<Violation>, XtaskError> {
    let metadata: Metadata = serde_json::from_str(json).map_err(|source| XtaskError::Json {
        what: "cargo metadata".to_owned(),
        source,
    })?;
    let listed = roots.join("/ or ") + "/";
    let prefix = format!("{}/", root.trim_end_matches('/'));
    Ok(metadata
        .packages
        .iter()
        .map(|package| package.manifest_path.replace('\\', "/"))
        .map(|path| path.strip_prefix(&prefix).unwrap_or(&path).to_owned())
        .filter(|path| !roots.iter().any(|dir| path.starts_with(&format!("{dir}/"))))
        .map(|path| {
            Violation::new(
                path,
                format!("workspace member outside {listed}; the quality gates do not scan it"),
            )
        })
        .collect())
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
    use super::{check, check_members};

    const ROOTS: [&str; 2] = ["crates", "xtask"];

    fn metadata(paths: &[&str]) -> String {
        let packages: Vec<String> = paths
            .iter()
            .map(|path| format!("{{\"manifest_path\":\"{path}\"}}"))
            .collect();
        format!("{{\"packages\":[{}]}}", packages.join(","))
    }

    #[test]
    fn accepts_members_inside_the_known_roots() {
        let json = metadata(&["/w/crates/a/Cargo.toml", "/w/xtask/Cargo.toml"]);
        assert!(matches!(check_members(&json, "/w", &ROOTS), Ok(found) if found.is_empty()));
    }

    #[test]
    fn flags_members_outside_the_known_roots() {
        let json = metadata(&["/w/crates/a/Cargo.toml", "/w/tools/foo/Cargo.toml"]);
        let found = check_members(&json, "/w", &ROOTS).unwrap_or_default();
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, "tools/foo/Cargo.toml");
        assert!(found[0].message.contains("outside crates/ or xtask/;"));
    }

    #[test]
    fn a_root_prefix_is_not_a_root_match() {
        let json = metadata(&["/w/crates-extra/a/Cargo.toml", "/w/Cargo.toml"]);
        let found = check_members(&json, "/w", &ROOTS).unwrap_or_default();
        assert_eq!(found.len(), 2);
    }

    #[test]
    fn rejects_invalid_metadata() {
        assert!(check_members("nope", "/w", &ROOTS).is_err());
    }

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
