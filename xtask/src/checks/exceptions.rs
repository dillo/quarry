//! Every lint suppression must be listed, with a reason, in
//! `quality/exceptions.toml`, so each one is visible in review.

use regex::Regex;
use serde::Deserialize;

use crate::{error::XtaskError, violation::Violation};

/// Registry lint name for a waiver of the `complexity` gate. These entries
/// have no source attribute, so they are never reported as stale.
pub const COMPLEXITY_LINT: &str = "quarry::complexity";

/// One registered, reviewed lint suppression.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
pub struct Exception {
    /// File the suppression lives in, relative to the workspace root.
    pub file: String,
    /// Full lint name, e.g. `clippy::too_many_lines`.
    pub lint: String,
    /// Function name; used only by complexity waivers.
    #[serde(default)]
    pub function: Option<String>,
    /// Why the suppression is justified.
    pub reason: String,
}

/// One lint suppression attribute found in source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Suppression {
    /// File it was found in, relative to the workspace root.
    pub file: String,
    /// 1-based line of the attribute.
    pub line: usize,
    /// The lint it suppresses.
    pub lint: String,
}

#[derive(Deserialize)]
struct Registry {
    #[serde(default)]
    exception: Vec<Exception>,
}

/// Parses the registry file contents; `origin` names the file in errors.
///
/// # Errors
/// Returns an error when the contents are not valid registry TOML.
pub fn parse_registry(source: &str, origin: &str) -> Result<Vec<Exception>, XtaskError> {
    toml::from_str::<Registry>(source)
        .map(|registry| registry.exception)
        .map_err(|source| XtaskError::Toml {
            path: origin.to_owned(),
            source,
        })
}

/// Finds lint suppression attributes in Rust source.
///
/// Every literal attribute (`#[...]` or `#![...]`) is scanned, and every
/// suppression inside it is reported, including each one inside a `cfg_attr`.
/// The scan is textual, so it can over-report: it also matches inside
/// comments and strings. It does not see attributes that macros generate.
pub struct Scanner {
    attribute: Regex,
    suppression: Regex,
}

impl Scanner {
    /// Builds the scanner.
    ///
    /// # Errors
    /// Returns an error only if a built-in pattern is invalid.
    pub fn new() -> Result<Self, XtaskError> {
        Ok(Self {
            attribute: Regex::new(r"#!?\[")?,
            suppression: Regex::new(r"\b(?:allow|expect)\s*\(([^)]*)\)")?,
        })
    }

    /// Returns every suppressed lint in `source`, which came from `file`.
    pub fn find(&self, file: &str, source: &str) -> Vec<Suppression> {
        let mut found = Vec::new();
        for start in self.attribute.find_iter(source) {
            let line = source[..start.start()].matches('\n').count() + 1;
            let span = bracketed_span(&source[start.start()..]);
            for captures in self.suppression.captures_iter(span) {
                let list = captures.get(1).map_or("", |m| m.as_str());
                found.extend(lint_names(list).into_iter().map(|lint| Suppression {
                    file: file.to_owned(),
                    line,
                    lint,
                }));
            }
        }
        found
    }
}

/// Returns the prefix of `text` (which starts at an attribute's `[`-opening
/// text) up to the bracket that balances the first `[`, or all of `text` when
/// it never balances.
fn bracketed_span(text: &str) -> &str {
    let mut depth = 0_usize;
    for (index, ch) in text.char_indices() {
        match ch {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    return &text[..=index];
                }
            }
            _ => {}
        }
    }
    text
}

fn lint_names(list: &str) -> Vec<String> {
    list.split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty() && !item.starts_with("reason"))
        .map(str::to_owned)
        .collect()
}

/// Reports unregistered suppressions, reasonless entries, and stale entries.
pub fn check(found: &[Suppression], registry: &[Exception]) -> Vec<Violation> {
    let mut violations: Vec<Violation> = found
        .iter()
        .filter(|s| {
            !registry
                .iter()
                .any(|e| e.file == s.file && e.lint == s.lint)
        })
        .map(|s| {
            Violation::new(
                format!("{}:{}", s.file, s.line),
                format!(
                    "{} is suppressed but not registered in quality/exceptions.toml",
                    s.lint
                ),
            )
        })
        .collect();
    violations.extend(registry.iter().filter_map(|e| registry_problem(e, found)));
    violations
}

