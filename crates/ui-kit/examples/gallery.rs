//! The Aurora component gallery: `cargo run -p quarry-ui-kit --example gallery`.
//!
//! Shows every component on the real window background in the current theme.
//! The segmented control switches Dark / Light / System with the cross-fade.

use gpui_kit::{
    App, AppContext, Bounds, Context, IntoElement, ParentElement, Render, Styled, TitlebarOptions,
    Window, WindowBackgroundAppearance, WindowBounds, WindowOptions, div, point, px, size,
};
use quarry_ui_kit::{
    Aurora, Preference, Role,
    components::{
        badge::{Badge, Tone},
        button::{Button, ButtonVariant},
        segmented::SegmentedControl,
        sidebar_item::SidebarItem,
        status_bar::StatusBar,
        surface::{glass_panel, sidebar_surface, window_background},
        tabs::{Tab, tab_bar},
    },
    tokens::{MONO_FONT, chrome, space, text},
};

const PREFERENCES: [Preference; 3] = [Preference::Dark, Preference::Light, Preference::System];

struct Gallery;

impl Gallery {
    /// Follows macOS light/dark changes while the "System" preference is on.
    fn new(window: &mut Window, cx: &mut Context<Self>) -> Self {
        cx.observe_window_appearance(window, |_, _, cx| Aurora::system_appearance_changed(cx))
            .detach();
        Self
    }
}

impl Render for Gallery {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        if Aurora::is_transitioning(cx) {
            window.request_animation_frame();
        }
        let colors = Aurora::colors(cx);
        div()
            .size_full()
            .flex()
            .bg(window_background(&colors))
            .text_color(colors.get(Role::Text))
            .child(sidebar(cx))
            .child(
                div()
                    .flex_1()
                    .flex()
                    .flex_col()
                    .child(workspace(cx))
                    .child(status_bar()),
            )
    }
}

fn sidebar(cx: &App) -> impl IntoElement {
    let selected = PREFERENCES
        .iter()
        .position(|p| *p == Aurora::preference(cx))
        .unwrap_or(0);
    sidebar_surface(&Aurora::colors(cx))
        .w(px(220.))
        .pt(px(chrome::TITLEBAR_HEIGHT))
        .px(px(space::MD))
        .flex()
        .flex_col()
        .gap(px(space::XS))
        .child(
            SegmentedControl::new("theme", ["Dark", "Light", "System"])
                .selected(selected)
                .on_change(|index, _, cx| Aurora::set_preference(PREFERENCES[index], cx)),
        )
        .child(
            SidebarItem::new("orbital", "🪐 Orbital Agency")
                .trailing("7/12")
                .selected(true),
        )
        .child(SidebarItem::new("vinyl", "🎧 Vinyl Vault").trailing("3/10"))
        .child(SidebarItem::new("missions", "missions").trailing("118"))
}

fn workspace(cx: &App) -> impl IntoElement {
    let colors = Aurora::colors(cx);
    div()
        .flex_1()
        .flex()
        .flex_col()
        .pt(px(chrome::TITLEBAR_HEIGHT - 4.))
        .child(
            tab_bar()
                .child(Tab::new("tab-1", "🪐 8 · Busiest crews").selected(true))
                .child(Tab::new("tab-2", "Scratch")),
        )
        .child(
            glass_panel(&colors)
                .mx(px(space::MD))
                .p(px(space::LG))
                .flex()
                .flex_col()
                .gap(px(space::MD))
                .child(code_sample(cx))
                .child(actions())
                .child(badges()),
        )
}

fn code_sample(cx: &App) -> impl IntoElement {
    let colors = Aurora::colors(cx);
    div()
        .font_family(MONO_FONT)
        .text_size(px(text::BODY))
        .child(
            div()
                .text_color(colors.get(Role::SyntaxComment))
                .child("-- Which astronauts flew more than 3 missions?"),
        )
        .child(
            div()
                .text_color(colors.get(Role::SyntaxKeyword))
                .child("SELECT name, count(*) AS flights"),
        )
        .child(
            div()
                .text_color(colors.get(Role::TextSecondary))
                .child("FROM astronauts"),
        )
}

fn actions() -> impl IntoElement {
    div()
        .flex()
        .gap(px(space::SM))
        .child(Button::new("check", "Check answer").shortcut("⌘⇧↵"))
        .child(
            Button::new("run", "Run")
                .variant(ButtonVariant::Primary)
                .shortcut("⌘↵"),
        )
        .child(Button::new("more", "More…").variant(ButtonVariant::Ghost))
}

fn badges() -> impl IntoElement {
    div()
        .flex()
        .gap(px(space::XS + 2.))
        .child(Badge::new("GROUP BY", Tone::Accent))
        .child(Badge::new("Correct", Tone::Success))
        .child(Badge::new("Hint 1 of 3", Tone::Warning))
        .child(Badge::new("Error", Tone::Danger))
        .child(Badge::new("4 rows", Tone::Neutral))
}

fn status_bar() -> impl IntoElement {
    StatusBar::new()
        .left(div().child("● Playground · orbital.db"))
        .right(div().child("4 rows · 2.1 ms · Ln 6, Col 26"))
}

fn main() {
    gpui_kit::application()
        .with_assets(gpui_kit::assets::Assets)
        .run(|cx: &mut App| {
            gpui_kit::init(cx);
            if let Err(error) = Aurora::install(Preference::Dark, cx) {
                eprintln!("{error}");
                cx.quit();
                return;
            }
            open_gallery(cx);
            cx.activate(true);
        });
}

fn open_gallery(cx: &mut App) {
    let bounds = Bounds::centered(None, size(px(1000.), px(640.)), cx);
    let (x, y) = chrome::TRAFFIC_LIGHTS;
    let options = WindowOptions {
        window_bounds: Some(WindowBounds::Windowed(bounds)),
        window_background: WindowBackgroundAppearance::Opaque,
        titlebar: Some(TitlebarOptions {
            title: Some("Aurora gallery".into()),
            appears_transparent: true,
            traffic_light_position: Some(point(px(x), px(y))),
        }),
        ..Default::default()
    };
    let opened = gpui_kit::open_window(options, cx, |window, cx| {
        cx.new(|cx| Gallery::new(window, cx))
    });
    if let Err(error) = opened {
        eprintln!("cannot open the gallery: {error}");
    }
}
