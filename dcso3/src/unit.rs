use super::{as_tbl, controller::Controller, cvt_err, group::Group, object::Object, String};
use crate::{simple_enum, wrapped_table};
use mlua::{prelude::*, Value};
use serde_derive::{Serialize, Deserialize};
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
    pub fn get_by_name(lua: &'lua Lua, name: &str) -> LuaResult<Unit<'lua>> {
        let globals = lua.globals();
        let unit = as_tbl("Unit", Some("Unit"), globals.raw_get("Unit")?)?;
        Self::from_lua(unit.call_method("getByName", name)?, lua)
    }

    pub fn as_object(&self) -> LuaResult<Object<'lua>> {
        Object::from_lua(Value::Table(self.t.clone()), self.lua)
    }

    pub fn is_active(&self) -> LuaResult<bool> {
        self.t.call_method("isActive", ())
    }

    pub fn get_player_name(&self) -> LuaResult<String> {
        self.t.call_method("getPlayerName", ())
    }

    pub fn get_id(&self) -> LuaResult<u32> {
        self.t.call_method("getID", ())
    }

    pub fn get_number(&self) -> LuaResult<u32> {
        self.t.call_method("getNumber", ())
    }

    pub fn get_object_id(&self) -> LuaResult<i64> {
        self.t.call_method("getObjectID", ())
    }

    pub fn get_controller(&self) -> LuaResult<Controller<'lua>> {
        Controller::from_lua(self.t.call_method("getController", ())?, self.lua)
    }

    pub fn get_group(&self) -> LuaResult<Group<'lua>> {
        Group::from_lua(self.t.call_method("getGroup", ())?, self.lua)
    }

    pub fn get_callsign(&self) -> LuaResult<String> {
        self.t.call_method("getCallsign", ())
    }

    pub fn get_life(&self) -> LuaResult<i32> {
        self.t.call_method("getLife", ())
    }

    pub fn get_life0(&self) -> LuaResult<i32> {
        self.t.call_method("getLife0", ())
    }

    pub fn get_fuel(&self) -> LuaResult<f32> {
        self.t.call_method("getFuel", ())
    }

    pub fn enable_emission(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("enableEmission", on)
    }

    pub fn get_category(&self) -> LuaResult<UnitCategory> {
        self.t.call_method("getCategory", ())
    }
}
