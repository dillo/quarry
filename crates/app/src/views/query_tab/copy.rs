//! Copying the grid: its selection, or the whole shown result, as text.
//!
//! The rows are formatted on the background executor and the clipboard is
//! written on the main thread.

use gpui_kit::{AppContext, ClipboardItem, Context, Task, component::table::TableSelection};
use quarry_engine::export::{Exporter, ExporterRegistry};

use super::{QueryTab, QueryTabEvent};
use crate::model::copy::{self, Copied, Selection};

/// The model's view of the grid's selection.
fn selection(table: TableSelection) -> Selection {
    match table {
        TableSelection::None => Selection::None,
        TableSelection::Row(row) => Selection::Row(row),
        TableSelection::Column(column) => Selection::Column(column),
        TableSelection::Cell(row, column) => Selection::Cell(row, column),
    }
}

impl QueryTab {
    /// Copies the grid's selection, or every kept row of the shown result,
    /// as the format named `format` (an [`ExporterRegistry`] id), and emits
    /// [`QueryTabEvent::Copied`] with the note to show.
    ///
    /// Returns the task that formats the rows and writes the clipboard, for
    /// the caller to keep; dropping it cancels the copy. It is `None` when
    /// nothing is formatted: an unknown format copies nothing, and a result
    /// without rows only emits its "Nothing to copy" note.
    #[must_use = "dropping the task cancels the copy"]
    pub fn copy(&mut self, format: &'static str, cx: &mut Context<Self>) -> Option<Task<()>> {
        let registry = ExporterRegistry::standard();
        let title = registry.get(format).map(Exporter::title)?;
        let copied = self.copied(cx);
        let note = copy::note(copied.as_ref(), title);
        let Some(copied) = copied else {
            cx.emit(QueryTabEvent::Copied(note));
            return None;
        };
        Some(cx.spawn(async move |this, cx| {
            let text = cx
                .background_spawn(async move {
                    let registry = ExporterRegistry::standard();
                    registry.get(format).map(|exporter| copied.text(exporter))
                })
                .await;
            let Some(text) = text else {
                return;
            };
            let _ = this.update(cx, |_, cx| {
                cx.write_to_clipboard(ClipboardItem::new_string(text));
                cx.emit(QueryTabEvent::Copied(note));
            });
        }))
    }

    /// What a copy would take from the shown statement, if anything.
    fn copied(&self, cx: &Context<Self>) -> Option<Copied> {
        let table = self.results.read(cx);
        let delegate = table.delegate();
        let statement = delegate.output().statement(delegate.shown())?;
        copy::copied(
            statement.columns(),
            statement.rows(),
            statement.received(),
            selection(table.selection()),
        )
    }
}
