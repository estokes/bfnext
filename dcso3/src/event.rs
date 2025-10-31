/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use std::marker::PhantomData;

use crate::{object::DcsOid, unit::ClassUnit};

use super::{
    as_tbl, as_tbl_ref, lua_err, object::Object, unit::Unit, value_to_json,
    weapon::Weapon, world::MarkPanel, String, Time,
};
use anyhow::{bail, Result};
use log::{error, info};
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
        let tbl = as_tbl("Shot", None, value).map_err(lua_err)?;
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
        let tbl = as_tbl("Shot", None, value).map_err(lua_err)?;
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
    pub initiator: Option<Object<'lua>>,
    pub target: Option<Object<'lua>>,
    pub weapon_name: String,
}

impl<'lua> FromLua<'lua> for WeaponUse<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("WeaponUse", None, value).map_err(lua_err)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            target: tbl.raw_get("target")?,
            weapon_name: tbl.raw_get("weapon_name")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LeaveUnit {
    pub initiator: Option<DcsOid<ClassUnit>>,
}

impl<'lua> FromLua<'lua> for LeaveUnit {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("LeaveUnit", None, value).map_err(lua_err)?;
        let tbl: Option<LuaTable> = tbl.raw_get("initiator")?;
        let initiator = tbl
            .map(|tbl| {
                Ok::<_, mlua::Error>(DcsOid {
                    id: tbl.raw_get("id_")?,
                    class: "Unit".into(),
                    t: PhantomData,
                })
            })
            .transpose()?;
        Ok(Self { initiator })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct UnitEvent<'lua> {
    pub time: Time,
    pub initiator: Option<Object<'lua>>,
}

impl<'lua> FromLua<'lua> for UnitEvent<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("UnitEvent", None, value).map_err(lua_err)?;
        Ok(Self { time: tbl.raw_get("time")?, initiator: tbl.raw_get("initiator")? })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct EjectionEvent<'lua> {
    pub time: Time,
    pub initiator: Object<'lua>,
    pub target: Object<'lua>,
}

impl<'lua> FromLua<'lua> for EjectionEvent<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("EjectionEvent", None, value).map_err(lua_err)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            target: tbl.raw_get("target")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Birth<'lua> {
    pub time: Time,
    pub initiator: Object<'lua>,
    pub place: Option<Object<'lua>>,
    pub subplace: Option<i64>,
}

impl<'lua> FromLua<'lua> for Birth<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("AtPlace", None, value).map_err(lua_err)?;
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
    pub place: Option<Object<'lua>>,
    pub subplace: Option<i64>,
}

impl<'lua> FromLua<'lua> for AtPlace<'lua> {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("AtPlace", None, value).map_err(lua_err)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            place: tbl.raw_get("place")?,
            subplace: tbl.raw_get("subPlace")?,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WeaponAdd<'lua> {
    pub time: Time,
    pub initiator: Object<'lua>,
    pub weapon_name: String,
}

impl<'lua> FromLua<'lua> for WeaponAdd<'lua> {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("WeaponAdd", None, value).map_err(lua_err)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            initiator: tbl.raw_get("initiator")?,
            weapon_name: tbl.raw_get("weapon_name")?,
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
    Crash(UnitEvent<'lua>),
    Ejection(EjectionEvent<'lua>),
    Refueling,
    Dead(UnitEvent<'lua>),
    PilotDead(UnitEvent<'lua>),
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
    PlayerLeaveUnit(LeaveUnit),
    PlayerComment,
    ShootingStart(WeaponUse<'lua>),
    ShootingEnd(ShootingEnd<'lua>),
    MarkAdded(MarkPanel<'lua>),
    MarkChange(MarkPanel<'lua>),
    MarkRemoved(MarkPanel<'lua>),
    Kill(WeaponUse<'lua>),
    Score(UnitEvent<'lua>),
    UnitLost(UnitEvent<'lua>),
    LandingAfterEjection,
    ParatrooperLanding,
    DiscardChairAfterEjection,
    WeaponAdd(WeaponAdd<'lua>),
    TriggerZone,
    LandingQualityMark,
    Bda,
    AiAbortMission(UnitEvent<'lua>),
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
    MacSubtaskScore,
    MacExtraScore,
    MissionRestart,
    MissionWinner,
    PostponedTakeoff(AtPlace<'lua>),
    PostponedLand(AtPlace<'lua>),
    Max,
}

fn translate<'a, 'lua: 'a>(
    lua: &'lua Lua,
    id: i64,
    value: Value<'lua>,
) -> Result<Event<'lua>> {
    Ok(match id {
        0 => Event::Invalid,
        1 => Event::Shot(Shot::from_lua(value, lua)?),
        2 => Event::Hit(WeaponUse::from_lua(value, lua)?),
        3 => Event::Takeoff(AtPlace::from_lua(value, lua)?),
        4 => Event::Land(AtPlace::from_lua(value, lua)?),
        5 => Event::Crash(UnitEvent::from_lua(value, lua)?),
        6 => Event::Ejection(EjectionEvent::from_lua(value, lua)?),
        7 => Event::Refueling,
        8 => Event::Dead(UnitEvent::from_lua(value, lua)?),
        9 => Event::PilotDead(UnitEvent::from_lua(value, lua)?),
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
        21 => Event::PlayerLeaveUnit(LeaveUnit::from_lua(value, lua)?),
        22 => Event::PlayerComment,
        23 => Event::ShootingStart(WeaponUse::from_lua(value, lua)?),
        24 => Event::ShootingEnd(ShootingEnd::from_lua(value, lua)?),
        25 => Event::MarkAdded(MarkPanel::from_lua(value, lua)?),
        26 => Event::MarkChange(MarkPanel::from_lua(value, lua)?),
        27 => Event::MarkRemoved(MarkPanel::from_lua(value, lua)?),
        28 => Event::Kill(WeaponUse::from_lua(value, lua)?),
        29 => Event::Score(UnitEvent::from_lua(value, lua)?),
        30 => Event::UnitLost(UnitEvent::from_lua(value, lua)?),
        31 => Event::LandingAfterEjection,
        32 => Event::ParatrooperLanding,
        33 => Event::DiscardChairAfterEjection,
        34 => Event::WeaponAdd(WeaponAdd::from_lua(value, lua)?),
        35 => Event::TriggerZone,
        36 => Event::LandingQualityMark,
        37 => Event::Bda,
        38 => Event::AiAbortMission(UnitEvent::from_lua(value, lua)?),
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
        51 => Event::MacSubtaskScore,
        52 => Event::MacExtraScore,
        53 => Event::MissionRestart,
        54 => {
            info!("mission winner event {}", value_to_json(&value));
            Event::MissionWinner
        }
        55 => Event::PostponedTakeoff(AtPlace::from_lua(value, lua)?),
        56 => Event::PostponedLand(AtPlace::from_lua(value, lua)?),
        57 => Event::Max,
        n => bail!("unknown event {n}"),
    })
}

impl<'lua> FromLua<'lua> for Event<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let id = as_tbl_ref("Event", &value).map_err(lua_err)?.raw_get("id")?;
        match translate(lua, id, value.clone()) {
            Ok(ev) => Ok(ev),
            Err(e) => {
                let s = value_to_json(&value);
                error!("error translating event {id}: {e:?}, value: {s}");
                Err(lua_err(e))
            }
        }
    }
}
