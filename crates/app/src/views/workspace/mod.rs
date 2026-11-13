//! The workspace: schema sidebar, the query tab, and the status bar.
//!
//! This module owns the workspace's state and its window commands;
//! [`runs`] submits runs and reads the schema, and [`layout`] draws it.

mod layout;
mod runs;

use std::time::Duration;

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Task, Window,
    component::input::EditorState,
};
use quarry_engine::session::Session;
use quarry_ui_kit::Aurora;

use super::{
    editor,
    query_tab::{QueryTab, QueryTabEvent},
    schema::SchemaTree,
};
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
    tab: Entity<QueryTab>,
    schema: Entity<SchemaTree>,
    sidebar_open: bool,
    focus: FocusHandle,
    sidebar_focus: FocusHandle,
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
        let tab = cx.new(|cx| QueryTab::new(editor::WELCOME, window, cx));
        cx.subscribe(&tab, |workspace, _, event, cx| match event {
            QueryTabEvent::RunEnded => workspace.read_schema(cx),
        })
        .detach();
        let mut workspace = Self {
            session,
            database: database.to_owned(),
            tab,
            schema: cx.new(|_| SchemaTree::default()),
            sidebar_open: true,
            focus: cx.focus_handle(),
            sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
            schema_read: None,
        };
        workspace.read_schema(cx);
        workspace
    }

    /// The active query tab.
    pub fn active_tab(&self) -> &Entity<QueryTab> {
        &self.tab
    }

    /// The active tab's editor.
    pub fn editor(&self, cx: &App) -> Entity<EditorState> {
        self.tab.read(cx).editor().clone()
    }

    /// The active tab's latest run.
    pub fn run_output<'a>(&self, cx: &'a App) -> &'a RunOutput {
        self.tab.read(cx).output(cx)
    }

    /// The sidebar's schema tree.
    pub fn schema(&self) -> &Entity<SchemaTree> {
        &self.schema
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
        self.editor(cx)
            .update(cx, |state, cx| state.focus(window, cx));
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
