extern crate nalgebra as na;
use self::{
    group::{DeployKind, GroupId, SpawnedGroup, SpawnedUnit, UnitId},
    objective::{Objective, ObjectiveId},
    persisted::Persisted,
    player::{InstancedPlayer, Player},
};
use crate::{
    cfg::{Cfg, Deployable, DeployableEwr, DeployableJtac, Troop},
    db::{ephemeral::Ephemeral, objective::ObjGroupClass},
    msgq::MsgQ,
    spawnctx::{Despawn, SpawnCtx, SpawnLoc},
};
use anyhow::{anyhow, bail, Result};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{
    azumith2d_to, azumith3d, centroid2d, centroid3d,
    coalition::Side,
    env::miz::{Group, GroupKind, Miz, MizIndex},
    group::GroupCategory,
    land::{Land, SurfaceType},
    net::Ucid,
    object::{DcsObject, DcsOid},
    rotate2d,
    unit::{ClassUnit, Unit},
    LuaVec2, MizLua, Position3, String, Vector2, Vector3,
};
use fxhash::FxHashMap;
use log::{error, info};
use smallvec::{smallvec, SmallVec};
use std::{cmp::max, collections::VecDeque, fs::File, path::Path};

pub mod cargo;
pub mod ephemeral;
pub mod group;
pub mod logistics;
pub mod mizinit;
pub mod objective;
pub mod persisted;
pub mod player;

pub type Map<K, V> = immutable_chunkmap::map::Map<K, V, 32>;
pub type Set<K> = immutable_chunkmap::set::Set<K, 32>;

#[macro_export]
macro_rules! maybe {
    ($t:expr, $id:expr, $name:expr) => {
        $t.get(&$id)
            .ok_or_else(|| anyhow!("no such {} {:?}", $name, $id))
    };
}

#[macro_export]
macro_rules! maybe_mut {
    ($t:expr, $id:expr, $name:expr) => {
        $t.get_mut(&$id)
            .ok_or_else(|| anyhow!("no such {} {:?}", $name, $id))
    };
}

#[macro_export]
macro_rules! unit {
    ($t:expr, $id:expr) => {
        $t.persisted
            .units
            .get(&$id)
            .ok_or_else(|| anyhow!("no such unit {:?}", $id))
    };
}

#[macro_export]
macro_rules! unit_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .units
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such unit {:?}", $id))
    };
}

#[macro_export]
macro_rules! unit_by_name {
    ($t:expr, $name:expr) => {
        $t.persisted
            .units_by_name
            .get($name)
            .and_then(|id| $t.persisted.units.get(id))
            .ok_or_else(|| anyhow!("no such unit {}", $name))
    };
}

#[macro_export]
macro_rules! group {
    ($t:expr, $id:expr) => {
        $t.persisted
            .groups
            .get(&$id)
            .ok_or_else(|| anyhow!("no such group {:?}", $id))
    };
}

#[macro_export]
macro_rules! group_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .groups
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such group {:?}", $id))
    };
}

#[macro_export]
macro_rules! group_by_name {
    ($t:expr, $name:expr) => {
        $t.persisted
            .groups_by_name
            .get($name)
            .and_then(|id| $t.persisted.groups.get(id))
            .ok_or_else(|| anyhow!("no such group {}", $name))
    };
}

#[macro_export]
macro_rules! objective {
    ($t:expr, $id:expr) => {
        $t.persisted
            .objectives
            .get(&$id)
            .ok_or_else(|| anyhow!("no such objective {:?}", $id))
    };
}

#[macro_export]
macro_rules! objective_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .objectives
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such objective {:?}", $id))
    };
}

#[derive(Debug, Default)]
pub struct Db {
    persisted: Persisted,
    ephemeral: Ephemeral,
}

impl Db {
    pub fn cfg(&self) -> &Cfg {
        &self.ephemeral.cfg
    }

    pub fn ephemeral(&self) -> &Ephemeral {
        &self.ephemeral
    }

