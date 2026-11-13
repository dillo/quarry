//! The workspace's query tabs: opening, closing and switching.

use gpui_kit::{AppContext, Context, Entity, Window};

use super::Workspace;
use crate::{
    model::tabs::TabId,
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

    pub(super) fn close_tab(&mut self, _: &CloseTab, window: &mut Window, cx: &mut Context<Self>) {
        let closing = self.tabs.active_id();
        let opened = self.tabs.close(closing);
        self.views.remove(&closing);
        if let Some(id) = opened {
            self.add_view(id, "", window, cx);
        }
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
