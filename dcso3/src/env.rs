use crate::{simple_enum, wrapped_table, Vec2, Sequence, country::Country};
use super::{as_tbl, coalition::Side, controller::Controller, cvt_err, unit::Unit, String};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(MizWeather, None);

wrapped_table!(MizTriggerZone, None);

wrapped_table!(MizNavPoint, None);

wrapped_table!(MizStaticGroup, None);

wrapped_table!(MizGroundGroup, None);

wrapped_table!(MizSeaGroup, None);

wrapped_table!(MizAirTask, None);

wrapped_table!(MizAirPoint, None);

wrapped_table!(MizAirRoute, None);

impl<'lua> MizAirRoute<'lua> {
    pub fn points(&self) -> LuaResult<Sequence<MizAirPoint>> {
        self.raw_get("points")
    }
}

wrapped_table!(MizAirGroup, None);

impl<'lua> MizAirGroup<'lua> {
    pub fn frequency(&self) -> LuaResult<f64> {
        self.raw_get("frequency")
    }

    pub fn modulation(&self) -> LuaResult<i64> {
        self.raw_get("modulation")
    }

    pub fn late_activation(&self) -> LuaResult<bool> {
        self.raw_get("lateActivation")
    }

    pub fn group_id(&self) -> LuaResult<i64> {
        self.raw_get("groupId")
    }

    pub fn tasks(&self) -> LuaResult<Sequence<MizAirTask>> {
        self.raw_get("tasks")
    }

    pub fn route(&self) -> LuaResult<MizAirRoute> {
        self.raw_get("route")
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

    pub fn planes(&self) -> LuaResult<Sequence<MizAirGroup>> {
        let g: mlua::Table = self.raw_get("plane")?;
        g.raw_get("group")
    }

    pub fn helicopters(&self) -> LuaResult<Sequence<MizAirGroup>> {
        let g: mlua::Table = self.raw_get("helicopter")?;
        g.raw_get("group")
    }

    pub fn ships(&self) -> LuaResult<Sequence<MizSeaGroup>> {
        let g: mlua::Table = self.raw_get("ship")?;
        g.raw_get("group")
    }

    pub fn vehicles(&self) -> LuaResult<Sequence<MizGroundGroup>> {
        let g: mlua::Table = self.raw_get("vehicle")?;
        g.raw_get("group")
    }

    pub fn statics(&self) -> LuaResult<Sequence<MizStaticGroup>> {
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
