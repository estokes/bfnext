use super::{group::GroupId, Db};
use crate::cfg::{
    Action, ActionKind, AiPlaneCfg, BomberCfg, Cfg, CruiseMissileCfg, DeployableCfg, LogiCfg,
    NukeCfg,
};
use anyhow::{anyhow, bail, Result};
use dcso3::{
    coalition::Side,
    net::Ucid,
    world::{MarkPanel, World},
    MizLua, String, Vector3,
};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone)]
struct WithPos<T> {
    cfg: T,
    pos: Vector3,
}

#[derive(Debug, Clone)]
struct WithPosAndGroup<T> {
    cfg: T,
    pos: Vector3,
    group: GroupId,
}

#[derive(Debug, Clone)]
struct WithJtac<T> {
    cfg: T,
    jtac: GroupId,
}

#[derive(Debug, Clone)]
pub enum ActionArgs {
    Tanker(WithPos<AiPlaneCfg>),
    Awacs(WithPos<AiPlaneCfg>),
    Bomber(WithJtac<BomberCfg>),
    Fighters(WithPos<AiPlaneCfg>),
    CruiseMissileStrike(WithPos<CruiseMissileCfg>),
    Nuke(WithPos<NukeCfg>),
    TankerWaypoint(WithPosAndGroup<()>),
    AwacsWaypoint(WithPosAndGroup<()>),
    Paratrooper(WithPos<DeployableCfg>),
    Deployable(WithPos<DeployableCfg>),
    LogisticsRepair(WithPos<LogiCfg>),
    LogisticsTransfer(WithPos<LogiCfg>),
}

impl ActionArgs {
    pub fn parse(action: &ActionKind, lua: MizLua, side: Side, s: &str) -> Result<Self> {
        fn get_key_pos(lua: MizLua, side: Side, key: &str) -> Result<Vector3> {
            let mut found: SmallVec<[MarkPanel; 4]> = smallvec![];
            for mk in World::singleton(lua)?.get_mark_panels()? {
                let mk = mk?;
                if mk.text.as_str() == key {
                    found.push(mk);
                }
            }
            if found.len() == 0 {
                bail!("key {key} was not found")
            } else if found.len() > 1 {
                bail!(
                    "key {key} was found {} times, make sure to choose a unique key",
                    found.len()
                )
            } else {
                Ok(found[0].pos.0)
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
        match action.clone() {
            ActionKind::Tanker(c) => Ok(Self::Tanker(pos(lua, side, c, s)?)),
            ActionKind::Awacs(c) => Ok(Self::Awacs(pos(lua, side, c, s)?)),
            ActionKind::Fighters(c) => Ok(Self::Fighters(pos(lua, side, c, s)?)),
            ActionKind::CruiseMissileStrike(c) => {
                Ok(Self::CruiseMissileStrike(pos(lua, side, c, s)?))
            }
            ActionKind::Nuke(c) => Ok(Self::Nuke(pos(lua, side, c, s)?)),
            ActionKind::Paratrooper(c) => Ok(Self::Paratrooper(pos(lua, side, c, s)?)),
            ActionKind::Deployable(c) => Ok(Self::Deployable(pos(lua, side, c, s)?)),
            ActionKind::LogisticsRepair(c) => Ok(Self::LogisticsRepair(pos(lua, side, c, s)?)),
            ActionKind::LogisticsTransfer(c) => Ok(Self::LogisticsTransfer(pos(lua, side, c, s)?)),
            ActionKind::AwacsWaypoint => Ok(Self::AwacsWaypoint(pos_group(lua, side, s)?)),
            ActionKind::TankerWaypoint => Ok(Self::TankerWaypoint(pos_group(lua, side, s)?)),
            ActionKind::Bomber(cfg) => Ok(Self::Bomber(WithJtac {
                cfg,
                jtac: s.parse()?,
            })),
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
    pub fn parse(cfg: &Cfg, lua: MizLua, side: Side, s: &str) -> Result<Self> {
        match s.split_once(" ") {
            None => bail!("expected <action> <args>"),
            Some((name, args)) => {
                let action = cfg
                    .actions
                    .get(&side)
                    .and_then(|actions| actions.get(name))
                    .ok_or_else(|| anyhow!("no such action {name}"))?
                    .clone();
                let args = ActionArgs::parse(&action.kind, lua, side, args)?;
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
    pub fn start_action(&mut self, lua: MizLua, ucid: &Ucid, cmd: ActionCmd) -> Result<()> {
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
            ActionArgs::Awacs(args) => {
                self.awacs(ucid, cmd.action, args)?
            }
            ActionArgs::AwacsWaypoint(args) => {
                self.move_awacs(ucid, cmd.action, args)?
            }
            ActionArgs::Bomber(args) => {
                self.bomber_strike(ucid, cmd.action, args)?
            }
            ActionArgs::CruiseMissileStrike(args) => {
                self.cruise_missile_strike(ucid, cmd.action, args)?
            }
            ActionArgs::Deployable(args) => {
                self.ai_deploy(ucid, cmd.action, args)?
            }
            ActionArgs::Fighters(args) => {
                self.ai_fighters(ucid, cmd.action, args)?
            }
            ActionArgs::LogisticsRepair(args) => {
                self.ai_logistics_repair(ucid, cmd.action, args)?
            }
            ActionArgs::LogisticsTransfer(args) => {
                self.ai_logistics_transfer(ucid, cmd.action, args)?
            }
            ActionArgs::Nuke(args) => {
                self.nuke(ucid, cmd.action, args)?
            }
            ActionArgs::Paratrooper(args) => {
                self.paratroops(ucid, cmd.action, args)?
            }
            ActionArgs::Tanker(args) => {
                self.tanker(ucid, cmd.action, args)?
            }
            ActionArgs::TankerWaypoint(args) => {
                self.move_tanker(ucid, cmd.action, args)?
            }
        }
        unimplemented!()
    }

    fn awacs(
        &mut self,
        ucid: &Ucid,
        action: Action,
        args: WithPos<AiPlaneCfg>,
    ) -> Result<()> {
        unimplemented!()
    }

    fn move_awacs(&mut self, ucid: &Ucid, action: Action, args: WithPosAndGroup<()>) -> Result<()> {
        unimplemented!()
    }
}
