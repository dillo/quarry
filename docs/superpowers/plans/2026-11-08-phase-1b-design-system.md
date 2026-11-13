# Phase 1B: Aurora Design System Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build `quarry-ui-kit`, the Aurora design system. It contains:
- **Theme data:** the Aurora (dark) and Aurora Dawn (light) palettes, by semantic role, guarded by a WCAG contrast test;
- **Tokens:** type, spacing, radius, chrome and motion;
- **Theme switching:** a Dark / Light / System preference with a 250 ms cross-fade that respects Reduce Motion;
- **Shared theming:** a generated `gpui-component` theme, so the editor and table share the palette;
- **Components:** glass surfaces, buttons, badges, keyboard hints, tabs, a segmented control, sidebar rows and a status bar;
- **A component gallery** for visual review.

**Architecture:**
- **Colors:** each palette is a table of `0xRRGGBBAA` values keyed by `Role`. That single source feeds three things: rendering (`Colors`, as GPUI `Hsla`), the `gpui-component` theme JSON, and the contrast test.
- **Theme state:** `Aurora` is a GPUI global holding the preference, the shown appearance and an optional `Transition`. Components read `Aurora::colors(cx)` at render time.
- **Components:** each is a small `RenderOnce` builder whose style decisions are pure, unit-tested functions. Rendering, layout and clicks are tested in headless GPUI windows (`gpui-kit`'s `test-support`).

**Tech Stack:** Rust 1.98.1, `gpui-kit` =0.9.0 (GPUI + `gpui-component`; default features off, `component` on), `serde_json`, `thiserror`. Dev: `gpui-kit` features `test-support` and `assets`.

**Spec:** `docs/superpowers/specs/2026-11-04-quarry-design.md` §4.1 (visual language) and §9 (Phase 1B scope). ADR 0006 (opaque base; blur is an enhancement; Reduce Motion). The approved mockups are in `docs/design/mockups/`. **The gallery this plan builds was reviewed and approved by the user on 2026-11-08, in both themes.**

## Global Constraints

- Branch: `phase-1b-design-system`, which already exists and holds this plan. Use Conventional Commits.
- `quarry-ui-kit` depends on **no workspace crate**; `check-deps` enforces it. Its dependencies are `gpui-kit` (pinned `=0.9.0`), `serde_json` and `thiserror`, with `gpui-kit` again as a dev-dependency with the `test-support` and `assets` features.
- Coverage for `ui-kit`: at least **80% lines / 70% regions** at the end of **every** task.
- Per function: cyclomatic ≤ 10, cognitive ≤ 15, ≤ 15 statements, ≤ 40 lines, ≤ 5 arguments, nesting ≤ 4. **This applies to tests and to the gallery example too.**
- Banned outside tests: `unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`, `dbg!`. No lint suppression attributes. No `anyhow` in this crate; errors are `thiserror` enums.
- **Never use `as` casts.** Clippy pedantic rejects lossy casts. Convert with `f32::from(u8)`, `u32::to_be_bytes` and similar.
- **Every text role must keep contrast** on every surface in both themes: 4.5:1 for body text, syntax and status colors; 3:1 for muted text, comments and line numbers. `tests/contrast.rs` enforces this. Change palette values only together with that test.
- Fonts: interface `.SystemUIFont` (SF Pro), code `Menlo`. SF Mono can't be loaded by name; see ADR 0008.
- Interactive components must call `.test_support()` on their root element, plus a stable id, so headless tests can find them. `test_support` is a no-op in normal builds.
- **Every task ends with `just check` passing.** Don't commit anything under `.superpowers/`.

---

## File Structure

```
crates/ui-kit/
├─ Cargo.toml                 deps grow per task (1, 4, 5, 8)
├─ src/lib.rs                 modules + re-exports (grows per task)
├─ src/color.rs               hex → Hsla, css, mix, flatten, WCAG contrast   (Task 1)
├─ src/palette.rs             Role, Palette (Aurora, Dawn), Colors           (Task 2)
├─ src/tokens.rs              fonts, text, space, radius, chrome, motion     (Task 3)
├─ src/appearance.rs          Appearance, Preference::resolve                (Task 3)
├─ src/transition.rs          timed cross-fade                               (Task 3)
├─ src/component_theme.rs     generated gpui-component theme JSON            (Task 4)
├─ src/aurora.rs              Aurora global: install, colors, switching      (Task 5)
├─ src/components/mod.rs      component list, ClickHandler                   (Tasks 6–7)
├─ src/components/{surface,kbd,button,badge}.rs                             (Task 6)
├─ src/components/{tabs,segmented,sidebar_item,status_bar}.rs               (Task 7)
├─ tests/contrast.rs          WCAG gate                                      (Task 2)
├─ tests/aurora.rs            the global in a headless app                   (Task 5)
├─ tests/components.rs        headless render, layout and clicks             (Tasks 6–7)
└─ examples/gallery.rs        the visual review window                       (Task 8)
deny.toml                     GPUI supply-chain policy                       (Task 1)
docs/adr/0008-aurora-design-system.md                                        (Task 8)
```

---

### Task 1: Bring GPUI into the workspace, set the supply-chain policy, add color math

**Files:**
- Modify: root `Cargo.toml`, `crates/ui-kit/Cargo.toml`, `crates/ui-kit/src/lib.rs`, `deny.toml`
- Create: `crates/ui-kit/src/color.rs` (unit tests inside)

**Interfaces (produced):**
- `color::hsla(u32) -> Hsla`
- `color::css(u32) -> String`, which gives `#rrggbbaa`
- `color::mix(Hsla, Hsla, f32) -> Hsla`, clamped, and exact at the ends
- `color::contrast(fg: u32, bg: u32) -> f32`, the WCAG ratio after compositing `fg`'s alpha over `bg`
- `color::flatten(fg: u32, bg: u32) -> u32`, opaque

**Supply-chain facts (verified):**
- Adding `gpui-kit` brings in about 600 crates.
- With `cargo deny`'s graph restricted to `aarch64-apple-darwin`, it reports the following:
  - **27 duplicate versions,** all inside GPUI's own tree. Resolved with `skip-tree` on `gpui-kit`.
  - **4 unmaintained advisories,** none vulnerabilities, all transitive through GPUI: RUSTSEC-2024-0384 `instant`, RUSTSEC-2024-0436 `paste`, RUSTSEC-2026-0206 `rustybuzz` and RUSTSEC-2026-0192 `ttf-parser`. Each is ignored with a reason.
  - **Licenses 0BSD, CC0-1.0 and MPL-2.0.** All three were allowed by the user on 2026-11-08.

- [ ] **Step 1: Switch to the branch**

```bash
git checkout phase-1b-design-system && git log --oneline -1
```

- [ ] **Step 2: Add the dependency.** In the root `Cargo.toml`, add this under `[workspace.dependencies]`, keeping the list alphabetical:

```toml
gpui-kit = { version = "=0.9.0", default-features = false, features = ["component"] }
```

In `crates/ui-kit/Cargo.toml`, add this before `[lints]`:

```toml
[dependencies]
gpui-kit.workspace = true

```

- [ ] **Step 3: Replace `deny.toml`** with the following. It's the existing policy plus the target restriction, the reasoned ignores, the three licenses and the `skip-tree`:

```toml
# cargo-deny: vulnerabilities, licenses, duplicate versions, and sources.
# Every skip or ignore added here is a reviewed exception: say why inline.

[graph]
all-features = true
# Quarry ships only for Apple Silicon macOS; other platforms' deps are out of scope.
targets = ["aarch64-apple-darwin"]

[advisories]
yanked = "deny"
# Unmaintained (not vulnerable) crates that only GPUI pulls in. We cannot
# replace them ourselves; revisit each on every gpui-kit upgrade.
ignore = [
    { id = "RUSTSEC-2024-0384", reason = "instant: unmaintained, transitive via gpui-kit, no vulnerability" },
    { id = "RUSTSEC-2024-0436", reason = "paste: unmaintained, transitive via gpui-kit, no vulnerability" },
    { id = "RUSTSEC-2026-0206", reason = "rustybuzz: unmaintained, transitive via gpui-kit, no vulnerability" },
    { id = "RUSTSEC-2026-0192", reason = "ttf-parser: unmaintained, transitive via gpui-kit, no vulnerability" },
]

[licenses]
allow = [
    "MIT",
    "Apache-2.0",
    "Apache-2.0 WITH LLVM-exception",
    "BSD-2-Clause",
    "BSD-3-Clause",
    "ISC",
    "Unicode-3.0",
    "Zlib",
    # Permissive / public-domain-style, brought in by GPUI (0BSD: enum-iterator, CC0: notify).
    "0BSD",
    "CC0-1.0",
    # Weak (file-level) copyleft, brought in by GPUI (cbindgen at build time, option-ext).
    # Accepted 2026-11-08: Quarry's own code stays MIT OR Apache-2.0.
    "MPL-2.0",
]
confidence-threshold = 0.9
# The allow-list anticipates later phases; not every license is used yet.
unused-allowed-license = "allow"

[licenses.private]
ignore = true

[bans]
multiple-versions = "deny"
wildcards = "deny"
skip = []
# GPUI's own dependency tree carries many duplicate versions we cannot unify.
# Duplicates anywhere else in the graph still fail.
skip-tree = [{ crate = "gpui-kit", reason = "duplicates inside GPUI's tree are upstream's to resolve" }]

[sources]
unknown-registry = "deny"
unknown-git = "deny"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
```

- [ ] **Step 4: Write `color.rs` test-first.**
1. Create `crates/ui-kit/src/color.rs` with only its `#[cfg(test)] mod tests` block, plus stub functions that return `0`, `String::new()` or their first argument.
2. Set `crates/ui-kit/src/lib.rs` to:

   ```rust
   //! The Aurora design system: theme tokens and reusable components.

   pub mod color;
   ```

3. Run `cargo nextest run -p quarry-ui-kit`. Expect the tests to FAIL against the stubs. The first build compiles GPUI, which takes a few minutes.
4. Implement. The final file:

```rust
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
```

- [ ] **Step 5: Verify**

Run: `cargo nextest run -p quarry-ui-kit`
Expected: 5 tests pass.
Run: `just check`
Expected: exit 0, with `cargo deny` printing `advisories ok, bans ok, licenses ok, sources ok`. Locally, `just check` takes about 1.5 minutes now that GPUI is in the graph.

- [ ] **Step 6: Commit**

```bash
git add Cargo.toml Cargo.lock deny.toml crates/ui-kit
git commit -m "build(ui-kit): add GPUI with a reviewed supply-chain policy and color math"
```

---

### Task 2: Palettes by role, guarded by a WCAG contrast test

**Files:**
- Create: `crates/ui-kit/src/palette.rs` (unit tests inside), `crates/ui-kit/tests/contrast.rs`
- Modify: `crates/ui-kit/src/lib.rs`

**Interfaces:**
- `Role` has 26 variants, and `Role::ALL` lists them in declaration order.
- `Palette::aurora()` and `Palette::dawn()`.
- `Palette::hex(Role) -> u32` and `Palette::colors() -> Colors`.
- `Colors::get(Role) -> Hsla` and `Colors::mix(&Colors, &Colors, f32) -> Colors`.

**Palette values** were taken from the approved mockups, then tuned until every role passes `tests/contrast.rs`. Compared with the mockups, Dawn's status and syntax colors are darker, and muted/secondary text is more opaque in both themes. Don't change any value without re-running the contrast test.

- [ ] **Step 1: Write the contrast gate first.** Create `crates/ui-kit/tests/contrast.rs`:

```rust
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
```

Run `cargo nextest run -p quarry-ui-kit --test contrast`. Expected: it fails to compile, because `Palette` and `Role` don't exist yet.

- [ ] **Step 2: Implement `palette.rs`, including its unit tests:**

```rust
//! The Aurora (dark) and Aurora Dawn (light) palettes, by semantic role.

use gpui_kit::Hsla;

use crate::color;

/// What a color is for. Components ask for roles, never raw colors.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Window gradient, first stop.
    BackgroundStart,
    /// Window gradient, middle stop.
    BackgroundMiddle,
    /// Window gradient, last stop.
    BackgroundEnd,
    /// Sidebar and inspector glass.
    Sidebar,
    /// Floating panel glass (editor, results).
    Panel,
    /// Panel and divider borders.
    Border,
    /// Hover wash on interactive items.
    Hover,
    /// Primary text.
    Text,
    /// Secondary text (labels, metadata).
    TextSecondary,
    /// Muted text (captions, placeholders).
    TextMuted,
    /// Accent gradient, first stop.
    AccentStart,
    /// Accent gradient, last stop.
    AccentEnd,
    /// Accent-colored text and icons.
    AccentText,
    /// Text on the accent gradient.
    OnAccent,
    /// Selected-item wash.
    Selection,
    /// Selected-item border.
    SelectionBorder,
    /// Success (correct answer, connected).
    Success,
    /// Warning (hints, amber states).
    Warning,
    /// Errors and destructive actions.
    Danger,
    /// SQL keywords.
    SyntaxKeyword,
    /// SQL functions.
    SyntaxFunction,
    /// String literals.
    SyntaxString,
    /// Numeric literals.
    SyntaxNumber,
    /// Comments.
    SyntaxComment,
    /// Editor line numbers.
    LineNumber,
    /// Editor current-line wash.
    ActiveLine,
}

impl Role {
    /// Every role, in declaration order.
    pub const ALL: [Self; 26] = [
        Self::BackgroundStart,
        Self::BackgroundMiddle,
        Self::BackgroundEnd,
        Self::Sidebar,
        Self::Panel,
        Self::Border,
        Self::Hover,
        Self::Text,
        Self::TextSecondary,
        Self::TextMuted,
        Self::AccentStart,
        Self::AccentEnd,
        Self::AccentText,
        Self::OnAccent,
        Self::Selection,
        Self::SelectionBorder,
        Self::Success,
        Self::Warning,
        Self::Danger,
        Self::SyntaxKeyword,
        Self::SyntaxFunction,
        Self::SyntaxString,
        Self::SyntaxNumber,
        Self::SyntaxComment,
        Self::LineNumber,
        Self::ActiveLine,
    ];

    fn index(self) -> usize {
        self as usize
    }
}

const AURORA: [(Role, u32); 26] = [
    (Role::BackgroundStart, 0x1b15_30ff),
    (Role::BackgroundMiddle, 0x0e14_24ff),
    (Role::BackgroundEnd, 0x0b1b_22ff),
    (Role::Sidebar, 0xffff_ff09),
    (Role::Panel, 0xffff_ff08),
    (Role::Border, 0xffff_ff10),
    (Role::Hover, 0xffff_ff0f),
    (Role::Text, 0xe6e4_f0ff),
    (Role::TextSecondary, 0xe6e4_f0b3),
    (Role::TextMuted, 0xe6e4_f08c),
    (Role::AccentStart, 0x8b6c_ffff),
    (Role::AccentEnd, 0x5b8c_ffff),
    (Role::AccentText, 0xb79c_ffff),
    (Role::OnAccent, 0xffff_ffff),
    (Role::Selection, 0x8b6c_ff47),
    (Role::SelectionBorder, 0xa087_ff40),
    (Role::Success, 0x5eea_d4ff),
    (Role::Warning, 0xfbbf_24ff),
    (Role::Danger, 0xfb92_3cff),
    (Role::SyntaxKeyword, 0xb79c_ffff),
    (Role::SyntaxFunction, 0x5eea_d4ff),
    (Role::SyntaxString, 0xfbbf_77ff),
    (Role::SyntaxNumber, 0xf9a8_d4ff),
    (Role::SyntaxComment, 0xe6e4_f080),
    (Role::LineNumber, 0xe6e4_f066),
    (Role::ActiveLine, 0xffff_ff0a),
];

const DAWN: [(Role, u32); 26] = [
    (Role::BackgroundStart, 0xf3ee_ffff),
    (Role::BackgroundMiddle, 0xeef3_ffff),
    (Role::BackgroundEnd, 0xe9f8_f6ff),
    (Role::Sidebar, 0xffff_ff73),
    (Role::Panel, 0xffff_ffb8),
    (Role::Border, 0x6e5a_c81a),
    (Role::Hover, 0x6e5a_c80f),
    (Role::Text, 0x2420_3aff),
    (Role::TextSecondary, 0x2420_3ab3),
    (Role::TextMuted, 0x2420_3a99),
    (Role::AccentStart, 0x8b6c_ffff),
    (Role::AccentEnd, 0x5b8c_ffff),
    (Role::AccentText, 0x6d4e_e8ff),
    (Role::OnAccent, 0xffff_ffff),
    (Role::Selection, 0x8b6c_ff2e),
    (Role::SelectionBorder, 0x8b6c_ff40),
    (Role::Success, 0x0f76_6eff),
    (Role::Warning, 0xa150_0aff),
    (Role::Danger, 0xc241_0cff),
    (Role::SyntaxKeyword, 0x6d4e_e8ff),
    (Role::SyntaxFunction, 0x0f76_6eff),
    (Role::SyntaxString, 0xa150_0aff),
    (Role::SyntaxNumber, 0xbe2a_7eff),
    (Role::SyntaxComment, 0x6b65_85ff),
    (Role::LineNumber, 0x2420_3a80),
    (Role::ActiveLine, 0x8b6c_ff0f),
];

/// A theme's colors as `0xRRGGBBAA` values, one per [`Role`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Palette {
    colors: [u32; Role::ALL.len()],
}

impl Palette {
    /// Aurora, the dark default theme.
    pub fn aurora() -> Self {
        Self::from_pairs(&AURORA)
    }

    /// Aurora Dawn, the light theme.
    pub fn dawn() -> Self {
        Self::from_pairs(&DAWN)
    }

    /// The `0xRRGGBBAA` value for `role`.
    pub fn hex(&self, role: Role) -> u32 {
        self.colors[role.index()]
    }

    /// The renderable colors.
    pub fn colors(&self) -> Colors {
        Colors {
            colors: self.colors.map(color::hsla),
        }
    }

    fn from_pairs(pairs: &[(Role, u32); Role::ALL.len()]) -> Self {
        let mut colors = [0; Role::ALL.len()];
        for (role, value) in pairs {
            colors[role.index()] = *value;
        }
        Self { colors }
    }
}

/// Renderable colors, possibly mid-way between two palettes.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Colors {
    colors: [Hsla; Role::ALL.len()],
}

impl Colors {
    /// The color for `role`.
    pub fn get(&self, role: Role) -> Hsla {
        self.colors[role.index()]
    }

    /// The colors `t` of the way from `from` to `to` (0 = `from`, 1 = `to`).
    pub fn mix(from: &Self, to: &Self, t: f32) -> Self {
        let mut colors = from.colors;
        for (index, color) in colors.iter_mut().enumerate() {
            *color = color::mix(*color, to.colors[index], t);
        }
        Self { colors }
    }
}

#[cfg(test)]
mod tests {
    use super::{AURORA, Colors, DAWN, Palette, Role};
    use crate::color;

    #[test]
    fn every_role_is_defined_exactly_once_in_each_palette() {
        for table in [AURORA, DAWN] {
            for (index, role) in Role::ALL.iter().enumerate() {
                assert_eq!(table[index].0, *role, "table order must follow Role::ALL");
            }
        }
    }

    #[test]
    fn palettes_read_back_their_values() {
        assert_eq!(Palette::aurora().hex(Role::Text), 0xe6e4_f0ff);
        assert_eq!(Palette::dawn().hex(Role::Text), 0x2420_3aff);
        assert_eq!(
            Palette::aurora().colors().get(Role::OnAccent),
            color::hsla(0xffff_ffff)
        );
    }

    #[test]
    fn mixing_moves_every_role_between_palettes() {
        let (dark, light) = (Palette::aurora().colors(), Palette::dawn().colors());
        assert_eq!(Colors::mix(&dark, &light, 0.0), dark);
        assert_eq!(Colors::mix(&dark, &light, 1.0), light);
        let half = Colors::mix(&dark, &light, 0.5);
        for role in Role::ALL {
            let expected = color::mix(dark.get(role), light.get(role), 0.5);
            assert_eq!(half.get(role), expected, "{role:?}");
        }
    }
}
```

Set `lib.rs` to declare `pub mod color;` and `pub mod palette;`, and add `pub use palette::{Colors, Palette, Role};`.

- [ ] **Step 3: Verify**

Run: `cargo nextest run -p quarry-ui-kit`
Expected: all tests pass, including the 3 contrast tests and the 3 palette tests.
Run: `just check`
Expected: exit 0.

- [ ] **Step 4: Commit**

```bash
git add crates/ui-kit
git commit -m "feat(ui-kit): add Aurora and Aurora Dawn palettes with a WCAG contrast gate"
```

---

### Task 3: Tokens, appearance preference and the cross-fade

**Files:**
- Create: `crates/ui-kit/src/tokens.rs`, `appearance.rs` and `transition.rs` (unit tests are inside the latter two)
- Modify: `crates/ui-kit/src/lib.rs`

**Interfaces:**
- `tokens::{UI_FONT, MONO_FONT}`
- `tokens::text::{CAPTION, SMALL, BODY, TITLE, DISPLAY}`
- `tokens::space::{XS, SM, MD, LG, XL}`
- `tokens::radius::{SM, MD, LG, PILL}`
- `tokens::chrome::{TITLEBAR_HEIGHT, TRAFFIC_LIGHTS}`
- `tokens::motion::{FAST, THEME}`
- `Appearance { Dark, Light }`, including `From<gpui_kit::WindowAppearance>`
- `Preference { Dark (default), Light, System }` with `resolve(Appearance) -> Appearance`
- `Transition::new(from, to, started: Instant, duration: Duration)`, with `colors(now)`, `is_done(now)` and `target()`

- [ ] **Step 1: Write the tests first.** Write the `#[cfg(test)]` blocks of `appearance.rs` and `transition.rs` against stub signatures, run them and see them fail, then implement. The final files:

`crates/ui-kit/src/tokens.rs` (constants only):
```rust
//! Non-color design tokens: type, spacing, radii and motion.

use std::time::Duration;

/// The interface font: SF Pro, through GPUI's system-font alias.
pub const UI_FONT: &str = ".SystemUIFont";

/// The code font. SF Mono is not loadable by name, so Quarry uses Menlo,
/// which ships with every Mac (see ADR 0008).
pub const MONO_FONT: &str = "Menlo";

/// Type scale in points.
pub mod text {
    /// Captions, badges, keyboard hints.
    pub const CAPTION: f32 = 11.0;
    /// Sidebar items, tabs, status bar.
    pub const SMALL: f32 = 12.0;
    /// Body text and the editor.
    pub const BODY: f32 = 13.0;
    /// Panel titles.
    pub const TITLE: f32 = 17.0;
    /// The Home greeting.
    pub const DISPLAY: f32 = 24.0;
}

/// Spacing on a 4-point grid.
pub mod space {
    /// 4 points.
    pub const XS: f32 = 4.0;
    /// 8 points.
    pub const SM: f32 = 8.0;
    /// 12 points.
    pub const MD: f32 = 12.0;
    /// 16 points.
    pub const LG: f32 = 16.0;
    /// 24 points.
    pub const XL: f32 = 24.0;
}

/// Corner radii.
pub mod radius {
    /// Items, buttons, badges.
    pub const SM: f32 = 6.0;
    /// Panels.
    pub const MD: f32 = 10.0;
    /// Cards and the window.
    pub const LG: f32 = 14.0;
    /// Pills (badges, segmented control thumb).
    pub const PILL: f32 = 999.0;
}

/// Window chrome.
pub mod chrome {
    /// Height reserved at the top of the window for the traffic lights.
    pub const TITLEBAR_HEIGHT: f32 = 40.0;
    /// Where the traffic lights sit, from the window's top-left corner.
    pub const TRAFFIC_LIGHTS: (f32, f32) = (14.0, 14.0);
}

/// Animation durations.
pub mod motion {
    use super::Duration;

    /// Hover and press feedback.
    pub const FAST: Duration = Duration::from_millis(150);
    /// The theme cross-fade.
    pub const THEME: Duration = Duration::from_millis(250);
}
```

`crates/ui-kit/src/appearance.rs`:
```rust
//! Which theme to show: the user's preference resolved against the system.

/// A concrete appearance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Appearance {
    /// Aurora.
    Dark,
    /// Aurora Dawn.
    Light,
}

/// The user's theme setting.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Preference {
    /// Always Aurora (the default).
    #[default]
    Dark,
    /// Always Aurora Dawn.
    Light,
    /// Follow macOS.
    System,
}

impl Preference {
    /// The appearance to show when macOS is currently `system`.
    pub fn resolve(self, system: Appearance) -> Appearance {
        match self {
            Self::Dark => Appearance::Dark,
            Self::Light => Appearance::Light,
            Self::System => system,
        }
    }
}

impl From<gpui_kit::WindowAppearance> for Appearance {
    fn from(appearance: gpui_kit::WindowAppearance) -> Self {
        match appearance {
            gpui_kit::WindowAppearance::Dark | gpui_kit::WindowAppearance::VibrantDark => {
                Self::Dark
            }
            gpui_kit::WindowAppearance::Light | gpui_kit::WindowAppearance::VibrantLight => {
                Self::Light
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use gpui_kit::WindowAppearance;

    use super::{Appearance, Preference};

    #[test]
    fn fixed_preferences_ignore_the_system() {
        for system in [Appearance::Dark, Appearance::Light] {
            assert_eq!(Preference::Dark.resolve(system), Appearance::Dark);
            assert_eq!(Preference::Light.resolve(system), Appearance::Light);
            assert_eq!(Preference::System.resolve(system), system);
        }
        assert_eq!(Preference::default(), Preference::Dark);
    }

    #[test]
    fn vibrant_system_appearances_map_to_their_base() {
        assert_eq!(
            Appearance::from(WindowAppearance::VibrantDark),
            Appearance::Dark
        );
        assert_eq!(Appearance::from(WindowAppearance::Dark), Appearance::Dark);
        assert_eq!(
            Appearance::from(WindowAppearance::VibrantLight),
            Appearance::Light
        );
        assert_eq!(Appearance::from(WindowAppearance::Light), Appearance::Light);
    }
}
```

`crates/ui-kit/src/transition.rs`:
```rust
//! The timed cross-fade between two palettes.

use std::time::{Duration, Instant};

use crate::palette::Colors;

/// A cross-fade from one set of colors to another.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Transition {
    from: Colors,
    to: Colors,
    started: Instant,
    duration: Duration,
}

impl Transition {
    /// Starts fading at `started`; a zero `duration` switches instantly.
    pub fn new(from: Colors, to: Colors, started: Instant, duration: Duration) -> Self {
        Self {
            from,
            to,
            started,
            duration,
        }
    }

    /// The colors to draw at `now`.
    pub fn colors(&self, now: Instant) -> Colors {
        Colors::mix(&self.from, &self.to, ease_out(self.progress(now)))
    }

    /// Whether the fade has finished at `now`.
    pub fn is_done(&self, now: Instant) -> bool {
        self.progress(now) >= 1.0
    }

    /// The colors the fade ends on.
    pub fn target(&self) -> Colors {
        self.to
    }

    fn progress(&self, now: Instant) -> f32 {
        if self.duration.is_zero() {
            return 1.0;
        }
        let elapsed = now.saturating_duration_since(self.started);
        (elapsed.as_secs_f32() / self.duration.as_secs_f32()).min(1.0)
    }
}

/// Cubic ease-out: fast start, gentle landing.
fn ease_out(t: f32) -> f32 {
    1.0 - (1.0 - t).powi(3)
}

#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use super::{Transition, ease_out};
    use crate::palette::{Colors, Palette};

    fn fade(duration: Duration) -> (Transition, Instant, Colors, Colors) {
        let (from, to) = (Palette::aurora().colors(), Palette::dawn().colors());
        let start = Instant::now();
        (Transition::new(from, to, start, duration), start, from, to)
    }

    #[test]
    fn fades_from_start_to_target_over_its_duration() {
        let (fade, start, from, to) = fade(Duration::from_millis(200));
        assert_eq!(fade.colors(start), from);
        assert!(!fade.is_done(start + Duration::from_millis(100)));
        let midway = fade.colors(start + Duration::from_millis(100));
        assert_eq!(midway, Colors::mix(&from, &to, ease_out(0.5)));
        assert_eq!(fade.colors(start + Duration::from_millis(500)), to);
        assert!(fade.is_done(start + Duration::from_millis(200)));
        assert_eq!(fade.target(), to);
    }

    #[test]
    fn a_zero_duration_switches_instantly() {
        let (fade, start, _, to) = fade(Duration::ZERO);
        assert_eq!(fade.colors(start), to);
        assert!(fade.is_done(start));
    }

    #[test]
    fn times_before_the_start_show_the_start_colors() {
        let (fade, start, from, _) = fade(Duration::from_millis(200));
        let earlier = start
            .checked_sub(Duration::from_millis(50))
            .unwrap_or(start);
        assert_eq!(fade.colors(earlier), from);
    }

    #[test]
    fn ease_out_is_monotonic_with_fixed_ends() {
        assert!((ease_out(0.0)).abs() < f32::EPSILON);
        assert!((ease_out(1.0) - 1.0).abs() < f32::EPSILON);
        assert!(ease_out(0.5) > 0.5, "ease-out runs ahead of linear");
        assert!(ease_out(0.25) < ease_out(0.75));
    }
}
```

Add `pub mod appearance; pub mod tokens; pub mod transition;` to `lib.rs`, keeping the module list alphabetical, and add `pub use appearance::{Appearance, Preference};`.

- [ ] **Step 2: Verify:** `cargo nextest run -p quarry-ui-kit` (all pass), then `just check` (exit 0).

- [ ] **Step 3: Commit**

```bash
git add crates/ui-kit
git commit -m "feat(ui-kit): add design tokens, theme preference and the cross-fade"
```

---

### Task 4: The generated `gpui-component` theme

**Files:**
- Create: `crates/ui-kit/src/component_theme.rs` (unit tests inside)
- Modify: `crates/ui-kit/Cargo.toml` (`serde_json.workspace = true` under `[dependencies]`), `lib.rs` (`pub mod component_theme;`)

**Interfaces:**
- `component_theme::{DARK_NAME = "Aurora", LIGHT_NAME = "Aurora Dawn"}`
- `component_theme::theme_set_json() -> String`
- **Facts (verified against `gpui-component` 0.9.0):**
  - A theme set is `{"name", "themes": [ThemeConfig…]}`. Each config has `name`, `mode` (`"dark"` or `"light"`), `font.family`, `font.size`, `mono_font.family`, `mono_font.size` and **`radius`, which must be an integer.** Sending `6.0` fails to load with "invalid type: floating point".
  - Colors are named by key, for example `background`, `foreground`, `border`, `primary.background`, `selection.background`, `list.active.background` and `table.row.border`.
  - `highlight` holds `editor.*` keys plus `syntax.<scope>` as `{ "color", "font_style" }`.

- [ ] **Step 1: Write the unit tests first** (the module's `#[cfg(test)]` block) against a stub `theme_set_json()` that returns `"{}"`. Run them and see them fail. Then implement. The final file:

```rust
//! Generates `gpui-component`'s theme from the Aurora palettes, so the
//! editor, table and inputs share Quarry's colors (one source of truth).

use serde_json::{Value, json};

use crate::{
    color::{css, flatten},
    palette::{Palette, Role},
    tokens::{MONO_FONT, UI_FONT, text},
};

/// Registered name of the dark theme.
pub const DARK_NAME: &str = "Aurora";
/// Registered name of the light theme.
pub const LIGHT_NAME: &str = "Aurora Dawn";

/// `gpui-component` reads its radius as a whole number; kept equal to
/// [`radius::SM`] by a test.
const COMPONENT_RADIUS: u16 = 6;

/// The theme set (both themes) as `gpui-component` theme JSON.
pub fn theme_set_json() -> String {
    json!({
        "name": "Quarry Aurora",
        "themes": [
            config(DARK_NAME, "dark", &Palette::aurora()),
            config(LIGHT_NAME, "light", &Palette::dawn()),
        ],
    })
    .to_string()
}

fn config(name: &str, mode: &str, palette: &Palette) -> Value {
    json!({
        "name": name,
        "mode": mode,
        "font.family": UI_FONT,
        "font.size": text::BODY,
        "mono_font.family": MONO_FONT,
        "mono_font.size": text::BODY,
        "radius": COMPONENT_RADIUS,
        "colors": colors(palette),
        "highlight": highlight(palette),
    })
}

fn colors(p: &Palette) -> Value {
    let c = |role| css(p.hex(role));
    let surface = css(flatten(p.hex(Role::Panel), p.hex(Role::BackgroundMiddle)));
    json!({
        "background": surface,
        "foreground": c(Role::Text),
        "border": c(Role::Border),
        "input.border": c(Role::Border),
        "muted.background": c(Role::Hover),
        "muted.foreground": c(Role::TextMuted),
        "accent.background": c(Role::Hover),
        "accent.foreground": c(Role::Text),
        "primary.background": c(Role::AccentStart),
        "primary.foreground": c(Role::OnAccent),
        "ring": c(Role::AccentStart),
        "caret": c(Role::AccentText),
        "selection.background": c(Role::Selection),
        "list.active.background": c(Role::Selection),
        "list.active.border": c(Role::SelectionBorder),
        "table.background": "#00000000",
        "table.head.foreground": c(Role::TextSecondary),
        "table.row.border": c(Role::Border),
        "scrollbar.thumb.background": c(Role::Border),
        "danger.background": c(Role::Danger),
        "success.background": c(Role::Success),
        "warning.background": c(Role::Warning),
    })
}

fn highlight(p: &Palette) -> Value {
    let c = |role| css(p.hex(role));
    json!({
        "editor.background": "#00000000",
        "editor.foreground": c(Role::Text),
        "editor.line_number": c(Role::LineNumber),
        "editor.active_line_number": c(Role::TextSecondary),
        "editor.active_line.background": c(Role::ActiveLine),
        "syntax": {
            "keyword": { "color": c(Role::SyntaxKeyword) },
            "function": { "color": c(Role::SyntaxFunction) },
            "string": { "color": c(Role::SyntaxString) },
            "number": { "color": c(Role::SyntaxNumber) },
            "boolean": { "color": c(Role::SyntaxNumber) },
            "type": { "color": c(Role::SyntaxKeyword) },
            "comment": { "color": c(Role::SyntaxComment), "font_style": "italic" },
        },
    })
}

#[cfg(test)]
mod tests {
    use serde_json::Value;

    use super::{COMPONENT_RADIUS, DARK_NAME, LIGHT_NAME, theme_set_json};
    use crate::{
        color::css,
        palette::{Palette, Role},
        tokens::radius,
    };

    #[test]
    fn component_radius_matches_the_small_radius_token() {
        assert!((f32::from(COMPONENT_RADIUS) - radius::SM).abs() < f32::EPSILON);
    }

    fn themes() -> Result<Vec<Value>, serde_json::Error> {
        let set: Value = serde_json::from_str(&theme_set_json())?;
        Ok(set["themes"].as_array().cloned().unwrap_or_default())
    }

    #[test]
    fn defines_a_dark_and_a_light_theme() -> Result<(), serde_json::Error> {
        let themes = themes()?;
        let modes: Vec<(&str, &str)> = themes
            .iter()
            .map(|t| {
                (
                    t["name"].as_str().unwrap_or(""),
                    t["mode"].as_str().unwrap_or(""),
                )
            })
            .collect();
        assert_eq!(modes, [(DARK_NAME, "dark"), (LIGHT_NAME, "light")]);
        Ok(())
    }

    #[test]
    fn colors_come_from_the_palettes() -> Result<(), serde_json::Error> {
        let themes = themes()?;
        let text = |i: usize| {
            themes[i]["colors"]["foreground"]
                .as_str()
                .map(str::to_owned)
        };
        assert_eq!(text(0), Some(css(Palette::aurora().hex(Role::Text))));
        assert_eq!(text(1), Some(css(Palette::dawn().hex(Role::Text))));
        let keyword = &themes[0]["highlight"]["syntax"]["keyword"]["color"];
        assert_eq!(
            keyword.as_str(),
            Some(css(Palette::aurora().hex(Role::SyntaxKeyword)).as_str())
        );
        assert_eq!(
            themes[1]["highlight"]["syntax"]["comment"]["font_style"],
            "italic"
        );
        Ok(())
    }

    #[test]
    fn the_editor_surface_is_transparent_so_panels_show_through() -> Result<(), serde_json::Error> {
        for theme in themes()? {
            assert_eq!(theme["highlight"]["editor.background"], "#00000000");
            assert_eq!(theme["mono_font.family"], "Menlo");
        }
        Ok(())
    }
}
```

- [ ] **Step 2: Verify:** `cargo nextest run -p quarry-ui-kit` (all pass), then `just check` (exit 0).

- [ ] **Step 3: Commit**

```bash
git add crates/ui-kit
git commit -m "feat(ui-kit): generate the gpui-component theme from the Aurora palettes"
```

---

### Task 5: The `Aurora` global: install, colors and theme switching

**Files:**
- Create: `crates/ui-kit/src/aurora.rs`, `crates/ui-kit/tests/aurora.rs`
- Modify: `crates/ui-kit/Cargo.toml`: add `thiserror.workspace = true` under `[dependencies]`, and add this before `[lints]`:

  ```toml
  [dev-dependencies]
  gpui-kit = { workspace = true, features = ["test-support"] }

  ```

- Modify: `lib.rs`: add `pub mod aurora;` and `pub use aurora::Aurora;`, and replace the crate doc with the final one shown in Task 7.

**Interfaces (Phase 1C builds on these):**
- `Aurora::install(Preference, &mut App) -> Result<(), ThemeError>`. Call it once, after `gpui_kit::init`. It registers both themes with `ThemeRegistry`, sets `Theme`'s light and dark configs, and applies the mode.
- `Aurora::preference(&App)`, `Aurora::appearance(&App)` and `Aurora::colors(&App) -> Colors`. During a fade, `colors` returns the interpolated colors.
- `Aurora::is_transitioning(&App)`. While it's true, views call `window.request_animation_frame()`.
- `Aurora::set_preference(Preference, &mut App)` and `Aurora::system_appearance_changed(&mut App)`. Both cross-fade over `motion::THEME`, or switch instantly when `cx.reduce_motion()` is set.
- `aurora::palette_for(Appearance) -> Palette`
- `ThemeError::Load(String)`

**Headless testing facts (verified):**
- Tests use `#[gpui_kit::test] fn name(cx: &mut TestAppContext)`, then `cx.update(|cx| { gpui_kit::init(cx); … })`.
- Under the test scheduler, `reduce_motion` is never read from the OS. Tests set it themselves with `cx.set_reduce_motion(..)`.

- [ ] **Step 1: Write the failing integration test.** Create `crates/ui-kit/tests/aurora.rs`:

```rust
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
```

Run `cargo nextest run -p quarry-ui-kit --test aurora`. Expected: compile failure, `unresolved import quarry_ui_kit::Aurora`.

- [ ] **Step 2: Implement `crates/ui-kit/src/aurora.rs`:**

```rust
//! The app-wide Aurora theme state: preference, appearance and cross-fade.

use std::time::Instant;

use gpui_kit::{App, Global, component::Theme, component::ThemeMode, component::ThemeRegistry};

use crate::{
    appearance::{Appearance, Preference},
    component_theme::{DARK_NAME, LIGHT_NAME, theme_set_json},
    palette::{Colors, Palette},
    tokens::motion,
    transition::Transition,
};

/// Why the theme could not be installed.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum ThemeError {
    /// `gpui-component` rejected the generated theme JSON.
    #[error("the Aurora theme could not be loaded: {0}")]
    Load(String),
}

/// Theme state, stored as a GPUI global.
pub struct Aurora {
    preference: Preference,
    appearance: Appearance,
    transition: Option<Transition>,
}

impl Global for Aurora {}

impl Aurora {
    /// Registers both themes with `gpui-component` and applies `preference`.
    /// Call once, after `gpui_kit::init`.
    ///
    /// # Errors
    /// Returns [`ThemeError::Load`] if the generated theme is rejected.
    pub fn install(preference: Preference, cx: &mut App) -> Result<(), ThemeError> {
        register_component_themes(cx)?;
        let appearance = preference.resolve(cx.window_appearance().into());
        cx.set_global(Self {
            preference,
            appearance,
            transition: None,
        });
        apply_component_mode(appearance, cx);
        Ok(())
    }

    /// The user's preference.
    pub fn preference(cx: &App) -> Preference {
        cx.global::<Self>().preference
    }

    /// The appearance being shown (the target, during a cross-fade).
    pub fn appearance(cx: &App) -> Appearance {
        cx.global::<Self>().appearance
    }

    /// The colors to draw right now.
    pub fn colors(cx: &App) -> Colors {
        let aurora = cx.global::<Self>();
        aurora.transition.map_or_else(
            || palette_for(aurora.appearance).colors(),
            |fade| fade.colors(Instant::now()),
        )
    }

    /// Whether a cross-fade is running; views keep requesting frames while it is.
    pub fn is_transitioning(cx: &App) -> bool {
        cx.global::<Self>()
            .transition
            .is_some_and(|fade| !fade.is_done(Instant::now()))
    }

    /// Changes the preference, cross-fading unless the system asks for reduced motion.
    pub fn set_preference(preference: Preference, cx: &mut App) {
        let target = preference.resolve(cx.window_appearance().into());
        cx.global_mut::<Self>().preference = preference;
        Self::show(target, cx);
    }

    /// Re-reads the system appearance; call when macOS switches light/dark.
    pub fn system_appearance_changed(cx: &mut App) {
        let target = Self::preference(cx).resolve(cx.window_appearance().into());
        Self::show(target, cx);
    }

    fn show(target: Appearance, cx: &mut App) {
        if Self::appearance(cx) == target {
            return;
        }
        let duration = if cx.reduce_motion() {
            std::time::Duration::ZERO
        } else {
            motion::THEME
        };
        let fade = Transition::new(
            Self::colors(cx),
            palette_for(target).colors(),
            Instant::now(),
            duration,
        );
        let aurora = cx.global_mut::<Self>();
        aurora.appearance = target;
        aurora.transition = Some(fade);
        apply_component_mode(target, cx);
    }
}

/// The palette for an appearance.
pub fn palette_for(appearance: Appearance) -> Palette {
    match appearance {
        Appearance::Dark => Palette::aurora(),
        Appearance::Light => Palette::dawn(),
    }
}

fn register_component_themes(cx: &mut App) -> Result<(), ThemeError> {
    ThemeRegistry::global_mut(cx)
        .load_themes_from_str(&theme_set_json())
        .map_err(|error| ThemeError::Load(error.to_string()))?;
    let themes = ThemeRegistry::global(cx).themes();
    let missing = |name: &str| ThemeError::Load(format!("theme {name} was not registered"));
    let dark = themes
        .get(DARK_NAME)
        .cloned()
        .ok_or_else(|| missing(DARK_NAME))?;
    let light = themes
        .get(LIGHT_NAME)
        .cloned()
        .ok_or_else(|| missing(LIGHT_NAME))?;
    let theme = Theme::global_mut(cx);
    theme.dark_theme = dark;
    theme.light_theme = light;
    Ok(())
}

fn apply_component_mode(appearance: Appearance, cx: &mut App) {
    let mode = match appearance {
        Appearance::Dark => ThemeMode::Dark,
        Appearance::Light => ThemeMode::Light,
    };
    Theme::change(mode, None, cx);
}
```

- [ ] **Step 3: Verify:** `cargo nextest run -p quarry-ui-kit` (all pass, including the 5 aurora tests), then `just check` (exit 0).

- [ ] **Step 4: Commit**

```bash
git add crates/ui-kit
git commit -m "feat(ui-kit): add the Aurora global with preference, fade and reduced motion"
```

---

### Task 6: Surfaces, keyboard hints, buttons and badges

**Files:**
- Create: `crates/ui-kit/src/components/mod.rs` (the Task 6 version is below), `surface.rs`, `kbd.rs`, `button.rs` and `badge.rs`, plus `crates/ui-kit/tests/components.rs` (the Task 6 version is below)
- Modify: `lib.rs` (`pub mod components;`)

**Interfaces:**
- `components::ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>`
- `surface::{window_background(&Colors) -> Background, glass_panel(&Colors) -> Div, sidebar_surface(&Colors) -> Div}`. The window gradient is **opaque** (ADR 0006).
- `kbd::kbd(SharedString) -> Kbd`
- `button::{Button::new(id, label).variant(..).shortcut(..).on_click(..), ButtonVariant::{Primary, Secondary, Ghost}, ButtonVariant::style(&Colors) -> ButtonStyle, accent_gradient(&Colors)}`
- `badge::{Badge::new(label, Tone), Tone::{Accent, Success, Warning, Danger, Neutral}, Tone::colors(&Colors) -> (Hsla, Hsla)}`

- [ ] **Step 1: Write the failing headless test.** Create `crates/ui-kit/tests/components.rs`. Task 7 replaces it with the full version.

```rust
//! Components rendered in a headless window: layout, ids and click callbacks.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, px, test::TestWindowExt,
};
use quarry_ui_kit::{
    Aurora, Preference,
    components::{
        badge::{Badge, Tone},
        button::{Button, ButtonVariant},
        surface::{glass_panel, sidebar_surface, window_background},
    },
};

/// Records clicks.
#[derive(Default)]
struct Probe {
    clicks: Cell<usize>,
}

struct Gallery {
    probe: Rc<Probe>,
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let clicks = Rc::clone(&self.probe);
        div()
            .size_full()
            .bg(window_background(&colors))
            .child(sidebar_surface(&colors).child("sidebar"))
            .child(glass_panel(&colors).child(Badge::new("JOIN", Tone::Accent)))
            .child(
                Button::new("run", "Run")
                    .variant(ButtonVariant::Primary)
                    .shortcut("⌘↵")
                    .on_click(move |_, _, _| clicks.clicks.set(clicks.clicks.get() + 1)),
            )
    }
}

fn open(cx: &mut TestAppContext) -> (&mut VisualTestContext, Rc<Probe>) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let probe = Rc::new(Probe::default());
    let view_probe = Rc::clone(&probe);
    let (_, cx) = cx.add_window_view(move |_, _| Gallery { probe: view_probe });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (cx, probe)
}

#[gpui_kit::test]
fn the_button_lays_out_with_a_visible_size(cx: &mut TestAppContext) {
    let (cx, _) = open(cx);
    cx.update(|window, _| {
        let bounds = window.find("run").bounds();
        assert!(
            bounds.size.width > px(0.) && bounds.size.height > px(0.),
            "{bounds:?}"
        );
    });
}

#[gpui_kit::test]
fn clicking_a_button_runs_its_handler(cx: &mut TestAppContext) {
    let (cx, probe) = open(cx);
    cx.update(|window, cx| window.click("run", cx));
    assert_eq!(probe.clicks.get(), 1);
}
```

Run `cargo nextest run -p quarry-ui-kit --test components`. Expected: compile failure, because `quarry_ui_kit::components` doesn't exist yet.

- [ ] **Step 2: Implement the components.** `crates/ui-kit/src/components/mod.rs` for this task:

```rust
//! Reusable Aurora components. Each reads its colors from [`crate::Aurora`].

pub mod badge;
pub mod button;
pub mod kbd;
pub mod surface;

use std::rc::Rc;

use gpui_kit::{App, ClickEvent, Window};

/// A click callback shared by components.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
```

`crates/ui-kit/src/components/surface.rs`:
```rust
//! Window background and glass surfaces.

use gpui_kit::{Background, Div, Styled, div, linear_color_stop, linear_gradient, px};

use crate::{
    palette::{Colors, Role},
    tokens::radius,
};

/// The diagonal window gradient (opaque, so it works without system blur).
pub fn window_background(colors: &Colors) -> Background {
    linear_gradient(
        135.,
        linear_color_stop(colors.get(Role::BackgroundStart), 0.),
        linear_color_stop(colors.get(Role::BackgroundEnd), 1.),
    )
}

/// A floating glass panel: faint fill, 1px light border, medium radius.
pub fn glass_panel(colors: &Colors) -> Div {
    div()
        .rounded(px(radius::MD))
        .bg(colors.get(Role::Panel))
        .border_1()
        .border_color(colors.get(Role::Border))
}

/// The sidebar surface, a slightly stronger glass with a right border.
pub fn sidebar_surface(colors: &Colors) -> Div {
    div()
        .bg(colors.get(Role::Sidebar))
        .border_r_1()
        .border_color(colors.get(Role::Border))
}
```

`crates/ui-kit/src/components/kbd.rs`:
```rust
//! Keyboard-shortcut hint chips, e.g. "⌘↵".

use gpui_kit::{App, Div, IntoElement, ParentElement, SharedString, Styled, div, px};

use crate::{
    Aurora,
    palette::Role,
    tokens::{MONO_FONT, radius, text},
};

/// A small monospace chip showing a shortcut. Colors are applied at render
/// time from the current theme by [`Kbd`].
pub fn kbd(keys: SharedString) -> Kbd {
    Kbd { keys }
}

/// A keyboard-shortcut chip.
#[derive(IntoElement)]
pub struct Kbd {
    keys: SharedString,
}

impl gpui_kit::RenderOnce for Kbd {
    fn render(self, _: &mut gpui_kit::Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        chip(self.keys)
            .text_color(colors.get(Role::TextSecondary))
            .bg(colors.get(Role::Hover))
            .border_1()
            .border_color(colors.get(Role::Border))
    }
}

fn chip(keys: SharedString) -> Div {
    div()
        .px(px(5.))
        .rounded(px(radius::SM - 2.))
        .font_family(MONO_FONT)
        .text_size(px(text::CAPTION))
        .child(keys)
}
```

`crates/ui-kit/src/components/button.rs`:
```rust
//! Buttons: the accent "primary", a glass "secondary", and a bare "ghost".

use std::rc::Rc;

use gpui_kit::{
    App, Background, BoxShadow, ClickEvent, ElementId, Hsla, InteractiveElement, IntoElement,
    ParentElement, RenderOnce, SharedString, StatefulInteractiveElement, Styled, TestSupportExt,
    Window, div, linear_color_stop, linear_gradient, point, px,
};

use super::{ClickHandler, kbd::kbd};
use crate::{
    Aurora,
    palette::{Colors, Role},
    tokens::{radius, space, text},
};

/// Visual weight of a button.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Accent gradient with a soft glow: the one main action (Run).
    Primary,
    /// Glass with a border: secondary actions (Check answer).
    Secondary,
    /// Text only until hovered: toolbar actions.
    Ghost,
}

/// Resolved colors for a variant.
#[derive(Debug, Clone, PartialEq)]
pub struct ButtonStyle {
    /// Fill.
    pub background: Option<Background>,
    /// Label color.
    pub text: Hsla,
    /// Border color, if any.
    pub border: Option<Hsla>,
    /// Glow color, if any.
    pub glow: Option<Hsla>,
}

impl ButtonVariant {
    /// The colors this variant uses with `colors`.
    pub fn style(self, colors: &Colors) -> ButtonStyle {
        match self {
            Self::Primary => ButtonStyle {
                background: Some(accent_gradient(colors)),
                text: colors.get(Role::OnAccent),
                border: None,
                glow: Some(colors.get(Role::AccentStart).opacity(0.45)),
            },
            Self::Secondary => ButtonStyle {
                background: Some(colors.get(Role::Hover).into()),
                text: colors.get(Role::Text),
                border: Some(colors.get(Role::Border)),
                glow: None,
            },
            Self::Ghost => ButtonStyle {
                background: None,
                text: colors.get(Role::TextSecondary),
                border: None,
                glow: None,
            },
        }
    }
}

/// The horizontal violet-to-blue accent gradient.
pub fn accent_gradient(colors: &Colors) -> Background {
    linear_gradient(
        135.,
        linear_color_stop(colors.get(Role::AccentStart), 0.),
        linear_color_stop(colors.get(Role::AccentEnd), 1.),
    )
}

/// A clickable button with an optional keyboard-shortcut hint.
#[derive(IntoElement)]
pub struct Button {
    id: ElementId,
    label: SharedString,
    variant: ButtonVariant,
    shortcut: Option<SharedString>,
    on_click: Option<ClickHandler>,
}

impl Button {
    /// A button with an id (for clicks and tests) and a label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            variant: ButtonVariant::Secondary,
            shortcut: None,
            on_click: None,
        }
    }

    /// Sets the visual weight.
    #[must_use]
    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    /// Shows a shortcut hint such as "⌘↵" after the label.
    #[must_use]
    pub fn shortcut(mut self, shortcut: impl Into<SharedString>) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }

    /// Runs `handler` when clicked.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Button {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let style = self.variant.style(&colors);
        let hover = colors.get(Role::Hover);
        let mut button = div()
            .id(self.id)
            .flex()
            .items_center()
            .gap(px(space::SM))
            .px(px(space::MD))
            .py(px(space::XS + 1.))
            .rounded(px(radius::SM))
            .text_size(px(text::SMALL))
            .text_color(style.text)
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .child(self.label)
            .children(self.shortcut.map(kbd));
        button = decorate(button, &style);
        if let Some(handler) = self.on_click {
            button = button.on_click(move |event, window, cx| handler(event, window, cx));
        }
        button.test_support()
    }
}

fn decorate(
    mut button: gpui_kit::Stateful<gpui_kit::Div>,
    style: &ButtonStyle,
) -> gpui_kit::Stateful<gpui_kit::Div> {
    if let Some(background) = style.background {
        button = button.bg(background);
    }
    if let Some(border) = style.border {
        button = button.border_1().border_color(border);
    }
    if let Some(glow) = style.glow {
        button = button.shadow(vec![glow_shadow(glow)]);
    }
    button
}

fn glow_shadow(color: Hsla) -> BoxShadow {
    BoxShadow {
        color,
        offset: point(px(0.), px(4.)),
        blur_radius: px(14.),
        spread_radius: px(0.),
        inset: false,
    }
}

#[cfg(test)]
mod tests {
    use super::{ButtonVariant, accent_gradient};
    use crate::palette::{Palette, Role};

    #[test]
    fn primary_uses_the_accent_gradient_and_glows() {
        let colors = Palette::aurora().colors();
        let style = ButtonVariant::Primary.style(&colors);
        assert_eq!(style.background, Some(accent_gradient(&colors)));
        assert_eq!(style.text, colors.get(Role::OnAccent));
        assert!(style.glow.is_some() && style.border.is_none());
    }

    #[test]
    fn secondary_is_bordered_glass_and_ghost_is_bare() {
        let colors = Palette::dawn().colors();
        let secondary = ButtonVariant::Secondary.style(&colors);
        assert_eq!(secondary.border, Some(colors.get(Role::Border)));
        assert_eq!(secondary.text, colors.get(Role::Text));
        let ghost = ButtonVariant::Ghost.style(&colors);
        assert!(ghost.background.is_none() && ghost.border.is_none() && ghost.glow.is_none());
        assert_eq!(ghost.text, colors.get(Role::TextSecondary));
    }
}
```

`crates/ui-kit/src/components/badge.rs`:
```rust
//! Small pill labels: concept tags, progress counts, statuses.

use gpui_kit::{
    App, Hsla, IntoElement, ParentElement, RenderOnce, SharedString, Styled, Window, div, px,
};

use crate::{
    Aurora,
    palette::{Colors, Role},
    tokens::{radius, text},
};

/// A badge's color family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tone {
    /// Violet, for concepts and accents.
    Accent,
    /// Teal, for success and progress.
    Success,
    /// Amber, for hints and warnings.
    Warning,
    /// Orange, for errors.
    Danger,
    /// Grey, for neutral metadata.
    Neutral,
}

impl Tone {
    /// `(background, text)` for this tone: the tone color at low opacity behind
    /// the full-strength tone color.
    pub fn colors(self, colors: &Colors) -> (Hsla, Hsla) {
        let ink = match self {
            Self::Accent => colors.get(Role::AccentText),
            Self::Success => colors.get(Role::Success),
            Self::Warning => colors.get(Role::Warning),
            Self::Danger => colors.get(Role::Danger),
            Self::Neutral => colors.get(Role::TextSecondary),
        };
        (ink.opacity(0.15), ink)
    }
}

/// A pill-shaped label.
#[derive(IntoElement)]
pub struct Badge {
    label: SharedString,
    tone: Tone,
}

impl Badge {
    /// A badge with `label` in `tone`.
    pub fn new(label: impl Into<SharedString>, tone: Tone) -> Self {
        Self {
            label: label.into(),
            tone,
        }
    }
}

impl RenderOnce for Badge {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let (background, ink) = self.tone.colors(&Aurora::colors(cx));
        div()
            .px(px(6.))
            .rounded(px(radius::PILL))
            .text_size(px(text::CAPTION))
            .bg(background)
            .text_color(ink)
            .child(self.label)
    }
}

#[cfg(test)]
mod tests {
    use super::Tone;
    use crate::palette::{Palette, Role};

    #[test]
    fn each_tone_draws_its_role_over_a_faint_wash_of_itself() {
        let colors = Palette::aurora().colors();
        let cases = [
            (Tone::Accent, Role::AccentText),
            (Tone::Success, Role::Success),
            (Tone::Warning, Role::Warning),
            (Tone::Danger, Role::Danger),
            (Tone::Neutral, Role::TextSecondary),
        ];
        for (tone, role) in cases {
            let (background, ink) = tone.colors(&colors);
            assert_eq!(ink, colors.get(role), "{tone:?}");
            assert_eq!(background, colors.get(role).opacity(0.15), "{tone:?}");
        }
    }
}
```

- [ ] **Step 3: Verify:** `cargo nextest run -p quarry-ui-kit` (all pass), then `just check` (exit 0; `ui-kit` coverage stays at or above 80/70).

- [ ] **Step 4: Commit**

```bash
git add crates/ui-kit
git commit -m "feat(ui-kit): add glass surfaces, keyboard hints, buttons and badges"
```

---

### Task 7: Tabs, segmented control, sidebar rows and status bar

**Files:**
- Create: `crates/ui-kit/src/components/tabs.rs`, `segmented.rs`, `sidebar_item.rs` and `status_bar.rs`
- Modify: `crates/ui-kit/src/components/mod.rs` (final version below), `crates/ui-kit/tests/components.rs` (replaced by the final version), `crates/ui-kit/src/lib.rs` (final version below)

**Interfaces:**
- `tabs::{Tab::new(id, label).selected(bool).on_click(..), tab_bar() -> Div}`
- `segmented::{SegmentedControl::new(id, options).selected(index).on_change(Fn(usize, &mut Window, &mut App)), ChangeHandler}`. Each segment's element id is `("segment", index)`, scoped under the control's id.
- `sidebar_item::SidebarItem::new(id, label).trailing(text).selected(bool).on_click(..)`
- `status_bar::StatusBar::new().left(item).right(item)`

- [ ] **Step 1: Replace `crates/ui-kit/tests/components.rs` with the full version.** It fails to compile until the new components exist.

```rust
//! Components rendered in a headless window: layout, ids and click callbacks.

use std::{cell::Cell, rc::Rc};

use gpui_kit::{
    Context, IntoElement, ParentElement, Render, Styled, TestAppContext, VisualTestContext, Window,
    div, px, test::TestWindowExt,
};
use quarry_ui_kit::{
    Aurora, Preference,
    components::{
        badge::{Badge, Tone},
        button::{Button, ButtonVariant},
        segmented::SegmentedControl,
        sidebar_item::SidebarItem,
        status_bar::StatusBar,
        surface::{glass_panel, sidebar_surface, window_background},
        tabs::{Tab, tab_bar},
    },
};

/// Records clicks and the last segment chosen.
#[derive(Default)]
struct Probe {
    clicks: Cell<usize>,
    segment: Cell<Option<usize>>,
}

struct Gallery {
    probe: Rc<Probe>,
}

impl Render for Gallery {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let (clicks, segment) = (Rc::clone(&self.probe), Rc::clone(&self.probe));
        div()
            .size_full()
            .flex()
            .flex_col()
            .bg(window_background(&colors))
            .child(
                sidebar_surface(&colors).child(
                    SidebarItem::new("item", "astronauts")
                        .trailing("42")
                        .selected(true),
                ),
            )
            .child(
                tab_bar()
                    .child(Tab::new("tab", "Scratch").selected(true))
                    .child(Tab::new("tab-2", "Query 2")),
            )
            .child(glass_panel(&colors).child(Badge::new("JOIN", Tone::Accent)))
            .child(
                Button::new("run", "Run")
                    .variant(ButtonVariant::Primary)
                    .shortcut("⌘↵")
                    .on_click(move |_, _, _| clicks.clicks.set(clicks.clicks.get() + 1)),
            )
            .child(
                SegmentedControl::new("mode", ["Playground", "Connections"])
                    .on_change(move |index, _, _| segment.segment.set(Some(index))),
            )
            .child(
                StatusBar::new()
                    .left("● Playground")
                    .right("4 rows · 2.1 ms"),
            )
    }
}

fn open(cx: &mut TestAppContext) -> (&mut VisualTestContext, Rc<Probe>) {
    let installed = cx.update(|cx| {
        gpui_kit::init(cx);
        Aurora::install(Preference::Dark, cx)
    });
    assert_eq!(installed, Ok(()));
    let probe = Rc::new(Probe::default());
    let view_probe = Rc::clone(&probe);
    let (_, cx) = cx.add_window_view(move |_, _| Gallery { probe: view_probe });
    cx.update(|window, cx| window.draw(cx).clear(cx));
    (cx, probe)
}

#[gpui_kit::test]
fn every_component_lays_out_with_a_visible_size(cx: &mut TestAppContext) {
    let (cx, _) = open(cx);
    cx.update(|window, _| {
        for id in ["item", "tab", "tab-2", "run", "mode"] {
            let bounds = window.find(id).bounds();
            assert!(
                bounds.size.width > px(0.) && bounds.size.height > px(0.),
                "{id}: {bounds:?}"
            );
        }
    });
}

#[gpui_kit::test]
fn clicking_a_button_runs_its_handler(cx: &mut TestAppContext) {
    let (cx, probe) = open(cx);
    cx.update(|window, cx| window.click("run", cx));
    assert_eq!(probe.clicks.get(), 1);
}

#[gpui_kit::test]
fn a_segment_reports_its_index(cx: &mut TestAppContext) {
    let (cx, probe) = open(cx);
    cx.update(|window, cx| window.within("mode").click(("segment", 1_usize), cx));
    assert_eq!(probe.segment.get(), Some(1));
    cx.update(|window, cx| window.within("mode").click(("segment", 0_usize), cx));
    assert_eq!(probe.segment.get(), Some(0));
}

#[gpui_kit::test]
fn the_selected_tab_sits_left_of_the_next(cx: &mut TestAppContext) {
    let (cx, _) = open(cx);
    cx.update(|window, _| {
        let (first, second) = (window.find("tab").bounds(), window.find("tab-2").bounds());
        assert!(first.origin.x < second.origin.x);
        assert_eq!(first.origin.y, second.origin.y);
    });
}
```

- [ ] **Step 2: Implement**

`crates/ui-kit/src/components/mod.rs` (final):
```rust
//! Reusable Aurora components. Each reads its colors from [`crate::Aurora`].

pub mod badge;
pub mod button;
pub mod kbd;
pub mod segmented;
pub mod sidebar_item;
pub mod status_bar;
pub mod surface;
pub mod tabs;

use std::rc::Rc;

use gpui_kit::{App, ClickEvent, Window};

/// A click callback shared by components.
pub type ClickHandler = Rc<dyn Fn(&ClickEvent, &mut Window, &mut App)>;
```

`crates/ui-kit/src/components/tabs.rs`:
```rust
//! Editor tabs.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, Div, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use super::ClickHandler;
use crate::{
    Aurora,
    palette::Role,
    tokens::{radius, space, text},
};

/// One tab; the selected tab is lifted onto a faint glass background.
#[derive(IntoElement)]
pub struct Tab {
    id: ElementId,
    label: SharedString,
    selected: bool,
    on_click: Option<ClickHandler>,
}

impl Tab {
    /// A tab with an id and label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            selected: false,
            on_click: None,
        }
    }

    /// Marks the tab as the selected one.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Runs `handler` when the tab is clicked.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for Tab {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let (background, ink) = if self.selected {
            (colors.get(Role::Hover), colors.get(Role::Text))
        } else {
            (
                gpui_kit::transparent_black(),
                colors.get(Role::TextSecondary),
            )
        };
        let mut tab = div()
            .id(self.id)
            .px(px(space::MD + 2.))
            .py(px(space::SM - 1.))
            .rounded_t(px(radius::SM + 1.))
            .text_size(px(text::SMALL))
            .bg(background)
            .text_color(ink)
            .cursor_pointer()
            .child(self.label);
        if let Some(handler) = self.on_click {
            tab = tab.on_click(move |event, window, cx| handler(event, window, cx));
        }
        tab.test_support()
    }
}

/// A row of tabs aligned to the bottom of the title-bar strip.
pub fn tab_bar() -> Div {
    div().flex().items_end().gap(px(2.)).px(px(space::MD))
}
```

`crates/ui-kit/src/components/segmented.rs`:
```rust
//! A segmented control: one of a few options (e.g. Playground / Connections).

use std::rc::Rc;

use gpui_kit::{
    App, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce, SharedString,
    StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use crate::{
    Aurora,
    palette::{Colors, Role},
    tokens::{radius, text},
};

/// Called with the index of the newly chosen option.
pub type ChangeHandler = Rc<dyn Fn(usize, &mut Window, &mut App)>;

/// A pill track with one highlighted option.
#[derive(IntoElement)]
pub struct SegmentedControl {
    id: ElementId,
    options: Vec<SharedString>,
    selected: usize,
    on_change: Option<ChangeHandler>,
}

impl SegmentedControl {
    /// A control with `options`; the first is selected.
    pub fn new(
        id: impl Into<ElementId>,
        options: impl IntoIterator<Item = impl Into<SharedString>>,
    ) -> Self {
        let options = options.into_iter().map(Into::into).collect();
        Self {
            id: id.into(),
            options,
            selected: 0,
            on_change: None,
        }
    }

    /// Selects option `index`.
    #[must_use]
    pub fn selected(mut self, index: usize) -> Self {
        self.selected = index;
        self
    }

    /// Runs `handler` with the clicked option's index.
    #[must_use]
    pub fn on_change(mut self, handler: impl Fn(usize, &mut Window, &mut App) + 'static) -> Self {
        self.on_change = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SegmentedControl {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let track = div()
            .id(self.id)
            .flex()
            .gap(px(2.))
            .p(px(3.))
            .rounded(px(radius::SM + 2.))
            .bg(colors.get(Role::Hover))
            .border_1()
            .border_color(colors.get(Role::Border));
        let (selected, handler) = (self.selected, self.on_change);
        let segments = self.options.into_iter().enumerate().map(|(index, label)| {
            segment(index, label, index == selected, &colors, handler.clone())
        });
        track.children(segments).test_support()
    }
}

fn segment(
    index: usize,
    label: SharedString,
    selected: bool,
    colors: &Colors,
    handler: Option<ChangeHandler>,
) -> impl IntoElement + use<> {
    let (background, ink) = if selected {
        (colors.get(Role::Selection), colors.get(Role::Text))
    } else {
        (gpui_kit::transparent_black(), colors.get(Role::TextMuted))
    };
    div()
        .id(("segment", index))
        .flex_1()
        .px(px(10.))
        .py(px(4.))
        .rounded(px(radius::SM))
        .text_size(px(text::CAPTION))
        .bg(background)
        .text_color(ink)
        .cursor_pointer()
        .child(label)
        .on_click(move |_, window, cx| {
            if let Some(handler) = &handler {
                handler(index, window, cx);
            }
        })
        .test_support()
}
```

`crates/ui-kit/src/components/sidebar_item.rs`:
```rust
//! A sidebar row: label, optional trailing text, selectable.

use std::rc::Rc;

use gpui_kit::{
    App, ClickEvent, ElementId, InteractiveElement, IntoElement, ParentElement, RenderOnce,
    SharedString, StatefulInteractiveElement, Styled, TestSupportExt, Window, div, px,
};

use super::ClickHandler;
use crate::{
    Aurora,
    palette::Role,
    tokens::{radius, space, text},
};

/// One row in the sidebar.
#[derive(IntoElement)]
pub struct SidebarItem {
    id: ElementId,
    label: SharedString,
    trailing: Option<SharedString>,
    selected: bool,
    on_click: Option<ClickHandler>,
}

impl SidebarItem {
    /// A row with an id and label.
    pub fn new(id: impl Into<ElementId>, label: impl Into<SharedString>) -> Self {
        Self {
            id: id.into(),
            label: label.into(),
            trailing: None,
            selected: false,
            on_click: None,
        }
    }

    /// Right-aligned secondary text, e.g. a row count.
    #[must_use]
    pub fn trailing(mut self, trailing: impl Into<SharedString>) -> Self {
        self.trailing = Some(trailing.into());
        self
    }

    /// Highlights the row with the selection wash and border.
    #[must_use]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Runs `handler` when the row is clicked.
    #[must_use]
    pub fn on_click(
        mut self,
        handler: impl Fn(&ClickEvent, &mut Window, &mut App) + 'static,
    ) -> Self {
        self.on_click = Some(Rc::new(handler));
        self
    }
}

impl RenderOnce for SidebarItem {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let hover = colors.get(Role::Hover);
        let mut row = div()
            .id(self.id)
            .flex()
            .justify_between()
            .items_center()
            .px(px(space::SM))
            .py(px(5.))
            .rounded(px(radius::SM))
            .text_size(px(text::SMALL))
            .text_color(colors.get(Role::Text))
            .cursor_pointer()
            .hover(move |s| s.bg(hover))
            .child(self.label)
            .children(
                self.trailing
                    .map(|t| div().text_color(colors.get(Role::TextMuted)).child(t)),
            );
        if self.selected {
            row = row
                .bg(colors.get(Role::Selection))
                .border_1()
                .border_color(colors.get(Role::SelectionBorder));
        }
        if let Some(handler) = self.on_click {
            row = row.on_click(move |event, window, cx| handler(event, window, cx));
        }
        row.test_support()
    }
}
```

`crates/ui-kit/src/components/status_bar.rs`:
```rust
//! The bottom status bar: left and right groups of small muted items.

use gpui_kit::{AnyElement, App, IntoElement, ParentElement, RenderOnce, Styled, Window, div, px};

use crate::{
    Aurora,
    palette::Role,
    tokens::{space, text},
};

/// A status bar with a left group and a right group.
#[derive(IntoElement, Default)]
pub struct StatusBar {
    left: Vec<AnyElement>,
    right: Vec<AnyElement>,
}

impl StatusBar {
    /// An empty status bar.
    pub fn new() -> Self {
        Self::default()
    }

    /// Adds an item to the left group.
    #[must_use]
    pub fn left(mut self, item: impl IntoElement) -> Self {
        self.left.push(item.into_any_element());
        self
    }

    /// Adds an item to the right group.
    #[must_use]
    pub fn right(mut self, item: impl IntoElement) -> Self {
        self.right.push(item.into_any_element());
        self
    }
}

impl RenderOnce for StatusBar {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let colors = Aurora::colors(cx);
        let group = || div().flex().items_center().gap(px(space::MD));
        div()
            .flex()
            .justify_between()
            .items_center()
            .px(px(space::LG))
            .py(px(space::SM))
            .text_size(px(text::CAPTION))
            .text_color(colors.get(Role::TextMuted))
            .child(group().children(self.left))
            .child(group().children(self.right))
    }
}
```

`crates/ui-kit/src/lib.rs` (final):
```rust
//! The Aurora design system: theme tokens and reusable components.
//!
//! Call [`Aurora::install`] once after `gpui_kit::init`; components then read
//! the current colors with [`Aurora::colors`]. Views that render while
//! [`Aurora::is_transitioning`] should call `window.request_animation_frame()`
//! so the theme cross-fade animates.

pub mod appearance;
pub mod aurora;
pub mod color;
pub mod component_theme;
pub mod components;
pub mod palette;
pub mod tokens;
pub mod transition;

pub use appearance::{Appearance, Preference};
pub use aurora::Aurora;
pub use palette::{Colors, Palette, Role};
```

- [ ] **Step 3: Verify:** `cargo nextest run -p quarry-ui-kit` (all 33 tests pass), then `just check` (exit 0). For reference, `ui-kit` measured about 96% lines and 95% regions when this plan was verified.

- [ ] **Step 4: Commit**

```bash
git add crates/ui-kit
git commit -m "feat(ui-kit): add tabs, segmented control, sidebar rows and status bar"
```

---

### Task 8: Gallery, review, ADR 0008 and the PR

**Files:**
- Create: `crates/ui-kit/examples/gallery.rs`, `docs/adr/0008-aurora-design-system.md`
- Modify: `crates/ui-kit/Cargo.toml` (dev-dependency features become `["test-support", "assets"]`), `docs/superpowers/specs/2026-11-04-quarry-design.md` §4.1

- [ ] **Step 1: Write the gallery.** Create `crates/ui-kit/examples/gallery.rs`:

```rust
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
```

In `crates/ui-kit/Cargo.toml`, set the dev-dependency to:
`gpui-kit = { workspace = true, features = ["test-support", "assets"] }`

Run `cargo clippy -p quarry-ui-kit --all-targets -- -D warnings`. Expected: clean. The example is also subject to the complexity gate, and every function in it is small.

- [ ] **Step 2: Visual review (the controller does this with the user; implementers skip it)**
1. Run `cargo run --release -p quarry-ui-kit --example gallery`.
2. Confirm it still matches the approved review from 2026-11-08: the dark and light themes, the accent glow on Run, the badges in five tones, and the selected sidebar row.
3. Click Light, Dark and System. With Reduce Motion on, switching is instant; otherwise it's a 250 ms cross-fade.

- [ ] **Step 3: Write ADR 0008, "Aurora design system implementation".** Status Accepted, Date 2026-11-08. It records the following:
- **Palette:** one palette table per theme, keyed by `Role`, as `0xRRGGBBAA`. It feeds rendering, the generated `gpui-component` theme and a WCAG contrast test. The palette values were tuned from the mockups to pass that test (4.5:1 for body text, 3:1 for quiet text).
- **Fonts:** `.SystemUIFont` (SF Pro) for the interface and Menlo for code. SF Mono is a private system font that GPUI can't load by name, and bundling a font would add a license.
- **Window background:** opaque, following ADR 0006. Theme changes cross-fade over 250 ms, or switch instantly under Reduce Motion.
- **Tests:** components are tested headlessly with `gpui-kit`'s `test-support`. Interactive roots call `.test_support()`.
- **Supply-chain policy for GPUI:** the `cargo deny` graph is limited to `aarch64-apple-darwin`; `skip-tree` covers duplicates inside `gpui-kit`; four unmaintained, non-vulnerable advisories are ignored with reasons; and 0BSD, CC0-1.0 and MPL-2.0 are allowed. All of it is to be revisited on each `gpui-kit` upgrade.
- **Deferred:** the progress ring (Phase 2, tracks) and a translucent window with real blur (to be checked on a Mac without Lockdown Mode).

- [ ] **Step 4: Update spec §4.1.**
- In the "Typography" bullet, replace "SF Mono for code and numbers" with "Menlo for code and numbers (SF Mono isn't loadable by name; see ADR 0008)".
- Add a bullet: "Colors are defined once per theme by semantic role in `ui-kit` and checked for WCAG contrast in CI."

- [ ] **Step 5: Final gate and commit**

Run: `just check` (exit 0) and `just mutants` (exit 0). `ui-kit` isn't in the mutation set, so this re-checks the core crates.
```bash
git add crates/ui-kit docs
git commit -m "docs: add the Aurora gallery and record the design-system decisions (ADR 0008)"
```

- [ ] **Step 6: Open the PR and watch CI**

```bash
git push -u origin phase-1b-design-system
gh pr create --base main --title "Phase 1B: Aurora design system" --body "Implements docs/superpowers/plans/2026-11-08-phase-1b-design-system.md. ADR 0008 records the decisions."
gh pr checks --watch
```
Expected: `check` and `mutants` pass. CI is slower now that GPUI builds. Don't merge; the controller merges after the final review, with the user's go-ahead.
