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

use crate::{
    bg::Task,
    db::{group::DeployKind, Db, SetS},
    msgq::MsgTyp,
    return_lives,
    spawnctx::{SpawnCtx, SpawnLoc},
    Context,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use bfprotocols::{
    cfg::Cfg,
    db::{group::GroupId, objective::ObjectiveId},
    perf::Perf,
    stats::StatKind,
};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{
    coalition::Side,
    degrees_to_radians,
    net::{DcsLuaEnvironment, Net, PlayerId, Ucid},
    object::DcsObject,
    perf::Perf as ApiPerf,
    pointing_towards2,
    trigger::{MarkId, Trigger},
    unit::Unit,
    value_to_json,
    world::World,
    MizLua, String, Vector2,
};
use enumflags2::BitFlags;
use fxhash::FxHashMap;
use log::warn;
use mlua::Value;
use netidx::publisher::Value as NetIdxValue;
use parking_lot::{Condvar, Mutex};
use regex::{Regex, RegexBuilder};
use smallvec::{smallvec, SmallVec};
use std::{
    mem,
    str::FromStr,
    sync::Arc,
    time::{Duration, Instant},
};
use tokio::sync::oneshot;

#[derive(Debug, Clone, Copy)]
pub enum WarehouseKind {
    Objective,
    DCS,
}

impl FromStr for WarehouseKind {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
        match s {
            "objective" => Ok(Self::Objective),
            "dcs" => Ok(Self::DCS),
            x => bail!("unknown warehouse kind {x}"),
        }
    }
}

#[derive(Debug, Clone)]
pub enum AdminCommand {
    Help,
    ReduceInventory {
        airbase: String,
        amount: u8,
    },
    TransferSupply {
        from: String,
        to: String,
    },
    LogisticsTickNow,
    LogisticsDeliverNow,
    Repair {
        airbase: String,
    },
    Tim {
        key: String,
        size: usize,
    },
    Spawn {
        key: String,
    },
    SideSwitch {
        side: Side,
        player: String,
    },
    Ban {
        player: String,
        until: Option<DateTime<Utc>>,
    },
    Unban {
        player: String,
    },
    Kick {
        player: String,
    },
    Connected,
    Banned,
    Search {
        expr: Regex,
    },
    LogWarehouse {
        kind: WarehouseKind,
        airbase: String,
    },
    Logdesc,
    ResetLives {
        player: String,
    },
    AddAdmin {
        player: String,
    },
    RemoveAdmin {
        player: String,
    },
    Balance {
        player: String,
    },
    SetPoints {
        amount: i32,
        player: String,
    },
    Delete {
        group: GroupId,
    },
    Deslot {
        player: String,
    },
    Remark {
        objective: String,
    },
    Reset {
        winner: Option<Side>,
    },
    Shutdown,
}

impl AdminCommand {
    pub fn help() -> &'static [&'static str] {
        &[
            "reduce <objective> <percent>: reduce supplies at objective by <percent>",
            "transfer <from-objective> <to-objective>: transfer supplies between two objectives",
            "tick: execute a logistics tick now",
            "deliver: execute a logistics delivery now",
            "repair <airbase>: repair one step at the specified airbase",
            "tim <key> [size]: create explosions of [size] default 3000 at every f10 mark with text <key>",
            "spawn <key>: spawn at f10 mark. <key> <troop|deployable> <side> <heading> <name>",
            "switch <side> <alias|playerid|ucid>: force side switch a player",
            "ban <duration|forever> <alias|playerid|ucid>: kick a player and ban them. e.g. ban 10days D4n",
            "unban <alias|ucid>: unban a player",
            "kick <alias|playerid|ucid>: kick a player",
            "reset-lives <alias|playerid|ucid>",
            "connected: list connected players",
            "banned: list banned players",
            "search <regex>: search the player list by regular expression",
            "log-warehouse <objective|dcs> <airbase>: write the contents of the selected warehouse to the log file",
            "log-desc: write the getDesc of the plane you are currently in to the log file",
            "add-admin <player>: make the specified player a server admin",
            "remove-admin <player>: remove the specified player from the admin list",
            "balance <player>: show <player>'s point balance",
            "set-points <n> <player>: set <player>'s point balance to <n>",
            "delete <groupid>: delete deployed group, now with 100% less mess",
            "deslot <player>: force <player> to spectators",
            "remark <obj>: force refresh the markup on objective",
            "reset [winner]: shutdown the server and reset the campaign state",
            "shutdown: shutdown the server"
        ]
    }
}

