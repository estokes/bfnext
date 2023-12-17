pub mod bg;
pub mod cfg;
pub mod db;
pub mod menu;
pub mod spawnctx;

extern crate nalgebra as na;
use crate::{cfg::Cfg, db::player::SlotAuth};
use anyhow::{anyhow, bail, Result};
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use db::{Db, UnitId};
use dcso3::{
    coalition::Side,
    env::{self, miz::Miz, Env},
    event::Event,
    hooks::UserHooks,
    lfs::Lfs,
    net::{Net, PlayerId, SlotId, Ucid},
    timer::Timer,
    trigger::Trigger,
    unit::Unit,
    world::World,
    HooksLua, LuaEnv, MizLua, String, Vector2,
};
use fxhash::{FxHashMap, FxHashSet};
use log::{debug, error, info};
use mlua::prelude::*;
use spawnctx::SpawnCtx;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
struct PlayerInfo {
    name: String,
    ucid: Ucid,
}

#[derive(Debug, Default)]
struct Context {
    idx: env::miz::MizIndex,
    db: Db,
    to_background: Option<UnboundedSender<bg::Task>>,
    units_by_obj_id: FxHashMap<i64, UnitId>,
    info_by_player_id: FxHashMap<PlayerId, PlayerInfo>,
    id_by_ucid: FxHashMap<Ucid, PlayerId>,
    recently_landed: FxHashMap<SlotId, (String, DateTime<Utc>)>,
    force_to_spectators: FxHashSet<PlayerId>,
}

static mut CONTEXT: Option<Context> = None;

impl Context {
    // this must be used cautiously. Reasons why it's not totally nuts,
    // - the dcs scripting api is single threaded
    // - the event handlers can be triggerred by api calls, making refcells and mutexes error prone
    // - as long as an event handler doesn't step on state in an api call it's ok, since concurrency never happens
    //   that isn't so hard to guarantee
    unsafe fn get_mut() -> &'static mut Context {
        match CONTEXT.as_mut() {
            Some(ctx) => ctx,
            None => {
                CONTEXT = Some(Context::default());
                CONTEXT.as_mut().unwrap()
            }
        }
    }

    unsafe fn _get() -> &'static Context {
        Context::get_mut()
    }

    fn do_bg_task(&mut self, task: bg::Task) {
        if let Some(to_bg) = &self.to_background {
            match to_bg.send(task) {
                Ok(()) => (),
                Err(_) => panic!("background thread is dead"),
            }
        }
    }

    fn init_async_bg(&mut self, lua: &Lua) -> Result<()> {
        if self.to_background.is_none() {
            let write_dir = PathBuf::from(Lfs::singleton(lua)?.writedir()?.as_str());
            self.to_background = Some(bg::init(write_dir));
        }
        Ok(())
    }

    fn respawn_groups(&mut self, lua: MizLua) -> Result<()> {
        let spctx = SpawnCtx::new(lua)?;
        for (_, group) in self.db.groups() {
            self.db.respawn_group(&self.idx, &spctx, group)?
        }
        Ok(())
    }
}

fn get_player_info<'a, 'lua, L: LuaEnv<'lua>>(
    tbl: &'a mut FxHashMap<PlayerId, PlayerInfo>,
    rtbl: &'a mut FxHashMap<Ucid, PlayerId>,
    lua: L,
    id: PlayerId,
) -> Result<&'a PlayerInfo> {
    if tbl.contains_key(&id) {
        Ok(&tbl[&id])
    } else {
        let net = Net::singleton(lua)?;
        let ifo = net.get_player_info(id)?;
        let ucid = ifo
            .ucid()?
            .ok_or_else(|| anyhow!("player {:?} has no ucid", ifo))?;
        let name = ifo.name()?;
        rtbl.insert(ucid.clone(), id);
        tbl.insert(id, PlayerInfo { name, ucid });
        Ok(&tbl[&id])
    }
}

fn on_player_try_connect(
    _: HooksLua,
    addr: String,
    name: String,
    ucid: Ucid,
    id: PlayerId,
) -> Result<bool> {
    info!(
        "onPlayerTryConnect addr: {:?}, name: {:?}, ucid: {:?}, id: {:?}",
        addr, name, ucid, id
    );
    let ctx = unsafe { Context::get_mut() };
    ctx.id_by_ucid.insert(ucid.clone(), id);
    ctx.info_by_player_id.insert(id, PlayerInfo { name, ucid });
    Ok(true)
}

