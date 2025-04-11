/*
Copyright 2024 Eric Stokes.

This file is part of bflib.

bflib is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your
option) any later version.

bflib is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
for more details.
*/

mod admin;
mod bg;
mod chatcmd;
mod db;
mod ewr;
mod jtac;
mod landcache;
mod menu;
mod msgq;
mod shots;
mod spawnctx;

extern crate nalgebra as na;
use crate::db::player::SlotAuth;
use admin::{run_admin_commands, AdminCommand, AdminResult};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use bfprotocols::{
    cfg::{Cfg, LifeType},
    db::objective::ObjectiveId,
    perf::{Perf, PerfInner},
    stats::Stat,
};
use bg::Task;
use chatcmd::run_action_commands;
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use crossbeam::queue::SegQueue;
use db::{
    group::BirthRes,
    player::{RegErr, TakeoffRes},
    Db,
};
use dcso3::{
    coalition::Side,
    env::{
        self,
        miz::{Miz, UnitId},
        Env,
    },
    event::Event,
    hooks::UserHooks,
    lfs::Lfs,
    net::{DcsLuaEnvironment, Net, PlayerId, SlotId, Ucid},
    object::{DcsObject, DcsOid},
    perf::record_perf,
    timer::Timer,
    trigger::Trigger,
    unit::{ClassUnit, Unit},
    world::{HandlerId, MarkPanel, World},
    HooksLua, LuaEnv, MizLua, String,
};
use ewr::Ewr;
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use indexmap::IndexSet;
use jtac::{JtId, Jtacs};
use landcache::LandCache;
use log::{debug, error, info, warn};
use mlua::prelude::*;
use msgq::MsgTyp;
use netidx::publisher::Value;
use shots::ShotDb;
use smallvec::{smallvec, SmallVec};
use spawnctx::SpawnCtx;
use std::{path::PathBuf, sync::Arc};
use tokio::sync::{mpsc::UnboundedSender, oneshot};

#[derive(Debug, Clone)]
struct PlayerInfo {
    name: String,
    addr: Option<String>,
    ucid: Ucid,
}

#[derive(Debug, Default)]
struct Connected {
    info_by_player_id: FxHashMap<PlayerId, PlayerInfo>,
    id_by_ucid: FxHashMap<Ucid, PlayerId>,
    id_by_name: FxHashMap<String, PlayerId>,
    id_by_addr: FxHashMap<Option<String>, PlayerId>,
}

impl Connected {
    pub fn len(&self) -> usize {
        self.info_by_player_id.len()
    }

    pub fn get(&self, id: &PlayerId) -> Option<&PlayerInfo> {
        self.info_by_player_id.get(id)
    }

    pub fn get_by_name(&self, name: &str) -> Option<&PlayerInfo> {
        self.id_by_name
            .get(name)
            .and_then(|id| self.info_by_player_id.get(id))
    }

