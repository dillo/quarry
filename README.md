# Quarry

A native macOS SQL client for learning SQL by hand and for real work, built in Rust with GPUI.

> **Status:** Phase 0 (foundations). Not usable yet. See the [design spec](docs/superpowers/specs/2026-11-04-quarry-design.md).

- Built-in SQLite playground with story-driven SQL challenge tracks
- PostgreSQL and MySQL connections for everyday work
- No autocomplete, ever: you write every query yourself

## Development

    brew install just
    just setup     # installs pinned tools and git hooks
    just check     # every gate, identical to CI

See [CONTRIBUTING.md](CONTRIBUTING.md) for the engineering standards.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or [MIT license](LICENSE-MIT), at your option.