impl FromStr for AdminCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        if s.trim() == "help" {
            Ok(Self::Help)
        } else if let Some(s) = s.strip_prefix("reduce ") {
            match s.split_once(" ") {
                None => bail!("reduce <airbase> <amount>"),
                Some((airbase, amount)) => {
                    let amount = amount.parse::<u8>()?;
                    Ok(Self::ReduceInventory {
                        airbase: String::from(airbase),
                        amount,
                    })
                }
            }
        } else if let Some(s) = s.strip_prefix("transfer ") {
            match s.split_once(" ") {
                None => bail!("transfer <from> <to>"),
                Some((from, to)) => Ok(Self::TransferSupply {
                    from: from.into(),
                    to: to.into(),
                }),
            }
        } else if let Some(_) = s.strip_prefix("tick") {
            Ok(Self::LogisticsTickNow)
        } else if let Some(_) = s.strip_prefix("deliver") {
            Ok(Self::LogisticsDeliverNow)
        } else if let Some(s) = s.strip_prefix("repair ") {
            Ok(Self::Repair { airbase: s.into() })
        } else if let Some(s) = s.strip_prefix("tim ") {
            match s.split_once(" ") {
                None => Ok(Self::Tim {
                    key: String::from(s),
                    size: 3000,
                }),
                Some((key, size)) => {
                    let size = size.parse::<usize>()?;
                    Ok(Self::Tim {
                        key: String::from(key),
                        size,
                    })
                }
            }
        } else if let Some(s) = s.strip_prefix("spawn ") {
            Ok(Self::Spawn { key: s.into() })
        } else if let Some(s) = s.strip_prefix("switch ") {
            match s.split_once(" ") {
                None => bail!("switch <side> <player>"),
                Some((side, player)) => {
                    let side = side.parse::<Side>()?;
                    Ok(Self::SideSwitch {
                        side,
                        player: player.into(),
                    })
                }
            }
        } else if let Some(s) = s.strip_prefix("ban ") {
            match s.split_once(" ") {
                None => bail!("ban <duration|forever> <alias|id|ucid>"),
                Some((dur, player)) => {
                    let until = if dur == "forever" {
                        None
                    } else {
                        let dur = humantime::Duration::from_str(dur)?;
                        Some(Utc::now() + chrono::Duration::seconds(dur.as_secs() as i64))
                    };
                    Ok(Self::Ban {
                        player: player.into(),
                        until,
                    })
                }
            }
        } else if let Some(s) = s.strip_prefix("unban ") {
            Ok(Self::Unban { player: s.into() })
        } else if let Some(s) = s.strip_prefix("kick ") {
            Ok(Self::Kick { player: s.into() })
        } else if let Some(_) = s.strip_prefix("connected") {
            Ok(Self::Connected)
        } else if let Some(_) = s.strip_prefix("banned") {
            Ok(Self::Banned)
        } else if let Some(s) = s.strip_prefix("search ") {
            Ok(Self::Search {
                expr: RegexBuilder::new(s).case_insensitive(true).build()?,
            })
        } else if let Some(s) = s.strip_prefix("log-warehouse ") {
            match s.split_once(" ") {
                None => bail!("log-warehouse <objective|dcs> <airbase>"),
                Some((kind, airbase)) => Ok(Self::LogWarehouse {
                    kind: kind.parse()?,
                    airbase: String::from(airbase),
                }),
            }
        } else if let Some(_) = s.strip_prefix("log-desc") {
            Ok(Self::Logdesc)
        } else if let Some(s) = s.strip_prefix("reset-lives ") {
            Ok(Self::ResetLives { player: s.into() })
        } else if let Some(_) = s.strip_prefix("shutdown") {
            Ok(Self::Shutdown)
        } else if let Some(s) = s.strip_prefix("add-admin ") {
            Ok(Self::AddAdmin { player: s.into() })
        } else if let Some(s) = s.strip_prefix("remove-admin ") {
            Ok(Self::RemoveAdmin { player: s.into() })
        } else if let Some(s) = s.strip_prefix("balance ") {
            Ok(Self::Balance { player: s.into() })
        } else if let Some(s) = s.strip_prefix("set-points ") {
            match s.split_once(" ") {
                None => bail!("set-points: <amount> <player>"),
                Some((amount, player)) => Ok(Self::SetPoints {
                    amount: amount.parse::<i32>()?,
                    player: player.into(),
                }),
            }
        } else if let Some(s) = s.strip_prefix("delete ") {
            Ok(Self::Delete { group: s.parse()? })
        } else if let Some(s) = s.strip_prefix("deslot ") {
            Ok(Self::Deslot { player: s.into() })
        } else if let Some(s) = s.strip_prefix("remark ") {
            Ok(Self::Remark {
                objective: s.into(),
            })
        } else if let Some(s) = s.strip_prefix("reset") {
            let winner = if s == "" {
                None
            } else {
                Some(Side::from_str(s)?)
            };
            Ok(Self::Reset { winner })
        } else {
            bail!("unknown command {s}")
        }
    }
}

