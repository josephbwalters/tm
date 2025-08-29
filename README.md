# Vim-Style Task Tracker Skeleton

Run `cargo run -p tm -- tui` to launch the TUI (press `q` to quit).

## Development Installation
### MacOS
brew install rust
brew install sqlite
brew install lua
cargo install cargo-watch
> warning: be sure to add `/Users/jwalters/.cargo/bin` to your PATH to be able to run the installed binaries

brew install just

## Building & Running
cargo build
cargo run -p tm -- init
cargo run -p tm -- add "Try me" --project inbox
cargo run -p tm -- ls
cargo run -p tm -- tui
