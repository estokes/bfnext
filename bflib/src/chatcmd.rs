use crate::{
    admin::{self, AdminCommand, Caller},
    bg::Task,
    db::{actions::ActionCmd, group::DeployKind, player::RegErr},
    jtac::JtId,
    lives,
    menu::{self, ArgQuad, ArgTriple, ArgTuple},
    msgq::MsgTyp,
    spawnctx::SpawnCtx,
    Context,
};
use anyhow::{anyhow, bail, Context as ErrContext, Result};
use bfprotocols::{
    cfg::{Action, ActionKind},
    db::group::GroupId,
    perf::PerfInner,
    stats::Stat,
};
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    net::{Net, PlayerId},
    HooksLua, MizLua, String,
};
use fxhash::FxBuildHasher;
use indexmap::IndexMap;
use log::{error, info};
use regex::Regex;
use std::{mem, sync::Arc, sync::OnceLock};

pub(crate) fn register_success(ctx: &mut Context, id: PlayerId, name: String, side: Side) {
    let msg = String::from(format_compact!(
        "Welcome to the {:?} team. You may only occupy slots belonging to your team. Good luck!",
        side
    ));
    ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
    ctx.db.ephemeral.msgs().send(
        MsgTyp::Chat(None),
        format_compact!("{} has joined {:?} team", name, side),
    );
}

pub(crate) fn register_already_on(ctx: &mut Context, id: PlayerId, side: Side) {
    ctx.db.ephemeral.msgs().send(
        MsgTyp::Chat(Some(id)),
        format_compact!("you are already on {:?} team!", side),
    )
}

fn register_player(ctx: &mut Context, lua: HooksLua, id: PlayerId, msg: String) -> Result<String> {
    let ifo = ctx.connected.get_or_lookup_player_info(lua, id)?;
    let name = ifo.name.clone();
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
        Ok(()) => register_success(ctx, id, name, side),
        Err(RegErr::AlreadyOn(side)) => register_already_on(ctx, id, side),
        Err(RegErr::AlreadyRegistered(side_switches, orig_side)) => {
            let msg = String::from(match side_switches {
                None => format_compact!("You are already on the {:?} team. You may switch sides by typing -switch {:?}.", orig_side, side),
                Some(0) => format_compact!("You are already on {:?} team, and you may not switch sides.", orig_side),
                Some(1) => format_compact!("You are already on {:?} team. You may sitch sides 1 time by typing -switch {:?}.", orig_side, side),
                Some(n) => format_compact!("You are already on {:?} team. You may switch sides {n} times. Type -switch {:?}.", orig_side, side),
            });
            ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
        }
    }
    Ok("".into())
}

pub(crate) fn sideswitch_success(ctx: &mut Context, name: String, side: Side) {
    let msg = String::from(format_compact!("{} has switched to {:?}", name, side));
    ctx.db.ephemeral.msgs().send(MsgTyp::Chat(None), msg);
}

fn sideswitch_player(
    ctx: &mut Context,
    lua: HooksLua,
    id: PlayerId,
    msg: String,
) -> Result<String> {
    let ifo = ctx.connected.get_or_lookup_player_info(lua, id)?;
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
            let name = ifo.name.clone();
            sideswitch_success(ctx, name, side);
        }
        Err(e) => ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), e),
    }
    Ok("".into())
}

fn lives_command(ctx: &mut Context, id: PlayerId) -> Result<()> {
    let ifo = ctx
        .connected
        .get(&id)
        .ok_or_else(|| anyhow!("missing info for player {:?}", id))?;
    let msg = lives(&mut ctx.db, &ifo.ucid, None)?;
    ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
    Ok(())
}

fn admin_command(ctx: &mut Context, id: PlayerId, cmd: &str) {
    let ifo = match ctx.connected.get(&id) {
        Some(ifo) => ifo,
        None => return,
    };
    if !ctx.db.ephemeral.cfg.admins.contains_key(&ifo.ucid) {
        return;
    }
    match cmd.parse::<AdminCommand>() {
        Err(e) => ctx.db.ephemeral.msgs().send(
            MsgTyp::Chat(Some(id)),
            format_compact!("parse error {:?}", e),
        ),
        Ok(AdminCommand::Help) => {
            for cmd in AdminCommand::help() {
                ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), *cmd);
            }
        }
        Ok(cmd) => {
            info!("queueing admin command {:?} from {:?}", cmd, ifo);
            ctx.admin_commands.push((Caller::Player(id), cmd))
        }
    }
}

