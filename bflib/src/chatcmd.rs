use crate::{
    admin::{self, AdminCommand},
    cfg::{Action, ActionKind},
    db::{
        actions::ActionCmd,
        group::{DeployKind, GroupId},
        player::RegErr,
    },
    get_player_info, lives,
    msgq::MsgTyp,
    spawnctx::SpawnCtx,
    Context,
};
use anyhow::{anyhow, bail, Context as ErrContext, Result};
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
use std::sync::Arc;

fn register_player(ctx: &mut Context, lua: HooksLua, id: PlayerId, msg: String) -> Result<String> {
    let ifo = get_player_info(
        &mut ctx.info_by_player_id,
        &mut ctx.id_by_ucid,
        &mut ctx.id_by_name,
        lua,
        id,
    )?;
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
            ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
            ctx.db.ephemeral.msgs().send(
                MsgTyp::Chat(None),
                format_compact!("{} has joined {:?} team", ifo.name, side),
            );
        }
        Err(RegErr::AlreadyOn(side)) => ctx.db.ephemeral.msgs().send(
            MsgTyp::Chat(Some(id)),
            format_compact!("you are already on {:?} team!", side),
        ),
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

fn sideswitch_player(
    ctx: &mut Context,
    lua: HooksLua,
    id: PlayerId,
    msg: String,
) -> Result<String> {
    let ifo = get_player_info(
        &mut ctx.info_by_player_id,
        &mut ctx.id_by_ucid,
        &mut ctx.id_by_name,
        lua,
        id,
    )?;
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
            ctx.db.ephemeral.msgs().send(MsgTyp::Chat(None), msg);
        }
        Err(e) => ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), e),
    }
    Ok("".into())
}

fn lives_command(ctx: &mut Context, id: PlayerId) -> Result<()> {
    let ifo = ctx
        .info_by_player_id
        .get(&id)
        .ok_or_else(|| anyhow!("missing info for player {:?}", id))?;
    let msg = lives(&mut ctx.db, &ifo.ucid, None)?;
    ctx.db.ephemeral.msgs().send(MsgTyp::Chat(Some(id)), msg);
    Ok(())
}

fn admin_command(ctx: &mut Context, id: PlayerId, cmd: &str) {
    let ifo = match ctx.info_by_player_id.get(&id) {
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
            ctx.admin_commands.push((id, cmd))
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
    if let Some(ifo) = ctx.info_by_player_id.get(&id) {
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
    if let Some(ifo) = ctx.info_by_player_id.get(&id) {
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
    if let Some(ifo) = ctx.info_by_player_id.get(&id) {
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
            ActionKind::CruiseMissileSpawn(_) => Some(format_compact!(
                "{name}: <key> | Spawn a cruise missile bomber at key, a mark point. cost {}",
                action.cost
            )),
            ActionKind::CruiseMissile(_) => Some(format_compact!(
                "{name}: <key> | Commence a cruise missile strike at key, a mark point. cost {}",
                action.cost
            )),
            ActionKind::CruiseMissileWaypoint => Some(format_compact!(
                "{name}: <group> <key> | Move a cruise missile bomber to key, a mark point. Group is the bomber group. cost {}",
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
        if let Some(ifo) = ctx.info_by_player_id.get(&id) {
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

pub(super) fn run_action_commands(ctx: &mut Context, lua: MizLua) -> Result<()> {
    let spctx = SpawnCtx::new(lua).context("creating spawn ctx")?;
    for (id, s) in ctx.action_commands.drain(..) {
        if let Some(ifo) = ctx.info_by_player_id.get(&id) {
            if let Some(player) = ctx.db.player(&ifo.ucid) {
                let ucid = ifo.ucid.clone();
                let side = player.side;
                let r = match ActionCmd::parse(&mut ctx.db, lua, side, &s) {
                    Err(e) => Err(e),
                    Ok(cmd) => {
                        ctx.db
                            .start_action(&spctx, &ctx.idx, &ctx.jtac, side, Some(ucid), cmd)
                    }
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

fn help_command(ctx: &mut Context, id: PlayerId) {
    let admin = match ctx.info_by_player_id.get(&id) {
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
