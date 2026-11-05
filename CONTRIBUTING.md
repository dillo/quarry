# Contributing to Quarry

The design spec is the source of truth: `docs/superpowers/specs/2026-11-04-quarry-design.md`. Section 7 has the full standards; this is the working summary.

## Principles

1. Dependencies flow one way. `cargo xtask check-deps` enforces it, and a new crate needs a rule in `xtask/src/checks/deps.rs`.
2. Logic lives in library crates; GPUI views stay thin.
3. **Single responsibility** for every function and module. If its honest name needs "and", split it. No `utils`, `helpers`, `misc` or `common` modules.
4. Deep modules: a narrow public API that hides a lot.
5. Errors are values. `thiserror` in libraries; no `unwrap`, `expect` or `panic!` outside tests.
6. No `unsafe`.
7. Tests first for the core crates. Bug fixes start with a failing test.
8. **Extension points first:** new features go through a trait or registry (spec §3.3).
9. Record significant decisions as ADRs in `docs/adr/`.

## Limits (per function, tests included)

Cyclomatic ≤ 10 · cognitive ≤ 15 · statements ≤ 15 · lines ≤ 40 · nesting ≤ 4 · arguments ≤ 5. Numbers live in `quality/thresholds.toml`.

## Coverage

Core crates (`engine`, `challenges`, `store`): 95% lines / 90% regions. UI crates (`ui-kit`, `app`): 80 / 70. `xtask`: 90 / 85. Mutation score ≥ 90% on the core crates.

## Exceptions

A lint suppression attribute needs a written reason **and** an entry in `quality/exceptions.toml`. A complexity waiver uses `lint = "quarry::complexity"` plus `function = "<name>"`. CI rejects unregistered, reasonless or stale entries.

## Workflow

- `just check` before you push (the pre-push hook runs it).
- Conventional Commits (`feat:`, `fix:`, `docs:`, `build:`, `ci:`, `refactor:`, `test:`).
- Fill in the PR template, including how many existing files you touched.

## Adding a quality gate

Gates are an extension point too: add `xtask/src/checks/<gate>.rs` (a pure function plus unit tests), one `Command` variant, one `commands.rs` arm, and an integration test in `xtask/tests/commands.rs`.