fn register_player(lua: HooksLua, id: PlayerId, msg: String) -> Result<String> {
    let net = Net::singleton(lua)?;
    let ctx = unsafe { Context::get_mut() };
    let ifo = get_player_info(&mut ctx.info_by_player_id, &mut ctx.id_by_ucid, lua, id)?;
    let side = if msg.eq_ignore_ascii_case("blue") {
        Side::Blue
    } else if msg.eq_ignore_ascii_case("red") {
        Side::Red
    } else {
        bail!("side \"{msg}\" is not blue or red")
    };
    match ctx
        .db
        .register_player(ifo.ucid.clone(), ifo.name.clone(), side)
    {
        Ok(()) => {
            let msg = String::from(format_compact!("Welcome to the {:?} team. You may only occupy slots belonging to your team. Good luck!", side));
            net.send_chat_to(msg, id, None)?;
            net.send_chat(
                String::from(format_compact!("{} has joined {:?} team", ifo.name, side)),
                true,
            )?
        }
        Err((side_switches, orig_side)) => {
            let msg = String::from(match side_switches {
                None => format_compact!("You are already on the {:?} team. You may switch sides by typing -switch {:?}.", orig_side, side),
                Some(0) => format_compact!("You are already on {:?} team, and you may not switch sides.", orig_side),
                Some(1) => format_compact!("You are already on {:?} team. You may sitch sides 1 time by typing -switch {:?}.", orig_side, side),
                Some(n) => format_compact!("You are already on {:?} team. You may switch sides {n} times. Type -switch {:?}.", orig_side, side),
            });
            net.send_chat_to(msg, id, None)?
        }
    }
    Ok(String::from(""))
}

fn sideswitch_player(lua: HooksLua, id: PlayerId, msg: String) -> Result<String> {
    let net = Net::singleton(lua)?;
    let ctx = unsafe { Context::get_mut() };
    let ifo = get_player_info(&mut ctx.info_by_player_id, &mut ctx.id_by_ucid, lua, id)?;
    let side = if msg.eq_ignore_ascii_case("-switch blue") {
        Side::Blue
    } else if msg.eq_ignore_ascii_case("-switch red") {
        Side::Red
    } else {
        bail!("side must be blue or red \"{msg}\"");
    };
    match ctx.db.sideswitch_player(&ifo.ucid, side) {
        Ok(()) => {
            let msg = String::from(format_compact!("{} has switched to {:?}", ifo.name, side));
            net.send_chat(msg, true)?
        }
        Err(e) => net.send_chat_to(String::from(e), id, None)?,
    }
    Ok(String::from(""))
}

fn on_player_try_send_chat(lua: HooksLua, id: PlayerId, msg: String, all: bool) -> Result<String> {
    info!(
        "onPlayerTrySendChat id: {:?}, msg: {:?}, all: {:?}",
        id, msg, all
    );
    if msg.eq_ignore_ascii_case("blue") || msg.eq_ignore_ascii_case("red") {
        register_player(lua, id, msg)
    } else if msg.eq_ignore_ascii_case("-switch blue") || msg.eq_ignore_ascii_case("-switch red") {
        sideswitch_player(lua, id, msg)
    } else if msg.eq_ignore_ascii_case("-lives") {
        let ctx = unsafe { Context::get_mut() };
        match ctx.info_by_player_id.get(&id) {
            Some(ifo) => match lives(&ctx.db, &ifo.ucid) {
                Ok(msg) => Ok(msg.into()),
                Err(e) => {
                    error!("getting player lives {:?} {:?}", ifo, e);
                    Ok("".into())
                }
            }
            None => {
                error!("no player {:?}", id);
                Ok("".into())
            }
        }
    } else {
        Ok(msg)
    }
}

fn try_occupy_slot(lua: HooksLua, net: &Net, id: PlayerId) -> Result<bool> {
    let now = Utc::now();
    let ctx = unsafe { Context::get_mut() };
    let (side, slot) = net.get_slot(id)?;
    let ifo = get_player_info(&mut ctx.info_by_player_id, &mut ctx.id_by_ucid, lua, id)?;
    match ctx.db.try_occupy_slot(now, side, slot, &ifo.ucid) {
        SlotAuth::NoLives => {
            info!("player {}{:?} has no lives", ifo.name, ifo.ucid);
            Ok(false)
        }
        SlotAuth::NotRegistered(side) => {
            info!("player {}{:?} isn't registered", ifo.name, ifo.ucid);
            let msg = String::from(format_compact!(
                "You must join {:?} to use this slot. Type {:?} in chat.",
                side,
                side
            ));
            Net::singleton(lua)?.send_chat_to(msg, id, None)?;
            Ok(false)
        }
        SlotAuth::ObjectiveNotOwned => {
            info!(
                "player {}{:?} coalition does not own the objective",
                ifo.name, ifo.ucid
            );
            let msg = String::from(format_compact!(
                "{:?} does not own the objective associated with this slot",
                side
            ));
            Net::singleton(lua)?.send_chat_to(msg, id, None)?;
            Ok(false)
        }
        SlotAuth::Yes => Ok(true),
    }
}

