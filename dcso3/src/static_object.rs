use super::{as_tbl, coalition::Side, country::Country, object::Object};
use crate::wrapped_table;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(StaticObject, Some("StaticObject"));

impl<'lua> StaticObject<'lua> {
    pub fn get_by_name(lua: &'lua Lua, name: &str) -> LuaResult<Self> {
        let globals = lua.globals();
        let unit = as_tbl(
            "StaticObject",
            Some("StaticObject"),
            globals.raw_get("StaticObject")?,
        )?;
        Self::from_lua(unit.call_method("getByName", name)?, lua)
    }

    pub fn get_coalition(&self) -> LuaResult<Side> {
        self.t.call_method("getCoalition", ())
    }

    pub fn get_country(&self) -> LuaResult<Country> {
        self.t.call_method("getCountry", ())
    }

    pub fn as_object(&self) -> LuaResult<Object<'lua>> {
        Object::from_lua(Value::Table(self.t.clone()), self.lua)
    }
}
