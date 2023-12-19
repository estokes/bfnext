use crate::cfg::LifeType;

use super::{Db, Map, Player};
use anyhow::{anyhow, Result};
use chrono::{prelude::*, Duration};
use dcso3::{
    coalition::Side,
    net::{SlotId, SlotIdKind, Ucid},
    String, Vector2,
};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone, Copy)]
pub enum SlotAuth {
    Yes,
    ObjectiveNotOwned,
    NoLives,
    NotRegistered(Side),
}

impl Db {
    pub fn takeoff(&mut self, time: DateTime<Utc>, slot: SlotId, position: Vector2) -> Result<bool> {
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
        let life_type = self.ephemeral.cfg.life_types[&objective.slots[&slot]];
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
            self.ephemeral.dirty = true;
            Ok(true)
        } else {
            Ok(false)
        }
    }

    pub fn land(&mut self, slot: SlotId, position: Vector2) -> bool {
        let objective = match self
            .persisted
            .objectives_by_slot
            .get(&slot)
            .and_then(|id| self.persisted.objectives.get(&id))
        {
            Some(objective) => objective,
            None => return true,
        };
        let player = match self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid))
        {
            Some(player) => player,
            None => return true,
        };
        let life_type = self.ephemeral.cfg.life_types[&objective.slots[&slot]];
        let (_, player_lives) = match player.lives.get_mut_cow(&life_type) {
            Some(l) => l,
            None => return true,
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
            self.ephemeral.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn maybe_reset_lives(&mut self, ucid: &Ucid) -> Result<()> {
        let now = Utc::now();
        let mut lt_to_reset: SmallVec<[LifeType; 2]> = smallvec![];
        let player = self
            .persisted
            .players
            .get_mut_cow(ucid)
            .ok_or_else(|| anyhow!("no such player {:?}", ucid))?;
        for (lt, (reset, _n)) in player.lives.into_iter() {
            let reset_after = Duration::seconds(self.ephemeral.cfg.default_lives[lt].1 as i64);
            if now - reset >= reset_after {
                lt_to_reset.push(*lt);
            }
        }
        for lt in lt_to_reset {
            player.lives.remove_cow(&lt);
            self.ephemeral.dirty = true;
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
            None => return SlotAuth::NotRegistered(slot_side),
        };
        player.current_slot = None;
        self.ephemeral.players_by_slot.remove(&slot);
        self.ephemeral.cargo.remove(&slot);
        if slot_side == Side::Neutral && slot == SlotId::spectator() {
            return SlotAuth::Yes;
        }
        if slot_side != player.side {
            return SlotAuth::ObjectiveNotOwned;
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
                    Some(_) | None => return SlotAuth::ObjectiveNotOwned,
                };
                if objective.owner != player.side {
                    return SlotAuth::ObjectiveNotOwned;
                }
                let life_type = &self.ephemeral.cfg.life_types[&objective.slots[&slot]];
                macro_rules! yes {
                    () => {
                        player.current_slot = Some(slot.clone());
                        self.ephemeral.players_by_slot.insert(slot, ucid.clone());
                        break SlotAuth::Yes;
                    };
                }
                loop {
                    match player.lives.get(life_type).map(|t| *t) {
                        None => {
                            yes!();
                        }
                        Some((reset, n)) => {
                            let reset_after = Duration::seconds(
                                self.ephemeral.cfg.default_lives[life_type].1 as i64,
                            );
                            if time - reset >= reset_after {
                                player.lives.remove_cow(life_type);
                                self.ephemeral.dirty = true;
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

    pub fn register_player(
        &mut self,
        ucid: Ucid,
        name: String,
        side: Side,
    ) -> Result<(), (Option<u8>, Side)> {
        match self.persisted.players.get(&ucid) {
            Some(p) if p.side != side => Err((p.side_switches, p.side)),
            Some(_) => Ok(()),
            None => {
                self.persisted.players.insert_cow(
                    ucid,
                    Player {
                        name,
                        side,
                        side_switches: self.ephemeral.cfg.side_switches,
                        lives: Map::new(),
                        current_slot: None,
                    },
                );
                self.ephemeral.dirty = true;
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
                    self.ephemeral.dirty = true;
                    Ok(())
                }
            }
        }
    }
}
