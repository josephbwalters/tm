# Vision
A blazing‑fast, modal, Vim‑like task tracker with both CLI and TUI frontends, a rock‑solid core library, and a first‑class extension system (Lua‑powered, with optional WASM later). Think: the “Neovim of tasks.”

## Primary Goals
- **Speed**: instant startup (<50ms), snappy rendering even with 100k tasks.
- **Ergonomics**: modal editing, motions, operators, text‑object‑like task actions.
- **Extensibility**: plug‑and‑play extensions with a stable API; easy install/update.
- **Portability**: macOS/Linux/Windows; single static binary where possible.
- **Reliability**: robust storage, crash‑safe, auditable change history.

## Non‑Goals (v1)
- Multi‑user real‑time collaboration (defer).
- Heavy GUI; we’re CLI/TUI‑first.

---

# Tech Stack Proposal
- **Core language**: **Rust** (performance, safety, great ecosystem).
- **TUI**: `ratatui` + `crossterm` (immediate‑mode, wide support).
- **CLI**: `clap` (or `bpaf`) with subcommands; consistent with TUI.
- **Storage**: **SQLite** with WAL + **FTS5** for search; `rusqlite` layer.
- **Config & Plugins**: **Lua** via `mlua` (familiar to Neovim ecosystem).
- **Async**: `tokio` for sync/networked plugins (e.g., Todoist).
- **Optional later**: WASM runtime (Wasmtime) for multi‑language plugins.

> Alternate stack: Go + Bubble Tea is viable; Rust picked for perf + safety + Lua embedding maturity.

---

# High‑Level Architecture
```
+----------------------+     +-----------------+     +----------------+
|       CLI (tm)       | --> |   Core Library  | <-- |   TUI (tm-ui)  |
|  (subcommands)       |     |  (tm-core)      |     |  (ratatui)     |
+----------------------+     +-----------------+     +----------------+
                                     |  ^
                                     v  |
                              +-----------------+
                              |  Plugin Host    |
                              | (Lua runtime)   |
                              +-----------------+
                                     |
                           +---------+----------+
                           |         |          |
                     +-----------+ +---------+ +----------------+
                     | Todoist   | | TW I/O | | Custom plugins |
                     | plugin    | | plugin | | (git URL)      |
                     +-----------+ +---------+ +----------------+
                                     |
                                 +-------+
                                 | SQLite|
                                 +-------+
```

- `tm-core`: domain model, repo, indexing, query engine, transactions, events.
- `tm-ui`: views, keymaps, state machine, render loop, command‑line.
- `tm-cli`: pure CLI tools; both frontends call `tm-core`.
- `tm-plugin-host`: loads Lua plugins, exposes host API, manages sandbox & perms.

---

# Data Model (initial)
**Project**
- id (ULID), key (slug), title, description (md), status (active/archived), tags (array), created_at, updated_at, sort_order.

**Task**
- id (ULID), key (slug), project_id, parent_id (nullable → points to another task), title, body (markdown), status (todo/doing/done/cancelled), priority (none/low/med/high/urgent), tags (array), due_at, start_at, done_at, repeat (RRULE), estimate_mins, actual_mins, assignee, created_at, updated_at, sort_order.

**Subtask**
- Represented by **Task** rows where `parent_id` references another Task. Unlimited depth supported, but UI will optimize for 2–3 levels.

**Event log** (append‑only): `entity_type` (project|task), `entity_id`, `action`, `payload`, `ts`, `actor`.

**Indices**: FTS5 on `title`, `body`, `tags`, plus btree on `project_id`, `parent_id`, dates, status.

---

# Commands, Motions, and Modes
**Modes**: Normal / Insert / Visual / Command (`:`), plus Search (`/`).

**Common keybinds (proposed)**
- Navigation: `j/k` move; `gg/G` top/bottom; `Ctrl-d/u` half page; `zt/zz/zb`.
- Selection: `v` visual; `V` line select; `gv` reselect.
- Actions (operators): `d` delete, `y` yank, `c` change, `g~` toggle, `>` indent (promote), `<` outdent (demote), `=` reflow.
- Text‑objects (task‑objects): `it` inner task, `at` around task, `ip` inner project, `aq` around query result.
- Task ops:
  - `x` toggle done; `X` cancel
  - `m` mark (flag/star)
  - `p` set priority; `du` set due; `de` set estimate; `r` set repeat
  - `t` tag add/remove; `P` move to project
  - `:` enter ex‑command (see DSL below)
