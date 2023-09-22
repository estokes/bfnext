use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use super::{as_tbl, String, controller::Controller, group::Group, object::Object};

#[derive(Debug, Clone, Serialize)]
pub enum UnitCategory {
    Airplane,
    Helicopter,
    GroundUnit,
    Ship,
    Structure,
}

#[derive(Debug, Clone, Serialize)]
pub struct Unit<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Unit<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Unit", Some("Unit"), value)?,
            lua,
        })
    }
}

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

    pub fn get_object_id(&self) -> LuaResult<u32> {
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
}