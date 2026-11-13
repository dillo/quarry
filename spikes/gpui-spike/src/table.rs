//! Experiment (c): a virtualized 1,000,000-row table with a live FPS readout.

use gpui_kit::component::table::{Column, DataTable, TableDelegate, TableState};
use gpui_kit::*;

use crate::fps::FrameCounter;

const ROWS: usize = 1_000_000;
const NAMES: [&str; 6] = ["id", "name", "nationality", "missions", "hours", "status"];

struct MillionRows {
    columns: Vec<Column>,
}

impl MillionRows {
    fn new() -> Self {
        let columns = NAMES.iter().map(|n| Column::new(*n, *n).width(px(160.))).collect();
        Self { columns }
    }
}

/// Deterministic cell text, computed on demand so no rows are stored.
fn cell(row: usize, col: usize) -> SharedString {
    match col {
        0 => row.to_string().into(),
        1 => format!("Astronaut {row}").into(),
        2 => ["Kenya", "Japan", "Brazil", "Norway"][row % 4].into(),
        3 => (row % 17).to_string().into(),
        4 => format!("{:.1}", (row % 9973) as f64 * 1.7).into(),
        _ => if row % 3 == 0 { "active" } else { "retired" }.into(),
    }
}

impl TableDelegate for MillionRows {
    fn columns_count(&self, _: &App) -> usize {
        self.columns.len()
    }

    fn rows_count(&self, _: &App) -> usize {
        ROWS
    }

    fn column(&self, col_ix: usize, _: &App) -> Column {
        self.columns[col_ix].clone()
    }

    fn render_td(
        &mut self,
        row_ix: usize,
        col_ix: usize,
        _: &mut Window,
        _: &mut Context<TableState<Self>>,
    ) -> impl IntoElement {
        cell(row_ix, col_ix)
    }
}

pub struct TableSpike {
    state: Entity<TableState<MillionRows>>,
    frames: FrameCounter,
}

impl TableSpike {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| TableState::new(MillionRows::new(), window, cx));
        Self { state, frames: FrameCounter::new() }
    }
}

impl Render for TableSpike {
    fn render(&mut self, window: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        let fps = self.frames.tick();
        // Keep rendering continuously so the counter measures sustained frame rate.
        window.request_animation_frame();
        div()
            .size_full()
            .pt(px(44.))
            .p_4()
            .flex()
            .flex_col()
            .gap_2()
            .bg(crate::glass::aurora_background())
            .child(format!("{fps} fps · {ROWS} rows"))
            .child(crate::glass::panel().flex_1().child(DataTable::new(&self.state)))
    }
}