fn admin_spawn(ctx: &mut Context, lua: MizLua, id: Option<PlayerId>, key: String) -> Result<()> {
    let mut to_remove: SmallVec<[MarkId; 8]> = smallvec![];
    let act = Trigger::singleton(lua)?.action()?;
    let spctx = SpawnCtx::new(lua)?;
    let key = format_compact!("{} ", key);
    let ucid = match id {
        None => Ucid::default(),
        Some(id) => {
            ctx.connected
                .get(&id)
                .ok_or_else(|| anyhow!("unknown admin"))?
                .ucid
        }
    };
    enum Kind {
        Troop,
        Deployable,
    }
    impl FromStr for Kind {
        type Err = anyhow::Error;

        fn from_str(s: &str) -> std::prelude::v1::Result<Self, Self::Err> {
            match s {
                "troop" => Ok(Kind::Troop),
                "deployable" => Ok(Kind::Deployable),
                s => bail!("invalid kind, expected troop or deployable got {s}"),
            }
        }
    }
    for mk in World::singleton(lua)?
        .get_mark_panels()
        .context("getting marks")?
    {
        let mk = mk?;
        if mk.text.starts_with(key.as_str()) {
            to_remove.push(mk.id);
            let spec = mk.text.as_str().strip_prefix(key.as_str()).unwrap();
            let mut iter = spec.splitn(4, " ");
            let kind = iter
                .next()
                .ok_or_else(|| {
                    anyhow!(
                        "spawn mark '{}' missing kind expected troop or deployable",
                        spec
                    )
                })?
                .parse::<Kind>()?;
            let side = iter
                .next()
                .ok_or_else(|| anyhow!("spawn mark {} missing side", spec))?;
            let side = side.parse::<Side>().with_context(|| {
                format_compact!("error parsing {} as a side in mark {}", side, spec)
            })?;
            let heading = iter
                .next()
                .ok_or_else(|| anyhow!("spawn mark {} missing heading", spec))?;
            let heading = degrees_to_radians(heading.parse::<u32>().with_context(|| {
                format_compact!("error parsing {} as a heading in mark {}", heading, spec)
            })? as f64);
            let name = iter
                .next()
                .ok_or_else(|| anyhow!("spawn mark {} missing name of the thing to spawn", spec))?;
            let pos = Vector2::new(mk.pos.x, mk.pos.z);
            let loc = SpawnLoc::AtPos {
                pos,
                offset_direction: pointing_towards2(heading),
                group_heading: heading,
            };
            match kind {
                Kind::Troop => {
                    let specs = ctx
                        .db
                        .ephemeral
                        .cfg
                        .troops
                        .get(&side)
                        .ok_or_else(|| anyhow!("no troops on {side}"))?;
                    let spec = specs
                        .iter()
                        .find(|tr| tr.name.as_str() == name)
                        .ok_or_else(|| anyhow!("no troop called {name} on {side}"))?
                        .clone();
                    let origin = DeployKind::Troop {
                        player: ucid,
                        moved_by: None,
                        spec: spec.clone(),
                        origin: None,
                    };
                    ctx.db
                        .add_and_queue_group(
                            &spctx,
                            &ctx.idx,
                            side,
                            loc,
                            &spec.template,
                            origin,
                            BitFlags::empty(),
                            None,
                        )
                        .context("adding group")?;
                }
                Kind::Deployable => {
                    let specs = ctx
                        .db
                        .ephemeral
                        .cfg
                        .deployables
                        .get(&side)
                        .ok_or_else(|| anyhow!("no deployables on {side}"))?;
                    let spec = specs
                        .iter()
                        .find(|dp| dp.path.ends_with(&[String::from(name)]))
                        .ok_or_else(|| anyhow!("no deployable called {name} on {side}"))?
                        .clone();
                    match &spec.logistics {
                        Some(parts) => {
                            ctx.db
                                .add_farp(lua, &spctx, &ctx.idx, side, pos, &spec, parts)
                                .context("adding farp")?;
                        }
                        None => {
                            let origin = DeployKind::Deployed {
                                player: ucid,
                                moved_by: None,
                                spec: spec.clone(),
                            };
                            ctx.db
                                .add_and_queue_group(
                                    &spctx,
                                    &ctx.idx,
                                    side,
                                    loc,
                                    &spec.template,
                                    origin,
                                    BitFlags::empty(),
                                    None,
                                )
                                .context("adding group")?;
                        }
                    }
                }
            }
        }
    }
    for id in to_remove {
        act.remove_mark(id).context("removing mark")?;
    }
    Ok(())
}

