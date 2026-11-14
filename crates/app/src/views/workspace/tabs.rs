//! The workspace's query tabs: opening, closing, reopening and switching.

use gpui_kit::{AppContext, Context, Entity, Window};
use quarry_store::drafts::Draft;

use super::Workspace;
use crate::{
    model::{closed_tabs, tabs::TabId},
    shell::actions::{CloseTab, NewTab, NextTab, PreviousTab},
    views::query_tab::{QueryTab, QueryTabEvent},
};

impl Workspace {
    /// Creates the view for tab `id`, whose editor holds `text`.
    pub(super) fn add_view(
        &mut self,
        id: TabId,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let view = cx.new(|cx| QueryTab::new(text, window, cx));
        cx.subscribe(&view, |workspace, _, event, cx| match event {
            QueryTabEvent::RunEnded => workspace.read_schema(cx),
            QueryTabEvent::Edited => workspace.schedule_save(cx),
            QueryTabEvent::Copied(note) => workspace.show_note(note.clone(), cx),
        })
        .detach();
        self.views.insert(id, view);
    }

    /// The active tab's view.
    pub fn active_tab(&self) -> &Entity<QueryTab> {
        &self.views[&self.tabs.active_id()]
    }

    pub(super) fn new_tab(&mut self, _: &NewTab, window: &mut Window, cx: &mut Context<Self>) {
        let id = self.tabs.open();
        self.add_view(id, "", window, cx);
        self.tabs_changed(window, cx);
    }

    /// Closes the active tab and remembers it for reopening, even when it
    /// is the only one and a fresh tab takes its place.
    pub(super) fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        let closing = self.tabs.active_id();
        self.remember(closing, cx);
        let opened = self.tabs.close(closing);
        let closed = self.views.remove(&closing);
        // A run that is cut off never emits `RunEnded`, so read the schema for it.
        if closed.is_some_and(|tab| tab.read(cx).output(cx).is_running()) {
            self.read_schema(cx);
        }
        if let Some(id) = opened {
            self.add_view(id, "", window, cx);
        }
        self.tabs_changed(window, cx);
    }

    /// Puts tab `id`'s title and text at the front of the closed tabs.
    fn remember(&mut self, id: TabId, cx: &mut Context<Self>) {
        let tab = self.tabs.tabs().iter().find(|tab| tab.id == id);
        let Some((tab, view)) = tab.zip(self.views.get(&id)) else {
            return;
        };
        let draft = Draft {
            id,
            title: tab.title.clone(),
            text: view.read(cx).text(cx),
        };
        closed_tabs::remember(&mut self.closed, draft);
    }

    /// The recently closed tabs, newest first.
    pub fn closed_tabs(&self) -> &[Draft] {
        &self.closed
    }

    /// Reopens closed tab `index` as a new tab after the active one, with
    /// its title and text, and saves at once. An index past the list does
    /// nothing.
    pub fn reopen_tab(&mut self, index: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(draft) = closed_tabs::take(&mut self.closed, index) else {
            return;
        };
        let id = self.tabs.reopen(draft.title);
        self.add_view(id, &draft.text, window, cx);
        self.tabs_changed(window, cx);
    }

    pub(super) fn next_tab(&mut self, _: &NextTab, window: &mut Window, cx: &mut Context<Self>) {
        self.tabs.next();
        self.tabs_changed(window, cx);
    }

    pub(super) fn previous_tab(
        &mut self,
        _: &PreviousTab,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.tabs.previous();
        self.tabs_changed(window, cx);
    }

    /// Activates tab `id`, as a click on it does.
    pub fn activate_tab(&mut self, id: TabId, window: &mut Window, cx: &mut Context<Self>) {
        if self.tabs.activate(id) {
            self.tabs_changed(window, cx);
        }
    }

    /// Saves, focuses the active tab's editor, and redraws.
    fn tabs_changed(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.save_now(cx);
        self.editor(cx)
            .update(cx, |state, cx| state.focus(window, cx));
        cx.notify();
    }
}
