use dcso3::{
    coalition::{Coalition, Side},
    country::Country,
    env::{self, miz::GroupKind},
    err,
    event::Event,
    group::{Group, GroupCategory},
    world::World,
    String, UserHooks, Vec2,
};
use mlua::prelude::*;
use std::{cell::RefCell, ops::Deref, rc::Rc};

#[derive(Default)]
struct ContextInner {
    idx: env::miz::MizIndex,
}

#[derive(Clone, Default)]
struct Context(Rc<RefCell<ContextInner>>);

impl Deref for Context {
    type Target = RefCell<ContextInner>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Context {
    fn get(lua: &Lua) -> LuaResult<Self> {
        Ok(lua
            .app_data_ref::<Self>()
            .ok_or_else(|| err("missing context"))?
            .clone())
    }
}

enum SpawnLoc {
    AtPos(Vec2),
    AtTrigger(String),
}

fn spawn<'lua>(
    lua: &'lua Lua,
    ctx: &ContextInner,
    side: Side,
    kind: GroupKind,
    location: &SpawnLoc,
    miz_name: &str,
    group_name: String,
) -> LuaResult<()> {
    let coalition = Coalition::singleton(lua)?;
    let miz = env::miz::Miz::singleton(lua)?;
    let ifo = miz
        .get_group(&ctx.idx, kind, side, miz_name)?
        .ok_or_else(|| err("no such group"))?;
    let loc = match location {
        SpawnLoc::AtPos(pos) => *pos,
        SpawnLoc::AtTrigger(name) => {
            let tz = miz
                .get_trigger_zone(&ctx.idx, name.as_str())?
                .ok_or_else(|| err("no such trigger zone"))?;
            tz.pos()?
        }
    };
    ifo.group.set_name(group_name.clone())?;
    ifo.group.set_pos(loc)?;
    match GroupCategory::from_kind(ifo.category) {
        None => coalition.add_static_object(ifo.country, ifo.group),
        Some(category) => coalition.add_group(ifo.country, category, ifo.group),
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
    println!("onEventTranslated: {:#?}", ev);
    Ok(())
}

fn on_mission_load_end(lua: &Lua) -> LuaResult<()> {
    let ctx = Context::get(lua)?;
    let miz = env::miz::Miz::singleton(lua)?;
    ctx.borrow_mut().idx = miz.index()?;
    Ok(())
}

fn on_simulation_start(lua: &Lua) -> LuaResult<()> {
    let ctx = Context::get(lua)?;
    let ctx = ctx.borrow();
    spawn(
        lua,
        &*ctx,
        Side::Blue,
        GroupKind::Vehicle,
        &SpawnLoc::AtTrigger("TEST_TZ".into()),
        "TMPL_TEST_GROUP",
        "TEST_GROUP".into(),
    )
}

fn init_hooks(lua: &Lua, _: ()) -> LuaResult<()> {
    if let None = lua.app_data_ref::<Context>() {
        lua.set_app_data(Context::default());
    }
    UserHooks::new(lua)
        .on_simulation_start(on_simulation_start)?
        .on_mission_load_end(on_mission_load_end)?
        .on_player_try_change_slot(on_player_try_change_slot)?
        .on_player_try_connect(on_player_try_connect)?
        .on_player_try_send_chat(on_player_try_send_chat)?
        .register()
}

fn init_miz(lua: &Lua, _: ()) -> LuaResult<()> {
    if let None = lua.app_data_ref::<Context>() {
        lua.set_app_data(Context::default());
    }
    World::get(lua)?.add_event_handler(on_event)?;
    Ok(())
}

#[mlua::lua_module]
fn bflib(lua: &Lua) -> LuaResult<LuaTable> {
    let exports = lua.create_table()?;
    exports.set("initHooks", lua.create_function(init_hooks)?)?;
    exports.set("initMiz", lua.create_function(init_miz)?)?;
    Ok(exports)
}
