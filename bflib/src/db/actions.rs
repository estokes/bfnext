use super::{group::GroupId, objective::ObjectiveId, Db};
use crate::{
    admin,
    cfg::{
        Action, ActionKind, AiPlaneCfg, BomberCfg, CruiseMissileCfg, DeployableCfg, LogiCfg,
        NukeCfg,
    },
    db::{ephemeral, group::DeployKind},
    group,
    spawnctx::{SpawnCtx, SpawnLoc},
};
use anyhow::{anyhow, bail, Context, Ok, Result};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    controller::{AltType, MissionPoint, OrbitPattern, PointType, Task},
    env::miz::MizIndex,
    group::Group,
    net::Ucid,
    pointing_towards2,
    timer::Timer,
    trigger::MarkId,
    world::World,
    LuaVec2, MizLua, String, Vector2, Vector3,
};
use mlua::Value;
use smallvec::{smallvec, SmallVec};
use std::f64;

#[derive(Debug, Clone)]
pub struct WithPos<T> {
    pub cfg: T,
    pub pos: Vector3,
}

#[derive(Debug, Clone)]
pub struct WithObj<T> {
    pub cfg: T,
    pub oid: ObjectiveId,
}

#[derive(Debug, Clone)]
pub struct WithFromTo<T> {
    pub cfg: T,
    pub from: ObjectiveId,
    pub to: ObjectiveId,
}

#[derive(Debug, Clone)]
pub struct WithPosAndGroup<T> {
    pub cfg: T,
    pub pos: Vector3,
    pub group: GroupId,
}

#[derive(Debug, Clone)]
pub struct WithJtac<T> {
    pub cfg: T,
    pub jtac: GroupId,
}

#[derive(Debug, Clone)]
pub enum ActionArgs {
    Tanker(WithPos<AiPlaneCfg>),
    Awacs(WithPos<AiPlaneCfg>),
    Bomber(WithJtac<BomberCfg>),
    Fighters(WithPos<AiPlaneCfg>),
    FightersWaypoint(WithPosAndGroup<()>),
    Drone(WithPos<AiPlaneCfg>),
    DroneWaypoint(WithPosAndGroup<()>),
    CruiseMissileStrike(WithJtac<CruiseMissileCfg>),
    Nuke(WithPos<NukeCfg>),
    TankerWaypoint(WithPosAndGroup<()>),
    AwacsWaypoint(WithPosAndGroup<()>),
    Paratrooper(WithPos<DeployableCfg>),
    Deployable(WithPos<DeployableCfg>),
    LogisticsRepair(WithObj<LogiCfg>),
    LogisticsTransfer(WithFromTo<LogiCfg>),
}

impl ActionArgs {
    pub fn parse(
        db: &mut Db,
        action: &ActionKind,
        lua: MizLua,
        side: Side,
        s: &str,
    ) -> Result<Self> {
        fn get_key_pos(db: &mut Db, lua: MizLua, side: Side, key: &str) -> Result<Vector3> {
            let mut found: SmallVec<[(MarkId, Vector3); 4]> = smallvec![];
            for mk in World::singleton(lua)?.get_mark_panels()? {
                let mk = mk?;
                if mk.side.is_match(&side) && mk.text.as_str() == key {
                    found.push((mk.id, mk.pos.0));
                }
            }
            if found.len() == 0 {
                Err(anyhow!("key {key} was not found"))
            } else if found.len() > 1 {
                Err(anyhow!(
                    "key {key} was found {} times, make sure to choose a unique key",
                    found.len()
                ))
            } else {
                db.ephemeral.msgs().delete_mark(found[0].0);
                Ok(found[0].1)
            }
        }
        fn pos_group(db: &mut Db, lua: MizLua, side: Side, s: &str) -> Result<WithPosAndGroup<()>> {
            match s.split_once(" ") {
                None => Err(anyhow!("expected <gid> <key>")),
                Some((gid, key)) => Ok(WithPosAndGroup {
                    cfg: (),
                    pos: get_key_pos(db, lua, side, key)?,
                    group: gid.parse()?,
                }),
            }
        }
        fn pos<T>(db: &mut Db, lua: MizLua, side: Side, cfg: T, s: &str) -> Result<WithPos<T>> {
            let pos = get_key_pos(db, lua, side, s)?;
            Ok(WithPos { cfg, pos })
        }
        fn jtac<T>(cfg: T, s: &str) -> Result<WithJtac<T>> {
            Ok(WithJtac {
                cfg,
                jtac: s.parse()?,
            })
        }
        fn obj<T>(db: &Db, cfg: T, s: &str) -> Result<WithObj<T>> {
            Ok(WithObj {
                cfg,
                oid: admin::get_airbase(db, s)?,
            })
        }
        fn from_to<T>(db: &Db, cfg: T, s: &str) -> Result<WithFromTo<T>> {
            match s.split_once(" ") {
                None => Err(anyhow!("expected two objectives <from> <to>")),
                Some((from, to)) => Ok(WithFromTo {
                    cfg,
                    from: admin::get_airbase(db, from).context("getting from airbase")?,
                    to: admin::get_airbase(db, to).context("getting to airbase")?,
                }),
            }
        }
        match action.clone() {
            ActionKind::Tanker(c) => Ok(Self::Tanker(pos(db, lua, side, c, s)?)),
            ActionKind::Awacs(c) => Ok(Self::Awacs(pos(db, lua, side, c, s)?)),
            ActionKind::Fighters(c) => Ok(Self::Fighters(pos(db, lua, side, c, s)?)),
            ActionKind::FighersWaypoint => Ok(Self::FightersWaypoint(pos_group(db, lua, side, s)?)),
            ActionKind::Drone(c) => Ok(Self::Drone(pos(db, lua, side, c, s)?)),
            ActionKind::DroneWaypoint => Ok(Self::DroneWaypoint(pos_group(db, lua, side, s)?)),
            ActionKind::CruiseMissileStrike(c) => Ok(Self::CruiseMissileStrike(jtac(c, s)?)),
            ActionKind::Nuke(c) => Ok(Self::Nuke(pos(db, lua, side, c, s)?)),
            ActionKind::Paratrooper(c) => Ok(Self::Paratrooper(pos(db, lua, side, c, s)?)),
            ActionKind::Deployable(c) => Ok(Self::Deployable(pos(db, lua, side, c, s)?)),
            ActionKind::LogisticsRepair(c) => Ok(Self::LogisticsRepair(obj(db, c, s)?)),
            ActionKind::LogisticsTransfer(c) => Ok(Self::LogisticsTransfer(from_to(db, c, s)?)),
            ActionKind::AwacsWaypoint => Ok(Self::AwacsWaypoint(pos_group(db, lua, side, s)?)),
            ActionKind::TankerWaypoint => Ok(Self::TankerWaypoint(pos_group(db, lua, side, s)?)),
            ActionKind::Bomber(c) => Ok(Self::Bomber(jtac(c, s)?)),
        }
    }
}

