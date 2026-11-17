//! Submits the active tab's runs and plans to its session, and reads its
//! schema.

use gpui_kit::{Context, Window};
use quarry_engine::splitter;

use super::Workspace;
use crate::{
    shell::actions::{CancelRun, Explain, RunAll, RunStatement},
    views::{
        poll::poll_reply,
        query_tab::{QueryTab, RunScope},
    },
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
        let handle = self.session().run(sql.clone());
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
        let handle = self.session().run(script.clone());
        tab.update(cx, |tab, cx| {
            tab.start(script, RunScope::Script, handle, cx);
        });
        cx.notify();
    }

    /// Shows the plan of the active tab's selection or current statement.
    pub(super) fn explain(&mut self, _: &Explain, _: &mut Window, cx: &mut Context<Self>) {
        let tab = self.active_tab().clone();
        let Some(sql) = tab.read(cx).target_sql(cx) else {
            return;
        };
        let reply = self.session().plan(sql);
        tab.update(cx, |tab, cx| tab.explain(reply, cx));
    }

    pub(super) fn cancel(&mut self, _: &CancelRun, _: &mut Window, cx: &mut Context<Self>) {
        self.active_tab().clone().update(cx, QueryTab::cancel);
    }

    /// Reads the active tab's schema after any queued runs and shows it in
    /// the sidebar.
    pub(super) fn read_schema(&mut self, cx: &mut Context<Self>) {
        let reply = self.session().schema();
        let tree = self.schema.clone();
        self.schema_read = Some(cx.spawn(async move |_, cx| {
            let schema = poll_reply(reply, cx.background_executor().clone()).await;
            tree.update(cx, |tree, cx| tree.set(schema, cx));
        }));
    }
}
