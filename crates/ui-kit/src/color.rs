//! Color math on `0xRRGGBBAA` values: conversion, blending and WCAG contrast.

use gpui_kit::{Hsla, Rgba, rgba};

/// Converts `0xRRGGBBAA` into a GPUI color.
pub fn hsla(value: u32) -> Hsla {
    rgba(value).into()
}

/// The `#rrggbbaa` form used by `gpui-component` theme files.
pub fn css(value: u32) -> String {
    format!("#{value:08x}")
}

/// Linear interpolation between two colors in RGBA space; `t` is clamped to
/// 0..=1, and the ends return `from` and `to` exactly.
pub fn mix(from: Hsla, to: Hsla, t: f32) -> Hsla {
    if t <= 0.0 {
        return from;
    }
    if t >= 1.0 {
        return to;
    }
    let (a, b) = (Rgba::from(from), Rgba::from(to));
    let channel = |x: f32, y: f32| x + (y - x) * t;
    Rgba {
        r: channel(a.r, b.r),
        g: channel(a.g, b.g),
        b: channel(a.b, b.b),
        a: channel(a.a, b.a),
    }
    .into()
}

/// WCAG 2.x contrast ratio of `foreground` drawn over the opaque `background`.
pub fn contrast(foreground: u32, background: u32) -> f32 {
    let front = luminance(over(foreground, background));
    let back = luminance(over(background, background));
    (front.max(back) + 0.05) / (front.min(back) + 0.05)
}

/// `foreground` composited over `background`, as opaque `0xRRGGBBff`.
pub fn flatten(foreground: u32, background: u32) -> u32 {
    let [r, g, b] = over(foreground, background).map(to_byte);
    u32::from_be_bytes([r, g, b, 0xff])
}

fn over(foreground: u32, background: u32) -> [f32; 3] {
    let [r, g, b, alpha] = channels(foreground);
    let [br, bg, bb, _] = channels(background);
    let blend = |front: f32, back: f32| front * alpha + back * (1.0 - alpha);
    [blend(r, br), blend(g, bg), blend(b, bb)]
}

fn channels(value: u32) -> [f32; 4] {
    value.to_be_bytes().map(|byte| f32::from(byte) / 255.0)
}

fn to_byte(channel: f32) -> u8 {
    let scaled = (channel.clamp(0.0, 1.0) * 255.0).round();
    (0..=u8::MAX)
        .find(|&byte| f32::from(byte) >= scaled)
        .unwrap_or(u8::MAX)
}

fn luminance([r, g, b]: [f32; 3]) -> f32 {
    0.2126 * linear(r) + 0.7152 * linear(g) + 0.0722 * linear(b)
}

fn linear(channel: f32) -> f32 {
    if channel <= 0.040_45 {
        channel / 12.92
    } else {
        ((channel + 0.055) / 1.055).powf(2.4)
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::Rgba;

    use super::{contrast, css, flatten, hsla, mix};

    fn close(a: f32, b: f32) -> bool {
        (a - b).abs() < 0.01
    }

    #[test]
    fn css_is_zero_padded_rgba_hex() {
        assert_eq!(css(0x0a0b_0c0d), "#0a0b0c0d");
        assert_eq!(css(0xffff_ffff), "#ffffffff");
    }

    #[test]
    fn contrast_matches_wcag_reference_values() {
        assert!(close(contrast(0x0000_00ff, 0xffff_ffff), 21.0));
        assert!(close(contrast(0xffff_ffff, 0xffff_ffff), 1.0));
        assert!(close(contrast(0x7777_77ff, 0xffff_ffff), 4.48));
        assert!(
            close(contrast(0xffff_ffff, 0x7777_77ff), 4.48),
            "contrast is symmetric"
        );
    }

    #[test]
    fn transparent_text_has_lower_contrast() {
        assert!(contrast(0xffff_ff80, 0x0000_00ff) < contrast(0xffff_ffff, 0x0000_00ff));
        assert!(close(contrast(0xffff_ff00, 0x0000_00ff), 1.0));
    }

    #[test]
    fn flatten_composites_alpha_over_the_background() {
        assert_eq!(flatten(0xffff_ff80, 0x0000_00ff), 0x8080_80ff);
        assert_eq!(flatten(0x1234_56ff, 0xffff_ffff), 0x1234_56ff);
        assert_eq!(flatten(0x1234_5600, 0xabcd_efff), 0xabcd_efff);
    }

    #[test]
    fn mix_interpolates_and_clamps() {
        let (black, white) = (hsla(0x0000_00ff), hsla(0xffff_ffff));
        let middle = Rgba::from(mix(black, white, 0.5));
        assert!(close(middle.r, 0.5) && close(middle.g, 0.5) && close(middle.b, 0.5));
        assert_eq!(
            mix(black, white, -1.0),
            black,
            "below 0 is exactly the start"
        );
        assert_eq!(mix(black, white, 0.0), black);
        assert_eq!(mix(black, white, 1.0), white);
        assert_eq!(mix(black, white, 2.0), white, "above 1 is exactly the end");
        let faded = Rgba::from(mix(hsla(0xff00_00ff), hsla(0xff00_0000), 0.25));
        assert!(close(faded.a, 0.75) && close(faded.r, 1.0));
    }
}
