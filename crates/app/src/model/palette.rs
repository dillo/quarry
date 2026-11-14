//! What the ⌘K palette lists, and how typing narrows it.

/// The most results the palette shows.
pub const PALETTE_LIMIT: usize = 50;

/// One thing the palette can pick.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteItem {
    /// The id of the source that listed it, such as "tabs".
    pub source: &'static str,
    /// What the source needs to find it again when it is picked.
    pub key: String,
    /// The text shown and searched.
    pub label: String,
    /// Quieter text after the label, such as "current".
    pub detail: String,
}

/// How well `label` matches `query`, both already lowercase: 0 for a
/// prefix, 1 for the start of a later word, 2 for anywhere else.
fn rank(label: &str, query: &str) -> Option<u8> {
    let mut best = None;
    for (at, _) in label.match_indices(query) {
        let word_start = label[..at]
            .chars()
            .next_back()
            .is_none_or(|before| !before.is_alphanumeric());
        let this = match (at, word_start) {
            (0, _) => 0,
            (_, true) => 1,
            _ => 2,
        };
        best = Some(best.map_or(this, |known: u8| known.min(this)));
    }
    best
}

/// The items whose label holds `query`, ignoring case, best first within
/// each source; sources keep their order, and so do equal matches. An empty
/// query keeps every item. At most [`PALETTE_LIMIT`] come back.
pub fn filter<'a>(items: &'a [PaletteItem], query: &str) -> Vec<&'a PaletteItem> {
    let query = query.to_lowercase();
    let group = |source: &str| items.iter().position(|item| item.source == source);
    let mut matched: Vec<(Option<usize>, u8, &PaletteItem)> = items
        .iter()
        .filter_map(|item| {
            let rank = rank(&item.label.to_lowercase(), &query)?;
            Some((group(item.source), rank, item))
        })
        .collect();
    matched.sort_by_key(|(group, rank, _)| (*group, *rank));
    matched
        .into_iter()
        .take(PALETTE_LIMIT)
        .map(|(_, _, item)| item)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::{PALETTE_LIMIT, PaletteItem, filter, rank};

    fn item(source: &'static str, label: &str) -> PaletteItem {
        PaletteItem {
            source,
            key: label.to_owned(),
            label: label.to_owned(),
            detail: String::new(),
        }
    }

    fn labels(found: &[&PaletteItem]) -> Vec<String> {
        found.iter().map(|item| item.label.clone()).collect()
    }

    #[test]
    fn ranks_a_prefix_then_a_word_start_then_anywhere() {
        assert_eq!(rank("run all", "run"), Some(0));
        assert_eq!(rank("toggle sidebar", "side"), Some(1));
        assert_eq!(rank("toggle sidebar", "debar"), Some(2));
        assert_eq!(rank("focus sidebar", "sidebar"), Some(1));
        assert_eq!(rank("crew_members", "members"), Some(1));
        assert_eq!(rank("astronauts", "naut"), Some(2));
        assert_eq!(rank("astronauts", "x"), None);
    }

    #[test]
    fn the_best_of_several_matches_counts() {
        assert_eq!(rank("an anagram", "an"), Some(0));
        assert_eq!(rank("banana nap", "na"), Some(1));
    }

    #[test]
    fn an_empty_query_keeps_every_item_in_order() {
        let items = [
            item("tabs", "Query 2"),
            item("tabs", "Query 1"),
            item("commands", "Run All"),
        ];
        assert_eq!(
            labels(&filter(&items, "")),
            ["Query 2", "Query 1", "Run All"]
        );
    }

    #[test]
    fn matching_ignores_case() {
        let items = [item("tables", "Missions"), item("tables", "crew")];
        assert_eq!(labels(&filter(&items, "MISSION")), ["Missions"]);
    }

    #[test]
    fn better_matches_come_first_within_a_source_and_ties_keep_order() {
        let items = [
            item("commands", "Toggle Sidebar"),
            item("commands", "Focus Sidebar"),
            item("commands", "Sidebar Width"),
            item("commands", "Sidebar Shown"),
        ];
        assert_eq!(
            labels(&filter(&items, "sidebar")),
            [
                "Sidebar Width",
                "Sidebar Shown",
                "Toggle Sidebar",
                "Focus Sidebar"
            ]
        );
    }

    #[test]
    fn sources_keep_their_order_and_empty_ones_drop_out() {
        let items = [
            item("tabs", "Notes about runs"),
            item("closed", "Old"),
            item("tables", "runways"),
            item("commands", "Run All"),
        ];
        assert_eq!(
            labels(&filter(&items, "run")),
            ["Notes about runs", "runways", "Run All"]
        );
    }

    #[test]
    fn a_later_source_never_moves_ahead_of_an_earlier_one() {
        let items = [item("tabs", "mission notes"), item("tables", "missions")];
        assert_eq!(
            labels(&filter(&items, "mission")),
            ["mission notes", "missions"]
        );
        let items = [item("tables", "my missions"), item("commands", "missions")];
        assert_eq!(
            labels(&filter(&items, "mission")),
            ["my missions", "missions"]
        );
    }

    #[test]
    fn at_most_the_limit_comes_back() {
        let items: Vec<PaletteItem> = (0..PALETTE_LIMIT + 5)
            .map(|n| item("tables", &format!("t{n}")))
            .collect();
        let found = filter(&items, "");
        assert_eq!(found.len(), PALETTE_LIMIT);
        assert_eq!(
            found[PALETTE_LIMIT - 1].label,
            format!("t{}", PALETTE_LIMIT - 1)
        );
        assert_eq!(filter(&items, "t").len(), PALETTE_LIMIT);
    }
}
