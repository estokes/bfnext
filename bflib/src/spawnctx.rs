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
use compact_str::format_compact;
use dcso3::{
    coalition::{Coalition, Side, Static},
    env::miz::{GroupInfo, GroupKind, Miz, MizIndex, TriggerZone},
    group::GroupCategory,
    land::Land,
    world::{SearchVolume, World},
    DeepClone, LuaEnv, LuaVec2, LuaVec3, MizLua, String, Vector2, Vector3,
};
use fxhash::FxHashMap;
use log::error;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnLoc {
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

pub struct SpawnCtx<'lua> {
    coalition: Coalition<'lua>,
    miz: Miz<'lua>,
    lua: MizLua<'lua>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Despawn {
    Group(String),
    Static(String),
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
    ) -> Result<GroupInfo> {
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

    pub fn spawn(&self, template: GroupInfo) -> Result<()> {
        match GroupCategory::from_kind(template.category) {
            Some(category) => {
                self.coalition
                    .add_group(template.country, category, template.group.clone())
                    .with_context(|| {
                        format_compact!("spawning group from template {:?}", template)
                    })?;
            }
            None => {
                // static objects are not fed to addStaticObject as groups
                let unit = template
                    .group
                    .units()
                    .context("getting static group units")?
                    .first()
                    .context("getting first unit in static group")?;
                self.coalition
                    .add_static_object(template.country, unit)
                    .with_context(|| {
                        format_compact!("spawning static object from template {:?}", template)
                    })?;
            }
        }
        Ok(())
    }

    pub fn despawn(&self, name: Despawn) -> Result<()> {
        match name {
            Despawn::Group(name) => {
                match dcso3::group::Group::get_by_name(self.lua, &*name) {
                    Ok(group) => group.destroy()?,
                    Err(e) => error!("attempt to despawn unknown group {} {}", name, e),
                }
                Ok(())
            }
            Despawn::Static(name) => {
                match dcso3::static_object::StaticObject::get_by_name(self.lua, &*name) {
                    Ok(Static::Airbase(obj)) => obj.destroy()?,
                    Ok(Static::Static(obj)) => obj.destroy()?,
                    Err(e) => error!("attempt to despawn unknown static {} {}", name, e),
                }
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
}
