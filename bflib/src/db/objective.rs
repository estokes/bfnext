use super::{logistics::Warehouse, Db, Map, Set, group::{GroupId, DeployKind, UnitId}};
use crate::{
    cfg::{Deployable, DeployableLogistics, UnitTag, Vehicle},
    group, group_mut, maybe, objective, objective_mut,
    spawnctx::{Despawn, SpawnCtx, SpawnLoc},
    unit, unit_mut,
};
use anyhow::{anyhow, Result};
use chrono::{prelude::*, Duration};
use compact_str::format_compact;
use dcso3::{
    atomic_id, centroid2d,
    coalition::Side,
    coord::Coord,
    cvt_err,
    env::miz::{GroupKind, MizIndex},
    land::Land,
    net::SlotId,
    LuaVec2, LuaVec3, MizLua, String, Vector2, Vector3, azumith2d_to,
};
use fxhash::{FxHashMap, FxHashSet};
use log::{debug, error};
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::{cmp::max, str::FromStr};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ObjectiveKind {
    Airbase,
    Fob,
    Logistics,
    Farp(Deployable),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum ObjGroupClass {
    Logi,
    Aaa,
    Lr,
    Mr,
    Sr,
    Armor,
    Other,
}

impl ObjGroupClass {
    pub fn is_logi(&self) -> bool {
        match self {
            Self::Logi => true,
            Self::Aaa | Self::Lr | Self::Mr | Self::Sr | Self::Armor | Self::Other => false,
        }
    }
}

impl From<&str> for ObjGroupClass {
    fn from(value: &str) -> Self {
        match value {
            "BLOGI" | "RLOGI" | "NLOGI" | "LOGI" => ObjGroupClass::Logi,
            s => {
                if s.starts_with("BAAA")
                    || s.starts_with("RAAA")
                    || s.starts_with("NAAA")
                    || s.starts_with("AAA")
                {
                    ObjGroupClass::Aaa
                } else if s.starts_with("BLR")
                    || s.starts_with("RLR")
                    || s.starts_with("NLR")
                    || s.starts_with("LR")
                {
                    ObjGroupClass::Lr
                } else if s.starts_with("BMR")
                    || s.starts_with("RMR")
                    || s.starts_with("NMR")
                    || s.starts_with("MR")
                {
                    ObjGroupClass::Mr
                } else if s.starts_with("BSR")
                    || s.starts_with("RSR")
                    || s.starts_with("NSR")
                    || s.starts_with("SR")
                {
                    ObjGroupClass::Sr
                } else if s.starts_with("BARMOR")
                    || s.starts_with("RARMOR")
                    || s.starts_with("NARMOR")
                    || s.starts_with("ARMOR")
                {
                    ObjGroupClass::Armor
                } else {
                    ObjGroupClass::Other
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ObjGroup(String);

impl FromStr for ObjGroup {
    type Err = LuaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(String::from(s)))
    }
}

impl<'lua> FromLua<'lua> for ObjGroup {
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::String(s) => s.to_str()?.parse(),
            _ => Err(cvt_err("ObjGroup")),
        }
    }
}

impl ObjGroup {
    pub(super) fn template(&self, side: Side) -> (Side, String) {
        let s = match self.0.rsplit_once("-") {
            Some((l, _)) => l,
            None => self.0.as_str(),
        };
        if s.starts_with("R") {
            (Side::Red, s.into())
        } else if s.starts_with("B") {
            (Side::Blue, s.into())
        } else if s.starts_with("N") {
            (Side::Neutral, s.into())
        } else {
            let pfx = match side {
                Side::Red => "R",
                Side::Blue => "B",
                Side::Neutral => "N",
            };
            (side, format_compact!("{}{}", pfx, s).into())
        }
    }
}

atomic_id!(ObjectiveId);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    pub(super) id: ObjectiveId,
    pub(super) name: String,
    pub(super) pos: Vector2,
    pub(super) radius: f64,
    pub(super) owner: Side,
    pub(super) kind: ObjectiveKind,
    pub(super) slots: Map<SlotId, Vehicle>,
    pub(super) groups: Map<Side, Set<GroupId>>,
    pub(super) health: u8,
    pub(super) logi: u8,
    pub(super) threatened: bool,
    pub(super) last_threatened_ts: DateTime<Utc>,
    pub(super) last_change_ts: DateTime<Utc>,
    pub(super) warehouse: Warehouse,
    #[serde(skip)]
    pub(super) spawned: bool,
    #[serde(skip)]
    pub(super) needs_mark: bool,
}

impl Objective {
    pub fn is_in_circle(&self, pos: Vector2) -> bool {
        na::distance_squared(&self.pos.into(), &pos.into()) <= self.radius.powi(2)
    }

    pub fn name(&self) -> &str {
        self.name.as_str()
    }

    pub fn health(&self) -> u8 {
        self.health
    }

    pub fn logi(&self) -> u8 {
        self.logi
    }

    pub fn captureable(&self) -> bool {
        self.logi == 0
    }

    pub fn owner(&self) -> Side {
        self.owner
    }
}

impl Db {
    pub fn objective(&self, id: &ObjectiveId) -> Result<&Objective> {
        objective!(self, id)
    }

    /// (distance, heading from objective to point, objective)
    pub fn objective_near_point(&self, pos: Vector2) -> (f64, f64, &Objective) {
        let (dist, obj) = self.persisted.objectives.into_iter().fold(
            (f64::MAX, None),
            |(cur_dist, cur_obj), (_, obj)| {
                let dist = na::distance_squared(&pos.into(), &obj.pos.into());
                if dist < cur_dist {
                    (dist, Some(obj))
                } else {
                    (cur_dist, cur_obj)
                }
            },
        );
        let obj = obj.unwrap();
        (dist.sqrt(), azumith2d_to(obj.pos, pos), obj)
    }

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
                        let unit = unit!(self, uid)?;
                        if !unit.tags.contains(UnitTag::Invincible) {
                            total += 1;
                            if logi {
                                logi_total += 1;
                            }
                            if !unit.dead {
                                alive += 1;
                                if logi {
                                    logi_alive += 1;
                                }
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
            warehouse: Warehouse::default(),
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
        self.ephemeral.dirty();
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
        self.ephemeral.dirty();
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
                        self.ephemeral.dirty();
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
        now: DateTime<Utc>,
    ) -> Result<(SmallVec<[ObjectiveId; 4]>, SmallVec<[ObjectiveId; 4]>)> {
        let land = Land::singleton(lua)?;
        let players = self
            .ephemeral
            .players_by_slot
            .values()
            .filter_map(|ucid| {
                let player = &self.persisted.players[ucid];
                let side = player.side;
                player
                    .current_slot
                    .as_ref()
                    .and_then(|(_, inst)| inst.as_ref())
                    .map(|inst| (side, inst.position.p, inst.typ.clone()))
            })
            .collect::<SmallVec<[_; 64]>>();
        let cfg = &self.ephemeral.cfg;
        let cull_distance = (cfg.unit_cull_distance as f64).powi(2);
        let ground_cull_distance = (cfg.ground_vehicle_cull_distance as f64).powi(2);
        let mut to_spawn: SmallVec<[ObjectiveId; 8]> = smallvec![];
        let mut to_cull: SmallVec<[ObjectiveId; 8]> = smallvec![];
        let mut threatened: SmallVec<[ObjectiveId; 16]> = smallvec![];
        let mut not_threatened: SmallVec<[ObjectiveId; 16]> = smallvec![];
        let mut is_on_walkabout: FxHashSet<UnitId> = FxHashSet::default();
        let mut is_close_to_enemies: FxHashSet<UnitId> = FxHashSet::default();
        let mut check_for_walkabout = |obj: &Objective, radius2: f64| -> Result<()> {
            let groups = maybe!(obj.groups, obj.owner, "owner")?;
            for uid in &self.ephemeral.units_potentially_on_walkabout {
                let unit = unit!(self, uid)?;
                match group!(self, unit.group)?.origin {
                    DeployKind::Crate { .. }
                    | DeployKind::Deployed { .. }
                    | DeployKind::Troop { .. } => (),
                    DeployKind::Objective => {
                        if groups.contains(&unit.group) {
                            let dist = na::distance_squared(&unit.pos.into(), &obj.pos.into());
                            if dist > radius2 || self.ephemeral.units_able_to_move.contains(uid) {
                                is_on_walkabout.insert(*uid);
                            }
                        }
                    }
                }
            }
            Ok(())
        };
        let mut check_close_units = |obj: &Objective, spawn: &mut bool, threat: &mut bool| {
            for uid in &self.ephemeral.units_potentially_close_to_enemies {
                let unit = unit!(self, uid)?;
                let group = group!(self, unit.group)?;
                if obj.owner != group.side {
                    let dist = na::distance_squared(&obj.pos.into(), &unit.pos.into());
                    if dist <= ground_cull_distance {
                        *spawn = true;
                        *threat = true;
                        is_close_to_enemies.insert(*uid);
                    }
                }
            }
            Ok::<_, anyhow::Error>(())
        };
        let check_close_players =
            |obj: &Objective, pos3: LuaVec3, spawn: &mut bool, threat: &mut bool| {
                for (side, pos, typ) in &players {
                    if obj.owner != *side {
                        let threat_dist = (cfg.threatened_distance[typ] as f64).powi(2);
                        let ppos = Vector2::new(pos.x, pos.z);
                        let dist = na::distance_squared(&obj.pos.into(), &ppos.into());
                        if dist <= cull_distance {
                            *spawn = true;
                        }
                        if dist <= threat_dist && land.is_visible(pos3, *pos)? {
                            *threat = true;
                        }
                    }
                }
                Ok::<_, anyhow::Error>(())
            };
        for (oid, obj) in &self.persisted.objectives {
            let pos3 = {
                let alt = land.get_height(LuaVec2(obj.pos))?;
                LuaVec3(Vector3::new(obj.pos.x, alt, obj.pos.y))
            };
            let radius2 = obj.radius.powi(2);
            let mut spawn = false;
            let mut is_threatened = false;
            if let Err(e) = check_close_players(obj, pos3, &mut spawn, &mut is_threatened) {
                error!("failed to check for close players {} {e}", obj.id)
            }
            if let Err(e) = check_for_walkabout(obj, radius2) {
                error!("failed to check walkabout for {} {e}", obj.id)
            }
            if let Err(e) = check_close_units(obj, &mut spawn, &mut is_threatened) {
                error!("failed to check close units {} {e}", obj.id)
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
        self.ephemeral
            .units_potentially_on_walkabout
            .retain(|uid| is_on_walkabout.contains(uid));
        self.ephemeral
            .units_potentially_close_to_enemies
            .retain(|uid| is_close_to_enemies.contains(uid));
        let mut became_threatened: SmallVec<[ObjectiveId; 4]> = smallvec![];
        let mut became_clear: SmallVec<[ObjectiveId; 4]> = smallvec![];
        for oid in &threatened {
            let obj = objective_mut!(self, oid)?;
            if !obj.threatened {
                became_threatened.push(*oid);
            }
            obj.threatened = true;
            obj.last_threatened_ts = now;
            self.ephemeral.dirty();
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
                let group = group!(self, gid)?;
                let logi = group.class.is_logi();
                let walkabout = group
                    .units
                    .into_iter()
                    .any(|u| self.ephemeral.units_potentially_on_walkabout.contains(u));
                if !logi && !walkabout {
                    self.ephemeral.spawnq.push_back(*gid);
                }
            }
        }
        for oid in to_cull {
            let obj = objective_mut!(self, oid)?;
            obj.spawned = false;
            for gid in maybe!(&obj.groups, obj.owner, "side group")? {
                let group = group!(self, gid)?;
                let logi = group.class.is_logi();
                let walkabout = group
                    .units
                    .into_iter()
                    .any(|u| self.ephemeral.units_potentially_on_walkabout.contains(u));
                if !logi && !walkabout {
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
                self.ephemeral.dirty();
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
