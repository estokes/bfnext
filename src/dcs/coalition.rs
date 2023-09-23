use super::{
    airbase::Airbase,
    as_tbl,
    country::Country,
    cvt_err,
    group::{Group, GroupCategory},
    unit::Unit,
};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
#[repr(u8)]
pub enum Side {
    Neutral = 0,
    Red = 1,
    Blue = 2,
}

impl<'lua> FromLua<'lua> for Side {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(match u8::from_lua(value, lua)? {
            0 => Side::Neutral,
            1 => Side::Red,
            2 => Side::Blue,
            _ => return Err(cvt_err("side")),
        })
    }
}

impl<'lua> IntoLua<'lua> for Side {
    fn into_lua(self, _: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Integer(self as i64))
    }
}

#[derive(Debug, Clone, Serialize)]
#[repr(u8)]
pub enum Service {
    Atc = 0,
    Awacs = 1,
    Fac = 3,
    Tanker = 2,
}

impl<'lua> FromLua<'lua> for Service {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(match u8::from_lua(value, lua)? {
            0 => Self::Atc,
            1 => Self::Awacs,
            2 => Self::Tanker,
            3 => Self::Fac,
            _ => return Err(cvt_err("Service")),
        })
    }
}

impl<'lua> IntoLua<'lua> for Service {
    fn into_lua(self, _: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Integer(self as i64))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Coalition<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Coalition<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("coalition", None, value)?,
            lua,
        })
    }
}

impl<'lua> Coalition<'lua> {
    pub fn get(lua: &'lua Lua) -> LuaResult<Self> {
        let globals = lua.globals();
        Ok(Self {
            t: as_tbl("coalition", None, globals.raw_get("coalition")?)?,
            lua,
        })
    }

    pub fn add_group(
        &self,
        country: Country,
        category: GroupCategory,
        data: Group,
    ) -> LuaResult<()> {
        self.t.call_method("addGroup", (country, category, data))
    }

    pub fn add_static_object(&self, country: Country, data: mlua::Table<'lua>) -> LuaResult<()> {
        self.t.call_method("addStaticObject", (country, data))
    }

    pub fn get_groups(&self, side: Side) -> LuaResult<impl Iterator<Item = LuaResult<Group>>> {
        Ok(as_tbl("GroupIter", None, self.t.call_method("getGroups", side)?)?.sequence_values())
    }

    pub fn get_static_objects(
        &self,
        side: Side,
    ) -> LuaResult<impl Iterator<Item = LuaResult<mlua::Table>>> {
        Ok(as_tbl(
            "StaticIter",
            None,
            self.t.call_method("getStaticObjects", side)?,
        )?
        .sequence_values())
    }

    pub fn get_airbases(&self, side: Side) -> LuaResult<impl Iterator<Item = LuaResult<Airbase>>> {
        Ok(as_tbl(
            "AirbaseIter",
            None,
            self.t.call_method("getAirbases", side)?,
        )?
        .sequence_values())
    }

    pub fn get_players(&self, side: Side) -> LuaResult<impl Iterator<Item = LuaResult<Unit>>> {
        Ok(as_tbl(
            "PlayerUnitsIter",
            None,
            self.t.call_method("getPlayers", side)?,
        )?
        .sequence_values())
    }

    pub fn get_service_providers(
        &self,
        side: Side,
        service: Service,
    ) -> LuaResult<impl Iterator<Item = LuaResult<Unit>>> {
        Ok(as_tbl(
            "ServiceProviderIter",
            None,
            self.t.call_method("getServiceProviders", (side, service))?,
        )?
        .sequence_values())
    }
}
