//! Saving the tabs' drafts: a second after the last edit, at once when the
//! tabs change, and when Quarry quits.
//!
//! Writes run on the background executor, one after another, so a later
//! snapshot never lands before an earlier one.

use std::{sync::Arc, time::Duration};

use gpui_kit::{App, AppContext, Context};
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts},
    error::StoreError,
};

use super::Workspace;

/// How long after the last edit the drafts are saved.
pub const AUTOSAVE: Duration = Duration::from_secs(1);

/// Where the drafts are kept, and why they may not be.
pub struct SavedTabs {
    /// The store the workspace loads from and saves to.
    pub store: Arc<dyn DraftStore>,
    /// Why drafts are not being kept across restarts, if they are not.
    pub problem: Option<String>,
}

impl Workspace {
    /// Why drafts aren't being saved, if they aren't.
    pub fn drafts_problem(&self) -> Option<&str> {
        self.save_problem
            .as_deref()
            .or(self.open_problem.as_deref())
    }

    /// Every tab's title and text, and which one is active.
    pub fn snapshot(&self, cx: &App) -> Drafts {
        let tabs = self
            .tabs
            .tabs()
            .iter()
            .filter_map(|tab| {
                Some(Draft {
                    id: tab.id,
                    title: tab.title.clone(),
                    text: self.views.get(&tab.id)?.read(cx).text(cx),
                })
            })
            .collect();
        Drafts {
            tabs,
            active: self.tabs.active(),
        }
    }

    /// Saves after [`AUTOSAVE`], unless another edit comes first.
    pub(super) fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.autosave = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AUTOSAVE).await;
            let _ = this.update(cx, Workspace::save_now);
        }));
    }

    /// Saves a snapshot now, after any save still being written.
    pub(super) fn save_now(&mut self, cx: &mut Context<Self>) {
        self.autosave = None;
        let drafts = self.snapshot(cx);
        let store = Arc::clone(&self.store);
        let previous = self.saving.take();
        self.saving = Some(cx.spawn(async move |this, cx| {
            if let Some(previous) = previous {
                previous.await;
            }
            let saved = cx
                .background_spawn(async move { store.save(&drafts) })
                .await;
            let _ = this.update(cx, |workspace, cx| workspace.saved(saved, cx));
        }));
    }

    /// Saves synchronously, for when Quarry is quitting.
    pub(super) fn save_before_quit(&self, cx: &App) {
        let _ = self.store.save(&self.snapshot(cx));
    }

    fn saved(&mut self, saved: Result<(), StoreError>, cx: &mut Context<Self>) {
        self.save_problem = saved.err().map(|error| error.to_string());
        cx.notify();
    }
}
