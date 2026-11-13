//! Every text role must stay readable on every surface, in both themes.
//!
//! Surfaces are each window-gradient stop on its own, with panel or sidebar
//! glass over it, and with the hover, selection and active-line washes over
//! that glass. Body text needs WCAG AA (4.5:1); muted text, comments
//! and line numbers need 3:1.

use quarry_ui_kit::{
    Palette, Role,
    color::{contrast, flatten, to_hex},
    components::{badge::Tone, button::accent_hover_stops, kbd::ON_ACCENT_CHIP_WASH},
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
    let mut all: Vec<u32> = bare.to_vec();
    for glass in [Role::Panel, Role::Sidebar] {
        let glazed = bare.map(|stop| flatten(palette.hex(glass), stop));
        all.extend(glazed);
        for wash in [Role::Hover, Role::Selection, Role::ActiveLine] {
            all.extend(glazed.map(|base| flatten(palette.hex(wash), base)));
        }
    }
    all
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

/// Every fill the Primary button can show: resting stops, then hover stops.
fn primary_fills(palette: &Palette) -> Vec<u32> {
    let hover = accent_hover_stops(&palette.colors()).map(to_hex);
    let mut fills = vec![palette.hex(Role::AccentStart), palette.hex(Role::AccentEnd)];
    fills.extend(hover);
    fills
}

#[test]
fn white_on_the_primary_fill_meets_the_large_text_minimum_when_resting_and_hovered() {
    for palette in [Palette::aurora(), Palette::dawn()] {
        for fill in primary_fills(&palette) {
            let ratio = contrast(palette.hex(Role::OnAccent), fill);
            assert!(ratio >= 3.0, "#{fill:08x}: {ratio:.2}");
        }
    }
}

#[test]
fn white_on_the_dark_chip_wash_over_the_primary_fill_is_readable() {
    for palette in [Palette::aurora(), Palette::dawn()] {
        let on_accent = palette.hex(Role::OnAccent);
        for fill in primary_fills(&palette) {
            let ratio = contrast(on_accent, flatten(ON_ACCENT_CHIP_WASH, fill));
            assert!(ratio >= 3.0, "#{fill:08x}: {ratio:.2}");
        }
    }
}

const TONES: [Tone; 5] = [
    Tone::Accent,
    Tone::Success,
    Tone::Warning,
    Tone::Danger,
    Tone::Neutral,
];

fn badge_failures(palette: &Palette) -> Vec<String> {
    let colors = palette.colors();
    let mut failed = Vec::new();
    for surface in surfaces(palette) {
        for tone in TONES {
            let (wash, ink) = tone.colors(&colors);
            let ratio = contrast(to_hex(ink), flatten(to_hex(wash), surface));
            if ratio < 4.5 {
                failed.push(format!("{tone:?} on #{surface:08x}: {ratio:.2}"));
            }
        }
    }
    failed
}

#[test]
fn aurora_badges_are_readable_on_every_surface() {
    assert_eq!(badge_failures(&Palette::aurora()), Vec::<String>::new());
}

#[test]
fn dawn_badges_are_readable_on_every_surface() {
    assert_eq!(badge_failures(&Palette::dawn()), Vec::<String>::new());
}

#[test]
fn the_focus_ring_stands_out_on_every_surface() {
    for palette in [Palette::aurora(), Palette::dawn()] {
        assert_eq!(
            failures(&palette, &[Role::FocusRing], 3.0),
            Vec::<String>::new()
        );
    }
}
