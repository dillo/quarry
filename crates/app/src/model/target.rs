//! Which part of the editor a Run executes.

use std::ops::Range;

use quarry_engine::splitter;

/// The byte range a Run sends: the selection when one exists, otherwise the
/// statement under the cursor. `None` when there is nothing to run.
pub fn run_target(text: &str, selection: Range<usize>) -> Option<Range<usize>> {
    if selection.is_empty() {
        return splitter::statement_at(text, selection.end);
    }
    text.get(selection.clone())
        .filter(|selected| !splitter::split(selected).is_empty())
        .map(|_| selection)
}

#[cfg(test)]
mod tests {
    use super::run_target;

    const SCRIPT: &str = "SELECT 1;\nSELECT 2;\n";

    #[test]
    fn a_cursor_runs_the_statement_it_sits_in() {
        assert_eq!(run_target(SCRIPT, 3..3), Some(0..8));
        assert_eq!(run_target(SCRIPT, 12..12), Some(10..18));
    }

    #[test]
    fn a_selection_runs_exactly_what_is_selected() {
        assert_eq!(run_target(SCRIPT, 0..18), Some(0..18));
    }

    #[test]
    fn a_selection_of_only_comments_runs_nothing() {
        assert_eq!(run_target("-- note\nSELECT 1", 0..7), None);
    }

    #[test]
    fn an_empty_editor_runs_nothing() {
        assert_eq!(run_target("", 0..0), None);
        assert_eq!(run_target("   ", 2..2), None);
    }

    #[test]
    fn a_selection_off_a_char_boundary_runs_nothing() {
        assert_eq!(run_target("SELECT 'é'", 9..10), None);
    }
}
