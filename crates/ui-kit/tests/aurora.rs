//! The Aurora global in a headless GPUI app.

use gpui_kit::TestAppContext;
use quarry_ui_kit::{Appearance, Aurora, Palette, Preference};

fn install(cx: &mut TestAppContext, preference: Preference) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        Aurora::install(preference, cx)
    });
    assert_eq!(installed, Ok(()));
}

#[gpui_kit::test]
fn installs_the_preferred_theme(cx: &mut TestAppContext) {
    install(cx, Preference::Light);
    cx.update(|cx| {
        assert_eq!(Aurora::preference(cx), Preference::Light);
        assert_eq!(Aurora::appearance(cx), Appearance::Light);
        assert_eq!(Aurora::colors(cx), Palette::dawn().colors());
        assert!(!Aurora::is_transitioning(cx));
        assert!(!gpui_kit::component::Theme::global(cx).is_dark());
    });
}

#[gpui_kit::test]
fn reduced_motion_switches_themes_instantly(cx: &mut TestAppContext) {
    install(cx, Preference::Dark);
    cx.update(|cx| {
        cx.set_reduce_motion(true);
        Aurora::set_preference(Preference::Light, cx);
        assert_eq!(Aurora::colors(cx), Palette::dawn().colors());
        assert!(!Aurora::is_transitioning(cx));
        assert!(!gpui_kit::component::Theme::global(cx).is_dark());
    });
}

#[gpui_kit::test]
fn switching_with_motion_cross_fades_toward_the_target(cx: &mut TestAppContext) {
    install(cx, Preference::Dark);
    cx.update(|cx| {
        cx.set_reduce_motion(false);
        Aurora::set_preference(Preference::Light, cx);
        assert_eq!(Aurora::appearance(cx), Appearance::Light);
        assert!(Aurora::is_transitioning(cx));
        assert!(!gpui_kit::component::Theme::global(cx).is_dark());
    });
}

#[gpui_kit::test]
fn the_same_appearance_does_not_restart_a_fade(cx: &mut TestAppContext) {
    install(cx, Preference::Dark);
    cx.update(|cx| {
        cx.set_reduce_motion(false);
        Aurora::set_preference(Preference::Dark, cx);
        assert!(!Aurora::is_transitioning(cx));
        Aurora::system_appearance_changed(cx);
        assert!(
            !Aurora::is_transitioning(cx),
            "a fixed preference ignores the system"
        );
    });
}

#[gpui_kit::test]
fn system_preference_follows_the_window_appearance(cx: &mut TestAppContext) {
    install(cx, Preference::System);
    cx.update(|cx| {
        let system: Appearance = cx.window_appearance().into();
        assert_eq!(Aurora::appearance(cx), system);
    });
}
