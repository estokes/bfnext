/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::as_tbl;
use crate::{
    airbase::Airbase, cvt_err, lua_err, simple_enum, wrapped_table, LuaEnv, MizLua, String,
};
use anyhow::{bail, Result};
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;

simple_enum!(LiquidType, u8, [
    JetFuel => 0,
    Avgas => 1,
    MW50 => 2,
    Diesel => 3
]);

impl LiquidType {
    pub const ALL: [LiquidType; 4] = [Self::Avgas, Self::Diesel, Self::JetFuel, Self::MW50];
}

wrapped_table!(ItemInventory, None);

impl<'lua> ItemInventory<'lua> {
    pub fn item(&self, name: &str) -> Result<u32> {
        Ok(self.t.raw_get(name)?)
    }

    pub fn for_each<F: FnMut(String, u32) -> Result<()>>(&self, mut f: F) -> Result<()> {
        Ok(self.t.for_each(|k, v| f(k, v).map_err(lua_err))?)
    }
}

wrapped_table!(LiquidInventory, None);

impl<'lua> LiquidInventory<'lua> {
    pub fn item(&self, name: LiquidType) -> Result<u32> {
        Ok(self.t.raw_get(name)?)
    }

    pub fn for_each<F: FnMut(LiquidType, u32) -> Result<()>>(&self, mut f: F) -> Result<()> {
        Ok(self.t.for_each(|k, v| f(k, v).map_err(lua_err))?)
    }
}

wrapped_table!(Inventory, None);

impl<'lua> Inventory<'lua> {
    pub fn weapons(&self) -> Result<ItemInventory<'lua>> {
        Ok(self.t.raw_get("weapon")?)
    }

    pub fn aircraft(&self) -> Result<ItemInventory<'lua>> {
        Ok(self.t.raw_get("aircraft")?)
    }

    pub fn liquids(&self) -> Result<LiquidInventory<'lua>> {
        Ok(self.t.raw_get("liquids")?)
    }

    pub fn is_unlimited(&self) -> Result<bool> {
        Ok(self.weapons()?.is_empty() && self.aircraft()?.is_empty() && self.liquids()?.is_empty())
    }
}

#[derive(Debug, Clone, Copy)]
pub enum WSFixedWingCategory {
    Fighters,
    FastBombers,
    Interceptors,
    Bombers,
    MiscSupport,
    Attack,
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum WSAircraftCategory {
    FixedWing(WSFixedWingCategory),
    Helicopters,
    None,
}

#[derive(Debug, Clone, Copy)]
pub enum WSCategory {
    Aircraft(WSAircraftCategory),
    Vehicles,
    Ships,
    Weapons,
    None,
}

impl WSCategory {
    pub fn is_aircraft(&self) -> bool {
        match self {
            Self::Aircraft(_) => true,
            Self::None | Self::Ships | Self::Vehicles | Self::Weapons => false,
        }
    }

    pub fn is_fixedwing(&self) -> bool {
        match self {
            Self::Aircraft(WSAircraftCategory::FixedWing(_)) => true,
            Self::Aircraft(WSAircraftCategory::Helicopters)
            | Self::Aircraft(WSAircraftCategory::None)
            | Self::None
            | Self::Ships
            | Self::Vehicles
            | Self::Weapons => false,
        }
    }

    pub fn is_helicopter(&self) -> bool {
        match self {
            Self::Aircraft(WSAircraftCategory::Helicopters) => true,
            Self::Aircraft(WSAircraftCategory::FixedWing(_))
            | Self::Aircraft(WSAircraftCategory::None)
            | Self::None
            | Self::Ships
            | Self::Vehicles
            | Self::Weapons => false,
        }
    }
 
    pub fn is_weapon(&self) -> bool {
        match self {
            Self::Weapons => true,
            Self::Aircraft(_) | Self::None | Self::Ships | Self::Vehicles => false,
        }
    }
    
    pub fn is_vehicle(&self) -> bool {
        match self {
            Self::Vehicles => true,
            Self::Aircraft(_) | Self::None | Self::Ships | Self::Weapons => false,
        }
    }
    