pub(super) fn get_player_ucid<'a>(ctx: &'a Context, key: &str) -> Result<Ucid> {
    if let Ok(id) = key.parse::<PlayerId>() {
        if let Some(ifo) = ctx.connected.get(&id) {
            return Ok(ifo.ucid.clone());
        }
    }
    if let Ok(ucid) = key.parse::<Ucid>() {
        if ctx.db.player(&ucid).is_some() {
            return Ok(ucid);
        }
    }
    enum Matcher<'a> {
        Re(Regex),
        Exact(&'a str),
    }
    impl<'a> Matcher<'a> {
        fn is_match(&self, candidate: &str) -> bool {
            match self {
                Self::Re(re) => re.is_match(candidate),
                Self::Exact(s) => *s == candidate,
            }
        }
    }
    let expr = match RegexBuilder::new(key).case_insensitive(true).build() {
        Ok(re) => Matcher::Re(re),
        Err(_) => Matcher::Exact(key),
    };
    let mut candidates: SmallVec<[(&Ucid, &String); 32]> = {
        ctx.db
            .persisted
            .players()
            .into_iter()
            .filter(|(_, player)| {
                player
                    .alts
                    .into_iter()
                    .any(|alt| expr.is_match(alt.as_str()))
            })
            .map(|(ucid, player)| (ucid, &player.name))
            .collect()
    };
    if candidates.len() == 1 {
        return Ok(candidates.pop().unwrap().0.clone());
    } else if candidates.len() > 1 {
        bail!("multiple matching candidates {:?}", candidates)
    }
    bail!("no player found for alias, player id, or ucid \"{}\"", key)
}

pub fn get_airbase(db: &Db, name: &str) -> Result<ObjectiveId> {
    for (oid, obj) in db.objectives() {
        if obj.name.as_str() == name {
            return Ok(*oid);
        }
    }
    let re = RegexBuilder::new(name)
        .case_insensitive(true)
        .build()
        .context("building regex")?;
    let mut candidates: SmallVec<[(ObjectiveId, String); 32]> = smallvec![];
    for (oid, obj) in db.objectives() {
        if re.is_match(obj.name.as_str()) {
            candidates.push((*oid, obj.name.clone()));
        }
    }
    if candidates.len() == 0 {
        bail!("no objective name matches {name}")
    } else if candidates.len() == 1 {
        Ok(candidates[0].0)
    } else {
        bail!("multiple objectives match {name}, matches: {candidates:?}")
    }
}

fn admin_sideswitch(ctx: &mut Context, side: Side, name: String) -> Result<()> {
    let ucid = get_player_ucid(ctx, name.as_str())?;
    ctx.db.force_sideswitch_player(&ucid, side)
}

fn with_mut_cfg<F: FnOnce(&mut Cfg) -> Result<()>>(ctx: &mut Context, f: F) -> Result<()> {
    {
        let cfg = Arc::make_mut(&mut ctx.db.ephemeral.cfg);
        f(cfg)?
    }
    let cfg = Arc::clone(&ctx.db.ephemeral.cfg);
    ctx.do_bg_task(Task::SaveConfig(ctx.miz_state_path.clone(), cfg));
    Ok(())
}

