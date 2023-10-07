use std::{thread, time::Duration};

use dcso3::{
    coalition::{Coalition, Side},
    country::Country,
    env::{self, miz::GroupKind},
    err,
    event::Event,
    group::{Group, GroupCategory},
    timer::Timer,
    value_to_json,
    world::World,
    wrap_unit, String, UserHooks, Vec2,
};
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

#[derive(Default)]
struct Context {
    idx: env::miz::MizIndex,
}

static CONTEXT: Lazy<Mutex<Context>> = Lazy::new(|| Mutex::new(Context::default()));

enum SpawnLoc {
    AtPos(Vec2),
    AtTrigger(String),
}

fn spawn<'lua>(
    lua: &'lua Lua,
    ctx: &Context,
    side: Side,
    kind: GroupKind,
    location: &SpawnLoc,
    name: &str,
) -> LuaResult<()> {
    let coalition = dbg!(Coalition::singleton(lua))?;
    let miz = dbg!(env::miz::Miz::singleton(lua))?;
    let ifo =
        dbg!(miz.get_group(&ctx.idx, kind, side, name))?.ok_or_else(|| err("no such group"))?;
    let loc = match location {
        SpawnLoc::AtPos(pos) => *pos,
        SpawnLoc::AtTrigger(name) => {
            let tz = dbg!(miz.get_trigger_zone(&ctx.idx, name.as_str()))?
                .ok_or_else(|| err("no such trigger zone"))?;
            tz.pos()?
        }
    };
    //dbg!(ifo.group.set_name(group_name.clone()))?;
    dbg!(ifo.group.set_pos(loc))?;
    match GroupCategory::from_kind(ifo.category) {
        None => dbg!(coalition.add_static_object(ifo.country, ifo.group)),
        Some(category) => dbg!(coalition.add_group(ifo.country, category, ifo.group)),
    }
}

fn on_player_try_connect(
    _: &Lua,
    addr: String,
    name: String,
    ucid: String,
    id: u32,
) -> LuaResult<bool> {
    println!(
        "onPlayerTryConnect addr: {:?}, name: {:?}, ucid: {:?}, id: {:?}",
        addr, name, ucid, id
    );
    Ok(true)
}

fn on_player_try_send_chat(_: &Lua, id: u32, msg: String, all: bool) -> LuaResult<String> {
    println!(
        "onPlayerTrySendChat id: {:?}, msg: {:?}, all: {:?}",
        id, msg, all
    );
    Ok(msg)
}

fn on_player_try_change_slot(_: &Lua, id: u32, side: Side, slot: String) -> LuaResult<bool> {
    println!(
        "onPlayerTryChangeSlot id: {:?}, side: {:?}, slot: {:?}",
        id, side, slot
    );
    Ok(true)
}

fn on_event(_lua: &Lua, ev: Event) -> LuaResult<()> {
    println!("onEventTranslated: {:?}", ev);
    Ok(())
}

fn on_mission_load_end(lua: &Lua) -> LuaResult<()> {
    println!("on_mission_load_end");
    let miz = dbg!(env::miz::Miz::singleton(lua))?;
    println!("indexing mission");
    CONTEXT.lock().idx = miz.index()?;
    println!("indexed mission");
    Ok(())
}

fn on_simulation_start(_lua: &Lua) -> LuaResult<()> {
    println!("on_simulation_start");
    Ok(())
}

fn init_hooks_(lua: &Lua) -> LuaResult<()> {
    println!("setting user hooks");
    UserHooks::new(lua)
        .on_simulation_start(on_simulation_start)?
        .on_mission_load_end(on_mission_load_end)?
        .on_player_try_change_slot(on_player_try_change_slot)?
        .on_player_try_connect(on_player_try_connect)?
        .on_player_try_send_chat(on_player_try_send_chat)?
        .register()?;
    println!("set user hooks");
    Ok(())
}

fn init_hooks(lua: &Lua, _: ()) -> LuaResult<()> {
    wrap_unit("init_hooks", init_hooks_(lua))
}
/*
    println!("scheduling spawn");
    let timer = Timer::singleton(lua)?;
    timer.schedule_function(timer.get_time()? + 60., Value::Nil, move |lua, _, _time| {
        println!("spawning");
        let ctx = &*CONTEXT;
        let ctx = ctx.lock();
        spawn(
            lua,
            &*ctx,
            Side::Blue,
            GroupKind::Vehicle,
            &SpawnLoc::AtTrigger("TEST_TZ".into()),
            "TMPL_TEST_GROUP",
        )?;
        Ok(None)
    })?;
*/

/*
    dbg!(timer.schedule_function(timer.get_time()? + 60., Value::Nil, move |_lua, _, time| {
        println!("scheduled function {}", time);
        Ok(Some(time + 60.))
    }))?;
*/

fn init_miz_(lua: &Lua) -> LuaResult<()> {
    println!("adding event handler");
    World::get(lua)?.add_event_handler(on_event)?;
    println!("scheduling print");
    let timer = dbg!(Timer::singleton(lua))?;
    dbg!(timer.get_time());
    dbg!(timer.get_abs_time());
    dbg!(timer.get_time0());
    dbg!(
        timer.schedule_function(timer.get_time()? + 10., Value::Nil, move |_lua, _, time| {
            println!("scheduled function {}", time);
            Ok(Some(time + 10.))
        })
    )?;
    Ok(())
}

fn init_miz(lua: &Lua, _: ()) -> LuaResult<()> {
    wrap_unit("init_miz", init_miz_(lua))
}

#[mlua::lua_module]
fn bflib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("initHooks", lua.create_function(init_hooks)?)?;
    exports.set("initMiz", lua.create_function(init_miz)?)?;
    Ok(exports)
}