pub(super) fn format_duration(d: Duration) -> CompactString {
    let hrs = d.num_hours();
    let min = d.num_minutes() - hrs * 60;
    let sec = d.num_seconds() - hrs * 3600 - min * 60;
    format_compact!("{:02}:{:02}:{:02}", hrs, min, sec)
}

fn time_command(ctx: &mut Context, id: PlayerId, now: DateTime<Utc>) {
    match ctx.shutdown.as_ref() {
        None => ctx.db.ephemeral.msgs().send(
            MsgTyp::Chat(Some(id)),
            "The server isn't configured to restart automatically",
        ),
        Some(asd) => {
            let remains = format_duration(asd.when - now);
            ctx.db.ephemeral.msgs().send(
                MsgTyp::Chat(Some(id)),
                format_compact!("The server will shutdown in {remains}"),
            )
        }
    }
}

fn balance_command(ctx: &mut Context, id: PlayerId) {
    if let Some(ifo) = ctx.connected.get(&id) {
        if let Some(player) = ctx.db.player(&ifo.ucid) {
            let points = player.points;
            ctx.db.ephemeral.msgs().send(
                MsgTyp::Chat(Some(id)),
                format_compact!("You have {points} points"),
            );
        }
    }
}

fn transfer_command(ctx: &mut Context, id: PlayerId, s: &str) {
    macro_rules! reply {
        ($msg:tt) => {
            ctx.db
                .ephemeral
                .msgs()
                .send(MsgTyp::Chat(Some(id)), format_compact!($msg))
        };
    }
    if let Some(ifo) = ctx.connected.get(&id) {
        match s.split_once(" ") {
            None => reply!("transfer expected amount and player"),
            Some((amount, player)) => match amount.parse::<u32>() {
                Err(e) => reply!("transfer expected a number {e:?}"),
                Ok(amount) => match admin::get_player_ucid(ctx, player) {
                    Err(e) => reply!("could not transfer to {player}, {e:?}"),
                    Ok(ucid) => match ctx.db.transfer_points(&ifo.ucid, &ucid, amount) {
                        Err(e) => reply!("transfer failed {e:?}"),
                        Ok(()) => reply!("transfer complete"),
                    },
                },
            },
        }
    }
}

fn delete_command(ctx: &mut Context, id: PlayerId, s: &str) {
    macro_rules! reply {
        ($msg:tt) => {
            ctx.db
                .ephemeral
                .msgs()
                .send(MsgTyp::Chat(Some(id)), format_compact!($msg))
        };
    }
    if let Some(ifo) = ctx.connected.get(&id) {
        match s.parse::<GroupId>() {
            Err(e) => reply!("delete expected a group id {e:?}"),
            Ok(id) => match ctx.db.group(&id) {
                Err(e) => reply!("could not get group {id} {e:?}"),
                Ok(group) => match &group.origin {
                    DeployKind::Crate { player, .. }
                    | DeployKind::Deployed { player, .. }
                    | DeployKind::Troop { player, .. }
                        if player != &ifo.ucid =>
                    {
                        reply!("group {id} wasn't deployed by you")
                    }
                    DeployKind::Action { .. } => reply!("can't delete an action group"),
                    DeployKind::Objective => reply!("can't delete an objective group"),
                    DeployKind::Crate { .. } => match ctx.db.delete_group(&id) {
                        Err(e) => reply!("could not delete group {id} {e:?}"),
                        Ok(()) => reply!("deleted {id}"),
                    },
                    DeployKind::Deployed {
                        player,
                        spec,
                        moved_by: _,
                    } => {
                        let player = player.clone();
                        let points = (spec.cost as f32 / 2.).ceil() as i32;
                        match ctx.db.delete_group(&id) {
                            Err(e) => reply!("could not delete group {id} {e:?}"),
                            Ok(()) => {
                                ctx.db.adjust_points(
                                    &player,
                                    points,
                                    &format_compact!("reclaimed {id}"),
                                );
                                reply!("deleted {id}")
                            }
                        }
                    }
                    DeployKind::Troop {
                        player,
                        spec,
                        moved_by: _,
                        origin: _,
                    } => {
                        let player = player.clone();
                        let points = (spec.cost as f32 / 2.).ceil() as i32;
                        match ctx.db.delete_group(&id) {
                            Err(e) => reply!("could not delete group {id} {e:?}"),
                            Ok(()) => {
                                ctx.db.adjust_points(
                                    &player,
                                    points,
                                    &format_compact!("reclaimed {id}"),
                                );
                                reply!("deleted {id}")
                            }
                        }
                    }
                },
            },
        }
    }
}

