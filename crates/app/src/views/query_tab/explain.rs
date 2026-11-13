//! Explain: asks the session for a statement's plan and shows it in the
//! Plan view. The reply is polled once per [`POLL`], like a run's events.

use async_channel::Receiver;
use gpui_kit::Context;
use quarry_engine::{error::EngineError, plan::Plan};

use super::QueryTab;
use crate::{
    model::{drain, plan_tree::PlanState},
    views::{results, workspace::POLL},
};

impl QueryTab {
    /// Shows the Plan view, and the plan once `reply` brings it.
    pub fn explain(&mut self, reply: Receiver<Result<Plan, EngineError>>, cx: &mut Context<Self>) {
        self.plan = PlanState::Planning;
        self.select_view(results::PLAN, cx);
        self.planning = Some(cx.spawn(async move |this, cx| {
            let mut reply_seen = None;
            while reply_seen.is_none() {
                cx.background_executor().timer(POLL).await;
                reply_seen = drain::reply(drain::drain(&reply));
            }
            let answer = reply_seen.unwrap_or(Err(EngineError::WorkerGone));
            let _ = this.update(cx, |tab, cx| tab.planned(answer, cx));
        }));
    }

    fn planned(&mut self, answer: Result<Plan, EngineError>, cx: &mut Context<Self>) {
        self.plan = PlanState::answered(answer);
        cx.notify();
    }

    /// What the Plan view shows.
    pub fn plan(&self) -> &PlanState {
        &self.plan
    }
}
