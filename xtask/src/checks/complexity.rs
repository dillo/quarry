//! Per-function size and complexity limits, measured by `rust-code-analysis-cli`.

use std::path::Path;

use serde::Deserialize;

use crate::{
    checks::exceptions::{COMPLEXITY_LINT, Exception},
    error::XtaskError,
    thresholds::Limits,
    violation::Violation,
};

#[derive(Deserialize)]
struct Space {
    kind: String,
    name: Option<String>,
    start_line: u64,
    metrics: Metrics,
    #[serde(default)]
    spaces: Vec<Space>,
}

#[derive(Deserialize)]
struct Metrics {
    cyclomatic: Sum,
    cognitive: Sum,
    loc: Loc,
}

#[derive(Deserialize)]
struct Sum {
    sum: f64,
}

#[derive(Deserialize)]
struct Loc {
    lloc: f64,
    ploc: f64,
}

/// Checks a report (one JSON object per file) against `limits`.
/// Functions waived in `waivers` with [`COMPLEXITY_LINT`] are skipped.
///
/// # Errors
/// Returns an error when the report is not valid JSON of the expected shape.
pub fn check_report(
    report: &str,
    root: &Path,
    limits: &Limits,
    waivers: &[Exception],
) -> Result<Vec<Violation>, XtaskError> {
    let mut violations = Vec::new();
    for unit in parse_units(report)? {
        let file = relative_name(unit.name.as_deref().unwrap_or_default(), root);
        if Path::new(&file).extension().is_none_or(|ext| ext != "rs") {
            continue;
        }
        let mut functions = Vec::new();
        collect_functions(&unit, &mut functions);
        for function in functions
            .into_iter()
            .filter(|f| !is_waived(&file, f, waivers))
        {
            violations.extend(measure(&file, function, limits));
        }
    }
    Ok(violations)
}

fn parse_units(report: &str) -> Result<Vec<Space>, XtaskError> {
    serde_json::Deserializer::from_str(report)
        .into_iter::<Space>()
        .collect::<Result<_, _>>()
        .map_err(|source| XtaskError::Json {
            what: "rust-code-analysis-cli".to_owned(),
            source,
        })
}

fn relative_name(name: &str, root: &Path) -> String {
    let path = Path::new(name);
    let relative = path.strip_prefix(root).unwrap_or(path);
    relative.to_string_lossy().replace('\\', "/")
}

fn collect_functions<'a>(space: &'a Space, out: &mut Vec<&'a Space>) {
    for child in &space.spaces {
        if child.kind == "function" {
            out.push(child);
        }
        collect_functions(child, out);
    }
}

fn is_waived(file: &str, function: &Space, waivers: &[Exception]) -> bool {
    waivers.iter().any(|w| {
        w.lint == COMPLEXITY_LINT
            && w.file == file
            && w.function.as_deref() == function.name.as_deref()
    })
}

fn measure(file: &str, function: &Space, limits: &Limits) -> Vec<Violation> {
    let m = &function.metrics;
    let name = function.name.as_deref().unwrap_or("<anonymous>");
    let location = format!("{file}:{} fn {name}", function.start_line);
    [
        ("cyclomatic complexity", m.cyclomatic.sum, limits.cyclomatic),
        ("cognitive complexity", m.cognitive.sum, limits.cognitive),
        ("logical lines", m.loc.lloc, limits.logical_lines),
        ("physical lines", m.loc.ploc, limits.physical_lines),
    ]
    .into_iter()
    .filter(|(_, value, max)| value > max)
    .map(|(metric, value, max)| {
        Violation::new(&location, format!("{metric} {value} exceeds {max}"))
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::check_report;
    use crate::{
        checks::exceptions::{COMPLEXITY_LINT, Exception},
        thresholds::Limits,
    };

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    const LIMITS: Limits = Limits {
        cyclomatic: 10.0,
        cognitive: 15.0,
        logical_lines: 15.0,
        physical_lines: 40.0,
    };

    fn function(name: &str, cyclomatic: f64, lloc: f64) -> String {
        format!(
            r#"{{"kind":"function","name":"{name}","start_line":3,"metrics":{{"cyclomatic":{{"sum":{cyclomatic}}},"cognitive":{{"sum":1}},"loc":{{"lloc":{lloc},"ploc":5}}}},"spaces":[]}}"#
        )
    }

    fn unit(file: &str, children: &str) -> String {
        format!(
            r#"{{"kind":"unit","name":"{file}","start_line":1,"metrics":{{"cyclomatic":{{"sum":1}},"cognitive":{{"sum":0}},"loc":{{"lloc":0,"ploc":0}}}},"spaces":[{children}]}}"#
        )
    }

    #[test]
    fn accepts_functions_within_limits() -> TestResult {
        let report = unit("/w/crates/a/src/lib.rs", &function("ok", 2.0, 4.0));
        assert!(check_report(&report, Path::new("/w"), &LIMITS, &[])?.is_empty());
        Ok(())
    }

    #[test]
    fn reports_each_metric_over_its_limit() -> TestResult {
        let report = unit("/w/crates/a/src/lib.rs", &function("big", 11.0, 16.0));
        let found = check_report(&report, Path::new("/w"), &LIMITS, &[])?;
        assert_eq!(found.len(), 2);
        assert_eq!(found[0].location, "crates/a/src/lib.rs:3 fn big");
        assert_eq!(found[0].message, "cyclomatic complexity 11 exceeds 10");
        Ok(())
    }

    #[test]
    fn finds_functions_nested_in_impls_across_files() -> TestResult {
        let nested = unit("a.rs", &unit("impl", &function("deep", 12.0, 1.0)));
        let report = format!("{nested}\n{}", unit("b.rs", &function("too", 12.0, 1.0)));
        assert_eq!(
            check_report(&report, Path::new("/w"), &LIMITS, &[])?.len(),
            2
        );
        Ok(())
    }

    #[test]
    fn skips_waived_functions_and_non_rust_units() -> TestResult {
        let waiver = Exception {
            file: "a.rs".to_owned(),
            lint: COMPLEXITY_LINT.to_owned(),
            function: Some("big".to_owned()),
            reason: "lookup table".to_owned(),
        };
        let report = format!(
            "{}\n{}",
            unit("a.rs", &function("big", 30.0, 1.0)),
            unit("a.py", &function("x", 30.0, 1.0))
        );
        assert!(check_report(&report, Path::new("/w"), &LIMITS, &[waiver])?.is_empty());
        Ok(())
    }

    #[test]
    fn rejects_malformed_report() {
        assert!(check_report("{", Path::new("/w"), &LIMITS, &[]).is_err());
    }
}
