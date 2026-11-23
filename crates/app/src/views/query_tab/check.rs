//! Checking a challenge tab's answer: the whole editor text is graded on
//! the background executor, and the Check view shows the outcome.

use std::sync::Arc;

use gpui_kit::{AppContext, Context};
use quarry_challenges::{
    check,
    run::{LIMITS, Runner, Stop},
    sandbox::Template,
    track::Challenge,
};

use super::QueryTab;
use crate::{model::verdict::CheckState, views::results::CHECK};

impl QueryTab {
    /// Gives the tab the Check view, for a challenge tab.
    pub fn play_challenge(&mut self, cx: &mut Context<Self>) {
        self.views = super::ResultViewRegistry::for_challenge();
        cx.notify();
    }

    /// Grades the editor's text as an answer to `challenge`, on copies of
    /// `template`, and shows the Check view. A newer check stops one still
    /// running and replaces it.
    pub fn check(&mut self, template: Arc<Template>, challenge: Challenge, cx: &mut Context<Self>) {
        let sql = self.text(cx);
        let runner = self.restart_check();
        self.check = CheckState::Checking;
        self.select_view(CHECK, cx);
        self.checking = Some(cx.spawn(async move |this, cx| {
            let outcome = cx
                .background_spawn(
                    async move { check::check_with(&template, &challenge, &sql, &runner) },
                )
                .await;
            let _ = this.update(cx, |tab, cx| {
                tab.check = CheckState::Done(outcome);
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// Stops the latest check, and returns a runner for the next one.
    fn restart_check(&mut self) -> Runner {
        self.check_stop.stop();
        self.check_stop = Stop::default();
        Runner::stoppable(LIMITS, self.check_stop.clone())
    }

    /// What stops the tab's latest check.
    pub fn check_stop(&self) -> &Stop {
        &self.check_stop
    }

    /// Where the tab's latest check is.
    pub fn check_state(&self) -> &CheckState {
        &self.check
    }
}
