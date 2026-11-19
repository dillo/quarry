//! Saving the tabs' drafts: a second after the last edit, at once when the
//! tabs change, when the window closes and when Quarry quits.
//!
//! Every snapshot gets a generation when it is taken, and every write holds
//! one lock, so an older snapshot never lands after a newer one.

use std::{
    sync::{Arc, Mutex, PoisonError},
    time::Duration,
};

use gpui_kit::{App, AppContext, Context, WindowId};
use quarry_store::{
    drafts::{Draft, DraftStore, Drafts, Forgetful},
    error::StoreError,
};

use super::Workspace;
use crate::model::save_order::SaveOrder;

/// How long after the last edit the drafts are saved.
pub const AUTOSAVE: Duration = Duration::from_secs(1);

/// Where the drafts are kept, and why they may not be.
pub struct SavedTabs {
    /// The store the workspace loads from and saves to.
    pub store: Arc<dyn DraftStore>,
    /// Why drafts are not being kept across restarts, if they are not.
    pub problem: Option<String>,
}

impl SavedTabs {
    /// The store to use, the drafts it holds and the warning to show. A store
    /// that cannot be read is replaced by one that keeps nothing, so a later
    /// save never overwrites what it holds.
    pub(super) fn open(self) -> (Arc<dyn DraftStore>, Drafts, Option<String>) {
        match self.store.load() {
            Ok(drafts) => (self.store, drafts, self.problem),
            Err(error) => (
                Arc::new(Forgetful),
                Drafts::default(),
                self.problem.or(Some(error.to_string())),
            ),
        }
    }
}

impl Workspace {
    /// Saves the drafts when the window `id` closes, so edits from the last
    /// second are not lost with the pending autosave.
    pub(super) fn save_on_close(id: WindowId, cx: &mut Context<Self>) {
        let this = cx.weak_entity();
        cx.on_window_closed(move |cx, closed| {
            if closed == id {
                let _ = this.read_with(cx, Workspace::save_before_quit);
            }
        })
        .detach();
    }

    /// Why drafts aren't being saved, if they aren't.
    pub fn drafts_problem(&self) -> Option<&str> {
        self.save_problem
            .as_deref()
            .or(self.open_problem.as_deref())
    }

    /// Every tab's title and text, which one is active, and the recently
    /// closed tabs.
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
                    challenge: self.challenge(tab.id).map(ToString::to_string),
                })
            })
            .collect();
        Drafts {
            tabs,
            active: self.tabs.active(),
            closed: self.closed.clone(),
        }
    }

    /// Saves after [`AUTOSAVE`], unless another edit comes first.
    pub(super) fn schedule_save(&mut self, cx: &mut Context<Self>) {
        self.autosave = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(AUTOSAVE).await;
            let _ = this.update(cx, Workspace::save_now);
        }));
    }

    /// A snapshot and its generation.
    fn numbered_snapshot(&self, cx: &App) -> (u64, Drafts) {
        let generation = lock(&self.saves).issue();
        (generation, self.snapshot(cx))
    }

    /// Saves a snapshot now, on the background executor.
    pub(super) fn save_now(&mut self, cx: &mut Context<Self>) {
        self.autosave = None;
        let (generation, drafts) = self.numbered_snapshot(cx);
        let store = Arc::clone(&self.store);
        let saves = Arc::clone(&self.saves);
        cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(
                    async move { save_in_order(&*store, &saves, generation, &drafts) },
                )
                .await;
            if let Some(outcome) = outcome {
                let _ = this.update(cx, |workspace, cx| workspace.saved(outcome, cx));
            }
        })
        .detach();
    }

    /// Saves synchronously, for when the window closes or Quarry quits.
    pub(super) fn save_before_quit(&self, cx: &App) {
        let (generation, drafts) = self.numbered_snapshot(cx);
        let _ = save_in_order(&*self.store, &self.saves, generation, &drafts);
    }

    fn saved(&mut self, saved: Result<(), StoreError>, cx: &mut Context<Self>) {
        self.save_problem = saved.err().map(|error| error.to_string());
        cx.notify();
    }
}

/// Writes `drafts`, snapshot `generation`, to `store` while holding the save
/// lock `saves`. Returns `None`, writing nothing, when a newer snapshot has
/// already been written.
pub fn save_in_order(
    store: &dyn DraftStore,
    saves: &Mutex<SaveOrder>,
    generation: u64,
    drafts: &Drafts,
) -> Option<Result<(), StoreError>> {
    let mut order = lock(saves);
    order.commit(generation).then(|| store.save(drafts))
}

/// The save lock; a panic while it was held leaves nothing to undo.
fn lock(saves: &Mutex<SaveOrder>) -> std::sync::MutexGuard<'_, SaveOrder> {
    saves.lock().unwrap_or_else(PoisonError::into_inner)
}