fn on_player_change_slot(lua: HooksLua, id: PlayerId) -> Result<()> {
    info!("onPlayerChangeSlot: {:?}", id);
    let net = Net::singleton(lua)?;
    match try_occupy_slot(lua, &net, id) {
        Err(e) => {
            error!("error checking slot {:?}", e);
            net.force_player_slot(id, Side::Neutral, SlotId::spectator())?
        }
        Ok(false) => net.force_player_slot(id, Side::Neutral, SlotId::spectator())?,
        Ok(true) => (),
    }
    Ok(())
}

fn force_player_in_slot_to_spectators(ctx: &mut Context, slot: &SlotId) {
    if let Some(ucid) = ctx.db.player_in_slot(slot) {
        if let Some(id) = ctx.id_by_ucid.get(ucid) {
            ctx.force_to_spectators.insert(*id);
        }
    }
}

fn on_event(_lua: MizLua, ev: Event) -> Result<()> {
    info!("onEvent: {:?}", ev);
    let ctx = unsafe { Context::get_mut() };
    match ev {
        Event::Birth(b) => {
            if let Ok(unit) = b.initiator.as_unit() {
                let name = unit.as_object()?.get_name()?;
                if let Ok(su) = ctx.db.unit_by_name(name.as_str()) {
                    let uid = su.id;
                    let oid: i64 = unit.get_object_id()?;
                    ctx.units_by_obj_id.insert(oid, uid);
                }
            }
        }
        Event::Dead(e) | Event::UnitLost(e) | Event::PilotDead(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let id = unit.get_object_id()?;
                if let Some(uid) = ctx.units_by_obj_id.remove(&id) {
                    if let Err(e) = ctx.db.unit_dead(uid, true, Utc::now()) {
                        error!("unit dead failed for {:?} {:?}", unit, e);
                    }
                }
                let slot = SlotId::from(unit.id()?);
                ctx.recently_landed.remove(&slot);
                force_player_in_slot_to_spectators(ctx, &slot)
            }
        }
        Event::Ejection(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let slot = SlotId::from(unit.id()?);
                ctx.recently_landed.remove(&slot);
                force_player_in_slot_to_spectators(ctx, &slot)
            }
        }
        Event::Takeoff(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let slot = SlotId::from(unit.id()?);
                let ctx = unsafe { Context::get_mut() };
                ctx.db.takeoff(Utc::now(), slot.clone());
                ctx.recently_landed.remove(&slot);
            }
        }
        Event::Land(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let slot = SlotId::from(unit.id()?);
                let name = unit.as_object()?.get_name()?;
                let ctx = unsafe { Context::get_mut() };
                ctx.recently_landed.insert(slot, (name, Utc::now()));
            }
        }
        _ => (),
    }
    Ok(())
}

fn on_mission_load_end(lua: HooksLua) -> Result<()> {
    info!("on_mission_load_end");
    let miz = env::miz::Miz::singleton(lua)?;
    debug!("indexing mission");
    let ctx = unsafe { Context::get_mut() };
    ctx.idx = miz.index()?;
    ctx.do_bg_task(bg::Task::MizInit);
    debug!("indexed mission");
    Ok(())
}

fn on_simulation_start(_lua: HooksLua) -> Result<()> {
    info!("on_simulation_start");
    Ok(())
}

fn init_hooks(lua: HooksLua) -> Result<()> {
    debug!("setting user hooks");
    UserHooks::new(lua)
        .on_simulation_start(on_simulation_start)?
        .on_mission_load_end(on_mission_load_end)?
        .on_player_change_slot(on_player_change_slot)?
        .on_player_try_connect(on_player_try_connect)?
        .on_player_try_send_chat(on_player_try_send_chat)?
        .register()?;
    debug!("set user hooks");
    Ok(())
}

fn get_unit_ground_pos(lua: MizLua, name: &str) -> Result<Vector2> {
    let pos = Unit::get_by_name(lua, name)?.as_object()?.get_point()?;
    Ok(Vector2::from(na::Vector2::new(pos.0.x, pos.0.z)))
}

