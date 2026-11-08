//! Every text role must stay readable on every surface, in both themes.
//!
//! Surfaces are each window-gradient stop on its own and with a glass panel
//! composited over it. Body text needs WCAG AA (4.5:1); muted text, comments
//! and line numbers need 3:1.

use quarry_ui_kit::{
    Palette, Role,
    color::{contrast, flatten},
};

const BODY: [Role; 10] = [
    Role::Text,
    Role::TextSecondary,
    Role::AccentText,
    Role::Success,
    Role::Warning,
    Role::Danger,
    Role::SyntaxKeyword,
    Role::SyntaxFunction,
    Role::SyntaxString,
    Role::SyntaxNumber,
];
const QUIET: [Role; 3] = [Role::TextMuted, Role::SyntaxComment, Role::LineNumber];

fn surfaces(palette: &Palette) -> Vec<u32> {
    let stops = [
        Role::BackgroundStart,
        Role::BackgroundMiddle,
        Role::BackgroundEnd,
    ];
    let bare = stops.map(|stop| palette.hex(stop));
    let glass = bare.map(|stop| flatten(palette.hex(Role::Panel), stop));
    bare.into_iter().chain(glass).collect()
}

fn failures(palette: &Palette, roles: &[Role], minimum: f32) -> Vec<String> {
    let mut failed = Vec::new();
    for surface in surfaces(palette) {
        for &role in roles {
            let ratio = contrast(palette.hex(role), surface);
            if ratio < minimum {
                failed.push(format!(
                    "{role:?} on #{surface:08x}: {ratio:.2} < {minimum}"
                ));
            }
        }
    }
    failed
}

#[test]
fn aurora_text_is_readable_on_every_surface() {
    let palette = Palette::aurora();
    assert_eq!(failures(&palette, &BODY, 4.5), Vec::<String>::new());
    assert_eq!(failures(&palette, &QUIET, 3.0), Vec::<String>::new());
}

#[test]
fn dawn_text_is_readable_on_every_surface() {
    let palette = Palette::dawn();
    assert_eq!(failures(&palette, &BODY, 4.5), Vec::<String>::new());
    assert_eq!(failures(&palette, &QUIET, 3.0), Vec::<String>::new());
}

#[test]
fn white_on_the_accent_gradient_is_readable_for_button_labels() {
    for palette in [Palette::aurora(), Palette::dawn()] {
        for stop in [Role::AccentStart, Role::AccentEnd] {
            let ratio = contrast(palette.hex(Role::OnAccent), palette.hex(stop));
            assert!(ratio >= 3.0, "{stop:?}: {ratio:.2}");
        }
    }
}
