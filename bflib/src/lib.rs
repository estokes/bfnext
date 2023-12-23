pub mod bg;
pub mod cfg;
pub mod db;
pub mod menu;
pub mod spawnctx;

extern crate nalgebra as na;
use crate::{cfg::Cfg, db::player::SlotAuth};
use anyhow::{anyhow, bail, Result};
use cfg::LifeType;
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use db::{Db, ObjectiveId};
use dcso3::{
    coalition::Side,
    env::{
        self,
        miz::{GroupId, Miz, UnitId},
        Env,
    },
    event::Event,
    hooks::UserHooks,
    lfs::Lfs,
    net::{Net, PlayerId, SlotId, Ucid},
    timer::Timer,
    trigger::{Action, Trigger},
    unit::Unit,
    world::World,
    HooksLua, LuaEnv, MizLua, String, Vector2,
};
use fxhash::{FxHashMap, FxHashSet};
use log::{debug, error, info};
use mlua::prelude::*;
use smallvec::{smallvec, SmallVec};
use spawnctx::SpawnCtx;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
struct PlayerInfo {
    name: String,
    ucid: Ucid,
}

#[derive(Debug, Clone, Copy)]
enum PanelDest {
    All,
    Side(Side),
    Group(GroupId),
    Unit(UnitId),
}

#[derive(Debug, Clone, Copy)]
enum MsgTyp {
    Chat(Option<PlayerId>),
    Panel {
        to: PanelDest,
        display_time: i64,
        clear_view: bool,
    },
}

#[derive(Debug, Clone)]
struct Msg {
    typ: MsgTyp,
    text: String,
}

#[derive(Debug, Clone, Default)]
struct MsgQ(Vec<Msg>);

impl MsgQ {
    fn send<S: Into<String>>(&mut self, typ: MsgTyp, text: S) {
        self.0.push(Msg {
            typ,
            text: text.into(),
        })
    }

    #[allow(dead_code)]
    fn panel_to_all<S: Into<String>>(&mut self, display_time: i64, clear_view: bool, text: S) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::All,
                display_time,
                clear_view,
            },
            text,
        )
    }

    fn panel_to_side<S: Into<String>>(
        &mut self,
        display_time: i64,
        clear_view: bool,
        side: Side,
        text: S,
    ) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::Side(side),
                display_time,
                clear_view,
            },
            text,
        )
    }

    fn panel_to_group<S: Into<String>>(
        &mut self,
        display_time: i64,
        clear_view: bool,
        group: GroupId,
        text: S,
    ) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::Group(group),
                display_time,
                clear_view,
            },
            text,
        )
    }

    fn panel_to_unit<S: Into<String>>(
        &mut self,
        display_time: i64,
        clear_view: bool,
        unit: UnitId,
        text: S,
    ) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::Unit(unit),
                display_time,
                clear_view,
            },
            text,
        )
    }

    fn process(&mut self, net: &Net, act: &Action) {
        for msg in self.0.drain(..) {
            debug!("server sending {:?}", msg);
            let res = match msg.typ {
                MsgTyp::Chat(to) => match to {
                    None => net.send_chat(msg.text, true),
                    Some(id) => net.send_chat_to(msg.text, id, Some(PlayerId::from(1))),
                },
                MsgTyp::Panel {
                    to,
                    display_time,
                    clear_view,
                } => match to {
                    PanelDest::All => act.out_text(msg.text, display_time, clear_view),
                    PanelDest::Group(gid) => {
                        act.out_text_for_group(gid, msg.text, display_time, clear_view)
                    }
                    PanelDest::Side(side) => {
                        act.out_text_for_coalition(side, msg.text, display_time, clear_view)
                    }
                    PanelDest::Unit(uid) => {
                        act.out_text_for_unit(uid, msg.text, display_time, clear_view)
                    }
                },
            };
            if let Err(e) = res {
                error!("could not send message {:?}", e)
            }
        }
    }
}