    fn get_or_lookup_player_info<'a, 'lua, L: LuaEnv<'lua>>(
        &'a mut self,
        lua: L,
        id: PlayerId,
    ) -> Result<&'a PlayerInfo> {
        if self.info_by_player_id.contains_key(&id) {
            Ok(&self.info_by_player_id[&id])
        } else {
            let net = Net::singleton(lua)?;
            let ifo = net.get_player_info(id)?;
            let ucid = ifo
                .ucid()?
                .ok_or_else(|| anyhow!("player {:?} has no ucid", ifo))?;
            let name = ifo.name()?;
            let addr = ifo.ip()?;
            info!("player name: '{}', id: {:?}, ucid: {:?}", name, id, ucid);
            self.player_connected(id, PlayerInfo { name, addr, ucid })?;
            Ok(&self.info_by_player_id[&id])
        }
    }

    pub fn player_connected(&mut self, id: PlayerId, ifo: PlayerInfo) -> Result<()> {
        if let Some(id) = self.id_by_ucid.remove(&ifo.ucid) {
            self.player_disconnected(id);
        }
        if self.id_by_name.contains_key(&ifo.name) {
            bail!("your callsign is already taken by another player")
        }
        if self.id_by_addr.contains_key(&ifo.addr) {
            bail!("another player is already connected from your ip address")
        }
        self.id_by_ucid.insert(ifo.ucid, id);
        self.id_by_name.insert(ifo.name.clone(), id);
        self.id_by_addr.insert(ifo.addr.clone(), id);
        self.info_by_player_id.insert(id, ifo);
        Ok(())
    }

    pub fn player_disconnected(&mut self, id: PlayerId) -> Option<PlayerInfo> {
        self.info_by_player_id.remove(&id).map(|ifo| {
            self.id_by_name.remove(&ifo.name);
            self.id_by_ucid.remove(&ifo.ucid);
            self.id_by_addr.remove(&ifo.addr);
            ifo
        })
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct AutoShutdown {
    when: DateTime<Utc>,
    thirty_minute_warning: bool,
    ten_minute_warning: bool,
    five_minute_warning: bool,
    one_minute_warning: bool,
}

impl AutoShutdown {
    fn new(ts: DateTime<Utc>) -> Self {
        let mut t = Self::default();
        t.when = ts;
        t
    }
}

#[derive(Debug, Clone, Copy)]
enum LoadState {
    Init,
    MissionLoaded { time: DateTime<Utc> },
    Running,
}

impl Default for LoadState {
    fn default() -> Self {
        Self::Init
    }
}

impl LoadState {
    fn login_ok(&self) -> Option<String> {
        match self {
            Self::Running => None,
            Self::Init => Some(String::from(
                "The server is not finished loading the mission",
            )),
            Self::MissionLoaded { time } => {
                let remains = (Duration::seconds(62) - (Utc::now() - time)).num_seconds();
                Some(format_compact!("The server is initializing ETA {remains}s").into())
            }
        }
    }

    fn init_ok(&self) -> bool {
        match self {
            Self::Init => false,
            Self::MissionLoaded { time } => Utc::now() - *time > Duration::seconds(1),
            Self::Running => true,
        }
    }

    fn step(&mut self) {
        match self {
            Self::Running | Self::Init => (),
            Self::MissionLoaded { time } => {
                if Utc::now() - *time >= Duration::minutes(1) {
                    *self = Self::Running;
                }
            }
        }
    }
}

#[derive(Debug, Default)]
struct JtacSlotIfo {
    subscribed_objectives: FxHashSet<ObjectiveId>,
    pinned: FxHashSet<JtId>,
}

#[derive(Debug, Default)]
struct Context {
    sortie: String,
    event_handler_id: Option<HandlerId>,
    miz_state_path: PathBuf,
    shutdown: Option<AutoShutdown>,
    last_perf_log: DateTime<Utc>,
    load_state: LoadState,
    idx: env::miz::MizIndex,
    db: Db,
    external_admin_commands: Arc<SegQueue<(AdminCommand, oneshot::Sender<Value>)>>,
    admin_commands: Vec<(admin::Caller, AdminCommand)>,
    action_commands: Vec<(PlayerId, String)>,
    to_background: Option<UnboundedSender<bg::Task>>,
    recently_landed: FxHashMap<DcsOid<ClassUnit>, DateTime<Utc>>,
    airborne: FxHashSet<DcsOid<ClassUnit>>,
    captureable: FxHashMap<ObjectiveId, usize>,
    shots_out: ShotDb,
    menu_init_queue: IndexSet<SlotId, FxBuildHasher>,
    last_frame: Option<DateTime<Utc>>,
    last_slow_timed_events: DateTime<Utc>,
    last_periodic_points: DateTime<Utc>,
    last_unit_position: usize,
    last_player_position: usize,
    subscribed_jtac_menus: FxHashMap<SlotId, JtacSlotIfo>,
    subscribed_action_menus: FxHashSet<SlotId>,
    connected: Connected,
    landcache: LandCache,
    ewr: Ewr,
    jtac: Jtacs,
}

impl Context {
    // this must be used cautiously. Reasons why it's not totally nuts,
    // - the dcs scripting api is single threaded
    // - the event handlers can be triggerred by api calls, making refcells and mutexes error prone
    // - as long as an event handler doesn't step on state in an api call it's ok, since concurrency never happens
    //   that isn't so hard to guarantee
    unsafe fn get_mut() -> &'static mut Self {
        static mut SELF: Option<Context> = None;
        #[allow(static_mut_refs)]
        let t = SELF.as_mut();
        match t {
            Some(ctx) => ctx,
            None => {
                SELF = Some(Context::default());
                #[allow(static_mut_refs)]
                SELF.as_mut().unwrap()
            }
        }
    }

    unsafe fn _get() -> &'static Context {
        Context::get_mut()
    }

    unsafe fn reset() {
        *Self::get_mut() = Self::default();
    }

    fn do_bg_task(&self, task: bg::Task) {
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

    fn respawn_groups(&mut self, lua: MizLua, miz: &Miz) -> Result<()> {
        let spctx = SpawnCtx::new(lua)?;
        let perf = Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner);
        self.db
            .respawn_after_load(perf, &self.idx, miz, &mut self.landcache, &spctx)
    }

    fn log_perf(&mut self, now: DateTime<Utc>) {
        if now - self.last_perf_log > Duration::seconds(60) {
            self.last_perf_log = now;
            self.do_bg_task(bg::Task::LogPerf {
                players: self.connected.len(),
                perf: unsafe { Perf::get_mut() }.clone(),
                api_perf: unsafe { dcso3::perf::Perf::get_mut() }.clone(),
            });
            info!("landcache {}", self.landcache.stats())
        }
    }
}

fn on_player_try_connect(
    _: HooksLua,
    addr: String,
    name: String,
    ucid: Ucid,
    id: PlayerId,
) -> Result<Option<String>> {
    let ts = Utc::now();
    info!(
        "onPlayerTryConnect addr: {:?}, name: {:?}, ucid: {:?}, id: {:?}",
        addr, name, ucid, id
    );
    let ctx = unsafe { Context::get_mut() };
    if let Some(msg) = ctx.load_state.login_ok() {
        return Ok(Some(msg));
    }
    if let Some(filter) = &ctx.db.ephemeral.cfg.name_filter {
        if !filter.check(&name) {
            let msg = format_compact!("name must match {}", filter.as_str());
            return Ok(Some(msg.into()));
        }
    }
    if let Some((until, _)) = ctx.db.ephemeral.cfg.banned.get(&ucid) {
        match until {
            None => return Ok(Some("you are banned forever".into())),
            Some(until) if until >= &Utc::now() => {
                return Ok(Some(
                    format_compact!("you are banned until {}", until).into(),
                ))
            }
            Some(_) => {
                let path = ctx.miz_state_path.clone();
                {
                    let cfg = Arc::make_mut(&mut ctx.db.ephemeral.cfg);
                    cfg.banned.remove(&ucid);
                }
                let cfg = Arc::clone(&ctx.db.ephemeral.cfg);
                ctx.do_bg_task(bg::Task::SaveConfig(path, cfg))
            }
        }
    }
    if let Err(e) = ctx.connected.player_connected(
        id,
        PlayerInfo {
            name: name.clone(),
            addr: Some(addr.clone()),
            ucid,
        },
    ) {
        return Ok(Some(String::from(format_compact!("{e}"))));
    }
    ctx.db.player_connected(ucid, name.clone());
    ctx.do_bg_task(Task::Stat(Stat::Connect {
        id: ucid,
        addr,
        name,
    }));
    record_perf(
        &mut Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner).dcs_hooks,
        ts,
    );
    Ok(None)
}

fn on_player_try_send_chat(lua: HooksLua, id: PlayerId, msg: String, all: bool) -> Result<String> {
    let start_ts = Utc::now();
    let ctx = unsafe { Context::get_mut() };
    let perf = &mut Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner).dcs_hooks;
    info!(
        "onPlayerTrySendChat id: {:?}, msg: {:?}, all: {:?}",
        id, msg, all
    );
    let r = chatcmd::process(ctx, lua, start_ts, id, msg);
    record_perf(perf, start_ts);
    match r {
        Ok(s) => Ok(s),
        Err(e) => {
            ctx.db
                .ephemeral
                .msgs()
                .send(MsgTyp::Chat(Some(id)), format_compact!("{e}"));
            Ok("".into())
        }
    }
}

