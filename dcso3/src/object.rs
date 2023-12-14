use super::{as_tbl, cvt_err, unit::Unit, weapon::Weapon, Position3, String, LuaVec3};
use crate::{simple_enum, wrapped_table};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::{Serialize, Deserialize};
use std::ops::Deref;

simple_enum!(ObjectCategory, u8, [
    Void => 0,
    Unit => 1,
    Weapon => 2,
    Static => 3,
    Base => 4,
    Scenery => 5,
    Cargo => 6
]);

wrapped_table!(Object, Some("Object"));

impl<'lua> Object<'lua> {
    pub fn destroy(&self) -> Result<()> {
        Ok(self.t.call_method("destroy", ())?)
    }

    pub fn get_category(&self) -> Result<ObjectCategory> {
        Ok(self.t.call_method("getCategory", ())?)
    }

    pub fn get_desc(&self) -> Result<mlua::Table<'lua>> {
        Ok(self.t.call_method("getDesc", ())?)
    }

    pub fn get_name(&self) -> Result<String> {
        Ok(self.t.call_method("getName", ())?)
    }

    pub fn get_point(&self) -> Result<LuaVec3> {
        Ok(self.t.call_method("getPoint", ())?)
    }

    pub fn get_position(&self) -> Result<Position3> {
        Ok(self.t.call_method("getPosition", ())?)
    }

    pub fn get_velocity(&self) -> Result<LuaVec3> {
        Ok(self.t.call_method("getPosition", ())?)
    }

    pub fn in_air(&self) -> Result<bool> {
        Ok(self.t.call_method("inAir", ())?)
    }

    pub fn as_unit(&self) -> Result<Unit<'lua>> {
        Ok(Unit::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }

    pub fn as_weapon(&self) -> Result<Weapon<'lua>> {
        Ok(Weapon::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }
}