#[derive(Debug, Clone)]
pub struct ActionCmd {
    name: String,
    action: Action,
    args: ActionArgs,
}

impl ActionCmd {
    pub fn parse(db: &mut Db, lua: MizLua, side: Side, s: &str) -> Result<Self> {
        match s.split_once(" ") {
            None => Err(anyhow!("expected <action> <args>")),
            Some((name, args)) => {
                let action = db
                    .ephemeral
                    .cfg
                    .actions
                    .get(&side)
                    .and_then(|actions| actions.get(name))
                    .ok_or_else(|| anyhow!("no such action {name}"))?
                    .clone();
                let args = ActionArgs::parse(db, &action.kind, lua, side, args)?;
                Ok(Self {
                    name: name.into(),
                    action,
                    args,
                })
            }
        }
    }
}

// setup the awacs race track 90 degrees offset from the heading
// to the nearest enemy objective
fn awacs_heading(db: &Db, pos: Vector2, enemy: Side) -> f64 {
    match dbg!(db.objective_near_point(pos, |o| o.owner == enemy)) {
        None => 0.,
        Some((_, hd, _)) => {
            let pi_2 = f64::consts::FRAC_PI_2;
            if hd < pi_2 {
                dbg!(hd + pi_2)
            } else {
                dbg!(hd - pi_2)
            }
        }
    }
}

fn awacs_orbit(
    db: &mut Db,
    lua: MizLua,
    side: Side,
    group: String,
    heading: f64,
    altitude: f64,
    alt_typ: AltType,
    pos: Vector2,
) -> Result<()> {
    let tm = Timer::singleton(lua)?;
    let point2 = dbg!(dbg!(pos) + dbg!(pointing_towards2(dbg!(heading))) * 60_000.);
    db.ephemeral.msgs().mark_to_side(side, pos, true, "awacs point 1");
    db.ephemeral.msgs().mark_to_side(side, point2, true, "awacs point 2");
    tm.schedule_function(tm.get_time()? + 1., Value::Nil, move |lua, _, _| {
        let group = Group::get_by_name(lua, &group)?;
        let con = group.get_controller().context("getting controller")?;
        macro_rules! wpt {
            ($name:expr, $pos:expr, $task:expr) => {
                MissionPoint {
                    action: None,
                    typ: PointType::TurningPoint,
                    airdrome_id: None,
                    helipad: None,
                    time_re_fu_ar: None,
                    link_unit: None,
                    pos: LuaVec2($pos),
                    alt: altitude,
                    alt_typ: Some(alt_typ.clone()),
                    speed: 200.,
                    eta: None,
                    speed_locked: None,
                    eta_locked: None,
                    name: Some($name.into()),
                    task: Box::new($task),
                }
            };
        }
        con.set_task(Task::Mission {
            airborne: None,
            route: vec![
                wpt!("ip", pos, Task::AWACS),
                wpt!(
                    "orbit",
                    point2,
                    Task::Orbit {
                        pattern: OrbitPattern::RaceTrack,
                        point: Some(LuaVec2(point2)),
                        point2: Some(LuaVec2(pos)),
                        speed: None,
                        altitude: None,
                    }
                ),
            ],
        })
        .context("setup orbit")?;
        Ok(None)
    })?;
    Ok(())
}

