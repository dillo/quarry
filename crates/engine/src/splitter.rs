//! Splits a SQL script into statements, respecting quotes and comments.
//!
//! Known limit: a `;` inside a `CREATE TRIGGER ... BEGIN ... END` body ends
//! the statement early, and so does a `;` inside a `SQLite` `[bracketed]`
//! identifier. Quarry's content avoids both.

use std::ops::Range;

/// Byte ranges of every statement in `script`, in order.
///
/// Each range excludes the terminating `;` and surrounding whitespace.
/// Segments holding only whitespace and comments are skipped.
pub fn split(script: &str) -> Vec<Range<usize>> {
    let bytes = script.as_bytes();
    let mut statements = Vec::new();
    let (mut start, mut pos) = (0, 0);
    while pos < bytes.len() {
        if bytes[pos] == b';' {
            push_statement(script, start..pos, &mut statements);
            start = pos + 1;
            pos += 1;
        } else {
            pos = skip_token(bytes, pos);
        }
    }
    push_statement(script, start..bytes.len(), &mut statements);
    statements
}

/// The statement to run for a cursor at byte `offset`: the last statement
/// starting at or before it, else the first statement.
pub fn statement_at(script: &str, offset: usize) -> Option<Range<usize>> {
    let statements = split(script);
    statements
        .iter()
        .rev()
        .find(|range| range.start <= offset)
        .or_else(|| statements.first())
        .cloned()
}

fn push_statement(script: &str, range: Range<usize>, out: &mut Vec<Range<usize>>) {
    let text = &script[range.clone()];
    let trimmed = text.trim();
    if trimmed.is_empty() || !has_code(trimmed.as_bytes()) {
        return;
    }
    let start = range.start + (text.len() - text.trim_start().len());
    out.push(start..start + trimmed.len());
}

fn has_code(bytes: &[u8]) -> bool {
    let mut pos = 0;
    while pos < bytes.len() {
        match (bytes[pos], bytes.get(pos + 1)) {
            (b'-', Some(b'-')) => pos = skip_line_comment(bytes, pos),
            (b'/', Some(b'*')) => pos = skip_block_comment(bytes, pos),
            (byte, _) if byte.is_ascii_whitespace() => pos += 1,
            _ => return true,
        }
    }
    false
}

fn skip_token(bytes: &[u8], pos: usize) -> usize {
    match (bytes[pos], bytes.get(pos + 1)) {
        (b'\'' | b'"' | b'`', _) => skip_quoted(bytes, pos),
        (b'-', Some(b'-')) => skip_line_comment(bytes, pos),
        (b'/', Some(b'*')) => skip_block_comment(bytes, pos),
        _ => pos + 1,
    }
}

/// Skips a quoted string or identifier; a doubled quote is an escape.
fn skip_quoted(bytes: &[u8], pos: usize) -> usize {
    let quote = bytes[pos];
    let mut i = pos + 1;
    while i < bytes.len() {
        if bytes[i] != quote {
            i += 1;
        } else if bytes.get(i + 1) == Some(&quote) {
            i += 2;
        } else {
            return i + 1;
        }
    }
    bytes.len()
}

/// Skips to the newline ending a `--` comment (the newline itself is kept).
fn skip_line_comment(bytes: &[u8], pos: usize) -> usize {
    bytes[pos..]
        .iter()
        .position(|&b| b == b'\n')
        .map_or(bytes.len(), |n| pos + n)
}

fn skip_block_comment(bytes: &[u8], pos: usize) -> usize {
    bytes[pos + 2..]
        .windows(2)
        .position(|w| w == b"*/")
        .map_or(bytes.len(), |n| pos + 2 + n + 2)
}

#[cfg(test)]
mod tests {
    use super::{split, statement_at};

    fn texts(script: &str) -> Vec<&str> {
        split(script).into_iter().map(|r| &script[r]).collect()
    }

    #[test]
    fn splits_on_semicolons_and_trims() {
        assert_eq!(
            texts("SELECT 1;\n  SELECT 2 ;SELECT 3"),
            ["SELECT 1", "SELECT 2", "SELECT 3"]
        );
    }

    #[test]
    fn ignores_semicolons_inside_quotes() {
        let script = "SELECT 'a;b', \"c;d\", `e;f`; SELECT 'it''s; fine'";
        assert_eq!(
            texts(script),
            ["SELECT 'a;b', \"c;d\", `e;f`", "SELECT 'it''s; fine'"]
        );
    }

    #[test]
    fn ignores_semicolons_inside_comments() {
        let script = "SELECT 1 -- not; here\n; /* nor; here */ SELECT 2";
        assert_eq!(
            texts(script),
            ["SELECT 1 -- not; here", "/* nor; here */ SELECT 2"]
        );
    }

    #[test]
    fn skips_empty_and_comment_only_segments() {
        assert_eq!(
            texts(";; -- only a comment\n; /* and this */ ;SELECT 1;"),
            ["SELECT 1"]
        );
        assert!(split("").is_empty());
        assert!(split("   \n\t ").is_empty());
    }

    #[test]
    fn comment_only_segments_may_contain_whitespace_between_comments() {
        assert_eq!(
            texts("/* a */  \n  -- b\n  /* c */; SELECT 1"),
            ["SELECT 1"]
        );
    }

    #[test]
    fn doubled_quotes_are_escapes_not_terminators() {
        assert_eq!(texts("SELECT '''' ; SELECT 2"), ["SELECT ''''", "SELECT 2"]);
        assert_eq!(
            texts("SELECT \"a\"\"b;\" ; SELECT 2"),
            ["SELECT \"a\"\"b;\"", "SELECT 2"]
        );
    }

    #[test]
    fn a_leading_line_comment_ends_at_its_newline() {
        let script = "-- first; comment\nSELECT 1; SELECT 2";
        assert_eq!(texts(script), ["-- first; comment\nSELECT 1", "SELECT 2"]);
    }

    #[test]
    fn unterminated_quote_or_comment_runs_to_the_end() {
        assert_eq!(
            texts("SELECT 'open; still open"),
            ["SELECT 'open; still open"]
        );
        assert_eq!(
            texts("SELECT 1 /* open; comment"),
            ["SELECT 1 /* open; comment"]
        );
        assert_eq!(texts("SELECT 1 -- trailing"), ["SELECT 1 -- trailing"]);
    }

    #[test]
    fn keeps_operators_that_look_like_comment_starts() {
        assert_eq!(
            texts("SELECT 4 - -1; SELECT 8 / 2"),
            ["SELECT 4 - -1", "SELECT 8 / 2"]
        );
    }

    #[test]
    fn ranges_are_byte_offsets_into_multibyte_text() {
        let script = "SELECT 'café'; SELECT 'ü'";
        assert_eq!(split(script), [0..14, 16..27]);
    }

    #[test]
    fn statement_at_picks_the_statement_under_or_before_the_cursor() {
        let script = "SELECT 1;\n\nSELECT 2;";
        assert_eq!(statement_at(script, 3), Some(0..8));
        assert_eq!(statement_at(script, 9), Some(0..8));
        assert_eq!(statement_at(script, 11), Some(11..19));
        assert_eq!(statement_at(script, 100), Some(11..19));
    }

    #[test]
    fn statement_at_before_the_first_statement_picks_the_first() {
        assert_eq!(statement_at("\n\n  SELECT 1", 0), Some(4..12));
        assert_eq!(statement_at("-- nothing", 0), None);
    }
}
