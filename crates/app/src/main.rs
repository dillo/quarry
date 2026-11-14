//! Quarry: a native macOS SQL client for learning and real work.

use gpui_kit::App;
use quarry::shell::{actions, store, window};
use quarry_ui_kit::{Aurora, Preference};

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            actions::install(cx);
            window::on_last_window_closed(cx, |cx| cx.quit());
            let started = Aurora::install(Preference::Dark, cx)
                .map_err(|error| error.to_string())
                .and_then(|()| {
                    let saved = store::saved_tabs(store::default_path().as_deref());
                    window::open_main(saved, cx).map_err(|error| error.to_string())
                });
            if let Err(error) = started {
                eprintln!("Quarry could not start: {error}");
                cx.quit();
                return;
            }
            cx.activate(true);
        });
}
