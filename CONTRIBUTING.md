# Contributing to Neuronic

Thanks for your interest in contributing to Neuronic!

## Getting Started

1. Fork and clone the repository
2. Install Rust 1.85+ via [rustup](https://rustup.rs/)
3. Run `cargo build` to verify everything compiles
4. Run `cargo test` to run the test suite

## Development

### Running locally

```bash
# With a config file
cargo run -- --config neuronic.toml

# With debug logging
cargo run -- --debug
```

You'll need a RabbitMQ instance with Caryatid monitor snapshots being published. See the [README](README.md) for configuration details.

### Code style

We use standard Rust formatting and linting:

```bash
cargo fmt --check
cargo clippy --all-targets --all-features -- -D warnings
```

### Running tests

```bash
cargo test
```

## Pull Requests

1. Create a feature branch from `main`
2. Make your changes
3. Run `cargo fmt` and `cargo clippy`
4. Run `cargo test`
5. Update CHANGELOG.md if applicable
6. Submit a PR with a clear description of the changes

### Commit messages

Use clear, descriptive commit messages. We loosely follow [Conventional Commits](https://www.conventionalcommits.org/):

- `feat:` new features
- `fix:` bug fixes
- `docs:` documentation changes
- `refactor:` code refactoring
- `test:` adding or updating tests
- `chore:` maintenance tasks

## Architecture

The codebase is organized as follows:

```
src/
├── main.rs           # CLI entry point
├── config.rs         # Configuration loading
├── subscriber.rs     # RabbitMQ connection
├── graph.rs          # Graph data model
└── ui/
    ├── app.rs        # Main eframe application
    ├── theme.rs      # Color themes
    ├── drawing.rs    # Graph rendering
    ├── layout.rs     # Layout algorithms
    ├── animations.rs # Visual effects
    ├── input.rs      # User interaction
    ├── panels.rs     # Side panels
    ├── search.rs     # Search functionality
    └── export.rs     # Export features
```

## Issues

Check the [issue tracker](https://github.com/lowhung/neuronic/issues) for open issues. Issues labeled `good first issue` are a good starting point.

## License

By contributing, you agree that your contributions will be licensed under the Apache-2.0 license.
