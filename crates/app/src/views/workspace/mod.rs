//! The workspace: schema sidebar, query tabs, and the status bar.
//!
//! This module owns the workspace's state and its window commands;
//! [`tabs`] opens, closes and switches tabs, [`drafts`] saves them,
//! [`runs`] submits runs and reads the schema, and [`layout`] draws it.

mod drafts;
mod layout;
mod runs;
mod tab_strip;
mod tabs;

use std::{collections::BTreeMap, sync::Arc, time::Duration};

use gpui_kit::{
    App, AppContext, Context, Entity, FocusHandle, Focusable, Subscription, Task, Window,
    component::input::EditorState,
};
use quarry_engine::session::Session;
use quarry_store::drafts::{DraftStore, Drafts};
use quarry_ui_kit::Aurora;

pub use self::{
    drafts::{AUTOSAVE, SavedTabs},
    tab_strip::tab_element_id,
};
use super::{editor, query_tab::QueryTab, schema::SchemaTree};
use crate::{
    model::{
        statements::RunOutput,
        tabs::{Tab, TabId, TabList},
    },
    shell::actions::{CycleTheme, FocusEditor, FocusSidebar, ToggleSidebar},
};

/// How often the workspace polls the session: about 30 times a second.
pub const POLL: Duration = Duration::from_millis(33);

/// Width of the open sidebar.
pub const SIDEBAR_WIDTH: f32 = 240.0;

/// The main window's content.
///
/// Every tab in `tabs` has a view in `views`.
pub struct Workspace {
    session: Session,
    database: String,
    tabs: TabList,
    views: BTreeMap<TabId, Entity<QueryTab>>,
    store: Arc<dyn DraftStore>,
    open_problem: Option<String>,
    save_problem: Option<String>,
    autosave: Option<Task<()>>,
    saving: Option<Task<()>>,
    schema: Entity<SchemaTree>,
    sidebar_open: bool,
    focus: FocusHandle,
    sidebar_focus: FocusHandle,
    schema_read: Option<Task<()>>,
    _quit: Subscription,
}

impl Workspace {
    /// A workspace on `session`, labelled `database` in the status bar,
    /// with the tabs saved in `saved`.
    pub fn new(
        session: Session,
        database: &str,
        saved: SavedTabs,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let (drafts, load_problem) = match saved.store.load() {
            Ok(drafts) => (drafts, None),
            Err(error) => (Drafts::default(), Some(error.to_string())),
        };
        let mut workspace = Self {
            session,
            database: database.to_owned(),
            tabs: restored_tabs(&drafts),
            views: BTreeMap::new(),
            store: saved.store,
            open_problem: saved.problem.or(load_problem),
            save_problem: None,
            autosave: None,
            saving: None,
            schema: cx.new(|_| SchemaTree::default()),
            sidebar_open: true,
            focus: cx.focus_handle(),
            sidebar_focus: cx.focus_handle().tab_index(0).tab_stop(false),
            schema_read: None,
            _quit: cx.on_app_quit(|workspace, cx| {
                workspace.save_before_quit(cx);
                async {}
            }),
        };
        workspace.add_views(&drafts, window, cx);
        workspace.read_schema(cx);
        workspace
    }

    /// Creates a view per tab, with its saved text; a fresh start shows the
    /// welcome query.
    fn add_views(&mut self, drafts: &Drafts, window: &mut Window, cx: &mut Context<Self>) {
        let ids: Vec<TabId> = self.tabs.tabs().iter().map(|tab| tab.id).collect();
        for id in ids {
            let saved = drafts.tabs.iter().find(|draft| draft.id == id);
            let text = saved.map_or(editor::WELCOME, |draft| draft.text.as_str());
            self.add_view(id, text, window, cx);
        }
    }

    /// The open tabs and which is active.
    pub fn tabs(&self) -> &TabList {
        &self.tabs
    }

    /// The active tab's editor.
    pub fn editor(&self, cx: &App) -> Entity<EditorState> {
        self.active_tab().read(cx).editor().clone()
    }

    /// The active tab's latest run.
    pub fn run_output<'a>(&self, cx: &'a App) -> &'a RunOutput {
        self.active_tab().read(cx).output(cx)
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

/// The saved tabs as a [`TabList`]; none saved opens "Query 1".
fn restored_tabs(drafts: &Drafts) -> TabList {
    let tabs = drafts
        .tabs
        .iter()
        .map(|draft| Tab {
            id: draft.id,
            title: draft.title.clone(),
        })
        .collect();
    TabList::restore(tabs, drafts.active)
}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus.clone()
    }
}
