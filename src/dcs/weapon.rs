use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use super::{as_tbl, object::Object, unit::Unit};

#[derive(Debug, Clone, Serialize)]
pub struct Weapon<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Weapon<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Weapon", Some("Weapon"), value)?,
            lua,
        })
    }
}

impl<'lua> Weapon<'lua> {
    pub fn as_object(&self) -> LuaResult<Object<'lua>> {
        Object::from_lua(Value::Table(self.t.clone()), self.lua)
    }

    pub fn get_launcher(&self) -> LuaResult<Unit<'lua>> {
        Unit::from_lua(self.t.call_method("getLauncher", ())?, self.lua)
    }

    pub fn get_target(&self) -> LuaResult<Option<Object<'lua>>> {
        match self.t.call_method("getTarget", ())? {
            Value::Nil => Ok(None),
            v => Ok(Some(Object::from_lua(v, self.lua)?)),
        }
    }
}
