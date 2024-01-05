pub mod bg;
pub mod cfg;
pub mod db;
pub mod ewr;
pub mod jtac;
pub mod menu;
pub mod msgq;
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
        miz::{Miz, UnitId},
        Env,
    },
    event::Event,
    hooks::UserHooks,
    lfs::Lfs,
    net::{Net, PlayerId, SlotId, Ucid},
    object::{DcsObject, DcsOid},
    timer::Timer,
    trigger::Trigger,
    unit::{ClassUnit, Unit},
    world::World,
    HooksLua, LuaEnv, MizLua, String, Vector2,
};
use ewr::Ewr;
use fxhash::{FxHashMap, FxHashSet};
use hdrhistogram::Histogram;
use jtac::Jtacs;
use log::{debug, error, info};
use mlua::prelude::*;
use msgq::MsgTyp;
use smallvec::{smallvec, SmallVec};
use spawnctx::SpawnCtx;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug)]
struct PlayerInfo {
    name: String,
    ucid: Ucid,
}

#[derive(Debug)]
struct Perf {
    timed_events: Histogram<u64>,
    slow_timed: Histogram<u64>,
    dcs_events: Histogram<u64>,
    dcs_hooks: Histogram<u64>,
}

impl Default for Perf {
    fn default() -> Self {
        Perf {
            timed_events: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            slow_timed: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            dcs_events: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            dcs_hooks: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
        }
    }
}

fn record_perf(h: &mut Histogram<u64>, start_ts: DateTime<Utc>) {
    if let Some(ns) = (Utc::now() - start_ts).num_nanoseconds() {
        if ns >= 1 && ns <= 1_000_000_000 {
            *h += ns as u64;
        }
    }
}

impl Perf {
    fn record_timed(&mut self, start_ts: DateTime<Utc>) {
        record_perf(&mut self.timed_events, start_ts)
    }

    fn record_slow_timed(&mut self, start_ts: DateTime<Utc>) {
        record_perf(&mut self.slow_timed, start_ts)
    }

    fn record_dcs_event(&mut self, start_ts: DateTime<Utc>) {
        record_perf(&mut self.dcs_events, start_ts)
    }

    fn record_dcs_hook(&mut self, start_ts: DateTime<Utc>) {
        record_perf(&mut self.dcs_hooks, start_ts)
    }

    fn log(&self) {
        fn log_histogram(h: &Histogram<u64>, name: &str) {
            let n = h.len();
            let twenty_five = h.value_at_quantile(0.25);
            let fifty = h.value_at_quantile(0.5);
            let ninety = h.value_at_quantile(0.9);
            let ninety_nine = h.value_at_quantile(0.99);
            info!(
                "{name} n: {:>7}, 25th: {:>7}, 50th: {:>7}, 90th: {:>7}, 99th: {:>8}",
                n, twenty_five, fifty, ninety, ninety_nine
            );
        }
        log_histogram(&self.timed_events, "timed events:      ");
        log_histogram(&self.slow_timed, "slow timed events: ");
        log_histogram(&self.dcs_events, "dcs events:        ");
        log_histogram(&self.dcs_hooks, "dcs hooks:         ")
    }
}

#[derive(Debug, Default)]
struct Context {
    perf: Perf,
    loaded: bool,
    idx: env::miz::MizIndex,
    db: Db,
    to_background: Option<UnboundedSender<bg::Task>>,
    info_by_player_id: FxHashMap<PlayerId, PlayerInfo>,
    id_by_ucid: FxHashMap<Ucid, PlayerId>,
    recently_landed: FxHashMap<DcsOid<ClassUnit>, DateTime<Utc>>,
    airborne: FxHashSet<DcsOid<ClassUnit>>,
    captureable: FxHashMap<ObjectiveId, usize>,
    force_to_spectators: FxHashSet<PlayerId>,
    last_slow_timed_events: DateTime<Utc>,
    ewr: Ewr,
    jtac: Jtacs,
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
    name: String,
    ucid: Ucid,
    id: PlayerId,
) -> Result<bool> {
    let start_ts = Utc::now();
    info!(
        "onPlayerTryConnect addr: {:?}, name: {:?}, ucid: {:?}, id: {:?}",
        addr, name, ucid, id
    );
    let ctx = unsafe { Context::get_mut() };
    ctx.id_by_ucid.insert(ucid.clone(), id);
    ctx.info_by_player_id.insert(id, PlayerInfo { name, ucid });
    ctx.perf.record_dcs_hook(start_ts);
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
            ctx.db.msgs().send(MsgTyp::Chat(Some(id)), msg);
            ctx.db.msgs().send(
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
            ctx.db.msgs().send(MsgTyp::Chat(Some(id)), msg);
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
            ctx.db.msgs().send(MsgTyp::Chat(None), msg);
        }
        Err(e) => ctx.db.msgs().send(MsgTyp::Chat(Some(id)), e),
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
    ctx.db.msgs().send(MsgTyp::Chat(Some(id)), msg);
    Ok(())
}

