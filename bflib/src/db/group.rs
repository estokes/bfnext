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

use super::{Db, SetS, ephemeral::SlotInfo, objective::ObjGroupClass, player::SlotAuth};
use crate::{
    Connected, group, group_by_name, group_health, group_mut, objective,
    spawnctx::{Despawn, SpawnCtx, SpawnLoc},
    unit, unit_by_name, unit_mut,
};
use anyhow::{Context, Result, anyhow, bail};
use bfprotocols::{
    cfg::{Action, ActionKind, Crate, Deployable, Troop, UnitTag, UnitTags, Vehicle},
    db::objective::ObjectiveId,
    stats::{self, EnId},
};
use bfprotocols::{
    db::group::{GroupId, UnitId},
    stats::Stat,
};
use chrono::prelude::*;
use compact_str::{CompactString, format_compact};
use dcso3::{
    LuaVec2, LuaVec3, MizLua, Position3, String, Vector2, Vector3, azumith3d, centroid2d,
    centroid3d, change_heading,
    coalition::Side,
    coord::Coord,
    env::miz,
    env::miz::{Group, GroupKind, MizIndex},
    group::GroupCategory,
    land::{Land, SurfaceType},
    net::{SlotId, Ucid},
    object::{DcsObject, DcsOid},
    rotate2d_gen,
    static_object::{ClassStatic, StaticObject},
    trigger::MarkId,
    unit::{ClassUnit, Unit},
};
use enumflags2::BitFlags;
use fxhash::{FxHashMap, FxHashSet};
use log::{error, warn};
use serde_derive::{Deserialize, Serialize};
use smallvec::{SmallVec, smallvec};
use std::{cmp::max, collections::VecDeque};

#[derive(Debug, Clone)]
pub enum BirthRes {
    None,
    OccupiedSlot(SlotId),
    DynamicSlotDenied(Ucid, SlotAuth),
}