fn process_slot_rejection(ctx: &mut Context, id: PlayerId, ucid: Ucid, rej: SlotAuth) {
    match rej {
        SlotAuth::Denied => {
            ctx.db.ephemeral.msgs().send(
                MsgTyp::Chat(Some(id)),
                format_compact!("access to slot is denied"),
            );
        }
        SlotAuth::NoPoints {
            vehicle,
            cost,
            balance,
        } => {
            ctx.db.ephemeral.msgs().send(
                MsgTyp::Chat(Some(id)),
                format_compact!("{vehicle} costs {cost}, you have {balance}"),
            );
        }
        SlotAuth::NoLives(typ) => {
            let msg = match lives(&mut ctx.db, &ucid, Some(typ)) {
                Ok(s) => s,
                Err(e) => {
                    error!("failed to get lives for {} {:?}", ucid, e);
                    "".into()
                }
            };
            ctx.db.ephemeral.msgs().send(
                MsgTyp::Chat(Some(id)),
                format_compact!("you have no {:?} lives remaining. {}", typ, msg),
            );
        }
        SlotAuth::VehicleNotAvailable(vehicle) => {
            let msg = format_compact!("Objective does not have any {} in stock", vehicle.0);
            ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
        }
        SlotAuth::ObjectiveHasNoLogistics => {
            let msg = format_compact!("Objective is capturable");
            ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
        }
        SlotAuth::ObjectiveNotOwned(side) => {
            let msg = String::from(format_compact!(
                "{:?} does not own the objective associated with this slot",
                side
            ));
            ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
        }
        SlotAuth::NotRegistered(_) => warn!("unexpected NotRegistered"),
        SlotAuth::Yes(_) => warn!("slot was not rejected!"),
    }
}

fn try_occupy_slot(
    ctx: &mut Context,
    lua: HooksLua,
    id: PlayerId,
    ifo: PlayerInfo,
    side: Side,
    slot: SlotId,
) -> Result<bool> {
    let now = Utc::now();
    match ctx.db.try_occupy_slot(now, side, slot, &ifo.ucid) {
        SlotAuth::NotRegistered(side) => {
            let name = ifo.name.clone();
            match ctx.db.register_player(ifo.ucid, name.clone(), side) {
                Ok(()) => {
                    chatcmd::register_success(ctx, id, name, side);
                    try_occupy_slot(ctx, lua, id, ifo, side, slot)
                }
                Err(RegErr::AlreadyRegistered(_, _)) => {
                    warn!("{:?} try_occupy_slot says NotRegistered but register_player says AlreadyRegistered", ifo.ucid);
                    Ok(false)
                }
                Err(RegErr::AlreadyOn(_)) => {
                    warn!("{:?} try_occupy_slot says NotRegistered but register_player says AlreadyOn", ifo.ucid);
                    Ok(false)
                }
            }
        }
        SlotAuth::Yes(typ) => {
            ctx.db.ephemeral.cancel_force_to_spectators(&ifo.ucid);
            ctx.subscribed_jtac_menus.remove(&slot);
            ctx.do_bg_task(Task::Stat(Stat::Slot {
                id: ifo.ucid,
                slot,
                typ,
            }));
            Ok(true)
        }
        rej => {
            process_slot_rejection(ctx, id, ifo.ucid, rej);
            Ok(false)
        }
    }
}

fn on_player_try_change_slot(
    lua: HooksLua,
    id: PlayerId,
    side: Side,
    slot: SlotId,
) -> Result<Option<bool>> {
    info!("onPlayerTryChangeSlot: {:?} {:?} {:?}", id, side, slot);
    let start_ts = Utc::now();
    let ctx = unsafe { Context::get_mut() };
    let res = match ctx.connected.get_or_lookup_player_info(lua, id) {
        Err(e) => {
            error!("failed to get player info for {:?} {:?}", id, e);
            Ok(Some(false))
        }
        Ok(ifo) => {
            let ifo = ifo.clone();
            match try_occupy_slot(ctx, lua, id, ifo, side, slot.clone()) {
                Err(e) => {
                    error!("error checking slot {:?}", e);
                    Ok(Some(false))
                }
                Ok(false) => Ok(Some(false)),
                Ok(true) => Ok(None),
            }
        }
    };
    record_perf(
        &mut Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner).dcs_hooks,
        start_ts,
    );
    res
}

fn unit_killed(
    lua: MizLua,
    ctx: &mut Context,
    id: DcsOid<ClassUnit>,
    now: DateTime<Utc>,
) -> Result<()> {
    ctx.recently_landed.remove(&id);
    ctx.shots_out.dead(id.clone(), now);
    if let Err(e) = ctx.jtac.unit_dead(lua, &mut ctx.db, &id) {
        error!("jtac unit dead failed for {:?} {:?}", id, e)
    }
    if let Err(e) = ctx.db.unit_dead(&id, Utc::now()) {
        error!("unit dead failed for {:?} {:?}", id, e);
    }
    Ok(())
}

