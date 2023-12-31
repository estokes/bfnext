use super::as_tbl;
use crate::{
    airbase::Airbase,
    cvt_err, lua_err,
    object::{DcsObject, DcsOid},
    simple_enum, wrapped_table, LuaEnv, MizLua, String,
};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::{marker::PhantomData, ops::Deref};

simple_enum!(LiquidType, u8, [
    JetFuel => 0,
    Avgas => 1,
    MW50 => 2,
    Diesel => 3
]);

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
}

wrapped_table!(Warehouse, Some("Warehouse"));

impl<'lua> Warehouse<'lua> {
    pub fn get_by_name(lua: MizLua<'lua>, name: String) -> Result<Self> {
        let wh: LuaTable = lua.inner().globals().raw_get("Warehouse")?;
        Ok(wh.call_function("getByName", name)?)
    }

    pub fn add_item(&self, name: String, count: u32) -> Result<()> {
        Ok(self.t.call_method("addItem", (name, count))?)
    }

    pub fn remove_item(&self, name: String, count: u32) -> Result<()> {
        Ok(self.t.call_method("removeItem", (name, count))?)
    }

    pub fn set_item(&self, name: String, count: u32) -> Result<()> {
        Ok(self.t.call_method("setItem", (name, count))?)
    }

    pub fn get_item_count(&self, name: String) -> Result<u32> {
        Ok(self.t.call_method("getItemCount", name)?)
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
}

#[derive(Debug, Clone)]
pub struct ClassWarehouse;

impl<'lua> DcsObject<'lua> for Warehouse<'lua> {
    type Class = ClassWarehouse;

    fn get_instance(lua: MizLua<'lua>, id: &DcsOid<Self::Class>) -> Result<Self> {
        let t = lua.inner().create_table()?;
        t.set_metatable(Some(lua.inner().globals().raw_get(&**id.class)?));
        t.raw_set("id_", id.id)?;
        let t = Warehouse {
            t,
            lua: lua.inner(),
        };
        Ok(t)
    }

    fn get_instance_dyn<T>(lua: MizLua<'lua>, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(lua, "Warehouse")?;
        let id = DcsOid {
            id: id.id,
            class: id.class.clone(),
            t: PhantomData,
        };
        Self::get_instance(lua, &id)
    }

    fn change_instance(self, id: &DcsOid<Self::Class>) -> Result<Self> {
        self.raw_set("id_", id.id)?;
        Ok(self)
    }

    fn change_instance_dyn<T>(self, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(MizLua(self.lua), "Warehouse")?;
        self.t.raw_set("id_", id.id)?;
        Ok(self)
    }
}
