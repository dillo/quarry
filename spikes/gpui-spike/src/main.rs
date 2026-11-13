//! Phase 0 GPUI spike: `cargo run --release -- glass|editor|table`.

mod editor;
mod fps;
mod glass;
mod table;

use gpui_kit::*;

fn main() {
    let experiment = std::env::args().nth(1).unwrap_or_else(|| "glass".into());
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(move |cx: &mut App| {
            gpui_kit::init(cx);
            match experiment.as_str() {
                "editor" => open(cx, editor::EditorSpike::new),
                "table" => open(cx, table::TableSpike::new),
                _ => open(cx, |_, _| glass::GlassSpike),
            }
            cx.activate(true);
        });
}

/// Opens the spike window: blurred background, transparent title bar.
fn open<V: Render + 'static>(
    cx: &mut App,
    build: impl FnOnce(&mut Window, &mut Context<V>) -> V + 'static,
) {
    let bounds = Bounds::centered(None, size(px(1100.), px(720.)), cx);
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_background: WindowBackgroundAppearance::Blurred,
        titlebar: Some(TitlebarOptions {
            title: Some("Quarry spike".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(14.), px(14.))),
        }),
        ..Default::default()
    };
    gpui_kit::open_window(options, cx, |window, cx| cx.new(|cx| build(window, cx)))
        .expect("open window");
}