fn on_event(lua: MizLua, ev: Event) -> Result<()> {
    let start_ts = Utc::now();
    let ctx = unsafe { Context::get_mut() };
    let perf = Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner);
    match &ev {
        Event::MarkAdded(e) | Event::MarkChange(e) | Event::MarkRemoved(e)
            if e.initiator.is_none() =>
        {
            ()
        }
        ev => info!("onEvent: {:?}", ev),
    }
    match ev {
        Event::Birth(b) => {
            if let Ok(unit) = b.initiator.as_unit() {
                match ctx.db.unit_born(lua, &unit, &ctx.connected) {
                    Ok(BirthRes::None) => (),
                    Ok(BirthRes::OccupiedSlot(slot)) => {
                        ctx.menu_init_queue.insert(slot);
                    }
                    Ok(BirthRes::DynamicSlotDenied(ucid, rej)) => {
                        if let Some(id) = ctx.connected.id_by_ucid.get(&ucid) {
                            process_slot_rejection(ctx, *id, ucid, rej)
                        }
                    }
                    Err(e) => {
                        error!("unit born failed {:?} {:?}", unit, e);
                    }
                }
            } else if let Ok(st) = b.initiator.as_static() {
                if let Err(e) = ctx.db.static_born(&st) {
                    error!("static born failed {:?} {:?}", st, e);
                }
            }
        }
        Event::PlayerLeaveUnit(e) => {
            if let Some(unit) = e.initiator.and_then(|u| u.as_unit().ok()) {
                let oid = unit.object_id()?;
                if let Some(ucid) = ctx.db.player_in_unit(false, &oid) {
                    if let Some(player) = ctx.db.player(&ucid) {
                        if let Some((_, Some(inst))) = player.current_slot.as_ref() {
                            if inst.landed_at_objective.is_none() {
                                ctx.shots_out.dead(oid, start_ts)
                            }
                        }
                    }
                }
                if let Err(e) = ctx.db.player_left_unit(lua, start_ts, &unit) {
                    error!("player left unit failed {:?} {:?}", unit, e)
                }
            }
        }
        Event::Hit(e) | Event::Kill(e) => {
            if let Some(target) = e.target.as_ref().and_then(|t| t.as_unit().ok()) {
                let dead = target.get_life()? < 1;
                if let Some(shooter) = e.initiator.and_then(|u| u.as_unit().ok()) {
                    if let Err(e) =
                        ctx.shots_out
                            .hit(&ctx.db, start_ts, dead, &target, &shooter, e.weapon_name)
                    {
                        error!("error processing hit event {:?}", e)
                    }
                }
                if dead {
                    if let Err(e) = unit_killed(lua, ctx, target.object_id()?, start_ts) {
                        error!("0 unit killed failed {:?}", e)
                    }
                }
            } else if let Some(target) = e.target.as_ref().and_then(|t| t.as_static().ok()) {
                if target.get_life()? < 1 {
                    if let Err(e) = ctx.db.static_dead(&target.object_id()?, start_ts) {
                        error!("static dead failed {e:?}")
                    }
                }
            }
        }
        Event::Shot(e) => {
            if let Err(e) = ctx.shots_out.shot(&ctx.db, start_ts, e) {
                error!("error processing shot event {:?}", e)
            }
        }
        Event::Dead(e) | Event::UnitLost(e) | Event::PilotDead(e) => {
            if let Some(unit) = e.initiator.as_ref().and_then(|u| u.as_unit().ok()) {
                let id = unit.object_id()?;
                if let Err(e) = unit_killed(lua, ctx, id, start_ts) {
                    error!("1 unit killed failed {:?}", e)
                }
            } else if let Some(st) = e.initiator.as_ref().and_then(|s| s.as_static().ok()) {
                if let Err(e) = ctx.db.static_dead(&st.object_id()?, start_ts) {
                    error!("static killed failed {e:?}")
                }
            }
        }
        Event::Ejection(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let id = unit.object_id()?;
                if let Err(e) = unit_killed(lua, ctx, id, start_ts) {
                    error!("2 unit killed failed {}", e)
                }
            }
        }
        Event::Takeoff(e) | Event::PostponedTakeoff(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let id = unit.object_id()?;
                let slot = unit.slot()?;
                if ctx.airborne.insert(id.clone()) && ctx.recently_landed.remove(&id).is_none() {
                    match ctx.db.takeoff(Utc::now(), slot, &unit) {
                        Err(e) => error!("could not process takeoff, {:?}", e),
                        Ok(TakeoffRes::NoLifeTaken) => (),
                        Ok(TakeoffRes::TookLife(typ)) => {
                            if let Err(e) = message_life(ctx, &slot, Some(typ), "life taken\n") {
                                error!("could not display life taken message {:?}", e)
                            }
                            let _ = menu::cargo::list_cargo_for_slot(lua, ctx, &slot);
                        }
                        Ok(TakeoffRes::OutOfLives | TakeoffRes::OutOfPoints) => {
                            if let Err(e) = unit.destroy() {
                                error!("failed to destroy unit that took off without lives or points {e:?}")
                            }
                        }
                    }
                }
            }
        }
        Event::Land(e) | Event::PostponedLand(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let id = unit.object_id()?;
                if ctx.airborne.remove(&id) {
                    ctx.recently_landed.insert(id, Utc::now());
                }
            }
        }
        Event::MarkAdded(MarkPanel {
            initiator: Some(unit),
            ..
        }) => {
            let oid = unit.object_id()?;
            if let Some(slot) = ctx.db.ephemeral.get_slot_by_object_id(&oid) {
                let slot = *slot;
                if let Some(ucid) = ctx.db.ephemeral.player_in_slot(&slot) {
                    let ucid = *ucid;
                    if ctx.subscribed_action_menus.contains(&slot) {
                        if let Err(e) =
                            menu::action::init_action_menu_for_slot(ctx, lua, &slot, &ucid)
                        {
                            error!("failed to init action menu for {ucid} {slot} {e:?}")
                        }
                    }
                }
            }
        }
        Event::MissionEnd => unsafe {
            Context::reset();
            Perf::reset();
            Context::get_mut().init_async_bg(lua.inner())?;
            return Ok(()); // avoid record perf with a reset perf context
        },
        _ => (),
    }
    record_perf(&mut perf.dcs_events, start_ts);
    Ok(())
}

