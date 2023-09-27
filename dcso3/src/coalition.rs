use super::{
    airbase::Airbase,
    as_tbl,
    country::Country,
    cvt_err,
    group::{Group, GroupCategory},
    static_object::StaticObject,
    unit::Unit,
};
use crate::{wrapped_table, simple_enum};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

simple_enum!(Side, u8, [Neutral => 0, Red => 1, Blue => 2]);
simple_enum!(Service, u8, [Atc => 0, Awacs => 1, Fac => 3, Tanker => 2]);
wrapped_table!(Coalition, None);

impl<'lua> Coalition<'lua> {
    pub fn singleton(lua: &'lua Lua) -> LuaResult<Self> {
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

    pub fn add_static_object(&self, country: Country, data: StaticObject) -> LuaResult<()> {
        self.t.call_method("addStaticObject", (country, data))
    }

    pub fn get_groups(&self, side: Side) -> LuaResult<impl Iterator<Item = LuaResult<Group>>> {
        Ok(as_tbl("GroupIter", None, self.t.call_method("getGroups", side)?)?.sequence_values())
    }

    pub fn get_static_objects(
        &self,
        side: Side,
    ) -> LuaResult<impl Iterator<Item = LuaResult<StaticObject>>> {
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

    pub fn get_country_coalition(&self, country: Country) -> LuaResult<Side> {
        self.t.call_method("getCountrySide", country)
    }
}
