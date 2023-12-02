use crate::{simple_enum, wrapped_table, Sequence, env::miz::GroupKind, MizLua, LuaEnv};
use super::{as_tbl, coalition::Side, controller::Controller, cvt_err, unit::Unit, String};
use mlua::{prelude::*, Value};
use serde_derive::{Serialize, Deserialize};
use std::ops::Deref;

simple_enum!(GroupCategory, u8, [
    Airplane => 0,
    Helicopter => 1,
    Ground => 2,
    Ship => 3,
    Train => 4
]);

impl GroupCategory {
    pub fn from_kind(k: GroupKind) -> Option<Self> {
        match k {
            GroupKind::Any | GroupKind::Static => None,
            GroupKind::Plane => Some(Self::Airplane),
            GroupKind::Helicopter => Some(Self::Helicopter),
            GroupKind::Vehicle => Some(Self::Ground),
            GroupKind::Ship => Some(Self::Ship),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum Owner {
    Contested,
    Side(Side),
}

impl<'lua> FromLua<'lua> for Owner {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match i64::from_lua(value.clone(), lua)? {
            3 => Ok(Self::Contested),
            _ => Ok(Owner::Side(Side::from_lua(value, lua)?)),
        }
    }
}

wrapped_table!(Group, Some("Group"));

impl<'lua> Group<'lua> {
    pub fn get_by_name(lua: MizLua<'lua>, name: &str) -> LuaResult<Group<'lua>> {
        let globals = lua.inner().globals();
        let class = as_tbl("Group", Some("Group"), globals.raw_get("Group")?)?;
        Self::from_lua(class.call_method("getByName", name)?, lua.inner())
    }

    pub fn destroy(&self) -> LuaResult<()> {
        self.t.call_method("destroy", ())
    }

    pub fn activate(&self) -> LuaResult<()> {
        self.t.call_method("activate", ())
    }

    pub fn get_category(&self) -> LuaResult<GroupCategory> {
        self.t.call_method("getCategory", ())
    }

    pub fn get_coalition(&self) -> LuaResult<Owner> {
        self.t.call_method("getCoalition", ())
    }

    pub fn get_name(&self) -> LuaResult<String> {
        self.t.call_method("getName", ())
    }

    pub fn get_id(&self) -> LuaResult<i64> {
        self.t.call_method("getID", ())
    }

    pub fn get_size(&self) -> LuaResult<i64> {
        self.t.call_method("getSize", ())
    }

    pub fn get_initial_size(&self) -> LuaResult<i64> {
        self.t.call_method("getInitialSize", ())
    }

    pub fn get_unit(&self, index: usize) -> LuaResult<Unit> {
        Unit::from_lua(self.t.call_method("getUnit", index)?, self.lua)
    }

    pub fn get_units(&self) -> LuaResult<Sequence<Unit>> {
        self.t.call_method("getUnits", ())
    }

    pub fn get_controller(&self) -> LuaResult<Controller> {
        Ok(Controller::from_lua(
            self.t.call_method("getController", ())?,
            self.lua,
        )?)
    }

    pub fn enable_emission(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("enableEmission", on)
    }
}
