//! The recently closed tabs, newest first, for reopening from ⌘K.

use quarry_store::drafts::Draft;

/// The most closed tabs Quarry remembers.
pub const CLOSED_LIMIT: usize = 10;

/// Puts `draft` at the front of `closed`. A tab with nothing but blank text
/// is not worth reopening and is ignored; one with the same title and text
/// as an entry already there moves that entry to the front. The list keeps
/// at most [`CLOSED_LIMIT`] entries.
pub fn remember(closed: &mut Vec<Draft>, draft: Draft) {
    if draft.text.trim().is_empty() {
        return;
    }
    closed.retain(|known| known.title != draft.title || known.text != draft.text);
    closed.insert(0, draft);
    closed.truncate(CLOSED_LIMIT);
}

/// Removes and returns entry `index`, if there is one.
pub fn take(closed: &mut Vec<Draft>, index: usize) -> Option<Draft> {
    (index < closed.len()).then(|| closed.remove(index))
}

#[cfg(test)]
mod tests {
    use quarry_store::drafts::Draft;

    use super::{CLOSED_LIMIT, remember, take};

    fn draft(title: &str, text: &str) -> Draft {
        Draft {
            id: 0,
            title: title.to_owned(),
            text: text.to_owned(),
        }
    }

    fn titles(closed: &[Draft]) -> Vec<&str> {
        closed.iter().map(|draft| draft.title.as_str()).collect()
    }

    #[test]
    fn the_newest_comes_first() {
        let mut closed = Vec::new();
        remember(&mut closed, draft("Query 1", "SELECT 1"));
        remember(&mut closed, draft("Query 2", "SELECT 2"));
        assert_eq!(titles(&closed), ["Query 2", "Query 1"]);
    }

    #[test]
    fn blank_text_is_ignored() {
        let mut closed = Vec::new();
        remember(&mut closed, draft("Query 1", ""));
        remember(&mut closed, draft("Query 2", " \n\t "));
        assert!(closed.is_empty());
    }

    #[test]
    fn the_same_tab_again_moves_to_the_front() {
        let mut closed = vec![draft("Query 1", "SELECT 1"), draft("Query 2", "SELECT 2")];
        remember(&mut closed, draft("Query 2", "SELECT 2"));
        assert_eq!(titles(&closed), ["Query 2", "Query 1"]);
    }

    #[test]
    fn a_different_title_or_text_is_a_different_tab() {
        let mut closed = vec![draft("Query 1", "SELECT 1")];
        remember(&mut closed, draft("Query 1", "SELECT 2"));
        remember(&mut closed, draft("Query 9", "SELECT 1"));
        assert_eq!(closed.len(), 3);
    }

    #[test]
    fn at_most_the_limit_is_kept() {
        let mut closed = Vec::new();
        for n in 0..=CLOSED_LIMIT {
            remember(&mut closed, draft(&format!("Query {n}"), "SELECT 1"));
        }
        assert_eq!(closed.len(), CLOSED_LIMIT);
        assert_eq!(closed[0].title, format!("Query {CLOSED_LIMIT}"));
        assert_eq!(closed[CLOSED_LIMIT - 1].title, "Query 1");
    }

    #[test]
    fn take_removes_one_entry() {
        let mut closed = vec![draft("Query 1", "a"), draft("Query 2", "b")];
        assert_eq!(take(&mut closed, 1), Some(draft("Query 2", "b")));
        assert_eq!(take(&mut closed, 1), None);
        assert_eq!(titles(&closed), ["Query 1"]);
    }
}
