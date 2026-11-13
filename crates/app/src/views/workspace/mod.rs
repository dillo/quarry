//! The workspace: schema sidebar, editor over results, and the status bar.
//!
//! This module owns the workspace's state and its window commands;
//! [`runner`] drives runs against the [`Session`] and [`layout`] draws it.

mod layout;
mod runner;

use std::time::Duration;

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Task, Window,
    component::{input::EditorState, table::TableState},
};
use quarry_engine::session::{RunHandle, Session};
use quarry_ui_kit::Aurora;

use super::{editor, results::ResultsDelegate, schema::SchemaTree};
use crate::{
    model::statements::RunOutput,
    shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
};

/// How often the workspace polls the session: about 30 times a second.
pub const POLL: Duration = Duration::from_millis(33);

/// Width of the open sidebar.
pub const SIDEBAR_WIDTH: f32 = 240.0;

/// The main window's content.
pub struct Workspace {
    session: Session,
    database: String,
    editor: Entity<EditorState>,
    results: Entity<TableState<ResultsDelegate>>,
    schema: Entity<SchemaTree>,
    sidebar_open: bool,
    focus: FocusHandle,
    sidebar_focus: FocusHandle,
    current: Option<RunHandle>,
    run: Option<Task<()>>,
    schema_read: Option<Task<()>>,
}

impl Workspace {
    /// A workspace on `session`, labelled `database` in the status bar.
    pub fn new(
        session: Session,
        database: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let editor = editor::new_editor(editor::WELCOME, window, cx);
        // The delegate indexes rows by model column, so a dragged header
        // would end up over the wrong data.
        let results =
            cx.new(|cx| TableState::new(ResultsDelegate::default(), window, cx).col_movable(false));
        cx.observe_window_appearance(window, |_, _, cx| Aurora::system_appearance_changed(cx))
            .detach();
        let mut workspace = Self {
            session,
            database: database.to_owned(),
            editor,
            results,
            schema: cx.new(|_| SchemaTree::default()),
            sidebar_open: true,
            focus: cx.focus_handle(),
            sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
            current: None,
            run: None,
            schema_read: None,
        };
        workspace.read_schema(cx);
        workspace
    }

    /// The editor, so callers can focus it or set its text.
    pub fn editor(&self) -> &Entity<EditorState> {
        &self.editor
    }

    /// The sidebar's schema tree.
    pub fn schema(&self) -> &Entity<SchemaTree> {
        &self.schema
    }

    /// The latest run.
    pub fn run_output<'a>(&self, cx: &'a App) -> &'a RunOutput {
        self.results.read(cx).delegate().output()
    }

    /// Whether the user can drag result columns into a new order.
    pub fn columns_movable(&self, cx: &App) -> bool {
        self.results.read(cx).col_movable
    }

    /// Whether the sidebar is shown.
    pub fn sidebar_open(&self) -> bool {
        self.sidebar_open
    }

    fn toggle_sidebar(&mut self, _: &ToggleSidebar, _: &mut Window, cx: &mut Context<Self>) {
        self.sidebar_open = !self.sidebar_open;
        cx.notify();
    }

    /// Focuses the sidebar's first table. A closed sidebar opens first, and
    /// focus moves once it has been drawn and its rows are tab stops.
    fn focus_sidebar(&mut self, _: &FocusSidebar, window: &mut Window, cx: &mut Context<Self>) {
        let container = self.sidebar_focus.clone();
        if self.sidebar_open {
            focus_first_stop(&container, window, cx);
            return;
        }
        self.sidebar_open = true;
        window.on_next_frame(move |window, cx| focus_first_stop(&container, window, cx));
        cx.notify();
    }

    fn focus_editor(&mut self, _: &FocusEditor, window: &mut Window, cx: &mut Context<Self>) {
        self.editor.update(cx, |state, cx| state.focus(window, cx));
    }

    fn cycle_theme(_: &mut Self, _: &CycleTheme, _: &mut Window, cx: &mut Context<Self>) {
        let next = Aurora::preference(cx).next();
        Aurora::set_preference(next, cx);
    }
}

/// Focuses the first keyboard stop inside `container`.
fn focus_first_stop(container: &FocusHandle, window: &mut Window, cx: &mut App) {
    window.focus(container, cx);
    window.focus_next(cx);
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
