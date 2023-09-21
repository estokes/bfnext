use mlua::{prelude::*, Value};
use netidx_derive::Pack;
use serde_derive::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

fn as_tbl<'a: 'lua, 'lua>(to: &'static str, value: &'a Value<'lua>) -> LuaResult<&'a mlua::Table<'lua>> {
    value
        .as_table()
        .ok_or_else(|| LuaError::FromLuaConversionError {
            from: "value",
            to,
            message: None,
        })
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Pack, Serialize, Deserialize)]
#[repr(transparent)]
pub struct String(compact_str::CompactString);

impl Deref for String {
    type Target = compact_str::CompactString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for String {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'lua> FromLua<'lua> for String {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        use compact_str::{format_compact, CompactString};
        match value {
            Value::String(s) => Ok(Self(CompactString::from(s.to_str()?))),
            Value::Boolean(b) => Ok(Self(format_compact!("{b}"))),
            Value::Integer(n) => Ok(Self(format_compact!("{n}"))),
            Value::Number(n) => Ok(Self(format_compact!("{n}"))),
            v => Ok(Self(CompactString::from(v.to_string()?))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Pack, Serialize, Deserialize)]
pub struct Time(f32);

impl<'lua> FromLua<'lua> for Time {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(value.as_f32().ok_or_else(|| {
            LuaError::FromLuaConversionError {
                from: "value",
                to: "Time",
                message: None,
            }
        })?))
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Pack, Serialize, Deserialize)]
pub struct ObjectId(u32);

impl<'lua> FromLua<'lua> for ObjectId {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("ObjectId", &value)?;
        Ok(Self(tbl.raw_get("id_")?))
    }
}

#[derive(Debug, Clone, PartialEq, Pack, Serialize, Deserialize)]
pub struct Vec2 {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, PartialEq, Pack, Serialize, Deserialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, PartialEq, Pack, Serialize, Deserialize)]
pub struct Box3 {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub enum VolumeType {
    Segment,
    Box,
    Sphere,
    Pyramid,
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub enum BirthPlace {
    Air,
    Runway,
    Park,
    HeliportHot,
    HeliportCold,
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub enum UnitCategory {
    Airplane,
    Helicopter,
    GroundUnit,
    Ship,
    Structure,
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub enum ObjectCategory {
    Unit,
    Weapon,
    Static,
    Base,
    Scenery,
    Cargo,
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub struct Shot {
    pub time: Time,
    pub initiator: ObjectId,
    pub weapon: ObjectId,
    pub weapon_name: String,
}

impl<'lua> FromLua<'lua> for Shot {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Shot", &value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            weapon: tbl.raw_get("weapon")?,
            weapon_name: tbl.raw_get("weapon_name")?,
        })
    }
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub struct WeaponUse {
    pub time: Time,
    pub initiator: ObjectId,
    pub target: ObjectId,
    pub weapon_name: String,
}

impl<'lua> FromLua<'lua> for WeaponUse {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("WeaponUse", &value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            target: tbl.raw_get("target")?,
            weapon_name: tbl.raw_get("weapon_name")?,
        })
    }
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub struct UnitEvent {
    pub time: Time,
    pub initiator: ObjectId,
}

impl<'lua> FromLua<'lua> for UnitEvent {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("UnitEvent", &value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
        })
    }
}

#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub struct AtPlace {
    pub time: Time,
    pub initiator: ObjectId,
    pub place: ObjectId,
    pub subplace: u32,
}

impl<'lua> FromLua<'lua> for AtPlace {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("AtPlace", &value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            place: tbl.raw_get("place")?,
            subplace: tbl.raw_get("subplace")?,
        })
    }
}

