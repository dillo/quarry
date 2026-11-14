//! Explain: asks the session for a statement's plan and shows it in the
//! Plan view.

use async_channel::Receiver;
use gpui_kit::Context;
use quarry_engine::{error::EngineError, plan::Plan};

use super::QueryTab;
use crate::{
    model::plan_tree::PlanState,
    views::{poll::poll_reply, results},
};

impl QueryTab {
    /// Shows the Plan view, and the plan once `reply` brings it. A plan
    /// still pending is dropped, so only the latest Explain is shown.
    pub fn explain(&mut self, reply: Receiver<Result<Plan, EngineError>>, cx: &mut Context<Self>) {
        self.plan = PlanState::Planning;
        self.select_view(results::PLAN, cx);
        self.planning = Some(cx.spawn(async move |this, cx| {
            let answer = poll_reply(reply, cx.background_executor().clone()).await;
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