fn admin_ban(
    ctx: &mut Context,
    lua: MizLua,
    until: Option<DateTime<Utc>>,
    name: &String,
) -> Result<()> {
    let ucid = get_player_ucid(ctx, name.as_str())?;
    let name = ctx
        .db
        .player(&ucid)
        .map(|p| p.name.clone())
        .unwrap_or_else(|| name.clone());
    with_mut_cfg(ctx, |cfg| {
        cfg.banned.insert(ucid.clone(), (until, name));
        Ok(())
    })?;
    if let Some(id) = ctx.connected.id_by_ucid.get(&ucid) {
        let msg = match until {
            None => format_compact!("you are banned forever"),
            Some(ts) => format_compact!("you are banned until {}", ts),
        };
        Net::singleton(lua)?.kick(*id, msg.into())?;
    }
    Ok(())
}

fn admin_kick(ctx: &mut Context, lua: MizLua, name: &String) -> Result<()> {
    let ucid = get_player_ucid(ctx, name.as_str())?;
    let id = match ctx.connected.id_by_ucid.get(&ucid) {
        None => bail!("no connected player found, is {name} on the server?"),
        Some(id) => *id,
    };
    Net::singleton(lua)?.kick(id, "you have been kicked by an admin".into())
}

// FreeDanielUnjustifiedBan
fn admin_unban(ctx: &mut Context, name: &String) -> Result<()> {
    let ucid = get_player_ucid(ctx, name.as_str())?;
    with_mut_cfg(ctx, |cfg| match cfg.banned.remove(&ucid) {
        None => bail!("was not banned"),
        Some(_) => Ok(()),
    })
}

fn admin_list_banned(ctx: &Context) -> SmallVec<[(Ucid, String, Option<DateTime<Utc>>); 16]> {
    ctx.db
        .ephemeral
        .cfg
        .banned
        .iter()
        .map(|(ucid, (until, name))| (ucid.clone(), name.clone(), *until))
        .collect()
}

fn admin_list_connected(ctx: &Context) -> SmallVec<[(PlayerId, Ucid, String); 64]> {
    ctx.connected
        .info_by_player_id
        .iter()
        .map(|(id, ifo)| (*id, ifo.ucid.clone(), ifo.name.clone()))
        .collect()
}

fn admin_search(
    ctx: &Context,
    expr: Regex,
) -> SmallVec<[(Option<PlayerId>, Ucid, SetS<String>); 64]> {
    ctx.db
        .persisted
        .players()
        .into_iter()
        .filter_map(|(ucid, player)| {
            if player
                .alts
                .into_iter()
                .any(|name| expr.is_match(name.as_str()))
            {
                Some((
                    ctx.connected.id_by_ucid.get(ucid).map(|id| *id),
                    ucid.clone(),
                    player.alts.clone(),
                ))
            } else {
                None
            }
        })
        .collect()
}

fn admin_log_desc(ctx: &Context, lua: MizLua, ucid: &Ucid) -> Result<()> {
    let slot = &ctx
        .db
        .player(ucid)
        .ok_or_else(|| anyhow!("no such player {ucid}"))?
        .current_slot
        .as_ref()
        .ok_or_else(|| anyhow!("player {ucid} isn't in a slot"))?
        .0;
    let id = ctx
        .db
        .ephemeral
        .get_object_id_by_slot(&slot)
        .ok_or_else(|| anyhow!("player {ucid} unit not found"))?;
    let unit = Unit::get_instance(lua, &id).context("getting unit")?;
    let mut tbl = FxHashMap::default();
    let desc = Value::Table(unit.get_desc().context("getting desc")?);
    let desc = value_to_json(&mut tbl, None, &desc);
    let ammo = Value::Table(unit.get_ammo().context("getting ammo")?.into_inner());
    let ammo = value_to_json(&mut tbl, None, &ammo);
    warn!("{desc}\n{ammo}");
    Ok(())
}

fn admin_reset_lives(ctx: &mut Context, player: &String) -> Result<()> {
    let ucid = get_player_ucid(ctx, player)?;
    ctx.db.player_reset_lives(&ucid)
}

