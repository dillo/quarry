# ADR 0004: Extension points, single responsibility and enforced quality gates

- **Status:** Accepted
- **Date:** 2026-11-05

## Context
The codebase must scale so that features are additive. The user requires strict, automated quality enforcement.

## Decision
- Features enter through traits and registries (spec §3.3).
- Gates: clippy pedantic plus the restriction lints; per-function limits of cyclomatic 10, cognitive 15, 15 statements and 40 lines; and per-crate coverage of 95/90 for the core crates and 80/70 for the UI crates.
- **`xtask` coverage is 90/85.** This is a tooling target chosen in the Phase 0 plan.
- Mutation score ≥ 90%; cargo-deny; a registered-exceptions file.
- **`cargo-geiger` is deferred.** Spec §7.4 lists it as a report on `unsafe` code in dependencies. Version 0.14.0 crashed while this plan was being verified: it reports a missing `tests/support.rs` on a standard `tests/support/mod.rs` layout. Our own crates still forbid `unsafe`, and `cargo-deny` covers known vulnerabilities. Revisit when a working release exists.
- The workspace-specific gates live in `xtask`, and adding a gate is itself an extension point.

## Consequences
- Gates exist from the first commit, so there's never a backlog to clear.
- Every exception is visible in review.
- Some code will be split into more functions than a looser codebase would have.
