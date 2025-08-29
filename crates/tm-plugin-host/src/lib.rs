//! Minimal Lua host (skeleton)
use mlua::{Lua, Result as LuaResult}; // note: use mlua::Result

pub fn init_lua() -> LuaResult<Lua> {
    let lua = Lua::new();
    let globals = lua.globals();
    globals.set("print_host", lua.create_function(|_, msg: String| {
        println!("[host] {}", msg);
        Ok(())
    })?)?;
    Ok(lua)
}

