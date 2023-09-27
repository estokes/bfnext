use super::{as_tbl, coalition::Side, object::Object, warehouse::Warehouse, String, Vec3};
use crate::wrapped_table;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(Runway, None);

wrapped_table!(Airbase, Some("Airbase"));

impl<'lua> Airbase<'lua> {
    pub fn get_by_name(&self, name: String) -> LuaResult<Self> {
        let globals = self.lua.globals();
        let class = as_tbl("Airbase", Some("Airbase"), globals.raw_get("Airbase")?)?;
        class.call_method("getByName", name)
    }

    pub fn get_callsign(&self) -> LuaResult<String> {
        self.t.call_method("getCallsign", ())
    }

    pub fn get_unit(&self, i: u32) -> LuaResult<Object> {
        self.t.call_method("getUnit", i)
    }

    pub fn get_id(&self) -> LuaResult<u32> {
        self.t.call_method("getId", ())
    }

    pub fn get_parking(&self, available: bool) -> LuaResult<mlua::Table<'lua>> {
        self.t.call_method("getParking", available)
    }

    pub fn get_runways(&self) -> LuaResult<impl Iterator<Item = LuaResult<Runway<'lua>>>> {
        Ok(as_tbl("Runways", None, self.t.call_method("getRunways", ())?)?.sequence_values())
    }

    pub fn get_tech_object_pos(&self, obj: String) -> LuaResult<Vec3> {
        self.t.call_method("getTechObjectPos", obj)
    }

    pub fn get_radio_silent_mode(&self) -> LuaResult<bool> {
        self.t.call_method("getRadioSilentMode", ())
    }

    pub fn set_radio_silent_mode(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("setRadioSilentMode", on)
    }

    pub fn auto_capture(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("autoCapture", on)
    }

    pub fn auto_capture_is_on(&self) -> LuaResult<bool> {
        self.t.call_method("autoCaptureIsOn", ())
    }

    pub fn set_coalition(&self, coa: Side) -> LuaResult<()> {
        self.t.call_method("setCoalition", coa)
    }

    pub fn get_warehouse(&self) -> LuaResult<Warehouse> {
        Warehouse::from_lua(self.t.call_method("getWarehouse", ())?, self.lua)
    }
}
