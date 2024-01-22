use crate::{
    bg::Task,
    db::{group::DeployKind, Set},
    msgq::MsgTyp,
    spawnctx::{SpawnCtx, SpawnLoc},
    Context,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    degrees_to_radians,
    net::{Net, PlayerId, Ucid},
    pointing_towards2,
    trigger::{MarkId, Trigger},
    world::World,
    MizLua, String, Vector2,
};
use log::error;
use regex::{Regex, RegexBuilder};
use smallvec::{smallvec, SmallVec};
use std::{mem, str::FromStr, sync::Arc};

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
}

impl AdminCommand {
    pub fn help() -> &'static [&'static str] {
        &[
            "reduce <objective> <percent>: reduce supplies at objective by <percent>",
            "transfer <from-objective> <to-objective>: transfer supplies between two objectives",
            "tick: execute a logistics tick now",
            "deliver: execute a logistics delivery now",
            "tim <key> [size]: create explosions of [size] default 3000 at every f10 mark with text <key>",
            "spawn <key>: spawn at f10 mark. <key> <troop|deployable> <side> <heading> <name>",
            "switch <side> <alias|playerid|ucid>: force side switch a player",
            "ban <duration|forever> <alias|playerid|ucid>: kick a player and ban them. e.g. ban 10days D4n",
            "unban <alias|ucid>: unban a player",
            "kick <alias|playerid|ucid>: kick a player",
            "connected: list connected players",
            "banned: list banned players",
            "search <regex>: search the player list by regular expression",
        ]
    }
}

impl FromStr for AdminCommand {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self> {
        let s = s
            .strip_prefix("-admin ")
            .ok_or_else(|| anyhow!("not an admin command {s}"))?;
        if s.trim() == "help" {
            Ok(Self::Help)
        } else if s.starts_with("reduce ") {
            let s = s.strip_prefix("reduce ").unwrap();
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
        } else if s.starts_with("transfer ") {
            let s = s.strip_prefix("transfer ").unwrap();
            match s.split_once(" ") {
                None => bail!("transfer <from> <to>"),
                Some((from, to)) => Ok(Self::TransferSupply {
                    from: from.into(),
                    to: to.into(),
                }),
            }
        } else if s.starts_with("tick") {
            Ok(Self::LogisticsTickNow)
        } else if s.starts_with("deliver") {
            Ok(Self::LogisticsDeliverNow)
        } else if s.starts_with("tim ") {
            let s = s.strip_prefix("tim ").unwrap();
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
        } else if s.starts_with("spawn ") {
            let s = s.strip_prefix("spawn ").unwrap();
            Ok(Self::Spawn { key: s.into() })
        } else if s.starts_with("switch ") {
            let s = s.strip_prefix("switch ").unwrap();
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
        } else if s.starts_with("ban ") {
            let s = s.strip_prefix("ban ").unwrap();
            match s.split_once(" ") {
                None => bail!("ban <duration|forever> <alias|id|ucid>"),
                Some((dur, player)) => {
                    let until = if dur == "forever" {
                        None
                    } else {
                        let dur = humantime::Duration::from_str(dur)?;
                        Some(Utc::now() + Duration::seconds(dur.as_secs() as i64))
                    };
                    Ok(Self::Ban {
                        player: player.into(),
                        until,
                    })
                }
            }
        } else if s.starts_with("unban ") {
            let s = s.strip_prefix("unban ").unwrap();
            Ok(Self::Unban { player: s.into() })
        } else if s.starts_with("kick ") {
            let s = s.strip_prefix("kick ").unwrap();
            Ok(Self::Kick { player: s.into() })
        } else if s.starts_with("connected") {
            Ok(Self::Connected)
        } else if s.starts_with("banned") {
            Ok(Self::Banned)
        } else if s.starts_with("search ") {
            let s = s.strip_prefix("search ").unwrap();
            Ok(Self::Search {
                expr: RegexBuilder::new(s).case_insensitive(true).build()?,
            })
        } else {
            bail!("unknown command {s}")
        }
    }
}

