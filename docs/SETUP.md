# Setup Instructions

## Requirements
- Rust (latest stable)
- Cargo
- Lua (for plugin runtime)
- SQLite (bundled via rusqlite)

## Build
```bash
cargo build
```

## Run
```bash
cargo run -- ls
```

## Tests
```bash
cargo test
```

## Optional
- Install `cargo-watch` for hot reload: `cargo install cargo-watch`
