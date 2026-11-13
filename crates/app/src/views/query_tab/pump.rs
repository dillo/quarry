//! Follows a run: polls its events once per [`POLL`] and folds them into
//! the grid.
//!
//! The channel is polled rather than awaited (see [`crate::model::drain`]);
//! each poll also redraws, which keeps the live timer moving.

use std::time::Instant;

use async_channel::Receiver;
use gpui_kit::{Context, Task};
use quarry_engine::session::RunEvent;

use super::{QueryTab, QueryTabEvent};
use crate::{
    model::drain::{Drained, drain},
    views::workspace::POLL,
};

impl QueryTab {
    /// Polls the run's events once per [`POLL`] until the run ends.
    pub(super) fn pump(events: Receiver<RunEvent>, cx: &mut Context<Self>) -> Task<()> {
        cx.spawn(async move |this, cx| {
            let mut more = true;
            while more {
                cx.background_executor().timer(POLL).await;
                let drained = drain(&events);
                more = this
                    .update(cx, |tab, cx| tab.receive(drained, cx))
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
            self.run = None;
            cx.emit(QueryTabEvent::RunEnded);
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
}