fn admin_spawn(ctx: &mut Context, lua: MizLua, id: PlayerId, key: String) -> Result<()> {
    let mut to_remove: SmallVec<[MarkId; 8]> = smallvec![];
    let act = Trigger::singleton(lua)?.action()?;
    let spctx = SpawnCtx::new(lua)?;
    let key = format_compact!("{} ", key);
    let ifo = ctx
        .info_by_player_id
        .get(&id)
        .ok_or_else(|| anyhow!("unknown admin"))?;
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
                offset_direction: pointing_towards2(heading, pos),
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
                        player: ifo.ucid.clone(),
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
                                .add_farp(&spctx, &ctx.idx, side, pos, &spec, parts)
                                .context("adding farp")?;
                        }
                        None => {
                            let origin = DeployKind::Deployed {
                                player: ifo.ucid.clone(),
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

fn get_player_ucid<'a>(ctx: &'a Context, key: &str) -> Result<Ucid> {
    if let Ok(id) = key.parse::<PlayerId>() {
        if let Some(ifo) = ctx.info_by_player_id.get(&id) {
            return Ok(ifo.ucid.clone());
        }
    }
    let ucid = Ucid::from(key);
    if ctx.db.player(&ucid).is_some() {
        return Ok(ucid);
    }
    if let Some(id) = ctx.id_by_name.get(key) {
        if let Some(ifo) = ctx.info_by_player_id.get(&id) {
            return Ok(ifo.ucid.clone());
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
        let connected: SmallVec<[(&Ucid, &String); 32]> = ctx
            .id_by_name
            .iter()
            .filter(|(name, _)| expr.is_match(name.as_str()))
            .filter_map(|(name, id)| ctx.info_by_player_id.get(id).map(|ifo| (&ifo.ucid, name)))
            .collect();
        if connected.len() > 0 {
            connected
        } else {
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
        }
    };
    if candidates.len() == 1 {
        return Ok(candidates.pop().unwrap().0.clone());
    } else if candidates.len() > 1 {
        bail!("multiple matching candidates {:?}", candidates)
    }
    bail!("no player found for alias, player id, or ucid \"{}\"", key)
}

fn admin_sideswitch(ctx: &mut Context, side: Side, name: String) -> Result<()> {
    let ucid = get_player_ucid(ctx, name.as_str())?;
    ctx.db.force_sideswitch_player(&ucid, side)
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
        .unwrap_or_else(|| name.clone())
        .clone();
    Arc::make_mut(&mut ctx.db.ephemeral.cfg)
        .banned
        .insert(ucid.clone(), (until, name));
    let cfg = Arc::clone(&ctx.db.ephemeral.cfg);
    ctx.do_bg_task(Task::SaveConfig(ctx.miz_state_path.clone(), cfg));
    if let Some(id) = ctx.id_by_ucid.get(&ucid) {
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
    let id = match ctx.id_by_ucid.get(&ucid) {
        None => bail!("no connected player found, is {name} on the server?"),
        Some(id) => *id,
    };
    Net::singleton(lua)?.kick(id, "you have been kicked by an admin".into())
}

// FreeDanielUnjustifiedBan
fn admin_unban(ctx: &mut Context, name: &String) -> Result<()> {
    let ucid = get_player_ucid(ctx, name.as_str())?;
    {
        let cfg = Arc::make_mut(&mut ctx.db.ephemeral.cfg);
        match cfg.banned.remove(&ucid) {
            None => bail!("was not banned"),
            Some(_) => (),
        }
    }
    let cfg = Arc::clone(&ctx.db.ephemeral.cfg);
    ctx.do_bg_task(Task::SaveConfig(ctx.miz_state_path.clone(), cfg));
    Ok(())
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
    ctx.info_by_player_id
        .iter()
        .map(|(id, ifo)| (*id, ifo.ucid.clone(), ifo.name.clone()))
        .collect()
}

fn admin_search(
    ctx: &Context,
    expr: Regex,
) -> SmallVec<[(Option<PlayerId>, Ucid, Set<String>); 64]> {
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
                    ctx.id_by_ucid.get(ucid).map(|id| *id),
                    ucid.clone(),
                    player.alts.clone(),
                ))
            } else {
                None
            }
        })
        .collect()
}

