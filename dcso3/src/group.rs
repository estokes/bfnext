use super::{as_tbl, coalition::Side, controller::Controller, cvt_err, unit::Unit, String};
use crate::{
    env::miz::{GroupId, GroupKind},
    simple_enum, wrapped_table, LuaEnv, MizLua, Sequence,
};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
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
    pub fn get_by_name(lua: MizLua<'lua>, name: &str) -> Result<Group<'lua>> {
        let globals = lua.inner().globals();
        let class = as_tbl("Group", None, globals.raw_get("Group")?)?;
        Ok(class.call_function("getByName", name)?)
    }

    pub fn destroy(&self) -> Result<()> {
        Ok(self.t.call_method("destroy", ())?)
    }

    pub fn activate(&self) -> Result<()> {
        Ok(self.t.call_method("activate", ())?)
    }

    pub fn get_category(&self) -> Result<GroupCategory> {
        Ok(self.t.call_method("getCategory", ())?)
    }

    pub fn get_coalition(&self) -> Result<Owner> {
        Ok(self.t.call_method("getCoalition", ())?)
    }

    pub fn get_name(&self) -> Result<String> {
        Ok(self.t.call_method("getName", ())?)
    }

    pub fn id(&self) -> Result<GroupId> {
        Ok(self.t.call_method("getID", ())?)
    }

    pub fn get_size(&self) -> Result<i64> {
        Ok(self.t.call_method("getSize", ())?)
    }

    pub fn get_initial_size(&self) -> Result<i64> {
        Ok(self.t.call_method("getInitialSize", ())?)
    }

    pub fn get_unit(&self, index: usize) -> Result<Unit> {
        Ok(self.t.call_method("getUnit", index)?)
    }

    pub fn get_units(&self) -> Result<Sequence<Unit>> {
        Ok(self.t.call_method("getUnits", ())?)
    }

    pub fn get_controller(&self) -> Result<Controller> {
        Ok(self.t.call_method("getController", ())?)
    }

    pub fn enable_emission(&self, on: bool) -> Result<()> {
        Ok(self.t.call_method("enableEmission", on)?)
    }
}
