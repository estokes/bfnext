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
    group::{DeployKind, GroupId},
    objective::{ObjectiveId, ObjectiveKind},
    Db, Map, Set,
};
use crate::{
    cfg::{LifeType, PointsCfg, UnitTag, Vehicle},
    maybe, maybe_mut, objective_mut,
    shots::Dead,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{prelude::*, Duration};
use compact_str::format_compact;
use dcso3::{
    airbase::Airbase,
    coalition::Side,
    net::{SlotId, Ucid},
    object::{DcsObject, DcsOid},
    unit::{ClassUnit, Unit},
    MizLua, Position3, String, Vector2, Vector3,
};
use log::{debug, error, warn};
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone)]
pub enum SlotAuth {
    Yes,
    ObjectiveNotOwned(Side),
    ObjectiveHasNoLogistics,
    NoLives(LifeType),
    NotRegistered(Side),
    VehicleNotAvailable(Vehicle),
    Denied,
}

pub enum RegErr {
    AlreadyRegistered(Option<u8>, Side),
    AlreadyOn(Side),
}

#[derive(Debug, Clone)]
pub enum TakeoffRes {
    TookLife(LifeType),
    NoLifeTaken,
    OutOfLives,
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
    #[serde(default)]
    pub airborne: Option<LifeType>,
    #[serde(default)]
    pub points: i32,
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
            player.airborne = None;
            if let Some((slot, _)) = player.current_slot.take() {
                let _ = self.ephemeral.player_deslot(&slot, kick);
            }
        }
    }

    pub fn player(&self, ucid: &Ucid) -> Option<&Player> {
        self.persisted.players.get(ucid)
    }

    pub fn player_mut(&mut self, ucid: &Ucid) -> Option<&mut Player> {
        self.persisted.players.get_mut_cow(ucid)
    }

    pub fn transfer_points(&mut self, source: &Ucid, target: &Ucid, amount: u32) -> Result<()> {
        let sp = self
            .persisted
            .players
            .get_mut_cow(source)
            .ok_or_else(|| anyhow!("source player not found"))?;
        if sp.points < amount as i32 {
            bail!(
                "insufficient balance, you have {}, you requested {}",
                sp.points,
                amount
            )
        }
        sp.points -= amount as i32;
        match self.persisted.players.get_mut_cow(target) {
            Some(tp) => {
                tp.points += amount as i32;
                self.ephemeral.dirty();
                Ok(())
            }
            None => {
                self.persisted.players[source].points += amount as i32;
                bail!("target player not found")
            }
        }
    }

    pub fn player_reset_lives(&mut self, ucid: &Ucid) -> Result<()> {
        maybe_mut!(self.persisted.players, ucid, "player")?.lives = Map::new();
        self.ephemeral.dirty();
        Ok(())
    }

    pub fn instanced_players(&self) -> impl Iterator<Item = (&Ucid, &Player, &InstancedPlayer)> {
        self.ephemeral.players_by_slot.values().filter_map(|ucid| {
            self.persisted.players.get(ucid).and_then(|player| {
                player
                    .current_slot
                    .as_ref()
                    .and_then(|(_, inst)| inst.as_ref())
                    .map(|inst| (ucid, player, inst))
            })
        })
    }

    pub fn player_in_unit(&self, include_deployed: bool, id: &DcsOid<ClassUnit>) -> Option<Ucid> {
        match self
            .ephemeral
            .get_slot_by_object_id(id)
            .and_then(|s| self.ephemeral.players_by_slot.get(s))
        {
            Some(ucid) => Some(ucid.clone()),
            None => {
                if !include_deployed {
                    None
                } else {
                    self.ephemeral
                        .uid_by_object_id
                        .get(id)
                        .and_then(|uid| self.persisted.units.get(uid))
                        .and_then(|unit| self.persisted.groups.get(&unit.group))
                        .and_then(|group| match &group.origin {
                            DeployKind::Deployed {
                                player,
                                spec: _,
                                moved_by: _,
                            } => Some(player.clone()),
                            DeployKind::Troop {
                                player,
                                spec: _,
                                moved_by: _,
                            } => Some(player.clone()),
                            DeployKind::Action { player, .. } => player.clone(),
                            DeployKind::Crate { .. } | DeployKind::Objective => None,
                        })
                }
            }
        }
    }

    pub fn takeoff(
        &mut self,
        time: DateTime<Utc>,
        slot: SlotId,
        position: Vector2,
    ) -> Result<TakeoffRes> {
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
            // paranoia
            if *player_lives == 0 {
                return Ok(TakeoffRes::OutOfLives);
            } else {
                player.airborne = Some(life_type);
                *player_lives -= 1;
            }
            self.ephemeral.dirty();
            Ok(TakeoffRes::TookLife(life_type))
        } else {
            Ok(TakeoffRes::NoLifeTaken)
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
            player.airborne = None;
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
        match slot {
            SlotId::Spectator => unreachable!(),
            SlotId::Instructor(_, _) => {
                if self.ephemeral.cfg.admins.contains_key(ucid) {
                    player.jtac_or_spectators = true;
                    SlotAuth::Yes
                } else {
                    SlotAuth::Denied
                }
            }
            SlotId::ArtilleryCommander(_, _)
            | SlotId::ForwardObserver(_, _)
            | SlotId::Observer(_, _) => {
                if self.ephemeral.cfg.rules.ca.check(ucid) {
                    player.jtac_or_spectators = true;
                    SlotAuth::Yes
                } else {
                    SlotAuth::Denied
                }
            }
            SlotId::Unit(_) | SlotId::MultiCrew(_, _) => {
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
                                break SlotAuth::NoLives(life_type);
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
                    ucid.clone(),
                    Player {
                        name: name.clone(),
                        alts: Set::from_iter([name.clone()]),
                        side,
                        side_switches: self.ephemeral.cfg.side_switches,
                        lives: Map::new(),
                        crates: Set::new(),
                        airborne: None,
                        points: self
                            .ephemeral
                            .cfg
                            .points
                            .map(|p| p.new_player_join as i32)
                            .unwrap_or(0),
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
                                "updating player positions, skipping invalid unit {ucid:?}, {id:?}, player {e:?}",
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

    pub fn player_entered_slot(
        &mut self,
        lua: MizLua,
        id: DcsOid<ClassUnit>,
        unit: &Unit,
        slot: SlotId,
        oid: ObjectiveId,
    ) -> Result<()> {
        self.ephemeral
            .slot_by_object_id
            .insert(id.clone(), slot.clone());
        self.ephemeral
            .object_id_by_slot
            .insert(slot.clone(), id.clone());
        self.ephemeral.dirty();
        match self.ephemeral.players_by_slot.get(&slot).map(|u| u.clone()) {
            None => {
                unit.clone()
                    .destroy()
                    .context("destroying slot unit with no player")?;
                self.unit_dead(lua, &id, Utc::now())?
            }
            Some(ucid) => {
                let obj = objective_mut!(self, oid)?;
                let sifo = maybe!(obj.slots, slot, "slot")?;
                let mut adjust_warehouse = || -> Result<()> {
                    let id = maybe!(self.ephemeral.airbase_by_oid, obj.id, "airbase")?;
                    let wh = Airbase::get_instance(lua, id)
                        .context("getting airbase")?
                        .get_warehouse()
                        .context("getting warehouse")?;
                    if sifo.ground_start {
                        wh.remove_item(sifo.typ.0.clone(), 1).with_context(|| {
                            format_compact!("removing {} from warehouse", sifo.typ.0)
                        })?;
                        for wep in unit.get_ammo()? {
                            let wep = wep?;
                            let count = wep.count()?;
                            let typ = wep.type_name()?;
                            let whcnt = wh.get_item_count(typ.clone())?;
                            debug!(
                                "removing {count} {typ} from the warehouse which contains {whcnt}"
                            );
                            wh.remove_item(typ.clone(), count)?;
                            if let Some(inv) = obj.warehouse.equipment.get_mut_cow(&typ) {
                                inv.stored = whcnt - count;
                            }
                        }
                    }
                    maybe_mut!(obj.warehouse.equipment, sifo.typ.0, "equip")?.stored =
                        wh.get_item_count(sifo.typ.0.clone()).with_context(|| {
                            format_compact!("getting warehouse count for {}", sifo.typ.0)
                        })?;
                    Ok(())
                };
                if let Err(e) = adjust_warehouse() {
                    error!("couldn't adjust warehouse {:?}", e)
                }
                let player = maybe_mut!(self.persisted.players, ucid, "player")?;
                let life_typ = self.ephemeral.cfg.life_types[&sifo.typ];
                match player.lives.get(&life_typ) {
                    Some((_, n)) if *n == 0 => self.ephemeral.force_player_to_spectators(&ucid),
                    None | Some((_, _)) => (),
                }
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
            self.ephemeral.units_able_to_move.swap_remove(&uid);
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

    pub fn award_kill_points(&mut self, cfg: PointsCfg, dead: Dead) {
        let mut hit_by: SmallVec<[&Ucid; 4]> = smallvec![];
        for shot in &dead.shots {
            if shot.hit {
                if !hit_by.contains(&&shot.shooter_ucid) {
                    hit_by.push(&shot.shooter_ucid)
                }
            }
        }
        if hit_by.is_empty() {
            for shot in &dead.shots {
                if dead.time - shot.time <= Duration::minutes(3) {
                    if !hit_by.contains(&&shot.shooter_ucid) {
                        hit_by.push(&shot.shooter_ucid)
                    }
                }
            }
        }
        if !hit_by.is_empty() {
            let total_points = if dead.victim_ucid.is_some() {
                cfg.air_kill
            } else {
                (&dead.shots)
                    .into_iter()
                    .find(|s| s.target_typ.trim() != "")
                    .map(|s| &s.target_typ)
                    .and_then(|typ| self.ephemeral.cfg.unit_classification.get(typ.as_str()))
                    .map(|tags| {
                        if tags.contains(UnitTag::LR | UnitTag::TrackRadar | UnitTag::SAM) {
                            cfg.ground_kill + cfg.lr_sam_bonus
                        } else if tags.contains(UnitTag::Aircraft)
                            || tags.contains(UnitTag::Helicopter)
                        {
                            cfg.air_kill
                        } else {
                            cfg.ground_kill
                        }
                    })
                    .unwrap_or(cfg.ground_kill)
            };
            let pps = (total_points as f32 / hit_by.len() as f32).ceil() as i32;
            let victim_info = dead
                .victim_ucid
                .as_ref()
                .and_then(|i| self.persisted.players.get(i))
                .map(|p| (p.name.clone(), p.airborne.unwrap_or(LifeType::Standard)));
            for ucid in hit_by {
                if let Some(player) = self.persisted.players.get_mut_cow(ucid) {
                    let msg = if player.side != dead.victim_side {
                        player.points += pps;
                        let tp = player.points;
                        match &victim_info {
                            None => format_compact!("{tp}(+{pps}) points"),
                            Some((victim, _)) => {
                                format_compact!("{tp}(+{pps}) points, killed {}", victim)
                            }
                        }
                    } else {
                        match &victim_info {
                            None => {
                                player.points -= total_points as i32;
                                let tp = player.points;
                                format_compact!(
                                    "{tp}(-{total_points}) points, you have killed a friendly unit"
                                )
                            }
                            Some((victim, life_type)) => {
                                let (_, player_lives) =
                                    player.lives.get_or_insert_cow(*life_type, || {
                                        (Utc::now(), self.ephemeral.cfg.default_lives[&life_type].0)
                                    });
                                let mut lost = false;
                                if *player_lives > 0 {
                                    lost = true;
                                    *player_lives -= 1;
                                }
                                player.points -= total_points as i32;
                                let tp = player.points;
                                self.ephemeral.dirty();
                                let lost = if lost {
                                    format_compact!("\nYou have lost a {life_type} life")
                                } else {
                                    format_compact!("")
                                };
                                format_compact!("{tp}(-{total_points}) points, you have team killed {victim}.{}", lost)
                            }
                        }
                    };
                    self.ephemeral.panel_to_player(&self.persisted, &ucid, msg)
                }
            }
        }
    }

    pub fn adjust_points(&mut self, ucid: &Ucid, amount: i32, why: &str) {
        if let Some(player) = self.persisted.players.get_mut_cow(ucid) {
            player.points += amount;
            let pp = player.points;
            if amount != 0 {
                let m = format_compact!("{}({}) points {}", pp, amount, why);
                self.ephemeral.panel_to_player(&self.persisted, ucid, m);
                self.ephemeral.dirty();
            }
        }
    }
}
