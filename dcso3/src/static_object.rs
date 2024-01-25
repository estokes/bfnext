/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::{as_tbl, coalition::Side, country::Country, object::Object};
use crate::{airbase::Airbase, coalition::Static, wrapped_table, LuaEnv, MizLua, wrapped_prim};
use anyhow::{anyhow, Result};
use mlua::{prelude::*, Value};
use serde_derive::{Serialize, Deserialize};
use std::ops::Deref;

wrapped_prim!(StaticObjectId, i64, Hash, Copy);

wrapped_table!(StaticObject, Some("StaticObject"));

impl<'lua> StaticObject<'lua> {
    pub fn get_by_name(lua: MizLua<'lua>, name: &str) -> Result<Static<'lua>> {
        let globals = lua.inner().globals();
        let sobj = as_tbl("StaticObject", None, globals.raw_get("StaticObject")?)?;
        let tbl: LuaTable = sobj.call_function("getByName", name)?;
        let mt = tbl
            .get_metatable()
            .ok_or_else(|| anyhow!("returned static object has no meta table"))?;
        if mt.raw_get::<_, String>("className_")?.as_str() == "Airbase" {
            Ok(Static::Airbase(Airbase::from_lua(
                Value::Table(tbl),
                lua.inner(),
            )?))
        } else {
            Ok(Static::Static(StaticObject::from_lua(
                Value::Table(tbl),
                lua.inner(),
            )?))
        }
    }

    pub fn destroy(self) -> Result<()> {
        Ok(self.t.call_method("destroy", ())?)
    }

    pub fn id(&self) -> Result<StaticObjectId> {
        Ok(self.t.call_method("getID", ())?)
    }

    pub fn get_coalition(&self) -> Result<Side> {
        Ok(self.t.call_method("getCoalition", ())?)
    }

    pub fn get_country(&self) -> Result<Country> {
        Ok(self.t.call_method("getCountry", ())?)
    }

    pub fn as_object(&self) -> Result<Object<'lua>> {
        Ok(Object::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }

    pub fn get_desc(&self) -> Result<mlua::Table<'lua>> {
        Ok(self.t.call_method("getDesc", ())?)
    }
}
