use super::{Db, DeployKind, GroupId, ObjGroupClass, Objective, ObjectiveId, Set};
use crate::{
    cfg::{Deployable, DeployableLogistics, Vehicle},
    db::{Map, ObjectiveKind},
    group, group_mut, maybe, objective, objective_mut,
    spawnctx::{Despawn, SpawnCtx, SpawnLoc},
    unit, unit_mut,
};
use anyhow::{anyhow, Result};
use chrono::{prelude::*, Duration};
use compact_str::format_compact;
use dcso3::{
    centroid2d,
    coalition::Side,
    coord::Coord,
    env::miz::{GroupKind, MizIndex},
    land::Land,
    LuaVec2, LuaVec3, MizLua, String, Vector2, Vector3,
};
use fxhash::FxHashMap;
use log::{debug, info};
use smallvec::{smallvec, SmallVec};
use std::cmp::max;

impl Db {
    fn compute_objective_status(&self, obj: &Objective) -> Result<(u8, u8)> {
        obj.groups
            .get(&obj.owner)
            .map(|groups| {
                let mut total = 0;
                let mut alive = 0;
                let mut logi_total = 0;
                let mut logi_alive = 0;
                for gid in groups {
                    let group = group!(self, gid)?;
                    let logi = match &group.class {
                        ObjGroupClass::Logi => true,
                        _ => false,
                    };
                    for uid in &group.units {
                        total += 1;
                        if logi {
                            logi_total += 1;
                        }
                        if !unit!(self, uid)?.dead {
                            alive += 1;
                            if logi {
                                logi_alive += 1;
                            }
                        }
                    }
                }
                let health = ((alive as f32 / total as f32) * 100.).trunc() as u8;
                let logi = ((logi_alive as f32 / logi_total as f32) * 100.).trunc() as u8;
                Ok((health, logi))
            })
            .unwrap_or(Ok((100, 100)))
    }

    pub(super) fn delete_objective(&mut self, oid: &ObjectiveId) -> Result<()> {
        let obj = self.persisted.objectives.remove_cow(oid).unwrap();
        self.persisted.objectives_by_name.remove_cow(&obj.name);
        for (_, groups) in &obj.groups {
            for gid in groups {
                self.persisted.objectives_by_group.remove_cow(gid);
                self.delete_group(gid)?;
            }
        }
        for (slot, _) in &obj.slots {
            self.persisted.objectives_by_slot.remove_cow(slot);
        }
        self.persisted.farps.remove_cow(oid);
        if let Some((fid, eid)) = self.ephemeral.objective_marks.remove(oid) {
            self.ephemeral.msgs.delete_mark(fid);
            if let Some(id) = eid {
                self.ephemeral.msgs.delete_mark(id)
            }
        }
        Ok(())
    }

    pub(super) fn add_farp(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        pos: Vector2,
        spec: &Deployable,
        parts: &DeployableLogistics,
    ) -> Result<ObjectiveId> {
        let now = Utc::now();
        let DeployableLogistics {
            pad_template,
            ammo_template,
            fuel_template,
            barracks_template,
        } = parts;
        let location = {
            let mut points: SmallVec<[Vector2; 16]> = smallvec![];
            let core = spctx.get_template_ref(idx, GroupKind::Any, side, &spec.template)?;
            let ammo = spctx.get_template_ref(idx, GroupKind::Any, side, &ammo_template)?;
            let fuel = spctx.get_template_ref(idx, GroupKind::Any, side, &fuel_template)?;
            let barracks = spctx.get_template_ref(idx, GroupKind::Any, side, &barracks_template)?;
            let pad = spctx.get_template_ref(idx, GroupKind::Any, side, &pad_template)?;
            for unit in core
                .group
                .units()?
                .into_iter()
                .chain(ammo.group.units()?.into_iter())
                .chain(fuel.group.units()?.into_iter())
                .chain(barracks.group.units()?.into_iter())
                .chain(pad.group.units()?.into_iter())
            {
                let unit = unit?;
                points.push(unit.pos()?)
            }
            let center = centroid2d(points);
            SpawnLoc::AtPosWithCenter { pos, center }
        };
        let mut groups: Set<GroupId> = Set::new();
        for name in [
            &spec.template,
            &ammo_template,
            &fuel_template,
            &barracks_template,
            &pad_template,
        ] {
            groups.insert_cow(self.add_and_queue_group(
                spctx,
                idx,
                side,
                location.clone(),
                &name,
                DeployKind::Objective,
                Some(now + Duration::seconds(60)),
            )?);
        }
        let name = {
            let coord = Coord::singleton(spctx.lua())?;
            let pos = coord.lo_to_ll(LuaVec3(Vector3::new(pos.x, 0., pos.y)))?;
            let mgrs = coord.ll_to_mgrs(pos.latitude, pos.longitude)?;
            let mut n = 0;
            loop {
                let name = String::from(format_compact!("farp {} {n}", mgrs.utm_zone));
                if self.persisted.objectives_by_name.get(&name).is_none() {
                    break name;
                } else {
                    n += 1
                }
            }
        };
        let obj = Objective {
            id: ObjectiveId::new(),
            name: name.clone(),
            groups: Map::from_iter([(side, groups)]),
            kind: ObjectiveKind::Farp(spec.clone()),
            pos,
            radius: 2000.,
            owner: side,
            slots: Map::new(),
            health: 100,
            logi: 100,
            spawned: true,
            threatened: true,
            last_threatened_ts: now,
            last_change_ts: now,
            needs_mark: false,
        };
        let oid = obj.id;
        for (_, groups) in &obj.groups {
            for gid in groups {
                self.persisted.objectives_by_group.insert_cow(*gid, oid);
            }
        }
        self.persisted.objectives.insert_cow(oid, obj);
        self.persisted.objectives_by_name.insert_cow(name, oid);
        self.ephemeral.dirty = true;
        self.mark_objective(&oid)?;
        Ok(oid)
    }

