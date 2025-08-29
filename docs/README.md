# Vim-Style Task Tracker

A blazing-fast, modal, Vim-like task tracker with both CLI and TUI frontends, a rock-solid core library, and a first-class extension system. Think: the Neovim of tasks.

## Features
- Local-first, offline-capable
- Markdown + YAML frontmatter as the source of truth, SQLite index for speed
- Modal TUI (list + detail views, Vim motions, ex-bar, command palettes)
- Hierarchy: projects, tasks, subtasks
- Extensible with Lua plugins
- Cross-platform (macOS, Linux, Windows)

## Quick Start
```bash
git clone https://github.com/yourname/vim-task-tracker.git
cd vim-task-tracker
cargo build --release
./target/release/tm ls
```

## Roadmap
See [ROADMAP.md](ROADMAP.md)

## Docs
- [Architecture](docs/ARCHITECTURE.md)
- [Guardrails](docs/GUARDRAILS.md)
- [Key Notes](docs/KEY_NOTES.md)
- [Starter Prompt](docs/STARTER_PROMPT.md)

## License
MIT, see [LICENSE](LICENSE).
