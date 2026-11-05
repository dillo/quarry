# ADR 0001: GPUI (via gpui-kit) over Tauri

- **Status:** Accepted
- **Date:** 2026-11-05

## Context
The UI must feel native and fast enough to beat Electron-based Beekeeper Studio, and it should be written in Rust.

## Decision
- Use GPUI, drawn on the GPU through Metal, with `gpui-component`.
- Depend on the **`gpui-kit`** crate, pinned with `=0.9.0`. It pins the matching GPUI snapshot (`gpui-pre 0.3.11`) and provides `gpui_kit::application()`.
- Don't depend on the `gpui` or `gpui-pre` crates directly, because their versions must match exactly.
- Upgrades are deliberate PRs.

## Consequences
- The whole app is Rust, renders at 120 fps, and gets real frosted-glass windows.
- The documentation is thin and the API changes between versions, which is why there's a Phase 0 spike.
- GPUI-specific code is confined to `app` and `ui-kit`.
