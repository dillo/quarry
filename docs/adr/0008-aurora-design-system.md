# ADR 0008: Aurora design system implementation

- **Status:** Accepted
- **Date:** 2026-11-08

## Context
Phase 1B turns the Aurora mockups into the `quarry-ui-kit` crate: tokens, a theme, and the first components, built on `gpui-kit` and `gpui-component`. Several choices shape every later screen, so they are recorded here.

## Decision
- **Palette:** one palette table per theme, keyed by `Role`, as `0xRRGGBBAA`. It feeds rendering, the generated `gpui-component` theme and a WCAG contrast test. The palette values were tuned from the mockups to pass that test (4.5:1 for body text, 3:1 for quiet text). The gate covers 27 surfaces: the bare gradient stops, Panel and Sidebar glass over each stop, and the Hover, Selection and ActiveLine washes over each glass surface. To pass, several Dawn text colors were darkened by 1.5-7.5% (hue kept) and the line-number alphas were raised. The selection tint was kept.
- **Components:** buttons have a per-variant hover (Primary deepens its gradient 10% toward black, Secondary uses the Selection wash, Ghost the Hover wash). The shortcut chip on Primary is white on an 18% black wash, pinned by a contrast test. The Primary label is white on the accent gradient at 14 pt semibold, which is WCAG large text, so it is held to 3:1 rather than 4.5:1 (white measures 3.16-3.69 on the resting stops and at least 3.0 on the deepened hover stops). Hover deepens rather than brightens because brightening dropped white below 3:1 on the blue stop. Both are checked by the contrast gate. Tabs, segments and sidebar rows resolve selected and unselected colors through pure functions, and selected items have no hover. Keyboard focus and navigation are deferred to Phase 1C.
- **Fonts:** `.SystemUIFont` (SF Pro) for the interface and Menlo for code. SF Mono is a private system font that GPUI can't load by name, and bundling a font would add a license.
- **Window background:** opaque, following ADR 0006. Theme changes cross-fade over 250 ms, or switch instantly under Reduce Motion.
- **Tests:** components are tested headlessly with `gpui-kit`'s `test-support`. Interactive roots call `.test_support()`.
- **Supply-chain policy for GPUI:** the `cargo deny` graph is limited to `aarch64-apple-darwin`; `skip-tree` covers duplicates inside `gpui-kit`; four unmaintained, non-vulnerable advisories are ignored with reasons; and 0BSD, CC0-1.0 and MPL-2.0 are allowed. All of it is to be revisited on each `gpui-kit` upgrade.
- **Deferred:** the progress ring (Phase 2, tracks) and a translucent window with real blur (to be checked on a Mac without Lockdown Mode).

## Consequences
- Every color goes through a `Role`, so a theme change is a palette change, and the contrast test catches regressions.
- Code uses Menlo until a licensed or loadable mono font is chosen.
- The gallery (`cargo run -p quarry-ui-kit --example gallery`) is the visual reference for review.