pub(super) fn run_admin_commands(ctx: &mut Context, lua: MizLua) -> Result<()> {
    use std::fmt::Write;
    let mut cmds = mem::take(&mut ctx.admin_commands);
    for (id, cmd) in cmds.drain(..) {
        macro_rules! reply {
            ($($arg:expr),+) => {
                ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), format_compact!($($arg),+))
            }
        }
        match cmd {
            AdminCommand::Help => (),
            AdminCommand::ReduceInventory { airbase, amount } => {
                match ctx.db.admin_reduce_inventory(lua, airbase.as_str(), amount) {
                    Err(e) => reply!("reduce inventory failed: {:?}", e),
                    Ok(()) => reply!("inventory reduced"),
                }
            }
            AdminCommand::TransferSupply { from, to } => {
                match ctx.db.admin_transfer_supplies(lua, &from, &to) {
                    Err(e) => reply!("transfer inventory failed {:?}", e),
                    Ok(()) => reply!("transfer complete. disconnect"),
                }
            }
            AdminCommand::LogisticsTickNow => {
                let mut msg = CompactString::new("");
                if let Err(e) = ctx.db.sync_objectives_from_warehouses(lua) {
                    write!(msg, "failed to sync objectives from warehouses {:?} ", e)?
                }
                if let Err(e) = ctx.db.deliver_supplies_from_logistics_hubs() {
                    write!(msg, "failed to deliver supplies from hubs {:?} ", e)?
                }
                if let Err(e) = ctx.db.sync_warehouses_from_objectives(lua) {
                    write!(msg, "failed to sync warehouses from objectives {:?}", e)?
                }
                if msg.is_empty() {
                    reply!("tick complete")
                } else {
                    ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg)
                }
            }
            AdminCommand::LogisticsDeliverNow => {
                let mut msg = CompactString::new("");
                if let Err(e) = ctx.db.sync_objectives_from_warehouses(lua) {
                    write!(msg, "failed to sync objectives from warehouses {:?} ", e)?
                }
                if let Err(e) = ctx.db.deliver_production(lua) {
                    error!("failed to deliver production {:?}", e)
                }
                if let Err(e) = ctx.db.sync_warehouses_from_objectives(lua) {
                    write!(msg, "failed to sync warehouses from objectives {:?}", e)?
                }
                if msg.is_empty() {
                    reply!("deliver complete")
                } else {
                    ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg)
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
                if let Err(e) = admin_spawn(ctx, lua, id, key) {
                    reply!("could not spawn {:?}", e)
                }
            }
            AdminCommand::SideSwitch { side, player } => {
                if let Err(e) = admin_sideswitch(ctx, side, player.clone()) {
                    reply!("could not sideswitch {:?}", e)
                } else {
                    reply!("{player} sideswitched to {side}")
                }
            }
            AdminCommand::Ban { player, until } => match admin_ban(ctx, lua, until, &player) {
                Ok(()) => reply!("{player} banned until {:?}", until),
                Err(e) => reply!("could not ban {player}, {:?}", e),
            },
            AdminCommand::Unban { player } => match admin_unban(ctx, &player) {
                Ok(()) => reply!("{player} unbanned"),
                Err(e) => reply!("could not unban {}, {:?}", player, e),
            },
            AdminCommand::Kick { player } => match admin_kick(ctx, lua, &player) {
                Ok(()) => reply!("{player} kicked"),
                Err(e) => reply!("could not kick {player}, {:?}", e),
            },
            AdminCommand::Banned => {
                for (ucid, name, until) in admin_list_banned(ctx) {
                    reply!("{ucid} \"{name}\" {:?}", until)
                }
            }
            AdminCommand::Connected => {
                for (pid, ucid, name) in admin_list_connected(ctx) {
                    reply!("{pid} {ucid} {name}")
                }
            }
            AdminCommand::Search { expr } => {
                for (pid, ucid, names) in admin_search(ctx, expr) {
                    match pid {
                        None => reply!("{ucid} {:?}", names),
                        Some(pid) => reply!("{pid} {ucid} {:?}", names),
                    }
                }
            }
        }
    }
    ctx.admin_commands = cmds;
    Ok(())
}