fn lives(db: &Db, ucid: &Ucid) -> Result<CompactString> {
    let player = db
        .player(ucid)
        .ok_or_else(|| anyhow!("no such player {:?}", ucid))?;
    let cfg = db.cfg();
    let lives = player.lives();
    let mut msg = CompactString::new("");
    let now = Utc::now();
    for (typ, (n, _)) in &cfg.default_lives {
        match lives.get(typ) {
            None => msg.push_str(&format_compact!("{typ} {n}/{n}\n")),
            Some((reset, cur)) => {
                let reset = now - reset;
                let hrs = reset.num_hours();
                let min = reset.num_minutes() - hrs * 60;
                let sec = reset.num_seconds() - hrs * 3600 - min * 60;
                msg.push_str(&format_compact!(
                    "{typ} {cur}/{n} resetting in {:02}:{:02}:{:02}\n",
                    hrs,
                    min,
                    sec
                ));
            }
        }
    }
    Ok(msg)
}

fn message_life_returned(db: &Db, lua: MizLua, slot: &SlotId) -> Result<()> {
    let uid = slot.as_unit_id().ok_or_else(|| anyhow!("not a unit"))?;
    let ucid = db
        .player_in_slot(slot)
        .ok_or_else(|| anyhow!("no player in slot {:?}", slot))?;
    let mut msg = CompactString::new("life returned\n");
    if let Ok(lives) = lives(db, ucid) {
        msg.push_str(&lives)
    }
    Trigger::singleton(lua)?
        .action()?
        .out_text_for_unit(uid, msg.into(), 10, false)
}

fn return_lives(lua: MizLua, ctx: &mut Context, ts: DateTime<Utc>) {
    let db = &mut ctx.db;
    ctx.recently_landed.retain(|slot, (name, landed_ts)| {
        if ts - *landed_ts >= Duration::seconds(10) {
            let pos = match get_unit_ground_pos(lua, &**name) {
                Ok(pos) => pos,
                Err(_) => return false,
            };
            let life_returned = !db.land(slot.clone(), pos);
            if life_returned {
                if let Err(e) = message_life_returned(db, lua, slot) {
                    error!("failed to send life returned message to {:?} {}", slot, e);
                }
            }
            life_returned
        } else {
            true
        }
    });
}

fn init_miz(lua: MizLua) -> Result<()> {
    info!("init_miz");
    let ctx = unsafe { Context::get_mut() };
    debug!("adding event handler");
    World::singleton(lua)?.add_event_handler(on_event)?;
    let sortie = Miz::singleton(lua)?.sortie()?;
    debug!("sortie is {:?}", sortie);
    let path = match Env::singleton(lua)?.get_value_dict_by_key(sortie)?.as_str() {
        "" => bail!("missing sortie in miz file"),
        s => PathBuf::from(format_compact!("{}\\{}", Lfs::singleton(lua)?.writedir()?, s).as_str()),
    };
    debug!("path to saved state is {:?}", path);
    let timer = Timer::singleton(lua)?;
    timer.schedule_function(timer.get_time()? + 1., mlua::Value::Nil, {
        let path = path.clone();
        move |lua, _, now| {
            let ts = Utc::now();
            let ctx = unsafe { Context::get_mut() };
            if let Err(e) = ctx.db.maybe_do_repairs(lua, &ctx.idx, ts) {
                error!("error doing repairs {:?}", e)
            }
            return_lives(lua, ctx, ts);
            if let Some(snap) = ctx.db.maybe_snapshot() {
                ctx.do_bg_task(bg::Task::SaveState(path.clone(), snap));
            }
            let net = Net::singleton(lua)?;
            for id in ctx.force_to_spectators.drain() {
                net.force_player_slot(id, Side::Neutral, SlotId::spectator())?
            }
            Ok(Some(now + 1.))
        }
    })?;
    debug!("spawning");
    if !path.exists() {
        debug!("saved state doesn't exist, starting from default");
        let cfg = Cfg::load(&path)?;
        ctx.db = Db::init(lua, cfg, &ctx.idx, &Miz::singleton(lua)?)?;
    } else {
        debug!("saved state exists, loading it");
        ctx.db = Db::load(&path)?;
    }
    debug!("spawning units");
    ctx.respawn_groups(lua)?;
    debug!("spawned");
    menu::init(&ctx, lua)?;
    Ok(())
}

#[mlua::lua_module]
fn bflib(lua: &Lua) -> LuaResult<LuaTable> {
    unsafe { Context::get_mut() }
        .init_async_bg(lua)
        .map_err(dcso3::lua_err)?;
    dcso3::create_root_module(lua, init_hooks, init_miz)
}
