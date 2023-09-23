use super::as_tbl;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Warehouse<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Warehouse<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Warehouse", Some("Warehouse"), value)?,
            lua,
        })
    }
}
