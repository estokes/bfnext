use super::{
    as_tbl, as_tbl_ref, cvt_err, object::Object, unit::Unit, weapon::Weapon, String, Time,
};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum BirthPlace {
    Air,
    Runway,
    Park,
    HeliportHot,
    HeliportCold,
}

#[derive(Debug, Clone, Serialize)]
pub struct Shot<'lua> {
    pub time: Time,
    pub initiator: Unit<'lua>,
    pub weapon: Weapon<'lua>,
    pub weapon_name: String,
}

impl<'lua> FromLua<'lua> for Shot<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Shot", None, value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            weapon: tbl.raw_get("weapon")?,
            weapon_name: tbl.raw_get("weapon_name")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ShootingEnd<'lua> {
    pub time: Time,
    pub initiator: Unit<'lua>,
    pub weapon_name: String,
}

impl<'lua> FromLua<'lua> for ShootingEnd<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Shot", None, value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            weapon_name: tbl.raw_get("weapon_name")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WeaponUse<'lua> {
    pub time: Time,
    pub initiator: Unit<'lua>,
    pub target: Object<'lua>,
    pub weapon_name: String,
}

impl<'lua> FromLua<'lua> for WeaponUse<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("WeaponUse", None, value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            target: tbl.raw_get("target")?,
            weapon_name: tbl.raw_get("weapon_name")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UnitEvent<'lua> {
    pub time: Time,
    pub initiator: Object<'lua>,
}

impl<'lua> FromLua<'lua> for UnitEvent<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("UnitEvent", None, value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Birth<'lua> {
    pub time: Time,
    pub initiator: Object<'lua>,
    pub place: Option<Object<'lua>>,
    pub subplace: Option<u32>,
}

impl<'lua> FromLua<'lua> for Birth<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("AtPlace", None, value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            place: tbl.raw_get("place")?,
            subplace: tbl.raw_get("subPlace")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct AtPlace<'lua> {
    pub time: Time,
    pub initiator: Object<'lua>,
    pub place: Object<'lua>,
    pub subplace: u32,
}

impl<'lua> FromLua<'lua> for AtPlace<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("AtPlace", None, value)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            place: tbl.raw_get("place")?,
            subplace: tbl.raw_get("subPlace")?,
        })
    }
}

/// This is a dcs event
#[derive(Debug, Clone, Serialize)]
pub enum Event<'lua> {
    Invalid,
    Shot(Shot<'lua>),
    Hit(WeaponUse<'lua>),
    Takeoff(AtPlace<'lua>),
    Land(AtPlace<'lua>),
    Crash,
    Ejection,
    Refueling,
    Dead(UnitEvent<'lua>),
    PilotDead,
    BaseCaptured,
    MissionStart,
    MissionEnd,
    TookControl,
    RefuelingStop,
    Birth(Birth<'lua>),
    HumanFailure,
    DetailedFailure,
    EngineStartup(AtPlace<'lua>),
    EngineShutdown(AtPlace<'lua>),
    PlayerEnterUnit(UnitEvent<'lua>),
    PlayerLeaveUnit,
    PlayerComment,
    ShootingStart(WeaponUse<'lua>),
    ShootingEnd(ShootingEnd<'lua>),
    MarkAdded,
    MarkChange,
    MarkRemoved,
    Kill(WeaponUse<'lua>),
    Score(UnitEvent<'lua>),
    UnitLost(UnitEvent<'lua>),
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

impl<'lua> FromLua<'lua> for Event<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl_ref("Event", &value)?;
        let ev = match tbl.raw_get("id")? {
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
            15 => Event::Birth(Birth::from_lua(value, lua)?),
            16 => Event::HumanFailure,
            17 => Event::DetailedFailure,
            18 => Event::EngineStartup(AtPlace::from_lua(value, lua)?),
            19 => Event::EngineShutdown(AtPlace::from_lua(value, lua)?),
            20 => Event::PlayerEnterUnit(UnitEvent::from_lua(value, lua)?),
            21 => Event::PlayerLeaveUnit,
            22 => Event::PlayerComment,
            23 => Event::ShootingStart(WeaponUse::from_lua(value, lua)?),
            24 => Event::ShootingEnd(ShootingEnd::from_lua(value, lua)?),
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
            _ => return Err(cvt_err("Event")),
        };
        Ok(ev)
    }
}
