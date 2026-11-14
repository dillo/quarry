//! Copying the active tab's result, and the note in the status bar that
//! says what was copied.

use std::time::Duration;

use gpui_kit::{Context, Window};

use super::Workspace;
use crate::shell::actions::{CopyCsv, CopyJson, CopyTsv};

/// How long the status bar shows a copy's note.
pub const NOTE_TIME: Duration = Duration::from_secs(3);

impl Workspace {
    pub(super) fn copy_tsv(&mut self, _: &CopyTsv, _: &mut Window, cx: &mut Context<Self>) {
        self.copy("tsv", cx);
    }

    pub(super) fn copy_csv(&mut self, _: &CopyCsv, _: &mut Window, cx: &mut Context<Self>) {
        self.copy("csv", cx);
    }

    pub(super) fn copy_json(&mut self, _: &CopyJson, _: &mut Window, cx: &mut Context<Self>) {
        self.copy("json", cx);
    }

    /// Copies the active tab's result as `format`.
    fn copy(&mut self, format: &'static str, cx: &mut Context<Self>) {
        self.active_tab()
            .clone()
            .update(cx, |tab, cx| tab.copy(format, cx));
    }

    /// Shows `note` in the status bar for [`NOTE_TIME`], replacing any
    /// note still showing.
    pub(super) fn show_note(&mut self, note: String, cx: &mut Context<Self>) {
        self.note = Some(note);
        self.note_timer = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(NOTE_TIME).await;
            let _ = this.update(cx, |workspace, cx| {
                workspace.note = None;
                cx.notify();
            });
        }));
        cx.notify();
    }

    /// The note the status bar shows after a copy, while it shows one.
    pub fn note(&self) -> Option<&str> {
        self.note.as_deref()
    }
}
