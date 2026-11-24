# Quarry

A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.

> **Status:** Phase 2B. On top of Phase 1's query tabs, scripts, plans, copying and ⌘K palette, you can open Vinyl Vault's first five challenges from ⌘K, each in its own tab on a fresh copy of the track's database, read its brief in the inspector (⌘⌥I), check your answer with ⌘⇧↵ and see what's different rule by rule, and move between challenges with ⌘⌥[ and ⌘⌥]. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).

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
