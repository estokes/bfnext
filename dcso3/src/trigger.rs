use crate::{
    as_tbl, atomic_id,
    coalition::Side,
    cvt_err,
    env::miz::{Country, GroupId, UnitId},
    simple_enum, wrapped_table, Color, LuaEnv, LuaVec3, MizLua, String
};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;

atomic_id!(MarkId);

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
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct CircleSpec {
    pub center: LuaVec3,
    pub radius: f64,
    pub color: Color,
    pub fill_color: Color,
    pub line_type: LineType,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RectSpec {
    pub start: LuaVec3,
    pub end: LuaVec3,
    pub color: Color,
    pub fill_color: Color,
    pub line_type: LineType,
    pub read_only: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct QuadSpec {
    pub p0: LuaVec3,
    pub p1: LuaVec3,
    pub p2: LuaVec3,
    pub p3: LuaVec3,
    pub color: Color,
    pub fill_color: Color,
    pub line_type: LineType,
    pub read_only: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextSpec {
    pub pos: LuaVec3,
    pub color: Color,
    pub fill_color: Color,
    pub font_size: u8,
    pub read_only: bool,
    pub text: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ArrowSpec {
    pub start: LuaVec3,
    pub end: LuaVec3,
    pub color: Color,
    pub fill_color: Color,
    pub line_type: LineType,
    pub read_only: bool,
}

wrapped_table!(Action, None);

impl<'lua> Action<'lua> {
    pub fn explosion(&self, position: LuaVec3, power: f32) -> Result<()> {
        Ok(self.call_function("explosion", (position, power))?)
    }

    pub fn smoke(&self, position: LuaVec3, color: SmokeColor) -> Result<()> {
        Ok(self.call_function("smoke", (position, color))?)
    }

    pub fn effect_smoke_big(
        &self,
        position: LuaVec3,
        preset: SmokePreset,
        density: f32,
        name: String,
    ) -> Result<()> {
        Ok(self.call_function("effectSmokeBig", (position, preset, density, name))?)
    }

    pub fn effect_smoke_stop(&self, name: String) -> Result<()> {
        Ok(self.call_function("effectSmokeStop", name)?)
    }

    pub fn illumination_bomb(&self, position: LuaVec3, power: f32) -> Result<()> {
        Ok(self.call_function("illuminationBomb", (position, power))?)
    }

    pub fn signal_flare(&self, position: LuaVec3, color: FlareColor, azimuth: u16) -> Result<()> {
        Ok(self.call_function("signalFlare", (position, color, azimuth))?)
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
    ) -> Result<()> {
        Ok(self.call_function(
            "radioTransmission",
            (file, origin, modulation, repeat, frequency, power, name),
        )?)
    }

    pub fn stop_transmission(&self, name: String) -> Result<()> {
        Ok(self.call_function("stopRadioTransmission", name)?)
    }

    pub fn set_unit_internal_cargo(&self, unit_name: String, mass: i64) -> Result<()> {
        Ok(self.call_function("setUnitInternalCargo", (unit_name, mass))?)
    }

    pub fn out_sound(&self, file: String) -> Result<()> {
        Ok(self.call_function("outSound", file)?)
    }

    pub fn out_sound_for_coalition(&self, side: Side, file: String) -> Result<()> {
        Ok(self.call_function("outSoundForCoalition", (side, file))?)
    }

    pub fn out_sound_for_country(&self, country: Country, file: String) -> Result<()> {
        Ok(self.call_function("outSoundForCountry", (country, file))?)
    }

    pub fn out_sound_for_group(&self, group: GroupId, file: String) -> Result<()> {
        Ok(self.call_function("outSoundForGroup", (group, file))?)
    }

    pub fn out_sound_for_unit(&self, unit: UnitId, file: String) -> Result<()> {
        Ok(self.call_function("outSoundForUnit", (unit, file))?)
    }

    pub fn out_text(&self, text: String, display_time: i64, clear_view: bool) -> Result<()> {
        Ok(self.call_function("outText", (text, display_time, clear_view))?)
    }

    pub fn out_text_for_coalition(
        &self,
        side: Side,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> Result<()> {
        Ok(self.call_function(
            "outTextForCoalition",
            (side, text, display_time, clear_view),
        )?)
    }

    pub fn out_text_for_country(
        &self,
        country: Country,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> Result<()> {
        Ok(self.call_function(
            "outTextForCountry",
            (country, text, display_time, clear_view),
        )?)
    }

    pub fn out_text_for_group(
        &self,
        group: GroupId,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> Result<()> {
        Ok(self.call_function("outTextForGroup", (group, text, display_time, clear_view))?)
    }

    pub fn out_text_for_unit(
        &self,
        unit: GroupId,
        text: String,
        display_time: i64,
        clear_view: bool,
    ) -> Result<()> {
        Ok(self.call_function("outTextForUnit", (unit, text, display_time, clear_view))?)
    }

    pub fn mark_to_all(
        &self,
        id: MarkId,
        text: String,
        position: LuaVec3,
        read_only: bool,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function("markToAll", (id, text, position, read_only, message))?)
    }

    pub fn mark_to_coalition(
        &self,
        id: MarkId,
        text: String,
        position: LuaVec3,
        side: Side,
        read_only: bool,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function(
            "markToCoalition",
            (id, text, position, side, read_only, message),
        )?)
    }

    pub fn mark_to_group(
        &self,
        id: MarkId,
        text: String,
        position: LuaVec3,
        group: GroupId,
        read_only: bool,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function(
            "markToGroup",
            (id, text, position, group, read_only, message),
        )?)
    }

    pub fn remove_mark(&self, id: MarkId) -> Result<()> {
        Ok(self.call_function("removeMark", id)?)
    }

    pub fn line_to_all(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: LineSpec,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function(
            "lineToAll",
            (
                side,
                id,
                spec.start,
                spec.end,
                spec.color,
                spec.line_type,
                spec.read_only,
                message,
            ),
        )?)
    }

    pub fn circle_to_all(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: CircleSpec,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function(
            "circleToAll",
            (
                side,
                id,
                spec.center,
                spec.radius,
                spec.color,
                spec.fill_color,
                spec.line_type,
                spec.read_only,
                message,
            ),
        )?)
    }

    pub fn rect_to_all(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: RectSpec,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function(
            "rectToAll",
            (
                side,
                id,
                spec.start,
                spec.end,
                spec.color,
                spec.fill_color,
                spec.line_type,
                spec.read_only,
                message,
            ),
        )?)
    }

    pub fn quad_to_all(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: QuadSpec,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function(
            "quadToAll",
            (
                side,
                id,
                spec.p0,
                spec.p1,
                spec.p2,
                spec.p3,
                spec.color,
                spec.fill_color,
                spec.line_type,
                spec.read_only,
                message,
            ),
        )?)
    }

    pub fn text_to_all(&self, side: SideFilter, id: MarkId, spec: TextSpec) -> Result<()> {
        Ok(self.call_function(
            "textToAll",
            (
                side,
                id,
                spec.pos,
                spec.color,
                spec.fill_color,
                spec.font_size,
                spec.read_only,
                spec.text,
            ),
        )?)
    }

    pub fn arrow_to_all(
        &self,
        side: SideFilter,
        id: MarkId,
        spec: ArrowSpec,
        message: Option<String>,
    ) -> Result<()> {
        Ok(self.call_function(
            "arrowToAll",
            (
                side,
                id,
                spec.start,
                spec.end,
                spec.color,
                spec.fill_color,
                spec.line_type,
                spec.read_only,
                message,
            ),
        )?)
    }

    pub fn set_markup_radius(&self, id: MarkId, radius: f64) -> Result<()> {
        Ok(self.call_function("setMarkupRadius", (id, radius))?)
    }

    pub fn set_markup_text(&self, id: MarkId, text: String) -> Result<()> {
        Ok(self.call_function("setMarkupText", (id, text))?)
    }

    pub fn set_markup_font_size(&self, id: MarkId, font_size: u8) -> Result<()> {
        Ok(self.call_function("setMarkupFontSize", (id, font_size))?)
    }

    pub fn set_markup_color(&self, id: MarkId, color: Color) -> Result<()> {
        Ok(self.call_function("setMarkupColor", (id, color))?)
    }

    pub fn set_markup_fill_color(&self, id: MarkId, fill_color: Color) -> Result<()> {
        Ok(self.call_function("setMarkupColorFill", (id, fill_color))?)
    }

    pub fn set_markup_line_type(&self, id: MarkId, line_type: LineType) -> Result<()> {
        Ok(self.call_function("setMarkupTypeLine", (id, line_type))?)
    }

    pub fn set_markup_position_end(&self, id: MarkId, pos: LuaVec3) -> Result<()> {
        Ok(self.call_function("setMarkupPositionEnd", (id, pos))?)
    }

    pub fn set_markup_position_start(&self, id: MarkId, pos: LuaVec3) -> Result<()> {
        Ok(self.call_function("setMarkupPositionStart", (id, pos))?)
    }
}

wrapped_table!(Trigger, None);

impl<'lua> Trigger<'lua> {
    pub fn singleton(lua: MizLua<'lua>) -> Result<Self> {
        Ok(lua.inner().globals().raw_get("trigger")?)
    }

    pub fn action(&self) -> Result<Action<'lua>> {
        Ok(self.raw_get("action")?)
    }

    pub fn get_zone(&self, name: String) -> Result<Zone> {
        let misc: LuaTable = self.t.raw_get("misc")?;
        Ok(misc.call_function("getZone", name)?)
    }
}
