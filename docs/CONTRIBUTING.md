# Contributing Guide

Thanks for your interest in contributing!

## How to Contribute
1. Fork the repo and create a new branch.
2. Make your changes.
3. Run tests and linters.
4. Submit a pull request with a clear description.

## Coding Standards
- Language: Rust 2021 edition
- Format: `cargo fmt`
- Lint: `cargo clippy --all-targets -- -D warnings`
- Tests: `cargo test`
- Naming: consistent domain language (task, project, subtask, query, index)

## Commit Style
- Use [Conventional Commits](https://www.conventionalcommits.org/)
  - `feat: add subtask support`
  - `fix: correct due date parsing`
  - `docs: update README`

## Development Setup
```bash
rustup default stable
cargo build
cargo test
```

See [SETUP.md](SETUP.md) for details.
