use super::{as_tbl, controller::Controller, cvt_err, unit::Unit, String};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum GroupCategory {
    Airplane,
    Ground,
    Helicopter,
    Ship,
    Train,
}

#[derive(Debug, Clone, Serialize)]
pub enum Coalition {
    Neutral,
    Red,
    Blue,
    Contested,
}

#[derive(Debug, Clone, Serialize)]
pub struct Group<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Group<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Group", Some("Group"), value)?,
            lua,
        })
    }
}

impl<'lua> Group<'lua> {
    pub fn get_by_name(lua: &'lua Lua, name: &str) -> LuaResult<Group<'lua>> {
        let globals = lua.globals();
        let unit = as_tbl("Group", Some("Group"), globals.raw_get("Group")?)?;
        Self::from_lua(unit.call_method("getByName", name)?, lua)
    }

    pub fn destroy(&self) -> LuaResult<()> {
        self.t.call_method("destroy", ())
    }

    pub fn activate(&self) -> LuaResult<()> {
        self.t.call_method("activate", ())
    }

    pub fn get_category(&self) -> LuaResult<GroupCategory> {
        Ok(match self.t.call_method("getCategory", ())? {
            0 => GroupCategory::Airplane,
            1 => GroupCategory::Ground,
            2 => GroupCategory::Helicopter,
            3 => GroupCategory::Ship,
            4 => GroupCategory::Train,
            _ => return Err(cvt_err("GroupCategory")),
        })
    }

    pub fn get_coalition(&self) -> LuaResult<Coalition> {
        Ok(match self.t.call_method("getCoalition", ())? {
            0 => Coalition::Neutral,
            1 => Coalition::Red,
            2 => Coalition::Blue,
            3 => Coalition::Contested,
            _ => return Err(cvt_err("Coalition")),
        })
    }

    pub fn get_name(&self) -> LuaResult<String> {
        self.t.call_method("getName", ())
    }

    pub fn get_id(&self) -> LuaResult<u32> {
        self.t.call_method("getID", ())
    }

    pub fn get_size(&self) -> LuaResult<u32> {
        self.t.call_method("getSize", ())
    }

    pub fn get_initial_size(&self) -> LuaResult<u32> {
        self.t.call_method("getInitialSize", ())
    }

    pub fn get_unit(&self, index: usize) -> LuaResult<Unit> {
        Unit::from_lua(self.t.call_method("getUnit", index)?, self.lua)
    }

    pub fn get_units(&self) -> LuaResult<impl Iterator<Item = LuaResult<Unit>>> {
        Ok(as_tbl("Units", None, self.t.call_method("getUnits", ())?)?.sequence_values())
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
