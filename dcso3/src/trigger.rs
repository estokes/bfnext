use crate::{
    as_tbl, coalition::Side, cvt_err, env::miz::Country, simple_enum, wrapped_table, Color, LuaEnv,
    LuaVec3, MizLua,
};
use mlua::{prelude::*, Value};
use serde::{de::Visitor, Deserializer};
use serde_derive::{Deserialize, Serialize};
use std::{
    fmt,
    ops::Deref,
    sync::atomic::{AtomicU64, Ordering},
};

static MARK: AtomicU64 = AtomicU64::new(0);

struct U64Visitor;

impl<'de> Visitor<'de> for U64Visitor {
    type Value = u64;

    fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        write!(formatter, "a u64")
    }

    fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        Ok(v)
    }
}

fn de_mark_id<'de, D>(de: D) -> Result<u64, D::Error>
where
    D: Deserializer<'de>,
{
    let n = de.deserialize_u64(U64Visitor)?;
    MarkId::set_max(n);
    Ok(n)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
pub struct MarkId(#[serde(deserialize_with = "de_mark_id")] u64);

impl From<u64> for MarkId {
    fn from(value: u64) -> Self {
        MarkId::set_max(value);
        Self(value)
    }
}

impl MarkId {
    pub fn new() -> MarkId {
        Self(MARK.fetch_add(1, Ordering::Relaxed))
    }

    fn set_max(n: u64) {
        let _: Result<_, _> = MARK.fetch_update(Ordering::Relaxed, Ordering::Relaxed, |cur| {
            if n >= cur {
                Some(n + 1)
            } else {
                None
            }
        });
    }
}

impl<'lua> FromLua<'lua> for MarkId {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let i = u64::from_lua(value, lua)?;
        Ok(MarkId(i))
    }
}

impl<'lua> IntoLua<'lua> for MarkId {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        self.0.into_lua(lua)
    }
}

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

simple_enum!(SideFilter, i8, [
    All => -1,
    Neutral => 0,
    Red => 1,
    Blue => 2
]);

simple_enum!(LineType, u8, [
    NoLine => 0,
    Solid => 1,
    Dashed => 2,
    Dotted => 3,
    DotDash => 4,
    LongDash => 5,
    TwoDash => 6
]);

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LineSpec {
    pub start: LuaVec3,
    pub end: LuaVec3,
    pub color: Color,
    pub line_type: LineType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CircleSpec {
    pub center: LuaVec3,
    pub radius: f64,
    pub color: Color,
    pub fill_color: Color,
    line_type: LineType,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RectSpec {
    start: LuaVec3,
    end: LuaVec3,
    color: Color,
    fill_color: Color,
    line_type: LineType,
}

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
        id: MarkId,
        text: String,
        position: LuaVec3,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function("markToAll", (id, text, position, read_only, message))
    }

    pub fn mark_to_coalition(
        &self,
        id: MarkId,
        text: String,
        position: LuaVec3,
        side: Side,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function(
            "markToCoalition",
            (id, text, position, side, read_only, message),
        )
    }

    pub fn mark_to_group(
        &self,
        id: MarkId,
        text: String,
        position: LuaVec3,
        group: i64,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function(
            "markToGroup",
            (id, text, position, group, read_only, message),
        )
    }

    pub fn remove_mark(&self, id: MarkId) -> LuaResult<()> {
        self.call_function("removeMark", id)
    }

    pub fn line(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: LineSpec,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function(
            "lineToAll",
            (
                side,
                id,
                spec.start,
                spec.end,
                spec.color,
                spec.line_type,
                read_only,
                message,
            ),
        )
    }

    pub fn circle(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: CircleSpec,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function(
            "circleToAll",
            (
                side,
                id,
                spec.center,
                spec.radius,
                spec.color,
                spec.fill_color,
                spec.line_type,
                read_only,
                message,
            ),
        )
    }

    pub fn rect(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: RectSpec,
        read_only: bool,
        message: Option<String>,
    ) -> LuaResult<()> {
        self.call_function(
            "rectToAll",
            (
                side,
                id,
                spec.start,
                spec.end,
                spec.color,
                spec.fill_color,
                spec.line_type,
                read_only,
                message,
            ),
        )
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
