# Quarry

A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.

> **Status:** Phase 1D. You can write and run SQL against a built-in sample database in query tabs that survive restarts, run whole scripts, and explain a statement's plan. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).

- Built-in SQLite playground with story-driven SQL challenge tracks
- PostgreSQL and MySQL connections for everyday work
- No autocomplete, ever: you write every query yourself

## Development

    brew install just
    just setup     # installs pinned tools and git hooks
    just check     # every gate, identical to CI
    cargo run -p quarry   # open Quarry on the sample database

See [CONTRIBUTING.md](CONTRIBUTING.md) for the engineering standards.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
