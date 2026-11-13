//! The results grid: the latest run's rows in a virtualized table.

use gpui_kit::{
    AnyElement, App, Context, IntoElement, ParentElement, SharedString, Styled, Window,
    component::table::{Column, TableDelegate, TableState},
    div,
    prelude::FluentBuilder,
    px,
};
use quarry_engine::{session::RunEvent, value::Value};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{MONO_FONT, text},
};

use crate::model::{
    format,
    run::{Phase, RunModel},
};

/// Default width of a result column.
pub const COLUMN_WIDTH: f32 = 160.0;

/// Feeds the latest [`RunModel`] to the table.
#[derive(Default)]
pub struct ResultsDelegate {
    model: RunModel,
    columns: Vec<Column>,
}

impl ResultsDelegate {
    /// The run being shown.
    pub fn model(&self) -> &RunModel {
        &self.model
    }

    /// Shows `model` from scratch, e.g. when a new run starts.
    pub fn reset(&mut self, model: RunModel) {
        self.model = model;
        self.columns.clear();
    }

    /// The channel closed without `Done`; see [`RunModel::end_closed`].
    pub fn end_closed(&mut self) {
        self.model.end_closed();
    }

    /// Folds `event` in; returns whether the table's columns changed.
    pub fn apply(&mut self, event: RunEvent) -> bool {
        let reshapes = matches!(event, RunEvent::Started { .. } | RunEvent::Columns(_));
        self.model.apply(event);
        if reshapes {
            self.columns = self.model.columns().iter().map(table_column).collect();
        }
        reshapes
    }

    fn value(&self, row: usize, col: usize) -> Option<&Value> {
        self.model.rows().get(row)?.get(col)
    }
}

fn table_column(column: &quarry_engine::result::Column) -> Column {
    Column::new(column.name.clone(), column.name.clone()).width(px(COLUMN_WIDTH))
}

impl TableDelegate for ResultsDelegate {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        self.model.rows().len()
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns
            .get(col_ix)
            .cloned()
            .unwrap_or_else(|| Column::new("", ""))
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        let value = self.value(row_ix, col_ix).cloned().unwrap_or(Value::Null);
        cell(&value, cx)
    }

    fn cell_text(&self, row_ix: usize, col_ix: usize, _: &App) -> String {
        self.value(row_ix, col_ix)
            .map(format::cell_text)
            .unwrap_or_default()
    }

    fn render_empty(
        &mut self,
        _: &mut Window,
        cx: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        empty_state(self.model.phase(), cx)
    }
}

/// One cell: Menlo, numbers on the right, `NULL` dim and italic.
fn cell(value: &Value, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    div()
        .w_full()
        .font_family(MONO_FONT)
        .text_size(px(text::BODY))
        .when(format::aligns_right(value), Styled::text_right)
        .when(value.is_null(), |cell| {
            cell.italic().text_color(colors.get(Role::TextMuted))
        })
        .child(SharedString::from(format::cell_text(value)))
        .into_any_element()
}

/// What the grid says when there are no rows to show.
fn empty_state(phase: &Phase, cx: &App) -> AnyElement {
    let colors = Aurora::colors(cx);
    let (message, role) = match phase {
        Phase::Idle => ("Run a statement with ⌘↵.".to_owned(), Role::TextMuted),
        Phase::Running => ("Running…".to_owned(), Role::TextMuted),
        Phase::Finished(_) | Phase::Failed(_) => ("No rows.".to_owned(), Role::TextMuted),
    };
    div()
        .size_full()
        .flex()
        .items_center()
        .justify_center()
        .px(px(24.))
        .text_size(px(text::BODY))
        .text_color(colors.get(role))
        .child(message)
        .into_any_element()
}