pub(super) fn admin_shutdown(
    ctx: &mut Context,
    lua: MizLua,
    reset: Option<Option<Side>>,
) -> Result<()> {
    let wait = Arc::new((Mutex::new(false), Condvar::new()));
    let se = {
        let perf = unsafe { Perf::get_mut() };
        let api_perf = unsafe { ApiPerf::get_mut() };
        StatKind::SessionEnd {
            perf: (*perf.inner).clone(),
            frame: (*perf.frame).clone(),
            api_perf: (*api_perf.0).clone(),
        }
    };
    if let Some(winner) = reset {
        ctx.do_bg_task(Task::ResetState(ctx.miz_state_path.clone()));
        ctx.do_bg_task(Task::Stat(se));
        ctx.do_bg_task(Task::Stat(StatKind::RoundEnd { winner }));
        ctx.do_bg_task(Task::RotateStats);
    } else {
        return_lives(lua, ctx, DateTime::<Utc>::MAX_UTC);
        ctx.do_bg_task(Task::SaveState(
            ctx.miz_state_path.clone(),
            ctx.db.persisted.clone(),
        ));
        ctx.do_bg_task(Task::Stat(se));
    }
    ctx.do_bg_task(Task::Shutdown(Arc::clone(&wait)));
    let start = Instant::now();
    let wait_for = Duration::from_secs(60);
    let &(ref lock, ref cvar) = &*wait;
    let mut synced = lock.lock();
    while !*synced && start.elapsed() < wait_for {
        cvar.wait_for(&mut synced, wait_for - start.elapsed());
    }
    println!("background shutdown complete");
    Net::singleton(lua)?.dostring_in(DcsLuaEnvironment::Server, "DCS.exitProcess()".into())?;
    println!("dcs shutdown initiated");
    Ok(())
}

fn add_admin(ctx: &mut Context, player: &String) -> Result<()> {
    let ucid = get_player_ucid(ctx, player)?;
    let name = ctx
        .db
        .player(&ucid)
        .ok_or_else(|| anyhow!("missing info for admin {ucid}"))?
        .name
        .clone();
    with_mut_cfg(ctx, move |cfg| {
        cfg.admins.insert(ucid, name);
        Ok(())
    })
}

fn remove_admin(ctx: &mut Context, player: &String) -> Result<()> {
    let ucid = get_player_ucid(ctx, player)?;
    with_mut_cfg(ctx, |cfg| {
        cfg.admins.remove(&ucid);
        Ok(())
    })
}

fn balance(ctx: &Context, player: &String) -> Result<i32> {
    let ucid = get_player_ucid(ctx, player)?;
    let player = ctx
        .db
        .player(&ucid)
        .ok_or_else(|| anyhow!("no such player {player}"))?;
    Ok(player.points)
}

fn set_points(ctx: &mut Context, player: &String, amount: i32) -> Result<()> {
    let ucid = get_player_ucid(ctx, player)?;
    let player = ctx
        .db
        .player_mut(&ucid)
        .ok_or_else(|| anyhow!("no such player {player}"))?;
    player.points = amount;
    ctx.db.ephemeral.dirty();
    Ok(())
}

fn delete(ctx: &mut Context, id: &GroupId) -> Result<()> {
    match &ctx.db.group(id)?.origin {
        DeployKind::Objective => bail!("you can't delete objective groups"),
        DeployKind::Crate { .. }
        | DeployKind::Deployed { .. }
        | DeployKind::Troop { .. }
        | DeployKind::Action { .. } => ctx.db.delete_group(id),
    }
}

fn deslot(ctx: &mut Context, player: &String) -> Result<()> {
    let ucid = get_player_ucid(ctx, player)?;
    ctx.db.ephemeral.force_player_to_spectators(&ucid);
    Ok(())
}

fn remark(ctx: &mut Context, objective: &String) -> Result<()> {
    let oid = get_airbase(&ctx.db, objective)?;
    let obj = ctx
        .db
        .persisted
        .objectives
        .get(&oid)
        .ok_or_else(|| anyhow!("no such objective {oid}"))?;
    ctx.db
        .ephemeral
        .create_objective_markup(&ctx.db.persisted, obj);
    Ok(())
}

#[derive(Debug)]
pub(super) enum Caller {
    Player(PlayerId),
    External(oneshot::Sender<NetIdxValue>),
}