fn default_cost_fraction() -> f32 {
    1.
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DeployKind {
    #[serde(rename = "Objective")]
    ObjectiveDeprecated,
    #[serde(rename = "ObjectiveV2")]
    Objective { origin: ObjectiveId },
    Deployed {
        player: Ucid,
        #[serde(default)]
        moved_by: Option<(Ucid, u32)>,
        spec: Deployable,
        #[serde(default = "default_cost_fraction")]
        cost_fraction: f32,
        #[serde(default)]
        origin: Option<ObjectiveId>,
    },
    Troop {
        player: Ucid,
        origin: Option<ObjectiveId>,
        #[serde(default)]
        moved_by: Option<(Ucid, u32)>,
        spec: Troop,
        #[serde(default = "default_cost_fraction")]
        cost_fraction: f32,
    },
    Crate {
        origin: ObjectiveId,
        player: Ucid,
        spec: Crate,
    },
    Action {
        #[serde(skip)]
        marks: FxHashSet<MarkId>,
        loc: SpawnLoc,
        player: Option<Ucid>,
        name: String,
        spec: Action,
        time: DateTime<Utc>,
        destination: Option<Vector2>,
        rtb: Option<Vector2>,
        #[serde(default)]
        origin: Option<ObjectiveId>,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpawnedUnit {
    pub name: String,
    pub id: UnitId,
    pub group: GroupId,
    pub side: Side,
    pub typ: Vehicle,
    pub tags: UnitTags,
    pub template_name: String,
    pub spawn_pos: Vector2,
    pub spawn_heading: f64,
    pub spawn_position: Position3,
    pub pos: Vector2,
    pub heading: f64,
    pub position: Position3,
    pub dead: bool,
    #[serde(skip)]
    pub moved: Option<DateTime<Utc>>,
    #[serde(skip)]
    pub airborne_velocity: Option<Vector3>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedGroup {
    pub id: GroupId,
    pub name: String,
    pub template_name: String,
    pub side: Side,
    pub kind: Option<GroupCategory>,
    pub class: ObjGroupClass,
    pub origin: DeployKind,
    pub units: SetS<UnitId>,
    pub tags: UnitTags,
}

impl Db {
    #[allow(dead_code)]
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

    #[allow(dead_code)]
    pub fn group_center3(&self, id: &GroupId) -> Result<Vector3> {
        let group = group!(self, id)?;
        Ok(centroid3d(
            group
                .units
                .into_iter()
                .filter_map(|uid| self.persisted.units.get(uid))
                .filter_map(|unit| {
                    if unit.dead {
                        None
                    } else {
                        Some(unit.position.p.0)
                    }
                }),
        ))
    }

    #[allow(dead_code)]
    pub fn group_by_name(&self, name: &str) -> Result<&SpawnedGroup> {
        group_by_name!(self, name)
    }

    pub fn unit(&self, id: &UnitId) -> Result<&SpawnedUnit> {
        unit!(self, id)
    }

    #[allow(dead_code)]
    pub fn unit_by_name(&self, name: &str) -> Result<&SpawnedUnit> {
        unit_by_name!(self, name)
    }

    pub fn first_living_unit(&self, gid: &GroupId) -> Result<&DcsOid<ClassUnit>> {
        group!(self, gid)?
            .units
            .into_iter()
            .find_map(|uid| self.ephemeral.get_object_id_by_uid(uid))
            .ok_or_else(|| anyhow!("all units are dead"))
    }

    pub fn instanced_units(&self) -> impl Iterator<Item = (&SpawnedUnit, &DcsOid<ClassUnit>)> {
        self.persisted
            .units
            .into_iter()
            .filter_map(|(uid, sp)| self.ephemeral.object_id_by_uid.get(uid).map(|id| (sp, id)))
    }

    pub fn deployed(&self) -> impl Iterator<Item = &SpawnedGroup> {
        self.persisted
            .deployed
            .into_iter()
            .chain(self.persisted.troops.into_iter())
            .filter_map(|gid| self.persisted.groups.get(gid))
    }

    pub(super) fn mark_group(&mut self, gid: &GroupId) -> Result<()> {
        if let Some(id) = self.ephemeral.group_marks.remove(gid) {
            self.ephemeral.msgs.delete_mark(id)
        }
        let group = group_mut!(self, gid)?;
        let group_center = centroid2d(
            group
                .units
                .into_iter()
                .map(|uid| self.persisted.units[uid].pos),
        );
        let id = match &mut group.origin {
            DeployKind::ObjectiveDeprecated => None,
            DeployKind::Objective { origin: oid } => match objective!(self, oid) {
                Err(_) => None,
                Ok(obj) => {
                    if group.side == obj.owner {
                        let msg = format_compact!(
                            "objective group id {} name {} of class {:?}",
                            group.id,
                            group.name,
                            group.class
                        );
                        Some(
                            self.ephemeral
                                .msgs
                                .mark_to_side(group.side, group_center, true, msg),
                        )
                    } else {
                        None
                    }
                }
            },
            DeployKind::Action {
                name,
                spec: _,
                destination,
                player,
                marks,
                ..
            } => {
                let pname = player
                    .as_ref()
                    .map(|p| self.persisted.players[p].name.clone())
                    .unwrap_or(String::from("Server"));
                let pos_msg = format_compact!("{name} {gid} deployed by {pname}");
                let pos_mark =
                    self.ephemeral
                        .msgs
                        .mark_to_side(group.side, group_center, true, pos_msg);
                match destination {
                    None => Some(pos_mark),
                    Some(dst) => {
                        if !marks.is_empty() {
                            Some(pos_mark)
                        } else {
                            let dst_msg = format_compact!("{name} {gid} destination");
                            marks.insert(
                                self.ephemeral
                                    .msgs
                                    .mark_to_side(group.side, *dst, true, dst_msg),
                            );
                            Some(pos_mark)
                        }
                    }
                }
            }
            DeployKind::Crate { player, spec, .. } => {
                let name = self.persisted.players[player].name.clone();
                let msg = format_compact!("{} {gid} deployed by {name}", spec.name);
                Some(
                    self.ephemeral
                        .msgs
                        .mark_to_side(group.side, group_center, true, msg),
                )
            }
            DeployKind::Deployed {
                spec,
                player,
                moved_by,
                cost_fraction: _,
                origin: _,
            } => {
                let name = self.persisted.players[player].name.clone();
                let resp = moved_by
                    .as_ref()
                    .map(|(u, _)| {
                        let name = self.persisted.players[u].name.clone();
                        format_compact!("\nresponsible party: {name}")
                    })
                    .unwrap_or(CompactString::from(""));
                let msg = format_compact!(
                    "{} {gid} deployed by {name}{resp}",
                    spec.path.last().unwrap()
                );
                Some(
                    self.ephemeral
                        .msgs
                        .mark_to_side(group.side, group_center, true, msg),
                )
            }
            DeployKind::Troop {
                player,
                spec,
                moved_by,
                origin: _,
                cost_fraction: _,
            } => {
                let name = self.persisted.players[player].name.clone();
                let resp = moved_by
                    .as_ref()
                    .map(|(u, _)| {
                        let name = self.persisted.players[u].name.clone();
                        format_compact!("\nresponsible party: {name}")
                    })
                    .unwrap_or(CompactString::from(""));
                let msg = format_compact!("{} {gid} deployed by {name}{resp}", spec.name);
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
            DeployKind::ObjectiveDeprecated | DeployKind::Objective { .. } => (),
            DeployKind::Action { marks, .. } => {
                for id in marks {
                    self.ephemeral.msgs().delete_mark(*id);
                }
                self.persisted.actions.remove_cow(gid);
                self.persisted.jtacs.remove_cow(gid);
            }
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
        if let Some(id) = self.ephemeral.group_marks.remove(gid) {
            self.ephemeral.msgs.delete_mark(id);
        }
        let mut units: SmallVec<[String; 16]> = smallvec![];
        for uid in &group.units {
            self.ephemeral
                .units_potentially_close_to_enemies
                .remove(uid);
            self.ephemeral.units_able_to_move.swap_remove(uid);
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
                        .push_despawn(*gid, Despawn::Static(unit.clone()))
                }
            }
            Some(_) => {
                // it's a normal group
                if let Some(oid) = self.ephemeral.object_id_by_gid.get(gid) {
                    self.ephemeral
                        .push_despawn(*gid, Despawn::Group(oid.clone()));
                }
            }
        }
        self.ephemeral.stat(Stat::GroupDeleted { id: *gid });
        Ok(())
    }

    /// add the units to the db, but don't actually spawn them
    pub(super) fn add_group<'lua>(
        &mut self,
        spctx: &'lua SpawnCtx<'lua>,
        idx: &MizIndex,
        side: Side,
        location: SpawnLoc,
        template_name: &str,
        origin: DeployKind,
        extra_tags: BitFlags<UnitTag>,
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
        #[derive(Debug)]
        struct UnitPosition {
            heading: f64,
            position: Vector2,
            altitude: Option<f64>,
        }
        #[derive(Debug)]
        struct GroupPosition {
            positions: VecDeque<UnitPosition>,
            by_type: FxHashMap<String, VecDeque<UnitPosition>>,
        }
        fn compute_unit_positions(
            spctx: &SpawnCtx,
            idx: &MizIndex,
            location: SpawnLoc,
            template: &Group,
        ) -> Result<GroupPosition> {
            let mut positions = template
                .units()?
                .into_iter()
                .map(|u| {
                    let u = u?;
                    Ok(UnitPosition {
                        heading: u.heading()?,
                        position: u.pos()?,
                        altitude: u.alt().unwrap_or(None),
                    })
                })
                .collect::<Result<VecDeque<_>>>()?;
            match location {
                SpawnLoc::InAir {
                    pos,
                    heading,
                    altitude,
                    speed: _,
                } => {
                    let group_center = centroid2d(positions.iter().map(|p| p.position));
                    let group_altitude = {
                        let (sum, i) = positions
                            .iter()
                            .filter_map(|p| p.altitude)
                            .fold((0., 0.), |(sum, i), a| (sum + a, i + 1.));
                        sum / i
                    };
                    for p in positions.iter_mut() {
                        p.position = p.position - group_center + pos;
                        p.heading = change_heading(p.heading, heading);
                        if let Some(a) = p.altitude {
                            p.altitude = Some(a - group_altitude + altitude);
                        }
                    }
                    rotate2d_gen(heading, positions.make_contiguous(), |p| &mut p.position);
                    Ok(GroupPosition {
                        positions,
                        by_type: FxHashMap::default(),
                    })
                }
                SpawnLoc::AtPosWithCenter { pos, center } => {
                    for p in positions.iter_mut() {
                        p.position = p.position - center + pos;
                        p.altitude = None;
                    }
                    Ok(GroupPosition {
                        positions,
                        by_type: FxHashMap::default(),
                    })
                }
                SpawnLoc::AtTrigger {
                    name,
                    group_heading,
                } => {
                    let group_center = centroid2d(positions.iter().map(|p| p.position));
                    let pos = spctx.get_trigger_zone(idx, name.as_str())?.pos()?;
                    for p in positions.iter_mut() {
                        p.position = p.position - group_center + pos;
                        p.heading = change_heading(p.heading, group_heading);
                        p.altitude = None;
                    }
                    rotate2d_gen(group_heading, positions.make_contiguous(), |p| {
                        &mut p.position
                    });
                    Ok(GroupPosition {
                        positions,
                        by_type: FxHashMap::default(),
                    })
                }
                SpawnLoc::AtPos {
                    pos,
                    offset_direction,
                    group_heading,
                } => {
                    let group_center = centroid2d(positions.iter().map(|p| p.position));
                    let radius = distance(
                        group_center,
                        f64::max,
                        positions.iter().map(|p| &p.position),
                    );
                    for p in positions.iter_mut() {
                        p.position = p.position - group_center + pos + radius * offset_direction;
                    }
                    rotate2d_gen(group_heading, positions.make_contiguous(), |p| {
                        &mut p.position
                    });
                    let offset_magnitude =
                        20. - distance(pos, f64::min, positions.iter().map(|p| &p.position));
                    for p in positions.iter_mut() {
                        p.position = p.position + offset_magnitude * offset_direction;
                        p.heading = change_heading(p.heading, group_heading);
                        p.altitude = None;
                    }
                    Ok(GroupPosition {
                        positions,
                        by_type: FxHashMap::default(),
                    })
                }
                SpawnLoc::AtPosWithComponents {
                    pos,
                    group_heading,
                    component_pos,
                } => {
                    let group_center = centroid2d(positions.iter().map(|p| p.position));
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
                    let mut by_type: FxHashMap<String, VecDeque<UnitPosition>> =
                        FxHashMap::default();
                    positions.clear();
                    for unit in template.units()? {
                        let unit = unit?;
                        let typ = unit.typ()?;
                        let heading = unit.heading()?;
                        let position = unit.pos()?;
                        let group_center = match center_by_typ.get(&typ) {
                            None => group_center,
                            Some(pos) => *pos,
                        };
                        match component_pos.get(&typ) {
                            None => positions.push_back(UnitPosition {
                                position: position - group_center + pos,
                                heading: change_heading(heading, group_heading),
                                altitude: None,
                            }),
                            Some(pos) => {
                                by_type
                                    .entry(typ.clone())
                                    .or_default()
                                    .push_back(UnitPosition {
                                        position: position - group_center + *pos,
                                        heading: change_heading(heading, group_heading),
                                        altitude: None,
                                    })
                            }
                        }
                    }
                    rotate2d_gen(group_heading, positions.make_contiguous(), |p| {
                        &mut p.position
                    });
                    for positions in by_type.values_mut() {
                        rotate2d_gen(group_heading, positions.make_contiguous(), |p| {
                            &mut p.position
                        });
                    }
                    Ok(GroupPosition { positions, by_type })
                }
            }
        }
        fn check_water(
            land: &Land,
            positions: &VecDeque<UnitPosition>,
            positions_by_typ: &FxHashMap<String, VecDeque<UnitPosition>>,
        ) -> Result<()> {
            for pos in positions
                .iter()
                .chain(positions_by_typ.values().flat_map(|v| v.iter()))
            {
                match land.get_surface_type(LuaVec2(pos.position))? {
                    SurfaceType::Land | SurfaceType::Road | SurfaceType::Runway => (),
                    SurfaceType::ShallowWater | SurfaceType::Water => {
                        bail!("you can't spawn this unit in water")
                    }
                }
            }
            Ok(())
        }
        fn check_land(
            land: &Land,
            positions: &VecDeque<UnitPosition>,
            positions_by_typ: &FxHashMap<String, VecDeque<UnitPosition>>,
        ) -> Result<()> {
            for pos in positions
                .iter()
                .chain(positions_by_typ.values().flat_map(|v| v.iter()))
            {
                match land.get_surface_type(LuaVec2(pos.position))? {
                    SurfaceType::ShallowWater | SurfaceType::Water => (),
                    SurfaceType::Land | SurfaceType::Road | SurfaceType::Runway => {
                        bail!("you can't spawn this unit on land")
                    }
                }
            }
            Ok(())
        }
        let land = Land::singleton(spctx.lua())?;
        let template_name = String::from(template_name);
        let template = spctx.get_template_ref(idx, GroupKind::Any, side, template_name.as_str())?;
        let mut gpos = compute_unit_positions(&spctx, idx, location.clone(), &template.group)?;
        let kind = GroupCategory::from_kind(template.category);
        let gid = GroupId::new();
        // naval spawn points need to be pre created in the miz, so they must be
        // spawned with the same name as the pre created group so that they move
        // to their destination.
        let group_name = if extra_tags.contains(UnitTag::NavalSpawnPoint) {
            template_name.clone()
        } else {
            String::from(format_compact!("{}-{}", template_name, gid))
        };
        let mut spawned = SpawnedGroup {
            id: gid,
            name: group_name.clone(),
            template_name: template_name.clone(),
            side,
            kind,
            origin,
            class: if extra_tags.contains(UnitTag::NavalSpawnPoint) {
                ObjGroupClass::Logi
            } else {
                ObjGroupClass::from(template_name.as_str())
            },
            units: SetS::new(),
            tags: UnitTags(BitFlags::empty()),
        };
        for unit in template.group.units()?.into_iter() {
            let unit = unit?;
            let typ = unit.typ()?;
            let tags = *self
                .ephemeral
                .cfg
                .unit_classification
                .get(typ.as_str())
                .ok_or_else(|| anyhow!("unit type not classified {typ}"))?;
            let tags = UnitTags(tags.0 | extra_tags);
            spawned.tags.0.insert(tags.0);
        }
        match &location {
            SpawnLoc::AtPos { .. }
            | SpawnLoc::AtPosWithCenter { .. }
            | SpawnLoc::AtPosWithComponents { .. }
            | SpawnLoc::AtTrigger { .. } => {
                if let Some(tmpl) = self.ephemeral.cfg.crate_template.get(&side)
                    && &template_name == tmpl
                {
                    () // it's ok to spawn crates on ships
                } else if spawned.tags.contains(UnitTag::Boat) {
                    check_land(&land, &gpos.positions, &gpos.by_type)
                        .with_context(|| format_compact!("placing group {group_name}"))?
                } else {
                    check_water(&land, &gpos.positions, &gpos.by_type)
                        .with_context(|| format_compact!("placing group {group_name}"))?
                }
            }
            SpawnLoc::InAir { .. } => (),
        }
        for unit in template.group.units()?.into_iter() {
            let uid = UnitId::new();
            let unit = unit?;
            let typ = unit.typ()?;
            let tags = *self
                .ephemeral
                .cfg
                .unit_classification
                .get(typ.as_str())
                .ok_or_else(|| anyhow!("unit type not classified {typ}"))?;
            let tags = UnitTags(tags.0 | extra_tags);
            let template_name = unit.name()?;
            let unit_name = if extra_tags.contains(UnitTag::NavalSpawnPoint) {
                template_name.clone()
            } else {
                String::from(format_compact!("{}-{}", group_name, uid))
            };
            let pos = match gpos.by_type.get_mut(&typ) {
                None => gpos.positions.pop_front().unwrap(),
                Some(positions) => positions.pop_front().unwrap(),
            };
            let position = {
                let mut p = Position3::default();
                p.p.x = pos.position.x;
                p.p.y = match pos.altitude {
                    None => land.get_height(LuaVec2(pos.position))?,
                    Some(alt) => alt,
                };
                p.p.z = pos.position.y;
                p
            };
            let spawned_unit = SpawnedUnit {
                id: uid,
                group: gid,
                side,
                typ: Vehicle(typ),
                tags,
                name: unit_name.clone(),
                template_name,
                spawn_position: position,
                spawn_pos: pos.position,
                spawn_heading: pos.heading,
                position,
                pos: pos.position,
                heading: pos.heading,
                dead: false,
                moved: None,
                airborne_velocity: None,
            };
            spawned.units.insert_cow(uid);
            self.persisted.units.insert_cow(uid, spawned_unit);
            self.persisted.units_by_name.insert_cow(unit_name, uid);
        }
        match &mut spawned.origin {
            DeployKind::ObjectiveDeprecated | DeployKind::Objective { .. } => (),
            DeployKind::Action { spec, .. } => {
                self.persisted.actions.insert_cow(gid);
                if let ActionKind::Drone(_) = &spec.kind {
                    self.persisted.jtacs.insert_cow(gid);
                }
            }
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
        extra_tags: BitFlags<UnitTag>,
        delay: Option<DateTime<Utc>>,
    ) -> Result<GroupId> {
        let gid = self.add_group(
            &spctx,
            idx,
            side,
            location,
            template_name,
            origin,
            extra_tags,
        )?;
        match delay {
            None => self.ephemeral.push_spawn(gid),
            Some(at) => self.ephemeral.delayspawnq.entry(at).or_default().push(gid),
        }
        Ok(gid)
    }

    pub(crate) fn unit_born(
        &mut self,
        lua: MizLua,
        unit: &Unit,
        connected: &Connected,
    ) -> Result<BirthRes> {
        let id = unit.object_id()?;
        let name = unit.get_name()?;
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            let unit = unit!(self, uid)?;
            self.ephemeral.uid_by_object_id.insert(id.clone(), *uid);
            self.ephemeral.object_id_by_uid.insert(*uid, id.clone());
            self.ephemeral
                .units_potentially_close_to_enemies
                .insert(*uid);
            if unit.tags.contains(UnitTag::Driveable) {
                self.ephemeral.units_able_to_move.insert(*uid);
            }
            self.ephemeral.stat(Stat::Unit {
                id: EnId::Unit(*uid),
                gid: Some(unit.group),
                owner: unit.side,
                typ: stats::Unit {
                    typ: unit.typ.clone(),
                    tags: unit.tags,
                },
                pos: stats::Pos {
                    pos: Coord::singleton(lua)?
                        .lo_to_ll(LuaVec3(Vector3::new(unit.pos.x, 0., unit.pos.y)))?,
                    velocity: unit.airborne_velocity.unwrap_or_default(),
                },
            });
            let gid = unit.group;
            if group_health!(self, gid)?.0 == 1 {
                self.mark_group(&gid)?
            }
            return Ok(BirthRes::None);
        }
        let slot = unit.slot()?;
        let (si, deferred_validate) = match self.ephemeral.slot_info.get(&slot) {
            Some(si) => (si, false),
            None => {
                // it's a dynamic slot
                let typ = Vehicle::from(unit.as_object()?.get_type_name()?);
                let pos = unit.get_ground_position()?;
                let obj = Db::objective_near_point(&self.persisted.objectives, pos.0, |_| true)
                    .map(|(_, _, o)| o)
                    .ok_or_else(|| anyhow!("dynamic slot not near any objective"))?;
                let gid = unit.get_group()?.id()?;
                let gid = miz::GroupId::from(gid.inner());
                self.ephemeral.slot_info.insert(
                    slot,
                    SlotInfo {
                        typ,
                        unit_name: unit.get_name()?,
                        objective: obj.id,
                        ground_start: false,
                        miz_gid: gid,
                        side: obj.owner,
                    },
                );
                self.ephemeral.slot_by_miz_gid.insert(gid, slot);
                (&self.ephemeral.slot_info[&slot], true)
            }
        };
        let name = unit.get_player_name()?;
        let ifo = name.and_then(|name| connected.get_by_name(&name));
        let ucid = match ifo {
            Some(ifo) => ifo.ucid,
            None => {
                error!("slot {slot} born with no player in it");
                unit.clone().destroy()?;
                return Ok(BirthRes::None);
            }
        };
        let side = si.side;
        let typ = si.typ.clone();
        let objective = si.objective;
        let tags = *self
            .ephemeral
            .cfg
            .unit_classification
            .get(&typ)
            .unwrap_or(&UnitTags::default());
        if deferred_validate {
            match self.try_occupy_slot_deferred(Utc::now(), &ucid, slot) {
                SlotAuth::Yes(typ) => {
                    self.ephemeral.stat(Stat::Slot {
                        id: ucid,
                        slot,
                        typ,
                    });
                }
                a => {
                    unit.clone().destroy()?;
                    return Ok(BirthRes::DynamicSlotDenied(ucid, a));
                }
            }
        }
        self.ephemeral.stat(Stat::Unit {
            id: EnId::Player(ucid),
            gid: None,
            owner: side,
            typ: stats::Unit { typ, tags },
            pos: stats::Pos {
                pos: Coord::singleton(lua)?.lo_to_ll(unit.get_point()?)?,
                velocity: Vector3::default(),
            },
        });
        self.player_entered_slot(lua, id, unit, slot, objective, ucid)
            .context("entering player into slot")?;
        Ok(BirthRes::OccupiedSlot(slot))
    }

    pub fn static_born(&mut self, st: &StaticObject) -> Result<()> {
        let id = st.object_id()?;
        let name = st.get_name()?;
        if let Some(uid) = self.persisted.units_by_name.get(name.as_str()) {
            self.ephemeral.uid_by_static.insert(id, *uid);
        }
        Ok(())
    }

    pub fn unit_dead(&mut self, id: &DcsOid<ClassUnit>, now: DateTime<Utc>) -> Result<()> {
        let uid = match self.ephemeral.unit_dead(&self.persisted, id) {
            None => return Ok(()),
            Some((uid, ucid)) => {
                if let Some(ucid) = ucid {
                    self.player_deslot(&ucid)
                }
                uid
            }
        };
        match self.persisted.units.get_mut_cow(&uid) {
            None => error!("unit_dead: missing unit {:?}", uid),
            Some(unit) => {
                unit.dead = true;
                unit.pos = unit.spawn_pos;
                unit.heading = unit.spawn_heading;
                unit.position = unit.spawn_position;
                self.ephemeral.dirty();
                let gid = unit.group;
                let health = group_health!(self, gid)?.0;
                if let Some(oid) = self.persisted.objectives_by_group.get(&gid).copied() {
                    self.update_objective_status(&oid, now)?;
                    self.ephemeral
                        .units_potentially_close_to_enemies
                        .remove(&uid);
                    if health == 0 {
                        if let Some(id) = self.ephemeral.group_marks.remove(&gid) {
                            self.ephemeral.msgs.delete_mark(id);
                        }
                    }
                }
                if self.persisted.deployed.contains(&gid)
                    || self.persisted.troops.contains(&gid)
                    || self.persisted.crates.contains(&gid)
                {
                    if health == 0 {
                        match &group!(self, gid)?.origin {
                            DeployKind::Troop {
                                player,
                                moved_by: Some((ucid, p)),
                                ..
                            }
                            | DeployKind::Deployed {
                                player,
                                moved_by: Some((ucid, p)),
                                ..
                            } => {
                                let owner = self.persisted.players[player].name.clone();
                                let ucid = ucid.clone();
                                let p = -(*p as i32);
                                let msg = format_compact!(
                                    "for the death of {gid} which was deployed by {owner} and moved by you"
                                );
                                self.adjust_points(&ucid, p, &msg)
                            }
                            DeployKind::Troop { .. }
                            | DeployKind::Deployed { .. }
                            | DeployKind::Action { .. }
                            | DeployKind::Crate { .. }
                            | DeployKind::Objective { .. }
                            | DeployKind::ObjectiveDeprecated => (),
                        }
                        self.delete_group(&gid)?
                    }
                }
                if self.persisted.actions.contains(&gid) {
                    if let DeployKind::Action { player, spec, .. } = &group!(self, gid)?.origin {
                        if self.group_health(&gid)?.0 == 0 {
                            if let Some((penalty, ucid)) = spec
                                .penalty
                                .and_then(|p| player.as_ref().map(|pl| (p, pl.clone())))
                            {
                                self.adjust_points(
                                    &ucid,
                                    -(penalty as i32),
                                    &format_compact!("for the loss of action group {gid}"),
                                )
                            }
                            self.delete_group(&gid)?
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn static_dead(&mut self, id: &DcsOid<ClassStatic>, now: DateTime<Utc>) -> Result<()> {
        if let Some(uid) = self.ephemeral.uid_by_static.remove(id) {
            match self.persisted.units.get_mut_cow(&uid) {
                None => error!("static_dead: missing unit {:?}", uid),
                Some(unit) => {
                    unit.dead = true;
                    let gid = unit.group;
                    self.ephemeral.dirty();
                    if let Some(oid) = self.persisted.objectives_by_group.get(&gid).copied() {
                        self.update_objective_status(&oid, now)?;
                    }
                    if self.persisted.deployed.contains(&gid)
                        || self.persisted.troops.contains(&gid)
                        || self.persisted.crates.contains(&gid)
                    {
                        if self.group_health(&gid)?.0 == 0 {
                            self.delete_group(&gid)?
                        }
                    }
                }
            }
        }
        Ok(())
    }

    pub fn group_health(&self, gid: &GroupId) -> Result<(usize, usize)> {
        group_health!(self, gid)
    }

    pub fn artillery_near_point(&self, side: Side, pos: Vector2) -> SmallVec<[GroupId; 8]> {
        let range2 = (self.ephemeral.cfg.artillery_mission_range as f64).powi(2);
        let artillery = self
            .deployed()
            .filter_map(|group| {
                if group.tags.contains(UnitTag::Artillery) && group.side == side {
                    let center = self.group_center(&group.id).ok()?;
                    if na::distance_squared(&center.into(), &pos.into()) <= range2 {
                        Some(group.id)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<SmallVec<[GroupId; 8]>>();
        artillery
    }

    pub fn update_unit_positions_incremental(
        &mut self,
        lua: MizLua,
        now: DateTime<Utc>,
        mut last: usize,
    ) -> Result<(usize, Vec<DcsOid<ClassUnit>>)> {
        let total = self.ephemeral.units_able_to_move.len();
        if last < total {
            let mut uids: SmallVec<[UnitId; 64]> = smallvec![];
            let elts = self.ephemeral.units_able_to_move.as_slice();
            let stop = last + max(1, total >> 4);
            while last < total && uids.len() < stop {
                uids.push(elts[last]);
                last += 1;
            }
            Ok((last, self.update_unit_positions(lua, now, &uids)?))
        } else {
            Ok((0, vec![]))
        }
    }

    pub fn update_unit_positions(
        &mut self,
        lua: MizLua,
        now: DateTime<Utc>,
        units: &[UnitId],
    ) -> Result<Vec<DcsOid<ClassUnit>>> {
        let coord = Coord::singleton(lua)?;
        let mut unit: Option<Unit> = None;
        let mut moved: SmallVec<[GroupId; 16]> = smallvec![];
        let mut dead: Vec<DcsOid<ClassUnit>> = vec![];
        for uid in units {
            let id = match self.ephemeral.object_id_by_uid.get(&uid) {
                Some(id) => id,
                None => {
                    warn!("update_unit_positions skipping unknown unit {uid}");
                    continue;
                }
            };
            let instance = match unit.take() {
                Some(unit) => unit.change_instance(id),
                None => Unit::get_instance(lua, id),
            };
            let instance = match instance {
                Ok(i) => i,
                Err(e) => {
                    warn!(
                        "update_unit_positions skipping invalid instance {uid}, {:?}",
                        e
                    );
                    dead.push(id.clone());
                    continue;
                }
            };
            let pos = instance.get_position()?;
            let spunit = unit_mut!(self, uid)?;
            if (spunit.position.p.0 - pos.p.0).magnitude_squared() > 1.0 {
                moved.push(spunit.group);
                spunit.moved = Some(now);
                spunit.position = pos;
                spunit.pos = Vector2::new(pos.p.x, pos.p.z);
                spunit.heading = azumith3d(pos.x.0);
                self.ephemeral
                    .units_potentially_close_to_enemies
                    .insert(*uid);
                let v = if spunit.tags.contains(UnitTag::Aircraft) && instance.in_air()? {
                    let v = instance.get_velocity()?.0;
                    spunit.airborne_velocity = Some(v);
                    Some(v)
                } else {
                    spunit.airborne_velocity = None;
                    None
                };
                self.ephemeral.stat(Stat::Position {
                    id: EnId::Unit(*uid),
                    pos: stats::Pos {
                        pos: coord.lo_to_ll(pos.p)?,
                        velocity: v.unwrap_or_default(),
                    },
                });
            }
            unit = Some(instance);
        }
        for gid in moved {
            self.ephemeral.dirty();
            self.mark_group(&gid)?;
        }
        Ok(dead)
    }
}
