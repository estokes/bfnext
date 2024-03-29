/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::{as_tbl, object::Object};
use crate::{
    object::{DcsObject, DcsOid},
    wrapped_table, LuaEnv, LuaVec3, MizLua,
};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::{marker::PhantomData, ops::Deref};

wrapped_table!(Spot, Some("Spot"));

impl<'lua> Spot<'lua> {
    pub fn create_laser(
        lua: MizLua<'lua>,
        source: Object<'lua>,
        local_ref: Option<LuaVec3>,
        target: LuaVec3,
        code: u16,
    ) -> Result<Self> {
        let globals = lua.inner().globals();
        let spot: LuaTable = globals.raw_get("Spot")?;
        Ok(spot.call_function("createLaser", (source, local_ref, target, code))?)
    }

    pub fn create_infra_red(
        lua: MizLua<'lua>,
        source: Object<'lua>,
        local_ref: Option<LuaVec3>,
        target: LuaVec3,
    ) -> Result<Self> {
        let globals = lua.inner().globals();
        let spot: LuaTable = globals.raw_get("Spot")?;
        Ok(spot.call_function("createInfraRed", (source, local_ref, target))?)
    }

    pub fn destroy(self) -> Result<()> {
        Ok(self.t.call_method("destroy", ())?)
    }

    pub fn get_point(&self) -> Result<LuaVec3> {
        Ok(self.t.call_method("getPoint", ())?)
    }

    pub fn set_point(&self, target: LuaVec3) -> Result<()> {
        Ok(self.t.call_method("setPoint", target)?)
    }

    pub fn get_code(&self) -> Result<u16> {
        Ok(self.t.call_method("getCode", ())?)
    }

    pub fn set_code(&self, code: u16) -> Result<()> {
        Ok(self.t.call_method("setCode", code)?)
    }
}

#[derive(Debug, Clone)]
pub struct ClassSpot;

impl<'lua> DcsObject<'lua> for Spot<'lua> {
    type Class = ClassSpot;

    fn get_instance(lua: MizLua<'lua>, id: &DcsOid<Self::Class>) -> Result<Self> {
        let t = lua.inner().create_table()?;
        t.set_metatable(Some(lua.inner().globals().raw_get(&**id.class)?));
        t.raw_set("id_", id.id)?;
        let t = Self {
            t,
            lua: lua.inner(),
        };
        Ok(t)
    }

    fn get_instance_dyn<T>(lua: MizLua<'lua>, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(lua, "Spot")?;
        let id = DcsOid {
            id: id.id,
            class: id.class.clone(),
            t: PhantomData,
        };
        Self::get_instance(lua, &id)
    }

    fn change_instance(self, id: &DcsOid<Self::Class>) -> Result<Self> {
        self.raw_set("id_", id.id)?;
        Ok(self)
    }

    fn change_instance_dyn<T>(self, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(MizLua(self.lua), "Spot")?;
        self.t.raw_set("id_", id.id)?;
        Ok(self)
    }
}
