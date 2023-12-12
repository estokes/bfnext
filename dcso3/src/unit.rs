use super::{as_tbl, controller::Controller, cvt_err, group::Group, object::Object, String};
use crate::{env::miz::UnitId, simple_enum, wrapped_table, LuaEnv, MizLua};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;

simple_enum!(UnitCategory, u8, [
    Airplane => 0,
    GroundUnit => 2,
    Helicopter => 1,
    Ship => 3,
    Structure => 4
]);

wrapped_table!(Unit, Some("Unit"));

impl<'lua> Unit<'lua> {
    pub fn get_by_name(lua: MizLua<'lua>, name: &str) -> Result<Unit<'lua>> {
        let globals = lua.inner().globals();
        let unit = as_tbl("Unit", None, globals.raw_get("Unit")?)?;
        Ok(unit.call_function("getByName", name)?)
    }

    pub fn as_object(&self) -> Result<Object<'lua>> {
        Ok(Object::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }

    pub fn is_active(&self) -> Result<bool> {
        Ok(self.t.call_method("isActive", ())?)
    }

    pub fn get_player_name(&self) -> Result<Option<String>> {
        Ok(self.t.call_method("getPlayerName", ())?)
    }

    pub fn id(&self) -> Result<UnitId> {
        Ok(self.t.call_method("getID", ())?)
    }

    pub fn get_number(&self) -> Result<i64> {
        Ok(self.t.call_method("getNumber", ())?)
    }

    pub fn get_object_id(&self) -> Result<i64> {
        Ok(self.t.call_method("getObjectID", ())?)
    }

    pub fn get_controller(&self) -> Result<Controller<'lua>> {
        Ok(self.t.call_method("getController", ())?)
    }

    pub fn get_group(&self) -> Result<Group<'lua>> {
        Ok(self.t.call_method("getGroup", ())?)
    }

    pub fn get_callsign(&self) -> Result<String> {
        Ok(self.t.call_method("getCallsign", ())?)
    }

    pub fn get_life(&self) -> Result<i32> {
        Ok(self.t.call_method("getLife", ())?)
    }

    pub fn get_life0(&self) -> Result<i32> {
        Ok(self.t.call_method("getLife0", ())?)
    }

    pub fn get_fuel(&self) -> Result<f32> {
        Ok(self.t.call_method("getFuel", ())?)
    }

    pub fn enable_emission(&self, on: bool) -> Result<()> {
        Ok(self.t.call_method("enableEmission", on)?)
    }

    pub fn get_category(&self) -> Result<UnitCategory> {
        Ok(self.t.call_method("getCategory", ())?)
    }
}