    pub fn load(miz: &Miz, idx: &MizIndex, path: &Path) -> Result<Self> {
        let file = File::open(&path)
            .map_err(|e| anyhow!("failed to open save file {:?}, {:?}", path, e))?;
        let persisted: Persisted = serde_json::from_reader(file)
            .map_err(|e| anyhow!("failed to decode save file {:?}, {:?}", path, e))?;
        let mut db = Db {
            persisted,
            ephemeral: Ephemeral::default(),
        };
        db.ephemeral.set_cfg(miz, idx, Cfg::load(path)?)?;
        Ok(db)
    }

    pub fn maybe_snapshot(&mut self) -> Option<Persisted> {
        if self.ephemeral.take_dirty() {
            Some(self.persisted.clone())
        } else {
            None
        }
    }

    pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &SpawnedGroup)> {
        self.persisted.groups.into_iter()
    }

    pub fn group(&self, id: &GroupId) -> Result<&SpawnedGroup> {
        group!(self, id)
    }

    pub fn group_center(&self, id: &GroupId) -> Result<Vector2> {
        let group = group!(self, id)?;
        Ok(centroid2d(
            group
                .units
                .into_iter()
                .filter_map(|uid| self.persisted.units.get(uid))
                .filter_map(|unit| if unit.dead { None } else { Some(unit.pos) }),
        ))
    }

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

    pub fn group_by_name(&self, name: &str) -> Result<&SpawnedGroup> {
        group_by_name!(self, name)
    }

    pub fn unit(&self, id: &UnitId) -> Result<&SpawnedUnit> {
        unit!(self, id)
    }

    pub fn unit_by_name(&self, name: &str) -> Result<&SpawnedUnit> {
        unit_by_name!(self, name)
    }

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

    pub fn instanced_units(&self) -> impl Iterator<Item = (&SpawnedUnit, &DcsOid<ClassUnit>)> {
        self.persisted
            .units
            .into_iter()
            .filter_map(|(uid, sp)| self.ephemeral.object_id_by_uid.get(uid).map(|id| (sp, id)))
    }

    pub fn ewrs(&self) -> impl Iterator<Item = (Vector3, Side, &DeployableEwr)> {
        self.persisted.ewrs.into_iter().filter_map(|gid| {
            let group = self.persisted.groups.get(gid)?;
            match &group.origin {
                DeployKind::Crate { .. } | DeployKind::Objective | DeployKind::Troop { .. } => None,
                DeployKind::Deployed { spec, .. } => {
                    let ewr = spec.ewr.as_ref()?;
                    let pos = centroid3d(
                        group
                            .units
                            .into_iter()
                            .map(|u| self.persisted.units[u].position.p.0),
                    );
                    Some((pos, group.side, ewr))
                }
            }
        })
    }

    pub fn jtacs(
        &self,
    ) -> impl Iterator<Item = (Vector3, &DcsOid<ClassUnit>, &SpawnedGroup, &DeployableJtac)> {
        self.persisted.jtacs.into_iter().filter_map(|gid| {
            let group = self.persisted.groups.get(gid)?;
            let id = group
                .units
                .into_iter()
                .find_map(|gid| self.ephemeral.object_id_by_uid.get(gid))?;
            match &group.origin {
                DeployKind::Troop {
                    spec: Troop {
                        jtac: Some(jtac), ..
                    },
                    ..
                }
                | DeployKind::Deployed {
                    spec:
                        Deployable {
                            jtac: Some(jtac), ..
                        },
                    ..
                } => {
                    let pos = centroid3d(
                        group
                            .units
                            .into_iter()
                            .map(|u| self.persisted.units[u].position.p.0),
                    );
                    Some((pos, id, group, jtac))
                }
                DeployKind::Crate { .. }
                | DeployKind::Objective
                | DeployKind::Troop { .. }
                | DeployKind::Deployed { .. } => None,
            }
        })
    }

    pub fn msgs(&mut self) -> &mut MsgQ {
        &mut self.ephemeral.msgs
    }

    fn spawn_group<'lua>(
        &self,
        idx: &MizIndex,
        spctx: &SpawnCtx,
        group: &SpawnedGroup,
    ) -> Result<()> {
        let template = spctx.get_template(
            idx,
            GroupKind::Any,
            group.side,
            group.template_name.as_str(),
        )?;
        template.group.set("lateActivation", false)?;
        template.group.set("hidden", false)?;
        template.group.set_name(group.name.clone())?;
        let mut points: SmallVec<[Vector2; 16]> = smallvec![];
        let by_tname: FxHashMap<&str, &SpawnedUnit> = group
            .units
            .into_iter()
            .filter_map(|uid| {
                self.persisted.units.get(uid).and_then(|u| {
                    points.push(u.pos);
                    if u.dead {
                        None
                    } else {
                        Some((u.template_name.as_str(), u))
                    }
                })
            })
            .collect();
        let alive = {
            let units = template.group.units()?;
            let mut i = 1;
            while i as usize <= units.len() {
                let unit = units.get(i)?;
                match by_tname.get(unit.name()?.as_str()) {
                    None => units.remove(i)?,
                    Some(su) => {
                        unit.raw_remove("unitId")?;
                        template.group.set_pos(su.pos)?;
                        unit.set_pos(su.pos)?;
                        unit.set_heading(su.heading)?;
                        unit.set_name(su.name.clone())?;
                        i += 1;
                    }
                }
            }
            units.len() > 0
        };
        if alive {
            let point = centroid2d(points.iter().map(|p| *p));
            let radius = points
                .iter()
                .map(|p: &Vector2| na::distance_squared(&(*p).into(), &point.into()))
                .fold(0., |acc, d| if d > acc { d } else { acc });
            spctx.remove_junk(point, radius.sqrt() * 1.10)?;
            spctx.spawn(template)
        } else {
            Ok(())
        }
    }

    pub fn process_spawn_queue(
        &mut self,
        now: DateTime<Utc>,
        idx: &MizIndex,
        spctx: &SpawnCtx,
    ) -> Result<()> {
        while let Some((at, gids)) = self.ephemeral.delayspawnq.first_key_value() {
            if now < *at {
                break;
            } else {
                for gid in gids {
                    self.ephemeral.spawnq.push_back(*gid);
                }
                let at = *at;
                self.ephemeral.delayspawnq.remove(&at);
            }
        }
        let dlen = self.ephemeral.despawnq.len();
        let slen = self.ephemeral.spawnq.len();
        if dlen > 0 {
            for _ in 0..max(2, dlen >> 2) {
                if let Some((gid, name)) = self.ephemeral.despawnq.pop_front() {
                    if let Some(group) = self.persisted.groups.get(&gid) {
                        for uid in &group.units {
                            self.ephemeral.units_able_to_move.remove(uid);
                            self.ephemeral
                                .units_potentially_close_to_enemies
                                .remove(uid);
                            self.ephemeral.units_potentially_on_walkabout.remove(uid);
                            if let Some(id) = self.ephemeral.object_id_by_uid.remove(uid) {
                                self.ephemeral.uid_by_object_id.remove(&id);
                            }
                        }
                    }
                    spctx.despawn(name)?
                }
            }
        } else if slen > 0 {
            for _ in 0..max(2, slen >> 2) {
                if let Some(gid) = self.ephemeral.spawnq.pop_front() {
                    self.spawn_group(idx, spctx, group!(self, gid)?)?
                }
            }
        }
        Ok(())
    }

    pub fn mark_group(&mut self, gid: &GroupId) -> Result<()> {
        if let Some(id) = self.ephemeral.group_marks.remove(gid) {
            self.ephemeral.msgs.delete_mark(id)
        }
        let group = group!(self, gid)?;
        let group_center = centroid2d(
            group
                .units
                .into_iter()
                .map(|uid| self.persisted.units[uid].pos),
        );
        let id = match &group.origin {
            DeployKind::Objective => None,
            DeployKind::Crate { player, spec, .. } => {
                let name = self.persisted.players[player].name.clone();
                let msg = format_compact!("{} {gid} deployed by {name}", spec.name);
                Some(
                    self.ephemeral
                        .msgs
                        .mark_to_side(group.side, group_center, true, msg),
                )
            }
            DeployKind::Deployed { spec, player } => {
                let name = self.persisted.players[player].name.clone();
                let msg = format_compact!("{} {gid} deployed by {name}", spec.path.last().unwrap());
                Some(
                    self.ephemeral
                        .msgs
                        .mark_to_side(group.side, group_center, true, msg),
                )
            }
            DeployKind::Troop { player, spec } => {
                let name = self.persisted.players[player].name.clone();
                let msg = format_compact!("{} {gid} deployed by {name}", spec.name);
                Some(
                    self.ephemeral
                        .msgs
                        .mark_to_side(group.side, group_center, true, msg),
                )
            }
        };
        if let Some(id) = id {
            self.ephemeral.group_marks.insert(*gid, id);
        }
        Ok(())
    }

    pub fn delete_group(&mut self, gid: &GroupId) -> Result<()> {
        self.ephemeral.spawnq.retain(|id| id != gid);
        let group = self
            .persisted
            .groups
            .remove_cow(gid)
            .ok_or_else(|| anyhow!("no such group {:?}", gid))?;
        self.persisted.groups_by_name.remove_cow(&group.name);
        self.persisted
            .groups_by_side
            .get_mut_cow(&group.side)
            .map(|m| m.remove_cow(gid));
        match &group.origin {
            DeployKind::Objective => (),
            DeployKind::Crate { player, .. } => {
                self.persisted.crates.remove_cow(gid);
                self.persisted.players[player].crates.remove_cow(gid);
            }
            DeployKind::Deployed { spec, .. } => {
                self.persisted.deployed.remove_cow(gid);
                if spec.jtac.is_some() {
                    self.persisted.jtacs.remove_cow(gid);
                }
                if spec.ewr.is_some() {
                    self.persisted.ewrs.remove_cow(gid);
                }
            }
            DeployKind::Troop { spec, .. } => {
                self.persisted.troops.remove_cow(gid);
                if spec.jtac.is_some() {
                    self.persisted.jtacs.remove_cow(gid);
                }
            }
        }
        if let Some(mark) = self.ephemeral.group_marks.remove(gid) {
            self.ephemeral.msgs.delete_mark(mark);
        }
        let mut units: SmallVec<[String; 16]> = smallvec![];
        for uid in &group.units {
            self.ephemeral
                .units_potentially_close_to_enemies
                .remove(uid);
            self.ephemeral.units_potentially_on_walkabout.remove(uid);
            if let Some(id) = self.ephemeral.object_id_by_uid.remove(uid) {
                self.ephemeral.uid_by_object_id.remove(&id);
            }
            if let Some(unit) = self.persisted.units.remove_cow(uid) {
                self.persisted.units_by_name.remove_cow(&unit.name);
                units.push(unit.name);
            }
        }
        self.ephemeral.dirty();
        match group.kind {
            None => {
                // it's a static, we have to get it's units
                for unit in &units {
                    self.ephemeral
                        .despawnq
                        .push_back((*gid, Despawn::Static(unit.clone())));
                }
            }
            Some(_) => {
                // it's a normal group
                self.ephemeral
                    .despawnq
                    .push_back((*gid, Despawn::Group(group.name.clone())));
            }
        }
        Ok(())
    }

    /// add the units to the db, but don't actually spawn them
    fn add_group<'lua>(
        &mut self,
        spctx: &'lua SpawnCtx<'lua>,
        idx: &MizIndex,
        side: Side,
        location: SpawnLoc,
        template_name: &str,
        origin: DeployKind,
    ) -> Result<GroupId> {
        fn distance<'a, F: Fn(f64, f64) -> f64>(
            pos: Vector2,
            cmp: F,
            positions: impl IntoIterator<Item = &'a Vector2>,
        ) -> f64 {
            positions
                .into_iter()
                .fold(None, |acc, p| {
                    let d = na::distance_squared(&(*p).into(), &pos.into());
                    let acc = match acc {
                        None => d,
                        Some(d) => d,
                    };
                    Some(cmp(acc, d))
                })
                .map(|d| d.sqrt())
                .unwrap_or(0.)
        }
        fn compute_unit_positions(
            spctx: &SpawnCtx,
            idx: &MizIndex,
            location: SpawnLoc,
            template: &Group,
        ) -> Result<(VecDeque<Vector2>, FxHashMap<String, VecDeque<Vector2>>, f64)> {
            let mut positions = template
                .units()?
                .into_iter()
                .map(|u| Ok(u?.pos()?))
                .collect::<Result<VecDeque<_>>>()?;
            match location {
                SpawnLoc::AtPosWithCenter { pos, center } => {
                    for p in positions.iter_mut() {
                        *p = *p - center + pos;
                    }
                    Ok((positions, FxHashMap::default(), 0.))
                }
                SpawnLoc::AtTrigger {
                    name,
                    group_heading,
                } => {
                    let group_center = centroid2d(positions.iter().map(|p| *p));
                    let pos = spctx.get_trigger_zone(idx, name.as_str())?.pos()?;
                    for p in positions.iter_mut() {
                        *p = *p - group_center + pos;
                    }
                    rotate2d(group_heading, positions.make_contiguous());
                    Ok((positions, FxHashMap::default(), group_heading))
                }
                SpawnLoc::AtPos {
                    pos,
                    offset_direction,
                    group_heading,
                } => {
                    let group_center = centroid2d(positions.iter().map(|p| *p));
                    let radius = distance(group_center, f64::max, positions.iter());
                    for p in positions.iter_mut() {
                        *p = *p - group_center + pos + radius * offset_direction;
                    }
                    rotate2d(group_heading, positions.make_contiguous());
                    let offset_magnitude = 20. - distance(pos, f64::min, positions.iter());
                    for p in positions.iter_mut() {
                        *p = *p + offset_magnitude * offset_direction
                    }
                    Ok((positions, FxHashMap::default(), group_heading))
                }
                SpawnLoc::AtPosWithComponents {
                    pos,
                    group_heading,
                    component_pos,
                } => {
                    let group_center = centroid2d(positions.iter().map(|p| *p));
                    let center_by_typ: FxHashMap<String, Vector2> = {
                        let mut tbl = FxHashMap::default();
                        for unit in template.units()? {
                            let unit = unit?;
                            let pos = unit.pos()?;
                            let typ = unit.typ()?;
                            if component_pos.contains_key(&**typ) {
                                let (n, v) = tbl
                                    .entry(typ.clone())
                                    .or_insert_with(|| (0, Vector2::new(0., 0.)));
                                *v += pos;
                                *n += 1;
                            }
                        }
                        tbl.into_iter()
                            .map(|(k, (n, v))| (k, v / (n as f64)))
                            .collect()
                    };
                    let mut final_position_by_type: FxHashMap<String, VecDeque<Vector2>> =
                        FxHashMap::default();
                    positions.clear();
                    for unit in template.units()? {
                        let unit = unit?;
                        let typ = unit.typ()?;
                        let group_center = match center_by_typ.get(&typ) {
                            None => group_center,
                            Some(pos) => *pos,
                        };
                        match component_pos.get(&typ) {
                            None => positions.push_back(unit.pos()? - group_center + pos),
                            Some(pos) => final_position_by_type
                                .entry(typ.clone())
                                .or_default()
                                .push_back(unit.pos()? - group_center + *pos),
                        }
                    }
                    rotate2d(group_heading, positions.make_contiguous());
                    for positions in final_position_by_type.values_mut() {
                        rotate2d(group_heading, positions.make_contiguous());
                    }
                    Ok((positions, final_position_by_type, group_heading))
                }
            }
        }
        fn check_water(
            land: &Land,
            positions: &VecDeque<Vector2>,
            positions_by_typ: &FxHashMap<String, VecDeque<Vector2>>,
        ) -> Result<()> {
            for pos in positions
                .iter()
                .chain(positions_by_typ.values().flat_map(|v| v.iter()))
            {
                match land.get_surface_type(LuaVec2(*pos))? {
                    SurfaceType::Land | SurfaceType::Road | SurfaceType::Runway => (),
                    SurfaceType::ShallowWater | SurfaceType::Water => {
                        bail!("you can't spawn units in water")
                    }
                }
            }
            Ok(())
        }
        let land = Land::singleton(spctx.lua())?;
        let template_name = String::from(template_name);
        let template = spctx.get_template_ref(idx, GroupKind::Any, side, template_name.as_str())?;
        let (mut positions, mut positions_by_typ, heading) =
            compute_unit_positions(&spctx, idx, location, &template.group)?;
        check_water(&land, &positions, &positions_by_typ)?;
        let kind = GroupCategory::from_kind(template.category);
        let gid = GroupId::new();
        let group_name = String::from(format_compact!("{}-{}", template_name, gid));
        let mut spawned = SpawnedGroup {
            id: gid,
            name: group_name.clone(),
            template_name: template_name.clone(),
            side,
            kind,
            origin,
            class: ObjGroupClass::from(template_name.as_str()),
            units: Set::new(),
        };
        for unit in template.group.units()?.into_iter() {
            let uid = UnitId::new();
            let unit = unit?;
            let typ = unit.typ()?;
            let template_name = unit.name()?;
            let unit_name = String::from(format_compact!("{}-{}", group_name, uid));
            let tags = *self
                .ephemeral
                .cfg
                .unit_classification
                .get(typ.as_str())
                .ok_or_else(|| anyhow!("unit type not classified {typ}"))?;
            let pos = match positions_by_typ.get_mut(&typ) {
                None => positions.pop_front().unwrap(),
                Some(positions) => positions.pop_front().unwrap(),
            };
            let position = {
                let mut p = Position3::default();
                p.p.x = pos.x;
                p.p.y = land.get_height(LuaVec2(pos))?;
                p.p.z = pos.y;
                p
            };
            let spawned_unit = SpawnedUnit {
                id: uid,
                group: gid,
                side,
                typ,
                tags,
                name: unit_name.clone(),
                template_name,
                spawn_position: position,
                spawn_pos: pos,
                spawn_heading: heading,
                position,
                pos,
                heading,
                dead: false,
                moved: None,
            };
            spawned.units.insert_cow(uid);
            self.persisted.units.insert_cow(uid, spawned_unit);
            self.persisted.units_by_name.insert_cow(unit_name, uid);
        }
        match &mut spawned.origin {
            DeployKind::Objective => (),
            DeployKind::Crate { player, .. } => {
                self.persisted.crates.insert_cow(gid);
                self.persisted.players[player].crates.insert_cow(gid);
            }
            DeployKind::Deployed { spec, .. } => {
                self.persisted.deployed.insert_cow(gid);
                if spec.jtac.is_some() {
                    self.persisted.jtacs.insert_cow(gid);
                }
                if spec.ewr.is_some() {
                    self.persisted.ewrs.insert_cow(gid);
                }
            }
            DeployKind::Troop { spec, .. } => {
                self.persisted.troops.insert_cow(gid);
                if spec.jtac.is_some() {
                    self.persisted.jtacs.insert_cow(gid);
                }
            }
        }
        self.persisted.groups.insert_cow(gid, spawned);
        self.persisted.groups_by_name.insert_cow(group_name, gid);
        self.persisted
            .groups_by_side
            .get_or_default_cow(side)
            .insert_cow(gid);
        self.ephemeral.dirty();
        self.mark_group(&gid)?;
        Ok(gid)
    }

    pub fn add_and_queue_group<'lua>(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        side: Side,
        location: SpawnLoc,
        template_name: &str,
        origin: DeployKind,
        delay: Option<DateTime<Utc>>,
    ) -> Result<GroupId> {
        let gid = self.add_group(&spctx, idx, side, location, template_name, origin)?;
        match delay {
            None => self.ephemeral.spawnq.push_back(gid),
            Some(at) => self.ephemeral.delayspawnq.entry(at).or_default().push(gid),
        }
        Ok(gid)
    }

    pub fn unit_born(&mut self, unit: &Unit) -> Result<()> {
        let id = unit.object_id()?;
        let name = unit.get_name()?;
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            self.ephemeral.uid_by_object_id.insert(id.clone(), *uid);
            self.ephemeral.object_id_by_uid.insert(*uid, id.clone());
        }
        let slot = unit.slot()?;
        if self.persisted.objectives_by_slot.get(&slot).is_some() {
            self.ephemeral
                .slot_by_object_id
                .insert(id.clone(), slot.clone());
            self.ephemeral.object_id_by_slot.insert(slot, id);
        }
        Ok(())
    }

    pub fn unit_dead(&mut self, id: &DcsOid<ClassUnit>, now: DateTime<Utc>) -> Result<()> {
        if let Some(slot) = self.ephemeral.slot_by_object_id.remove(&id) {
            self.ephemeral.object_id_by_slot.remove(&slot);
            self.ephemeral.cargo.remove(&slot);
            if let Some(ucid) = self.ephemeral.players_by_slot.remove(&slot) {
                self.persisted.players[&ucid].current_slot = None;
            }
        }
        let uid = match self.ephemeral.uid_by_object_id.remove(&id) {
            None => {
                info!("no uid for object id {:?}", id);
                return Ok(());
            }
            Some(uid) => {
                self.ephemeral.object_id_by_uid.remove(&uid);
                uid
            }
        };
        self.ephemeral
            .units_potentially_close_to_enemies
            .remove(&uid);
        self.ephemeral.units_potentially_on_walkabout.remove(&uid);
        self.ephemeral.units_able_to_move.remove(&uid);
        match self.persisted.units.get_mut_cow(&uid) {
            None => error!("unit_dead: missing unit {:?}", uid),
            Some(unit) => {
                unit.dead = true;
                unit.pos = unit.spawn_pos;
                unit.heading = unit.spawn_heading;
                unit.position = unit.spawn_position;
                self.ephemeral.dirty();
                let gid = unit.group;
                if let Some(oid) = self.persisted.objectives_by_group.get(&gid).copied() {
                    self.update_objective_status(&oid, now)?
                }
                if self.persisted.deployed.contains(&gid)
                    || self.persisted.troops.contains(&gid)
                    || self.persisted.crates.contains(&gid)
                {
                    let group = group_mut!(self, gid)?;
                    let mut dead = true;
                    for uid in &group.units {
                        dead &= unit!(self, uid)?.dead
                    }
                    if dead {
                        self.delete_group(&gid)?
                    }
                }
            }
        }
        Ok(())
    }

    pub fn update_unit_positions<'a, I: Iterator<Item = UnitId> + 'a>(
        &'a mut self,
        lua: MizLua,
        units: Option<I>,
    ) -> Result<()> {
        let mut unit: Option<Unit> = None;
        let mut moved: SmallVec<[GroupId; 16]> = smallvec![];
        let units = units
            .map(|i| Box::new(i) as Box<dyn Iterator<Item = UnitId>>)
            .unwrap_or_else(|| {
                Box::new(self.ephemeral.units_able_to_move.iter().map(|i| *i))
                    as Box<dyn Iterator<Item = UnitId>>
            });
        for uid in units {
            let id = maybe!(self.ephemeral.object_id_by_uid, uid, "object id")?;
            let instance = match unit.take() {
                Some(unit) => unit.change_instance(id)?,
                None => Unit::get_instance(lua, id)?,
            };
            let pos = instance.get_position()?;
            let point = Vector2::new(pos.p.x, pos.p.z);
            let heading = azumith3d(pos.x.0);
            let spunit = unit_mut!(self, uid)?;
            if spunit.position != pos {
                moved.push(spunit.group);
                spunit.position = pos;
                spunit.pos = point;
                spunit.heading = heading;
                self.ephemeral
                    .units_potentially_close_to_enemies
                    .insert(uid);
                self.ephemeral.units_potentially_on_walkabout.insert(uid);
            }
            unit = Some(instance);
        }
        for gid in moved {
            self.ephemeral.dirty();
            self.mark_group(&gid)?;
        }
        Ok(())
    }
}
