use super::{as_tbl, coalition::Side, object::Object, warehouse::Warehouse, String, LuaVec3};
use crate::{wrapped_table, Sequence, object::{DcsObject, DcsOid}, MizLua, LuaEnv};
use anyhow::{Result, bail};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::{ops::Deref, marker::PhantomData};

wrapped_table!(Runway, None);

wrapped_table!(Airbase, Some("Airbase"));

impl<'lua> Airbase<'lua> {
    pub fn get_by_name(&self, name: String) -> Result<Self> {
        let globals = self.lua.globals();
        let class = as_tbl("Airbase", Some("Airbase"), globals.raw_get("Airbase")?)?;
        Ok(class.call_method("getByName", name)?)
    }

    pub fn is_exist(&self) -> Result<bool> {
        Ok(self.t.call_method("isExist", ())?)
    }

    pub fn destroy(&self) -> Result<()> {
        Ok(self.t.call_method("destroy", ())?)
    }

    pub fn get_callsign(&self) -> Result<String> {
        Ok(self.t.call_method("getCallsign", ())?)
    }

    pub fn get_unit(&self, i: i64) -> Result<Object<'lua>> {
        Ok(self.t.call_method("getUnit", i)?)
    }

    pub fn get_id(&self) -> Result<i64> {
        Ok(self.t.call_method("getId", ())?)
    }

    pub fn get_parking(&self, available: bool) -> Result<mlua::Table<'lua>> {
        Ok(self.t.call_method("getParking", available)?)
    }

    pub fn get_runways(&self) -> Result<Sequence<Runway<'lua>>> {
        Ok(self.t.call_method("getRunways", ())?)
    }

    pub fn get_tech_object_pos(&self, obj: String) -> Result<LuaVec3> {
        Ok(self.t.call_method("getTechObjectPos", obj)?)
    }

    pub fn get_radio_silent_mode(&self) -> Result<bool> {
        Ok(self.t.call_method("getRadioSilentMode", ())?)
    }

    pub fn set_radio_silent_mode(&self, on: bool) -> Result<()> {
        Ok(self.t.call_method("setRadioSilentMode", on)?)
    }

    pub fn auto_capture(&self, on: bool) -> Result<()> {
        Ok(self.t.call_method("autoCapture", on)?)
    }

    pub fn auto_capture_is_on(&self) -> Result<bool> {
        Ok(self.t.call_method("autoCaptureIsOn", ())?)
    }

    pub fn set_coalition(&self, coa: Side) -> Result<()> {
        Ok(self.t.call_method("setCoalition", coa)?)
    }

    pub fn get_warehouse(&self) -> Result<Warehouse<'lua>> {
        Ok(self.t.call_method("getWarehouse", ())?)
    }

    pub fn as_object(&self) -> Result<Object<'lua>> {
        Ok(Object::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }
}

#[derive(Debug, Clone)]
pub struct ClassAirbase;

impl<'lua> DcsObject<'lua> for Airbase<'lua> {
    type Class = ClassAirbase;

    fn get_instance(lua: MizLua<'lua>, id: &DcsOid<Self::Class>) -> Result<Self> {
        let t = lua.inner().create_table()?;
        t.set_metatable(Some(lua.inner().globals().raw_get(&**id.class)?));
        t.raw_set("id_", id.id)?;
        let t = Airbase {
            t,
            lua: lua.inner(),
        };
        if !t.is_exist()? {
            bail!("{} is an invalid airbase", id.id)
        }
        Ok(t)
    }

    fn get_instance_dyn<T>(lua: MizLua<'lua>, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(lua, "Airbase")?;
        let id = DcsOid {
            id: id.id,
            class: id.class.clone(),
            t: PhantomData,
        };
        Self::get_instance(lua, &id)
    }

    fn change_instance(self, id: &DcsOid<Self::Class>) -> Result<Self> {
        self.raw_set("id_", id.id)?;
        if !self.is_exist()? {
            bail!("{} is an invalid airbase", id.id)
        }
        Ok(self)
    }

    fn change_instance_dyn<T>(self, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(MizLua(self.lua), "Airbase")?;
        self.t.raw_set("id_", id.id)?;
        if !self.is_exist()? {
            bail!("{} is an invalid airbase", id.id)
        }
        Ok(self)
    }
}
