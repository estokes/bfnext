use crate::{
    db::group::DeployKind,
    msgq::MsgTyp,
    spawnctx::{SpawnCtx, SpawnLoc},
    Context,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    degrees_to_radians,
    net::PlayerId,
    pointing_towards2,
    trigger::{MarkId, Trigger},
    world::World,
    MizLua, String, Vector2,
};
use log::error;
use smallvec::{smallvec, SmallVec};
use std::{mem, str::FromStr};

#[derive(Debug, Clone)]
pub enum AdminCommand {
    Help,
    ReduceInventory { airbase: String, amount: u8 },
    TransferSupply { from: String, to: String },
    LogisticsTickNow,
    LogisticsDeliverNow,
    Tim { key: String, size: usize },
    Spawn { key: String },
    SideSwitch { side: Side, player: String },
}

impl AdminCommand {
    pub fn help() -> &'static str {
        "reduce-inventory <airbase> <amount>, transfer-supply <from> <to>, logistics-tick-now, logistics-deliver-now, tim <key> [size], spawn <key>, sideswitch <side> <player>"
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
        } else if s.starts_with("reduce-inventory ") {
            let s = s.strip_prefix("reduce-inventory ").unwrap();
            match s.split_once(" ") {
                None => bail!("reduce-inventory <airbase> <amount>"),
                Some((airbase, amount)) => {
                    let amount = amount.parse::<u8>()?;
                    Ok(Self::ReduceInventory {
                        airbase: String::from(airbase),
                        amount,
                    })
                }
            }
        } else if s.starts_with("transfer-supply ") {
            let s = s.strip_prefix("transfer-supply ").unwrap();
            match s.split_once(" ") {
                None => bail!("transfer-supply <from> <to>"),
                Some((from, to)) => Ok(Self::TransferSupply {
                    from: from.into(),
                    to: to.into(),
                }),
            }
        } else if s.starts_with("logistics-tick-now") {
            Ok(Self::LogisticsTickNow)
        } else if s.starts_with("logistics-deliver-now") {
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
        } else if s.starts_with("sideswitch ") {
            let s = s.strip_prefix("sideswitch ").unwrap();
            match s.split_once(" ") {
                None => bail!("sideswitch <side> <player>"),
                Some((side, player)) => {
                    let side = side.parse::<Side>()?;
                    Ok(Self::SideSwitch {
                        side,
                        player: player.into(),
                    })
                }
            }
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
                        .cfg()
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
                        .cfg()
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

fn admin_sideswitch(ctx: &mut Context, side: Side, name: String) -> Result<()> {
    let id = ctx
        .id_by_name
        .get(&name)
        .ok_or_else(|| anyhow!("no player by name \"{}\"", name))?;
    let ifo = ctx
        .info_by_player_id
        .get(&id)
        .ok_or_else(|| anyhow!("missing player with id {:?}", id))?;
    ctx.db.force_sideswitch_player(&ifo.ucid, side)
}

pub(super) fn run_admin_commands(ctx: &mut Context, lua: MizLua) -> Result<()> {
    use std::fmt::Write;
    let mut cmds = mem::take(&mut ctx.admin_commands);
    for (id, cmd) in cmds.drain(..) {
        match cmd {
            AdminCommand::Help => (),
            AdminCommand::ReduceInventory { airbase, amount } => {
                match ctx.db.admin_reduce_inventory(lua, airbase.as_str(), amount) {
                    Err(e) => ctx.db.ephemeral.msgs().send(
                        MsgTyp::Chat(Some(id)),
                        format_compact!("reduce inventory failed: {:?}", e),
                    ),
                    Ok(()) => ctx
                        .db
                        .ephemeral
                        .msgs()
                        .send(MsgTyp::Chat(Some(id)), "inventory reduced"),
                }
            }
            AdminCommand::TransferSupply { from, to } => {
                match ctx.db.admin_transfer_supplies(lua, &from, &to) {
                    Err(e) => ctx.db.ephemeral.msgs().send(
                        MsgTyp::Chat(Some(id)),
                        format_compact!("transfer inventory failed {:?}", e),
                    ),
                    Ok(()) => ctx
                        .db
                        .ephemeral
                        .msgs()
                        .send(MsgTyp::Chat(Some(id)), "transfer complete. disconnect"),
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
                    ctx.db
                        .ephemeral
                        .msgs()
                        .send(MsgTyp::Chat(Some(id)), "tick complete")
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
                    ctx.db
                        .ephemeral
                        .msgs()
                        .send(MsgTyp::Chat(Some(id)), "deliver complete")
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
                        act.explosion(mk.pos, size as f32).context("making boom")?;
                    }
                }
                for id in to_remove {
                    ctx.db.ephemeral.msgs().delete_mark(id);
                }
            }
            AdminCommand::Spawn { key } => {
                if let Err(e) = admin_spawn(ctx, lua, id, key) {
                    ctx.db.ephemeral.msgs().send(
                        MsgTyp::Chat(Some(id)),
                        format_compact!("could not spawn {:?}", e),
                    );
                }
            }
            AdminCommand::SideSwitch { side, player } => {
                if let Err(e) = admin_sideswitch(ctx, side, player.clone()) {
                    ctx.db.ephemeral.msgs().send(
                        MsgTyp::Chat(Some(id)),
                        format_compact!("could not sideswitch {:?}", e),
                    );
                } else {
                    ctx.db.ephemeral.msgs().send(
                        MsgTyp::Chat(Some(id)),
                        format_compact!("{player} sideswitched to {side}"),
                    );
                }
            }
        }
    }
    ctx.admin_commands = cmds;
    Ok(())
}
