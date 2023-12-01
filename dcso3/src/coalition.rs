use super::{
    airbase::Airbase,
    as_tbl,
    country::Country,
    cvt_err, env,
    group::{Group, GroupCategory},
    static_object::StaticObject,
    unit::Unit,
};
use crate::{simple_enum, wrapped_table, Sequence, MizLua, LuaEnv};
use mlua::{prelude::*, Value};
use serde_derive::{Serialize, Deserialize};
use std::{ops::Deref, str::FromStr};

simple_enum!(Side, u8, [Neutral => 0, Red => 1, Blue => 2]);

impl FromStr for Side {
    type Err = LuaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "blue" => Side::Blue,
            "red" => Side::Red,
            "neutrals" => Side::Neutral,
            _ => return Err(cvt_err("side_str")),
        })
    }
}

impl Side {
    pub fn to_str(&self) -> &'static str {
        match self {
            Side::Blue => "blue",
            Side::Red => "red",
            Side::Neutral => "neutrals",
        }
    }
}

simple_enum!(Service, u8, [Atc => 0, Awacs => 1, Fac => 3, Tanker => 2]);
wrapped_table!(Coalition, None);

impl<'lua> Coalition<'lua> {
    pub fn singleton(lua: MizLua<'lua>) -> LuaResult<Self> {
        Ok(Self {
            t: lua.inner().globals().raw_get("coalition")?,
            lua: lua.inner(),
        })
    }

    pub fn add_group(
        &self,
        country: Country,
        category: GroupCategory,
        data: env::miz::Group,
    ) -> LuaResult<()> {
        self.t.call_function("addGroup", (country, category, data))
    }

    pub fn add_static_object(&self, country: Country, data: env::miz::Group) -> LuaResult<()> {
        self.t.call_function("addStaticObject", (country, data))
    }

    pub fn get_groups(&self, side: Side) -> LuaResult<Sequence<Group>> {
        self.t.call_function("getGroups", side)
    }

    pub fn get_static_objects(&self, side: Side) -> LuaResult<Sequence<StaticObject>> {
        self.t.call_function("getStaticObjects", side)
    }

    pub fn get_airbases(&self, side: Side) -> LuaResult<Sequence<Airbase>> {
        self.t.call_function("getAirbases", side)
    }

    pub fn get_players(&self, side: Side) -> LuaResult<Sequence<Unit>> {
        self.t.call_function("getPlayers", side)
    }

    pub fn get_service_providers(&self, side: Side, service: Service) -> LuaResult<Sequence<Unit>> {
        self.t.call_function("getServiceProviders", (side, service))
    }

    pub fn get_country_coalition(&self, country: Country) -> LuaResult<Side> {
        self.t.call_function("getCountrySide", country)
    }
}
