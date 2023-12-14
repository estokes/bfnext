use super::{as_tbl, coalition::Side, country::Country, object::Object};
use crate::{wrapped_table, LuaEnv, MizLua};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(StaticObject, Some("StaticObject"));

impl<'lua> StaticObject<'lua> {
    pub fn get_by_name(lua: MizLua<'lua>, name: &str) -> Result<Self> {
        let globals = lua.inner().globals();
        let sobj = as_tbl("StaticObject", None, globals.raw_get("StaticObject")?)?;
        Ok(sobj.call_function("getByName", name)?)
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
}