fn on_player_try_send_chat(lua: HooksLua, id: PlayerId, msg: String, all: bool) -> Result<String> {
    let start_ts = Utc::now();
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
        unsafe { Context::get_mut() }.perf.record_dcs_hook(start_ts);
        Ok("".into())
    } else {
        unsafe { Context::get_mut() }.perf.record_dcs_hook(start_ts);
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
        SlotAuth::ObjectiveHasNoLogistics => {
            let msg = format_compact!("Objective is capturable");
            ctx.db.msgs().send(MsgTyp::Chat(Some(id)), msg);
            Ok(false)
        }
        SlotAuth::NotRegistered(side) => {
            let msg = String::from(format_compact!(
                "You must join {:?} to use this slot. Type {:?} in chat.",
                side,
                side
            ));
            ctx.db.msgs().send(MsgTyp::Chat(Some(id)), msg);
            Ok(false)
        }
        SlotAuth::ObjectiveNotOwned(side) => {
            let msg = String::from(format_compact!(
                "{:?} does not own the objective associated with this slot",
                side
            ));
            ctx.db.msgs().send(MsgTyp::Chat(Some(id)), msg);
            Ok(false)
        }
        SlotAuth::Yes => Ok(true),
    }
}

fn on_player_change_slot(lua: HooksLua, id: PlayerId) -> Result<()> {
    let start_ts = Utc::now();
    info!("onPlayerChangeSlot: {:?}", id);
    let ctx = unsafe { Context::get_mut() };
    if let Some(ifo) = ctx.info_by_player_id.get(&id) {
        ctx.db.player_deslot(&ifo.ucid);
    }
    let net = Net::singleton(lua)?;
    match try_occupy_slot(lua, &net, id) {
        Err(e) => {
            error!("error checking slot {:?}", e);
            ctx.force_to_spectators.insert(id);
        }
        Ok(false) => {
            ctx.force_to_spectators.insert(id);
        }
        Ok(true) => (),
    }
    ctx.perf.record_dcs_hook(start_ts);
    Ok(())
}

fn force_player_in_slot_to_spectators(ctx: &mut Context, slot: &SlotId) {
    if let Some(ucid) = ctx.db.player_in_slot(slot) {
        let ucid = ucid.clone();
        ctx.db.player_deslot(&ucid);
        if let Some(id) = ctx.id_by_ucid.get(&ucid) {
            ctx.force_to_spectators.insert(*id);
        }
    }
}

