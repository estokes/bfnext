use super::{
    group::GroupId,
    objective::{Objective, ObjectiveId},
    Db, Map,
};
use crate::{
    admin,
    cfg::{
        Action, ActionKind, AiPlaneCfg, AiPlaneKind, BomberCfg, DeployableCfg, LimitEnforceTyp,
        NukeCfg, UnitTag,
    },
    db::{cargo::Oldest, ephemeral, group::DeployKind},
    group, group_mut,
    jtac::{JtId, Jtacs},
    objective,
    spawnctx::{SpawnCtx, SpawnLoc},
    unit,
};
use anyhow::{anyhow, bail, Context, Ok, Result};
use chrono::{prelude::*, Duration};
use compact_str::format_compact;
use dcso3::{
    attribute::Attribute,
    change_heading,
    coalition::Side,
    controller::{Command, MissionPoint, OrbitPattern, PointType, Task, TurnMethod},
    env::miz::MizIndex,
    group::Group,
    land::Land,
    net::Ucid,
    pointing_towards2,
    timer::Timer,
    trigger::{MarkId, Trigger},
    world::World,
    LuaVec2, LuaVec3, MizLua, String, Vector2, Vector3,
};
use enumflags2::BitFlags;
use fxhash::FxHashSet;
use log::error;
use mlua::Value;
use rand::{thread_rng, Rng};
use smallvec::{smallvec, SmallVec};
use std::{cmp::max, f64, vec};

#[derive(Debug, Clone)]
pub struct WithPos<T> {
    pub cfg: T,
    pub pos: Vector2,
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
    pub pos: Vector2,
    pub group: GroupId,
}

#[derive(Debug, Clone)]
pub struct WithJtac<T> {
    pub cfg: T,
    pub jtac: JtId,
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
    Nuke(WithPos<NukeCfg>),
    TankerWaypoint(WithPosAndGroup<()>),
    AwacsWaypoint(WithPosAndGroup<()>),
    Paratrooper(WithPos<DeployableCfg>),
    Deployable(WithPos<DeployableCfg>),
    LogisticsRepair(WithObj<AiPlaneCfg>),
    LogisticsTransfer(WithFromTo<AiPlaneCfg>),
}

