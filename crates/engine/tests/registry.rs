//! Integration tests for connection strings and the driver registry.

use quarry_engine::{error::EngineError, registry::DriverRegistry, result::QueryResult};

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[test]
fn builtin_registry_knows_sqlite() {
    assert_eq!(
        DriverRegistry::builtin().schemes().collect::<Vec<_>>(),
        ["sqlite"]
    );
}

#[test]
fn connects_to_memory_and_file_databases() -> TestResult {
    let registry = DriverRegistry::builtin();
    let mut memory = registry.connect("sqlite::memory:")?;
    assert_eq!(
        memory
            .execute("SELECT 1", &mut QueryResult::default())?
            .rows,
        1
    );
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("f.db");
    let url = format!("sqlite:{}", path.display());
    registry
        .connect(&url)?
        .execute("CREATE TABLE t (x)", &mut QueryResult::default())?;
    assert!(path.exists(), "a file URL must create the file");
    assert_eq!(
        registry.connect(&url)?.schema()?.tables.len(),
        1,
        "the table must persist"
    );
    Ok(())
}

#[test]
fn rejects_unknown_or_malformed_urls() {
    let registry = DriverRegistry::builtin();
    for url in ["postgres://localhost/db", "no-scheme-here"] {
        let error = registry.connect(url).err();
        assert_eq!(
            error,
            Some(EngineError::Unsupported(url.to_owned())),
            "{url}"
        );
    }
}
