//! Challenge tabs: each plays one challenge on its own sandbox, a session
//! on a fresh copy of its track's database. Plain tabs share the sample
//! database.

use std::collections::BTreeMap;

use gpui_kit::{Context, Window};
use quarry_challenges::catalog::ChallengeId;
use quarry_challenges::track::Challenge;
use quarry_engine::session::Session;
use quarry_store::drafts::Draft;

use super::Workspace;
use crate::{
    model::tabs::TabId, shell::actions::CheckAnswer, tracks::Tracks, views::query_tab::QueryTab,
};

/// What ⌘⇧↵ says in a tab that isn't a challenge tab.
pub const NOT_A_CHALLENGE: &str = "Check answer works in challenge tabs";

/// The tracks, and the sandbox of every open challenge tab.
pub(super) struct Challenges {
    tracks: Tracks,
    sandboxes: BTreeMap<TabId, Sandbox>,
}

impl Challenges {
    /// The embedded tracks, with no challenge tab open.
    pub(super) fn embedded() -> Self {
        Self {
            tracks: Tracks::embedded(),
            sandboxes: BTreeMap::new(),
        }
    }
}

/// What makes a tab a challenge tab.
struct Sandbox {
    challenge: ChallengeId,
    session: Session,
}

impl Workspace {
    /// The challenge tracks.
    pub fn tracks(&self) -> &Tracks {
        &self.challenges.tracks
    }

    /// Opens challenge `id`: activates its tab if one is open, or opens a
    /// new tab after the active one holding its skeleton. A challenge that
    /// doesn't exist does nothing.
    pub fn open_challenge(
        &mut self,
        id: &ChallengeId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if let Some(tab) = self.challenge_tab(id) {
            self.activate_tab(tab, window, cx);
            return;
        }
        let Some((_, challenge)) = self.challenges.tracks.challenge(id) else {
            return;
        };
        let skeleton = challenge.skeleton.clone();
        if self.open_sandbox_tab(id, &skeleton, window, cx) {
            self.tabs_changed(window, cx);
        }
    }

    /// Opens a tab for challenge `id`, titled after it and holding `text`,
    /// after the active one, on a fresh sandbox. Returns `false`, opening
    /// nothing, when the challenge doesn't exist or its sandbox can't be
    /// made.
    pub(super) fn open_sandbox_tab(
        &mut self,
        id: &ChallengeId,
        text: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> bool {
        let Some((_, challenge)) = self.challenges.tracks.challenge(id) else {
            return false;
        };
        let title = challenge.title.clone();
        let Some(sandbox) = self.sandbox(id, &title, cx) else {
            return false;
        };
        let tab = self.tabs.reopen(title);
        self.add_view(tab, text, window, cx);
        self.attach(tab, sandbox, cx);
        true
    }

    /// Makes the restored tab `draft` a challenge tab again, when it played
    /// a challenge that still exists. Otherwise it stays a plain tab and
    /// keeps its text.
    pub(super) fn restore_sandbox(&mut self, draft: &Draft, cx: &mut Context<Self>) {
        let id = draft.challenge.as_deref().and_then(ChallengeId::parse);
        let Some(id) = id.filter(|id| self.challenges.tracks.challenge(id).is_some()) else {
            return;
        };
        if let Some(sandbox) = self.sandbox(&id, &draft.title, cx) {
            self.attach(draft.id, sandbox, cx);
        }
    }

    /// A fresh sandbox for challenge `id`. When it can't be made, the status
    /// bar says why, naming the tab `title`.
    fn sandbox(
        &mut self,
        id: &ChallengeId,
        title: &str,
        cx: &mut Context<Self>,
    ) -> Option<Sandbox> {
        match self.challenges.tracks.sandbox(id) {
            Ok(session) => Some(Sandbox {
                challenge: id.clone(),
                session,
            }),
            Err(error) => {
                self.show_note(format!("Couldn't open {title}: {error}"), cx);
                None
            }
        }
    }

    /// Checks the active challenge tab's answer; anywhere else, the status
    /// bar says checking needs a challenge tab.
    pub(super) fn check_answer(&mut self, _: &CheckAnswer, _: &mut Window, cx: &mut Context<Self>) {
        let Some((slug, challenge)) = self.active_challenge_details() else {
            self.show_note(NOT_A_CHALLENGE.to_owned(), cx);
            return;
        };
        match self.challenges.tracks.template(&slug) {
            Ok(template) => {
                let tab = self.active_tab().clone();
                tab.update(cx, |tab, cx| tab.check(template, challenge, cx));
            }
            Err(error) => self.show_note(error.to_string(), cx),
        }
    }

    /// The active challenge's track slug and challenge.
    fn active_challenge_details(&self) -> Option<(String, Challenge)> {
        let id = self.active_challenge()?;
        let (track, challenge) = self.challenges.tracks.challenge(id)?;
        Some((track.slug.clone(), challenge.clone()))
    }

    /// Makes `tab` a challenge tab on `sandbox`, with the Check view.
    fn attach(&mut self, tab: TabId, sandbox: Sandbox, cx: &mut Context<Self>) {
        self.challenges.sandboxes.insert(tab, sandbox);
        if let Some(view) = self.views.get(&tab) {
            view.update(cx, QueryTab::play_challenge);
        }
    }

    /// The open tab playing challenge `id`.
    pub(super) fn challenge_tab(&self, id: &ChallengeId) -> Option<TabId> {
        self.challenges
            .sandboxes
            .iter()
            .find(|(_, sandbox)| &sandbox.challenge == id)
            .map(|(tab, _)| *tab)
    }

    /// The challenge tab `tab` plays, if it is a challenge tab.
    pub fn challenge(&self, tab: TabId) -> Option<&ChallengeId> {
        self.challenges
            .sandboxes
            .get(&tab)
            .map(|sandbox| &sandbox.challenge)
    }

    /// The challenge the active tab plays, if it is a challenge tab.
    pub fn active_challenge(&self) -> Option<&ChallengeId> {
        self.challenge(self.tabs.active_id())
    }

    /// The session the active tab runs on: its sandbox, or the sample
    /// database.
    pub(super) fn session(&self) -> &Session {
        self.challenges
            .sandboxes
            .get(&self.tabs.active_id())
            .map_or(&self.session, |sandbox| &sandbox.session)
    }

    /// What the status bar calls the active tab's database, such as
    /// "Vinyl Vault · sandbox".
    pub fn database(&self) -> String {
        let track = self
            .active_challenge()
            .and_then(|id| self.challenges.tracks.catalog().track(id.track()));
        track.map_or_else(
            || self.database.clone(),
            |track| format!("{} · sandbox", track.title),
        )
    }

    /// Forgets tab `tab`'s sandbox, ending its session.
    pub(super) fn drop_sandbox(&mut self, tab: TabId) {
        self.challenges.sandboxes.remove(&tab);
    }
}
