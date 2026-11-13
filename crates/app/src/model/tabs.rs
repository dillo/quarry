//! The query tabs: their order, titles, and which one is active.

/// A tab's identity; it never changes and is never reused while open.
pub type TabId = i64;

/// The prefix of every generated title, as in "Query 3".
const TITLE_PREFIX: &str = "Query ";

/// One tab's identity and title.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tab {
    /// The tab's identity.
    pub id: TabId,
    /// The title shown on the tab.
    pub title: String,
}

/// The open tabs, left to right, with one always active.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TabList {
    tabs: Vec<Tab>,
    active: usize,
    next_id: TabId,
}

impl TabList {
    /// Restores saved tabs; with none, opens "Query 1". `active` is clamped.
    pub fn restore(tabs: Vec<Tab>, active: usize) -> Self {
        let next_id = tabs.iter().map(|tab| tab.id).max().unwrap_or(0) + 1;
        let mut list = Self {
            active: active.min(tabs.len().saturating_sub(1)),
            tabs,
            next_id,
        };
        if list.tabs.is_empty() {
            list.open();
        }
        list
    }

    /// The tabs, left to right.
    pub fn tabs(&self) -> &[Tab] {
        &self.tabs
    }

    /// The active tab's index.
    pub fn active(&self) -> usize {
        self.active
    }

    /// The active tab's identity.
    pub fn active_id(&self) -> TabId {
        self.tabs[self.active].id
    }

    /// Opens a new "Query N" right after the active tab and activates it.
    pub fn open(&mut self) -> TabId {
        let id = self.next_id;
        self.next_id += 1;
        let title = format!("{TITLE_PREFIX}{}", self.next_number());
        let at = if self.tabs.is_empty() {
            0
        } else {
            self.active + 1
        };
        self.tabs.insert(at, Tab { id, title });
        self.active = at;
        id
    }

    /// Closes tab `id` and activates its right neighbour, or its left one if
    /// it was last. Closing the only tab opens a fresh one, whose id is
    /// returned. Returns `None` when `id` is not open or no tab was opened.
    pub fn close(&mut self, id: TabId) -> Option<TabId> {
        let index = self.index_of(id)?;
        self.tabs.remove(index);
        if self.tabs.is_empty() {
            return Some(self.open());
        }
        if index < self.active || self.active == self.tabs.len() {
            self.active -= 1;
        }
        None
    }

    /// Activates the next tab, wrapping to the first.
    pub fn next(&mut self) {
        self.active = (self.active + 1) % self.tabs.len();
    }

    /// Activates the previous tab, wrapping to the last.
    pub fn previous(&mut self) {
        self.active = (self.active + self.tabs.len() - 1) % self.tabs.len();
    }

    /// Activates tab `id`; returns whether it is open.
    pub fn activate(&mut self, id: TabId) -> bool {
        self.index_of(id).map(|index| self.active = index).is_some()
    }

    fn index_of(&self, id: TabId) -> Option<usize> {
        self.tabs.iter().position(|tab| tab.id == id)
    }

    /// One more than the highest "Query N" in use, so titles never repeat
    /// among open tabs.
    fn next_number(&self) -> u64 {
        self.tabs
            .iter()
            .filter_map(|tab| tab.title.strip_prefix(TITLE_PREFIX)?.parse::<u64>().ok())
            .max()
            .unwrap_or(0)
            + 1
    }
}

#[cfg(test)]
mod tests {
    use super::{Tab, TabList};

    fn tab(id: i64, title: &str) -> Tab {
        Tab {
            id,
            title: title.to_owned(),
        }
    }

    fn titles(list: &TabList) -> Vec<&str> {
        list.tabs().iter().map(|tab| tab.title.as_str()).collect()
    }

    fn three() -> TabList {
        TabList::restore(
            vec![tab(4, "Query 1"), tab(9, "Query 2"), tab(2, "Notes")],
            1,
        )
    }

    #[test]
    fn restoring_nothing_opens_query_one() {
        let list = TabList::restore(Vec::new(), 3);
        assert_eq!(titles(&list), ["Query 1"]);
        assert_eq!(list.active(), 0);
        assert_eq!(list.active_id(), 1);
    }

    #[test]
    fn restoring_keeps_order_and_clamps_the_active_tab() {
        let list = TabList::restore(vec![tab(4, "Query 1"), tab(9, "Query 2")], 7);
        assert_eq!(titles(&list), ["Query 1", "Query 2"]);
        assert_eq!(list.active(), 1);
        assert_eq!(three().active_id(), 9);
    }

    #[test]
    fn open_inserts_after_the_active_tab_with_the_next_number_and_a_new_id() {
        let mut list = three();
        let id = list.open();
        assert_eq!(id, 10);
        assert_eq!(titles(&list), ["Query 1", "Query 2", "Query 3", "Notes"]);
        assert_eq!(list.active(), 2);
        assert_eq!(list.open(), 11);
        assert_eq!(titles(&list)[3], "Query 4");
    }

    #[test]
    fn numbers_ignore_titles_that_are_not_query_numbers() {
        let mut list = TabList::restore(vec![tab(1, "Query x"), tab(2, "Query 7b")], 0);
        list.open();
        assert_eq!(titles(&list)[1], "Query 1");
    }

    #[test]
    fn closing_the_active_tab_activates_its_right_neighbour() {
        let mut list = three();
        assert_eq!(list.close(9), None);
        assert_eq!(titles(&list), ["Query 1", "Notes"]);
        assert_eq!(list.active_id(), 2);
    }

    #[test]
    fn closing_the_last_active_tab_activates_its_left_neighbour() {
        let mut list = three();
        list.next();
        list.close(2);
        assert_eq!(list.active_id(), 9);
        assert_eq!(list.active(), 1);
    }

    #[test]
    fn closing_a_tab_left_of_the_active_one_keeps_the_active_tab() {
        let mut list = three();
        list.close(4);
        assert_eq!(list.active_id(), 9);
        assert_eq!(list.active(), 0);
    }

    #[test]
    fn closing_a_tab_right_of_the_active_one_keeps_the_active_tab() {
        let mut list = three();
        list.close(2);
        assert_eq!(list.active_id(), 9);
        assert_eq!(list.active(), 1);
    }

    #[test]
    fn closing_the_only_tab_opens_a_fresh_one() {
        let mut list = TabList::restore(vec![tab(5, "Query 3")], 0);
        assert_eq!(list.close(5), Some(6));
        assert_eq!(titles(&list), ["Query 1"]);
        assert_eq!(list.active_id(), 6);
    }

    #[test]
    fn closing_an_unknown_tab_changes_nothing() {
        let mut list = three();
        assert_eq!(list.close(99), None);
        assert_eq!(list, three());
    }

    #[test]
    fn next_and_previous_wrap() {
        let mut list = three();
        list.next();
        assert_eq!(list.active(), 2);
        list.next();
        assert_eq!(list.active(), 0);
        list.previous();
        assert_eq!(list.active(), 2);
        list.previous();
        assert_eq!(list.active(), 1);
    }

    #[test]
    fn activate_selects_an_open_tab_only() {
        let mut list = three();
        assert!(list.activate(2));
        assert_eq!(list.active(), 2);
        assert!(!list.activate(99));
        assert_eq!(list.active(), 2);
    }
}