fn on_event(lua: MizLua, ev: Event) -> Result<()> {
    let start_ts = Utc::now();
    info!("onEvent: {:?}", ev);
    let ctx = unsafe { Context::get_mut() };
    match ev {
        Event::Birth(b) => {
            if let Ok(unit) = b.initiator.as_unit() {
                if let Err(e) = ctx.db.unit_born(&unit) {
                    error!("unit born failed {:?} {:?}", unit, e);
                }
            }
        }
        Event::PlayerEnterUnit(e) => {
            if let Some(o) = &e.initiator {
                if let Ok(unit) = o.as_unit() {
                    if let Err(e) = ctx.db.player_entered_unit(&unit) {
                        error!("player enter unit failed {:?} {:?}", unit, e)
                    }
                }
            }
        }
        Event::PlayerLeaveUnit(e) => {
            if let Some(o) = &e.initiator {
                if let Ok(unit) = o.as_unit() {
                    if let Err(e) = ctx.db.player_left_unit(lua, &unit) {
                        error!("player left unit failed {:?} {:?}", unit, e)
                    }
                }
            }
        }
        Event::Dead(e) | Event::UnitLost(e) | Event::PilotDead(e) => {
            if let Some(unit) = e.initiator {
                if let Ok(unit) = unit.as_unit() {
                    force_player_in_slot_to_spectators(ctx, &unit.slot()?);
                    let id = unit.object_id()?;
                    if let Err(e) = ctx.db.unit_dead(&id, Utc::now()) {
                        error!("unit dead failed for {:?} {:?}", unit, e);
                    }
                    ctx.recently_landed.remove(&id);
                }
            }
        }
        Event::Ejection(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                ctx.recently_landed.remove(&unit.object_id()?);
                force_player_in_slot_to_spectators(ctx, &unit.slot()?)
            }
        }
        Event::Takeoff(e) | Event::PostponedTakeoff(e) => {
            if let Ok(unit) = e.initiator.as_unit() {
                let id = unit.object_id()?;
                let slot = unit.slot()?;
                let ctx = unsafe { Context::get_mut() };
                if ctx.airborne.insert(id.clone()) && ctx.recently_landed.remove(&id).is_none() {
                    let pos = unit.get_point()?;
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
                let id = unit.object_id()?;
                let ctx = unsafe { Context::get_mut() };
                if ctx.airborne.remove(&id) {
                    ctx.recently_landed.insert(id, Utc::now());
                }
            }
        }
        Event::MissionEnd => unsafe {
            CONTEXT = None;
            Context::get_mut().init_async_bg(lua.inner())?;
        },
        _ => (),
    }
    ctx.perf.record_dcs_event(start_ts);
    Ok(())
}

fn lives(db: &mut Db, ucid: &Ucid, typfilter: Option<LifeType>) -> Result<CompactString> {
    db.maybe_reset_lives(ucid, Utc::now())?;
    let player = db
        .player(ucid)
        .ok_or_else(|| anyhow!("no such player {:?}", ucid))?;
    let cfg = db.cfg();
    let lives = &player.lives;
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
    ctx.db.msgs().panel_to_unit(10, false, uid, msg);
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
            if let Some(typ) = db.land(slot.clone(), pos.0) {
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
            ctx.db.msgs().panel_to_all(30, false, m);
        }
    }
    ctx.captureable.retain(|oid, _| cur_cap.contains(oid));
    Ok(())
}

fn advise_captured(ctx: &mut Context, ts: DateTime<Utc>) -> Result<()> {
    for (side, oid) in ctx.db.check_capture(ts)? {
        let name = ctx.db.objective(&oid)?.name();
        let mcap = format_compact!("our forces have captured {}", name);
        let mlost = format_compact!("we have lost {}", name);
        ctx.db.msgs().panel_to_side(15, false, side, mcap);
        ctx.db
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
        ctx.db.msgs().panel_to_unit(10, false, uid, msg)
    }
    Ok(())
}