fn action_help(ctx: &mut Context, actions: &IndexMap<String, Action, FxBuildHasher>, id: PlayerId) {
    for (name, action) in actions {
        let msg = match &action.kind {
            ActionKind::Attackers(_) => Some(format_compact!("{name}: <key> | Spawn ai attackers. cost {}", action.cost)),
            ActionKind::AttackersWaypoint => Some(format_compact!("{name}: <group> <key> | Move ai attackers. cost {}", action.cost)),
            ActionKind::Move(_) => Some(format_compact!("{name}: <group> <key> | Move a ground unit. cost {}", action.cost)),
            ActionKind::Awacs(_) => Some(format_compact!(
                "{name}: <key> | Spawn an awacs at key, a mark point. cost {}",
                action.cost
            )),
            ActionKind::AwacsWaypoint => Some(format_compact!(
                "{name}: <group> <key> | Move an awacs to key, a mark point. Group is the awacs group. cost {}",
                action.cost
            )),
            ActionKind::Bomber(_) => None,
            ActionKind::Deployable(d) => Some(format_compact!(
                "{name}: <key> | Ai deploy a {} at key a mark point. cost {}",
                d.name,
                action.cost
            )),
            ActionKind::Drone(_) => Some(format_compact!(
                "{name}: <key> | Spawn a drone at key a mark point. cost {}",
                action.cost
            )),
            ActionKind::DroneWaypoint => Some(format_compact!(
                "{name}: <group> <key> | Move a drone to key, a mark point. Group is the drone group. cost {}",
                action.cost
            )),
            ActionKind::FighersWaypoint => Some(format_compact!(
                "{name}: <group> <key> | Move an a figher group to key, a mark point. Group is the fighter group. cost {}",
                action.cost
            )),
            ActionKind::Fighters(_) => Some(format_compact!(
                "{name}: <key> | Spawn ai fighters at key, a mark point. cost {}",
                action.cost
            )),
            ActionKind::LogisticsRepair(_) => Some(format_compact!(
                "{name}: <objective> | Start a logistics repair mission to objective. cost {}",
                action.cost
            )),
            ActionKind::LogisticsTransfer(_) => Some(format_compact!(
                "{name}: <from> <to> | Start a logistics transfer mission between from and to. cost {}",
                action.cost
            )),
            ActionKind::Nuke(_) => Some(format_compact!(
                "{name}: <key> | Nuke key, a mark point. cost {}", action.cost
            )),
            ActionKind::Paratrooper(d) => Some(format_compact!(
                "{name}: <key> | Drop {} troops at key, a mark point. cost {}", d.name, action.cost
            )),
            ActionKind::Tanker(_) => Some(format_compact!(
                "{name}: <key> | Spawn a tanker at key, a mark point. cost {}", action.cost
            )),
            ActionKind::TankerWaypoint => Some(format_compact!(
                "{name}: <group> <key> | Move a tanker to key. Group is the tanker group. cost {}",
                action.cost
            ))
        };
        if let Some(msg) = msg {
            ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg)
        }
    }
}

fn action_command(ctx: &mut Context, id: PlayerId, cmd: &str) {
    if cmd.trim().eq_ignore_ascii_case("help") {
        if let Some(ifo) = ctx.connected.get(&id) {
            if let Some(player) = ctx.db.player(&ifo.ucid) {
                let cfg = Arc::clone(&ctx.db.ephemeral.cfg);
                if let Some(actions) = cfg.actions.get(&player.side) {
                    action_help(ctx, actions, id)
                }
            }
        }
    } else {
        ctx.action_commands.push((id, String::from(cmd)))
    }
}

pub(super) fn run_action_commands(
    ctx: &mut Context,
    perf: &mut PerfInner,
    lua: MizLua,
) -> Result<()> {
    let spctx = SpawnCtx::new(lua).context("creating spawn ctx")?;
    for (id, s) in ctx.action_commands.drain(..) {
        if let Some(ifo) = ctx.connected.get(&id) {
            if let Some(player) = ctx.db.player(&ifo.ucid) {
                let ucid = ifo.ucid.clone();
                let side = player.side;
                let r = match ActionCmd::parse(&mut ctx.db, lua, side, &s) {
                    Err(e) => Err(e),
                    Ok(cmd) => ctx.db.start_action(
                        lua,
                        perf,
                        &spctx,
                        &ctx.idx,
                        &ctx.jtac,
                        side,
                        Some(ucid),
                        cmd,
                    ),
                };
                let msg = match r {
                    Err(e) => format_compact!("could not run action {s}: {e:?}"),
                    Ok(()) => format_compact!("action {s} started"),
                };
                ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg)
            }
        }
    }
    Ok(())
}

