use super::{group::GroupId, objective::ObjectiveId, Db};
use crate::{
    admin,
    cfg::{
        Action, ActionKind, AiPlaneCfg, BomberCfg, CruiseMissileCfg, DeployableCfg, LogiCfg,
        NukeCfg,
    }, spawnctx::SpawnCtx,
};
use anyhow::{anyhow, bail, Result};
use dcso3::{coalition::Side, env::miz::MizIndex, net::Ucid, world::World, MizLua, String, Vector3};
use smallvec::{smallvec, SmallVec};

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
    LogisticsTransfer(WithObj<LogiCfg>),
}

impl ActionArgs {
    pub fn parse(db: &Db, action: &ActionKind, lua: MizLua, side: Side, s: &str) -> Result<Self> {
        fn get_key_pos(lua: MizLua, side: Side, key: &str) -> Result<Vector3> {
            let mut found: SmallVec<[Vector3; 4]> = smallvec![];
            for mk in World::singleton(lua)?.get_mark_panels()? {
                let mk = mk?;
                if mk.side.is_match(&side) && mk.text.as_str() == key {
                    found.push(mk.pos.0);
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
                Ok(found[0])
            }
        }
        fn pos_group(lua: MizLua, side: Side, s: &str) -> Result<WithPosAndGroup<()>> {
            match s.split_once(" ") {
                None => bail!("expected <gid> <key>"),
                Some((gid, key)) => Ok(WithPosAndGroup {
                    cfg: (),
                    pos: get_key_pos(lua, side, key)?,
                    group: gid.parse()?,
                }),
            }
        }
        fn pos<T>(lua: MizLua, side: Side, cfg: T, s: &str) -> Result<WithPos<T>> {
            let pos = get_key_pos(lua, side, s)?;
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
        match action.clone() {
            ActionKind::Tanker(c) => Ok(Self::Tanker(pos(lua, side, c, s)?)),
            ActionKind::Awacs(c) => Ok(Self::Awacs(pos(lua, side, c, s)?)),
            ActionKind::Fighters(c) => Ok(Self::Fighters(pos(lua, side, c, s)?)),
            ActionKind::FighersWaypoint => Ok(Self::FightersWaypoint(pos_group(lua, side, s)?)),
            ActionKind::Drone(c) => Ok(Self::Drone(pos(lua, side, c, s)?)),
            ActionKind::DroneWaypoint => Ok(Self::DroneWaypoint(pos_group(lua, side, s)?)),
            ActionKind::CruiseMissileStrike(c) => Ok(Self::CruiseMissileStrike(jtac(c, s)?)),
            ActionKind::Nuke(c) => Ok(Self::Nuke(pos(lua, side, c, s)?)),
            ActionKind::Paratrooper(c) => Ok(Self::Paratrooper(pos(lua, side, c, s)?)),
            ActionKind::Deployable(c) => Ok(Self::Deployable(pos(lua, side, c, s)?)),
            ActionKind::LogisticsRepair(c) => Ok(Self::LogisticsRepair(obj(db, c, s)?)),
            ActionKind::LogisticsTransfer(c) => Ok(Self::LogisticsTransfer(obj(db, c, s)?)),
            ActionKind::AwacsWaypoint => Ok(Self::AwacsWaypoint(pos_group(lua, side, s)?)),
            ActionKind::TankerWaypoint => Ok(Self::TankerWaypoint(pos_group(lua, side, s)?)),
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
    pub fn parse(db: &Db, lua: MizLua, side: Side, s: &str) -> Result<Self> {
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

impl Db {
    pub fn start_action(&mut self, spctx: &SpawnCtx, idx: MizIndex, ucid: &Ucid, cmd: ActionCmd) -> Result<()> {
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
                let n = self
                    .ephemeral
                    .actions_taken
                    .entry(player.side)
                    .or_default()
                    .entry(cmd.name.clone())
                    .or_default();
                if let Some(limit) = cmd.action.limit {
                    if *n >= limit {
                        bail!("{} is out of {} actions", player.side, cmd.name)
                    }
                }
                *n += 1;
            }
        }
        match cmd.args {
            ActionArgs::Awacs(args) => self.awacs(lua, ucid, cmd.name, cmd.action, args)?,
            ActionArgs::AwacsWaypoint(args) => {
                self.move_awacs(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::Bomber(args) => {
                self.bomber_strike(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::CruiseMissileStrike(args) => {
                self.cruise_missile_strike(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::Deployable(args) => {
                self.ai_deploy(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::Fighters(args) => {
                self.ai_fighters(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::FightersWaypoint(args) => {
                self.move_ai_fighters(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::Drone(args) => self.drone(lua, ucid, cmd.name, cmd.action, args)?,
            ActionArgs::DroneWaypoint(args) => {
                self.move_drone(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::LogisticsRepair(args) => {
                self.ai_logistics_repair(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::LogisticsTransfer(args) => {
                self.ai_logistics_transfer(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::Nuke(args) => self.nuke(lua, ucid, cmd.name, cmd.action, args)?,
            ActionArgs::Paratrooper(args) => {
                self.paratroops(lua, ucid, cmd.name, cmd.action, args)?
            }
            ActionArgs::Tanker(args) => self.tanker(lua, ucid, cmd.name, cmd.action, args)?,
            ActionArgs::TankerWaypoint(args) => {
                self.move_tanker(lua, ucid, cmd.name, cmd.action, args)?
            }
        }
        Ok(())
    }

    fn move_drone(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn drone(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn move_ai_fighters(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn move_tanker(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn tanker(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn paratroops(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPos<DeployableCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn nuke(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPos<NukeCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_logistics_transfer(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithObj<LogiCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_logistics_repair(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithObj<LogiCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_fighters(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn ai_deploy(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPos<DeployableCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn cruise_missile_strike(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithJtac<CruiseMissileCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn bomber_strike(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithJtac<BomberCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn awacs(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        
        unimplemented!()
    }

    fn move_awacs(
        &mut self,
        spctx: &SpawnCtx,
        ucid: &Ucid,
        name: String,
        action: Action,
        args: WithPosAndGroup<()>,
    ) -> Result<()> {
        unimplemented!()
    }
}
