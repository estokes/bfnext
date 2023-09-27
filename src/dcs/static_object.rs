use std::ops::Deref;
use super::{
    airbase::Airbase,
    as_tbl,
    country::Country,
    cvt_err,
    group::{Group, GroupCategory},
    unit::Unit,
};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct StaticObject<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua
}

impl<'lua> Deref for StaticObject<'lua> {
    type Target = mlua::Table<'lua>;

    fn deref(&self) -> &Self::Target {
        &self.t
    }
}

impl<'lua> FromLua<'lua> for StaticObject<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("StaticObject", None, value)?,
            lua
        })
    }
}

impl<'lua> IntoLua<'lua> for StaticObject<'lua> {
    fn into_lua(self, _lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Table(self.t))
    }
}