use crate::{wrapped_table, Vec2, Sequence, country::Country};
use super::{as_tbl, coalition::Side, String};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(MizWeather, None);

wrapped_table!(MizTriggerZone, None);

wrapped_table!(MizNavPoint, None);

wrapped_table!(MizTask, None);

wrapped_table!(MizPoint, None);

wrapped_table!(MizRoute, None);

impl<'lua> MizRoute<'lua> {
    pub fn points(&self) -> LuaResult<Sequence<MizPoint>> {
        self.raw_get("points")
    }
}

wrapped_table!(MizUnit, None);

wrapped_table!(MizGroup, None);

impl<'lua> MizGroup<'lua> {
    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn frequency(&self) -> LuaResult<f64> {
        self.raw_get("frequency")
    }

    pub fn modulation(&self) -> LuaResult<i64> {
        self.raw_get("modulation")
    }

    pub fn late_activation(&self) -> bool {
        self.raw_get("lateActivation").unwrap_or(false)
    }

    pub fn group_id(&self) -> LuaResult<i64> {
        self.raw_get("groupId")
    }

    pub fn tasks(&self) -> LuaResult<Sequence<MizTask>> {
        self.raw_get("tasks")
    }

    pub fn route(&self) -> LuaResult<MizRoute> {
        self.raw_get("route")
    }

    pub fn hidden(&self) -> bool {
        self.raw_get("hidden").unwrap_or(false)
    }

    pub fn units(&self) -> LuaResult<Sequence<MizUnit>> {
        self.raw_get("units")
    }

    pub fn uncontrollable(&self) -> bool {
        self.raw_get("uncontrollable").unwrap_or(true)
    }
}

wrapped_table!(MizCountry, None);

impl<'lua> MizCountry<'lua> {
    pub fn id(&self) -> LuaResult<Country> {
        self.raw_get("id")
    }

    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn planes(&self) -> LuaResult<Sequence<MizGroup>> {
        let g: mlua::Table = self.raw_get("plane")?;
        g.raw_get("group")
    }

    pub fn helicopters(&self) -> LuaResult<Sequence<MizGroup>> {
        let g: mlua::Table = self.raw_get("helicopter")?;
        g.raw_get("group")
    }

    pub fn ships(&self) -> LuaResult<Sequence<MizGroup>> {
        let g: mlua::Table = self.raw_get("ship")?;
        g.raw_get("group")
    }

    pub fn vehicles(&self) -> LuaResult<Sequence<MizGroup>> {
        let g: mlua::Table = self.raw_get("vehicle")?;
        g.raw_get("group")
    }

    pub fn statics(&self) -> LuaResult<Sequence<MizGroup>> {
        let g: mlua::Table = self.raw_get("static")?;
        g.raw_get("group")
    }
}

wrapped_table!(MizCoalition, None);

impl<'lua> MizCoalition<'lua> {
    pub fn bullseye(&self) -> LuaResult<Vec2> {
        self.t.raw_get("bullseye")
    }

    pub fn nav_points(&self) -> LuaResult<Sequence<MizNavPoint>> {
        self.t.raw_get("nav_points")
    }

    pub fn name(&self) -> LuaResult<String> {
        self.t.raw_get("name")
    }

    pub fn countries(&self) -> LuaResult<Sequence<MizCountry>> {
        self.t.raw_get("country")
    }
}

wrapped_table!(Mission, None);

impl<'lua> Mission<'lua> {
    pub fn coalition(&self, side: Side) -> LuaResult<MizCoalition<'lua>> {
        match side {
            Side::Blue => self.t.raw_get("blue"),
            Side::Red => self.t.raw_get("red"),
            Side::Neutral => self.t.raw_get("neutral")
        }
    }

    pub fn triggers(&self) -> LuaResult<Sequence<MizTriggerZone>> {
        let triggers: mlua::Table = self.t.raw_get("triggers")?;
        triggers.raw_get("zones")
    }

    pub fn weather(&self) -> LuaResult<MizWeather<'lua>> {
        self.t.raw_get("weather")
    }
}
