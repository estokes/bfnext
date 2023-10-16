mod db;
extern crate nalgebra as na;
use compact_str::format_compact;
use db::{Db, GroupId, SpawnedGroup, SpawnedUnit, UnitId};
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

impl Context {
}

static CONTEXT: Lazy<Mutex<Context>> = Lazy::new(|| Mutex::new(Context::default()));

enum SpawnLoc {
    AtPos(Vector2),
    AtTrigger { name: String, offset: Vector2 },
}

fn spawn_template<'lua>(
    lua: &'lua Lua,
    ctx: &mut Context,
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
        SpawnLoc::AtTrigger { name, offset } => {
            let tz = dbg!(miz.get_trigger_zone(&ctx.idx, name.as_str()))?
                .ok_or_else(|| err("no such trigger zone"))?;
            tz.pos()? + offset
        }
    };
    ctx.instance_template(name, loc, &ifo.group)?;
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
    spawn_template(
        lua,
        &mut *ctx,
        Side::Blue,
        GroupKind::Vehicle,
        &SpawnLoc::AtTrigger {
            name: "TEST_TZ".into(),
            offset: Vector2::new(10., 10.),
        },
        "TMPL_TEST_GROUP",
    )?;
    spawn_template(
        lua,
        &mut *ctx,
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
