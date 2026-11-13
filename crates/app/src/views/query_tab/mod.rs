//! One query tab: its editor, its latest run, and that run's results.
//!
//! The workspace owns the [`quarry_engine::session::Session`] and submits
//! runs; a tab follows its run's events (see [`pump`]) and tells the
//! workspace when the run ends.

mod explain;
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

use super::{
    editor,
    results::{self, ResultViewRegistry, ResultsDelegate},
};
use crate::model::{
    plan_tree::PlanState,
    statements::{Phase, RunOutput},
    target,
};

/// What a tab tells the workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum QueryTabEvent {
    /// The tab's run ended; the schema may have changed.
    RunEnded,
    /// The user changed the editor's text.
    Edited,
}

/// What a run was asked to run.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RunScope {
    /// The selection, or the statement under the cursor (⌘↵).
    Statement,
    /// The whole editor (⌘⇧R).
    Script,
}

/// A query tab.
pub struct QueryTab {
    editor: Entity<EditorState>,
    results: Entity<TableState<ResultsDelegate>>,
    views: ResultViewRegistry,
    view: usize,
    scope: RunScope,
    run: Option<RunHandle>,
    pump: Option<Task<()>>,
    plan: PlanState,
    planning: Option<Task<()>>,
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
            views: ResultViewRegistry::standard(),
            view: 0,
            scope: RunScope::Statement,
            run: None,
            pump: None,
            plan: PlanState::Idle,
            planning: None,
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
    pub fn start(
        &mut self,
        script: String,
        scope: RunScope,
        handle: RunHandle,
        cx: &mut Context<Self>,
    ) {
        self.follow(script, scope, handle.events.clone(), cx);
        self.run = Some(handle);
    }

    /// Shows the run of `script` whose events arrive on `events`, in the
    /// Results view.
    ///
    /// This is the seam tests use to inject a channel of their own. Such a
    /// run has no [`RunHandle`], so Cancel does nothing to it.
    pub fn follow(
        &mut self,
        script: String,
        scope: RunScope,
        events: Receiver<RunEvent>,
        cx: &mut Context<Self>,
    ) {
        self.run = None;
        self.scope = scope;
        self.select_view(results::RESULTS, cx);
        self.results.update(cx, |table, cx| {
            table.delegate_mut().reset(RunOutput::submitted(script));
            table.clear_selection(cx);
            table.scroll_to_row(0, cx);
            table.refresh(cx);
        });
        self.pump = Some(Self::pump(events, cx));
        cx.notify();
    }

    /// The views under the editor.
    pub fn views(&self) -> &ResultViewRegistry {
        &self.views
    }

    /// The id of the view shown under the editor.
    pub fn view_id(&self) -> &'static str {
        self.views.views()[self.view].id()
    }

    /// Shows the view named `id`; an unknown id changes nothing.
    pub fn select_view(&mut self, id: &str, cx: &mut Context<Self>) {
        if let Some(position) = self.views.position(id) {
            self.select_view_at(position, cx);
        }
    }

    /// Shows the view at `position` in the switch; past the end changes
    /// nothing.
    pub fn select_view_at(&mut self, position: usize, cx: &mut Context<Self>) {
        if position < self.views.views().len() {
            self.view = position;
            cx.notify();
        }
    }

    /// Shows statement `index`'s rows in the grid.
    pub fn show_statement(&mut self, index: usize, cx: &mut Context<Self>) {
        self.results.update(cx, |table, cx| {
            table.delegate_mut().show(index);
            table.clear_selection(cx);
            table.scroll_to_row(0, cx);
            table.refresh(cx);
            cx.notify();
        });
    }

    /// Cancels the tab's run, if one is going.
    ///
    /// A run still queued behind another tab's would otherwise keep reading
    /// "Waiting" until that run ended, so it is dropped on the spot: its
    /// handle and pump go, and it shows as cancelled. The session may have
    /// started it within the last poll, so it emits `RunEnded` and the
    /// workspace re-reads the schema.
    pub fn cancel(&mut self, cx: &mut Context<Self>) {
        let Some(handle) = &self.run else {
            return;
        };
        handle.cancel();
        if self.output(cx).phase() == &Phase::Waiting {
            self.run = None;
            self.pump = None;
            self.results.update(cx, |table, cx| {
                table.delegate_mut().cancel_queued();
                cx.notify();
            });
            cx.emit(QueryTabEvent::RunEnded);
            cx.notify();
        }
    }
}
