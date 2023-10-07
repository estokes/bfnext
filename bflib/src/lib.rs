use std::{sync::atomic::AtomicUsize, thread, time::Duration};

use compact_str::format_compact;
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
    wrap_unit, String, UserHooks, Vec2, DeepClone,
};
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use once_cell::sync::Lazy;
use parking_lot::Mutex;

#[derive(Default)]
struct Context {
    instance_id: usize,
    idx: env::miz::MizIndex,
    instances: FxHashMap<usize, String>,
}

impl Context {
    fn new_instance_id(&mut self) -> usize {
        let iid = self.instance_id;
        self.instance_id += 1;
        iid
    }

    fn instance_template(&mut self, name: &str, group: &env::miz::Group) -> LuaResult<usize> {
        let id = self.new_instance_id();
        let group_name = String::from(format_compact!("{}{}", name, id));
        group.set("lateActivation", false)?;
        group.set_name(group_name.clone())?;
        for (i, unit) in group.units()?.into_iter().enumerate() {
            let unit = unit?;
            unit.set_name(String::from(format_compact!("{}{}{}", name, id, i)))?
        }
        self.instances.insert(id, group_name);
        Ok(id)
    }
}

static CONTEXT: Lazy<Mutex<Context>> = Lazy::new(|| Mutex::new(Context::default()));

enum SpawnLoc {
    AtPos(Vec2),
    AtTrigger(String),
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
    ctx.instance_template(name, &ifo.group)?;
    let loc = match location {
        SpawnLoc::AtPos(pos) => *pos,
        SpawnLoc::AtTrigger(name) => {
            let tz = dbg!(miz.get_trigger_zone(&ctx.idx, name.as_str()))?
                .ok_or_else(|| err("no such trigger zone"))?;
            tz.pos()?
        }
    };
    ifo.group.set_pos(loc)?;
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
        &SpawnLoc::AtTrigger("TEST_TZ".into()),
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
