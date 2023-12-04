use crate::{
    as_tbl,
    coalition::Side,
    cvt_err,
    env::miz::{Country, Miz},
    net::SlotId,
    simple_enum, wrapped_table, LuaEnv, LuaVec3, MizLua, Time,
};
use log::error;
use mlua::{prelude::*, serde::de, Value};
use nalgebra::Storage;
use serde_derive::{Deserialize, Serialize};
use std::{ops::Deref, ptr::metadata};

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Zone {
    pub point: LuaVec3,
    pub radius: f64,
}

impl<'lua> FromLua<'lua> for Zone {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::Table(tbl) => Ok(Self {
                point: tbl.raw_get("point")?,
                radius: tbl.raw_get("radius")?,
            }),
            _ => Err(cvt_err("trigger::Zone")),
        }
    }
}

impl<'lua> IntoLua<'lua> for Zone {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let table = lua.create_table()?;
        table.raw_set("point", self.point)?;
        table.raw_set("radius", self.radius)?;
        Ok(Value::Table(table))
    }
}

simple_enum!(SmokeColor, u8, [
    Green => 0,
    Red => 1,
    White => 2,
    Orange => 3,
    Blue => 4
]);

simple_enum!(SmokePreset, u8, [
    SmallSmokeAndFire => 1,
    MediumSmokeAndFire => 2,
    LargeSmokeAndFire => 3,
    HugeSmokeAndFire => 4,
    SmallSmoke => 5,
    MediumSmoke => 6,
    LargeSmoke => 7,
    HugeSmoke => 8
]);

simple_enum!(FlareColor, u8, [
    Green => 0,
    Red => 1,
    White => 2,
    Yellow => 3
]);

simple_enum!(Modulation, u8, [
    AM => 0,
    FM => 1
]);

wrapped_table!(Action, None);

impl<'lua> Action<'lua> {
    pub fn explosion(&self, position: LuaVec3, power: f32) -> LuaResult<()> {
        self.call_function("explosion", (position, power))
    }

    pub fn smoke(&self, position: LuaVec3, color: SmokeColor) -> LuaResult<()> {
        self.call_function("smoke", (position, color))
    }

    pub fn effect_smoke_big(
        &self,
        position: LuaVec3,
        preset: SmokePreset,
        density: f32,
        name: String,
    ) -> LuaResult<()> {
        self.call_function("effectSmokeBig", (position, preset, density, name))
    }

    pub fn effect_smoke_stop(&self, name: String) -> LuaResult<()> {
        self.call_function("effectSmokeStop", name)
    }

    pub fn illumination_bomb(&self, position: LuaVec3, power: f32) -> LuaResult<()> {
        self.call_function("illuminationBomb", (position, power))
    }

    pub fn signal_flare(
        &self,
        position: LuaVec3,
        color: FlareColor,
        azimuth: u16,
    ) -> LuaResult<()> {
        self.call_function("signalFlare", (position, color, azimuth))
    }

    pub fn radio_transmission(
        &self,
        file: String,
        origin: LuaVec3,
        modulation: Modulation,
        repeat: bool,
        frequency: u64,
        power: u64,
        name: String,
    ) -> LuaResult<()> {
        self.call_function(
            "radioTransmission",
            (file, origin, modulation, repeat, frequency, power, name),
        )
    }

    pub fn stop_transmission(&self, name: String) -> LuaResult<()> {
        self.call_function("stopRadioTransmission", name)
    }

    pub fn set_unit_internal_cargo(&self, unit_name: String, mass: i64) -> LuaResult<()> {
        self.call_function("setUnitInternalCargo", (unit_name, mass))
    }

    pub fn out_sound(&self, file: String) -> LuaResult<()> {
        self.call_function("outSound", file)
    }

    pub fn out_sound_for_coalition(&self, side: Side, file: String) -> LuaResult<()> {
        self.call_function("outSoundForCoalition", (side, file))
    }

    pub fn out_sound_for_country(&self, country: Country, file: String) -> LuaResult<()> {
        self.call_function("outSoundForCountry", (country, file))
    }

    pub fn out_sound_for_group(&self, group: i64, file: String) -> LuaResult<()> {
        self.call_function("outSoundForGroup", (group, file))
    }

    pub fn out_sound_for_unit(&self, unit: i64, file: String) -> LuaResult<()> {
        self.call_function("outSoundForUnit", (unit, file))
    }

    pub fn out_text(&self, text: String, display_time: i64, clear_view: bool) -> LuaResult<()> {
        self.call_function("outText", (text, display_time, clear_view))
    }

    pub fn out_text_for_coalition(
        &self,
        side: Side,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> LuaResult<()> {
        self.call_function(
            "outTextForCoalition",
            (side, text, display_time, clear_view),
        )
    }

    pub fn out_text_for_country(
        &self,
        country: Country,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> LuaResult<()> {
        self.call_function(
            "outTextForCountry",
            (country, text, display_time, clear_view),
        )
    }

    pub fn out_text_for_group(
        &self,
        group: i64,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> LuaResult<()> {
        self.call_function("outTextForGroup", (group, text, display_time, clear_view))
    }

    pub fn out_text_for_unit(
        &self,
        unit: i64,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> LuaResult<()> {
        self.call_function("outTextForUnit", (unit, text, display_time, clear_view))
    }

    pub fn mark_to_all(
        &self,
        number: i64,
        text: String,
        position: LuaVec3,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function("markToAll", (number, text, position, read_only, message))
    }

    pub fn mark_to_coalition(
        &self,
        number: i64,
        text: String,
        position: LuaVec3,
        side: Side,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function(
            "markToCoalition",
            (number, text, position, side, read_only, message),
        )
    }

    pub fn mark_to_group(
        &self,
        number: i64,
        text: String,
        position: LuaVec3,
        group: i64,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function(
            "markToGroup",
            (number, text, position, group, read_only, message),
        )
    }

    pub fn remove_mark(&self, number: i64) -> LuaResult<()> {
        self.call_function("removeMark", number)
    }
}

wrapped_table!(Trigger, None);

impl<'lua> Trigger<'lua> {
    pub fn singleton(lua: MizLua<'lua>) -> LuaResult<Self> {
        lua.inner().globals().raw_get("trigger")
    }

    pub fn action(&self) -> LuaResult<Action<'lua>> {
        self.raw_get("action")
    }

    pub fn get_zone(&self, name: String) -> LuaResult<Zone> {
        let misc: LuaTable = self.t.raw_get("misc")?;
        misc.call_function("getZone", name)
    }
}
