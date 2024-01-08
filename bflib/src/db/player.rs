use super::{group::GroupId, Db, Map, Set};
use crate::{
    cfg::{LifeType, Vehicle},
    maybe,
};
use anyhow::{anyhow, Result};
use chrono::{prelude::*, Duration};
use dcso3::{
    coalition::Side,
    net::{SlotId, SlotIdKind, Ucid},
    object::DcsObject,
    unit::Unit,
    MizLua, Position3, String, Vector2, Vector3,
};
use log::error;
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone, Copy)]
pub enum SlotAuth {
    Yes,
    ObjectiveNotOwned(Side),
    ObjectiveHasNoLogistics,
    NoLives,
    NotRegistered(Side),
}

pub enum RegErr {
    AlreadyRegistered(Option<u8>, Side),
    AlreadyOn(Side),
}

#[derive(Debug, Clone)]
pub struct InstancedPlayer {
    pub position: Position3,
    pub velocity: Vector3,
    pub typ: Vehicle,
    pub in_air: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub side: Side,
    pub side_switches: Option<u8>,
    pub lives: Map<LifeType, (DateTime<Utc>, u8)>,
    pub crates: Set<GroupId>,
    #[serde(skip)]
    pub current_slot: Option<(SlotId, Option<InstancedPlayer>)>,
}

impl Db {
    pub fn player_deslot(&mut self, ucid: &Ucid) {
        if let Some(player) = self.persisted.players.get_mut_cow(ucid) {
            if let Some((slot, _)) = player.current_slot.take() {
                self.ephemeral.player_deslot(&slot)
            }
        }
    }

    pub fn player(&self, ucid: &Ucid) -> Option<&Player> {
        self.persisted.players.get(ucid)
    }

    pub fn instanced_players(&self) -> impl Iterator<Item = (&Ucid, &Player, &InstancedPlayer)> {
        self.ephemeral.players_by_slot.values().filter_map(|ucid| {
            let player = &self.persisted.players[ucid];
            player
                .current_slot
                .as_ref()
                .and_then(|(_, inst)| inst.as_ref())
                .map(|inst| (ucid, player, inst))
        })
    }

