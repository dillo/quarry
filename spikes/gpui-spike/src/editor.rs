//! Experiment (b): the gpui-component editor with SQL highlighting and no
//! completion of any kind. No completion provider is ever attached.

use gpui_kit::component::input::{Editor, EditorState};
use gpui_kit::*;

const SAMPLE: &str = "-- Which astronauts flew more than 3 missions?\nSELECT a.name, count(*) AS flights\nFROM astronauts a\nJOIN crew_assignments ca ON ca.astronaut_id = a.id\nGROUP BY a.name\nHAVING count(*) > 3\nORDER BY flights DESC;\n";

pub struct EditorSpike {
    state: Entity<EditorState>,
}

impl EditorSpike {
    pub fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        let state = cx.new(|cx| {
            EditorState::new(window, cx)
                .language("sql")
                .line_number(true)
                .default_value(SAMPLE)
        });
        Self { state }
    }
}

impl Render for EditorSpike {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
            .size_full()
            .pt(px(44.))
            .p_4()
            .bg(crate::glass::aurora_background())
            .child(crate::glass::panel().size_full().child(Editor::new(&self.state).h_full()))
    }
}
