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

use anyhow::{anyhow, Context, Result};
use chrono::Utc;
use compact_str::format_compact;
use dcso3::{
    coalition::{Coalition, Side, Static},
    env::miz::{self, GroupInfo, GroupKind, Miz, MizIndex, TriggerZone},
    group::{ClassGroup, Group, GroupCategory},
    land::Land,
    object::{DcsObject, DcsOid, ObjectCategory},
    world::{SearchVolume, World},
    DeepClone, LuaEnv, LuaVec2, LuaVec3, MizLua, String, Vector2, Vector3,
};
use fxhash::FxHashMap;
use log::info;
use mlua::Value;
use serde_derive::{Deserialize, Serialize};

use crate::perf::{record_perf, PerfInner};

fn default_speed() -> f64 {
    220.
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnLoc {
    /// only for air units, obviously
    InAir {
        pos: Vector2,
        heading: f64,
        altitude: f64,
        #[serde(default = "default_speed")]
        speed: f64,
    },
    AtPos {
        /// the position of the player. the group will be offset in the
        /// direction offset_direction from this point by the group radius + 10 meters
        pos: Vector2,
        /// this should be a unit vector pointing in the direction
        /// you want to offset the group
        offset_direction: Vector2,
        /// rotate the group to this heading in radians
        group_heading: f64,
    },
    AtPosWithComponents {
        pos: Vector2,
        /// the position of sub components of the group by unit type
        component_pos: FxHashMap<String, Vector2>,
        /// rotate the group to this heading in radians
        group_heading: f64,
    },
    /// spawn the group as a direct translation from an original (provided) center
    /// to a new center. This is useful if you have statics, or multiple groups,
    /// and you want their relative positions to be preserved
    AtPosWithCenter {
        /// pos is the new center position of the group
        pos: Vector2,
        /// center is the original center of the group
        center: Vector2,
    },
    AtTrigger {
        name: String,
        /// rotate the group to this heading in radians
        group_heading: f64,
    },
}

impl Default for SpawnLoc {
    fn default() -> Self {
        Self::AtPos {
            pos: Vector2::new(0., 0.),
            offset_direction: Vector2::new(0., 0.),
            group_heading: 0.,
        }
    }
}

pub struct SpawnCtx<'lua> {
    coalition: Coalition<'lua>,
    miz: Miz<'lua>,
    lua: MizLua<'lua>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Despawn {
    Group(DcsOid<ClassGroup>),
    Static(String),
}

#[derive(Debug, Clone)]
pub enum Spawned<'lua> {
    Group(Group<'lua>),
    Static,
}

impl<'lua> SpawnCtx<'lua> {
    pub fn new(lua: MizLua<'lua>) -> Result<Self> {
        Ok(Self {
            coalition: Coalition::singleton(lua)?,
            miz: Miz::singleton(lua)?,
            lua,
        })
    }

    pub fn lua(&self) -> MizLua<'lua> {
        self.lua
    }

    pub fn get_template(
        &self,
        idx: &MizIndex,
        kind: GroupKind,
        side: Side,
        template_name: &str,
    ) -> Result<GroupInfo<'lua>> {
        let mut template = self
            .miz
            .get_group_by_name(idx, kind, side, template_name)?
            .ok_or_else(|| anyhow!("no such template {template_name}"))?;
        template.group = template.group.deep_clone(self.lua.inner())?;
        Ok(template)
    }

    /// get at template that you pinky promise not to modify
    pub fn get_template_ref(
        &self,
        idx: &MizIndex,
        kind: GroupKind,
        side: Side,
        template_name: &str,
    ) -> Result<GroupInfo> {
        self.miz
            .get_group_by_name(idx, kind, side, template_name)?
            .ok_or_else(|| anyhow!("no such template {template_name}"))
    }

    pub fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> Result<TriggerZone> {
        Ok(self
            .miz
            .get_trigger_zone(idx, name)?
            .ok_or_else(|| anyhow!("no such trigger zone {name}"))?)
    }

    pub fn move_farp_pad(
        &self,
        perf: &mut PerfInner,
        idx: &MizIndex,
        side: Side,
        pad_template: &str,
        pos: Vector2,
    ) -> Result<Spawned<'lua>> {
        let pad = {
            let pad = self
                .get_template(idx, GroupKind::Any, side, &pad_template)
                .context("getting the pad")?;
            pad.group.set("hidden", false)?;
            let pad_unit = pad
                .group
                .units()
                .context("getting pad units")?
                .get(1)
                .context("getting pad unit")?;
            pad_unit.set_pos(pos).context("setting pad pos")?;
            drop(pad_unit);
            pad
        };
        self.spawn(perf, pad).context("moving the pad")
    }

    pub fn spawn(&self, perf: &mut PerfInner, template: GroupInfo<'lua>) -> Result<Spawned<'lua>> {
        match GroupCategory::from_kind(template.category) {
            Some(category) => {
                let ts = Utc::now();
                let res = Ok(Spawned::Group(
                    self.coalition
                        .add_group(template.country, category, template.group.clone())
                        .with_context(|| {
                            format_compact!("spawning group from template {:?}", template)
                        })?,
                ));
                record_perf(&mut perf.add_group, ts);
                res
            }
            None => {
                // static objects are not fed to addStaticObject as groups
                let unit: miz::Unit<'lua> = template
                    .group
                    .units()
                    .context("getting static group units")?
                    .first()
                    .context("getting first unit in static group")?
                    .clone();
                let ts = Utc::now();
                self.coalition
                    .add_static_object(template.country, unit)
                    .with_context(|| {
                        format_compact!("spawning static object from template {:?}", template)
                    })?;
                record_perf(&mut perf.add_static_object, ts);
                Ok(Spawned::Static)
            }
        }
    }

    pub fn despawn(&self, perf: &mut PerfInner, name: Despawn) -> Result<()> {
        let ts = Utc::now();
        match name {
            Despawn::Group(oid) => {
                match dcso3::group::Group::get_instance(self.lua, &oid) {
                    Ok(group) => group.destroy()?,
                    Err(e) => info!("attempt to despawn invalid group {e:?}"),
                }
                record_perf(&mut perf.despawn, ts);
                Ok(())
            }
            Despawn::Static(name) => {
                match dcso3::static_object::StaticObject::get_by_name(self.lua, &*name) {
                    Ok(Static::Airbase(obj)) => obj.destroy()?,
                    Ok(Static::Static(obj)) => obj.destroy()?,
                    Err(e) => info!("attempt to despawn unknown static {} {}", name, e),
                }
                record_perf(&mut perf.despawn, ts);
                Ok(())
            }
        }
    }

    pub fn remove_junk(&self, point: Vector2, radius: f64) -> Result<()> {
        let alt = Land::singleton(self.lua)?.get_height(LuaVec2(point))?;
        let point = LuaVec3(Vector3::new(point.x, alt, point.y));
        let vol = SearchVolume::Sphere { point, radius };
        World::singleton(self.lua)?.remove_junk(vol)?;
        Ok(())
    }

    #[allow(dead_code)]
    pub fn remove_scenery(&self, point: Vector2, radius: f64) -> Result<()> {
        let alt = Land::singleton(self.lua)?.get_height(LuaVec2(point))?;
        let point = LuaVec3(Vector3::new(point.x, alt, point.y));
        let vol = SearchVolume::Sphere { point, radius };
        World::singleton(self.lua)?.search_objects(
            ObjectCategory::Scenery,
            vol,
            Value::Nil,
            |_, o, _| {
                o.destroy()?;
                Ok(true)
            },
        )?;
        Ok(())
    }
}