- Search: `/pattern` live filter; `n/N` next/prev.

**DSL (ex‑commands)**
```
:open inbox               # switch view
:new Pick up dry cleaning +home +errands due:2025-09-02 p:med
:done 1234                # complete by id/handle
:bulk /@work/ p:high      # set priority for filtered set
:move /due<today/ @today  # move filtered tasks to @today project
:sync todoist --full
```

---

# TUI Views
- **List** (Inbox/Project/Tag/Search): multi‑column; sortable; virtualized. Shows tree indicators for subtasks (`›` expanded / `▸` collapsed). `za` toggles fold on a task with children; `zR/zM` open/close all in current view.
- **Detail**: right pane or full‑screen; markdown preview; breadcrumb (`Project / Task / Subtask`). Quick actions include **Add subtask**, **Promote/Demote** (change parent), **Reorder**.
- **Board** (later): column by status/tag/project; honors hierarchy.
- **Calendar** (later): monthly/weekly due/plan grid.
- **Command‑line**: bottom ex‑bar with history & completion.

Layout uses a central **AppState** + unidirectional updates; minimal diff renders.

---

# CLI UX (mirrors core)
```
$ tm add "Pick up dry cleaning" +home +errands due:2025-09-02 p:med
$ tm ls proj:@work status:todo sort:due desc
$ tm done 1234 1235 1236
$ tm edit 1234 title:"Pick up suit" due:+1d
$ tm sync todoist --full
```

- All operations map to core transactions → event log → SQLite.

---

# Config
- Location: `$XDG_CONFIG_HOME/tm/` (or `%APPDATA%\tm` on Windows)
- Main: `config.lua` (keymaps, colors, views, defaults)
- Plugins dir: `plugins/` (git‑managed; see manager below)

**Example `config.lua`**
```lua
-- Keymaps
map('n', 'x', op.toggle_done)
map('n', 'du', ui.prompt_date)
map('n', 't', function() ui.tag_picker() end)

-- Defaults
set('inbox_project', 'Inbox')
set('date_format', 'YYYY-MM-DD')
set('color_scheme', 'gruvbox-dark')

-- Plugins
use({ 'tm-plugins/todoist', version = '>=1.0.0' })
use({ 'yourname/kanban' })
```

---

# Plugin System (v1: Lua)
**Manifest**: `plugin.toml`
```toml
name = "todoist"
id = "tm.todoist"
version = "1.0.0"
entry = "init.lua"
permissions = ["network", "secrets:todoist"]
```

**Entry (`init.lua`)**
```lua
local M = {}

function M.setup(host)
  host.register_command('SyncTodoist', function(args)
    local token = host.secrets.get('todoist')
    local items = host.http.get_json('https://api.todoist.com/rest/v2/tasks', {
      headers = { Authorization = 'Bearer '..token }
    })
    host.tx.begin()
    for _, it in ipairs(items) do host.tasks.upsert(host.map.todoist_to_task(it)) end
    host.tx.commit()
    host.notify('Synced '..#items..' tasks from Todoist')
  end, { desc = 'Sync from Todoist' })
end

return M
```

**Host API (excerpt)**
```ts
host.tasks: { create(t), update(id, patch), upsert(t), by_query(q), delete(id) }
host.query: { parse(str) -> AST, run(ast) -> ids }
host.ui: { prompt(str), pick(list), notify(msg), input(opts) }
host.tx: { begin(), commit(), rollback() }
host.fs: { read(path), write(path, data) }
host.http: { get_json(url, opts), post_json(url, body, opts) }
host.secrets: { get(key), set(key, value) }
host.events: { subscribe(topic, cb), emit(topic, payload) }
```

**Sandboxing & perms**
- Per‑plugin capability grant (network, fs scope, secrets).
- Optional signature verification (plugin registry later).

**Plugin Manager** (lazy‑style)
- `use { repo = 'user/repo', version = '...' }`
- Supports git URLs, local path, version pins.
- Auto install/update; lockfile `tm-lock.json`.

---

# Storage & Sync
- SQLite schemas with migrations; WAL on by default.
- FTS5 virtual table for fast search.
- Event log → deterministic, debuggable sync.
- Todoist plugin maps fields; conflict strategy: host wins or newest wins.
- Importers: Taskwarrior/CSV/JSON (as plugins).

