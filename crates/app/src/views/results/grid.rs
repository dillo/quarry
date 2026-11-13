//! The grid: one statement's rows in a virtualized table.

use gpui_kit::{
    AnyElement, App, Context, IntoElement, ParentElement, SharedString, Styled, Window,
    component::table::{Column, TableDelegate, TableState},
    div,
    prelude::FluentBuilder,
    px,
};
use std::time::Instant;

use quarry_engine::{result::Row, session::RunEvent, value::Value};
use quarry_ui_kit::{
    Aurora, Role,
    tokens::{MONO_FONT, text},
};

use crate::model::{
    format,
    statements::{Phase, RunOutput, StatementOutput},
};

/// Default width of a result column.
pub const COLUMN_WIDTH: f32 = 160.0;

/// Feeds one statement of a [`RunOutput`] to the table.
#[derive(Default)]
pub struct ResultsDelegate {
    output: RunOutput,
    shown: usize,
    columns: Vec<Column>,
}

impl ResultsDelegate {
    /// The run being shown.
    pub fn output(&self) -> &RunOutput {
        &self.output
    }

    /// The index of the statement the grid shows.
    pub fn shown(&self) -> usize {
        self.shown
    }

    /// Shows `output` from scratch, e.g. when a new run starts.
    pub fn reset(&mut self, output: RunOutput) {
        self.output = output;
        self.shown = 0;
        self.columns.clear();
    }

    /// A queued run was cancelled before it started; see
    /// [`RunOutput::cancel_queued`].
    pub fn cancel_queued(&mut self) {
        self.output.cancel_queued();
    }

    /// The channel closed without `Done`; see [`RunOutput::end_closed`].
    pub fn end_closed(&mut self) {
        self.output.end_closed();
    }

    /// Folds `event` in; returns whether the table's columns changed.
    ///
    /// The grid shows the first statement that returns rows: until one
    /// does, each statement's columns take over the grid as they arrive.
    pub fn apply(&mut self, event: RunEvent, now: Instant) -> bool {
        let columns = matches!(event, RunEvent::Columns(_));
        self.output.apply(event, now);
        if !columns || !self.columns.is_empty() {
            return false;
        }
        self.shown = self.output.statements().len().saturating_sub(1);
        self.columns = self.shown_columns();
        true
    }

    /// Shows statement `index`.
    pub fn show(&mut self, index: usize) {
        self.shown = index;
        self.columns = self.shown_columns();
    }

    fn shown_columns(&self) -> Vec<Column> {
        self.output
            .statement(self.shown)
            .map(|statement| statement.columns().iter().map(table_column).collect())
            .unwrap_or_default()
    }

    fn rows(&self) -> &[Row] {
        self.output
            .statement(self.shown)
            .map(StatementOutput::rows)
            .unwrap_or_default()
    }

    fn value(&self, row: usize, col: usize) -> Option<&Value> {
        self.rows().get(row)?.get(col)
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
        self.rows().len()
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
        cell(self.value(row_ix, col_ix).unwrap_or(&Value::Null), cx)
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
        empty_state(self.output.phase(), cx)
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
        Phase::Waiting => ("Waiting for the previous run…".to_owned(), Role::TextMuted),
        Phase::Running => ("Running…".to_owned(), Role::TextMuted),
        Phase::Finished | Phase::Failed(_) => ("No rows.".to_owned(), Role::TextMuted),
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
