//! The SQL editor: highlighting and line numbers, and no completion of any
//! kind. Quarry never attaches a completion or other language provider.

use gpui_kit::{App, AppContext, Entity, Window, component::input::EditorState};

/// What a new workspace's editor starts with.
pub const WELCOME: &str = "-- Welcome to Quarry. ⌘↵ runs the statement under the cursor.\n\
SELECT name, nationality, call_sign\n\
FROM astronauts\n\
ORDER BY joined;\n";

/// A SQL editor holding `text`.
pub fn new_editor(text: &str, window: &mut Window, cx: &mut App) -> Entity<EditorState> {
    let text = text.to_owned();
    cx.new(|cx| {
        EditorState::new(window, cx)
            .language("sql")
            .line_number(true)
            .default_value(text)
    })
}
