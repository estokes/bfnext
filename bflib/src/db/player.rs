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

use super::{ephemeral::SlotInfo, group::DeployKind, Db, MapS, SetS};
use crate::{maybe, maybe_mut, objective_mut};
use anyhow::{anyhow, bail, Context, Result};
use bfprotocols::{
    cfg::{LifeType, PointsCfg, UnitTag, Vehicle},
    db::{group::GroupId, objective::ObjectiveId},
    shots::{Dead, Who},
    stats::{self, EnId, Stat},
};
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use dcso3::{
    airbase::Airbase, coalition::Side, coord::Coord, net::{SlotId, Ucid}, object::{DcsObject, DcsOid}, unit::{ClassUnit, Unit}, value_to_json, MizLua, Position3, String, Vector2, Vector3
};
use log::{debug, error, info, warn};
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::cmp::{max, min};

struct VictimInfo {
    ucid: Ucid,
    name: String,
    ai_deployable: bool,
    life_type: Option<LifeType>,
}

#[derive(Debug, Clone)]
pub enum SlotAuth {
    Yes(Option<stats::Unit>),
    ObjectiveNotOwned(Side),
    ObjectiveHasNoLogistics,
    NoLives(LifeType),
    NoPoints {
        vehicle: Vehicle,
        cost: u32,
        balance: i32,
    },
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
    OutOfPoints,
}

#[derive(Debug, Clone, Default)]
pub struct InstancedPlayer {
    pub position: Position3,
    pub velocity: Vector3,
    pub typ: Vehicle,
    pub in_air: bool,
    pub landed_at_objective: Option<ObjectiveId>,
    pub stopped_at_objective: bool,
    pub moved: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    pub alts: SetS<String>,
    pub side: Side,
    pub side_switches: Option<u8>,
    pub lives: MapS<LifeType, (DateTime<Utc>, u8)>,
    pub crates: SetS<GroupId>,
    #[serde(default)]
    pub airborne: Option<LifeType>,
    #[serde(default)]
    pub points: i32,
    #[serde(default)]
    pub ai_team_kills: SetS<DateTime<Utc>>,
    #[serde(default)]
    pub player_team_kills: MapS<DateTime<Utc>, Ucid>,
    #[serde(skip)]
    pub current_slot: Option<(SlotId, Option<InstancedPlayer>)>,
    #[serde(skip)]
    pub changing_slots: bool,
    #[serde(skip)]
    pub jtac_or_spectators: bool,
    #[serde(skip)]
    pub provisional_points: i32,
}

