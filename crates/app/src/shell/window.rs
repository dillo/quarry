//! Opens the main window on the sample database.

use gpui_kit::{
    App, AppContext, Bounds, TitlebarOptions, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, point, px, size,
};
use quarry_engine::error::EngineError;
use quarry_ui_kit::tokens::chrome;

use crate::{sample, views::workspace::Workspace};

/// The main window's initial size.
pub const WINDOW_SIZE: (f32, f32) = (1200.0, 760.0);

/// Window chrome: transparent title bar, traffic lights inset, opaque
/// Aurora background (blur stays deferred, ADR 0008).
pub fn options(cx: &App) -> WindowOptions {
    let (width, height) = WINDOW_SIZE;
    let (x, y) = chrome::TRAFFIC_LIGHTS;
    WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(Bounds::centered(
            None,
            size(px(width), px(height)),
            cx,
        ))),
        window_background: WindowBackgroundAppearance::Opaque,
        titlebar: Some(TitlebarOptions {
            title: Some("Quarry".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(x), px(y))),
        }),
        ..Default::default()
    }
}

/// Opens a workspace on a fresh sample database and focuses its editor.
///
/// # Errors
/// Returns the engine's error when the sample database cannot be opened.
pub fn open_main(cx: &mut App) -> Result<(), EngineError> {
    let session = sample::open()?;
    let opened = gpui_kit::open_window(options(cx), cx, |window, cx| {
        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, window, cx));
        let editor = workspace.read(cx).editor().clone();
        editor.update(cx, |state, cx| state.focus(window, cx));
        workspace
    });
    opened.map(|_| ()).map_err(|error| EngineError::Open {
        target: "the main window".to_owned(),
        message: error.to_string(),
    })
}
