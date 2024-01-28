/*
Copyright 2024 Eric Stokes.

This file is part of bflib.

bflib is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your
option) any later version.

bflib is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
for more details.
*/

use super::{
    group::GroupId,
    objective::{ObjectiveId, ObjectiveKind},
    Db, Map, Set,
};
use crate::{
    cfg::{LifeType, Vehicle},
    maybe, maybe_mut, objective_mut,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{prelude::*, Duration};
use dcso3::{
    airbase::Airbase,
    coalition::Side,
    net::{SlotId, SlotIdKind, Ucid},
    object::{DcsObject, DcsOid},
    unit::{ClassUnit, Unit},
    MizLua, Position3, String, Vector2, Vector3,
};
use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone)]
pub enum SlotAuth {
    Yes,
    ObjectiveNotOwned(Side),
    ObjectiveHasNoLogistics,
    NoLives,
    NotRegistered(Side),
    VehicleNotAvailable(Vehicle),
    Denied,
}

pub enum RegErr {
    AlreadyRegistered(Option<u8>, Side),
    AlreadyOn(Side),
}

#[derive(Debug, Clone, Default)]
pub struct InstancedPlayer {
    pub position: Position3,
    pub velocity: Vector3,
    pub typ: Vehicle,
    pub in_air: bool,
    pub landed_at_objective: Option<ObjectiveId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub alts: Set<String>,
    pub side: Side,
    pub side_switches: Option<u8>,
    pub lives: Map<LifeType, (DateTime<Utc>, u8)>,
    pub crates: Set<GroupId>,
    #[serde(skip)]
    pub current_slot: Option<(SlotId, Option<InstancedPlayer>)>,
    #[serde(skip)]
    pub changing_slots: Option<SlotId>,
    #[serde(skip)]
    pub jtac_or_spectators: bool,
}

