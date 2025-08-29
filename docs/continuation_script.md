Project: Vim-like Task Tracker (Rust, 2025)

Structure:
- crates/
  - tm-core: vault, task file format (Markdown+YAML), core API (create/list/edit tasks).
  - tm-cli: CLI binary (add/list/status/start).
  - tm-ui: TUI (ratatui + crossterm).
  - tm-gui: GUI (egui/eframe).
  - tm-plugin-host: (placeholder for Lua plugins).
- tm/: meta-binary wiring CLI/TUI/GUI.

Core:
- Tasks are Markdown files with YAML frontmatter (id, title, status, project, tags, due, etc.).
- Core API: create_task, list_tasks, set_status/cycle_status, set_due, set_tags_csv, rename_title.
- Status enum: Todo | Doing | Done.

✅ Done:
- CLI: add/list/status/start mapped to core.
- TUI: j/k, gg/G, Ctrl-d/u, filter (/), quit (q), status (x/X/1/2/3), edit panels for title (R), due (D), tags (T).
- GUI: task list + detail panel, motions, filter (/), status buttons (1/2/3), inline editing (title, due, tags).
- Basic edits (title, due, tags, status) all functional.

⏭️ Next Up:
1. Ex-bar (: commands) in TUI/GUI
   - :quit, :new, :status <id> <state>, :due <id> YYYY-MM-DD, :tags <id> +a,+b, :rename <id> "title".
2. ProjectshT
   - Vault::create_project, Vault::list_projects.
   - CLI: `tm project add <name>`.
   - TUI/GUI: project picker, `:open project:<name>`.
3. Indexing (SQLite FTS5)
   - Markdown remains source of truth, add SQLite index for fast filter/search.
4. Config + Plugins
   - `~/.tm/config.lua`, hot-reload keymaps/themes.
   - Lua plugin host (mlua), first sync integrations (Todoist, Jira).

Current state: Editing flows ✅, next major milestone = ex-bar + projects.