impl Db {
    pub fn player_deslot(&mut self, ucid: &Ucid) {
        if let Some(player) = self.persisted.players.get_mut_cow(ucid) {
            player.airborne = None;
            player.provisional_points = 0;
            if let Some((slot, _)) = player.current_slot.take() {
                let _ = self
                    .ephemeral
                    .player_deslot(&self.persisted, &slot, Some(*ucid));
            }
            self.ephemeral.stat(Stat::Deslot { id: *ucid });
            self.ephemeral.dirty()
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
        let sp_name = sp.name.clone();
        match self.persisted.players.get_mut_cow(target) {
            Some(tp) => {
                tp.points += amount as i32;
                let msg = format_compact!(
                    "{}(+{}) you received points from {}",
                    tp.points,
                    amount,
                    sp_name
                );
                self.ephemeral
                    .panel_to_player(&self.persisted, 10, target, msg);
                self.ephemeral.stat(Stat::PointsTransfer {
                    from: *source,
                    to: *target,
                    points: amount,
                });
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
        maybe_mut!(self.persisted.players, ucid, "player")?.lives = MapS::new();
        self.ephemeral.stat(Stat::Life {
            id: *ucid,
            lives: MapS::new(),
        });
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
                                origin: _,
                            } => Some(*player),
                            DeployKind::Action { player, .. } => player.clone(),
                            DeployKind::Crate { .. } | DeployKind::Objective => None,
                        })
                }
            }
        }
    }

    fn compute_flight_cost(&self, sifo: &SlotInfo, unit: &Unit) -> Result<(u32, bool, String)> {
        use std::fmt::Write;
        let mut m = String::from("");
        match self.ephemeral.cfg.points.as_ref() {
            None => Ok((0, false, m)),
            Some(points) => {
                let mut cost = *points.airframe_cost.get(&sifo.typ).unwrap_or(&0);
                write!(m, "{cost} for {}", sifo.typ).unwrap();
                if !points.weapon_cost.is_empty() {
                    for ammo in unit.get_ammo().context("getting ammo")? {
                        let ammo = ammo.context("unwrapping ammo")?;
                        let typ = ammo.type_name().context("getting ammo type name")?;
                        info!("ammo of type {typ} loaded");
                        if let Some(unit_cost) = points.weapon_cost.get(&typ) {
                            let n = ammo.count().context("getting ammo count")?;
                            let wcost = n * (*unit_cost);
                            write!(m, ", {wcost} for {n}x{typ}").unwrap();
                            cost += wcost;
                        }
                    }
                }
                Ok((cost, points.strict, m))
            }
        }
    }

    pub fn takeoff(
        &mut self,
        time: DateTime<Utc>,
        slot: SlotId,
        unit: &Unit,
        position: Vector2,
    ) -> Result<TakeoffRes> {
        let sifo = self
            .ephemeral
            .slot_info
            .get(&slot)
            .ok_or_else(|| anyhow!("could not find slot {:?}", slot))?;
        let (cost, strict, cost_msg) = match self.compute_flight_cost(&sifo, unit) {
            Ok(cost) => cost,
            Err(e) => {
                error!("failed to compute flight cost {e:?}");
                (0, false, String::from(""))
            }
        };
        let (ucid, player) = self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid).map(|p| (*ucid, p)))
            .ok_or_else(|| anyhow!("could not find player in slot {:?}", slot))?;
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
                res || (obj.owner == player.side && obj.zone.contains(position))
            });
        let res = if strict && cost as i32 > player.points {
            return Ok(TakeoffRes::OutOfPoints);
        } else if !self.ephemeral.cfg.limited_lives {
            player.airborne = Some(life_type);
            self.ephemeral.dirty();
            Ok(TakeoffRes::NoLifeTaken)
        } else if is_on_owned_objective {
            // paranoia
            if *player_lives == 0 {
                return Ok(TakeoffRes::OutOfLives);
            } else {
                player.airborne = Some(life_type);
                *player_lives -= 1;
                self.ephemeral.stat(Stat::Life {
                    id: ucid,
                    lives: player.lives.clone(),
                });
            }
            self.ephemeral.dirty();
            Ok(TakeoffRes::TookLife(life_type))
        } else {
            Ok(TakeoffRes::NoLifeTaken)
        };
        if cost > 0 {
            self.adjust_points(&ucid, -(cost as i32), cost_msg.as_str());
            self.ephemeral.dirty();
        }
        self.ephemeral.stat(Stat::Takeoff { id: ucid });
        res
    }

    pub fn land(&mut self, slot: SlotId, position: Vector2, unit: &Unit) -> Option<LifeType> {
        let sifo = match self.ephemeral.slot_info.get(&slot) {
            Some(sifo) => sifo,
            None => return None,
        };
        let (cost, _, cost_msg) = match self.compute_flight_cost(&sifo, unit) {
            Ok(cost) => cost,
            Err(e) => {
                error!("failed to compute flight cost {e:?}");
                (0, false, String::from(""))
            }
        };
        let (ucid, player) = match self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid).map(|p| (*ucid, p)))
        {
            Some(player) => player,
            None => return None,
        };
        let life_type = self.ephemeral.cfg.life_types[&sifo.typ];
        let (_, player_lives) = match player.lives.get_mut_cow(&life_type) {
            Some(l) => l,
            None => return None,
        };
        let on_owned_objective = self
            .persisted
            .objectives
            .into_iter()
            .find_map(|(oid, obj)| {
                if obj.owner == player.side && obj.zone.contains(position) {
                    Some(*oid)
                } else {
                    None
                }
            });
        self.ephemeral.stat(Stat::Land { id: ucid });
        if let Some(oid) = on_owned_objective {
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
            let lives = player.lives.clone();
            if let Some(points) = self.ephemeral.cfg.points.as_ref() {
                let is_provisional = points.provisional;
                let provisional_points = player.provisional_points;
                player.provisional_points = 0;
                if cost > 0 {
                    self.adjust_points(&ucid, cost as i32, cost_msg.as_str());
                }
                if is_provisional && provisional_points > 0 {
                    self.adjust_points(
                        &ucid,
                        provisional_points as i32,
                        "provisional points committed",
                    );
                }
            }
            self.ephemeral.dirty();
            if !self.ephemeral.cfg.limited_lives {
                None
            } else {
                self.ephemeral.stat(Stat::Life { id: ucid, lives });
                Some(life_type)
            }
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
        let mut reset = false;
        for lt in lt_to_reset {
            player.lives.remove_cow(&lt);
            reset = true;
            self.ephemeral.dirty();
        }
        if reset {
            self.ephemeral.stat(Stat::Life {
                id: *ucid,
                lives: player.lives.clone(),
            });
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
                    return SlotAuth::Yes(None);
                }
                return SlotAuth::NotRegistered(slot_side);
            }
        };
        if slot.is_spectator() {
            player.jtac_or_spectators = true;
            return SlotAuth::Yes(None);
        }
        if slot_side != player.side {
            if self.ephemeral.cfg.lock_sides {
                return SlotAuth::ObjectiveNotOwned(player.side);
            } else {
                player.side = slot_side;
            }
        }
        match slot {
            SlotId::Spectator => unreachable!(),
            SlotId::Instructor(_, _) => {
                if self.ephemeral.cfg.admins.contains_key(ucid) {
                    player.jtac_or_spectators = true;
                    SlotAuth::Yes(None)
                } else {
                    SlotAuth::Denied
                }
            }
            SlotId::ArtilleryCommander(_, _)
            | SlotId::ForwardObserver(_, _)
            | SlotId::Observer(_, _) => {
                if self.ephemeral.cfg.rules.ca.check(ucid) {
                    player.jtac_or_spectators = true;
                    SlotAuth::Yes(None)
                } else {
                    SlotAuth::Denied
                }
            }
            SlotId::Unit(_) | SlotId::MultiCrew(_, _) => {
                if self.ephemeral.slot_info.contains_key(&slot) {
                    self.try_occupy_slot_deferred(time, ucid, slot)
                } else {
                    player.changing_slots = true;
                    player.jtac_or_spectators = false;
                    return SlotAuth::Yes(None);
                }
            }
        }
    }

    pub fn try_occupy_slot_deferred(
        &mut self,
        time: DateTime<Utc>,
        ucid: &Ucid,
        slot: SlotId,
    ) -> SlotAuth {
        let sifo = match self.ephemeral.slot_info.get(&slot) {
            None => return SlotAuth::Denied,
            Some(sifo) => sifo,
        };
        let player = match self.persisted.players.get_mut_cow(ucid) {
            Some(player) => player,
            None => {
                if slot.is_spectator() {
                    return SlotAuth::Yes(None);
                }
                return SlotAuth::NotRegistered(sifo.side);
            }
        };
        let objective = match self.persisted.objectives.get(&sifo.objective) {
            Some(o) if o.owner != Side::Neutral => o,
            Some(_) | None => return SlotAuth::ObjectiveNotOwned(player.side),
        };
        if objective.owner != player.side {
            return SlotAuth::ObjectiveNotOwned(player.side);
        }
        if objective.captureable() {
            return SlotAuth::ObjectiveHasNoLogistics;
        }
        let life_type = self.ephemeral.cfg.life_types[&sifo.typ];
        macro_rules! yes {
            () => {
                match objective.warehouse.equipment.get(sifo.typ.as_str()) {
                    Some(inv) if inv.stored > 0 => (),
                    Some(_) | None => break SlotAuth::VehicleNotAvailable(sifo.typ.clone()),
                }
                player.changing_slots = false;
                player.jtac_or_spectators = false;
                break SlotAuth::Yes(Some(stats::Unit {
                    typ: sifo.typ.clone(),
                    tags: self
                        .ephemeral
                        .cfg
                        .unit_classification
                        .get(&sifo.typ)
                        .map(|t| *t)
                        .unwrap_or_default(),
                }));
            };
        }
        if let Some(points) = self.ephemeral.cfg.points.as_ref() {
            let cost = *points.airframe_cost.get(&sifo.typ).unwrap_or(&0);
            if cost > 0 && player.points < cost as i32 {
                return SlotAuth::NoPoints {
                    cost,
                    vehicle: sifo.typ.clone(),
                    balance: player.points,
                };
            }
        }
        loop {
            match player.lives.get(&life_type).map(|t| *t) {
                None => {
                    yes!();
                }
                Some((reset, n)) => {
                    let reset_after =
                        Duration::seconds(self.ephemeral.cfg.default_lives[&life_type].1 as i64);
                    if time - reset >= reset_after {
                        player.lives.remove_cow(&life_type);
                        self.ephemeral.stat(Stat::Life {
                            id: *ucid,
                            lives: player.lives.clone(),
                        });
                        self.ephemeral.dirty = true;
                    } else if n == 0 {
                        break SlotAuth::NoLives(life_type);
                    }
                    yes!();
                }
            }
        }
    }

    pub fn player_connected(&mut self, ucid: Ucid, name: String) {
        if let Some(player) = self.persisted.players.get(&ucid) {
            if player.current_slot.is_some() {
                self.player_deslot(&ucid)
            }
        }
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
                let points = self
                    .ephemeral
                    .cfg
                    .points
                    .as_ref()
                    .map(|p| p.new_player_join as i32)
                    .unwrap_or(0);
                self.persisted.players.insert_cow(
                    ucid,
                    Player {
                        name: name.clone(),
                        alts: SetS::from_iter([name.clone()]),
                        side,
                        side_switches: self.ephemeral.cfg.side_switches,
                        lives: MapS::new(),
                        crates: SetS::new(),
                        airborne: None,
                        points,
                        provisional_points: 0,
                        current_slot: None,
                        changing_slots: false,
                        jtac_or_spectators: true,
                        ai_team_kills: SetS::new(),
                        player_team_kills: MapS::new(),
                    },
                );
                self.ephemeral.stat(Stat::Register {
                    initial_points: points,
                    name,
                    side,
                    id: ucid,
                });
                self.ephemeral.dirty();
                Ok(())
            }
        }
    }

    pub fn force_sideswitch_player(&mut self, ucid: &Ucid, side: Side) -> Result<()> {
        let player = maybe_mut!(self.persisted.players, ucid, "no such player")?;
        player.side = side;
        self.ephemeral.stat(Stat::Sideswitch { id: *ucid, side });
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
                    self.ephemeral.stat(Stat::Sideswitch { id: *ucid, side });
                    self.ephemeral.dirty();
                    Ok(())
                }
            }
        }
    }

    pub fn update_player_positions<'a>(
        &mut self,
        lua: MizLua,
        now: DateTime<Utc>,
        ids: impl IntoIterator<Item = &'a Ucid>,
    ) -> Result<Vec<DcsOid<ClassUnit>>> {
        let mut dead: Vec<DcsOid<ClassUnit>> = vec![];
        let mut unit: Option<Unit> = None;
        let coord = Coord::singleton(lua)?;
        for ucid in ids {
            let mut inform_cost = None;
            if let Some(player) = self.persisted.players.get_mut_cow(ucid) {
                if let Some((slot, Some(inst))) = &mut player.current_slot {
                    if let Some(id) = self.ephemeral.object_id_by_slot.get(slot) {
                        let instance = match unit.take() {
                            Some(unit) => unit.change_instance(id),
                            None => Unit::get_instance(lua, id),
                        };
                        match instance {
                            Ok(instance) => {
                                let pos = instance.get_position()?;
                                if (inst.position.p.0 - pos.p.0).magnitude_squared() > 1.0 {
                                    if inst.stopped_at_objective {
                                        inform_cost = Some((*slot, player.points));
                                    }
                                    inst.stopped_at_objective = false;
                                    inst.position = pos;
                                    inst.velocity = instance.get_velocity()?.0;
                                    inst.in_air = instance.in_air()?;
                                    inst.moved = Some(now);
                                } else if inst.landed_at_objective.is_some() {
                                    inst.stopped_at_objective = true;
                                }
                                unit = Some(instance);
                                self.ephemeral.stat(Stat::Position {
                                    id: EnId::Player(*ucid),
                                    pos: stats::Pos {
                                        pos: coord.lo_to_ll(inst.position.p)?,
                                        velocity: inst.velocity,
                                    },
                                });
                            }
                            Err(e) => {
                                warn!(
                                    "updating player positions, skipping invalid unit {ucid:?}, {id:?}, player {e:?}",
                                );
                                // dead.push(id.clone())
                            }
                        }
                    }
                }
            }
            if let (Some(unit), Some((slot, balance))) = (&unit, inform_cost) {
                let sifo = self
                    .ephemeral
                    .slot_info
                    .get(&slot)
                    .ok_or_else(|| anyhow!("could not find slot {:?}", slot))?;
                let (cost, strict, cost_msg) = self.compute_flight_cost(sifo, &unit)?;
                if cost > 0 {
                    let m = if strict && cost as i32 > balance {
                        format_compact!(
                            "Your flight will cost {cost}, and you have {balance}. {cost_msg}"
                        )
                    } else {
                        format_compact!("Your flight will cost {cost}. {cost_msg}")
                    };
                    self.ephemeral.panel_to_player(&self.persisted, 60, ucid, m)
                }
            }
        }
        Ok(dead)
    }

    pub fn update_player_positions_incremental(
        &mut self,
        lua: MizLua,
        now: DateTime<Utc>,
        i: usize,
    ) -> Result<(usize, Vec<DcsOid<ClassUnit>>)> {
        let total = self.ephemeral.players_by_slot.len();
        if i < total {
            let stop = min(total, i + max(1, total / 10));
            let players: SmallVec<[Ucid; 64]> = self.ephemeral.players_by_slot.as_slice()[i..stop]
                .into_iter()
                .map(|(_, ucid)| *ucid)
                .collect();
            let dead = self.update_player_positions(lua, now, &players)?;
            Ok((stop, dead))
        } else {
            Ok((0, vec![]))
        }
    }

    pub fn player_entered_slot(
        &mut self,
        lua: MizLua,
        id: DcsOid<ClassUnit>,
        unit: &Unit,
        slot: SlotId,
        oid: ObjectiveId,
        ucid: Ucid,
    ) -> Result<()> {
        if let Some(old_ucid) = self.ephemeral.players_by_slot.get(&slot) {
            let old_ucid = *old_ucid;
            if old_ucid != ucid {
                self.player_deslot(&old_ucid)
            }
        }
        let obj = objective_mut!(self, oid)?;
        let sifo = maybe!(self.ephemeral.slot_info, slot, "slot")?;
        let player = maybe!(self.persisted.players, ucid, "player")?;
        let life_typ = self.ephemeral.cfg.life_types[&sifo.typ];
        match player.lives.get(&life_typ) {
            Some((_, n)) if *n == 0 => {
                info!("player {ucid} has no lives for this unit type");
                self.player_deslot(&ucid);
                unit.clone().destroy()?;
                return Ok(());
            }
            None | Some((_, _)) => (),
        }
        self.ephemeral.players_by_slot.insert(slot, ucid);
        self.ephemeral
            .slot_by_object_id
            .insert(id.clone(), slot.clone());
        self.ephemeral
            .object_id_by_slot
            .insert(slot.clone(), id.clone());
        let mut adjust_warehouse = || -> Result<()> {
            let id = maybe!(self.ephemeral.airbase_by_oid, obj.id, "airbase")?;
            let wh = Airbase::get_instance(lua, id)
                .context("getting airbase")?
                .get_warehouse()
                .context("getting warehouse")?;
            if sifo.ground_start {
                wh.remove_item(sifo.typ.0.clone(), 1)
                    .with_context(|| format_compact!("removing {} from warehouse", sifo.typ.0))?;
                for wep in unit.get_ammo()? {
                    let wep = wep?;
                    let count = wep.count()?;
                    let typ = wep.type_name()?;
                    let whcnt = wh.get_item_count(typ.clone())?;
                    debug!("removing {count} {typ} from the warehouse which contains {whcnt}");
                    wh.remove_item(typ.clone(), count)?;
                    if let Some(inv) = obj.warehouse.equipment.get_mut_cow(&typ) {
                        inv.stored = whcnt - count;
                    }
                }
            }
            maybe_mut!(obj.warehouse.equipment, sifo.typ.0, "equip")?.stored = wh
                .get_item_count(sifo.typ.0.clone())
                .with_context(|| format_compact!("getting warehouse count for {}", sifo.typ.0))?;
            Ok(())
        };
        if let Err(e) = adjust_warehouse() {
            error!("couldn't adjust warehouse {:?}", e)
        }
        let player = maybe_mut!(self.persisted.players, ucid, "player")?;
        let position = unit.get_position()?;
        let point = Vector2::new(position.p.x, position.p.z);
        let landed_at_objective = self
            .persisted
            .objectives
            .into_iter()
            .find(|(_, obj)| obj.zone.contains(point))
            .map(|(oid, _)| *oid);
        player.current_slot = Some((
            slot,
            Some(InstancedPlayer {
                position,
                velocity: unit.get_velocity()?.0,
                in_air: unit.in_air()?,
                typ: Vehicle::from(unit.get_type_name()?),
                landed_at_objective,
                stopped_at_objective: true,
                moved: None,
            }),
        ));
        player.changing_slots = false;
        player.provisional_points = 0;
        self.ephemeral.dirty();
        Ok(())
    }

    pub fn player_left_unit(
        &mut self,
        lua: MizLua,
        now: DateTime<Utc>,
        unit: &Unit,
    ) -> Result<Vec<DcsOid<ClassUnit>>> {
        let name = unit.get_name()?;
        let mut dead = vec![];
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            let uid = *uid;
            match self.update_unit_positions(lua, now, &[uid]) {
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
                if let Some((_, Some(inst))) = player.current_slot.as_mut() {
                    let typ = inst.typ.clone();
                    if let Some(oid) = inst.landed_at_objective {
                        let mut fix_warehouse = || -> Result<()> {
                            let obj = objective_mut!(self, oid).context("get objective")?;
                            let id = maybe!(self.ephemeral.airbase_by_oid, oid, "airbase")?;
                            let airbase = Airbase::get_instance(lua, &id).context("get airbase")?;
                            let wh = airbase.get_warehouse().context("get warehouse")?;
                            let airbase = obj.kind.is_airbase()
                                || self
                                    .ephemeral
                                    .cfg
                                    .extra_fixed_wing_objectives
                                    .contains(obj.name());
                            let mut sync: SmallVec<[String; 4]> = smallvec![typ.0.clone()];
                            if !airbase {
                                wh.add_item(typ.0.clone(), 1)?;
                                for ammo in unit.get_ammo().context("get ammo")? {
                                    let ammo = ammo.context("ammo")?;
                                    let count = ammo.count().context("ammo count")?;
                                    let typ = ammo.type_name().context("ammo typ")?;
                                    sync.push(typ.clone());
                                    wh.add_item(typ, count).context("add item to warehouse")?;
                                }
                            }
                            for typ in sync {
                                if let Some(inv) = obj.warehouse.equipment.get_mut_cow(&typ) {
                                    inv.stored = wh.get_item_count(typ).context("getting item")?;
                                    self.ephemeral.dirty();
                                }
                            }
                            Ok(())
                        };
                        if let Err(e) = fix_warehouse() {
                            error!("unable to fix warehouse {:?}", e)
                        }
                    }
                }
                self.player_deslot(&ucid)
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
        self.ephemeral.stat(Stat::Disconnect { id: *ucid });
        self.player_deslot(ucid);
    }

    fn apply_teamkill_penalty(
        &mut self,
        shooter: Ucid,
        total_points: u32,
        victim_info: &Option<VictimInfo>,
    ) -> CompactString {
        let player = &mut self.persisted.players[&shooter];
        let window = self
            .ephemeral
            .cfg
            .points
            .as_ref()
            .map(|p| p.tk_window as i64)
            .unwrap_or(0);
        let now = Utc::now();
        match victim_info.as_ref() {
            None => {
                let penalty: u32 = player
                    .ai_team_kills
                    .into_iter()
                    .map(|ts| total_points >> ((now - *ts).num_hours() / window))
                    .sum();
                let total_points = total_points + penalty;
                player.points -= total_points as i32;
                player.ai_team_kills.insert_cow(now);
                let tp = player.points;
                format_compact!("{tp}(-{total_points}) points, you have killed a friendly unit")
            }
            Some(VictimInfo {
                name,
                life_type: None,
                ai_deployable: false,
                ucid: _,
            }) => {
                player.points -= total_points as i32;
                format_compact!(
                    "{}(-{total_points})you have team killed {name} on the ground",
                    player.points
                )
            }
            Some(VictimInfo {
                name,
                life_type: None,
                ai_deployable: true,
                ucid: _,
            }) => {
                player.points -= total_points as i32;
                format_compact!(
                    "{}(-{total_points})you have team killed {name}'s ai unit",
                    player.points
                )
            }
            Some(VictimInfo {
                ucid,
                name,
                life_type: Some(life_type),
                ..
            }) => {
                let (penalty_points, penalty_lives): (u32, f32) = player
                    .player_team_kills
                    .into_iter()
                    .fold((total_points, 1.), |(pp, pl), (ts, _)| {
                        let windows = (now - *ts).num_hours() / window;
                        let pp = pp + (total_points >> windows);
                        let pl = pl + (1. / (max(1, windows * 2) as f32));
                        (pp, pl)
                    });
                let deplane_possible = penalty_lives > 1.5;
                let mut penalty_lives = penalty_lives.round() as u32;
                let mut lost: SmallVec<[(LifeType, u8); 5]> = smallvec![];
                let mut life_type = *life_type;
                let deplane = loop {
                    let (_, player_lives) = player.lives.get_or_insert_cow(life_type, || {
                        (Utc::now(), self.ephemeral.cfg.default_lives[&life_type].0)
                    });
                    if *player_lives as u32 >= penalty_lives {
                        lost.push((life_type, penalty_lives as u8));
                        *player_lives -= penalty_lives as u8;
                        break false;
                    } else {
                        if *player_lives > 0 {
                            lost.push((life_type, *player_lives));
                        }
                        penalty_lives -= *player_lives as u32;
                        *player_lives = 0;
                        match life_type.up() {
                            None => break deplane_possible,
                            Some(lt) => {
                                life_type = lt;
                            }
                        }
                    }
                };
                self.ephemeral.stat(Stat::Life {
                    id: shooter,
                    lives: player.lives.clone(),
                });
                player.points -= penalty_points as i32;
                player.player_team_kills.insert_cow(now, *ucid);
                let tp = player.points;
                self.ephemeral.dirty();
                use std::fmt::Write;
                let mut msg = CompactString::from("");
                write!(
                    msg,
                    "{tp}(-{penalty_points}) points, you have team killed {name}.\n",
                )
                .unwrap();
                if lost.len() > 0 {
                    write!(msg, "\nYou have lost\n").unwrap();
                    for (ty, n) in lost {
                        write!(msg, "{n} {ty} life\n").unwrap()
                    }
                };
                if deplane {
                    write!(msg, "Shortly you will be deplaned\n").unwrap();
                    write!(
                        msg,
                        "your death may be monitored for quality assurance purposes\n"
                    )
                    .unwrap();
                    write!(msg, "have a nice day").unwrap();
                    self.ephemeral
                        .force_player_to_spectators_at(&shooter, now + Duration::seconds(30));
                }
                msg
            }
        }
    }

    pub fn award_kill_points(&mut self, cfg: &PointsCfg, dead: &Dead) {
        let mut hit_by: SmallVec<[(Ucid, bool); 16]> = smallvec![];
        let valid_shots = || {
            // why are you hitting yourself
            dead.shots
                .iter()
                .filter(|shot| match (&shot.shooter, &shot.target) {
                    (Who::AI { gid: g0, .. }, Who::AI { gid: g1, .. }) => g0 != g1,
                    (Who::Player { ucid: u0, .. }, Who::Player { ucid: u1, .. }) => u0 != u1,
                    (
                        Who::AI {
                            ucid: Some(u0),
                            side: s0,
                            ..
                        },
                        Who::Player {
                            side: s1, ucid: u1, ..
                        },
                    ) => u0 != u1 && s0 != s1,
                    (Who::Player { ucid: u1, .. }, Who::AI { ucid: Some(u0), .. }) => u0 != u1,
                    (Who::AI { .. }, Who::Player { .. }) | (Who::Player { .. }, Who::AI { .. }) => {
                        true
                    }
                })
        };
        for shot in valid_shots() {
            let k = match shot.shooter {
                Who::Player { ucid, .. } => (ucid, cfg.provisional),
                Who::AI { ucid, .. } => match ucid {
                    Some(ucid) => (ucid, false),
                    None => continue,
                },
            };
            if shot.hit && !hit_by.contains(&k) {
                hit_by.push(k)
            }
        }
        if hit_by.is_empty() {
            for shot in valid_shots() {
                let k = match shot.shooter {
                    Who::Player { ucid, .. } => (ucid, cfg.provisional),
                    Who::AI { ucid, .. } => match ucid {
                        Some(ucid) => (ucid, false),
                        None => continue,
                    },
                };
                if dead.time - shot.time <= Duration::minutes(3) && !hit_by.contains(&k) {
                    hit_by.push(k);
                }
            }
        }
        if !hit_by.is_empty() {
            let total_points = (&dead.shots)
                .into_iter()
                .find(|s| s.target_typ.trim() != "")
                .map(|s| &s.target_typ)
                .and_then(|typ| self.ephemeral.cfg.unit_classification.get(typ.as_str()))
                .map(|tags| {
                    if tags.contains(UnitTag::LR | UnitTag::TrackRadar | UnitTag::SAM) {
                        cfg.ground_kill + cfg.lr_sam_bonus
                    } else if tags.contains(UnitTag::Aircraft) || tags.contains(UnitTag::Helicopter)
                    {
                        cfg.air_kill
                    } else {
                        cfg.ground_kill
                    }
                })
                .unwrap_or(cfg.ground_kill);
            let pps = (total_points as f32 / hit_by.len() as f32).ceil() as i32;
            let victim_info = match &dead.victim {
                Who::Player { ucid, .. } => self.persisted.players.get(ucid).map(|p| VictimInfo {
                    ucid: *ucid,
                    name: p.name.clone(),
                    life_type: p.airborne,
                    ai_deployable: false,
                }),
                Who::AI { ucid: None, .. } => None,
                Who::AI { ucid: Some(i), .. } => {
                    self.persisted.players.get(i).map(|p| VictimInfo {
                        ucid: *i,
                        name: p.name.clone(),
                        life_type: None,
                        ai_deployable: true,
                    })
                }
            };
            for (ucid, provisional) in hit_by {
                if let Some(player) = self.persisted.players.get_mut_cow(&ucid) {
                    let msg = if player.side == *dead.victim.side() {
                        self.apply_teamkill_penalty(ucid, total_points, &victim_info)
                    } else {
                        let tp = if provisional {
                            player.provisional_points += pps;
                            player.provisional_points
                        } else {
                            player.points += pps;
                            player.points
                        };
                        let pm = if provisional { " provisional" } else { "" };
                        match &victim_info {
                            None => format_compact!("{tp}(+{pps}){pm} points"),
                            Some(vi) => {
                                if vi.ai_deployable {
                                    format_compact!(
                                        "{tp}(+{pps}){pm} points, killed {}'s deployed ai unit",
                                        vi.name
                                    )
                                } else {
                                    format_compact!("{tp}(+{pps}){pm} points, killed {}", vi.name)
                                }
                            }
                        }
                    };
                    debug!("{ucid} kill message: {msg}");
                    self.ephemeral
                        .panel_to_player(&self.persisted, 10, &ucid, msg)
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
                self.ephemeral.stat(Stat::Points {
                    points: amount,
                    reason: m.clone().into(),
                    id: *ucid,
                });
                self.ephemeral.panel_to_player(&self.persisted, 10, ucid, m);
                self.ephemeral.dirty();
            }
        }
    }
}