#[derive(Debug, Default)]
struct Context {
    idx: env::miz::MizIndex,
    db: Db,
    to_background: Option<UnboundedSender<bg::Task>>,
    units_by_obj_id: FxHashMap<i64, db::UnitId>,
    info_by_player_id: FxHashMap<PlayerId, PlayerInfo>,
    id_by_ucid: FxHashMap<Ucid, PlayerId>,
    recently_landed: FxHashMap<SlotId, (String, DateTime<Utc>)>,
    airborne: FxHashSet<SlotId>,
    captureable: FxHashMap<ObjectiveId, usize>,
    force_to_spectators: FxHashSet<PlayerId>,
    pending_messages: MsgQ,
    last_cull: DateTime<Utc>,
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
        self.db.respawn_after_load(&self.idx, &spctx)
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
    ucid: Ucid,
    name: String,
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
            ctx.pending_messages.send(MsgTyp::Chat(Some(id)), msg);
            ctx.pending_messages.send(
                MsgTyp::Chat(None),
                format_compact!("{} has joined {:?} team", ifo.name, side),
            );
        }
        Err((side_switches, orig_side)) => {
            let msg = String::from(match side_switches {
                None => format_compact!("You are already on the {:?} team. You may switch sides by typing -switch {:?}.", orig_side, side),
                Some(0) => format_compact!("You are already on {:?} team, and you may not switch sides.", orig_side),
                Some(1) => format_compact!("You are already on {:?} team. You may sitch sides 1 time by typing -switch {:?}.", orig_side, side),
                Some(n) => format_compact!("You are already on {:?} team. You may switch sides {n} times. Type -switch {:?}.", orig_side, side),
            });
            ctx.pending_messages.send(MsgTyp::Chat(Some(id)), msg);
        }
    }
    Ok(String::from(""))
}

fn sideswitch_player(lua: HooksLua, id: PlayerId, msg: String) -> Result<String> {
    let ctx = unsafe { Context::get_mut() };
    let ifo = get_player_info(&mut ctx.info_by_player_id, &mut ctx.id_by_ucid, lua, id)?;
    let (_, slot) = Net::singleton(lua)?.get_slot(id)?;
    if !slot.is_spectator() {
        bail!("you must be in spectators to switch sides")
    }
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
            ctx.pending_messages.send(MsgTyp::Chat(None), msg);
        }
        Err(e) => ctx.pending_messages.send(MsgTyp::Chat(Some(id)), e),
    }
    Ok(String::from(""))
}

