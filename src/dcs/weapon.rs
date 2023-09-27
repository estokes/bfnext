use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use crate::wrapped_table;
use super::{as_tbl, object::Object, unit::Unit};
use std::ops::Deref;

wrapped_table!(Weapon, Some("Weapon"));

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