impl ActionArgs {
    pub fn parse(
        db: &mut Db,
        action: &ActionKind,
        lua: MizLua,
        side: Side,
        s: &str,
    ) -> Result<Self> {
        fn get_key_pos(db: &mut Db, lua: MizLua, side: Side, key: &str) -> Result<Vector2> {
            let mut found: SmallVec<[(MarkId, Vector2); 4]> = smallvec![];
            for mk in World::singleton(lua)?.get_mark_panels()? {
                let mk = mk?;
                if mk.side.is_match(&side) && mk.text.as_str() == key {
                    let pos = Vector2::new(mk.pos.0.x, mk.pos.0.z);
                    found.push((mk.id, pos));
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
    pub name: String,
    pub action: Action,
    pub args: ActionArgs,
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
fn racetrack_dist_and_heading(
    obj: &Map<ObjectiveId, Objective>,
    pos: Vector2,
    enemy: Side,
) -> (f64, f64) {
    match Db::objective_near_point(obj, pos, |o| o.owner == enemy) {
        None => (9999999., 0.),
        Some((dist, hd, _)) => (dist, change_heading(hd, f64::consts::FRAC_PI_2)),
    }
}

impl Db {
    pub fn start_action(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        jtacs: &Jtacs,
        side: Side,
        ucid: Option<Ucid>,
        cmd: ActionCmd,
    ) -> Result<()> {
        let cost = match &cmd.action.kind {
            ActionKind::Nuke(nc) => {
                let div = max(1, self.persisted.nukes_used * nc.cost_scale as u32);
                max(1, cmd.action.cost / div)
            }
            ActionKind::Paratrooper(p) => {
                let sq = self
                    .ephemeral
                    .deployable_idx
                    .get(&side)
                    .and_then(|idx| idx.squads_by_name.get(&p.name))
                    .ok_or_else(|| anyhow!("missin squad"))?;
                sq.cost + cmd.action.cost
            }
            ActionKind::Deployable(d) => {
                let dp = self
                    .ephemeral
                    .deployable_idx
                    .get(&side)
                    .and_then(|idx| idx.deployables_by_name.get(&d.name))
                    .ok_or_else(|| anyhow!("missing deployable"))?;
                dp.cost + cmd.action.cost
            }
            _ => cmd.action.cost,
        };
        if let Some(ucid) = ucid.as_ref() {
            if !self.ephemeral.cfg.rules.actions.check(ucid) {
                bail!("you are not authorized for actions")
            }
            match self.persisted.players.get(ucid) {
                None => bail!("unknown player {ucid}"),
                Some(player) => {
                    if cost > 0 && player.points < cost as i32 {
                        bail!(
                            "{ucid}({}) this action costs {} points and you have {} points",
                            player.name,
                            cost,
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
        match cmd.args {
            ActionArgs::Awacs(args) => self
                .awacs(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling awacs")?,
            ActionArgs::AwacsWaypoint(args) => self
                .move_awacs(spctx, side, ucid.clone(), args)
                .context("moving awacs")?,
            ActionArgs::Bomber(args) => self
                .bomber_strike(
                    jtacs,
                    spctx,
                    idx,
                    side,
                    ucid.clone(),
                    name,
                    cmd.action,
                    args,
                )
                .context("calling bomber strike")?,
            ActionArgs::Deployable(args) => self
                .ai_deploy(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai deployment")?,
            ActionArgs::Fighters(args) => self
                .ai_fighters(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai fighters")?,
            ActionArgs::FightersWaypoint(args) => self
                .move_ai_fighters(spctx, side, ucid.clone(), args)
                .context("moving ai fighters")?,
            ActionArgs::Drone(args) => self
                .drone(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling drone")?,
            ActionArgs::DroneWaypoint(args) => self
                .move_drone(spctx, side, ucid.clone(), args)
                .context("moving drone")?,
            ActionArgs::LogisticsRepair(args) => self
                .ai_logistics_repair(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai logi repair")?,
            ActionArgs::LogisticsTransfer(args) => self
                .ai_logistics_transfer(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling ai log transfer")?,
            ActionArgs::Nuke(args) => self.nuke(spctx, args).context("calling nuke")?,
            ActionArgs::Paratrooper(args) => self
                .paratroops(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling paratroops")?,
            ActionArgs::Tanker(args) => self
                .tanker(spctx, idx, side, ucid.clone(), name, cmd.action, args)
                .context("calling tanker")?,
            ActionArgs::TankerWaypoint(args) => self
                .move_tanker(spctx, side, ucid.clone(), args)
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

    pub(super) fn respawn_action(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        gid: GroupId,
    ) -> Result<()> {
        let group = group!(self, gid)?;
        let side = group.side;
        if let DeployKind::Action {
            loc, player, spec, ..
        } = &group.origin
        {
            if let SpawnLoc::InAir { pos, .. } = loc {
                let args = WithPosAndGroup {
                    pos: *pos,
                    group: gid,
                    cfg: (),
                };
                if let ActionKind::Awacs(_) = &spec.kind {
                    ephemeral::spawn_group(&self.persisted, idx, spctx, group)?;
                    return self.move_awacs(spctx, side, player.clone(), args);
                }
                if let ActionKind::Tanker(_) = &spec.kind {
                    ephemeral::spawn_group(&self.persisted, idx, spctx, group)?;
                    return self.move_tanker(spctx, side, player.clone(), args);
                }
                if let ActionKind::Drone(_) = &spec.kind {
                    ephemeral::spawn_group(&self.persisted, idx, spctx, group)?;
                    return self.move_drone(spctx, side, player.clone(), args);
                }
                if let ActionKind::Fighters(_) = &spec.kind {
                    ephemeral::spawn_group(&self.persisted, idx, spctx, group)?;
                    return self.move_ai_fighters(spctx, side, player.clone(), args);
                }
            }
        }
        self.delete_group(&gid)
    }

    fn move_drone(
        &mut self,
        spctx: &SpawnCtx,
        side: Side,
        ucid: Option<Ucid>,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        self.move_ai_loiter_point(spctx, side, ucid, args, OrbitPattern::Circle, || {
            Task::ComboTask(vec![])
        })
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
        let gid =
            self.add_and_spawn_ai_air(spctx, idx, side, &ucid, name, action, 0., &args, None)?;
        self.move_drone(
            spctx,
            side,
            ucid,
            WithPosAndGroup {
                group: gid,
                pos: args.pos,
                cfg: (),
            },
        )
    }

    fn move_ai_fighters(
        &mut self,
        spctx: &SpawnCtx,
        side: Side,
        ucid: Option<Ucid>,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        self.move_ai_loiter_point(spctx, side, ucid, args, OrbitPattern::Circle, || {
            Task::EngageTargets {
                target_types: vec![
                    Attribute::Fighters,
                    Attribute::MultiroleFighters,
                    Attribute::BattleAirplanes,
                    Attribute::Battleplanes,
                    Attribute::Helicopters,
                    Attribute::AttackHelicopters,
                ],
                max_dist: Some(50_000.),
                priority: None,
            }
        })
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
        let gid =
            self.add_and_spawn_ai_air(spctx, idx, side, &ucid, name, action, 0., &args, None)?;
        self.move_ai_fighters(
            spctx,
            side,
            ucid,
            WithPosAndGroup {
                pos: args.pos,
                group: gid,
                cfg: (),
            },
        )
    }

    fn move_tanker(
        &mut self,
        spctx: &SpawnCtx,
        side: Side,
        ucid: Option<Ucid>,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        self.move_ai_loiter_point(spctx, side, ucid, args, OrbitPattern::RaceTrack, || {
            Task::Tanker
        })
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
        let gid =
            self.add_and_spawn_ai_air(spctx, idx, side, &ucid, name, action, 0., &args, None)?;
        self.move_ai_fighters(
            spctx,
            side,
            ucid,
            WithPosAndGroup {
                pos: args.pos,
                group: gid,
                cfg: (),
            },
        )
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
        let gid = self.add_and_spawn_ai_air(
            spctx,
            idx,
            side,
            &ucid,
            name,
            action,
            0.,
            &WithPos {
                cfg: args.cfg.plane.clone(),
                pos: args.pos,
            },
            Some(args.pos),
        )?;
        self.ai_point_to_point_mission(spctx, gid, || Task::ComboTask(vec![]))
    }

    fn nuke(&mut self, spctx: &SpawnCtx, args: WithPos<NukeCfg>) -> Result<()> {
        let land = Land::singleton(spctx.lua())?;
        let act = Trigger::singleton(spctx.lua())?.action()?;
        let alt = land.get_height(LuaVec2(args.pos))? + 500.;
        let pos = Vector3::new(args.pos.x, alt, args.pos.y);
        act.explosion(LuaVec3(pos), args.cfg.power as f32)?;
        self.persisted.nukes_used += 1;
        self.ephemeral.dirty();
        Ok(())
    }

    fn ai_logistics_transfer(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithFromTo<AiPlaneCfg>,
    ) -> Result<()> {
        let from = objective!(self, args.from)?.pos;
        let to = objective!(self, args.to)?.pos;
        let gid = self.add_and_spawn_ai_air(
            spctx,
            idx,
            side,
            &ucid,
            name,
            action,
            0.,
            &WithPos {
                cfg: args.cfg.clone(),
                pos: from,
            },
            Some(to),
        )?;
        self.ai_point_to_point_mission(spctx, gid, || Task::ComboTask(vec![]))
    }

    fn ai_logistics_repair(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithObj<AiPlaneCfg>,
    ) -> Result<()> {
        let pos = objective!(self, args.oid)?.pos;
        let gid = self.add_and_spawn_ai_air(
            spctx,
            idx,
            side,
            &ucid,
            name,
            action,
            0.,
            &WithPos {
                pos,
                cfg: args.cfg.clone(),
            },
            Some(pos),
        )?;
        self.ai_point_to_point_mission(spctx, gid, || Task::ComboTask(vec![]))
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
        let gid = self.add_and_spawn_ai_air(
            spctx,
            idx,
            side,
            &ucid,
            name,
            action,
            0.,
            &WithPos {
                cfg: args.cfg.plane.clone(),
                pos: args.pos,
            },
            Some(args.pos),
        )?;
        self.ai_point_to_point_mission(spctx, gid, || Task::ComboTask(vec![]))
    }

    fn ai_point_to_point_mission<'a>(
        &mut self,
        spctx: &SpawnCtx,
        gid: GroupId,
        task: impl Fn() -> Task<'a> + 'static,
    ) -> Result<()> {
        let group = group!(self, gid)?;
        let name = group.name.clone();
        let (src, tgt, alt, alt_typ, speed) = match &group.origin {
            DeployKind::Action {
                spec,
                destination: Some(tgt),
                rtb: Some(src),
                ..
            } => match &spec.kind {
                ActionKind::Bomber(b) => (
                    *src,
                    *tgt,
                    b.plane.altitude,
                    b.plane.altitude_typ.clone(),
                    b.plane.speed,
                ),
                ActionKind::LogisticsRepair(p)
                | ActionKind::LogisticsTransfer(p)
                | ActionKind::Paratrooper(DeployableCfg { name: _, plane: p })
                | ActionKind::Deployable(DeployableCfg { name: _, plane: p }) => {
                    (*src, *tgt, p.altitude, p.altitude_typ.clone(), p.speed)
                }
                _ => bail!("expected a point to point action"),
            },
            _ => bail!("expected action group with rtb and destination"),
        };
        let tm = Timer::singleton(spctx.lua())?;
        tm.schedule_function(tm.get_time()? + 1., Value::Nil, move |lua, _, _| {
            let group = Group::get_by_name(lua, &name)?;
            let con = group.get_controller()?;
            macro_rules! wpt {
                ($name:expr, $pos:expr) => {
                    MissionPoint {
                        action: None,
                        typ: PointType::TurningPoint,
                        airdrome_id: None,
                        helipad: None,
                        time_re_fu_ar: None,
                        link_unit: None,
                        pos: LuaVec2($pos),
                        alt,
                        alt_typ: Some(alt_typ.clone()),
                        speed,
                        eta: None,
                        speed_locked: None,
                        eta_locked: None,
                        name: Some($name.into()),
                        task: Box::new(task()),
                    }
                };
            }
            con.set_task(Task::Mission {
                airborne: None,
                route: vec![wpt!("ip", src), wpt!("tgt", tgt), wpt!("rtb", src)],
            })?;
            Ok(None)
        })?;
        Ok(())
    }

    fn bomber_strike(
        &mut self,
        jtacs: &Jtacs,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: Option<Ucid>,
        name: String,
        action: Action,
        args: WithJtac<BomberCfg>,
    ) -> Result<()> {
        let jt = jtacs.get(&args.jtac)?;
        let tgt = jt
            .target
            .as_ref()
            .map(|t| Vector2::new(t.pos.x, t.pos.z))
            .unwrap_or(jt.location.pos);
        let gid = self.add_and_spawn_ai_air(
            spctx,
            idx,
            side,
            &ucid,
            name,
            action,
            0.,
            &WithPos {
                cfg: args.cfg.plane,
                pos: tgt,
            },
            Some(tgt),
        )?;
        self.ai_point_to_point_mission(spctx, gid, || Task::ComboTask(vec![]))
    }

    fn add_and_spawn_ai_air(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        ucid: &Option<Ucid>,
        name: String,
        action: Action,
        heading: f64,
        args: &WithPos<AiPlaneCfg>,
        destination: Option<Vector2>,
    ) -> Result<GroupId> {
        let (_, _, obj) = Self::objective_near_point(&self.persisted.objectives, args.pos, |o| {
            o.owner == side
                && match args.cfg.kind {
                    AiPlaneKind::Helicopter => true,
                    AiPlaneKind::FixedWing => o.is_airbase(),
                }
                && o.pos != args.pos
        })
        .ok_or_else(|| anyhow!("no objectives available for the ai mission"))?;
        let pos = obj.pos;
        let sloc = SpawnLoc::InAir {
            pos,
            heading,
            altitude: args.cfg.altitude,
        };
        let origin = DeployKind::Action {
            marks: FxHashSet::default(),
            loc: sloc.clone(),
            player: ucid.clone(),
            name,
            spec: action,
            time: Utc::now(),
            destination,
            rtb: Some(pos),
        };
        let gid = self
            .add_group(
                spctx,
                idx,
                side,
                sloc,
                &args.cfg.template,
                origin,
                UnitTag::Driveable.into(),
            )
            .context("creating group")?;
        ephemeral::spawn_group(&self.persisted, idx, spctx, group!(self, gid)?)
            .context("spawning group")?;
        Ok(gid)
    }

    fn move_awacs(
        &mut self,
        spctx: &SpawnCtx,
        side: Side,
        ucid: Option<Ucid>,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        self.move_ai_loiter_point(spctx, side, ucid, args, OrbitPattern::RaceTrack, || {
            Task::AWACS
        })
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
        let gid =
            self.add_and_spawn_ai_air(spctx, idx, side, &ucid, name, action, 0., &args, None)?;
        self.move_awacs(
            spctx,
            side,
            ucid,
            WithPosAndGroup {
                cfg: (),
                pos: args.pos,
                group: gid,
            },
        )
        .context("setup orbit")?;
        Ok(())
    }

    fn move_ai_loiter_point<'a>(
        &mut self,
        spctx: &SpawnCtx,
        side: Side,
        ucid: Option<Ucid>,
        args: WithPosAndGroup<()>,
        pattern: OrbitPattern,
        task: impl Fn() -> Task<'a> + 'static,
    ) -> Result<()> {
        let pos = args.pos;
        let enemy = side.opposite();
        let heading = match pattern {
            OrbitPattern::Circle => 0.,
            OrbitPattern::RaceTrack => {
                racetrack_dist_and_heading(&self.persisted.objectives, pos, enemy).1
            }
            OrbitPattern::Custom(x) => bail!("invalid orbit pattern {x}"),
        };
        let group = group_mut!(self, args.group)?;
        if group.side != side {
            bail!("can't move the other team's awacs")
        }
        let name = group.name.clone();
        let (altitude, alt_typ, speed, marks, player) = match &mut group.origin {
            DeployKind::Action {
                marks,
                spec,
                loc,
                player,
                ..
            } => match &mut spec.kind {
                ActionKind::Awacs(a)
                | ActionKind::Tanker(a)
                | ActionKind::Drone(a)
                | ActionKind::Fighters(a) => {
                    match loc {
                        SpawnLoc::InAir { pos: oldpos, .. } => {
                            let dir = *oldpos - pos;
                            let step = dir.magnitude() / 4.;
                            let dir = dir.normalize();
                            let (old_dist, _) = racetrack_dist_and_heading(
                                &self.persisted.objectives,
                                *oldpos,
                                enemy,
                            );
                            for i in 1..4 {
                                let pos = *oldpos + dir * (step * i as f64);
                                let (dist, _) = racetrack_dist_and_heading(
                                    &self.persisted.objectives,
                                    pos,
                                    enemy,
                                );
                                if old_dist < dist && dist - old_dist >= 500. {
                                    *player = ucid.clone();
                                }
                            }
                            *oldpos = pos;
                            for id in marks.drain() {
                                self.ephemeral.msgs().delete_mark(id)
                            }
                        }
                        _ => bail!("race tracker not spawning in air"),
                    }
                    (a.altitude, a.altitude_typ.clone(), a.speed, marks, player)
                }
                _ => bail!("not a race tracker"),
            },
            _ => bail!("not a race tracker"),
        };
        let responsible = player
            .as_ref()
            .and_then(|u| self.persisted.players.get(u))
            .map(|p| p.name.clone())
            .unwrap_or(String::from(""));
        let (point1, point2) = match pattern {
            OrbitPattern::Circle => {
                marks.insert(self.ephemeral.msgs().mark_to_side(
                    side,
                    pos,
                    true,
                    format_compact!(
                        "{} orbit point 1\nresponsible party: {}",
                        args.group,
                        responsible
                    ),
                ));
                (pos, None)
            }
            OrbitPattern::RaceTrack => {
                let point1 =
                    pos + pointing_towards2(change_heading(heading, -f64::consts::PI)) * 30_000.;
                let point2 = pos + pointing_towards2(heading) * 30_000.;
                marks.insert(self.ephemeral.msgs().mark_to_side(
                    side,
                    point1,
                    true,
                    format_compact!(
                        "{} race point 1\nresponsible party: {}",
                        args.group,
                        responsible
                    ),
                ));
                marks.insert(self.ephemeral.msgs().mark_to_side(
                    side,
                    point2,
                    true,
                    format_compact!(
                        "{} race point 2\nresponsible party: {}",
                        args.group,
                        responsible
                    ),
                ));
                (point1, Some(point2))
            }
            OrbitPattern::Custom(x) => bail!("invalid orbit pattern {x}"),
        };
        self.ephemeral.dirty();
        let tm = Timer::singleton(spctx.lua())?;
        tm.schedule_function(tm.get_time()? + 1., Value::Nil, move |lua, _, _| {
            let group = Group::get_by_name(lua, &name)?;
            let pos = group.get_unit(1)?.get_point()?;
            let pos = Vector2::new(pos.x, pos.z);
            let con = group.get_controller().context("getting controller")?;
            macro_rules! wpt {
                ($name:expr, $pos:expr, $task:expr) => {
                    MissionPoint {
                        action: Some(TurnMethod::FlyOverPoint),
                        typ: PointType::TurningPoint,
                        airdrome_id: None,
                        helipad: None,
                        time_re_fu_ar: None,
                        link_unit: None,
                        pos: LuaVec2($pos),
                        alt: altitude,
                        alt_typ: Some(alt_typ.clone()),
                        speed,
                        eta: None,
                        speed_locked: None,
                        eta_locked: None,
                        name: Some($name.into()),
                        task: Box::new($task),
                    }
                };
            }
            con.set_command(Command::SetUnlimitedFuel(true))?;
            match &pattern {
                OrbitPattern::Circle => con
                    .set_task(Task::Mission {
                        airborne: Some(true),
                        route: vec![
                            wpt!("ip", pos, task()),
                            wpt!(
                                "orbit",
                                point1,
                                Task::ComboTask(vec![
                                    Task::Orbit {
                                        pattern: OrbitPattern::Circle,
                                        point: Some(LuaVec2(point1)),
                                        point2: None,
                                        speed: Some(speed),
                                        altitude: Some(altitude)
                                    },
                                    task(),
                                ])
                            )
                        ],
                    })
                    .context("setup orbit")?,
                OrbitPattern::RaceTrack => con
                    .set_task(Task::Mission {
                        airborne: Some(true),
                        route: vec![
                            wpt!("ip", pos, task()),
                            wpt!(
                                "point1",
                                point1,
                                Task::ComboTask(vec![
                                    Task::Orbit {
                                        pattern: OrbitPattern::RaceTrack,
                                        point: Some(LuaVec2(point1)),
                                        point2: Some(LuaVec2(point2.unwrap())),
                                        speed: Some(speed),
                                        altitude: Some(altitude),
                                    },
                                    task(),
                                ])
                            ),
                            wpt!("point2", point2.unwrap(), task()),
                        ],
                    })
                    .context("setup racetrack")?,
                OrbitPattern::Custom(x) => bail!("invalid orbit pattern {x}"),
            }
            Ok(None)
        })?;
        Ok(())
    }

    fn bomb_targets(
        &self,
        lua: MizLua,
        side: Side,
        jtacs: &Jtacs,
        cfg: &BomberCfg,
        target: Vector2,
    ) -> Result<()> {
        let mut rng = thread_rng();
        let land = Land::singleton(lua)?;
        let act = Trigger::singleton(lua)?.action()?;
        for (i, (_, ct)) in jtacs.contacts_near_point(side, target, 15_000.).enumerate() {
            if i < cfg.targets as usize {
                let dir = Vector2::new(rng.gen_range(0. ..1.), rng.gen_range(0. ..1.)).normalize();
                let mag = rng.gen_range(0. ..cfg.accuracy as f64);
                let pos = Vector2::new(ct.pos.x, ct.pos.z) + dir * mag;
                let alt = land.get_height(LuaVec2(pos))?;
                let pos = Vector3::new(pos.x, alt, pos.y);
                act.explosion(LuaVec3(pos), cfg.power as f32)?
            }
        }
        Ok(())
    }

    fn repair_target(&mut self, target: Vector2, side: Side) -> Result<()> {
        let (dist, _, obj) =
            Self::objective_near_point(&self.persisted.objectives, target, |o| o.owner == side)
                .ok_or_else(|| anyhow!("no friendly objective near drop off point"))?;
        if dist > 5_000. {
            bail!("no friendly objective near drop off point")
        }
        let oid = obj.id;
        self.repair_one_logi_step(side, Utc::now(), oid)?;
        Ok(())
    }

    fn transfer_to_target(
        &mut self,
        lua: MizLua,
        src: Vector2,
        target: Vector2,
        side: Side,
    ) -> Result<()> {
        let (dist, _, src) =
            Self::objective_near_point(&self.persisted.objectives, src, |o| o.owner == side)
                .ok_or_else(|| anyhow!("no friendly objective near source point"))?;
        if dist > 5_000. {
            bail!("no friendly objective near source point")
        }
        let (dist, _, tgt) =
            Self::objective_near_point(&self.persisted.objectives, target, |o| o.owner == side)
                .ok_or_else(|| anyhow!("no friendly objective near target point"))?;
        if dist > 5_000. {
            bail!("no friendly objective near target point")
        }
        let src = src.id;
        let tgt = tgt.id;
        self.transfer_supplies(lua, src, tgt)
    }

    fn deployable_to_point(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        pos: Vector2,
        dep: String,
        side: Side,
        ucid: Ucid,
    ) -> Result<()> {
        let spec = self
            .ephemeral
            .deployable_idx
            .get(&side)
            .ok_or_else(|| anyhow!("no such deployable {dep} for {side}"))?
            .deployables_by_name
            .get(dep.as_str())
            .ok_or_else(|| anyhow!("no such deployable {dep} for {side}"))?
            .clone();
        let (n, oldest) = self.number_deployed(side, &**dep)?;
        if n >= spec.limit as usize {
            match spec.limit_enforce {
                LimitEnforceTyp::DenyCrate => {
                    bail!("the max number of {:?} are already deployed", dep)
                }
                LimitEnforceTyp::DeleteOldest => match oldest {
                    Some(Oldest::Group(gid)) => self.delete_group(&gid)?,
                    Some(Oldest::Objective(oid)) => self.delete_objective(&oid)?,
                    None => (),
                },
            }
        }
        let spctx = SpawnCtx::new(lua)?;
        let spawnloc = SpawnLoc::AtPos {
            pos,
            offset_direction: Vector2::new(1., 0.),
            group_heading: 0.,
        };
        let origin = DeployKind::Deployed {
            player: ucid,
            spec: spec.clone(),
        };
        self.add_and_queue_group(
            &spctx,
            idx,
            side,
            spawnloc,
            &*spec.template,
            origin,
            BitFlags::empty(),
            None,
        )?;
        Ok(())
    }

    fn paratroops_to_point(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        pos: Vector2,
        troop: String,
        side: Side,
        ucid: Ucid,
    ) -> Result<()> {
        let troop_cfg = self
            .ephemeral
            .deployable_idx
            .get(&side)
            .ok_or_else(|| anyhow!("no such troop {troop} for {side}"))?
            .squads_by_name
            .get(troop.as_str())
            .ok_or_else(|| anyhow!("no such troop {troop} for {side}"))?
            .clone();
        let spawnpos = SpawnLoc::AtPos {
            pos,
            offset_direction: Vector2::new(1., 0.),
            group_heading: 0.,
        };
        let dk = DeployKind::Troop {
            player: ucid.clone(),
            spec: troop_cfg.clone(),
        };
        let spctx = SpawnCtx::new(lua)?;
        let (n, oldest) = self.number_troops_deployed(side, troop_cfg.name.as_str())?;
        let to_delete = if n < troop_cfg.limit as usize {
            None
        } else {
            match troop_cfg.limit_enforce {
                LimitEnforceTyp::DeleteOldest => oldest,
                LimitEnforceTyp::DenyCrate => {
                    bail!(
                        "the maximum number of {} troops are already deployed",
                        troop_cfg.name
                    )
                }
            }
        };
        if let Some(gid) = to_delete {
            self.delete_group(&gid)?
        }
        self.add_and_queue_group(
            &spctx,
            idx,
            side,
            spawnpos,
            &*troop_cfg.template,
            dk,
            BitFlags::empty(),
            None,
        )?;
        Ok(())
    }

    pub fn advance_actions(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        jtacs: &Jtacs,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let mut to_delete: SmallVec<[GroupId; 4]> = smallvec![];
        let mut to_bomb: SmallVec<[(BomberCfg, Vector2, Side); 2]> = smallvec![];
        let mut to_repair: SmallVec<[(Vector2, Side); 2]> = smallvec![];
        let mut to_transfer: SmallVec<[(Vector2, Vector2, Side); 2]> = smallvec![];
        let mut to_deploy: SmallVec<[(Vector2, String, Side, Ucid); 2]> = smallvec![];
        let mut to_paratroop: SmallVec<[(Vector2, String, Side, Ucid); 2]> = smallvec![];
        macro_rules! at_dest {
            ($group:expr, $dest:expr, $radius:expr) => {{
                let r2 = f64::powi($radius, 2);
                let mut iter = $group.units.into_iter();
                loop {
                    match iter.next() {
                        None => break false,
                        Some(uid) => {
                            let unit = unit!(self, uid)?;
                            if na::distance_squared(&unit.pos.into(), &$dest.into()) <= r2 {
                                break true;
                            }
                        }
                    }
                }
            }};
        }
        for gid in &self.persisted.actions {
            let group = group_mut!(self, gid)?;
            if let DeployKind::Action {
                spec,
                time,
                destination,
                rtb,
                player,
                ..
            } = &mut group.origin
            {
                match &spec.kind {
                    ActionKind::Awacs(ai)
                    | ActionKind::Fighters(ai)
                    | ActionKind::Drone(ai)
                    | ActionKind::Tanker(ai) => {
                        if let Some(d) = ai.duration {
                            if now - *time > Duration::hours(d as i64) {
                                to_delete.push(*gid);
                            }
                        }
                    }
                    ActionKind::Bomber(b) => {
                        if let Some(target) = *destination {
                            if at_dest!(group, target, 10_000.) {
                                destination.take();
                                to_bomb.push((b.clone(), target, group.side));
                            }
                        }
                        if destination.is_none() {
                            if let Some(target) = *rtb {
                                if at_dest!(group, target, 10_000.) {
                                    to_delete.push(*gid);
                                }
                            }
                        }
                    }
                    ActionKind::LogisticsRepair(_) => {
                        if let Some(target) = *destination {
                            if at_dest!(group, target, 800.) {
                                destination.take();
                                to_repair.push((target, group.side));
                                to_delete.push(group.id);
                            }
                        }
                    }
                    ActionKind::LogisticsTransfer(_) => {
                        if let Some(target) = *destination {
                            if at_dest!(group, target, 800.) {
                                destination.take();
                                if let Some(rtb) = *rtb {
                                    to_transfer.push((rtb, target, group.side));
                                    to_delete.push(group.id);
                                }
                            }
                        }
                    }
                    ActionKind::Paratrooper(t) => {
                        if let Some(target) = *destination {
                            if at_dest!(group, target, 800.) {
                                destination.take();
                                let ucid = player
                                    .as_ref()
                                    .map(|u| u.clone())
                                    .ok_or_else(|| anyhow!("paratroop missions require a ucid"))?;
                                to_paratroop.push((target, t.name.clone(), group.side, ucid));
                            }
                        }
                        if let Some(target) = *rtb {
                            if at_dest!(group, target, 800.) {
                                to_delete.push(*gid);
                            }
                        }
                    }
                    ActionKind::Deployable(d) => {
                        if let Some(target) = *destination {
                            if at_dest!(group, target, 800.) {
                                destination.take();
                                let ucid = player.as_ref().map(|u| u.clone()).ok_or_else(|| {
                                    anyhow!("deployables missions require a ucid")
                                })?;
                                to_deploy.push((target, d.name.clone(), group.side, ucid));
                            }
                        }
                        if destination.is_none() {
                            if let Some(target) = *rtb {
                                if at_dest!(group, target, 800.) {
                                    to_delete.push(*gid);
                                }
                            }
                        }
                    }
                    ActionKind::AwacsWaypoint
                    | ActionKind::FighersWaypoint
                    | ActionKind::TankerWaypoint
                    | ActionKind::DroneWaypoint
                    | ActionKind::Nuke(_) => {
                        bail!("should not be a group")
                    }
                }
            }
        }
        for gid in to_delete {
            if let Err(e) = self.delete_group(&gid) {
                error!("delete action group failed {e:?}")
            }
        }
        for (cfg, target, side) in to_bomb {
            if let Err(e) = self.bomb_targets(lua, side, jtacs, &cfg, target) {
                error!("bomb targets failed {e:?}")
            }
        }
        for (target, side) in to_repair {
            if let Err(e) = self.repair_target(target, side) {
                self.ephemeral.msgs().panel_to_side(
                    10,
                    false,
                    side,
                    format_compact!("repair mission failed {e:?}"),
                );
            }
        }
        for (src, target, side) in to_transfer {
            if let Err(e) = self.transfer_to_target(lua, src, target, side) {
                self.ephemeral.msgs().panel_to_side(
                    10,
                    false,
                    side,
                    format_compact!("transfer mission failed {e:?}"),
                );
            }
        }
        for (dst, troop, side, ucid) in to_paratroop {
            if let Err(e) = self.paratroops_to_point(lua, idx, dst, troop, side, ucid.clone()) {
                self.ephemeral.panel_to_player(
                    &self.persisted,
                    &ucid,
                    format_compact!("paratroop mission failed {e:?}"),
                )
            }
        }
        for (dst, dep, side, ucid) in to_deploy {
            if let Err(e) = self.deployable_to_point(lua, idx, dst, dep, side, ucid.clone()) {
                self.ephemeral.panel_to_player(
                    &self.persisted,
                    &ucid,
                    format_compact!("deploy mission failed {e:?}"),
                )
            }
        }
        Ok(())
    }
}