fn lives_command(id: PlayerId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let ifo = ctx
        .info_by_player_id
        .get(&id)
        .ok_or_else(|| anyhow!("missing info for player {:?}", id))?;
    let msg = lives(&mut ctx.db, &ifo.ucid, None)?;
    ctx.pending_messages.send(MsgTyp::Chat(Some(id)), msg);
    Ok(())
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
        if let Err(e) = lives_command(id) {
            error!("lives command failed for player {:?} {:?}", id, e);
        }
        Ok("".into())
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
        SlotAuth::NoLives => Ok(false),
        SlotAuth::NotRegistered(side) => {
            let msg = String::from(format_compact!(
                "You must join {:?} to use this slot. Type {:?} in chat.",
                side,
                side
            ));
            ctx.pending_messages.send(MsgTyp::Chat(Some(id)), msg);
            Ok(false)
        }
        SlotAuth::ObjectiveNotOwned(side) => {
            let msg = String::from(format_compact!(
                "{:?} does not own the objective associated with this slot",
                side
            ));
            ctx.pending_messages.send(MsgTyp::Chat(Some(id)), msg);
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

fn on_event(lua: MizLua, ev: Event) -> Result<()> {
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
        Event::Takeoff(e) | Event::PostponedTakeoff(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let slot = SlotId::from(unit.id()?);
                let ctx = unsafe { Context::get_mut() };
                if ctx.airborne.insert(slot.clone()) && ctx.recently_landed.remove(&slot).is_none()
                {
                    let pos = unit.as_object()?.get_point()?;
                    match ctx
                        .db
                        .takeoff(Utc::now(), slot.clone(), Vector2::new(pos.x, pos.z))
                    {
                        Err(e) => error!("could not process takeoff, {:?}", e),
                        Ok(None) => (),
                        Ok(Some(typ)) => {
                            if let Err(e) = message_life(ctx, &slot, Some(typ), "life taken\n") {
                                error!("could not display life taken message {:?}", e)
                            }
                            let _ = menu::list_cargo_for_slot(lua, ctx, &slot);
                        }
                    }
                }
            }
        }
        Event::Land(e) | Event::PostponedLand(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let slot = SlotId::from(unit.id()?);
                let ctx = unsafe { Context::get_mut() };
                if ctx.airborne.remove(&slot) {
                    let name = unit.as_object()?.get_name()?;
                    ctx.recently_landed.insert(slot, (name, Utc::now()));
                }
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

fn lives(db: &mut Db, ucid: &Ucid, typfilter: Option<LifeType>) -> Result<CompactString> {
    db.maybe_reset_lives(ucid)?;
    let player = db
        .player(ucid)
        .ok_or_else(|| anyhow!("no such player {:?}", ucid))?;
    let cfg = db.cfg();
    let lives = player.lives();
    let mut msg = CompactString::new("");
    let now = Utc::now();
    for (typ, (n, reset_after)) in &cfg.default_lives {
        if typfilter.is_none() || Some(*typ) == typfilter {
            match lives.get(typ) {
                None => msg.push_str(&format_compact!("{typ} {n}/{n}\n")),
                Some((reset, cur)) => {
                    let since_reset = now - *reset;
                    let reset = Duration::seconds(*reset_after as i64) - since_reset;
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
    }
    Ok(msg)
}

fn message_life(ctx: &mut Context, slot: &SlotId, typ: Option<LifeType>, msg: &str) -> Result<()> {
    let uid = slot.as_unit_id().ok_or_else(|| anyhow!("not a unit"))?;
    let ucid = ctx
        .db
        .player_in_slot(slot)
        .ok_or_else(|| anyhow!("no player in slot {:?}", slot))?
        .clone();
    let mut msg = CompactString::new(msg);
    if let Ok(lives) = lives(&mut ctx.db, &ucid, typ) {
        msg.push_str(&lives)
    }
    ctx.pending_messages.panel_to_unit(10, false, uid, msg);
    Ok(())
}

fn return_lives(lua: MizLua, ctx: &mut Context, ts: DateTime<Utc>) {
    let db = &mut ctx.db;
    let mut returned: SmallVec<[(LifeType, SlotId); 4]> = smallvec![];
    ctx.recently_landed.retain(|slot, (name, landed_ts)| {
        if ts - *landed_ts >= Duration::seconds(10) {
            let pos = match get_unit_ground_pos(lua, &**name) {
                Ok(pos) => pos,
                Err(_) => return false,
            };
            if let Some(typ) = db.land(slot.clone(), pos) {
                returned.push((typ, slot.clone()));
                return false;
            }
        }
        true
    });
    for (typ, slot) in returned {
        if let Err(e) = message_life(ctx, &slot, Some(typ), "life returned\n") {
            error!("failed to send life returned message to {:?} {}", slot, e);
        }
    }
}

fn advise_captureable(ctx: &mut Context) -> Result<()> {
    let cur_cap = ctx.db.capturable_objectives();
    for oid in &cur_cap {
        let dur = ctx.captureable.entry(*oid).or_default();
        *dur += 1;
        if *dur == 10 {
            let m = format_compact!("{} is now capturable", ctx.db.objective(oid)?.name());
            ctx.pending_messages.panel_to_all(30, false, m);
        }
    }
    ctx.captureable.retain(|oid, _| cur_cap.contains(oid));
    Ok(())
}

fn advise_captured(ctx: &mut Context, ts: DateTime<Utc>) -> Result<()> {
    for (side, oid) in ctx.db.check_capture(ts)? {
        let name = ctx.db.objective(&oid)?.name();
        let m = format_compact!("our forces have captured {}", name);
        ctx.pending_messages.panel_to_side(15, false, side, m);
        let m = format_compact!("we have lost {}", name);
        ctx.pending_messages
            .panel_to_side(15, false, side.opposite(), m);
        ctx.captureable.remove(&oid);
    }
    Ok(())
}

fn cull_or_spawn_units(lua: MizLua, ctx: &mut Context, ts: DateTime<Utc>) -> Result<()> {
    let cull_freq = Duration::seconds(ctx.db.cfg().unit_cull_freq as i64);
    if ts - ctx.last_cull > cull_freq {
        ctx.last_cull = ts;
        let threatened = ctx.db.cull_or_respawn_objectives(lua, &ctx.idx)?;
        for oid in threatened {
            let obj = ctx.db.objective(&oid)?;
            ctx.pending_messages.panel_to_side(
                10,
                false,
                obj.owner(),
                format_compact!("enemies spotted near {}", obj.name()),
            )
        }
    }
    Ok(())
}

fn run_timed_events(lua: MizLua, path: &PathBuf) -> Result<()> {
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
    let act = Trigger::singleton(lua)?.action()?;
    for id in ctx.force_to_spectators.drain() {
        net.force_player_slot(id, Side::Neutral, SlotId::spectator())?
    }
    cull_or_spawn_units(lua, ctx, ts)?;
    let spctx = SpawnCtx::new(lua)?;
    ctx.db.process_spawn_queue(&ctx.idx, &spctx)?;
    advise_captured(ctx, ts)?;
    advise_captureable(ctx)?;
    ctx.pending_messages.process(&net, &act);
    Ok(())
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
            if let Err(e) = run_timed_events(lua, &path) {
                error!("failed to run timed events {:?}", e)
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
        ctx.db = Db::load(&Miz::singleton(lua)?, &ctx.idx, &path)?;
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