fn lives(db: &mut Db, ucid: &Ucid, typfilter: Option<LifeType>) -> Result<CompactString> {
    db.maybe_reset_lives(ucid, Utc::now())?;
    let player = db
        .player(ucid)
        .ok_or_else(|| anyhow!("no such player {:?}", ucid))?;
    let cfg = &db.ephemeral.cfg;
    let lives = &player.lives;
    let mut msg = CompactString::new("");
    let now = Utc::now();
    for (typ, (n, reset_after)) in &cfg.default_lives {
        if typfilter.is_none() || Some(*typ) == typfilter {
            match lives.get(typ) {
                None => msg.push_str(&format_compact!("{typ} {n}/{n}\n")),
                Some((reset, cur)) => {
                    let since_reset = now - *reset;
                    let reset = chatcmd::format_duration(
                        Duration::seconds(*reset_after as i64) - since_reset,
                    );
                    msg.push_str(&format_compact!("{typ} {cur}/{n} resetting in {reset}\n"));
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
        .ephemeral
        .player_in_slot(slot)
        .ok_or_else(|| anyhow!("no player in slot {:?}", slot))?
        .clone();
    let mut msg = CompactString::new(msg);
    if let Ok(lives) = lives(&mut ctx.db, &ucid, typ) {
        msg.push_str(&lives)
    }
    ctx.db.ephemeral.msgs().panel_to_unit(10, false, uid, msg);
    Ok(())
}

fn return_lives(lua: MizLua, ctx: &mut Context, ts: DateTime<Utc>) {
    macro_rules! or_false {
        ($e:expr) => {
            match $e {
                Ok(r) => r,
                Err(_) => return false,
            }
        };
    }
    let db = &mut ctx.db;
    let mut returned: SmallVec<[(LifeType, SlotId); 4]> = smallvec![];
    ctx.recently_landed.retain(|id, landed_ts| {
        if ts - *landed_ts >= Duration::seconds(10) {
            let unit = or_false!(Unit::get_instance(lua, id));
            let pos = or_false!(unit.get_ground_position());
            let slot = or_false!(unit.slot());
            if let Some(typ) = db.land(slot.clone(), pos.0, &unit) {
                returned.push((typ, slot));
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
            ctx.db.ephemeral.msgs().panel_to_all(30, false, m);
        }
    }
    ctx.captureable.retain(|oid, _| cur_cap.contains(oid));
    Ok(())
}

fn advise_captured(ctx: &mut Context, lua: MizLua, ts: DateTime<Utc>) -> Result<()> {
    for (side, oid) in ctx.db.check_capture(lua, ts)? {
        let name = ctx.db.objective(&oid)?.name();
        let mcap = format_compact!("our forces have captured {}", name);
        let mlost = format_compact!("we have lost {}", name);
        ctx.db.ephemeral.msgs().panel_to_side(15, false, side, mcap);
        ctx.db
            .ephemeral
            .msgs()
            .panel_to_side(15, false, side.opposite(), mlost);
        ctx.captureable.remove(&oid);
    }
    Ok(())
}

fn generate_ewr_reports(ctx: &mut Context, now: DateTime<Utc>) -> Result<()> {
    use std::fmt::Write;
    let mut msgs: SmallVec<[(UnitId, CompactString); 64]> = smallvec![];
    for (ucid, player, inst) in ctx.db.instanced_players() {
        let uid = match player
            .current_slot
            .as_ref()
            .and_then(|(sl, _)| sl.as_unit_id())
        {
            Some(uid) => uid,
            None => continue,
        };
        let braa_to_chickens = ctx.ewr.where_chicken(now, false, false, ucid, player, inst);
        if !braa_to_chickens.is_empty() {
            let mut report = format_compact!("Bandits BRAA\n");
            write!(report, "{}\n", ewr::HEADER)?;
            for gibbraa in braa_to_chickens {
                write!(report, "{gibbraa}\n")?;
            }
            msgs.push((uid, report));
        }
    }
    for (uid, msg) in msgs {
        ctx.db.ephemeral.msgs().panel_to_unit(10, false, uid, msg)
    }
    Ok(())
}

fn check_auto_shutdown(ctx: &mut Context, lua: MizLua, now: DateTime<Utc>) -> Result<AdminResult> {
    if let Some(asd) = ctx.shutdown.as_mut() {
        if asd.when - now <= Duration::minutes(30) && !asd.thirty_minute_warning {
            asd.thirty_minute_warning = true;
            ctx.db.ephemeral.msgs().panel_to_all(
                60,
                false,
                "The server will restart in 30 minutes",
            );
        }
        if asd.when - now <= Duration::minutes(10) && !asd.ten_minute_warning {
            asd.ten_minute_warning = true;
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_all(60, true, "The server will restart in 10 minutes");
        }
        if asd.when - now <= Duration::minutes(5) && !asd.five_minute_warning {
            asd.five_minute_warning = true;
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_all(60, true, "The server will restart in 5 minutes")
        }
        if asd.when - now <= Duration::minutes(1) && !asd.one_minute_warning {
            asd.one_minute_warning = true;
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_all(60, true, "The server will restart in one minute")
        }
        if now > asd.when {
            return admin::admin_shutdown(ctx, lua, None);
        }
    }
    if let Some(victor) = ctx.db.check_victory(now) {
        return admin::admin_shutdown(ctx, lua, Some(Some(victor)));
    }
    Ok(AdminResult::Continue)
}

fn force_players_to_spectators(ctx: &mut Context, net: &Net, ts: DateTime<Utc>) {
    for (_, ids) in ctx.db.ephemeral.players_to_force_to_spectators(ts) {
        for ucid in ids {
            match ctx.connected.id_by_ucid.get(&ucid) {
                None => warn!("no id for player ucid {:?}", ucid),
                Some(id) => {
                    info!("forcing player {} to spectators", ucid);
                    if let Err(e) = net.force_player_slot(*id, Side::Neutral, SlotId::Spectator) {
                        error!("error forcing player {:?} to spectators {:?}", id, e);
                    }
                    match net.get_slot(*id) {
                        Err(_) => ctx.db.ephemeral.force_player_to_spectators(&ucid),
                        Ok((side, slot)) => {
                            if side != Side::Neutral || !slot.is_spectator() {
                                ctx.db.ephemeral.force_player_to_spectators(&ucid)
                            }
                        }
                    }
                }
            }
        }
    }
}

fn update_jtac_contacts(ctx: &mut Context, lua: MizLua) {
    match ctx
        .jtac
        .update_contacts(lua, &mut ctx.landcache, &mut ctx.db)
    {
        Err(e) => error!("could not update jtac contacts {e}"),
        Ok(dirty_menus) => {
            let mut dirty_slots: SmallVec<[SlotId; 16]> = smallvec![];
            for (side, oids) in dirty_menus {
                for (_, player, _) in ctx.db.instanced_players() {
                    if player.side == side {
                        if let Some((slot, _)) = player.current_slot.as_ref() {
                            let mut dead: SmallVec<[JtId; 4]> = smallvec![];
                            let mut expunge = false;
                            if let Some(subd) = ctx.subscribed_jtac_menus.get_mut(&slot) {
                                let pinned: SmallVec<[ObjectiveId; 16]> = subd
                                    .pinned
                                    .iter()
                                    .filter_map(|jt| match ctx.jtac.get(jt) {
                                        Ok(jt) => Some(jt.location().oid),
                                        Err(_) => {
                                            dead.push(*jt);
                                            None
                                        }
                                    })
                                    .collect();
                                for oid in &oids {
                                    if subd.subscribed_objectives.contains(oid) {
                                        if !dirty_slots.contains(slot) {
                                            dirty_slots.push(*slot);
                                        }
                                    }
                                    if !pinned.contains(oid) {
                                        subd.subscribed_objectives.remove(oid);
                                    }
                                }
                                expunge = subd.subscribed_objectives.is_empty();
                            }
                            if dead.len() > 0 {
                                let dead = dead.drain(..);
                                if let Some(subd) = ctx.subscribed_jtac_menus.get_mut(slot) {
                                    for jtid in dead {
                                        subd.pinned.remove(&jtid);
                                    }
                                }
                            }
                            if expunge {
                                ctx.subscribed_jtac_menus.remove(slot);
                            }
                        }
                    }
                }
            }
            for slot in dirty_slots {
                if let Err(e) = menu::jtac::init_jtac_menu_for_slot(ctx, lua, &slot) {
                    error!("could not init jtac menu for slot {slot}, {e:?}")
                }
            }
        }
    }
}

fn award_periodic_points(ctx: &mut Context, ts: DateTime<Utc>) {
    if let Some(points) = ctx.db.ephemeral.cfg.points.as_ref() {
        let (award, period) = points.periodic_point_gain;
        if award != 0 && period > 0 {
            let elapsed = (ts - ctx.last_periodic_points).num_seconds();
            if elapsed >= period as i64 {
                ctx.last_periodic_points = ts;
                for ifo in ctx.connected.info_by_player_id.values() {
                    ctx.db.adjust_points(&ifo.ucid, award, "periodic award")
                }
            }
        }
    }
}

fn run_slow_timed_events(
    lua: MizLua,
    ctx: &mut Context,
    perf: &mut PerfInner,
    path: &PathBuf,
    ts: DateTime<Utc>,
) -> Result<AdminResult> {
    let freq = Duration::seconds(ctx.db.ephemeral.cfg.slow_timed_events_freq as i64);
    if ts - ctx.last_slow_timed_events >= freq {
        let start_ts = Utc::now();
        ctx.last_slow_timed_events = start_ts;
        match check_auto_shutdown(ctx, lua, ts) {
            Ok(AdminResult::Continue) => (),
            Ok(AdminResult::Shutdown) => return Ok(AdminResult::Shutdown),
            Err(e) => error!("failed to check for auto shutdown {e:?}"),
        }
        for (oid, vh) in ctx.db.ephemeral.warehouses_to_sync() {
            if let Err(e) = ctx.db.sync_vehicle_at_obj(lua, oid, vh.clone()) {
                error!(
                    "failed to sync warehouse at objective {:?} vehicle {:?} {:?}",
                    oid, vh, e
                )
            }
        }
        return_lives(lua, ctx, ts);
        {
            // report kills
            let cfg = Arc::clone(&ctx.db.ephemeral.cfg);
            for dead in ctx.shots_out.bring_out_your_dead(ts) {
                info!("kill {:?}", dead);
                if let Some(points) = cfg.points.as_ref() {
                    ctx.db.award_kill_points(points, &dead)
                }
                ctx.do_bg_task(Task::Stat(Stat::Kill(dead)));
            }
        }
        if let Err(e) = ctx.db.maybe_do_repairs(ts) {
            error!("error doing repairs {:?}", e)
        }
        record_perf(&mut perf.do_repairs, start_ts);
        if let Err(e) = ctx.db.advance_actions(lua, &ctx.idx, &ctx.jtac, start_ts) {
            error!("could not advance actions {e:?}")
        }
        let ts = Utc::now();
        if let Err(e) = ctx.ewr.update_tracks(lua, &mut ctx.landcache, &ctx.db, ts) {
            error!("could not update ewr tracks {e}")
        }
        record_perf(&mut perf.ewr_tracks, ts);
        let ts = Utc::now();
        if let Err(e) = generate_ewr_reports(ctx, ts) {
            error!("could not generate ewr reports {e}")
        }
        record_perf(&mut perf.ewr_reports, ts);
        let ts = Utc::now();
        match ctx
            .db
            .cull_or_respawn_objectives(lua, &mut ctx.landcache, ts)
        {
            Err(e) => error!("could not cull or respawn objectives {e}"),
            Ok((threatened, cleared)) => {
                for oid in threatened {
                    let obj = ctx.db.objective(&oid)?;
                    let owner = obj.owner();
                    let msg = format_compact!("enemies spotted near {}", obj.name());
                    ctx.db.ephemeral.msgs().panel_to_side(10, false, owner, msg)
                }
                for oid in cleared {
                    let obj = ctx.db.objective(&oid)?;
                    let owner = obj.owner();
                    let msg = format_compact!("{} is no longer threatened", obj.name());
                    ctx.db.ephemeral.msgs().panel_to_side(10, false, owner, msg)
                }
            }
        }
        record_perf(&mut perf.unit_culling, ts);
        let ts = Utc::now();
        if let Err(e) = ctx.db.update_objectives_markup() {
            error!("could not remark objectives {e}")
        }
        record_perf(&mut perf.remark_objectives, ts);
        let ts = Utc::now();
        update_jtac_contacts(ctx, lua);
        record_perf(&mut perf.update_jtac_contacts, ts);
        let now = Utc::now();
        if let Some(snap) = ctx.db.maybe_snapshot() {
            ctx.do_bg_task(bg::Task::SaveState(path.clone(), snap));
        }
        record_perf(&mut perf.snapshot, now);
        award_periodic_points(ctx, start_ts);
        record_perf(&mut perf.slow_timed, start_ts);
    }
    Ok(AdminResult::Continue)
}

fn run_timed_events(ctx: &mut Context, lua: MizLua, path: &PathBuf) -> Result<AdminResult> {
    let ts = Utc::now();
    let perf = Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner);
    let net = Net::singleton(lua)?;
    let act = Trigger::singleton(lua)?.action()?;
    force_players_to_spectators(ctx, &net, ts);
    match ctx
        .db
        .update_unit_positions_incremental(lua, ts, ctx.last_unit_position)
    {
        Err(e) => error!("could not update unit positions {e}"),
        Ok((i, dead)) => {
            ctx.last_unit_position = i;
            for id in dead {
                if let Err(e) = unit_killed(lua, ctx, id.clone(), ts) {
                    error!("unit killed failed {:?} {:?}", id, e)
                }
            }
        }
    }
    record_perf(&mut perf.unit_positions, ts);
    let ts = Utc::now();
    match ctx
        .db
        .update_player_positions_incremental(lua, ts, ctx.last_player_position)
    {
        Err(e) => error!("could not update player positions {e}"),
        Ok((i, dead)) => {
            ctx.last_player_position = i;
            for id in dead {
                if let Err(e) = unit_killed(lua, ctx, id.clone(), ts) {
                    error!("unit killed failed {:?} {:?}", id, e)
                }
            }
        }
    }
    record_perf(&mut perf.player_positions, ts);
    match run_slow_timed_events(lua, ctx, perf, path, ts) {
        Ok(AdminResult::Continue) => (),
        Ok(AdminResult::Shutdown) => return Ok(AdminResult::Shutdown),
        Err(e) => error!("error running slow timed events {:?}", e),
    }
    if let Some(slot) = ctx.menu_init_queue.shift_remove_index(0) {
        if let Err(e) = menu::init_for_slot(ctx, lua, &slot) {
            error!("could not init menus for slot {:?} {:?}", slot, e)
        }
    }
    let now = Utc::now();
    let spctx = SpawnCtx::new(lua)?;
    if let Err(e) =
        ctx.db
            .ephemeral
            .process_spawn_queue(perf, &ctx.db.persisted, ts, &ctx.idx, &spctx)
    {
        error!("error processing spawn queue {:?}", e)
    }
    record_perf(&mut perf.spawn_queue, now);
    let now = Utc::now();
    if let Err(e) = advise_captured(ctx, lua, ts) {
        error!("error advise captured {:?}", e)
    }
    record_perf(&mut perf.advise_captured, now);
    let now = Utc::now();
    if let Err(e) = advise_captureable(ctx) {
        error!("error advise capturable {:?}", e)
    }
    record_perf(&mut perf.advise_capturable, now);
    let now = Utc::now();
    match ctx.jtac.update_target_positions(lua, now, &mut ctx.db) {
        Err(e) => error!("error updating jtac target positions {:?}", e),
        Ok(dead) => {
            for id in dead {
                if let Err(e) = unit_killed(lua, ctx, id.clone(), now) {
                    error!("unit killed failed {:?} {:?}", id, e)
                }
            }
        }
    }
    record_perf(&mut perf.jtac_target_positions, now);
    let now = Utc::now();
    let max_rate = ctx.db.ephemeral.cfg.max_msgs_per_second;
    ctx.db.ephemeral.msgs().process(max_rate, &net, &act);
    record_perf(&mut perf.process_messages, now);
    if let Err(e) = ctx.db.logistics_step(lua, perf, ts) {
        error!("error running logistics events {e:?}")
    }
    match run_admin_commands(ctx, lua) {
        Err(e) => error!("failed to run admin commands {e:?}"),
        Ok(AdminResult::Continue) => (),
        Ok(AdminResult::Shutdown) => return Ok(AdminResult::Shutdown),
    }
    if let Err(e) = run_action_commands(ctx, perf, lua) {
        error!("failed to run action commands {e:?}")
    }
    ctx.load_state.step();
    record_perf(&mut perf.timed_events, ts);
    ctx.log_perf(now);
    Ok(AdminResult::Continue)
}

fn start_timed_events(ctx: &mut Context, lua: MizLua, path: PathBuf) -> Result<()> {
    ctx.last_slow_timed_events = Utc::now();
    let timer = Timer::singleton(lua)?;
    timer.schedule_function(timer.get_time()? + 1., mlua::Value::Nil, {
        let path = path.clone();
        move |lua, _, now| {
            let ctx = unsafe { Context::get_mut() };
            match run_timed_events(ctx, lua, &path) {
                Ok(AdminResult::Continue) => (),
                Err(e) => error!("failed to run timed events {:?}", e),
                Ok(AdminResult::Shutdown) => {
                    println!("initiating DCS shutdown");
                    if let Some(id) = ctx.event_handler_id.take() {
                        World::singleton(lua)?
                            .remove_event_handler(id)
                            .context("removing event handler")?
                    }
                    Net::singleton(lua)?.dostring_in(
                        DcsLuaEnvironment::Server,
                        "DCS.setUserCallbacks({}); DCS.exitProcess()".into(),
                    )?;
                    println!("removing timer event");
                    return Ok(None);
                }
            }
            Ok(Some(now + 1.))
        }
    })?;
    Ok(())
}

fn delayed_init_miz(lua: MizLua) -> Result<()> {
    info!("init_miz: welcome to blue flag v3");
    let ctx = unsafe { Context::get_mut() };
    info!("indexing the miz");
    let miz = Miz::singleton(lua)?;
    ctx.idx = miz.index().context("indexing the mission")?;
    info!("adding event handlers");
    ctx.event_handler_id = Some(
        World::singleton(lua)?
            .add_event_handler(on_event)
            .context("adding event handlers")?,
    );
    let sortie = miz.sortie().context("getting the sortie")?;
    let path = {
        let s = Env::singleton(lua)?.get_value_dict_by_key(sortie)?;
        if s.is_empty() {
            bail!("missing sortie in miz file")
        }
        ctx.sortie = s;
        ctx.miz_state_path =
            PathBuf::from(Lfs::singleton(lua)?.writedir()?.as_str()).join(ctx.sortie.as_str());
        ctx.miz_state_path.clone()
    };
    debug!("sortie is {:?}", ctx.sortie);
    let cfg = Arc::new(Cfg::load(&path)?);
    ctx.do_bg_task(Task::CfgLoaded {
        sortie: ctx.sortie.clone(),
        cfg: Arc::clone(&cfg),
        admin_channel: Arc::clone(&ctx.external_admin_commands),
    });
    debug!("path to saved state is {:?}", path);
    info!("initializing db");
    let to_bg = ctx.to_background.as_ref().unwrap().clone();
    if !path.exists() {
        debug!("saved state doesn't exist, starting from default");
        ctx.do_bg_task(Task::Stat(Stat::NewRound {
            sortie: ctx.sortie.clone(),
        }));
        ctx.db = Db::init(lua, cfg, &ctx.idx, &miz, to_bg).context("initalizing the mission")?;
    } else {
        debug!("saved state exists, loading it");
        ctx.db = Db::load(&miz, &ctx.idx, to_bg, cfg, &path).context("loading the saved state")?;
    }
    ctx.shutdown = ctx
        .db
        .ephemeral
        .cfg
        .shutdown
        .map(|hrs| AutoShutdown::new(Utc::now() + Duration::hours(hrs as i64)));
    ctx.do_bg_task(Task::Stat(Stat::SessionStart {
        stop: ctx.shutdown.map(|a| a.when),
        cfg: Box::new((*ctx.db.ephemeral.cfg).clone()),
    }));
    info!("spawning units");
    ctx.respawn_groups(lua, &miz)
        .context("setting up the mission after load")?;
    info!("starting timed events");
    start_timed_events(ctx, lua, path).context("starting the timed events loop")?;
    Ok(())
}

fn on_mission_load_end(_lua: HooksLua) -> Result<()> {
    unsafe { Context::get_mut().load_state = LoadState::MissionLoaded { time: Utc::now() } };
    info!("mission loaded");
    Ok(())
}

fn on_player_disconnect(_: HooksLua, id: PlayerId) -> Result<()> {
    info!("onPlayerDisconnect({id})");
    let start_ts = Utc::now();
    let ctx = unsafe { Context::get_mut() };
    if let Some(ifo) = ctx.connected.player_disconnected(id) {
        info!("deslotting disconnected player {}", ifo.ucid);
        ctx.db.player_disconnected(&ifo.ucid)
    }
    record_perf(
        &mut Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner).dcs_hooks,
        start_ts,
    );
    Ok(())
}

fn on_simulation_frame(_: HooksLua) -> Result<()> {
    let frame = Arc::make_mut(&mut unsafe { Perf::get_mut() }.frame);
    let now = Utc::now();
    let ctx = unsafe { Context::get_mut() };
    match &mut ctx.last_frame {
        Some(last) => {
            if let Some(ns) = (now - *last).num_nanoseconds() {
                if ns >= 1 && ns <= 1_000_000_000 {
                    **frame += ns as u64;
                }
            }
            *last = now;
        }
        None => {
            ctx.last_frame = Some(now);
        }
    }
    Ok(())
}

fn init_hooks(lua: HooksLua) -> Result<()> {
    info!("setting user hooks");
    UserHooks::new(lua)
        .on_player_try_change_slot(on_player_try_change_slot)?
        .on_mission_load_end(on_mission_load_end)?
        .on_player_try_connect(on_player_try_connect)?
        .on_player_try_send_chat(on_player_try_send_chat)?
        .on_player_disconnect(on_player_disconnect)?
        .on_simulation_frame(on_simulation_frame)?
        .register()?;
    Ok(())
}

fn init_miz(lua: MizLua) -> Result<()> {
    info!("initializing mission");
    let timer = Timer::singleton(lua)?;
    let when = timer.get_time()? + 1.;
    timer.schedule_function(when, mlua::Value::Nil, move |lua, _, now| {
        let ctx = unsafe { Context::get_mut() };
        if ctx.load_state.init_ok() {
            if let Err(e) = delayed_init_miz(lua) {
                error!("THE MISSION CANNOT START: {:?}", e);
                let timer = Timer::singleton(lua)?;
                timer.schedule_function(now + 1., mlua::Value::Nil, move |lua, _, now| {
                    let ctx = unsafe { Context::get_mut() };
                    let _ = Trigger::singleton(lua)?.action()?.out_text(
                        format_compact!("THE MISSION CANNOT START BECAUSE OF AN ERROR\n\n{:?}", e)
                            .into(),
                        3600,
                        true,
                    );
                    ctx.load_state.step();
                    Ok(Some(now + 10.))
                })?;
            }
            Ok(None)
        } else {
            info!("waiting for the mission to finish loading");
            Ok(Some(now + 1.))
        }
    })?;
    Ok(())
}

#[mlua::lua_module]
fn bflib(lua: &Lua) -> LuaResult<LuaTable> {
    unsafe { Context::get_mut() }
        .init_async_bg(lua.inner())
        .map_err(dcso3::lua_err)?;
    dcso3::create_root_module(lua, init_hooks, init_miz)
}
