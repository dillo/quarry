# ADR 0006: Phase 0 spike outcome

- **Status:** Accepted
- **Date:** 2026-11-05

## Context
The Phase 0 GPUI spike tested three assumptions: frosted-glass windows, a completion-free SQL editor, and a 1M-row table. Evidence is in docs/spikes/phase-0-gpui.md. The test Mac had Lockdown Mode on and a 60 Hz display, so real blur and 120 fps remain unverified.

## Decision
- **(a)** Use `WindowBackgroundAppearance::Blurred`, with an Aurora base that is fully usable when opaque. GPUI matched native fallback behaviour where blur was disabled.
- **(b)** Use the `gpui-component` editor. No custom editor is needed. The source audit shows every completion path is guarded by an optional provider that defaults to None.
- **(c)** Use `DataTable`. It held 61 fps on a 60 Hz display with about 94 MB RSS and reached row 999999 without stalling.

## Consequences
- Aurora must be designed as an opaque base, with blur as progressive enhancement, and must honour Reduce Motion.
- The editor needs Aurora theming, and a regression test must assert it never has a completion provider.
- The titlebar inset becomes a ui-kit token.
- Blur and 120 Hz must be verified on a Mac without Lockdown Mode and with ProMotion before Phase 1 visual sign-off.
