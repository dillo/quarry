//! Per-crate line and region coverage thresholds.

use std::{
    collections::BTreeMap,
    path::{Component, Path},
};

use serde::Deserialize;

use crate::{error::XtaskError, thresholds::CoverageTarget, violation::Violation};

#[derive(Deserialize)]
struct Report {
    data: Vec<Export>,
}

#[derive(Deserialize)]
struct Export {
    files: Vec<FileEntry>,
}

#[derive(Deserialize)]
struct FileEntry {
    filename: String,
    summary: Summary,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
struct Summary {
    lines: Counts,
    regions: Counts,
}

#[derive(Debug, Default, Clone, Copy, Deserialize)]
struct Counts {
    count: u32,
    covered: u32,
}

/// Checks an `llvm-cov` JSON summary report; `targets` is keyed by the crate's
/// directory name (`engine`, `ui-kit`, ..., `xtask`).
///
/// # Errors
/// Returns an error when the report is not valid JSON of the expected shape.
pub fn check_report(
    json: &str,
    root: &Path,
    targets: &BTreeMap<String, CoverageTarget>,
) -> Result<Vec<Violation>, XtaskError> {
    let report: Report = serde_json::from_str(json).map_err(|source| XtaskError::Json {
        what: "llvm-cov report".to_owned(),
        source,
    })?;
    let mut totals: BTreeMap<String, Summary> = BTreeMap::new();
    let mut violations = Vec::new();
    for file in report.data.iter().flat_map(|export| &export.files) {
        match crate_of(&file.filename, root).filter(|name| targets.contains_key(name)) {
            Some(name) => add(totals.entry(name).or_default(), file.summary),
            None => violations.push(Violation::new(
                &file.filename,
                "file belongs to no crate with a coverage target",
            )),
        }
    }
    for (name, summary) in &totals {
        if let Some(target) = targets.get(name) {
            violations.extend(judge(name, *summary, *target));
        }
    }
    Ok(violations)
}

fn crate_of(filename: &str, root: &Path) -> Option<String> {
    let relative = Path::new(filename).strip_prefix(root).ok()?;
    let mut parts = relative.components().filter_map(|part| match part {
        Component::Normal(name) => Some(name.to_string_lossy().into_owned()),
        _ => None,
    });
    match parts.next()?.as_str() {
        "crates" => parts.next(),
        "xtask" => Some("xtask".to_owned()),
        _ => None,
    }
}

fn add(total: &mut Summary, file: Summary) {
    total.lines.count = total.lines.count.saturating_add(file.lines.count);
    total.lines.covered = total.lines.covered.saturating_add(file.lines.covered);
    total.regions.count = total.regions.count.saturating_add(file.regions.count);
    total.regions.covered = total.regions.covered.saturating_add(file.regions.covered);
}

fn judge(name: &str, summary: Summary, target: CoverageTarget) -> Vec<Violation> {
    [
        ("line", summary.lines, target.lines),
        ("region", summary.regions, target.regions),
    ]
    .into_iter()
    .filter_map(|(metric, counts, min)| {
        percent(counts)
            .filter(|actual| *actual < min)
            .map(|actual| {
                Violation::new(
                    name,
                    format!("{metric} coverage {actual:.1}% is below {min}%"),
                )
            })
    })
    .collect()
}

fn percent(counts: Counts) -> Option<f64> {
    (counts.count > 0).then(|| f64::from(counts.covered) * 100.0 / f64::from(counts.count))
}

#[cfg(test)]
mod tests {
    use std::{collections::BTreeMap, path::Path};

    use super::check_report;
    use crate::thresholds::CoverageTarget;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn targets() -> BTreeMap<String, CoverageTarget> {
        BTreeMap::from([
            (
                "engine".to_owned(),
                CoverageTarget {
                    lines: 95.0,
                    regions: 90.0,
                },
            ),
            (
                "xtask".to_owned(),
                CoverageTarget {
                    lines: 90.0,
                    regions: 85.0,
                },
            ),
        ])
    }

    fn file(name: &str, lines: (u32, u32), regions: (u32, u32)) -> String {
        format!(
            r#"{{"filename":"{name}","summary":{{"lines":{{"count":{},"covered":{}}},"regions":{{"count":{},"covered":{}}}}}}}"#,
            lines.0, lines.1, regions.0, regions.1
        )
    }

    fn report(files: &[String]) -> String {
        format!(r#"{{"data":[{{"files":[{}]}}]}}"#, files.join(","))
    }

    #[test]
    fn passes_when_every_crate_meets_its_target() -> TestResult {
        let json = report(&[file("/w/crates/engine/src/a.rs", (100, 96), (10, 9))]);
        assert!(check_report(&json, Path::new("/w"), &targets())?.is_empty());
        Ok(())
    }

    #[test]
    fn aggregates_files_per_crate_before_judging() -> TestResult {
        let json = report(&[
            file("/w/crates/engine/src/a.rs", (50, 50), (10, 10)),
            file("/w/crates/engine/src/b.rs", (50, 40), (10, 10)),
        ]);
        let found = check_report(&json, Path::new("/w"), &targets())?;
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].location, "engine");
        assert_eq!(found[0].message, "line coverage 90.0% is below 95%");
        Ok(())
    }

    #[test]
    fn crates_without_code_pass() -> TestResult {
        let json = report(&[file("/w/xtask/src/lib.rs", (0, 0), (0, 0))]);
        assert!(check_report(&json, Path::new("/w"), &targets())?.is_empty());
        Ok(())
    }

    #[test]
    fn flags_files_outside_any_target() -> TestResult {
        let json = report(&[
            file("/w/crates/mystery/src/a.rs", (1, 1), (1, 1)),
            file("/elsewhere/x.rs", (1, 1), (1, 1)),
        ]);
        assert_eq!(check_report(&json, Path::new("/w"), &targets())?.len(), 2);
        Ok(())
    }

    #[test]
    fn rejects_malformed_report() {
        assert!(check_report("[]", Path::new("/w"), &targets()).is_err());
    }
}
