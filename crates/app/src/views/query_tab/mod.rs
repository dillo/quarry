//! One query tab: its editor, its latest run, and that run's results.
//!
//! The workspace owns the [`quarry_engine::session::Session`] and submits
//! runs; a tab follows its run's events (see [`pump`]) and tells the
//! workspace when the run ends.

mod layout;
mod pump;

use async_channel::Receiver;
use gpui_kit::{
    App, AppContext, Context, Entity, EventEmitter, Subscription, Task, Window,
    component::{
        input::{EditorState, InputEvent},
        table::TableState,
    },
};
use quarry_engine::session::{RunEvent, RunHandle};

use super::{editor, results::ResultsDelegate};
use crate::model::{statements::RunOutput, target};

/// What a tab tells the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryTabEvent {
    /// The tab's run ended; the schema may have changed.
    RunEnded,
    /// The user changed the editor's text.
    Edited,
}

/// A query tab.
pub struct QueryTab {
    editor: Entity<EditorState>,
    results: Entity<TableState<ResultsDelegate>>,
    run: Option<RunHandle>,
    pump: Option<Task<()>>,
    _edits: Subscription,
}

impl EventEmitter<QueryTabEvent> for QueryTab {}

impl QueryTab {
    /// A tab whose editor holds `text`.
    pub fn new(text: &str, window: &mut Window, cx: &mut Context<Self>) -> Self {
        // The delegate indexes rows by model column, so a dragged header
        // would end up over the wrong data.
        let results =
            cx.new(|cx| TableState::new(ResultsDelegate::default(), window, cx).col_movable(false));
        let editor = editor::new_editor(text, window, cx);
        let edits = cx.subscribe(&editor, |_, _, event: &InputEvent, cx| {
            if matches!(event, InputEvent::Change) {
                cx.emit(QueryTabEvent::Edited);
            }
        });
        Self {
            editor,
            results,
            run: None,
            pump: None,
            _edits: edits,
        }
    }

    /// The editor, so callers can focus it or read its text.
    pub fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// The editor's text.
    pub fn text(&self, cx: &App) -> String {
        self.editor.read(cx).value().to_string()
    }

    /// The latest run.
    pub fn output<'a>(&self, cx: &'a App) -> &'a RunOutput {
        self.results.read(cx).delegate().output()
    }

    /// The results grid's state.
    pub fn results(&self) -> &Entity<TableState<ResultsDelegate>> {
        &self.results
    }

    /// The SQL a Run sends: the selection, or the statement under the cursor.
    pub fn target_sql(&self, cx: &App) -> Option<String> {
        let state = self.editor.read(cx);
        let script = state.value();
        let range = target::run_target(&script, state.selected_range())?;
        script.get(range).map(str::to_owned)
    }

    /// Shows the run of `script` that `handle` follows, replacing the last.
    pub fn start(&mut self, script: String, handle: RunHandle, cx: &mut Context<Self>) {
        self.follow(script, handle.events.clone(), cx);
        self.run = Some(handle);
    }

    /// Shows the run of `script` whose events arrive on `events`.
    pub fn follow(&mut self, script: String, events: Receiver<RunEvent>, cx: &mut Context<Self>) {
        self.run = None;
        self.results.update(cx, |table, cx| {
            table.delegate_mut().reset(RunOutput::submitted(script));
            table.clear_selection(cx);
            table.scroll_to_row(0, cx);
            table.refresh(cx);
        });
        self.pump = Some(Self::pump(events, cx));
        cx.notify();
    }

    /// Cancels the tab's run, if one is going.
    pub fn cancel(&self) {
        if let Some(handle) = &self.run {
            handle.cancel();
        }
    }
}
