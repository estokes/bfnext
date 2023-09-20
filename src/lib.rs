use mlua::{prelude::*, Value};

fn print_value(key: bool, val: bool, lvl: usize, v: &Value) {
    fn indent(lvl: usize) {
        for _ in 0..lvl {
            print!(" ");
        }
    }
    match v {
        Value::Nil
        | Value::Boolean(_)
        | Value::LightUserData(_)
        | Value::Integer(_)
        | Value::Number(_)
        | Value::UserData(_)
        | Value::String(_)
        | Value::Function(_)
        | Value::Thread(_)
        | Value::Error(_) => {
            if key {
                indent(lvl);
            }
            if key || val {
                print!("{:?}", v)
            } else {
                println!("{:?}", v)
            }
        }
        Value::Table(tbl) => {
            if !val {
                indent(lvl);
            }
            println!("{}", "{");
            for pair in tbl.clone().pairs::<Value, Value>() {
                let (k, v) = pair.unwrap();
                print_value(true, false, lvl + 4, &k);
                print!(" = ");
                print_value(false, true, lvl + 4, &v);
                println!(", ")
            }
            indent(lvl);
            if val {
                print!("{}", "}")
            } else {
                println!("{}", "}")
            }
        }
    }
}

fn on_player_try_connect(
    _: &Lua,
    (addr, name, ucid, id): (Value, Value, Value, Value),
) -> LuaResult<bool> {
    println!(
        "onPlayerTryConnect addr: {:?}, name: {:?}, ucid: {:?}, id: {:?}",
        addr, name, ucid, id
    );
    Ok(true)
}

fn on_player_try_send_chat<'a>(
    _: &Lua,
    (id, msg, all): (Value<'a>, Value<'a>, Value<'a>),
) -> LuaResult<Value<'a>> {
    println!(
        "onPlayerTrySendChat id: {:?}, msg: {:?}, all: {:?}",
        id, msg, all
    );
    Ok(msg)
}

fn on_player_try_change_slot(_: &Lua, (id, side, slot): (i64, i64, String)) -> LuaResult<bool> {
    println!(
        "onPlayerTryChangeSlot id: {:?}, side: {:?}, slot: {:?}",
        id, side, slot
    );
    Ok(true)
}

fn on_event(_: &Lua, (_, ev): (Value, Value)) -> LuaResult<()> {
    print!("onEvent: ");
    print_value(false, false, 0, &ev);
    Ok(())
}

#[mlua::lua_module]
fn bflib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set(
        "onPlayerTryConnect",
        lua.create_function(on_player_try_connect)?,
    )?;
    exports.set(
        "onPlayerTrySendChat",
        lua.create_function(on_player_try_send_chat)?,
    )?;
    exports.set(
        "onPlayerTryChangeSlot",
        lua.create_function(on_player_try_change_slot)?,
    )?;
    exports.set("onEvent", lua.create_function(on_event)?)?;
    Ok(exports)
}
