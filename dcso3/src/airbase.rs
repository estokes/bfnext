use super::{as_tbl, coalition::Side, object::Object, warehouse::Warehouse, String, LuaVec3};
use crate::{wrapped_table, Sequence};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(Runway, None);

wrapped_table!(Airbase, Some("Airbase"));

impl<'lua> Airbase<'lua> {
    pub fn get_by_name(&self, name: String) -> Result<Self> {
        let globals = self.lua.globals();
        let class = as_tbl("Airbase", Some("Airbase"), globals.raw_get("Airbase")?)?;
        Ok(class.call_method("getByName", name)?)
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
}