impl Db {
    pub fn player_deslot(&mut self, ucid: &Ucid, kick: bool) {
        if let Some(player) = self.persisted.players.get_mut_cow(ucid) {
            if let Some((slot, _)) = player.current_slot.take() {
                let _ = self.ephemeral.player_deslot(&slot, kick);
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
        let oid = *self
            .persisted
            .objectives_by_slot
            .get(&slot)
            .ok_or_else(|| anyhow!("could not find objective for slot {:?}", slot))?;
        let objective = self
            .persisted
            .objectives
            .get(&oid)
            .ok_or_else(|| anyhow!("could not find objective for slot {:?}", slot))?;
        let player = self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid))
            .ok_or_else(|| anyhow!("could not find player in slot {:?}", slot))?;
        let sifo = maybe!(objective.slots, slot, "slot")?.clone();
        let life_type = match self.ephemeral.cfg.life_types.get(&sifo.typ) {
            None => bail!("no life type for vehicle {:?}", sifo.typ),
            Some(typ) => *typ,
        };
        let (_, player_lives) = player.lives.get_or_insert_cow(life_type, || {
            (time, self.ephemeral.cfg.default_lives[&life_type].0)
        });
        if let Some((_, Some(inst))) = &mut player.current_slot {
            inst.landed_at_objective = None;
        }
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
        let oid = match self.persisted.objectives_by_slot.get(&slot) {
            Some(oid) => *oid,
            None => return None,
        };
        let objective = match self.persisted.objectives.get(&oid) {
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
        let sifo = objective.slots[&slot].clone();
        let life_type = self.ephemeral.cfg.life_types[&sifo.typ];
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
            if let Some((_, Some(inst))) = &mut player.current_slot {
                inst.position.p.x = position.x;
                inst.position.p.z = position.y;
                inst.landed_at_objective = Some(oid);
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
        if slot.is_observer() && slot_side == Side::Neutral {
            return SlotAuth::Yes
        }
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
            player.jtac_or_spectators = true;
            return SlotAuth::Yes;
        }
        if slot_side != player.side {
            return SlotAuth::ObjectiveNotOwned(player.side);
        }
        match slot.classify() {
            SlotIdKind::Spectator => unreachable!(),
            SlotIdKind::Instructor(_) => {
                if self.ephemeral.cfg.admins.contains(ucid) {
                    player.jtac_or_spectators = true;
                    SlotAuth::Yes
                } else {
                    SlotAuth::Denied
                }
            }
            SlotIdKind::ArtilleryCommander(_) | SlotIdKind::ForwardObserver(_) | SlotIdKind::Observer(_) => {
                player.jtac_or_spectators = true;
                // CR estokes: add permissions for game master
                SlotAuth::Yes
            }
            SlotIdKind::Normal => {
                let oid = match self.persisted.objectives_by_slot.get(&slot) {
                    None => {
                        player.changing_slots = Some(slot.clone());
                        player.jtac_or_spectators = false;
                        return SlotAuth::Yes; // it's a multicrew slot
                    }
                    Some(oid) => oid,
                };
                let objective = match self.persisted.objectives.get(oid) {
                    Some(o) if o.owner != Side::Neutral => o,
                    Some(_) | None => return SlotAuth::ObjectiveNotOwned(player.side),
                };
                if objective.owner != player.side {
                    return SlotAuth::ObjectiveNotOwned(player.side);
                }
                if objective.captureable() {
                    return SlotAuth::ObjectiveHasNoLogistics;
                }
                let sifo = &objective.slots[&slot];
                let life_type = self.ephemeral.cfg.life_types[&sifo.typ];
                macro_rules! yes {
                    () => {
                        match objective.warehouse.equipment.get(sifo.typ.as_str()) {
                            Some(inv) if inv.stored > 0 => (),
                            Some(_) | None => break SlotAuth::VehicleNotAvailable(sifo.typ.clone()),
                        }
                        self.ephemeral
                            .players_by_slot
                            .insert(slot.clone(), ucid.clone());
                        player.changing_slots = Some(slot);
                        player.jtac_or_spectators = false;
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

    pub fn player_connected(&mut self, ucid: Ucid, name: String) {
        if let Some(player) = self.persisted.players.get_mut_cow(&ucid) {
            if player.name != name {
                player.alts.insert(name.clone());
                player.name = name;
                self.ephemeral.dirty()
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
                        name: name.clone(),
                        alts: Set::from_iter([name]),
                        side,
                        side_switches: self.ephemeral.cfg.side_switches,
                        lives: Map::new(),
                        crates: Set::new(),
                        current_slot: None,
                        changing_slots: None,
                        jtac_or_spectators: true,
                    },
                );
                self.ephemeral.dirty();
                Ok(())
            }
        }
    }

    pub fn force_sideswitch_player(&mut self, ucid: &Ucid, side: Side) -> Result<()> {
        let player = maybe_mut!(self.persisted.players, ucid, "no such player")?;
        player.side = side;
        self.ephemeral.dirty();
        Ok(())
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

    pub fn update_player_positions(&mut self, lua: MizLua) -> Result<Vec<DcsOid<ClassUnit>>> {
        let mut dead: Vec<DcsOid<ClassUnit>> = vec![];
        let mut unit: Option<Unit> = None;
        for (slot, id) in &self.ephemeral.object_id_by_slot {
            if let Some(ucid) = self.ephemeral.players_by_slot.get(slot) {
                if let Some(player) = self.persisted.players.get_mut_cow(&ucid) {
                    let instance = match unit.take() {
                        Some(unit) => unit.change_instance(id),
                        None => Unit::get_instance(lua, id),
                    };
                    match instance {
                        Err(e) => {
                            warn!(
                                "updating player positions, skipping invalid unit {:?}, {:?}, player {:?}",
                                player, id, e
                            );
                            dead.push(id.clone())
                        }
                        Ok(instance) => {
                            if let Some((_, Some(inst))) = &mut player.current_slot {
                                inst.position = instance.get_position()?;
                                inst.velocity = instance.get_velocity()?.0;
                                inst.in_air = instance.in_air()?;
                            }
                            unit = Some(instance);
                        }
                    }
                }
            }
        }
        Ok(dead)
    }

    pub fn player_entered_unit(&mut self, unit: &Unit) -> Result<()> {
        let name = unit.get_name()?;
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            self.ephemeral.units_able_to_move.insert(*uid);
        }
        let slot = unit.slot()?;
        if let Some(ucid) = self.ephemeral.players_by_slot.get(&slot).map(|u| u.clone()) {
            if let Some(player) = self.persisted.players.get_mut_cow(&ucid) {
                let position = unit.get_position()?;
                let point = Vector2::new(position.p.x, position.p.z);
                let landed_at_objective = self
                    .persisted
                    .objectives
                    .into_iter()
                    .find(|(_, obj)| {
                        let radius2 = obj.radius.powi(2);
                        na::distance_squared(&point.into(), &obj.pos.into()) <= radius2
                    })
                    .map(|(oid, _)| *oid);
                player.current_slot = Some((
                    slot,
                    Some(InstancedPlayer {
                        position,
                        velocity: unit.get_velocity()?.0,
                        in_air: unit.in_air()?,
                        typ: Vehicle::from(unit.get_type_name()?),
                        landed_at_objective,
                    }),
                ));
                if let Some(_) = player.changing_slots.take() {
                    self.ephemeral.cancel_force_to_spectators(&ucid);
                }
            }
        }
        Ok(())
    }

    pub fn player_left_unit(&mut self, lua: MizLua, unit: &Unit) -> Result<Vec<DcsOid<ClassUnit>>> {
        let name = unit.get_name()?;
        let mut dead = vec![];
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            let uid = *uid;
            match self.update_unit_positions(lua, &[uid]) {
                Ok(v) => dead = v,
                Err(e) => error!("could not sync final CA unit position {e}"),
            }
            self.ephemeral.units_able_to_move.remove(&uid);
        }
        let id = unit.object_id()?;
        if let Some(slot) = self.ephemeral.slot_by_object_id.get(&id) {
            if let Some(ucid) = self.ephemeral.player_in_slot(slot) {
                let ucid = ucid.clone();
                let player = maybe_mut!(self.persisted.players, ucid, "player")?;
                let kick = !player.jtac_or_spectators;
                if let Some((_, Some(inst))) = player.current_slot.as_mut() {
                    let typ = inst.typ.clone();
                    let ppos = inst.position.p.0;
                    if let Some(oid) = inst.landed_at_objective {
                        let fix_warehouse = || -> Result<()> {
                            let obj = objective_mut!(self, oid).context("get objective")?;
                            let id = maybe!(self.ephemeral.airbase_by_oid, oid, "airbase")?;
                            let airbase = Airbase::get_instance(lua, &id).context("get airbase")?;
                            let wh = airbase.get_warehouse().context("get warehouse")?;
                            if let Some(inv) = obj.warehouse.equipment.get_mut_cow(&typ.0) {
                                inv.stored = wh.get_item_count(typ.0).context("getting item")?;
                                self.ephemeral.dirty();
                            }
                            match &obj.kind {
                                ObjectiveKind::Airbase => (),
                                ObjectiveKind::Farp { .. }
                                | ObjectiveKind::Fob
                                | ObjectiveKind::Logistics => {
                                    let pos = airbase.get_point().context("get airbase pos")?.0;
                                    if na::distance_squared(&pos.into(), &ppos.into()) > 10000. {
                                        for ammo in unit.get_ammo().context("get ammo")? {
                                            let ammo = ammo.context("ammo")?;
                                            let count = ammo.count().context("ammo count")?;
                                            let typ = ammo.type_name().context("ammo typ")?;
                                            wh.add_item(typ, count)
                                                .context("add item to warehouse")?;
                                        }
                                    }
                                }
                            }
                            Ok(())
                        };
                        if let Err(e) = fix_warehouse() {
                            error!("unable to fix warehouse {:?}", e)
                        }
                    }
                }
                self.player_deslot(&ucid, kick)
            }
        }
        Ok(dead)
    }

    pub fn player_disconnected(&mut self, ucid: &Ucid) {
        if let Some((_, Some(inst))) = self
            .persisted
            .players
            .get(&ucid)
            .and_then(|p| p.current_slot.as_ref())
        {
            if let Some(oid) = inst.landed_at_objective {
                self.ephemeral.push_sync_warehouse(oid, inst.typ.clone());
            }
        }
        self.player_deslot(ucid, false);
    }
}