---

# Performance Plan
- Virtualized lists; diff‑based rendering.
- Background indexing; query cache.
- Batch transactions for imports/syncs.
- Startup target < 50ms with lazy plugin init.

---

# Testing & Tooling
- Core property tests (task ops), snapshot tests for TUI (
`ratatui` test harness), CLI golden tests.
- Benchmarks: add/list/search on 10k/100k tasks.
- Lints/format: `rustfmt`, `clippy`.

---

# Security & Telemetry
- Opt‑in anonymous metrics; redact content.
- Secrets stored via OS keychain when available.

---

# Repo Layout
```
.
├── crates/
│   ├── tm-core/
│   ├── tm-ui/
│   ├── tm-cli/
│   └── tm-plugin-host/
├── plugins/
│   └── todoist/ (example)
├── docs/
│   ├── api/
│   └── design/
├── assets/
└── tm (workspace binary)
```

---

# Roadmap (MVP → v0.5)
**MVP (0.1)**
- Core model + SQLite + FTS5
- TUI list + detail; Normal/Insert/Command
- Basic motions (`j/k/gg/G`, `/`, `:new`, `:done`)
- CLI add/ls/done
- Config.lua, keymaps

**0.2**
- Plugin host (Lua) + Plugin manager (git install)
- Todoist read‑only sync

**0.3**
- Write‑back to Todoist; conflict handling
- Visual mode + bulk ops; saved filters

**0.4**
- Board view; recurring tasks
- Task templates/snippets

**0.5**
- Calendar view; reminders
- Plugin registry alpha; signatures

---

# Decisions from Q&A (v1 locks)

## Human‑Editable Store: Best of Both
- **Authoritative on disk**: Markdown files with **YAML frontmatter** + optional Markdown body. Example:
  ```markdown
  ---
  id: 01J8ZX6K8W5K3ZP4X2V8XKQ9QG   # ULID
  key: pick-up-dry-cleaning        # human slug (stable once set)
  title: Pick up dry cleaning
  status: todo                     # todo|doing|done|cancelled
  project: @home
  tags: [errands]
  priority: med
  due: 2025-09-02
  created: 2025-08-29T12:34:56Z
  updated: 2025-08-29T12:34:56Z
  ---
  Notes here in Markdown. Check hours before going.
  ```
- **Fast index**: Background **SQLite** index mirrors frontmatter + body for FTS and queries. TUI/CLI write both (file + index) in a single transaction; external edits detected via file watcher → reindex.
- **Performance**: Queries/read use SQLite; writes update the Markdown first, then index. If index corrupt, rebuild from files (`tm reindex`).
- **Round‑trip safety**: Unknown frontmatter keys preserved. Body is free‑form Markdown.

### File layout
```
~/TasksVault/
├── projects/
│   ├── 2025/
│   │   └── 2025-08-29--work-platform--01J8Z....md
│   └── index.md  # optional project catalog
├── tasks/
│   └── 2025/08/
│       ├── 2025-08-29--pick-up-dry-cleaning--01J8Z....md
│       └── 2025-08-29--tailor-pants--01J8Z....md
└── .tm/
    ├── index.sqlite (WAL)
    └── config.lua
```
- **Project files** are Markdown with frontmatter (id/key/title/description/status/tags).
- **Tasks** can encode hierarchy in frontmatter: `project: work-platform`, `parent: 01J8Z...` (or by slug).
- Optional directory convention for subtasks is **not required**; we keep flat storage + explicit `parent` links to avoid file move churn.

## IDs & Handles
- **Primary ID**: ULID (sortable, unique).
- **Human key/slug**: derived from first title, editable but **stabilized** after creation (we keep `key` in frontmatter). If user changes it, TUI updates filename on save.
- **Handles for commands**: Accept any of:
  - ULID (`01J8ZX…`)
  - `key` (`pick-up-dry-cleaning`)
  - Date + key (`2025-08-29/pick-up-dry-cleaning`)
  - Short ULID prefix if unique (`01J8ZX6`)

## Command Palettes (two styles)
- **Global Fuzzy (Telescope‑like)** — `<leader>p`:
  - Sources: commands, tasks, projects, tags, saved filters.
  - Multi-source with prefixes: `>` for commands, `#` for tags, `@` for projects.