fn run_slow_timed_events(lua: MizLua, ctx: &mut Context, ts: DateTime<Utc>) -> Result<()> {
    let freq = Duration::seconds(ctx.db.cfg().slow_timed_events_freq as i64);
    if ts - ctx.last_slow_timed_events > freq {
        ctx.last_slow_timed_events = ts;
        let start_ts = Utc::now();
        if let Err(e) = ctx.db.update_unit_positions(lua) {
            error!("could not update unit positions {e}")
        }
        if let Err(e) = ctx.db.update_player_positions(lua) {
            error!("could not update player positions {e}")
        }
        if let Err(e) = ctx.ewr.update_tracks(lua, &ctx.db, ts) {
            error!("could not update ewr tracks {e}")
        }
        if let Err(e) = generate_ewr_reports(ctx, ts) {
            error!("could not generate ewr reports {e}")
        }
        match ctx.db.cull_or_respawn_objectives(lua, ts) {
            Err(e) => error!("could not cull or respawn objectives {e}"),
            Ok((threatened, cleared)) => {
                for oid in threatened {
                    let obj = ctx.db.objective(&oid)?;
                    let owner = obj.owner();
                    let msg = format_compact!("enemies spotted near {}", obj.name());
                    ctx.db.msgs().panel_to_side(10, false, owner, msg)
                }
                for oid in cleared {
                    let obj = ctx.db.objective(&oid)?;
                    let owner = obj.owner();
                    let msg = format_compact!("{} is no longer threatened", obj.name());
                    ctx.db.msgs().panel_to_side(10, false, owner, msg)
                }
            }
        }
        if let Err(e) = ctx.db.remark_objectives() {
            error!("could not remark objectives {e}")
        }
        if let Err(e) = ctx.jtac.update_contacts(lua, &mut ctx.db) {
            error!("could not update jtac contacts {e}")
        }
        ctx.perf.record_slow_timed(start_ts);
        ctx.perf.log();
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
    let net = Net::singleton(lua)?;
    let act = Trigger::singleton(lua)?.action()?;
    for id in ctx.force_to_spectators.drain() {
        if let Err(e) = net.force_player_slot(id, Side::Neutral, SlotId::spectator()) {
            error!("error forcing player {:?} to spectators {e}", id);
        }
    }
    if let Err(e) = run_slow_timed_events(lua, ctx, ts) {
        error!("error running slow timed events {e}")
    }
    let spctx = SpawnCtx::new(lua)?;
    if let Err(e) = ctx.db.process_spawn_queue(ts, &ctx.idx, &spctx) {
        error!("error processing spawn queue {e}")
    }
    if let Err(e) = advise_captured(ctx, ts) {
        error!("error advise captured {e}")
    }
    if let Err(e) = advise_captureable(ctx) {
        error!("error advise capturable {e}")
    }
    if let Err(e) = ctx.jtac.update_target_positions(lua, &ctx.db) {
        error!("error updating jtac target positions {e}")
    }
    ctx.db.msgs().process(&net, &act);
    if let Some(snap) = ctx.db.maybe_snapshot() {
        ctx.do_bg_task(bg::Task::SaveState(path.clone(), snap));
    }
    ctx.perf.record_timed(ts);
    Ok(())
}

fn start_timed_events(lua: MizLua, path: PathBuf) -> Result<()> {
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
    Ok(())
}

fn delayed_init_miz(lua: MizLua) -> Result<()> {
    info!("init_miz");
    let ctx = unsafe { Context::get_mut() };
    info!("indexing the miz");
    let miz = Miz::singleton(lua)?;
    ctx.idx = miz.index()?;
    info!("adding event handlers");
    World::singleton(lua)?.add_event_handler(on_event)?;
    let sortie = miz.sortie()?;
    debug!("sortie is {:?}", sortie);
    let path = match Env::singleton(lua)?.get_value_dict_by_key(sortie)?.as_str() {
        "" => bail!("missing sortie in miz file"),
        s => PathBuf::from(format_compact!("{}\\{}", Lfs::singleton(lua)?.writedir()?, s).as_str()),
    };
    debug!("path to saved state is {:?}", path);
    info!("initializing db");
    if !path.exists() {
        debug!("saved state doesn't exist, starting from default");
        let cfg = Cfg::load(&path)?;
        ctx.db = Db::init(lua, cfg, &ctx.idx, &miz)?;
    } else {
        debug!("saved state exists, loading it");
        ctx.db = Db::load(&miz, &ctx.idx, &path)?;
    }
    info!("spawning units");
    ctx.respawn_groups(lua)?;
    info!("initializing menus");
    menu::init(&ctx, lua)?;
    info!("starting timed events");
    start_timed_events(lua, path)?;
    Ok(())
}

fn on_mission_load_end(_lua: HooksLua) -> Result<()> {
    unsafe { Context::get_mut().loaded = true };
    debug!("mission loaded");
    Ok(())
}

fn on_player_disconnect(_: HooksLua, id: PlayerId) -> Result<()> {
    let start_ts = Utc::now();
    let ctx = unsafe { Context::get_mut() };
    if let Some(ifo) = ctx.info_by_player_id.remove(&id) {
        ctx.db.player_deslot(&ifo.ucid)
    }
    ctx.perf.record_dcs_hook(start_ts);
    Ok(())
}

fn init_hooks(lua: HooksLua) -> Result<()> {
    info!("setting user hooks");
    UserHooks::new(lua)
        .on_player_change_slot(on_player_change_slot)?
        .on_mission_load_end(on_mission_load_end)?
        .on_player_try_connect(on_player_try_connect)?
        .on_player_try_send_chat(on_player_try_send_chat)?
        .on_player_disconnect(on_player_disconnect)?
        .register()?;
    Ok(())
}

fn init_miz(lua: MizLua) -> Result<()> {
    let timer = Timer::singleton(lua)?;
    let when = timer.get_time()? + 1.;
    timer.schedule_function(when, mlua::Value::Nil, move |lua, _, now| {
        let ctx = unsafe { Context::get_mut() };
        if ctx.loaded {
            delayed_init_miz(lua)?;
            Ok(None)
        } else {
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
