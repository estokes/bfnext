mod db;
extern crate nalgebra as na;
use compact_str::format_compact;
use db::{Db, GroupId, SpawnLoc, SpawnedGroup, SpawnedUnit, UnitId};
use dcso3::{
    coalition::{Coalition, Side},
    env::{self, miz::GroupKind},
    err,
    event::Event,
    group::GroupCategory,
    timer::Timer,
    world::World,
    wrap_unit, String, UserHooks, Vector2,
};
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

#[derive(Debug, Default)]
struct Context {
    idx: env::miz::MizIndex,
    db: Db,
    units_by_obj_id: FxHashMap<i64, UnitId>,
}

static CONTEXT: Lazy<Mutex<Context>> = Lazy::new(|| Mutex::new(Context::default()));

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

fn init_miz_(lua: &Lua) -> LuaResult<()> {
    println!("adding event handler");
    World::get(lua)?.add_event_handler(on_event)?;
    println!("scheduling print");
    let timer = Timer::singleton(lua)?;
    timer.schedule_function(timer.get_time()? + 10., Value::Nil, move |_lua, _, time| {
        println!("scheduled function {}", time);
        Ok(Some(time + 10.))
    })?;
    println!("spawning");
    let ctx = &*CONTEXT;
    let mut ctx = ctx.lock();
    let idx = &ctx.idx;
    let db = &mut ctx.db;
    db.spawn_template_as_new(
        lua,
        idx,
        Side::Blue,
        GroupKind::Vehicle,
        &SpawnLoc::AtTrigger {
            name: "TEST_TZ".into(),
            offset: Vector2::new(10., 10.),
        },
        "TMPL_TEST_GROUP",
    )?;
    db.spawn_template_as_new(
        lua,
        idx,
        Side::Blue,
        GroupKind::Vehicle,
        &SpawnLoc::AtTrigger {
            name: "TEST_TZ".into(),
            offset: Vector2::new(-10., -10.),
        },
        "TMPL_TEST_GROUP",
    )?;
    println!("spawned");
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