fn bind_command(ctx: &mut Context, id: PlayerId, s: &str) {
    static RX: OnceLock<Regex> = OnceLock::new();
    match ctx.connected.get(&id) {
        None => ctx.db.ephemeral.msgs().send(
            MsgTyp::Chat(Some(id)),
            "You must register first. Type red or blue in chat",
        ),
        Some(ifo) => {
            let rx = RX.get_or_init(|| {
                Regex::new("^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$")
                    .unwrap()
            });
            let s = s.trim();
            if !rx.is_match(s) {
                ctx.db
                    .ephemeral
                    .msgs()
                    .send(MsgTyp::Chat(Some(id)), "Invalid token")
            } else {
                ctx.db
                    .ephemeral
                    .msgs()
                    .send(MsgTyp::Chat(Some(id)), "Success");
                ctx.do_bg_task(Task::Stat(Stat::Bind {
                    id: ifo.ucid,
                    token: s.into(),
                }))
            }
        }
    }
}

fn jtac_command(ctx: &mut Context, id: PlayerId, s: &str) {
    if s.trim().eq_ignore_ascii_case("help") {
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), " -jtac <id> autoshift");
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), " -jtac <id> shift");
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), " -jtac <id> status");
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), " -jtac <id> smoke");
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), " -jtac <id> code <code>");
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), " -jtac <id> arty <id> <n>");
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), " -jtac <id> bomber");
    } else if let Some((jtid, cmd)) = s.split_once(" ") {
        if let Ok(jtid) = jtid.parse::<JtId>() {
            ctx.jtac_commands.push((id, jtid, cmd.into()));
        } else {
            ctx.db
                .ephemeral
                .msgs()
                .send(MsgTyp::Chat(Some(id)), "invalid jtac id {jtid}");
        }
    } else {
        ctx.db.ephemeral.msgs().send(
            MsgTyp::Chat(Some(id)),
            "expected -jtac <id> <cmd>, see -jtac <help>",
        );
    }
}

fn run_jtac_command(
    ctx: &mut Context,
    lua: MizLua,
    id: PlayerId,
    jtid: JtId,
    cmd: String,
) -> Result<()> {
    let ucid = ctx
        .connected
        .get(&id)
        .ok_or_else(|| anyhow!("unknown player"))?
        .ucid;
    match (ctx.jtac.get(&jtid), ctx.db.player(&ucid)) {
        (Ok(jtac), Some(player)) => {
            if jtac.side() != player.side {
                ctx.db.ephemeral.msgs().send(
                    MsgTyp::Chat(Some(id)),
                    "you can't give orders to enemy jtacs",
                );
                return Ok(());
            }
        }
        (Err(_), _) | (_, None) => {
            ctx.db
                .ephemeral
                .msgs()
                .send(MsgTyp::Chat(Some(id)), "no such jtac {jtid}");
            return Ok(());
        }
    }
    if let Some(_) = cmd.strip_prefix("autoshift") {
        let arg = ArgTuple {
            fst: ucid,
            snd: jtid,
        };
        menu::jtac::jtac_toggle_auto_shift(lua, arg)?;
    } else if let Some(_) = cmd.strip_prefix("shift") {
        let arg = ArgTuple {
            fst: ucid,
            snd: jtid,
        };
        menu::jtac::jtac_shift(lua, arg)?;
    } else if let Some(_) = cmd.strip_prefix("status") {
        let panel_to_side = ctx
            .db
            .player(&ucid)
            .map(|p| p.jtac_or_spectators)
            .unwrap_or(true);
        let arg = ArgTuple {
            fst: (!panel_to_side).then_some(ucid),
            snd: jtid,
        };
        menu::jtac::jtac_status(lua, arg)?
    } else if let Some(_) = cmd.strip_prefix("smoke") {
        let arg = ArgTuple {
            fst: ucid,
            snd: jtid,
        };
        menu::jtac::jtac_smoke_target(lua, arg)?
    } else if let Some(s) = cmd.strip_prefix("code ") {
        let code = match s.parse::<u16>() {
            Ok(c) => c,
            Err(_) => {
                ctx.db
                    .ephemeral
                    .msgs()
                    .send(MsgTyp::Chat(Some(id)), "invalid laser code {s}");
                return Ok(());
            }
        };
        let arg = ArgTriple {
            fst: jtid,
            snd: code,
            trd: ucid,
        };
        menu::jtac::jtac_set_code(lua, arg)?
    } else if let Some(arty) = cmd.strip_prefix("arty ") {
        if let Some((aid, n)) = arty.split_once(" ") {
            let aid = match aid.parse::<GroupId>() {
                Ok(id) => id,
                Err(_) => {
                    ctx.db
                        .ephemeral
                        .msgs()
                        .send(MsgTyp::Chat(Some(id)), "invalid arty group id {id}");
                    return Ok(());
                }
            };
            let n = match n.parse::<u8>() {
                Ok(n) => n,
                Err(_) => {
                    ctx.db.ephemeral.msgs().send(
                        MsgTyp::Chat(Some(id)),
                        "expected a number of shots between 0 and 255",
                    );
                    return Ok(());
                }
            };
            let arg = ArgQuad {
                fst: jtid,
                snd: aid,
                trd: n,
                fth: ucid,
            };
            menu::jtac::jtac_artillery_mission(lua, arg)?
        } else {
            ctx.db
                .ephemeral
                .msgs()
                .send(MsgTyp::Chat(Some(id)), "arty expected <id> and <n>");
        }
    } else {
        ctx.db
            .ephemeral
            .msgs()
            .send(MsgTyp::Chat(Some(id)), "invalid jtac command {s}");
    }
    Ok(())
}

