//! Enforces the one-way dependency direction between workspace crates.

use serde::Deserialize;

use crate::{error::XtaskError, violation::Violation};

/// For each workspace package, the workspace packages it may depend on.
/// Adding a crate means adding a rule here (design spec §3.2).
pub const RULES: &[(&str, &[&str])] = &[
    (
        "quarry",
        &[
            "quarry-ui-kit",
            "quarry-challenges",
            "quarry-store",
            "quarry-engine",
        ],
    ),
    ("quarry-ui-kit", &[]),
    ("quarry-challenges", &["quarry-engine"]),
    ("quarry-store", &["quarry-engine"]),
    ("quarry-engine", &[]),
    ("xtask", &[]),
];

#[derive(Deserialize)]
struct Metadata {
    packages: Vec<Package>,
}

#[derive(Deserialize)]
struct Package {
    name: String,
    dependencies: Vec<Dependency>,
}

#[derive(Deserialize)]
struct Dependency {
    name: String,
    path: Option<String>,
}

/// Checks `cargo metadata --format-version 1 --no-deps` output against [`RULES`].
///
/// # Errors
/// Returns an error when the metadata is not valid JSON of the expected shape.
pub fn check_metadata(json: &str) -> Result<Vec<Violation>, XtaskError> {
    let metadata: Metadata = serde_json::from_str(json).map_err(|source| XtaskError::Json {
        what: "cargo metadata".to_owned(),
        source,
    })?;
    Ok(metadata.packages.iter().flat_map(check_package).collect())
}

fn check_package(package: &Package) -> Vec<Violation> {
    let Some(allowed) = allowed_for(&package.name) else {
        return vec![Violation::new(
            &package.name,
            "no dependency rule; add one to RULES in xtask/src/checks/deps.rs",
        )];
    };
    package
        .dependencies
        .iter()
        .filter(|dep| dep.path.is_some() && !allowed.contains(&dep.name.as_str()))
        .map(|dep| Violation::new(&package.name, format!("must not depend on {}", dep.name)))
        .collect()
}

fn allowed_for(name: &str) -> Option<&'static [&'static str]> {
    RULES
        .iter()
        .find(|(package, _)| *package == name)
        .map(|(_, allowed)| *allowed)
}

#[cfg(test)]
mod tests {
    use super::check_metadata;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn metadata(package: &str, dep: &str, path: Option<&str>) -> String {
        let path = path.map_or("null".to_owned(), |p| format!("\"{p}\""));
        format!(
            r#"{{"packages":[{{"name":"{package}","dependencies":[{{"name":"{dep}","path":{path}}}]}}]}}"#
        )
    }

    #[test]
    fn allows_declared_direction() -> TestResult {
        let json = metadata("quarry-store", "quarry-engine", Some("/w/crates/engine"));
        assert!(check_metadata(&json)?.is_empty());
        Ok(())
    }

    #[test]
    fn flags_reversed_direction() -> TestResult {
        let json = metadata("quarry-engine", "quarry-ui-kit", Some("/w/crates/ui-kit"));
        let found = check_metadata(&json)?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].message, "must not depend on quarry-ui-kit");
        Ok(())
    }

    #[test]
    fn ignores_registry_dependencies() -> TestResult {
        assert!(check_metadata(&metadata("quarry-engine", "serde", None))?.is_empty());
        Ok(())
    }

    #[test]
    fn flags_package_without_rule() -> TestResult {
        let found = check_metadata(&metadata("quarry-new", "serde", None))?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, "quarry-new");
        Ok(())
    }

    #[test]
    fn rejects_malformed_metadata() {
        assert!(check_metadata("{").is_err());
    }
}
