use std::{collections::HashMap, fs, path::PathBuf};

use anyhow::{anyhow, Context, Result};
use directories::ProjectDirs;
use mlua::{Lua, Table, Value};

use crate::Action;

/// Cross-frontend keymap: normalized tokens like "j", "k", "Ctrl-d", "G", "/", "1"
#[derive(Clone, Debug, Default)]
pub struct Keymap {
    pub normal: HashMap<String, Action>,
}

impl Keymap {
    pub fn lookup(&self, token: &str) -> Option<Action> {
        self.normal.get(token).copied()
    }
}

/// Built-in defaults (what we hardcoded previously)
pub fn default_keymap() -> Keymap {
    use Action::*;
    let mut m = HashMap::new();

    // navigation
    m.insert("j".into(), MoveDown);
    m.insert("Down".into(), MoveDown);
    m.insert("k".into(), MoveUp);
    m.insert("Up".into(), MoveUp);
    m.insert("Ctrl-d".into(), HalfPageDown);
    m.insert("Ctrl-u".into(), HalfPageUp);
    m.insert("G".into(), GoBottom);
    m.insert("/".into(), FocusFilter);
    m.insert("q".into(), Quit);

    // status
    m.insert("x".into(), StatusNext);
    m.insert("X".into(), StatusPrev);
    m.insert("1".into(), SetTodo);
    m.insert("2".into(), SetDoing);
    m.insert("3".into(), SetDone);

    Keymap { normal: m }
}

/// XDG: ~/.config/tm/config.lua  (also accept ~/.config/tm/config as a plain file)
pub fn default_config_path() -> PathBuf {
    let proj = ProjectDirs::from("dev", "example", "tm").expect("project dirs");
    let base = proj.config_dir().to_path_buf(); // ~/.config/tm
    let lua = base.join("config.lua");
    let plain = base.join("config");
    if lua.exists() {
        lua
    } else if plain.exists() {
        plain
    } else {
        lua
    }
}

/// Try to load user keymap; fallback to defaults on any error
pub fn load_keymap_from_user() -> Keymap {
    let path = default_config_path();
    if !path.exists() {
        return default_keymap();
    }
    match load_keymap_from_file(&path) {
        Ok(km) => km,
        Err(e) => {
            eprintln!("[tm] failed to load keymap from {:?}: {e}", path);
            default_keymap()
        }
    }
}

fn load_keymap_from_file(path: &PathBuf) -> Result<Keymap> {
    let lua_src = fs::read_to_string(path).with_context(|| format!("reading {:?}", path))?;

    // IMPORTANT: never bubble mlua::Error with `?` directly; map to string.
    let lua = Lua::new();
    let cfg_val = lua
        .load(&lua_src)
        .eval::<Value>()
        .map_err(|e| anyhow!(e.to_string()))?;

    let cfg_tbl: Table = match cfg_val {
        Value::Table(t) => t,
        _ => return Ok(default_keymap()),
    };

    let mut km = default_keymap(); // start with defaults, allow overrides

    // cfg.keymaps.normal = { ["j"] = "move_down", ... }
    if let Ok(keymaps_val) = cfg_tbl.get::<Value>("keymaps") {
        if let Value::Table(keymaps_tbl) = keymaps_val {
            if let Ok(normal_val) = keymaps_tbl.get::<Value>("normal") {
                if let Value::Table(normal_tbl) = normal_val {
                    for pair in normal_tbl.pairs::<Value, Value>() {
                        // Map mlua::Error to anyhow via to_string()
                        let (k, v) = pair.map_err(|e| anyhow!(e.to_string()))?;

                        // token (key)
                        let token = match k {
                            Value::String(s) => s
                                .to_str()
                                .map_err(|e| anyhow!(e.to_string()))?
                                .to_string(),
                            Value::Integer(n) => n.to_string(),
                            Value::Number(n) => n.to_string(),
                            _ => continue,
                        };

                        // action string
                        let action_name = match v {
                            Value::String(s) => s
                                .to_str()
                                .map_err(|e| anyhow!(e.to_string()))?
                                .to_string(),
                            _ => continue,
                        };

                        if let Some(act) = parse_action_name(&action_name) {
                            km.normal.insert(token, act);
                        }
                    }
                }
            }
        }
    }

    Ok(km)
}

/// Map action names from Lua strings to Action enum
fn parse_action_name(s: &str) -> Option<Action> {
    use Action::*;
    match s {
        // nav
        "move_down" => Some(MoveDown),
        "move_up" => Some(MoveUp),
        "half_page_down" => Some(HalfPageDown),
        "half_page_up" => Some(HalfPageUp),
        "go_top" => Some(GoTop),
        "go_bottom" => Some(GoBottom),
        "focus_filter" => Some(FocusFilter),
        "quit" => Some(Quit),

        // status ops
        "status_next" => Some(StatusNext),
        "status_prev" => Some(StatusPrev),
        "set_todo" => Some(SetTodo),
        "set_doing" => Some(SetDoing),
        "set_done" => Some(SetDone),

        _ => None,
    }
}