pub(super) fn run_jtac_commands(ctx: &mut Context, lua: MizLua) -> Result<()> {
    let cmds = mem::take(&mut ctx.jtac_commands);
    for (id, jtid, cmd) in cmds {
        run_jtac_command(ctx, lua, id, jtid, cmd)?
    }
    Ok(())
}

fn help_command(ctx: &mut Context, id: PlayerId) {
    let admin = match ctx.connected.get(&id) {
        None => false,
        Some(ifo) => ctx.db.ephemeral.cfg.admins.contains_key(&ifo.ucid),
    };
    for cmd in [
        " blue: join the blue team",
        " red: join the red team",
        " -switch <color>: side switch to <color>",
        " -lives: display your current lives",
        " -time: how long until server restart",
        " -balance: show your points balance",
        " -transfer <amount> <player>: transfer points to another player",
        " -delete <groupid>: delete a group you deployed for a partial refund",
        " -action <name> <args>: perform an action, -action help for a list of actions",
        " -bind <token>: bind your ucid to the specified token (for the web gui)",
        " -jtac <jtid> <cmd>",
        " -help: show this help message",
    ] {
        ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), cmd)
    }
    if admin {
        ctx.db.ephemeral.msgs().send(
            MsgTyp::Chat(Some(id)),
            " -admin <command>: run admin commands, -admin help for details",
        );
    }
}

pub(super) fn process(
    ctx: &mut Context,
    lua: HooksLua,
    now: DateTime<Utc>,
    id: PlayerId,
    msg: String,
) -> Result<String> {
    if msg.eq_ignore_ascii_case("blue") || msg.eq_ignore_ascii_case("red") {
        register_player(ctx, lua, id, msg)
    } else if msg.eq_ignore_ascii_case("-switch blue") || msg.eq_ignore_ascii_case("-switch red") {
        sideswitch_player(ctx, lua, id, msg)
    } else if msg.eq_ignore_ascii_case("-lives") {
        if let Err(e) = lives_command(ctx, id) {
            error!("lives command failed for player {:?} {:?}", id, e);
        }
        Ok("".into())
    } else if msg.eq_ignore_ascii_case("-time") {
        time_command(ctx, id, now);
        Ok("".into())
    } else if let Some(msg) = msg.strip_prefix("-admin ") {
        admin_command(ctx, id, msg);
        Ok("".into())
    } else if let Some(msg) = msg.strip_prefix("-action ") {
        action_command(ctx, id, msg);
        Ok("".into())
    } else if msg.starts_with("-balance") {
        balance_command(ctx, id);
        Ok("".into())
    } else if let Some(s) = msg.strip_prefix("-transfer ") {
        transfer_command(ctx, id, s);
        Ok("".into())
    } else if let Some(s) = msg.strip_prefix("-delete ") {
        delete_command(ctx, id, s);
        Ok("".into())
    } else if let Some(s) = msg.strip_prefix("-bind ") {
        bind_command(ctx, id, s);
        Ok("".into())
    } else if let Some(s) = msg.strip_prefix("-jtac ") {
        jtac_command(ctx, id, s);
        Ok("".into())
    } else if msg.starts_with("-help") {
        help_command(ctx, id);
        Ok("".into())
    } else if msg.starts_with("-")
        || msg.as_str() == "help"
        || msg.as_str() == "points"
        || msg.as_str() == "credits"
    {
        ctx.db.ephemeral.msgs().send(
            MsgTyp::Chat(Some(id)),
            format_compact!(" {msg} is not a valid command. Valid commands follow."),
        );
        help_command(ctx, id);
        Ok("".into())
    } else {
        Ok(msg)
    }
}