fn registry_problem(entry: &Exception, found: &[Suppression]) -> Option<Violation> {
    if entry.reason.trim().is_empty() {
        return Some(Violation::new(
            &entry.file,
            format!("exception for {} has no reason", entry.lint),
        ));
    }
    let used = found
        .iter()
        .any(|s| s.file == entry.file && s.lint == entry.lint);
    (entry.lint != COMPLEXITY_LINT && !used).then(|| {
        Violation::new(
            &entry.file,
            format!(
                "exception for {} matches no suppression; remove it",
                entry.lint
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::{COMPLEXITY_LINT, Exception, Scanner, check, parse_registry};

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    /// Builds an attribute without writing it literally, so this file does not
    /// trip the scanner when the gate runs on the workspace itself.
    fn attr(inner: bool, kind: &str, lints: &str) -> String {
        let bang = if inner { "!" } else { "" };
        format!("#{bang}[{kind}({lints})]")
    }

    fn entry(file: &str, lint: &str, reason: &str) -> Exception {
        Exception {
            file: file.to_owned(),
            lint: lint.to_owned(),
            function: None,
            reason: reason.to_owned(),
        }
    }

    #[test]
    fn finds_outer_inner_and_expect_attributes() -> TestResult {
        let source = format!(
            "{}\nfn a() {{}}\n{}\n",
            attr(true, "allow", "dead_code"),
            attr(false, "expect", "clippy::too_many_lines, reason = \"x\"")
        );
        let found = Scanner::new()?.find("f.rs", &source);
        let lints: Vec<_> = found.iter().map(|s| (s.lint.as_str(), s.line)).collect();
        assert_eq!(lints, [("dead_code", 1), ("clippy::too_many_lines", 3)]);
        Ok(())
    }

    #[test]
    fn finds_attributes_inside_cfg_attr() -> TestResult {
        let source = format!("#[cfg_attr(test, {}(clippy::a, clippy::b))]", "allow");
        assert_eq!(Scanner::new()?.find("f.rs", &source).len(), 2);
        Ok(())
    }

    #[test]
    fn finds_attributes_inside_cfg_attr_with_nested_conditions() -> TestResult {
        let source = format!(
            "#[cfg_attr(not(test), {}(dead_code))]\n#[cfg_attr(all(unix, not(test)), {}(clippy::x))]",
            "allow", "allow"
        );
        let found = Scanner::new()?.find("f.rs", &source);
        let lints: Vec<_> = found.iter().map(|s| s.lint.as_str()).collect();
        assert_eq!(lints, ["dead_code", "clippy::x"]);
        Ok(())
    }

    #[test]
    fn finds_every_suppression_in_one_cfg_attr() -> TestResult {
        let source = format!(
            "#[cfg_attr(all(), {}(dead_code), {}(clippy::unwrap_used))]",
            "allow", "allow"
        );
        let found = Scanner::new()?.find("f.rs", &source);
        let lints: Vec<_> = found.iter().map(|s| (s.lint.as_str(), s.line)).collect();
        assert_eq!(lints, [("dead_code", 1), ("clippy::unwrap_used", 1)]);
        Ok(())
    }

    #[test]
    fn finds_suppressions_when_reason_contains_brackets() -> TestResult {
        let source = attr(false, "allow", "dead_code, reason = \"see [1]\"");
        let found = Scanner::new()?.find("f.rs", &source);
        let lints: Vec<_> = found.iter().map(|s| s.lint.as_str()).collect();
        assert_eq!(lints, ["dead_code"]);
        Ok(())
    }

    #[test]
    fn flags_unregistered_suppression() -> TestResult {
        let found = Scanner::new()?.find("f.rs", &attr(false, "allow", "clippy::x"));
        let violations = check(&found, &[]);
        assert_eq!(violations.len(), 1);
        assert_eq!(violations[0].location, "f.rs:1");
        Ok(())
    }

    #[test]
    fn accepts_registered_suppression() -> TestResult {
        let found = Scanner::new()?.find("f.rs", &attr(false, "allow", "clippy::x"));
        assert!(check(&found, &[entry("f.rs", "clippy::x", "generated code")]).is_empty());
        Ok(())
    }

    #[test]
    fn flags_reasonless_and_stale_entries() {
        let registry = [
            entry("f.rs", "clippy::x", " "),
            entry("g.rs", "clippy::y", "why"),
        ];
        let messages: Vec<_> = check(&[], &registry)
            .into_iter()
            .map(|v| v.message)
            .collect();
        assert_eq!(messages.len(), 2);
        assert!(messages[0].contains("no reason"));
        assert!(messages[1].contains("matches no suppression"));
    }

    #[test]
    fn complexity_waivers_are_never_stale() {
        assert!(check(&[], &[entry("f.rs", COMPLEXITY_LINT, "parser table")]).is_empty());
    }

    #[test]
    fn parses_registry_and_reports_bad_toml() -> TestResult {
        let src = "[[exception]]\nfile = \"f.rs\"\nlint = \"clippy::x\"\nreason = \"why\"\n";
        assert_eq!(
            parse_registry(src, "e.toml")?,
            [entry("f.rs", "clippy::x", "why")]
        );
        assert!(parse_registry("", "e.toml")?.is_empty());
        assert!(parse_registry("[[exception]]\nfile = 1", "e.toml").is_err());
        Ok(())
    }
}
