//! Opens the main window on the sample database.

use gpui_kit::{
    App, AppContext, Bounds, TitlebarOptions, WindowBackgroundAppearance, WindowBounds,
    WindowOptions, point, px, size,
};
use quarry_engine::error::EngineError;
use quarry_ui_kit::{Aurora, tokens::chrome};

use crate::{
    sample,
    views::workspace::{SavedTabs, Workspace},
};

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

/// Opens a workspace on a fresh sample database with the tabs in `saved`,
/// and focuses its editor. The window follows the system's light and dark
/// appearance.
///
/// # Errors
/// Returns the engine's error when the sample database cannot be opened.
pub fn open_main(saved: SavedTabs, cx: &mut App) -> Result<(), EngineError> {
    let session = sample::open()?;
    let opened = gpui_kit::open_window(options(cx), cx, |window, cx| {
        window
            .observe_window_appearance(|_, cx| Aurora::system_appearance_changed(cx))
            .detach();
        let workspace = cx.new(|cx| Workspace::new(session, sample::NAME, saved, window, cx));
        let editor = workspace.read(cx).editor(cx);
        editor.update(cx, |state, cx| state.focus(window, cx));
        workspace
    });
    opened.map(|_| ()).map_err(|error| EngineError::Open {
        target: "the main window".to_owned(),
        message: error.to_string(),
    })
}

/// Runs `then` when the last window closes. Quarry quits then, so it never
/// lingers with no window and no way to open one.
pub fn on_last_window_closed(cx: &mut App, then: impl Fn(&mut App) + 'static) {
    cx.on_window_closed(move |cx, _| {
        if cx.windows().is_empty() {
            then(cx);
        }
    })
    .detach();
}
