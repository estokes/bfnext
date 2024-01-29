/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::{as_tbl, coalition::Side, String};
use crate::{wrapped_table, HooksLua, LuaEnv, Time};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(Dcs, None);

impl<'lua> Dcs<'lua> {
    pub fn singleton(lua: HooksLua<'lua>) -> Result<Self> {
        let globals = lua.inner().globals();
        Ok(globals.raw_get("DCS")?)
    }

    pub fn get_mission_name(&self) -> Result<String> {
        Ok(self.t.call_function("getMissionName", ())?)
    }

    pub fn get_mission_filename(&self) -> Result<String> {
        Ok(self.t.call_function("getMissionFilename", ())?)
    }

    pub fn get_mission_result(&self, side: Side) -> Result<i64> {
        Ok(self.t.call_function("getMissionResult", side)?)
    }

    pub fn get_unit_property(&self, name: String) -> Result<Value<'lua>> {
        Ok(self.t.call_function("getUnitProperty", name)?)
    }

    pub fn set_pause(&self, pause: bool) -> Result<()> {
        Ok(self.t.call_function("setPause", pause)?)
    }

    pub fn get_pause(&self) -> Result<bool> {
        Ok(self.t.call_function("getPause", ())?)
    }

    pub fn stop_mission(&self) -> Result<()> {
        Ok(self.t.call_function("stopMission", ())?)
    }

    pub fn exit_process(&self) -> Result<()> {
        Ok(self.t.call_function("exitProcess", ())?)
    }

    pub fn is_multiplayer(&self) -> Result<bool> {
        Ok(self.t.call_function("isMultiplayer", ())?)
    }

    pub fn is_server(&self) -> Result<bool> {
        Ok(self.t.call_function("isServer", ())?)
    }

    pub fn get_model_time(&self) -> Result<Time> {
        Ok(self.t.call_function("getModelTime", ())?)
    }

    pub fn get_real_time(&self) -> Result<Time> {
        Ok(self.t.call_function("getRealTime", ())?)
    }

    pub fn get_mission_options(&self) -> Result<LuaTable<'lua>> {
        Ok(self.t.call_function("getMissionOptions", ())?)
    }

    pub fn get_available_coalitions(&self) -> Result<LuaTable<'lua>> {
        Ok(self.t.call_function("getAvailableCoalitions", ())?)
    }

    pub fn get_available_slots(&self) -> Result<LuaTable<'lua>> {
        Ok(self.t.call_function("getAvailableSlots", ())?)
    }

    pub fn get_current_mission(&self) -> Result<LuaTable<'lua>> {
        Ok(self.t.call_function("getCurrentMission", ())?)
    }
}
