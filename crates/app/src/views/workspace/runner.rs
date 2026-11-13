//! Drives runs: reads the editor, submits the target, polls the session for
//! events and the schema, and folds them into the grid and the sidebar.
//!
//! The session's channels are polled once per [`POLL`] rather than awaited
//! (see [`crate::model::drain`]); each poll also redraws, which keeps the
//! live timer moving.

use std::time::Instant;

use async_channel::Receiver;
use gpui_kit::{App, Context, Task, Window};
use quarry_engine::{error::EngineError, schema::Schema, session::RunEvent};

use super::{POLL, Workspace};
use crate::{
    model::{
        drain::{Drained, drain},
        statements::RunOutput,
        target,
    },
    shell::actions::{CancelRun, RunStatement},
};

impl Workspace {
    pub(super) fn run_statement(
        &mut self,
        _: &RunStatement,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(sql) = self.target_sql(cx) else {
            return;
        };
        let handle = self.session.run(sql.clone());
        self.results.update(cx, |table, cx| {
            table.delegate_mut().reset(RunOutput::submitted(sql));
            table.refresh(cx);
        });
        self.run = Some(Self::pump(handle.events.clone(), cx));
        self.current = Some(handle);
        cx.notify();
    }

    pub(super) fn cancel(&mut self, _: &CancelRun, _: &mut Window, _: &mut Context<Self>) {
        if let Some(handle) = &self.current {
            handle.cancel();
        }
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

    /// The SQL a Run sends: the selection, or the statement under the cursor.
    fn target_sql(&self, cx: &App) -> Option<String> {
        let state = self.editor.read(cx);
        let script = state.value();
        let range = target::run_target(&script, state.selected_range())?;
        script.get(range).map(str::to_owned)
    }

    /// Polls the run's events once per [`POLL`] until the run ends.
    fn pump(events: Receiver<RunEvent>, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            let mut more = true;
            while more {
                cx.background_executor().timer(POLL).await;
                let drained = drain(&events);
                more = this
                    .update(cx, |w, cx| w.receive(drained, cx))
                    .unwrap_or(false);
            }
        })
    }

    /// Applies one poll's events; returns whether the run goes on.
    fn receive(&mut self, drained: Drained<RunEvent>, cx: &mut Context<Self>) -> bool {
        let done = drained.items.contains(&RunEvent::Done);
        let vanished = drained.closed && !done;
        let ended = drained.closed || done;
        self.apply(drained.items, vanished, cx);
        if ended {
            self.run_ended(cx);
        }
        cx.notify();
        !ended
    }

    /// Folds the events in; a channel that closed without `Done` ends the run
    /// as a vanished worker, unless an earlier event already failed it.
    fn apply(&mut self, events: Vec<RunEvent>, vanished: bool, cx: &mut Context<Self>) {
        self.results.update(cx, |table, cx| {
            let delegate = table.delegate_mut();
            let now = Instant::now();
            let mut reshaped = false;
            for event in events {
                reshaped |= delegate.apply(event, now);
            }
            if vanished {
                delegate.end_closed();
            }
            if reshaped {
                table.refresh(cx);
            }
            cx.notify();
        });
    }

    /// Releases the finished run's handle and re-reads the schema, which
    /// the run may have changed.
    fn run_ended(&mut self, cx: &mut Context<Self>) {
        self.current = None;
        self.read_schema(cx);
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
