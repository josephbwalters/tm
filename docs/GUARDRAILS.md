# Guardrails for Simplicity

## Principles
- Obvious over clever; minimal surface; stable names.
- Functional core, imperative shell.
- One way to do each thing.

## Coding Rules
- No global singletons; pass deps explicitly.
- Errors: anyhow in shell, enums in core. No panics in core.
- Tests: golden/snapshot for queries, round‑trip MD↔Task, basic TUI render.

## Project Structure
- No utils crate. Shared code under tm-core::util until proven.
- Filenames match domain nouns.

## Dependencies
- Keep minimal: serde, ulid, rusqlite, mlua, ratatui, crossterm, notify.

## PR Checklist
- [ ] Public docs for exported APIs
- [ ] At least one test per new module
- [ ] No direct FS/DB from UI/CLI
- [ ] Names match domain language
- [ ] cargo fmt, clippy clean

## Simplicity Gate
1. Reduces steps for common workflows?
2. Understandable in one file?
3. Exactly one clear place to put code?
4. Perf in budget (<250ms cold start, <50ms ls for 5k tasks)?
