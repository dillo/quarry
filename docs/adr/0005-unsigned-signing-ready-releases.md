# ADR 0005: Unsigned releases, with signing ready to switch on

- **Status:** Accepted
- **Date:** 2026-11-05

## Context
- Distribution is a personal tap, `dillo/homebrew-tap`.
- Homebrew's Gatekeeper requirements apply to the official `homebrew/cask` repository, not to personal taps.
- There's no Apple Developer ID yet.

## Decision
- Ship an ad-hoc signed app.
- The cask's postflight step removes the quarantine flag, as `brewery.rb` already does.
- The release workflow's codesign and `notarytool` steps run only when `APPLE_DEVELOPER_ID_*` secrets exist.

## Consequences
- No cost now and no rework later.
- Downloading the DMG directly, outside Homebrew, shows a Gatekeeper warning until signing is switched on.