    pub fn is_ship(&self) -> bool {
        match self {
            Self::Ships => true,
            Self::Aircraft(_) | Self::None | Self::Weapons | Self::Vehicles => false,
        }
    }
}

wrapped_table!(WSType, None);

impl<'lua> WSType<'lua> {
    pub fn category(&self) -> Result<WSCategory> {
        match self.t.raw_get(1)? {
            0 => Ok(WSCategory::None),
            1 => match self.t.raw_get(2)? {
                0 => Ok(WSCategory::Aircraft(WSAircraftCategory::None)),
                1 => Ok(WSCategory::Aircraft(WSAircraftCategory::FixedWing(
                    match self.t.raw_get(3)? {
                        0 => WSFixedWingCategory::None,
                        1 => WSFixedWingCategory::Fighters,
                        2 => WSFixedWingCategory::FastBombers,
                        3 => WSFixedWingCategory::Interceptors,
                        4 => WSFixedWingCategory::Bombers,
                        5 => WSFixedWingCategory::MiscSupport,
                        6 => WSFixedWingCategory::Attack,
                        n => bail!("unknown airplane category {n}"),
                    },
                ))),
                2 => Ok(WSCategory::Aircraft(WSAircraftCategory::Helicopters)),
                n => bail!("unknown aircraft category {n}"),
            },
            2 => Ok(WSCategory::Vehicles),
            3 => Ok(WSCategory::Ships),
            4 => Ok(WSCategory::Weapons),
            n => bail!("unknown major category {n}"),
        }
    }
}

wrapped_table!(ResourceMap, None);

impl<'lua> ResourceMap<'lua> {
    pub fn for_each<F: FnMut(String, WSType) -> Result<()>>(&self, mut f: F) -> Result<()> {
        Ok(self.t.for_each(|k, v| f(k, v).map_err(lua_err))?)
    }
}

#[derive(Debug, Clone)]
pub enum WarehouseItem<'lua> {
    Name(String),
    Typ(WSType<'lua>),
}

impl<'lua> IntoLua<'lua> for WarehouseItem<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        match self {
            Self::Name(s) => s.into_lua(lua),
            Self::Typ(t) => Ok(Value::Table(t.t)),
        }
    }
}

impl<'lua> From<String> for WarehouseItem<'lua> {
    fn from(value: String) -> Self {
        Self::Name(value)
    }
}

impl<'lua> From<WSType<'lua>> for WarehouseItem<'lua> {
    fn from(value: WSType<'lua>) -> Self {
        Self::Typ(value)
    }
}

wrapped_table!(Warehouse, Some("Warehouse"));

impl<'lua> Warehouse<'lua> {
    pub fn get_by_name(lua: MizLua<'lua>, name: String) -> Result<Self> {
        let wh: LuaTable = lua.inner().globals().raw_get("Warehouse")?;
        Ok(wh.call_function("getByName", name)?)
    }

    pub fn get_resource_map(lua: MizLua<'lua>) -> Result<ResourceMap<'lua>> {
        let wh: LuaTable = lua.inner().globals().raw_get("Warehouse")?;
        Ok(wh.call_function("getResourceMap", ())?)
    }

    pub fn add_item<T: Into<WarehouseItem<'lua>>>(&self, item: T, count: u32) -> Result<()> {
        Ok(self
            .t
            .call_method("addItem", (Into::<WarehouseItem>::into(item), count))?)
    }

    pub fn remove_item<T: Into<WarehouseItem<'lua>>>(&self, item: T, count: u32) -> Result<()> {
        Ok(self
            .t
            .call_method("removeItem", (Into::<WarehouseItem>::into(item), count))?)
    }

    pub fn set_item<T: Into<WarehouseItem<'lua>>>(&self, item: T, count: u32) -> Result<()> {
        Ok(self
            .t
            .call_method("setItem", (Into::<WarehouseItem>::into(item), count))?)
    }

    pub fn get_item_count<T: Into<WarehouseItem<'lua>>>(&self, item: T) -> Result<u32> {
        Ok(self
            .t
            .call_method("getItemCount", Into::<WarehouseItem>::into(item))?)
    }

    pub fn add_liquid(&self, typ: LiquidType, count: u32) -> Result<()> {
        Ok(self.t.call_method("addLiquid", (typ, count))?)
    }

    pub fn remove_liquid(&self, typ: LiquidType, count: u32) -> Result<()> {
        Ok(self.t.call_method("removeLiquid", (typ, count))?)
    }

    pub fn get_liquid_amount(&self, typ: LiquidType) -> Result<u32> {
        Ok(self.t.call_method("getLiquidAmount", typ)?)
    }

    pub fn set_liquid_amount(&self, typ: LiquidType, count: u32) -> Result<()> {
        Ok(self.t.call_method("setLiquidAmount", (typ, count))?)
    }

    pub fn get_inventory(&self, filter: Option<String>) -> Result<Inventory<'lua>> {
        Ok(self.t.call_method("getInventory", filter)?)
    }

    pub fn get_owner(&self) -> Result<Airbase> {
        Ok(self.t.call_method("getOwner", ())?)
    }

    pub fn whid(&self) -> Result<String> {
        Ok(self.t.raw_get("whid_")?)
    }
}
