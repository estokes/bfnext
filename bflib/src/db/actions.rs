use super::{group::GroupId, Db};
use crate::cfg::{Action, ActionKind};
use anyhow::{anyhow, bail, Result};
use dcso3::{
    coalition::Side,
    controller::AltType,
    net::Ucid,
    world::{MarkPanel, World},
    MizLua, String, Vector3,
};
use smallvec::{smallvec, SmallVec};

impl Db {
    pub fn start_action(
        &mut self,
        lua: MizLua,
        ucid: &Ucid,
        key: Option<&str>,
        target_gid: Option<GroupId>,
        name: &str,
    ) -> Result<()> {
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
        if !self.ephemeral.cfg.rules.actions.check(ucid) {
            bail!("you are not authorized for actions")
        }
        let (action, pos, n) = match self.persisted.players.get(ucid) {
            None => bail!("unknown player {ucid}"),
            Some(player) => {
                let action = self
                    .ephemeral
                    .cfg
                    .actions
                    .get(&player.side)
                    .and_then(|acts| acts.get(name))
                    .ok_or_else(|| anyhow!("no such action {name} for side {}", player.side))?
                    .clone();
                if action.cost > 0 && player.points < action.cost as i32 {
                    bail!(
                        "{ucid}({}) this action costs {} points and you have {} points",
                        player.name,
                        action.cost,
                        player.points
                    )
                }
                let pos = match key.as_ref() {
                    None => None,
                    Some(k) => Some(get_key_pos(lua, player.side, k)?),
                };
                let n = self
                    .ephemeral
                    .actions_taken
                    .entry(player.side)
                    .or_default()
                    .entry(String::from(name))
                    .or_default();
                if let Some(limit) = action.limit {
                    if *n >= limit {
                        bail!("{} can't perform any more {name} actions", player.side)
                    }
                }
                (action, pos, n)
            }
        };
        let mark_required = move || -> Result<Vector3> {
            match pos {
                None => bail!("a mark is required for the {name} action"),
                Some(pos) => Ok(pos),
            }
        };
        let target_required = move || -> Result<GroupId> {
            match target_gid {
                None => bail!("a target is required for the {name} action"),
                Some(gid) => Ok(gid),
            }
        };
        match &action.kind {
            ActionKind::Awacs {
                altitude,
                altitude_typ,
                duration: _,
                template: _,
            } => {
                let pos = mark_required()?;
                *n += 1;
                self.spawn_awacs(ucid, action, *altitude, *altitude_typ, pos)?
            }
            ActionKind::AwacsWaypoint => {
                let pos = mark_required()?;
                let target = target_required()?;
                *n += 1;
                self.move_awacs(ucid, action, target, pos)?
            }
            _ => unimplemented!(),
        }
        unimplemented!()
    }

    fn spawn_awacs(
        &mut self,
        ucid: &Ucid,
        action: Action,
        altitude: f64,
        altitude_typ: AltType,
        pos: Vector3,
    ) -> Result<()> {
        unimplemented!()
    }

    fn move_awacs(
        &mut self,
        ucid: &Ucid,
        action: Action,
        target: GroupId,
        pos: Vector3,
    ) -> Result<()> {
        unimplemented!()
    }
}
