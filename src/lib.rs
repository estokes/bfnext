pub mod dcs;
use mlua::{prelude::*, Value};
use std::{io, collections::{HashMap, hash_map::Entry}, cell::RefCell};
use fxhash::FxHashMap;

fn to_json(ctx: &mut FxHashMap<usize, String>, key: Option<&str>, v: &Value) -> serde_json::Value {
    use serde_json::{Value as JVal, json, Map};
    match v {
        Value::Nil => JVal::Null,
        Value::Boolean(b) => json!(b),
        Value::LightUserData(_) => json!("<LightUserData>"),
        Value::Integer(i) => json!(*i),
        Value::Number(i) => json!(*i),
        Value::UserData(_) => json!("<UserData>"),
        Value::String(s) => json!(s),
        Value::Function(_) => json!("<Function>"),
        Value::Thread(_) => json!("<Thread>"),
        Value::Error(e) => json!(format!("{e}")), 
        Value::Table(tbl) => {
            let address = tbl.to_pointer() as usize;
            match ctx.entry(address) {
                Entry::Occupied(e) => json!(format!("<Table(0x{:x} {})>", address, e.get())),
                Entry::Vacant(e) => {
                    e.insert(String::from(key.unwrap_or("Root")));
                    let mut map = Map::new();
                    for pair in tbl.clone().pairs::<Value, Value>() {
                        let (k, v) = pair.unwrap();
                        let k = match to_json(ctx, None, &k) {
                            JVal::String(s) => s,
                            v => v.to_string()
                        };
                        let v = to_json(ctx, Some(k.as_str()), &v);
                        map.insert(k, v);
                    }
                    JVal::Object(map)
                }
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

//serde_json::to_writer_pretty(&mut io::stdout(), &to_json(&mut *ctx, None, &Value::Table(lua.globals()))).unwrap();
fn on_event(lua: &Lua, (_, ev): (Value, Value)) -> LuaResult<()> {
    thread_local! {
        static CTX: RefCell<FxHashMap<usize, String>> = RefCell::new(HashMap::default());
    }
    CTX.with(|ctx| {
        let mut ctx = ctx.borrow_mut();
        ctx.clear();
        print!("onEvent: ");
        serde_json::to_writer_pretty(&mut io::stdout(), &to_json(&mut ctx, None, &ev)).unwrap();
        println!();
        println!("onEventTranslated: {:#?}", dcs::Event::from_lua(ev, lua));
    });
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
