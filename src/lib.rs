use mlua::{prelude::*, Value};

fn on_player_try_connect(_: &Lua, (addr, name, ucid, id): (Value, Value, Value, Value)) -> LuaResult<bool> {
    println!("onPlayerTryConnect addr: {:?}, name: {:?}, ucid: {:?}, id: {:?}", addr, name, ucid, id);
    Ok(true)
}

fn on_player_try_send_chat<'a>(_: &Lua, (id, msg, all): (Value<'a>, Value<'a>, Value<'a>)) -> LuaResult<Value<'a>> {
    println!("onPlayerTrySendChat id: {:?}, msg: {:?}, all: {:?}", id, msg, all);
    Ok(msg)
}

fn on_player_try_change_slot(_: &Lua, (id, side, slot): (i64, i64, String)) -> LuaResult<bool> {
    println!("onPlayerTryChangeSlot id: {:?}, side: {:?}, slot: {:?}", id, side, slot);
    Ok(true)
}

fn on_event(_: &Lua, ev: Value) -> LuaResult<()> {
    println!("onEvent {:?}", ev);
    Ok(())
}

#[mlua::lua_module]
fn bflib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("onPlayerTryConnect", lua.create_function(on_player_try_connect)?)?;
    exports.set("onPlayerTrySendChat", lua.create_function(on_player_try_send_chat)?)?;
    exports.set("onPlayerTryChangeSlot", lua.create_function(on_player_try_change_slot)?)?;
    exports.set("onEvent", lua.create_function(on_event)?)?;
    Ok(exports)
}
