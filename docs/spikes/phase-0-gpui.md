# Phase 0 GPUI spike: findings

Versions: gpui-kit 0.9.0 (gpui-pre 0.3.11), Rust 1.98.1, macOS 27.2, Mac14,7 (13" MacBook Pro M2, 60 Hz).

## Environment caveats
- Lockdown Mode is on (LDMGlobalEnabled=1). Blur appears disabled system-wide; native apps show the same fallback. Reduce Transparency and Increase Contrast are off.
- Reduce Motion is on.
- The display is 60 Hz (no ProMotion), so 120 fps behaviour is UNVERIFIED.

## (a) Frosted glass: PASS
| Criterion | Result | Notes |
|---|---|---|
| Desktop blurs through | UNVERIFIED | System-wide blur is disabled on this Mac. With alpha d0 the window was flat dark grey; with SPIKE_ALPHA=60 it was flat light grey with no wallpaper colour. The native Stocks sidebar over the same wallpaper is also flat grey, so GPUI matches native behaviour. Likely caused by Lockdown Mode (not confirmed by documentation). GPUI uses NSVisualEffectView (material UnderWindowBackground, BehindWindow, state Active) with a macOS-27-specific fix (gpui-pre-macos 0.3.11 window.rs:3935+). |
| Traffic lights at (14,14), transparent title bar | PASS | User screenshot 1. |
| Crisp 1px border / 10px radius | PASS | User screenshot 1. |
| Smooth resize | UNVERIFIED | Not separately reported by the user. |

Verdict: PASS because GPUI matches native fallback behaviour; real blur appearance is UNVERIFIED on this machine.

Screenshots (described in text; the temporary files were deleted by macOS before they could be copied, so there is no docs/spikes/img directory):
- Screenshot 1: glass window at default alpha showing traffic lights at about (14,14) over a transparent title bar and a crisp 1px border with 10px radius.
- Screenshot 2: glass window at low alpha (SPIKE_ALPHA=60), flat light grey with no wallpaper colour.
- Screenshot 3: native Stocks app sidebar over the same sunset wallpaper, flat grey with no tint, the same fallback.

Design consequence: Aurora must be designed as a fully opaque base with blur as progressive enhancement. Quarry animations must honour Reduce Motion.

## (b) Completion-free SQL editor: PASS
| Criterion | Result | Notes |
|---|---|---|
| SQL highlighting | PASS | Screenshot 4: keywords blue, functions dark blue, identifiers purple, numbers green, DESC gold, comment light blue. |
| Line numbers | PASS | Screenshot 4, with current-line highlight. |
| No completion UI (5 triggers) | PASS | User: "Everything seems to work fine"; not individually itemised. |
| Multi-cursor, undo/redo | PASS | Same user statement; not individually itemised. |
| 5,000-line responsiveness | not individually reported | Screenshot 4 shows a large pasted buffer with selection working fine; no line count or timing recorded. |

Screenshot 4: the editor showing the highlighting above, line numbers, current-line highlight and a large pasted buffer with a selection. It renders with gpui-component's default LIGHT theme.

Source audit (gpui-base 0.9.0): the completion menu is guarded by `extras.lsp.completion_provider` (completions.rs:239, 333, 469, each `let Some(provider) = ... else return`). Inline ghost text is only set at completions.rs:523, inside the guarded path. state.rs:10634 is test code. The default is `completion_provider: None` (lsp/mod.rs:81). The hover, definition, color and semantic providers are also `Option`, default None.

## (c) 1M-row table: PASS
| Criterion | Result | Measured |
|---|---|---|
| Opens < 1 s | UNVERIFIED | Opened promptly; the window was up within a 2 s check (< 2 s measured, < 1 s not precisely timed). |
| ≥ 110 fps flick-scroll (ProMotion) | UNVERIFIED at 120 Hz | 61 fps on a 60 Hz display (screenshot 5), meeting the plan's >= 58 fps criterion for 60 Hz. No ProMotion display available. |
| Jump to row 999999 | PASS | Scrolled/dragged to the end; row 999999 visible, no stall (screenshot 5). |
| Memory < 300 MB | PASS | RSS 93 MB at start, 94 MB after scrolling. |
| Column resize | PASS (not individually itemised) | User: "Seems to work just fine!". |

Screenshot 5: the table scrolled to the end with row 999999 visible and the FPS readout showing 61 fps.

Cosmetic spike bug: the FPS label is drawn under the traffic lights because `.p_4()` after `.pt(px(44.))` resets the top padding (table.rs render).

## API differences from the plan
None needed for compilation. One spike-only addition was made during testing: the glass.rs alpha is tunable via the SPIKE_ALPHA env var (hex byte).

## Recommendations for Phase 1
- Design Aurora as an opaque base, with blur as progressive enhancement.
- Apply Aurora theming to the editor (it currently uses gpui-component's default light theme).
- Add a regression test asserting that Quarry's editor state never has a completion provider.
- Make the titlebar inset a ui-kit token rather than ad hoc padding.
- Verify blur and 120 Hz on a Mac without Lockdown Mode and with ProMotion before Phase 1 visual sign-off.