    pub(super) fn update_objective_status(
        &mut self,
        oid: &ObjectiveId,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let (kind, health, logi) = {
            let obj = objective!(self, oid)?;
            let (health, logi) = self.compute_objective_status(obj)?;
            let obj = objective_mut!(self, oid)?;
            obj.health = health;
            obj.logi = logi;
            obj.last_change_ts = now;
            obj.needs_mark = true;
            (obj.kind.clone(), health, logi)
        };
        if let ObjectiveKind::Farp(_) = &kind {
            if logi == 0 {
                self.delete_objective(oid)?;
            }
        }
        self.ephemeral.dirty = true;
        debug!("objective {oid} health: {}, logi: {}", health, logi);
        Ok(())
    }

    fn repair_objective(
        &mut self,
        idx: &MizIndex,
        spctx: &SpawnCtx,
        oid: ObjectiveId,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let obj = self
            .persisted
            .objectives
            .get(&oid)
            .ok_or_else(|| anyhow!("no such objective {:?}", oid))?;
        if let Some(groups) = obj.groups.get(&obj.owner) {
            let mut damaged_by_class: FxHashMap<ObjGroupClass, Vec<(GroupId, usize)>> =
                groups.into_iter().fold(
                    Ok(FxHashMap::default()),
                    |m: Result<FxHashMap<ObjGroupClass, Vec<(GroupId, usize)>>>, id| {
                        let mut m = m?;
                        let group = group!(self, id)?;
                        let mut damaged = 0;
                        for uid in &group.units {
                            damaged += if unit!(self, uid)?.dead { 1 } else { 0 };
                        }
                        if damaged > 0 {
                            m.entry(group.class).or_default().push((*id, damaged));
                            Ok(m)
                        } else {
                            Ok(m)
                        }
                    },
                )?;
            for class in [
                ObjGroupClass::Logi,
                ObjGroupClass::Sr,
                ObjGroupClass::Aaa,
                ObjGroupClass::Mr,
                ObjGroupClass::Lr,
                ObjGroupClass::Armor,
                ObjGroupClass::Other,
            ] {
                if let Some(groups) = damaged_by_class.get_mut(&class) {
                    groups.sort_by_key(|(_, d)| *d); // pick the most damaged group
                    if let Some((gid, _)) = groups.pop() {
                        let group = group!(self, gid)?;
                        for uid in &group.units {
                            unit_mut!(self, uid)?.dead = false;
                        }
                        if class == ObjGroupClass::Logi || obj.spawned {
                            self.spawn_group(idx, spctx, group)?;
                        }
                        self.update_objective_status(&oid, now)?;
                        self.ephemeral.dirty = true;
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn cull_or_respawn_objectives(
        &mut self,
        lua: MizLua,
    ) -> Result<(SmallVec<[ObjectiveId; 4]>, SmallVec<[ObjectiveId; 4]>)> {
        let now = Utc::now();
        let land = Land::singleton(lua)?;
        let players = self
            .ephemeral
            .players_by_slot
            .iter()
            .filter_map(|(sl, ucid)| {
                let side = self.persisted.players[ucid].side;
                let pos_typ = self.slot_instance_unit(lua, sl).and_then(|u| {
                    let pos = u.get_point()?;
                    let typ = u.get_type_name()?;
                    Ok((pos, Vehicle::from(typ)))
                });
                match pos_typ {
                    Ok((pos, typ)) => Some((side, pos, typ)),
                    Err(e) => {
                        info!(
                            "failed to get position of player {:?} {:?} {:?}",
                            sl, ucid, e
                        );
                        None
                    }
                }
            })
            .collect::<SmallVec<[_; 64]>>();
        let cfg = self.cfg();
        let cull_distance = (cfg.unit_cull_distance as f64).powi(2);
        let mut to_spawn: SmallVec<[ObjectiveId; 8]> = smallvec![];
        let mut to_cull: SmallVec<[ObjectiveId; 8]> = smallvec![];
        let mut threatened: SmallVec<[ObjectiveId; 16]> = smallvec![];
        let mut not_threatened: SmallVec<[ObjectiveId; 16]> = smallvec![];
        for (oid, obj) in &self.persisted.objectives {
            let pos3 = {
                let alt = land.get_height(LuaVec2(obj.pos))?;
                LuaVec3(Vector3::new(obj.pos.x, alt, obj.pos.y))
            };
            let mut spawn = false;
            let mut is_threatened = false;
            for (side, pos, typ) in &players {
                if obj.owner != *side {
                    let threat_dist = (cfg.threatened_distance[typ] as f64).powi(2);
                    let ppos = Vector2::new(pos.x, pos.z);
                    let dist = na::distance_squared(&obj.pos.into(), &ppos.into());
                    if dist <= cull_distance {
                        spawn = true;
                    }
                    if dist <= threat_dist && land.is_visible(pos3, *pos)? {
                        is_threatened = true;
                    }
                }
            }
            if is_threatened {
                threatened.push(*oid);
            } else {
                not_threatened.push(*oid);
            }
            if !obj.spawned && spawn {
                to_spawn.push(*oid);
            } else if obj.spawned && !spawn {
                to_cull.push(*oid);
            }
        }
        let mut became_threatened: SmallVec<[ObjectiveId; 4]> = smallvec![];
        let mut became_clear: SmallVec<[ObjectiveId; 4]> = smallvec![];
        for oid in &threatened {
            let obj = objective_mut!(self, oid)?;
            if !obj.threatened {
                became_threatened.push(*oid);
            }
            obj.threatened = true;
            obj.last_threatened_ts = now;
        }
        let cooldown = Duration::seconds(self.ephemeral.cfg.threatened_cooldown as i64);
        for oid in &not_threatened {
            let obj = objective_mut!(self, oid)?;
            if now - obj.last_threatened_ts >= cooldown {
                if obj.threatened {
                    became_clear.push(*oid);
                }
                obj.threatened = false;
            }
        }
        for oid in to_spawn {
            let obj = objective_mut!(self, oid)?;
            obj.spawned = true;
            for gid in maybe!(&obj.groups, obj.owner, "side group")? {
                if !group!(self, gid)?.class.is_logi() {
                    self.ephemeral.spawnq.push_back(*gid);
                }
            }
        }
        for oid in to_cull {
            let obj = objective_mut!(self, oid)?;
            obj.spawned = false;
            for gid in maybe!(&obj.groups, obj.owner, "side group")? {
                let group = group!(self, gid)?;
                if !group.class.is_logi() {
                    match group.kind {
                        Some(_) => self
                            .ephemeral
                            .despawnq
                            .push_back((*gid, Despawn::Group(group.name.clone()))),
                        None => {
                            for uid in &group.units {
                                let unit = unit!(self, uid)?;
                                self.ephemeral
                                    .despawnq
                                    .push_back((*gid, Despawn::Static(unit.name.clone())))
                            }
                        }
                    }
                }
            }
        }
        Ok((became_threatened, became_clear))
    }

    pub fn repair_one_logi_step(
        &mut self,
        side: Side,
        now: DateTime<Utc>,
        oid: ObjectiveId,
    ) -> Result<()> {
        let obj = objective_mut!(self, oid)?;
        let mut to_repair = None;
        let current_logi = obj.logi as f64 / 100.;
        for gid in maybe!(&obj.groups, &side, "side group")? {
            let group = group_mut!(self, gid)?;
            if group.class.is_logi() {
                if to_repair.is_none() {
                    let len = group.units.len();
                    let cur = (current_logi * len as f64).ceil() as usize;
                    to_repair = Some(cur + max(1, len >> 1));
                }
                if let Some(to_repair) = to_repair.as_mut() {
                    for uid in &group.units {
                        let unit = unit_mut!(self, uid)?;
                        if *to_repair > 0 {
                            *to_repair -= 1;
                            unit.dead = false;
                        } else {
                            unit.dead = true;
                        }
                    }
                    self.ephemeral.spawnq.push_back(*gid);
                }
            }
        }
        self.update_objective_status(&oid, now)
    }

    pub fn maybe_do_repairs(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let spctx = SpawnCtx::new(lua)?;
        let to_repair = self
            .persisted
            .objectives
            .into_iter()
            .filter_map(|(oid, obj)| {
                let logi = obj.logi as f32 / 100.;
                let repair_time = self.ephemeral.cfg.repair_time as f32 / logi;
                if repair_time < i64::MAX as f32 {
                    let repair_time = Duration::seconds(repair_time as i64);
                    if obj.health < 100 && (now - obj.last_change_ts) >= repair_time {
                        Some(*oid)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for oid in to_repair {
            self.repair_objective(idx, &spctx, oid, now)?
        }
        Ok(())
    }

    pub fn capturable_objectives(&self) -> SmallVec<[ObjectiveId; 1]> {
        let mut cap = smallvec![];
        for (oid, obj) in &self.persisted.objectives {
            if obj.captureable() {
                cap.push(*oid)
            }
        }
        cap
    }

    pub fn check_capture(
        &mut self,
        now: DateTime<Utc>,
    ) -> Result<SmallVec<[(Side, ObjectiveId); 1]>> {
        let mut captured: FxHashMap<ObjectiveId, Vec<(Side, GroupId)>> = FxHashMap::default();
        for (oid, obj) in &self.persisted.objectives {
            if obj.captureable() {
                let r2 = obj.radius.powi(2);
                for gid in &self.persisted.troops {
                    let group = group!(self, gid)?;
                    match &group.origin {
                        DeployKind::Troop { spec, .. } if spec.can_capture => {
                            let in_range = group
                                .units
                                .into_iter()
                                .filter_map(|uid| self.persisted.units.get(uid))
                                .any(|u| {
                                    na::distance_squared(&u.pos.into(), &obj.pos.into()) <= r2
                                });
                            if in_range {
                                captured.entry(*oid).or_default().push((group.side, *gid));
                            }
                        }
                        DeployKind::Crate { .. }
                        | DeployKind::Deployed { .. }
                        | DeployKind::Objective
                        | DeployKind::Troop { .. } => (),
                    }
                }
            }
        }
        let mut actually_captured = smallvec![];
        for (oid, gids) in captured {
            let (side, _) = gids.first().unwrap();
            let captured = gids.iter().all(|(s, _)| side == s);
            if captured {
                let obj = objective_mut!(self, &oid)?;
                obj.owner = *side;
                actually_captured.push((*side, oid));
                self.repair_one_logi_step(*side, now, oid)?;
                for (_, gid) in gids {
                    self.delete_group(&gid)?
                }
                self.ephemeral.dirty = true;
            }
        }
        Ok(actually_captured)
    }

    pub fn mark_objective(&mut self, oid: &ObjectiveId) -> Result<()> {
        if let Some((id0, id1)) = self.ephemeral.objective_marks.remove(oid) {
            self.ephemeral.msgs.delete_mark(id0);
            if let Some(id) = id1 {
                self.ephemeral.msgs.delete_mark(id);
            }
        }
        let obj = objective!(self, oid)?;
        let name = &obj.name;
        let logi = obj.logi;
        let owner = obj.owner;
        let cap = if obj.captureable() { " capturable" } else { "" };
        let friendly_msg = match obj.kind {
            ObjectiveKind::Airbase => format_compact!("{name} airbase {owner} logi {logi}{cap}"),
            ObjectiveKind::Fob => format_compact!("{name} fob {owner} logi {logi}{cap}"),
            ObjectiveKind::Farp(_) => format_compact!("{name} farp {owner} logi {logi}{cap}"),
            ObjectiveKind::Logistics => {
                format_compact!("{name} logistics depot {owner} logi {logi}{cap}")
            }
        };
        let enemy_msg = match obj.kind {
            ObjectiveKind::Airbase => Some(format_compact!("{name} airbase {owner}{cap}")),
            ObjectiveKind::Fob => Some(format_compact!("{name} fob {owner}{cap}")),
            ObjectiveKind::Logistics => {
                Some(format_compact!("{name} logistics depot {owner}{cap}"))
            }
            ObjectiveKind::Farp(_) => None,
        };
        let fid = self
            .ephemeral
            .msgs
            .mark_to_side(owner, obj.pos, true, friendly_msg);
        let eid = match enemy_msg {
            None => None,
            Some(msg) => Some(self.ephemeral.msgs.mark_to_side(
                owner.opposite(),
                obj.pos,
                true,
                msg,
            )),
        };
        self.ephemeral.objective_marks.insert(*oid, (fid, eid));
        Ok(())
    }

    pub fn remark_objectives(&mut self) -> Result<()> {
        let objectives = self
            .persisted
            .objectives
            .into_iter()
            .map(|(oid, _)| *oid)
            .collect::<SmallVec<[_; 64]>>();
        for oid in objectives {
            let obj = objective_mut!(self, oid)?;
            if obj.needs_mark {
                obj.needs_mark = false;
                self.mark_objective(&oid)?
            }
        }
        Ok(())
    }
}