pub(super) fn run_admin_commands(ctx: &mut Context, lua: MizLua) -> Result<()> {
    let mut cmds = mem::take(&mut ctx.admin_commands);
    while let Some((cmd, ch)) = ctx.external_admin_commands.pop() {
        cmds.push((Caller::External(ch), cmd));
    }
    for (caller, cmd) in cmds.drain(..) {
        let mut replies: SmallVec<[NetIdxValue; 4]> = smallvec![];
        macro_rules! reply_ok {
            ($($arg:expr),+) => {
                match caller {
                    Caller::Player(id) => {
                        ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), format_compact!($($arg),+))
                    },
                    Caller::External(_) => {
                        replies.push(NetIdxValue::from(format!($($arg),+)));
                    }
                }

            }
        }
        macro_rules! reply_err {
            ($($arg:expr),+) => {
                match caller {
                    Caller::Player(id) => {
                        ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), format_compact!($($arg),+))
                    },
                    Caller::External(_) => {
                        replies.push(NetIdxValue::Error(format!($($arg),+).into()));
                    }
                }

            }
        }
        macro_rules! airbase {
            ($name:expr) => {
                match get_airbase(&ctx.db, $name) {
                    Ok(oid) => oid,
                    Err(e) => {
                        reply_err!("{e:?}");
                        continue;
                    }
                }
            };
        }
        match cmd {
            AdminCommand::Help => (),
            AdminCommand::ReduceInventory { airbase, amount } => {
                match ctx
                    .db
                    .admin_reduce_inventory(lua, airbase!(&airbase), amount)
                {
                    Err(e) => reply_err!("reduce inventory failed: {:?}", e),
                    Ok(()) => reply_ok!("inventory reduced"),
                }
            }
            AdminCommand::TransferSupply { from, to } => {
                let from = airbase!(&from);
                let to = airbase!(&to);
                match ctx.db.transfer_supplies(lua, from, to) {
                    Err(e) => reply_err!("transfer inventory failed {:?}", e),
                    Ok(()) => reply_ok!("transfer complete. disconnect"),
                }
            }
            AdminCommand::LogisticsTickNow => {
                ctx.db.admin_tick_now();
                reply_ok!("tick scheduled")
            }
            AdminCommand::LogisticsDeliverNow => {
                ctx.db.admin_deliver_now();
                reply_ok!("delivery scheduled")
            }
            AdminCommand::Repair { airbase } => {
                match ctx.db.repair_objective(airbase!(&airbase), Utc::now()) {
                    Ok(()) => reply_ok!("repaired {airbase}"),
                    Err(e) => reply_ok!("failed to repair {e:?}"),
                }
            }
            AdminCommand::Tim { key, size } => {
                let mut to_remove: SmallVec<[MarkId; 8]> = smallvec![];
                let act = Trigger::singleton(lua)?.action()?;
                for mk in World::singleton(lua)?
                    .get_mark_panels()
                    .context("getting marks")?
                {
                    let mk = mk?;
                    if mk.text == key {
                        to_remove.push(mk.id);
                        act.explosion(mk.pos, size as f32)
                            .context("making boom beserker!")?;
                    }
                }
                for id in to_remove {
                    ctx.db.ephemeral.msgs().delete_mark(id);
                }
            }
            AdminCommand::Spawn { key } => {
                let id = match &caller {
                    Caller::Player(id) => Some(*id),
                    Caller::External(_) => None,
                };
                if let Err(e) = admin_spawn(ctx, lua, id, key) {
                    reply_ok!("could not spawn {:?}", e)
                }
            }
            AdminCommand::SideSwitch { side, player } => {
                if let Err(e) = admin_sideswitch(ctx, side, player.clone()) {
                    reply_err!("could not sideswitch {:?}", e)
                } else {
                    reply_ok!("{player} sideswitched to {side}")
                }
            }
            AdminCommand::Ban { player, until } => match admin_ban(ctx, lua, until, &player) {
                Ok(()) => reply_ok!("{player} banned until {:?}", until),
                Err(e) => reply_err!("could not ban {player}, {:?}", e),
            },
            AdminCommand::Unban { player } => match admin_unban(ctx, &player) {
                Ok(()) => reply_ok!("{player} unbanned"),
                Err(e) => reply_err!("could not unban {}, {:?}", player, e),
            },
            AdminCommand::Kick { player } => match admin_kick(ctx, lua, &player) {
                Ok(()) => reply_ok!("{player} kicked"),
                Err(e) => reply_err!("could not kick {player}, {:?}", e),
            },
            AdminCommand::Banned => {
                for (ucid, name, until) in admin_list_banned(ctx) {
                    reply_ok!("{ucid} \"{name}\" {:?}", until)
                }
            }
            AdminCommand::Connected => {
                for (pid, ucid, name) in admin_list_connected(ctx) {
                    reply_ok!("{pid} {ucid} {name}")
                }
            }
            AdminCommand::Search { expr } => {
                for (pid, ucid, names) in admin_search(ctx, expr) {
                    match pid {
                        None => reply_err!("{ucid} {:?}", names),
                        Some(pid) => reply_ok!("{pid} {ucid} {:?}", names),
                    }
                }
            }
            AdminCommand::LogWarehouse { kind, airbase } => {
                match ctx.db.admin_log_inventory(lua, kind, airbase!(&airbase)) {
                    Ok(()) => reply_err!("{airbase} inventory logged"),
                    Err(e) => reply_ok!("could not log {airbase} inventory {:?}", e),
                }
            }
            AdminCommand::Logdesc => match &caller {
                Caller::External(_) => reply_err!("external clients can't be in a plane"),
                Caller::Player(id) => match ctx.connected.get(&id) {
                    None => reply_err!("no player {id}"),
                    Some(ifo) => match admin_log_desc(ctx, lua, &ifo.ucid) {
                        Ok(()) => reply_ok!("{} desc logged", ifo.ucid),
                        Err(e) => reply_err!("could not log admin desc {:?}", e),
                    },
                },
            },
            AdminCommand::ResetLives { player } => match admin_reset_lives(ctx, &player) {
                Ok(()) => reply_ok!("{player} lives reset"),
                Err(e) => reply_err!("could not reset {player} lives {:?}", e),
            },
            AdminCommand::Shutdown => match admin_shutdown(ctx, lua, None) {
                Ok(()) => reply_ok!("shutting down"),
                Err(e) => reply_err!("failed to shutdown {:?}", e),
            },
            AdminCommand::AddAdmin { player } => match add_admin(ctx, &player) {
                Ok(()) => reply_ok!("{player} is now an admin"),
                Err(e) => reply_err!("failed to make {player} an admin {e:?}"),
            },
            AdminCommand::RemoveAdmin { player } => match remove_admin(ctx, &player) {
                Ok(()) => reply_ok!("{player} is no longer an admin"),
                Err(e) => reply_err!("failed to remove {player} from the admin list {e:?}"),
            },
            AdminCommand::Balance { player } => match balance(ctx, &player) {
                Ok(b) => reply_ok!("{player}'s balance is {b}"),
                Err(e) => reply_err!("could not get {player}'s balance {e:?}"),
            },
            AdminCommand::SetPoints { amount, player } => match set_points(ctx, &player, amount) {
                Ok(()) => reply_ok!("{player}'s points set to {amount}"),
                Err(e) => reply_err!("could not set {player}'s points {e:?}"),
            },
            AdminCommand::Delete { group } => match delete(ctx, &group) {
                Ok(()) => reply_ok!("{group} deleted"),
                Err(e) => reply_err!("could not delete group {e:?}"),
            },
            AdminCommand::Deslot { player } => match deslot(ctx, &player) {
                Ok(()) => reply_ok!("{player} deslotted"),
                Err(e) => reply_err!("could not deslot {player} {e:?}"),
            },
            AdminCommand::Remark { objective } => match remark(ctx, &objective) {
                Ok(()) => reply_ok!("{objective} remark queued"),
                Err(e) => reply_err!("could not remark {objective} {e:?}"),
            },
            AdminCommand::Reset { winner } => match admin_shutdown(ctx, lua, Some(winner)) {
                Ok(()) => reply_ok!("the state has been reset"),
                Err(e) => reply_err!("the state could not be reset {e:?}"),
            },
        }
        match caller {
            Caller::Player(_) => (),
            Caller::External(ch) => {
                if replies.len() == 1 {
                    let _ = ch.send(replies.pop().unwrap());
                } else {
                    let _ = ch.send(NetIdxValue::from(replies));
                }
            }
        }
    }
    ctx.admin_commands = cmds;
    Ok(())
}