    pub fn takeoff(
        &mut self,
        time: DateTime<Utc>,
        slot: SlotId,
        position: Vector2,
    ) -> Result<Option<LifeType>> {
        let objective = self
            .persisted
            .objectives_by_slot
            .get(&slot)
            .and_then(|id| self.persisted.objectives.get(&id))
            .ok_or_else(|| anyhow!("could not find objective for slot {:?}", slot))?;
        let player = self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid))
            .ok_or_else(|| anyhow!("could not find player in slot {:?}", slot))?;
        let vehicle = maybe!(objective.slots, slot, "slot")?;
        let life_type = *maybe!(self.ephemeral.cfg.life_types, *vehicle, "life type")?;
        let (_, player_lives) = player.lives.get_or_insert_cow(life_type, || {
            (time, self.ephemeral.cfg.default_lives[&life_type].0)
        });
        let is_on_owned_objective = self
            .persisted
            .objectives
            .into_iter()
            .fold(false, |res, (_, obj)| {
                res || (obj.owner == player.side && obj.is_in_circle(position))
            });
        if is_on_owned_objective {
            if *player_lives > 0 {
                // paranoia
                *player_lives -= 1;
            }
            self.ephemeral.dirty();
            Ok(Some(life_type))
        } else {
            Ok(None)
        }
    }

    pub fn land(&mut self, slot: SlotId, position: Vector2) -> Option<LifeType> {
        let objective = match self
            .persisted
            .objectives_by_slot
            .get(&slot)
            .and_then(|id| self.persisted.objectives.get(&id))
        {
            Some(objective) => objective,
            None => return None,
        };
        let player = match self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid))
        {
            Some(player) => player,
            None => return None,
        };
        let life_type = self.ephemeral.cfg.life_types[&objective.slots[&slot]];
        let (_, player_lives) = match player.lives.get_mut_cow(&life_type) {
            Some(l) => l,
            None => return None,
        };
        let is_on_owned_objective = self
            .persisted
            .objectives
            .into_iter()
            .fold(false, |res, (_, obj)| {
                res || (obj.owner == player.side && obj.is_in_circle(position))
            });
        if is_on_owned_objective {
            *player_lives += 1;
            if *player_lives >= self.ephemeral.cfg.default_lives[&life_type].0 {
                player.lives.remove_cow(&life_type);
            }
            self.ephemeral.dirty();
            Some(life_type)
        } else {
            None
        }
    }

    pub fn maybe_reset_lives(&mut self, ucid: &Ucid, now: DateTime<Utc>) -> Result<()> {
        let mut lt_to_reset: SmallVec<[LifeType; 2]> = smallvec![];
        let player = self
            .persisted
            .players
            .get_mut_cow(ucid)
            .ok_or_else(|| anyhow!("no such player {:?}", ucid))?;
        for (lt, (reset, _n)) in player.lives.into_iter() {
            let reset_after = Duration::seconds(
                maybe!(self.ephemeral.cfg.default_lives, lt, "default life")?.1 as i64,
            );
            if now - reset >= reset_after {
                lt_to_reset.push(*lt);
            }
        }
        for lt in lt_to_reset {
            player.lives.remove_cow(&lt);
            self.ephemeral.dirty();
        }
        Ok(())
    }

    pub fn try_occupy_slot(
        &mut self,
        time: DateTime<Utc>,
        slot_side: Side,
        slot: SlotId,
        ucid: &Ucid,
    ) -> SlotAuth {
        let player = match self.persisted.players.get_mut_cow(ucid) {
            Some(player) => player,
            None => {
                if slot.is_spectator() {
                    return SlotAuth::Yes;
                }
                return SlotAuth::NotRegistered(slot_side);
            }
        };
        if slot.is_spectator() {
            return SlotAuth::Yes;
        }
        if slot_side != player.side {
            return SlotAuth::ObjectiveNotOwned(player.side);
        }
        match slot.classify() {
            SlotIdKind::ArtilleryCommander
            | SlotIdKind::ForwardObserver
            | SlotIdKind::Instructor
            | SlotIdKind::Observer => {
                // CR estokes: add permissions for game master
                SlotAuth::Yes
            }
            SlotIdKind::Normal => {
                let objective = match self
                    .persisted
                    .objectives_by_slot
                    .get(&slot)
                    .and_then(|id| self.persisted.objectives.get(id))
                {
                    Some(o) if o.owner != Side::Neutral => o,
                    Some(_) | None => return SlotAuth::ObjectiveNotOwned(player.side),
                };
                if objective.owner != player.side {
                    return SlotAuth::ObjectiveNotOwned(player.side);
                }
                if objective.captureable() {
                    return SlotAuth::ObjectiveHasNoLogistics;
                }
                let life_type = self.ephemeral.cfg.life_types[&objective.slots[&slot]];
                macro_rules! yes {
                    () => {
                        player.current_slot = Some((slot.clone(), None));
                        self.ephemeral.players_by_slot.insert(slot, ucid.clone());
                        break SlotAuth::Yes;
                    };
                }
                loop {
                    match player.lives.get(&life_type).map(|t| *t) {
                        None => {
                            yes!();
                        }
                        Some((reset, n)) => {
                            let reset_after = Duration::seconds(
                                self.ephemeral.cfg.default_lives[&life_type].1 as i64,
                            );
                            if time - reset >= reset_after {
                                player.lives.remove_cow(&life_type);
                                self.ephemeral.dirty();
                            } else if n == 0 {
                                break SlotAuth::NoLives;
                            } else {
                                yes!();
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn register_player(&mut self, ucid: Ucid, name: String, side: Side) -> Result<(), RegErr> {
        match self.persisted.players.get(&ucid) {
            Some(p) if p.side != side => Err(RegErr::AlreadyRegistered(p.side_switches, p.side)),
            Some(_) => Err(RegErr::AlreadyOn(side)),
            None => {
                self.persisted.players.insert_cow(
                    ucid,
                    Player {
                        name,
                        side,
                        side_switches: self.ephemeral.cfg.side_switches,
                        lives: Map::new(),
                        crates: Set::new(),
                        current_slot: None,
                    },
                );
                self.ephemeral.dirty();
                Ok(())
            }
        }
    }

    pub fn sideswitch_player(&mut self, ucid: &Ucid, side: Side) -> Result<(), &'static str> {
        match self.persisted.players.get_mut_cow(ucid) {
            None => Err("You are not registered. Type blue or red to join a side"),
            Some(player) => {
                if side == player.side {
                    Err("you are already on the requested side")
                } else if let Some(0) = player.side_switches {
                    Err("you can't switch sides again this round")
                } else if side == Side::Neutral {
                    Err("you can't switch to neutral")
                } else {
                    match &mut player.side_switches {
                        Some(n) => {
                            *n -= 1;
                        }
                        None => (),
                    }
                    player.side = side;
                    self.ephemeral.dirty();
                    Ok(())
                }
            }
        }
    }

    pub fn update_player_positions(&mut self, lua: MizLua) -> Result<()> {
        let mut unit: Option<Unit> = None;
        for (slot, id) in &self.ephemeral.object_id_by_slot {
            if let Some(ucid) = self.ephemeral.players_by_slot.get(slot) {
                if let Some(player) = self.persisted.players.get_mut_cow(&ucid) {
                    let instance = match unit.take() {
                        Some(unit) => unit.change_instance(id)?,
                        None => Unit::get_instance(lua, id)?,
                    };
                    let instanced_player = InstancedPlayer {
                        position: instance.get_position()?,
                        velocity: instance.get_velocity()?.0,
                        in_air: instance.in_air()?,
                        typ: Vehicle::from(instance.get_type_name()?),
                    };
                    player.current_slot = Some((slot.clone(), Some(instanced_player)));
                    unit = Some(instance);
                }
            }
        }
        Ok(())
    }

    pub fn player_entered_unit(&mut self, unit: &Unit) -> Result<()> {
        let name = unit.get_name()?;
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            self.ephemeral.units_able_to_move.insert(*uid);
        }
        Ok(())
    }

    pub fn player_left_unit(&mut self, lua: MizLua, unit: &Unit) -> Result<()> {
        let name = unit.get_name()?;
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            let uid = *uid;
            if let Err(e) = self.update_unit_positions(lua, Some(std::iter::once(uid))) {
                error!("could not sync final CA unit position {e}");
            }
            self.ephemeral.units_able_to_move.remove(&uid);
        }
        Ok(())
    }
}
