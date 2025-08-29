# Architecture Overview

## Layers
- **tm-core**: domain model, storage (Markdown+YAML), index (SQLite+FTS), query parser/engine.
- **tm-ui**: TUI using ratatui+crossterm, palettes, keymaps, ex-bar, list+detail views.
- **tm-cli**: git‑like subcommands, mirrors tm-core actions, scripts friendly.
- **tm-plugin-host**: Lua runtime, exposes host API (tasks, query, ui, http, secrets).

## Data Model
- **Project**: id, key, title, description, status, tags, created, updated.
- **Task**: id, key, project_id, parent_id, title, body, status, priority, tags, due, start, done, estimate, actual, assignee, created, updated, sort_order.
- **Subtask**: Task with parent_id set.

## File Layout
```
~/TasksVault/
├── projects/
│   └── YYYY/...
├── tasks/
│   └── YYYY/MM/...
└── .tm/
    ├── index.sqlite
    └── config.lua
```

## Key Flows
- **Write**: edit Markdown → update index in same txn.
- **Read**: query SQLite index → hydrate Markdown fields for display.
- **Plugins**: register commands, use host API, sync tasks, contribute palette entries.
