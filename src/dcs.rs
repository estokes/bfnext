use mlua::{prelude::*, Value};
use netidx_derive::Pack;
use serde_derive::{Deserialize, Serialize};
use std::ops::{Deref, DerefMut};

fn as_tbl<'lua>(to: &'static str, value: Value<'lua>) -> LuaResult<mlua::Table<'lua>> {
    match value {
        Value::Table(t) => Ok(t),
        _ => Err(LuaError::FromLuaConversionError {
            from: "value",
            to,
            message: None,
        }),
    }
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
        let tbl = as_tbl("ObjectId", value)?;
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
        let tbl = as_tbl("Shot", value)?;
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
        let tbl = as_tbl("WeaponUse", value)?;
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
        let tbl = as_tbl("UnitEvent", value)?;
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

impl<'lua> FromLua<'lua> for AtPlace{
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("AtPlace", value)?;
        
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
}