/// This is a dcs event
#[derive(Debug, Clone, Pack, Serialize, Deserialize)]
pub enum Event {
    Invalid,
    Shot(Shot),
    Hit(WeaponUse),
    Takeoff(AtPlace),
    Land(AtPlace),
    Crash,
    Ejection,
    Refueling,
    Dead(UnitEvent),
    PilotDead,
    BaseCaptured,
    MissionStart,
    MissionEnd,
    TookControl,
    RefuelingStop,
    Birth(AtPlace),
    HumanFailure,
    DetailedFailure,
    EngineStartup(AtPlace),
    EngineShutdown(AtPlace),
    PlayerEnterUnit(UnitEvent),
    PlayerLeaveUnit,
    PlayerComment,
    ShootingStart(WeaponUse),
    ShootingEnd(Shot),
    MarkAdded,
    MarkChange,
    MarkRemoved,
    Kill(WeaponUse),
    Score(UnitEvent),
    UnitLost(UnitEvent),
    LandingAfterEjection,
    ParatrooperLanding,
    DiscardChairAfterEjection,
    WeaponAdd,
    TriggerZone,
    LandingQualityMark,
    Bda,
    AiAbortMission,
    DayNight,
    FlightTime,
    PlayerSelfKillPilot,
    PlayerCaptureAirfield,
    EmergencyLanding,
    UnitCreateTask,
    UnitDeleteTask,
    SimulationStart,
    WeaponRearm,
    WeaponDrop,
    UnitTaskTimeout,
    UnitTaskStage,
    Max,
    Unknown(u32)
}

impl<'lua> FromLua<'lua> for Event {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Event", &value)?;
        let id: u32 = tbl.raw_get("id")?;
        let ev = match id {
            0 => Event::Invalid,
            1 => Event::Shot(Shot::from_lua(value, lua)?),
            2 => Event::Hit(WeaponUse::from_lua(value, lua)?),
            3 => Event::Takeoff(AtPlace::from_lua(value, lua)?),
            4 => Event::Land(AtPlace::from_lua(value, lua)?),
            5 => Event::Crash,
            6 => Event::Ejection,
            7 => Event::Refueling,
            8 => Event::Dead(UnitEvent::from_lua(value, lua)?),
            9 => Event::PilotDead,
            10 => Event::BaseCaptured,
            11 => Event::MissionStart,
            12 => Event::MissionEnd,
            13 => Event::TookControl,
            14 => Event::RefuelingStop,
            15 => Event::Birth(AtPlace::from_lua(value, lua)?),
            16 => Event::HumanFailure,
            17 => Event::DetailedFailure,
            18 => Event::EngineStartup(AtPlace::from_lua(value, lua)?),
            19 => Event::EngineShutdown(AtPlace::from_lua(value, lua)?),
            20 => Event::PlayerEnterUnit(UnitEvent::from_lua(value, lua)?),
            21 => Event::PlayerLeaveUnit,
            22 => Event::PlayerComment,
            23 => Event::ShootingStart(WeaponUse::from_lua(value, lua)?),
            24 => Event::ShootingEnd(Shot::from_lua(value, lua)?),
            25 => Event::MarkAdded,
            26 => Event::MarkChange,
            27 => Event::MarkRemoved,
            28 => Event::Kill(WeaponUse::from_lua(value, lua)?),
            29 => Event::Score(UnitEvent::from_lua(value, lua)?),
            30 => Event::UnitLost(UnitEvent::from_lua(value, lua)?),
            31 => Event::LandingAfterEjection,
            32 => Event::ParatrooperLanding,
            33 => Event::DiscardChairAfterEjection,
            34 => Event::WeaponAdd,
            35 => Event::TriggerZone,
            36 => Event::LandingQualityMark,
            37 => Event::Bda,
            38 => Event::AiAbortMission,
            39 => Event::DayNight,
            40 => Event::FlightTime,
            41 => Event::PlayerSelfKillPilot,
            42 => Event::PlayerCaptureAirfield,
            43 => Event::EmergencyLanding,
            44 => Event::UnitCreateTask,
            45 => Event::UnitDeleteTask,
            46 => Event::SimulationStart,
            47 => Event::WeaponRearm,
            48 => Event::WeaponDrop,
            49 => Event::UnitTaskTimeout,
            50 => Event::UnitTaskStage,
            51 => Event::Max,
            u => Event::Unknown(u),
        };
        Ok(ev)
    }
}