- **Quick Actions (VS Code‑like)** — `<leader>a`:
  - Context‑aware verbs for the current task/view: `Complete`, `Edit due`, `Move to project`, etc.
- `/` remains the live filter in list view.
- `:` opens ex‑bar for the DSL.

## Plugins & Permissions
- **Lua plugins** at launch; full FS + network by default (**v1**), with a visible banner on first run: "Plugins have full access; enable limited‑perms mode in config when available."
- Future: capability flags (`network`, `fs-scope`, `secrets`) and prompts per plugin.

## Keymaps (initial defaults)
- Navigation: `j/k`, `gg/G`, `Ctrl-d/u`, `zt/zz/zb`.
- Task ops (subject to the new language):
  - `x` toggle done, `:done <handle>`
  - `du` set due (invokes date picker)
  - `p` set priority, `t` add/remove tags
  - `:` ex‑bar, `/` filter, `<leader>p` palette, `<leader>a` quick actions
- **Leader** configurable (default `\`).
- No visual mode; bulk ops via `:bulk` over the filtered set.

## CLI (mirrors core; simple by default)
```
# Create
$ tm add "Pick up dry cleaning" +home +errands due:2025-09-02 p:med

# List (uses SQLite index)
$ tm ls project:@home status:todo sort:due

# Update
$ tm edit pick-up-dry-cleaning due:+1d

# Complete
$ tm done 01J8ZX6K8W5K3ZP4X2V8XKQ9QG

# Export/Import
$ tm export --json > tasks.json
$ tm import tasks.json
```
- DSL optional; flags kept minimal. JSON I/O is available but not required by TUI.

## Config & Theming
- `~/.tm/config.lua` (dotfile‑friendly). Vault path configurable.
- Default theme: **Gruvbox dark** (truecolor). Theming system exists but not prioritized.
- **Hot‑reload** for config; plugin hot‑reload later.

## Performance Targets
- Cold start: **<250ms** on typical dev laptop; **lazy‑load** plugins.
- Reindex from disk (2–5k tasks) should finish in seconds; done incrementally on change.

## Roadmap adjustments
- MVP focuses on Markdown+SQLite hybrid, two palettes, live filters, list+detail, CLI parity, plugin loader.
- Post‑MVP: permissions model, board view, recurring tasks, event log/undo, calendar, sync plugins (Jira/GitLab/GitHub/Todoist), Obsidian helpers (template commands).



---

# Projects, Tasks, Subtasks — Behavior & DSL

## DSL quick commands
```
# Projects
:project.new Work Platform +engineering
:project.rename work-platform "Core Platform"
:project.archive work-platform
:open project:work-platform

# Tasks
:new "Pick up dry cleaning" +home +errands due:2025-09-02 p:med project:life-admin
:new "Tailor pants" project:life-admin parent:pick-up-dry-cleaning  # subtask
:done pick-up-dry-cleaning
:promote 01J8Z...                 # subtask → top level
:demote 01J8ZCHILD under 01J8ZPAR  # reparent under another task
:move 01J8Z... project:work-platform
:reorder 01J8Z... before 01J8ZA    # sibling ordering

# Bulk over current filter
:bulk /project:work-platform status:todo p:high
```

## CLI parallels
```
$ tm project add "Work Platform" +engineering
$ tm project ls
$ tm project archive work-platform

$ tm add "Pick up dry cleaning" --project life-admin --due 2025-09-02 --prio med +home +errands
$ tm add "Tailor pants" --project life-admin --parent pick-up-dry-cleaning
$ tm ls --project life-admin --tree
$ tm reparent 01J8ZCHILD --parent 01J8ZPAR
$ tm reorder 01J8Z --before 01J8ZA
```

## Semantics & validation
- A **Project** owns tasks; deleting a project requires empty or explicit `--with-tasks` migration.
- A **Task** may have a **parent** (becoming a subtask). Circular parenting prevented.
- **Status roll‑up**: parent shows progress as % done of children; completing a parent auto‑prompts to complete children (configurable).
- **Due dates**: parent due defaults to the max(due of children) unless explicitly set.
- **Ordering**: stable per‑parent `sort_order`; manual reorder via commands or drag in a future UI.
- **Queries** support hierarchy filters: `has:children`, `is:leaf`, `parent:<id|key>`.

