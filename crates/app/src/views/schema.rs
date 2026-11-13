//! The sidebar's schema tree: tables and views, expandable to their columns.

use std::collections::BTreeSet;

use gpui_kit::{
    AnyElement, Context, IntoElement, ParentElement, Render, SharedString, Styled, Window, div, px,
};
use quarry_engine::{error::EngineError, schema::Schema};
use quarry_ui_kit::{
    Aurora, Role,
    components::sidebar_item::SidebarItem,
    tokens::{space, text},
};

use crate::model::schema_tree::{self, TreeRow};

/// The schema as last read, or why it could not be read.
pub struct SchemaTree {
    schema: Result<Schema, EngineError>,
    expanded: BTreeSet<String>,
}

impl Default for SchemaTree {
    fn default() -> Self {
        Self {
            schema: Ok(Schema::default()),
            expanded: BTreeSet::new(),
        }
    }
}

impl SchemaTree {
    /// Shows a freshly read schema, keeping which tables are expanded.
    pub fn set(&mut self, schema: Result<Schema, EngineError>, cx: &mut Context<Self>) {
        self.schema = schema;
        cx.notify();
    }

    /// Names of the tables whose columns are listed.
    pub fn expanded(&self) -> &BTreeSet<String> {
        &self.expanded
    }

    fn toggle(&mut self, name: &str, cx: &mut Context<Self>) {
        schema_tree::toggle(&mut self.expanded, name);
        cx.notify();
    }

    fn row(row: TreeRow, cx: &mut Context<Self>) -> AnyElement {
        match row {
            TreeRow::Table {
                name,
                detail,
                expanded,
            } => table_row(name, detail, expanded, cx),
            TreeRow::Column { name, detail } => column_row(name, detail, cx),
        }
    }
}

/// The element id of a table's row: `table:<name>`.
pub fn table_row_id(name: &str) -> SharedString {
    SharedString::from(format!("table:{name}"))
}

fn table_row(
    name: String,
    detail: String,
    expanded: bool,
    cx: &mut Context<SchemaTree>,
) -> AnyElement {
    let label = SharedString::from(name.clone());
    SidebarItem::new(table_row_id(&name), label)
        .trailing(detail)
        .selected(expanded)
        .on_click(cx.listener(move |tree, _, _, cx| tree.toggle(&name, cx)))
        .into_any_element()
}

fn column_row(name: String, detail: String, cx: &Context<SchemaTree>) -> AnyElement {
    let colors = Aurora::colors(cx);
    div()
        .flex()
        .justify_between()
        .pl(px(space::LG + space::SM))
        .pr(px(space::SM))
        .py(px(2.))
        .text_size(px(text::CAPTION))
        .text_color(colors.get(Role::TextSecondary))
        .child(name)
        .child(div().text_color(colors.get(Role::TextMuted)).child(detail))
        .into_any_element()
}

impl Render for SchemaTree {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let list = div().flex().flex_col().gap(px(1.));
        match &self.schema {
            Err(error) => list
                .text_size(px(text::SMALL))
                .text_color(colors.get(Role::Danger))
                .child(format!("Cannot read the schema: {error}")),
            Ok(schema) => {
                let rows = schema_tree::rows(schema, &self.expanded);
                let children: Vec<AnyElement> =
                    rows.into_iter().map(|row| Self::row(row, cx)).collect();
                list.children(children)
            }
        }
    }
}
