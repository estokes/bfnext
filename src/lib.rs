use mlua::prelude::*;

fn hello(_: &Lua, name: String) -> LuaResult<()> {
    println!("hello, {}!", name);
    Ok(())
}

fn on_player_try_connect(_: &Lua, (addr, name, ucid, id): (String, String, String, String)) -> LuaResult<bool> {
    println!("onPlayerTryConnect addr: {addr}, name: {name}, ucid: {ucid}, id: {id}");
    Ok(true)
}

fn on_player_try_send_chat(_: &Lua, (id, msg, all): (String, String, bool)) -> LuaResult<String> {
    println!("onPlayerTrySendChat id: {id}, msg: {msg}, all: {all}");
    Ok(msg)
}

fn on_player_try_change_slot(_: &Lua, (id, side, slot): (String, String, String)) -> LuaResult<bool> {
    println!("onPlayerTryChangeSlot id: {id}, side: {side}, slot: {slot}");
    Ok(true)
}

#[mlua::lua_module]
fn bflib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("hello", lua.create_function(hello)?)?;
    exports.set("onPlayerTryConnect", lua.create_function(on_player_try_connect)?)?;
    exports.set("onPlayerTrySendChat", lua.create_function(on_player_try_send_chat)?)?;
    exports.set("onPlayerTryChangeSlot", lua.create_function(on_player_try_change_slot)?)?;
    Ok(exports)
}
