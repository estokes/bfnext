/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::{as_tbl, cvt_err, LuaVec3};
use crate::{simple_enum, wrapped_table, LuaEnv, LuaVec2, MizLua, Sequence};
use anyhow::Result;
use mlua::{prelude::*, Value};
use na::Vector2;
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;

simple_enum!(SurfaceType, u8, [
    Land => 1,
    ShallowWater => 2,
    Water => 3,
    Road => 4,
    Runway => 5
]);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum RoadType {
    Road,
    Rail,
}

wrapped_table!(Land, None);

impl<'lua> Land<'lua> {
    pub fn singleton(lua: MizLua<'lua>) -> Result<Self> {
        Ok(lua.inner().globals().raw_get("land")?)
    }

    pub fn get_height(&self, p: LuaVec2) -> Result<f64> {
        Ok(self.t.call_function("getHeight", p)?)
    }

    pub fn get_surface_height_with_seabed(&self, p: LuaVec2) -> Result<(f64, f64)> {
        Ok(self.t.call_function("getSurfaceHeightWithSeabed", p)?)
    }

    pub fn get_surface_type(&self, p: LuaVec2) -> Result<SurfaceType> {
        Ok(self.t.call_function("getSurfaceType", p)?)
    }

    pub fn is_visible(&self, origin: LuaVec3, destination: LuaVec3) -> Result<bool> {
        Ok(self.t.call_function("isVisible", (origin, destination))?)
    }

    pub fn get_ip(&self, origin: LuaVec3, direction: LuaVec3, distance: f64) -> Result<LuaVec3> {
        Ok(self
            .t
            .call_function("getIP", (origin, direction, distance))?)
    }

    pub fn get_profile(&self, origin: LuaVec3, destination: LuaVec3) -> Result<Sequence<LuaVec3>> {
        Ok(self.t.call_function("profile", (origin, destination))?)
    }

    pub fn get_closest_point_on_roads(&self, typ: RoadType, from: LuaVec2) -> Result<LuaVec2> {
        let typ = match typ {
            RoadType::Road => "roads",
            RoadType::Rail => "railroads",
        };
        let (x, y) = self
            .t
            .call_function("getClosestPointOnRoads", (typ, from.x, from.y))?;
        Ok(LuaVec2(Vector2::new(x, y)))
    }

    pub fn find_path_on_roads(
        &self,
        typ: RoadType,
        origin: LuaVec2,
        destination: LuaVec2,
    ) -> Result<Sequence<LuaVec2>> {
        let typ = match typ {
            RoadType::Road => "roads",
            RoadType::Rail => "rails",
        };
        Ok(self.t.call_function(
            "findPathOnRoads",
            (typ, origin.x, origin.y, destination.x, destination.y),
        )?)
    }
}
