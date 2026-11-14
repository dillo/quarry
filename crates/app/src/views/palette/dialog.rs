//! The palette's dialog: a search field over every source's items, in
//! groups.
//!
//! The items are collected when the palette opens. Each keystroke filters
//! them again with [`palette::filter`]; gpui-component's own filtering is
//! off. Picking an item closes the dialog, then asks its source to act.

use std::rc::Rc;

use gpui_kit::{
    AnyElement, App, AppContext, Context, Entity, InteractiveElement, IntoElement, ParentElement,
    Render, SharedString, Styled, TestSupportExt, WeakEntity, Window,
    component::{
        IndexPath, WindowExt,
        command::{Command, CommandGroup, CommandItem, CommandState},
    },
    div, px,
};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{space, text},
};

use super::PaletteSourceRegistry;
use crate::{
    model::palette::{self, PaletteItem},
    shell::actions::{OpenPalette, PALETTE},
    views::workspace::Workspace,
};

/// The palette's width.
pub const PALETTE_WIDTH: f32 = 560.0;

/// What the search field says before anything is typed.
pub const PLACEHOLDER: &str = "Search tabs, tables and commands";

/// One source's matching items, under its heading.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaletteGroup {
    /// The source's heading.
    pub title: &'static str,
    /// Its matching items, best first.
    pub items: Vec<PaletteItem>,
}

/// The open palette.
pub struct Palette {
    state: Entity<CommandState>,
    workspace: WeakEntity<Workspace>,
    registry: Rc<PaletteSourceRegistry>,
    items: Vec<PaletteItem>,
    query: String,
}

impl Palette {
    /// The groups the current query shows; a source without matches has
    /// none.
    pub fn groups(&self) -> Vec<PaletteGroup> {
        let mut groups: Vec<PaletteGroup> = Vec::new();
        for item in palette::filter(&self.items, &self.query) {
            match groups.last_mut() {
                Some(group) if group.items[0].source == item.source => {
                    group.items.push(item.clone());
                }
                _ => groups.push(PaletteGroup {
                    title: self.title(item.source),
                    items: vec![item.clone()],
                }),
            }
        }
        groups
    }

    fn title(&self, source: &str) -> &'static str {
        self.registry
            .get(source)
            .map_or("", |source| source.title())
    }

    /// The search field's state, to focus it or type into it.
    pub fn state(&self) -> &Entity<CommandState> {
        &self.state
    }

    fn set_query(&mut self, query: &str, cx: &mut Context<Self>) {
        query.clone_into(&mut self.query);
        cx.notify();
    }

    /// The item at `index` among the groups shown now.
    fn item_at(&self, index: IndexPath) -> Option<PaletteItem> {
        let groups = self.groups();
        groups.get(index.section)?.items.get(index.row).cloned()
    }
}

/// Opens the palette over `workspace`'s window, listing what `registry`'s
/// sources have now, and focuses its search field.
pub fn open(
    workspace: &Workspace,
    registry: Rc<PaletteSourceRegistry>,
    window: &mut Window,
    cx: &mut Context<Workspace>,
) -> Entity<Palette> {
    let items = registry.items(workspace, cx);
    let owner = cx.weak_entity();
    let palette = cx.new(|cx| Palette {
        state: cx.new(|cx| CommandState::new(window, cx)),
        workspace: owner,
        registry,
        items,
        query: String::new(),
    });
    let shown = palette.clone();
    window.open_dialog(cx, move |dialog, _, _| {
        dialog
            .w(px(PALETTE_WIDTH))
            .centered()
            .close_button(false)
            .child(shown.clone())
    });
    let state = palette.read(cx).state.clone();
    state.update(cx, |state, cx| state.focus(window, cx));
    palette
}

/// Closes the palette, then lets the picked item's source act on it.
fn confirm(palette: &WeakEntity<Palette>, index: IndexPath, window: &mut Window, cx: &mut App) {
    let Some(palette) = palette.upgrade() else {
        return;
    };
    let picked = palette.read(cx).item_at(index);
    let workspace = palette.read(cx).workspace.clone();
    let registry = Rc::clone(&palette.read(cx).registry);
    window.close_dialog(cx);
    let Some(item) = picked else {
        return;
    };
    let _ = workspace.update(cx, |workspace, cx| {
        if let Some(source) = registry.get(item.source) {
            source.pick(&item.key, workspace, window, cx);
        }
    });
}

impl Render for Palette {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let on_query = cx.weak_entity();
        let on_confirm = cx.weak_entity();
        let command = Command::new(&self.state)
            .filterable(false)
            .bordered(false)
            .placeholder(PLACEHOLDER)
            .empty(|_, _, cx| no_matches(cx))
            .on_query(move |query, _, cx| {
                let _ = on_query.update(cx, |palette, cx| palette.set_query(query, cx));
            })
            .on_confirm(move |index, window, cx| confirm(&on_confirm, index, window, cx));
        let command = self.groups().into_iter().fold(command, |command, group| {
            command.group(command_group(group))
        });
        div()
            .id("palette")
            .key_context(PALETTE)
            .on_action(|_: &OpenPalette, window, cx| window.close_dialog(cx))
            .child(command)
            .test_support()
    }
}

fn command_group(group: PaletteGroup) -> CommandGroup {
    let items = group.items.into_iter().map(|item| {
        let label = SharedString::from(item.label);
        let detail = SharedString::from(item.detail);
        let shown = label.clone();
        CommandItem::new()
            .label(label)
            .child(move |_, cx| row(shown.clone(), detail.clone(), cx))
    });
    CommandGroup::new().label(group.title).items(items)
}

/// An item: its label, and its detail quietly on the right.
fn row(label: SharedString, detail: SharedString, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    div()
        .w_full()
        .flex()
        .justify_between()
        .gap(px(space::MD))
        .child(label)
        .child(
            div()
                .text_size(px(text::CAPTION))
                .text_color(colors.get(Role::TextMuted))
                .child(detail),
        )
        .into_any_element()
}

/// What the palette says when nothing matches.
fn no_matches(cx: &App) -> AnyElement {
    div()
        .id("palette-empty")
        .py(px(space::LG))
        .flex()
        .justify_center()
        .text_size(px(text::BODY))
        .text_color(Aurora::colors(cx).get(Role::TextMuted))
        .child("No matches")
        .test_support()
        .into_any_element()
}