impl Db {
    pub fn start_action(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        cmd: ActionCmd,
    ) -> Result<()> {
        if let Some(ucid) = ucid.as_ref() {
            if !self.ephemeral.cfg.rules.actions.check(ucid) {
                bail!("you are not authorized for actions")
            }
            match self.persisted.players.get(ucid) {
                None => bail!("unknown player {ucid}"),
                Some(player) => {
                    if cmd.action.cost > 0 && player.points < cmd.action.cost as i32 {
                        bail!(
                            "{ucid}({}) this action costs {} points and you have {} points",
                            player.name,
                            cmd.action.cost,
                            player.points
                        )
                    }
                    if side != player.side {
                        bail!(
                            "mismatched action side {side} vs player side {}",
                            player.side
                        )
                    }
                }
            }
        }
        let n = self
            .ephemeral
            .actions_taken
            .entry(side)
            .or_default()
            .entry(cmd.name.clone())
            .or_default();
        if let Some(limit) = cmd.action.limit {
            if *n >= limit {
                bail!("{side} is out of {} actions", cmd.name)
            }
        }
        let name = cmd.name.clone();
        let cost = cmd.action.cost;
        match cmd.args {
            ActionArgs::Awacs(args) => self
                .awacs(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling awacs")?,
            ActionArgs::AwacsWaypoint(args) => {
                self.move_awacs(spctx, side, args).context("moving awacs")?
            }
            ActionArgs::Bomber(args) => self
                .bomber_strike(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling bomber strike")?,
            ActionArgs::CruiseMissileStrike(args) => self
                .cruise_missile_strike(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling cruise missile strike")?,
            ActionArgs::Deployable(args) => self
                .ai_deploy(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai deployment")?,
            ActionArgs::Fighters(args) => self
                .ai_fighters(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai fighters")?,
            ActionArgs::FightersWaypoint(args) => self
                .move_ai_fighters(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("moving ai fighters")?,
            ActionArgs::Drone(args) => self
                .drone(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling drone")?,
            ActionArgs::DroneWaypoint(args) => self
                .move_drone(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("moving drone")?,
            ActionArgs::LogisticsRepair(args) => self
                .ai_logistics_repair(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai logi repair")?,
            ActionArgs::LogisticsTransfer(args) => self
                .ai_logistics_transfer(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai log transfer")?,
            ActionArgs::Nuke(args) => self
                .nuke(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling nuke")?,
            ActionArgs::Paratrooper(args) => self
                .paratroops(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling paratroops")?,
            ActionArgs::Tanker(args) => self
                .tanker(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling tanker")?,
            ActionArgs::TankerWaypoint(args) => self
                .move_tanker(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("moving tanker")?,
        }
        if let Some(ucid) = ucid.as_ref() {
            self.persisted.players[ucid].points -= cost as i32;
        }
        *self
            .ephemeral
            .actions_taken
            .entry(side)
            .or_default()
            .entry(cmd.name.clone())
            .or_default() += 1;
        Ok(())
    }

    fn move_drone(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn drone(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn move_ai_fighters(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn move_tanker(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn tanker(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn paratroops(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPos<DeployableCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn nuke(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPos<NukeCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_logistics_transfer(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithFromTo<LogiCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_logistics_repair(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithObj<LogiCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_fighters(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_deploy(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPos<DeployableCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn cruise_missile_strike(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithJtac<CruiseMissileCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn bomber_strike(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithJtac<BomberCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn awacs(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        let pos = Vector2::new(args.pos.x, args.pos.z);
        let enemy = side.opposite();
        let heading = dbg!(awacs_heading(self, pos, enemy));
        let sloc = dbg!(SpawnLoc::InAir {
            pos,
            heading,
            altitude: args.cfg.altitude,
        });
        let origin = DeployKind::Action {
            player: ucid,
            name,
            spec: action,
            time: Utc::now(),
            destination: None,
            rtb: None,
        };
        let gid = self
            .add_group(spctx, idx, side, sloc, &args.cfg.template, origin)
            .context("creating group")?;
        let group = group!(self, gid)?;
        let name = group.name.clone();
        ephemeral::spawn_group(&self.persisted, idx, spctx, group).context("spawning group")?;
        awacs_orbit(
            self,
            spctx.lua(),
            side,
            name,
            heading,
            args.cfg.altitude,
            args.cfg.altitude_typ,
            pos,
        )
        .context("setup orbit")?;
        Ok(())
    }

    fn move_awacs(
        &mut self,
        spctx: &SpawnCtx,
        side: Side,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        /*
        let pos = Vector2::new(args.pos.x, args.pos.z);
        let enemy = side.opposite();
        let heading = awacs_heading(self, pos, enemy);
        let group = group!(self, args.group)?;
        let dgrp = Group::get_by_name(spctx.lua(), &group.name).context("getting awacs")?;
        awacs_orbit(dgrp, heading, pos).context("moving awacs")
        */
        unimplemented!()
    }
}
