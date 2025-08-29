# Task Manager (tm, tmtui, tmgui)

## Installation
TO BE DETERMINED!

Objective is a single line install via brew, choco, etc

## Usage Guide
### CLI
TBD - but should end up like some of the following commands for v1
- `tm add Task Name`
- `tm new Task Name`
- `tm new project ProjectName`
- `tm new -p ProjectName`
hoping to keep the syntax fast - the faster someone can create a task - the better!

### TUI
TBD

### GUI
Mostly same as TUI, but also has buttons :smile:

## Features

- ⚡ **Blazing fast startup** — sub-250ms even with thousands of tasks. Markdown for storage + SQLite/FTS for speed.
- 🎹 **Vim-like motions for tasks** — `j/k`, `gg/G`, `/`, `:ex` commands, and status cycles (`x`, `X`, `1/2/3`). It feels like editing code, but for tasks.
- 🖥️ **Triple frontends** — one codebase, three interfaces:
  - **CLI** for scripts and automation
  - **TUI** for terminal-first workflows
  - **GUI** for point-and-click users  
  All consistent, all powerful.
- 🧩 **Extensible plugin system** — Lua-based (Neovim-style), load from git repos with `use {}`. Planned plugins include Todoist, Jira, GitLab, GitHub syncs.
- ⚙️ **Config via Lua** — `~/.config/tm/config.lua` controls keymaps, colors, and defaults. Hot-reload with `:config.reload`.
- 📜 **Ex command language** — inspired by Vim/Taskwarrior. Examples:
  - `:new "Pick up dry cleaning" +home due:2025-09-01`
  - `:done 01ABC…`
  - `:view.save today`
- 📂 **Projects + subtasks** — native support for hierarchical projects, tasks, and subtasks. Roll-up statuses, due-date inheritance, reorder & reparent.
- 🌍 **Cross-platform** — macOS, Linux, Windows (incl. WSL). Single binary distribution with Brew/Chocolatey/Scoop planned.
- 🔒 **Offline-first, local-first** — everything works without network. Markdown vault is always the source of truth; sync via plugins is optional.
- 🔄 **Live reload** — external edits (e.g. Obsidian/Vim) detected automatically via file-watcher.
- 🔍 **Saved views & filters** — persist reusable queries like “today”, “work”, or “high-priority” and recall them instantly.
- ⏪ **Future: Undo & history** — append-only event log enables session undo (`u`, `Ctrl-r`) and full audit trail.
- 🤝 **Community-friendly** — MIT-licensed, plugin registry planned, with a roadmap toward secure plugin permissions.

> **Goal:** the “Neovim of task management” — fast, extensible, local-first, and hackable. A solid base you can bend to your workflow instead of bending to someone else’s.


## Why build another task manager?
In short, nothing met my requirements. Used every task management tool possible and I could never get cli + tui + gui + a syntax to interact with tasks like vim motions. With a majority being web based or GUI app based this seemed unintuitive with nvim/terminal/vscode focused flows.

I want a optimized language to manage tasks because as a manager, architect, mentor, devops/infra engineer, and tech lead - theres too damn much!

I've always loved vim and terminal based workflows and apps. In this case nothing else met my needs so I want to officially create what I would call the "vim/nvim of task management". Super fast, super easy to use, and a extension/modification layer to allow myself and others to easily add plugins.

## Running for develpment
Run `cargo run -p tm -- ls` to list tasks


Run `cargo run -p tm -- tui` to launch the TUI (press `q` to quit).


Run `cargo run -p tm -- gui` to launch the TUI (press `q` to quit).



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

## Contributing
- MIT license, feel free to do what you want!
- Would love for ppl to contribute and make this thing amazing

### AI Use
- Vibecoding allowed, but scrutanize it a ton
- Goal is simple, efficient, and easy to work with both in the tool and in the code
- If it doesnt mean that goal, it shouldn't get merged!
- I will be transparent, I used AI to build a large chunk of the inital version here but I wish to decompose it and make it understandable to me and others! (A nightmare probably but its a good way to decompose and learn a new lang IMO)



