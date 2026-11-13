//! Submits the active tab's runs to the session, and reads the schema.
//!
//! The schema reply is polled once per [`POLL`] rather than awaited (see
//! [`crate::model::drain`]).

use gpui_kit::{Context, Window};
use quarry_engine::{error::EngineError, schema::Schema, splitter};

use super::{POLL, Workspace};
use crate::{
    model::drain::{Drained, drain},
    shell::actions::{CancelRun, RunAll, RunStatement},
    views::query_tab::RunScope,
};

impl Workspace {
    pub(super) fn run_statement(
        &mut self,
        _: &RunStatement,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let tab = self.active_tab().clone();
        let Some(sql) = tab.read(cx).target_sql(cx) else {
            return;
        };
        let handle = self.session.run(sql.clone());
        tab.update(cx, |tab, cx| {
            tab.start(sql, RunScope::Statement, handle, cx);
        });
        cx.notify();
    }

    /// Runs the active tab's whole script, statement by statement.
    pub(super) fn run_all(&mut self, _: &RunAll, _: &mut Window, cx: &mut Context<Self>) {
        let tab = self.active_tab().clone();
        let script = tab.read(cx).text(cx);
        if splitter::split(&script).is_empty() {
            return;
        }
        let handle = self.session.run(script.clone());
        tab.update(cx, |tab, cx| {
            tab.start(script, RunScope::Script, handle, cx);
        });
        cx.notify();
    }

    pub(super) fn cancel(&mut self, _: &CancelRun, _: &mut Window, cx: &mut Context<Self>) {
        self.active_tab().read(cx).cancel();
    }

    /// Reads the schema after any queued runs and shows it in the sidebar.
    pub(super) fn read_schema(&mut self, cx: &mut Context<Self>) {
        let reply = self.session.schema();
        let tree = self.schema.clone();
        self.schema_read = Some(cx.spawn(async move |_, cx| {
            let mut answer = None;
            while answer.is_none() {
                cx.background_executor().timer(POLL).await;
                answer = schema_answer(drain(&reply));
            }
            if let Some(schema) = answer {
                tree.update(cx, |tree, cx| tree.set(schema, cx));
            }
        }));
    }
}

/// The schema reply, once it has come: the reply itself, or `WorkerGone`
/// when the channel closed without one.
fn schema_answer(
    drained: Drained<Result<Schema, EngineError>>,
) -> Option<Result<Schema, EngineError>> {
    let Drained { mut items, closed } = drained;
    items
        .pop()
        .or_else(|| closed.then_some(Err(EngineError::WorkerGone)))
}
