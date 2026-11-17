//! Checking a challenge tab's answer: the whole editor text is graded on
//! the background executor, and the Check view shows the outcome.

use std::sync::Arc;

use gpui_kit::{AppContext, Context};
use quarry_challenges::{check, sandbox::Template, track::Challenge};

use super::QueryTab;
use crate::{model::verdict::CheckState, views::results::CHECK};

impl QueryTab {
    /// Gives the tab the Check view, for a challenge tab.
    pub fn play_challenge(&mut self, cx: &mut Context<Self>) {
        self.views = super::ResultViewRegistry::for_challenge();
        cx.notify();
    }

    /// Grades the editor's text as an answer to `challenge`, on copies of
    /// `template`, and shows the Check view. A newer check replaces one
    /// still running.
    pub fn check(&mut self, template: Arc<Template>, challenge: Challenge, cx: &mut Context<Self>) {
        let sql = self.text(cx);
        self.check = CheckState::Checking;
        self.select_view(CHECK, cx);
        self.checking = Some(cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(async move { check::check(&template, &challenge, &sql) })
                .await;
            let _ = this.update(cx, |tab, cx| {
                tab.check = CheckState::Done(outcome);
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Where the tab's latest check is.
    pub fn check_state(&self) -> &CheckState {
        &self.check
    }
}
