/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::{as_tbl, attribute::Attributes, cvt_err, object::Object, LuaVec3, String};
use crate::{
    airbase::{AirbaseId, RunwayId},
    attribute::Attribute,
    bitflags_enum,
    env::miz::{GroupId, TriggerZoneId, UnitId},
    err, lua_err, simple_enum,
    static_object::StaticObjectId,
    string_enum,
    trigger::Modulation,
    wrapped_table, LuaVec2, Sequence, Time,
};
use anyhow::{bail,Result};
use compact_str::format_compact;
use enumflags2::{bitflags, BitFlags};
use mlua::{prelude::*, Value, Variadic};
use na::Vector2;
use serde_derive::{Deserialize, Serialize};
use std::{mem, ops::Deref};

string_enum!(PointType, u8, [
    TakeOffGround => "TakeOffGround",
    TakeOffGroundHot => "TakeOffGroundHot",
    TurningPoint => "Turning Point",
    TakeOffParking => "TakeOffParking",
    TakeOff => "TakeOff",
    Land => "Land",
    Nil => ""
]);

string_enum!(WeaponExpend, u8, [
    Quarter => "QUARTER",
    Two => "TWO",
    One => "ONE",
    Four => "FOUR",
    Half => "HALF",
    All => "ALL"
]);

string_enum!(OrbitPattern, u8, [
    RaceTrack => "Race-Track",
    Circle => "Circle"
]);

string_enum!(TurnMethod, u8, [
    FlyOverPoint => "Fly Over Point",
    OffRoad => "Off Road"
]);

string_enum!(Designation, u8, [
    No => "No",
    WP => "WP",
    IrPointer => "IR-Pointer",
    Laser => "Laser",
    Auto => "Auto"
]);

string_enum!(AltType, u8, [
    BARO => "BARO",
    RADIO => "RADIO"
]);

simple_enum!(FACCallsign, u8, [
    Axeman	=> 1,
    Darknight => 2,
    Warrior => 3,
    Pointer	=> 4,
    Eyeball	=> 5,
    Moonbeam => 6,
    Whiplash => 7,
    Finger => 8,
    Pinpoint => 9,
    Ferret => 10,
    Shaba => 11,
    Playboy	=> 12,
    Hammer => 13,
    Jaguar => 14,
    Deathstar => 15,
    Anvil => 16,
    Firefly => 17,
    Mantis => 18,
    Badger => 19
]);

#[derive(Debug, Clone)]
pub struct AttackParams {
    pub weapon_type: Option<u64>, // weapon flag(s)
    pub point: Option<LuaVec2>,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub expend: Option<WeaponExpend>,
    pub direction: Option<f64>, // in radians
    pub direction_enabled: Option<bool>,
    pub altitude: Option<f64>,
    pub altitude_enabled: Option<bool>,
    pub attack_qty: Option<i64>,
    pub attack_qty_limit: Option<bool>,
    pub group_attack: Option<bool>,
}

impl<'lua> FromLua<'lua> for AttackParams {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: LuaTable = FromLua::from_lua(value, lua)?;
        Ok(Self {
            weapon_type: tbl.raw_get("weaponType")?,
            expend: tbl.raw_get("expend")?,
            direction: if tbl.raw_get("directionEnabled")? {
                tbl.raw_get("direction")?
            } else {
                None
            },
            altitude: if tbl.raw_get("altitudeEnabled")? {
                tbl.raw_get("altitude")?
            } else {
                None
            },
            attack_qty_limit: tbl.raw_get("attackQtyLimit")?,
            attack_qty: if tbl.raw_get("attackQtyLimit")? {
                tbl.raw_get("attackQty")?
            } else {
                None
            },
            group_attack: tbl.raw_get("groupAttack")?,
            altitude_enabled: tbl.raw_get("altitudeEnabled")?,
            direction_enabled: tbl.raw_get("directionEnabled")?,
            point: tbl.raw_get("point")?,
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
        })
    }
}

impl AttackParams {
    fn push_tbl(&self, tbl: &LuaTable) -> LuaResult<()> {
        // if let Some(wt) = self.weapon_type {
        //     tbl.raw_set("weaponType", wt)?
        // }
        // if let Some(exp) = &self.expend {
        //     tbl.raw_set("expend", exp.to_owned())?
        // }
        // if let Some(dir) = self.direction {
        //     tbl.raw_set("direction", dir)?
        // }
        // if let Some(alt) = self.altitude {
        //     tbl.raw_set("altitude", alt)?;
        // }
        if let Some(qty) = self.attack_qty {
            tbl.raw_set("attackQty", qty)?;
        }
        // if let Some(alt_enabled) = self.altitude_enabled {
        //     tbl.raw_set("altitudeEnabled", alt_enabled)?;
        // }
        // if let Some(qty_limit) = self.attack_qty_limit {
        //     tbl.raw_set("attackQtyLimit", qty_limit)?;
        // }
        // if let Some(grp) = self.group_attack {
        //     tbl.raw_set("groupAttack", grp)?;
        // }
        // if let Some(dir_enabled) = self.direction_enabled {
        //     tbl.raw_set("directionEnabled", dir_enabled)?;
        // }
        // if let Some(point) = self.point {
        //     tbl.raw_set("point", point)?;
        // }
        if let Some(x) = self.x {
            tbl.raw_set("x", x)?;
        }
        if let Some(y) = self.y {
            tbl.raw_set("y", y)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FollowParams {
    pub group: GroupId,
    pub pos: LuaVec3,
    pub last_waypoint_index: Option<i64>,
}

impl<'lua> FromLua<'lua> for FollowParams {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: LuaTable = FromLua::from_lua(value, lua)?;
        Ok(Self {
            group: tbl.raw_get("groupId")?,
            pos: tbl.raw_get("pos")?,
            last_waypoint_index: if tbl.raw_get("lastWptIndexFlag")? {
                tbl.raw_get("lastWptIndex")?
            } else {
                None
            },
        })
    }
}

impl FollowParams {
    fn push_tbl(&self, tbl: &LuaTable) -> LuaResult<()> {
        tbl.raw_set("groupId", self.group)?;
        tbl.raw_set("pos", self.pos)?;
        if let Some(idx) = self.last_waypoint_index {
            tbl.raw_set("lastWptIndexFlag", true)?;
            tbl.raw_set("lastWptIndex", idx)?
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FACParams {
    pub weapon_type: Option<u64>, // weapon flag(s),
    pub designation: Option<Designation>,
    pub datalink: Option<bool>,
    pub frequency: Option<f64>,
    pub modulation: Option<Modulation>,
    pub callname: Option<FACCallsign>,
    pub number: Option<u8>,
}

impl<'lua> FromLua<'lua> for FACParams {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: LuaTable = FromLua::from_lua(value, lua)?;
        Ok(Self {
            weapon_type: tbl.raw_get("weaponType")?,
            designation: tbl.raw_get("designation")?,
            datalink: tbl.raw_get("datalink")?,
            frequency: tbl.raw_get("frequency")?,
            modulation: tbl.raw_get("modulation")?,
            callname: tbl.raw_get("callname")?,
            number: tbl.raw_get("number")?,
        })
    }
}

impl FACParams {
    fn push_tbl(&self, tbl: &LuaTable) -> LuaResult<()> {
        if let Some(wt) = self.weapon_type {
            tbl.raw_set("weaponType", wt)?;
        }
        if let Some(d) = &self.designation {
            tbl.raw_set("designation", d.clone())?;
        }
        if let Some(dl) = self.datalink {
            tbl.raw_set("datalink", dl)?;
        }
        if let Some(frq) = self.frequency {
            tbl.raw_set("frequency", frq)?;
        }
        if let Some(md) = self.modulation {
            tbl.raw_set("modulation", md)?;
        }
        if let Some(cn) = self.callname {
            tbl.raw_set("callname", cn)?;
        }
        if let Some(n) = self.number {
            tbl.raw_set("number", n)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub enum ActionTyp {
    Air(TurnMethod),
    Ground(VehicleFormation),
}

impl<'lua> IntoLua<'lua> for ActionTyp {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        match self {
            ActionTyp::Air(a) => IntoLua::into_lua(a, lua),
            ActionTyp::Ground(a) => IntoLua::into_lua(a, lua),
        }
    }
}

impl<'lua> FromLua<'lua> for ActionTyp {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match TurnMethod::from_lua(value.clone(), lua) {
            Ok(v) => Ok(Self::Air(v)),
            Err(te) => match VehicleFormation::from_lua(value, lua) {
                Ok(v) => Ok(Self::Ground(v)),
                Err(ve) => Err(err(&format_compact!(
                    "unknown action turn method: {te:?}, vehicle formation: {ve:?}"
                ))),
            },
        }
    }
}

#[derive(Debug, Clone)]
pub struct MissionPoint<'lua> {
    pub typ: PointType,
    pub airdrome_id: Option<AirbaseId>,
    pub time_re_fu_ar: Option<i64>,
    pub helipad: Option<AirbaseId>,
    pub link_unit: Option<UnitId>,
    pub action: Option<ActionTyp>,
    pub pos: LuaVec2,
    pub alt: f64,
    pub alt_typ: Option<AltType>,
    pub speed: f64,
    pub speed_locked: Option<bool>,
    pub eta: Option<Time>,
    pub eta_locked: Option<bool>,
    pub name: Option<String>,
    pub task: Box<Task<'lua>>,
}

impl<'lua> FromLua<'lua> for MissionPoint<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: LuaTable = FromLua::from_lua(value, lua)?;
        Ok(Self {
            typ: tbl.raw_get("type")?,
            airdrome_id: tbl.raw_get("airdromId")?,
            time_re_fu_ar: tbl.raw_get("timeReFuAr")?,
            helipad: tbl.raw_get("helipadId")?,
            link_unit: tbl.raw_get("linkUnit")?,
            action: tbl.raw_get("action")?,
            pos: LuaVec2(Vector2::new(tbl.raw_get("x")?, tbl.raw_get("y")?)),
            alt: tbl.raw_get("alt")?,
            alt_typ: tbl.raw_get("alt_type")?,
            speed: tbl.raw_get("speed")?,
            speed_locked: tbl.raw_get("speed_locked")?,
            eta: tbl.raw_get("ETA")?,
            eta_locked: tbl.raw_get("ETA_locked")?,
            name: tbl.raw_get("name")?,
            task: Box::new(tbl.raw_get("task")?),
        })
    }
}

impl<'lua> IntoLua<'lua> for MissionPoint<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let iter = [
            ("type", self.typ.into_lua(lua)?),
            ("airdromId", self.airdrome_id.into_lua(lua)?),
            ("timeReFuAr", self.time_re_fu_ar.into_lua(lua)?),
            ("helipadId", self.helipad.into_lua(lua)?),
            ("linkUnit", self.link_unit.into_lua(lua)?),
            ("action", self.action.into_lua(lua)?),
            ("x", self.pos.x.into_lua(lua)?),
            ("y", self.pos.y.into_lua(lua)?),
            ("alt", self.alt.into_lua(lua)?),
            ("alt_type", self.alt_typ.into_lua(lua)?),
            ("speed", self.speed.into_lua(lua)?),
            ("speed_locked", self.speed_locked.into_lua(lua)?),
            ("ETA", self.eta.into_lua(lua)?),
            ("ETA_locked", self.eta_locked.into_lua(lua)?),
            ("name", self.name.into_lua(lua)?),
            ("task", self.task.into_lua(lua)?),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_nil());
        Ok(Value::Table(lua.create_table_from(iter)?))
    }
}

#[derive(Debug, Clone)]
pub struct TaskStartCond<'lua> {
    pub time: Option<Time>,
    pub user_flag: Option<Value<'lua>>,
    pub user_flag_value: Option<Value<'lua>>,
    pub probability: Option<u8>,
    pub condition: Option<String>, // lua code
}

impl<'lua> FromLua<'lua> for TaskStartCond<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: LuaTable = FromLua::from_lua(value, lua)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            user_flag: tbl.raw_get("userFlag")?,
            user_flag_value: tbl.raw_get("userFlagValue")?,
            probability: tbl.raw_get("probability")?,
            condition: tbl.raw_get("condition")?,
        })
    }
}

impl<'lua> IntoLua<'lua> for TaskStartCond<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let iter = [
            ("time", self.time.into_lua(lua)?),
            ("userFlag", self.user_flag.into_lua(lua)?),
            ("userFlagValue", self.user_flag_value.into_lua(lua)?),
            ("probability", self.probability.into_lua(lua)?),
            ("condition", self.condition.into_lua(lua)?),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_nil());
        Ok(Value::Table(lua.create_table_from(iter)?))
    }
}

#[derive(Debug, Clone)]
pub struct TaskStopCond<'lua> {
    pub time: Option<Time>,
    pub user_flag: Option<Value<'lua>>,
    pub user_flag_value: Option<Value<'lua>>,
    pub last_waypoint: Option<i64>,
    pub duration: Option<Time>,
    pub condition: Option<String>, // lua code
}

impl<'lua> FromLua<'lua> for TaskStopCond<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: LuaTable = FromLua::from_lua(value, lua)?;
        Ok(Self {
            time: tbl.raw_get("time")?,
            user_flag: tbl.raw_get("userFlag")?,
            user_flag_value: tbl.raw_get("userFlagValue")?,
            last_waypoint: tbl.raw_get("lastWaypoint")?,
            duration: tbl.raw_get("duration")?,
            condition: tbl.raw_get("condition")?,
        })
    }
}

impl<'lua> IntoLua<'lua> for TaskStopCond<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let iter = [
            ("time", self.time.into_lua(lua)?),
            ("userFlag", self.user_flag.into_lua(lua)?),
            ("userFlagValue", self.user_flag_value.into_lua(lua)?),
            ("lastWaypoint", self.last_waypoint.into_lua(lua)?),
            ("duration", self.duration.into_lua(lua)?),
            ("condition", self.condition.into_lua(lua)?),
        ]
        .into_iter()
        .filter(|(_, v)| !v.is_nil());
        Ok(Value::Table(lua.create_table_from(iter)?))
    }
}

#[derive(Debug, Clone)]
pub enum Task<'lua> {
    AttackGroup {
        group: GroupId,
        params: AttackParams,
    },
    AttackUnit {
        unit: UnitId,
        params: AttackParams,
    },
    Bombing {
        params: AttackParams,
    },
    Strafing {
        point: LuaVec2,
        length: f64,
        params: AttackParams,
    },
    CarpetBombing {
        point: LuaVec2,
        carpet_length: f64,
        params: AttackParams,
    },
    AttackMapObject {
        point: LuaVec2,
        params: AttackParams,
    },
    BombRunway {
        runway: RunwayId,
        params: AttackParams,
    },
    Orbit {
        pattern: OrbitPattern,
        point: Option<LuaVec2>,
        point2: Option<LuaVec2>,
        speed: Option<f64>,
        altitude: Option<f64>,
    },
    Refuelling,
    Land {
        point: LuaVec2,
        duration: Option<Time>,
    },
    Follow(FollowParams),
    FollowBigFormation(FollowParams),
    Escort {
        engagement_dist_max: f64,
        target_types: Vec<Attribute>,
        params: FollowParams,
    },
    Embarking {
        pos: LuaVec2,
        groups_for_embarking: Vec<GroupId>,
        duration: Option<Time>,
        distribution: Option<Vec<UnitId>>,
    },
    FireAtPoint {
        point: LuaVec2,
        radius: Option<f64>,
        expend_qty: Option<i64>,
        weapon_type: Option<u64>, // weapon flag(s)
        altitude: Option<f64>,
        altitude_type: Option<AltType>,
    },
    Hold,
    FACAttackGroup {
        group: GroupId,
        params: FACParams,
    },
    EmbarkToTransport {
        pos: LuaVec2,
        radius: Option<f64>,
    },
    DisembarkFromTransport {
        pos: LuaVec2,
        radius: Option<f64>,
    },
    CargoTransportation {
        group: Option<StaticObjectId>,
        zone: Option<TriggerZoneId>,
    },
    GoToWaypoint {
        from_waypoint: i64,
        to_waypoint: i64,
    },
    GroundEscort {
        group: GroupId,
        engagement_max_distance: f64,
        target_types: Vec<Attribute>,
        last_wpt_index: Option<i64>,
    },
    RecoveryTanker {
        group: GroupId,
        speed: f64,
        altitude: f64,
        last_wpt_idx: Option<i64>,
    },
    EngageTargets {
        target_types: Vec<Attribute>,
        max_dist: Option<f64>,
        priority: Option<i64>,
    },
    EngageTargetsInZone {
        point: LuaVec2,
        zone_radius: f64,
        target_types: Vec<Attribute>,
        priority: Option<i64>,
    },
    EngageGroup {
        group: GroupId,
        params: AttackParams,
        priority: Option<i64>,
    },
    EngageUnit {
        unit: UnitId,
        params: AttackParams,
        priority: Option<i64>,
    },
    AWACS,
    Tanker,
    EWR,
    PinpointStrike,
    FACEngageGroup {
        group: GroupId,
        params: FACParams,
        priority: Option<i64>,
    },
    FAC {
        params: FACParams,
        priority: Option<i64>,
    },
    Mission {
        airborne: Option<bool>,
        route: Vec<MissionPoint<'lua>>,
    },
    ComboTask(Vec<Task<'lua>>),
    ControlledTask {
        task: Box<Task<'lua>>,
        condition: TaskStartCond<'lua>,
        stop_condition: TaskStopCond<'lua>,
    },
    WrappedCommand(Command),
    WrappedOption(AiOption<'lua>),
}

impl<'lua> FromLua<'lua> for Task<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let root: LuaTable = FromLua::from_lua(value, lua)?;
        let id: String = root.raw_get("id")?;
        let params = match root.raw_get::<_, Option<LuaTable>>("params")? {
            Some(tbl) => tbl,
            None => lua.create_table()?,
        };
        match id.as_str() {
            "AttackGroup" => Ok(Self::AttackGroup {
                group: params.raw_get("groupId")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "AttackUnit" => Ok(Self::AttackUnit {
                unit: params.raw_get("unitId")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "Bombing" => Ok(Self::Bombing {
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "Strafing" => Ok(Self::Strafing {
                point: params.raw_get("point")?,
                length: params.raw_get("length")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "CarpetBombing" => Ok(Self::CarpetBombing {
                point: params.raw_get("point")?,
                carpet_length: params.raw_get("carpetLength")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "AttackMapObject" => Ok(Self::AttackMapObject {
                point: params.raw_get("point")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "BombingRunway" => Ok(Self::BombRunway {
                runway: params.raw_get("runwayId")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "Orbit" => Ok(Self::Orbit {
                pattern: params.raw_get("pattern")?,
                point: params.raw_get("point")?,
                point2: params.raw_get("point2")?,
                speed: params.raw_get("speed")?,
                altitude: params.raw_get("altitude")?,
            }),
            "Refueling" => Ok(Self::Refuelling),
            "Follow" => Ok(Self::Follow(FromLua::from_lua(Value::Table(params), lua)?)),
            "FollowBigFormation" => Ok(Self::FollowBigFormation(FromLua::from_lua(
                Value::Table(params),
                lua,
            )?)),
            "Escort" => Ok(Self::Escort {
                engagement_dist_max: params.raw_get("engagementDistMax")?,
                target_types: params.raw_get("targetTypes")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "Embarking" => Ok(Self::Embarking {
                pos: LuaVec2(Vector2::new(params.raw_get("x")?, params.raw_get("y")?)),
                groups_for_embarking: params.raw_get("groupsForEmbarking")?,
                duration: params.raw_get("duration")?,
                distribution: params.raw_get("distribution")?,
            }),
            "FireAtPoint" => Ok(Self::FireAtPoint {
                point: params.raw_get("point")?,
                radius: params.raw_get("radius")?,
                expend_qty: if params.raw_get("expendQtyEnabled")? {
                    params.raw_get("expendQty")?
                } else {
                    None
                },
                weapon_type: params.raw_get("weaponType")?,
                altitude: params.raw_get("altitude")?,
                altitude_type: params.raw_get("alt_type")?,
            }),
            "Hold" => Ok(Self::Hold),
            "FAC_AttackGroup" => Ok(Self::FACAttackGroup {
                group: params.raw_get("groupId")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "EmbarkToTransport" => Ok(Self::EmbarkToTransport {
                pos: LuaVec2(Vector2::new(params.raw_get("x")?, params.raw_get("y")?)),
                radius: params.raw_get("zoneRadius")?,
            }),
            "DisembarkFromTransport" => Ok(Self::DisembarkFromTransport {
                pos: LuaVec2(Vector2::new(params.raw_get("x")?, params.raw_get("y")?)),
                radius: params.raw_get("zoneRadius")?,
            }),
            "CargoTransportation" => Ok(Self::CargoTransportation {
                group: params.raw_get("groupId")?,
                zone: params.raw_get("zoneId")?,
            }),
            // "goToGulag" => Ok(Self::GoToGulag),
            "goToWaypoint" => Ok(Self::GoToWaypoint {
                from_waypoint: params.raw_get("fromWaypointIndex")?,
                to_waypoint: params.raw_get("goToWaypointIndex")?,
            }),
            "GroundEscort" => Ok(Self::GroundEscort {
                group: params.raw_get("groupId")?,
                engagement_max_distance: params.raw_get("engagementDistMax")?,
                target_types: params.raw_get("targetTypes")?,
                last_wpt_index: if params.raw_get("lastWptIndexFlag")? {
                    params.raw_get("lastWptIndex")?
                } else {
                    None
                },
            }),
            "RecoveryTanker" => Ok(Self::RecoveryTanker {
                group: params.raw_get("groupId")?,
                speed: params.raw_get("speed")?,
                altitude: params.raw_get("altitude")?,
                last_wpt_idx: if params.raw_get("lastWptIndexFlag")? {
                    params.raw_get("lastWptIndex")?
                } else {
                    None
                },
            }),
            "EngageTargets" => Ok(Self::EngageTargets {
                target_types: params.raw_get("targetTypes")?,
                max_dist: params.raw_get("maxDist")?,
                priority: params.raw_get("priority")?,
            }),
            "EngageTargetsInZone" => Ok(Self::EngageTargetsInZone {
                point: params.raw_get("point")?,
                zone_radius: params.raw_get("zoneRadius")?,
                target_types: params.raw_get("targetTypes")?,
                priority: params.raw_get("priority")?,
            }),
            "EngageGroup" => Ok(Self::EngageGroup {
                group: params.raw_get("groupId")?,
                priority: params.raw_get("priority")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "EngageUnit" => Ok(Self::EngageUnit {
                unit: params.raw_get("unitId")?,
                priority: params.raw_get("priority")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "AWACS" => Ok(Self::AWACS),
            "Tanker" => Ok(Self::Tanker),
            "EWR" => Ok(Self::EWR),
            "FAC_EngageGroup" => Ok(Self::FACEngageGroup {
                group: params.raw_get("groupId")?,
                priority: params.raw_get("priority")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "FAC" => Ok(Self::FAC {
                priority: params.raw_get("priority")?,
                params: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "Mission" => Ok(Self::Mission {
                airborne: params.raw_get("airborne")?,
                route: FromLua::from_lua(Value::Table(params), lua)?,
            }),
            "ComboTask" => Ok(Self::ComboTask(FromLua::from_lua(
                Value::Table(params.raw_get("tasks")?),
                lua,
            )?)),
            "ControlledTask" => Ok(Self::ControlledTask {
                task: Box::new(params.raw_get("task")?),
                condition: params.raw_get("condition")?,
                stop_condition: params.raw_get("stopCondition")?,
            }),
            "WrappedAction" => {
                let action: LuaTable = params.raw_get("action")?;
                match action.raw_get::<_, String>("id")?.as_str() {
                    "Option" => {
                        let params: LuaTable = action.raw_get("params")?;
                        let tag: u8 = params.raw_get("name")?;
                        let val: Value = params.raw_get("value")?;
                        Ok(Self::WrappedOption(AiOption::from_tag_val(lua, tag, val)?))
                    }
                    _cmd => Ok(Self::WrappedCommand(params.raw_get("action")?)),
                }
            }
            s => Err(err(&format_compact!("invalid action {s}"))),
        }
    }
}

impl<'lua> IntoLua<'lua> for Task<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let root = lua.create_table()?;
        let params = lua.create_table()?;
        match self {
            Self::AttackGroup { group, params: atp } => {
                root.raw_set("id", "AttackGroup")?;
                params.raw_set("groupId", group)?;
                atp.push_tbl(&params)?;
            }
            Self::AttackUnit { unit, params: atp } => {
                root.raw_set("id", "AttackUnit")?;
                params.raw_set("unitId", unit)?;
                atp.push_tbl(&params)?;
            }
            Self::Bombing {params: atp } => {
                root.raw_set("id", "Bombing")?;
                atp.push_tbl(&params)?;
            }
            Self::Strafing {
                point,
                length,
                params: atp,
            } => {
                root.raw_set("id", "Strafing")?;
                params.raw_set("point", point)?;
                params.raw_set("length", length)?;
                atp.push_tbl(&params)?;
            }
            Self::CarpetBombing {
                point,
                carpet_length,
                params: atp,
            } => {
                root.raw_set("id", "CarpetBombing")?;
                params.raw_set("point", point)?;
                params.raw_set("carpetLength", carpet_length)?;
                atp.push_tbl(&params)?;
            }
            Self::AttackMapObject { point, params: atp } => {
                root.raw_set("id", "AttackMapObject")?;
                params.raw_set("point", point)?;
                atp.push_tbl(&params)?;
            }
            Self::BombRunway {
                runway,
                params: atp,
            } => {
                root.raw_set("id", "BombingRunway")?;
                params.raw_set("runwayId", runway)?;
                atp.push_tbl(&params)?;
            }
            Self::Orbit {
                pattern,
                point,
                point2,
                speed,
                altitude,
            } => {
                root.raw_set("id", "Orbit")?;
                params.raw_set("pattern", pattern)?;
                params.raw_set("point", point)?;
                params.raw_set("point2", point2)?;
                params.raw_set("speed", speed)?;
                params.raw_set("altitude", altitude)?;
            }
            Self::Refuelling => root.raw_set("id", "Refueling")?,
            Self::Land { point, duration } => {
                root.raw_set("id", "Land")?;
                params.raw_set("point", point)?;
                if let Some(dur) = duration {
                    params.raw_set("durationFlag", true)?;
                    params.raw_set("duration", dur)?;
                }
            }
            Self::Follow(fp) => {
                root.raw_set("id", "Follow")?;
                fp.push_tbl(&params)?;
            }
            Self::FollowBigFormation(fp) => {
                root.raw_set("id", "FollowBigFormation")?;
                fp.push_tbl(&params)?;
            }
            Self::Escort {
                engagement_dist_max,
                target_types,
                params: fp,
            } => {
                root.raw_set("id", "Escort")?;
                params.raw_set("engagementDistMax", engagement_dist_max)?;
                params.raw_set("targetTypes", target_types)?;
                fp.push_tbl(&params)?;
            }
            Self::Embarking {
                pos,
                groups_for_embarking,
                duration,
                distribution,
            } => {
                root.raw_set("id", "Embarking")?;
                params.raw_set("x", pos.x)?;
                params.raw_set("y", pos.y)?;
                params.raw_set("groupsForEmbarking", groups_for_embarking)?;
                if let Some(dur) = duration {
                    params.raw_set("duration", dur)?;
                }
                if let Some(dist) = distribution {
                    params.raw_set("distribution", dist)?;
                }
            }
            Self::FireAtPoint {
                point,
                radius,
                expend_qty,
                weapon_type,
                altitude,
                altitude_type,
            } => {
                root.raw_set("id", "FireAtPoint")?;
                params.raw_set("point", point)?;
                if let Some(radius) = radius {
                    params.raw_set("radius", radius)?;
                }
                if let Some(qty) = expend_qty {
                    params.raw_set("expendQtyEnabled", true)?;
                    params.raw_set("expendQty", qty)?;
                }
                if let Some(wt) = weapon_type {
                    params.raw_set("weaponType", wt)?;
                }
                if let Some(alt) = altitude {
                    params.raw_set("altitude", alt)?;
                }
                if let Some(at) = altitude_type {
                    params.raw_set("alt_type", at)?;
                }
            }
            Self::Hold => root.raw_set("id", "Hold")?,
            Self::PinpointStrike => root.raw_set("id", "Pinpoint Strike")?,
            Self::FACAttackGroup { group, params: fp } => {
                root.raw_set("id", "FAC_AttackGroup")?;
                params.raw_set("groupId", group)?;
                fp.push_tbl(&params)?;
            }
            Self::EmbarkToTransport { pos, radius } => {
                root.raw_set("id", "EmbarkToTransport")?;
                params.raw_set("x", pos.x)?;
                params.raw_set("y", pos.y)?;
                if let Some(rad) = radius {
                    params.raw_set("zoneRadius", rad)?
                }
            }
            Self::DisembarkFromTransport { pos, radius } => {
                root.raw_set("id", "DisembarkFromTransport")?;
                params.raw_set("x", pos.x)?;
                params.raw_set("y", pos.y)?;
                if let Some(rad) = radius {
                    params.raw_set("zoneRadius", rad)?;
                }
            }
            Self::CargoTransportation { group, zone } => {
                root.raw_set("id", "CargoTransportation")?;
                if let Some(gid) = group {
                    params.raw_set("groupId", gid)?;
                }
                if let Some(zone) = zone {
                    params.raw_set("zoneId", zone)?;
                }
            }
            Self::GoToWaypoint {
                from_waypoint,
                to_waypoint,
            } => {
                root.raw_set("id", "goToWaypoint")?;
                params.raw_set("fromWaypointIndex", from_waypoint)?;
                params.raw_set("goToWaypointIndex", to_waypoint)?;
            }
            Self::GroundEscort {
                group,
                engagement_max_distance,
                target_types,
                last_wpt_index,
            } => {
                root.raw_set("id", "GroundEscort")?;
                params.raw_set("groupId", group)?;
                params.raw_set("engagementDistMax", engagement_max_distance)?;
                params.raw_set("targetTypes", target_types)?;
                if let Some(wpi) = last_wpt_index {
                    params.raw_set("lastWptIndexFlag", true)?;
                    params.raw_set("lastWptIndex", wpi)?;
                }
            }
            Self::RecoveryTanker {
                group,
                speed,
                altitude,
                last_wpt_idx,
            } => {
                root.raw_set("id", "RecoveryTanker")?;
                params.raw_set("groupId", group)?;
                params.raw_set("speed", speed)?;
                params.raw_set("altitude", altitude)?;
                if let Some(idx) = last_wpt_idx {
                    params.raw_set("lastWptIndexFlag", true)?;
                    params.raw_set("lastWptIndex", idx)?;
                }
            }
            Self::EngageTargets {
                target_types,
                max_dist,
                priority,
            } => {
                root.raw_set("id", "EngageTargets")?;
                params.raw_set("targetTypes", target_types)?;
                if let Some(d) = max_dist {
                    params.raw_set("maxDist", d)?;
                }
                if let Some(p) = priority {
                    params.raw_set("priority", p)?;
                }
            }
            Self::EngageTargetsInZone {
                point,
                zone_radius,
                target_types,
                priority,
            } => {
                root.raw_set("id", "EngageTargetsInZone")?;
                params.raw_set("point", point)?;
                params.raw_set("zoneRadius", zone_radius)?;
                params.raw_set("targetTypes", target_types)?;
                if let Some(p) = priority {
                    params.raw_set("priority", p)?;
                }
            }
            Self::EngageGroup {
                group,
                params: atp,
                priority,
            } => {
                root.raw_set("id", "EngageGroup")?;
                params.raw_set("groupId", group)?;
                atp.push_tbl(&params)?;
                if let Some(p) = priority {
                    params.raw_set("priority", p)?;
                }
            }
            Self::EngageUnit {
                unit,
                params: atp,
                priority,
            } => {
                root.raw_set("id", "EngageUnit")?;
                params.raw_set("unitId", unit)?;
                atp.push_tbl(&params)?;
                if let Some(p) = priority {
                    params.raw_set("priority", p)?;
                }
            }
            Self::AWACS => root.raw_set("id", "AWACS")?,
            Self::Tanker => root.raw_set("id", "Tanker")?,
            Self::EWR => root.raw_set("id", "EWR")?,
            Self::FACEngageGroup {
                group,
                params: fp,
                priority,
            } => {
                root.raw_set("id", "FAC_EngageGroup")?;
                params.raw_set("groupId", group)?;
                fp.push_tbl(&params)?;
                if let Some(p) = priority {
                    params.raw_set("priority", p)?;
                }
            }
            Self::FAC {
                params: fp,
                priority,
            } => {
                root.raw_set("id", "FAC")?;
                fp.push_tbl(&params)?;
                if let Some(p) = priority {
                    params.raw_set("priority", p)?;
                }
            }
            Self::Mission { airborne, route } => {
                root.raw_set("id", "Mission")?;
                params.raw_set("airborne", airborne)?;
                let points = lua.create_table()?;
                points.raw_set(
                    "points",
                    route
                        .into_iter()
                        .map(|m| m.into_lua(lua))
                        .collect::<LuaResult<Vec<Value>>>()?,
                )?;
                params.raw_set("route", points)?;
            }
            Self::ComboTask(tasks) => {
                root.raw_set("id", "ComboTask")?;
                let tbl = lua.create_table()?;
                for (i, task) in tasks.into_iter().enumerate() {
                    tbl.push(task)?;
                    tbl.raw_get::<_, LuaTable>(i + 1)?
                        .raw_set("number", i + 1)?;
                }
                params.raw_set("tasks", tbl)?;
            }
            Self::ControlledTask {
                mut task,
                condition,
                stop_condition,
            } => {
                let task = mem::replace(task.as_mut(), Task::AWACS);
                root.raw_set("id", "ControlledTask")?;
                params.raw_set("task", task)?;
                params.raw_set("condition", condition)?;
                params.raw_set("stopCondition", stop_condition)?;
            }
            Self::WrappedCommand(cmd) => {
                root.raw_set("id", "WrappedAction")?;
                let cmd = cmd.into_lua(lua)?;
                params.raw_set("action", cmd)?;
            }
            Self::WrappedOption(val) => {
                root.raw_set("id", "WrappedAction")?;
                let opt = lua.create_table()?;
                let optpar = lua.create_table()?;
                opt.raw_set("id", "Option")?;
                optpar.raw_set("name", val.tag())?;
                optpar.raw_set("value", val)?;
                opt.raw_set("params", optpar)?;
                params.raw_set("action", opt)?;
            }
        }
        root.raw_set("params", params)?;

        dbg!("{:?}", Value::Table(root.clone()).clone().into_lua(lua));

        Ok(Value::Table(root))
    }
}

simple_enum!(BeaconType, u16, [
    Null => 0,
    VOR => 1,
    DME => 2,
    VORDME => 3,
    TACAN => 4,
    VORTAC => 5,
    RSBN => 32,
    BroadcastStation => 1024,
    Homer => 8,
    AirportHomer => 4104,
    AirportHomerWithMarker => 4136,
    ILSFarHomer => 16408,
    ILSNearHomer => 16456,
    ILSLocalizer => 16640,
    ILSGlideslope => 16896,
    NauticalHomer => 32776
]);

simple_enum!(BeaconSystem, u8, [
    PAR10 => 1,
    RSBN5 => 2,
    TACAN => 3,
    TACANTanker => 4,
    ILSLocalizer => 5,
    ILSGlideslope => 6,
    BroadcastStation => 7
]);

#[derive(Debug, Clone)]
pub enum Command {
    Script(String),
    SetCallsign {
        callname: i64,
        number: u8,
    },
    SetFrequency {
        frequency: i64,
        modulation: Modulation,
        power: i64,
    },
    SetFrequencyForUnit {
        frequency: i64,
        modulation: Modulation,
        power: i64,
        unit: UnitId,
    },
    SwitchWaypoint {
        from_waypoint: i64,
        to_waypoint: i64,
    },
    StopRoute(bool),
    SwitchAction(i64),
    SetInvisible(bool),
    SetImmortal(bool),
    SetUnlimitedFuel(bool),
    ActivateBeacon {
        typ: BeaconType,
        system: BeaconSystem,
        name: Option<String>,
        callsign: String,
        frequency: i64,
    },
    DeactivateBeacon,
    ActivateICLS {
        channel: i64,
        unit: UnitId,
        name: Option<String>,
    },
    DeactivateICLS,
    EPLRS {
        enable: bool,
        group: Option<GroupId>,
    },
    Start,
    TransmitMessage {
        duration: Option<Time>,
        subtitle: Option<String>,
        looping: Option<bool>,
        file: String,
    },
    StopTransmission,
    Smoke(bool),
    ActivateLink4 {
        unit: UnitId,
        frequency: i64,
        name: Option<String>,
    },
    DeactivateLink4,
    ActivateACLS {
        unit: UnitId,
        name: Option<String>,
    },
    DeactivateACLS,
    LoadingShip {
        cargo: i64,
        unit: UnitId,
    },
}

impl<'lua> IntoLua<'lua> for Command {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let root = lua.create_table()?;
        let params = lua.create_table()?;
        match self {
            Self::Script(s) => {
                root.raw_set("id", "Script")?;
                params.raw_set("command", s)?;
            }
            Self::SetCallsign { callname, number } => {
                root.raw_set("id", "SetCallsign")?;
                params.raw_set("callname", callname)?;
                params.raw_set("number", number)?;
            }
            Self::SetFrequency {
                frequency,
                modulation,
                power,
            } => {
                root.raw_set("id", "SetFrequency")?;
                params.raw_set("frequency", frequency)?;
                params.raw_set("modulation", modulation)?;
                params.raw_set("power", power)?;
            }
            Self::SetFrequencyForUnit {
                frequency,
                modulation,
                power,
                unit,
            } => {
                root.raw_set("id", "SetFrequencyForUnit")?;
                params.raw_set("frequency", frequency)?;
                params.raw_set("modulation", modulation)?;
                params.raw_set("power", power)?;
                params.raw_set("unitId", unit)?;
            }
            Self::SwitchWaypoint {
                from_waypoint,
                to_waypoint,
            } => {
                root.raw_set("id", "SwitchWaypoint")?;
                params.raw_set("fromWaypointIndex", from_waypoint)?;
                params.raw_set("goToWaypointIndex", to_waypoint)?;
            }
            Self::StopRoute(stop) => {
                root.raw_set("id", "StopRoute")?;
                params.raw_set("value", stop)?;
            }
            Self::SwitchAction(action) => {
                root.raw_set("id", "SwitchAction")?;
                params.raw_set("actionIndex", action)?;
            }
            Self::SetInvisible(invisible) => {
                root.raw_set("id", "SetInvisible")?;
                params.raw_set("value", invisible)?;
            }
            Self::SetImmortal(immortal) => {
                root.raw_set("id", "SetImmortal")?;
                params.raw_set("value", immortal)?;
            }
            Self::SetUnlimitedFuel(unlimited_fuel) => {
                root.raw_set("id", "SetUnlimitedFuel")?;
                params.raw_set("value", unlimited_fuel)?;
            }
            Self::ActivateBeacon {
                typ,
                system,
                name,
                callsign,
                frequency,
            } => {
                root.raw_set("id", "ActivateBeacon")?;
                params.raw_set("type", typ)?;
                params.raw_set("system", system)?;
                params.raw_set("callsign", callsign)?;
                params.raw_set("frequency", frequency)?;
                if let Some(name) = name {
                    params.raw_set("name", name)?;
                }
            }
            Self::DeactivateBeacon => root.raw_set("id", "DeactivateBeacon")?,
            Self::ActivateICLS {
                channel,
                unit,
                name,
            } => {
                root.raw_set("id", "ActivateICLS")?;
                params.raw_set("type", 131584)?;
                params.raw_set("channel", channel)?;
                params.raw_set("unitId", unit)?;
                if let Some(name) = name {
                    params.raw_set("name", name)?;
                }
            }
            Self::DeactivateICLS => root.raw_set("id", "DeactivateICLS")?,
            Self::EPLRS { enable, group } => {
                root.raw_set("id", "EPLRS")?;
                params.raw_set("value", enable)?;
                if let Some(group) = group { 
                    params.raw_set("groupId", group)?;
                }
            }
            Self::Start => root.raw_set("id", "Start")?,
            Self::TransmitMessage {
                duration,
                subtitle,
                looping,
                file,
            } => {
                root.raw_set("id", "TransmitMessage")?;
                params.raw_set("file", file)?;
                if let Some(d) = duration {
                    params.raw_set("duration", d)?;
                }
                if let Some(s) = subtitle {
                    params.raw_set("subtitle", s)?;
                }
                if let Some(l) = looping {
                    params.raw_set("loop", l)?;
                }
            }
            Self::StopTransmission => root.raw_set("id", "stopTransmission")?,
            Self::Smoke(on) => {
                root.raw_set("id", "SMOKE_ON_OFF")?;
                params.raw_set("value", on)?
            }
            Self::ActivateLink4 {
                unit,
                frequency,
                name,
            } => {
                root.raw_set("id", "ActivateLink4")?;
                params.raw_set("unitId", unit)?;
                params.raw_set("frequency", frequency)?;
                if let Some(name) = name {
                    params.raw_set("name", name)?;
                }
            }
            Self::DeactivateLink4 => root.raw_set("id", "DeactivateLink4")?,
            Self::ActivateACLS { unit, name } => {
                root.raw_set("id", "ActivateACLS")?;
                params.raw_set("unitId", unit)?;
                if let Some(name) = name {
                    params.raw_set("name", name)?;
                }
            }
            Self::DeactivateACLS => root.raw_set("id", "DeactivateACLS")?,
            Self::LoadingShip { cargo, unit } => {
                root.raw_set("id", "LoadingShip")?;
                params.raw_set("cargo", cargo)?;
                params.raw_set("unitId", unit)?;
            }
        }
        root.raw_set("params", params)?;
        Ok(Value::Table(root))
    }
}

impl<'lua> FromLua<'lua> for Command {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let root: LuaTable = FromLua::from_lua(value, lua)?;
        let params: LuaTable = root.raw_get("params")?;
        match root.raw_get::<_, String>("id")?.as_str() {
            "Script" => Ok(Self::Script(params.raw_get("command")?)),
            "SetCallsign" => Ok(Self::SetCallsign {
                callname: params.raw_get("callname")?,
                number: params.raw_get("number")?,
            }),
            "SetFrequency" => Ok(Self::SetFrequency {
                frequency: params.raw_get("frequency")?,
                modulation: params.raw_get("modulation")?,
                power: params.raw_get("power")?,
            }),
            "SetFrequencyForUnit" => Ok(Self::SetFrequencyForUnit {
                frequency: params.raw_get("frequency")?,
                modulation: params.raw_get("modulation")?,
                power: params.raw_get("power")?,
                unit: params.raw_get("unitId")?,
            }),
            "SwitchWaypoint" => Ok(Self::SwitchWaypoint {
                from_waypoint: params.raw_get("fromWaypointIndex")?,
                to_waypoint: params.raw_get("goToWaypointIndex")?,
            }),
            "StopRoute" => Ok(Self::StopRoute(params.raw_get("value")?)),
            "SwitchAction" => Ok(Self::SwitchAction(params.raw_get("actionIndex")?)),
            "SetInvisible" => Ok(Self::SetInvisible(params.raw_get("value")?)),
            "SetImmortal" => Ok(Self::SetImmortal(params.raw_get("value")?)),
            "SetUnlimitedFuel" => Ok(Self::SetUnlimitedFuel(params.raw_get("value")?)),
            "ActivateBeacon" => Ok(Self::ActivateBeacon {
                typ: params.raw_get("type")?,
                system: params.raw_get("system")?,
                name: params.raw_get("name")?,
                callsign: params.raw_get("callsign")?,
                frequency: params.raw_get("frequency")?,
            }),
            "DeactivateBeacon" => Ok(Self::DeactivateBeacon),
            "DeactivateICLS" => Ok(Self::DeactivateACLS),
            "EPLRS" => Ok(Self::EPLRS {
                enable: params.raw_get("value")?,
                group: params.raw_get("groupId")?,
            }),
            "Start" => Ok(Self::Start),
            "TransmitMessage" => Ok(Self::TransmitMessage {
                duration: params.raw_get("duration")?,
                subtitle: params.raw_get("subtitle")?,
                looping: params.raw_get("loop")?,
                file: params.raw_get("file")?,
            }),
            "stopTransmission" => Ok(Self::StopTransmission),
            "ActivateLink4" => Ok(Self::ActivateLink4 {
                unit: params.raw_get("unitId")?,
                frequency: params.raw_get("frequency")?,
                name: params.raw_get("name")?,
            }),
            "DeactivateLink4" => Ok(Self::DeactivateLink4),
            "DeactivateACLS" => Ok(Self::DeactivateACLS),
            "ActivateACLS" => Ok(Self::ActivateACLS {
                unit: params.raw_get("unitId")?,
                name: params.raw_get("name")?,
            }),
            "LoadingShip" => Ok(Self::LoadingShip {
                cargo: params.raw_get("cargo")?,
                unit: params.raw_get("unitId")?,
            }),
            x => Err(err(&format_compact!("unknown {x}"))),
        }
    }
}

simple_enum!(AirRoe, u8, [
    OpenFire => 2,
    OpenFireWeaponFree => 1,
    ReturnFire => 3,
    WeaponFree => 0,
    WeaponHold => 4
]);

simple_enum!(AirReactionToThreat, u8, [
    NoReaction => 0,
    PassiveDefence => 1,
    EvadeFire => 2,
    BypassAndEscape => 3,
    AllowAbortMission => 4
]);

simple_enum!(AirEcmUsing, u8, [
    AlwaysUse => 3,
    NeverUse => 0,
    UseIfDetectedLockByRadar => 2,
    UseIfOnlyLockByRadar => 1
]);

simple_enum!(AirFlareUsing, u8, [
    AgainstFiredMissile => 1,
    Never => 0,
    WhenFlyingInSamWez => 2,
    WhenFlyingNearEnemies => 3
]);

string_enum!(VehicleFormation, u8, [
    Cone => "Cone",
    Diamond => "Diamond",
    EchelonLeft => "EchelonL",
    EchelonRight => "EchelonR",
    OffRoad => "Off Road",
    OnRoad => "On Road",
    Rank => "Rank",
    Vee => "Vee"
]);

simple_enum!(AirMissileAttack, u8, [
    HalfwayRmaxNez => 2,
    MaxRange => 0,
    NezRange => 1,
    RandomRange => 4,
    TargetThreatEst => 3
]);

simple_enum!(AirRadarUsing, u8, [
    ForAttackOnly => 1,
    ForContinuousSearch => 3,
    ForSearchIfRequired => 2,
    Never => 0
]);

#[derive(Debug, Clone, Serialize)]
pub enum AirOption<'lua> {
    EcmUsing(AirEcmUsing),
    FlareUsing(AirFlareUsing),
    ForcedAttack(bool),
    Formation(VehicleFormation),
    JettTanksIfEmpty(bool),
    MissileAttack(AirMissileAttack),
    OptionRadioUsageContact(Attributes<'lua>),
    OptionRadioUsageEngage(Attributes<'lua>),
    OptionRadioUsageKill(Attributes<'lua>),
    ProhibitAA(bool),
    ProhibitAB(bool),
    ProhibitAG(bool),
    ProhibitJett(bool),
    ProhibitWPPassReport(bool),
    RadarUsing(AirRadarUsing),
    ReactionOnThreat(AirReactionToThreat),
    Roe(AirRoe),
    RtbOnBingo(bool),
    RtbOnOutOfAmmo(bool),
    Silence(bool),
}

impl<'lua> IntoLua<'lua> for AirOption<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        match self {
            Self::EcmUsing(v) => v.into_lua(lua),
            Self::FlareUsing(v) => v.into_lua(lua),
            Self::ForcedAttack(v) => v.into_lua(lua),
            Self::Formation(v) => v.into_lua(lua),
            Self::JettTanksIfEmpty(v) => v.into_lua(lua),
            Self::MissileAttack(v) => v.into_lua(lua),
            Self::OptionRadioUsageContact(v) => v.into_lua(lua),
            Self::OptionRadioUsageEngage(v) => v.into_lua(lua),
            Self::OptionRadioUsageKill(v) => v.into_lua(lua),
            Self::ProhibitAA(v)
            | Self::ProhibitAB(v)
            | Self::ProhibitAG(v)
            | Self::ProhibitJett(v)
            | Self::ProhibitWPPassReport(v) => v.into_lua(lua),
            Self::RadarUsing(v) => v.into_lua(lua),
            Self::ReactionOnThreat(v) => v.into_lua(lua),
            Self::Roe(v) => v.into_lua(lua),
            Self::RtbOnBingo(v) | Self::RtbOnOutOfAmmo(v) | Self::Silence(v) => v.into_lua(lua),
        }
    }
}

impl<'lua> AirOption<'lua> {
    fn tag(&self) -> u8 {
        match self {
            Self::EcmUsing(_) => 13,
            Self::FlareUsing(_) => 4,
            Self::ForcedAttack(_) => 26,
            Self::Formation(_) => 5,
            Self::JettTanksIfEmpty(_) => 25,
            Self::MissileAttack(_) => 18,
            Self::OptionRadioUsageContact(_) => 21,
            Self::OptionRadioUsageEngage(_) => 22,
            Self::OptionRadioUsageKill(_) => 23,
            Self::ProhibitAA(_) => 14,
            Self::ProhibitAB(_) => 16,
            Self::ProhibitAG(_) => 17,
            Self::ProhibitJett(_) => 15,
            Self::ProhibitWPPassReport(_) => 19,
            Self::RadarUsing(_) => 3,
            Self::ReactionOnThreat(_) => 1,
            Self::Roe(_) => 0,
            Self::RtbOnBingo(_) => 6,
            Self::RtbOnOutOfAmmo(_) => 10,
            Self::Silence(_) => 7,
        }
    }

    fn from_tag_val(lua: &'lua Lua, tag: u8, val: Value<'lua>) -> LuaResult<Self> {
        let attr_or_none = |val: Value<'lua>| match val {
            Value::String(s) if s.to_string_lossy().as_ref().starts_with("none") => {
                Attributes::new(lua).map_err(|e| err(&format_compact!("{}", e)))
            }
            v => FromLua::from_lua(v, lua),
        };
        match tag {
            13 => Ok(Self::EcmUsing(FromLua::from_lua(val, lua)?)),
            4 => Ok(Self::FlareUsing(FromLua::from_lua(val, lua)?)),
            26 => Ok(Self::ForcedAttack(FromLua::from_lua(val, lua)?)),
            5 => Ok(Self::Formation(FromLua::from_lua(val, lua)?)),
            25 => Ok(Self::JettTanksIfEmpty(FromLua::from_lua(val, lua)?)),
            18 => Ok(Self::MissileAttack(FromLua::from_lua(val, lua)?)),
            21 => Ok(Self::OptionRadioUsageContact(attr_or_none(val)?)),
            22 => Ok(Self::OptionRadioUsageEngage(attr_or_none(val)?)),
            23 => Ok(Self::OptionRadioUsageKill(attr_or_none(val)?)),
            14 => Ok(Self::ProhibitAA(FromLua::from_lua(val, lua)?)),
            16 => Ok(Self::ProhibitAB(FromLua::from_lua(val, lua)?)),
            17 => Ok(Self::ProhibitAG(FromLua::from_lua(val, lua)?)),
            15 => Ok(Self::ProhibitJett(FromLua::from_lua(val, lua)?)),
            19 => Ok(Self::ProhibitWPPassReport(FromLua::from_lua(val, lua)?)),
            3 => Ok(Self::RadarUsing(FromLua::from_lua(val, lua)?)),
            1 => Ok(Self::ReactionOnThreat(FromLua::from_lua(val, lua)?)),
            0 => Ok(Self::Roe(FromLua::from_lua(val, lua)?)),
            6 => Ok(Self::RtbOnBingo(FromLua::from_lua(val, lua)?)),
            10 => Ok(Self::RtbOnOutOfAmmo(FromLua::from_lua(val, lua)?)),
            7 => Ok(Self::Silence(FromLua::from_lua(val, lua)?)),
            e => Err(err(&format_compact!("invalid AirOption {e}"))),
        }
    }
}

simple_enum!(AlarmState, u8, [
    Auto => 0,
    Green => 1,
    Red => 2
]);

simple_enum!(GroundRoe, u8, [
    OpenFire => 2,
    ReturnFire => 3,
    WeaponHold => 4
]);

#[derive(Debug, Clone, Serialize)]
pub enum GroundOption {
    AcEngagementRangeRestriction(u8),
    AlarmState(AlarmState),
    DisperseOnAttack(i64),
    EngageAirWeapons(bool),
    Formation(VehicleFormation),
    Roe(GroundRoe),
}

impl<'lua> IntoLua<'lua> for GroundOption {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        match self {
            Self::AcEngagementRangeRestriction(v) => v.into_lua(lua),
            Self::AlarmState(v) => v.into_lua(lua),
            Self::DisperseOnAttack(v) => v.into_lua(lua),
            Self::EngageAirWeapons(v) => v.into_lua(lua),
            Self::Formation(v) => v.into_lua(lua),
            Self::Roe(v) => v.into_lua(lua),
        }
    }
}

impl GroundOption {
    fn tag(&self) -> u8 {
        match self {
            Self::AcEngagementRangeRestriction(_) => 24,
            Self::AlarmState(_) => 9,
            Self::DisperseOnAttack(_) => 8,
            Self::EngageAirWeapons(_) => 20,
            Self::Formation(_) => 5,
            Self::Roe(_) => 0,
        }
    }

    fn from_tag_val(lua: &Lua, tag: u8, val: Value) -> LuaResult<Self> {
        match tag {
            0 => Ok(Self::Roe(FromLua::from_lua(val, lua)?)),
            5 => Ok(Self::Formation(FromLua::from_lua(val, lua)?)),
            8 => Ok(Self::DisperseOnAttack(FromLua::from_lua(val, lua)?)),
            9 => Ok(Self::AlarmState(FromLua::from_lua(val, lua)?)),
            20 => Ok(Self::EngageAirWeapons(FromLua::from_lua(val, lua)?)),
            24 => Ok(Self::AcEngagementRangeRestriction(FromLua::from_lua(
                val, lua,
            )?)),
            e => Err(err(&format_compact!("unknown GroundOption {e}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum NavalOption {
    Roe(GroundRoe),
}

impl<'lua> IntoLua<'lua> for NavalOption {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        match self {
            Self::Roe(v) => v.into_lua(lua),
        }
    }
}

impl NavalOption {
    fn tag(&self) -> u8 {
        match self {
            Self::Roe(_) => 0,
        }
    }

    fn from_tag_val(lua: &Lua, tag: u8, val: Value) -> LuaResult<Self> {
        match tag {
            0 => Ok(Self::Roe(FromLua::from_lua(val, lua)?)),
            e => Err(err(&format_compact!("unkown NavalOption {e}"))),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum AiOption<'lua> {
    Air(AirOption<'lua>),
    Ground(GroundOption),
    Naval(NavalOption),
}

impl<'lua> IntoLua<'lua> for AiOption<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        match self {
            Self::Air(v) => v.into_lua(lua),
            Self::Ground(v) => v.into_lua(lua),
            Self::Naval(v) => v.into_lua(lua),
        }
    }
}

impl<'lua> AiOption<'lua> {
    fn tag(&self) -> u8 {
        match self {
            Self::Air(v) => v.tag(),
            Self::Ground(v) => v.tag(),
            Self::Naval(v) => v.tag(),
        }
    }

    fn from_tag_val(lua: &'lua Lua, tag: u8, val: Value<'lua>) -> LuaResult<Self> {
        match AirOption::from_tag_val(lua, tag, val.clone()) {
            Ok(v) => Ok(Self::Air(v)),
            Err(ae) => match GroundOption::from_tag_val(lua, tag, val.clone()) {
                Ok(v) => Ok(Self::Ground(v)),
                Err(ge) => match NavalOption::from_tag_val(lua, tag, val) {
                    Ok(v) => Ok(Self::Naval(v)),
                    Err(ne) => Err(err(&format_compact!(
                        "unknown option, air: {ae:?} ground: {ge:?} naval: {ne:?}"
                    ))),
                },
            },
        }
    }
}

bitflags_enum!(Detection, u8, [
    Dlink => 32,
    Irst => 8,
    Optic => 2,
    Radar => 4,
    Rwr => 16,
    Visual => 1
]);

#[derive(Debug, Clone, Serialize)]
pub struct DetectedTargetInfo {
    pub is_detected: bool,
    pub is_visible: bool,
    pub last_time_seen: f64,
    pub type_known: bool,
    pub distance_known: bool,
    pub last_position: LuaVec3,
    pub last_velocity: LuaVec3,
}

impl<'lua> FromLuaMulti<'lua> for DetectedTargetInfo {
    fn from_lua_multi(mut values: LuaMultiValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let is_detected = FromLua::from_lua(
            values
                .pop_front()
                .ok_or_else(|| cvt_err("DetectedTargetInfo:is_detected"))?,
            lua,
        )?;
        let is_visible = FromLua::from_lua(
            values
                .pop_front()
                .ok_or_else(|| cvt_err("DetectedTargetInfo:is_visible"))?,
            lua,
        )?;
        let last_time_seen = FromLua::from_lua(
            values
                .pop_front()
                .ok_or_else(|| cvt_err("DetectedTargetInfo:last_time_seen"))?,
            lua,
        )?;
        let type_known = FromLua::from_lua(
            values
                .pop_front()
                .ok_or_else(|| cvt_err("DetectedTargetInfo:type_known"))?,
            lua,
        )?;
        let distance_known = FromLua::from_lua(
            values
                .pop_front()
                .ok_or_else(|| cvt_err("DetectedTargetInfo:distance_known"))?,
            lua,
        )?;
        let last_position = FromLua::from_lua(
            values
                .pop_front()
                .ok_or_else(|| cvt_err("DetectedTargetInfo:last_position"))?,
            lua,
        )?;
        let last_velocity = FromLua::from_lua(
            values
                .pop_front()
                .ok_or_else(|| cvt_err("DetectedTargetInfo:last_velocity"))?,
            lua,
        )?;
        Ok(Self {
            is_detected,
            is_visible,
            last_time_seen,
            type_known,
            distance_known,
            last_position,
            last_velocity,
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct DetectedTarget<'lua> {
    pub object: Object<'lua>,
    pub is_visible: bool,
    pub type_known: bool,
    pub distance_known: bool,
}

impl<'lua> FromLua<'lua> for DetectedTarget<'lua> {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("DetectedTarget", None, value).map_err(lua_err)?;
        Ok(Self {
            object: tbl.raw_get("object")?,
            is_visible: tbl.raw_get("visible")?,
            type_known: tbl.raw_get("type")?,
            distance_known: tbl.raw_get("distance")?,
        })
    }
}

string_enum!(AltitudeKind, u8, [
    Radio => "Radio",
    Baro => "Baro"
], [
    Radio => "RADIO",
    Baro => "BARO"
]);

wrapped_table!(Controller, Some("Controller"));

impl<'lua> Controller<'lua> {
    pub fn set_task(&self, task: Task) -> Result<()> {
        Ok(self.t.call_method("setTask", task)?)
    }

    pub fn reset_task(&self) -> Result<()> {
        Ok(self.t.call_method("resetTask", ())?)
    }

    pub fn push_task(&self, task: Task) -> Result<()> {
        Ok(self.t.call_method("pushTask", task)?)
    }

    pub fn pop_task(&self) -> Result<()> {
        Ok(self.t.call_method("popTask", ())?)
    }

    pub fn has_task(&self) -> Result<bool> {
        Ok(self.t.call_method("hasTask", ())?)
    }

    pub fn set_command(&self, command: Command) -> Result<()> {
        Ok(self.t.call_method("setCommand", command)?)
    }

    pub fn set_option(&self, option: AiOption<'lua>) -> Result<()> {
        Ok(self.t.call_method("setOption", (option.tag(), option))?)
    }

    pub fn set_on_off(&self, on: bool) -> Result<()> {
        Ok(self.t.call_method("setOnOff", on)?)
    }

    pub fn set_altitude(
        &self,
        altitude: f32,
        keep: bool,
        kind: Option<AltitudeKind>,
    ) -> Result<()> {
        Ok(match kind {
            None => self.t.call_method("setAltitude", (altitude, keep)),
            Some(kind) => self.t.call_method("setAltitude", (altitude, keep, kind)),
        }?)
    }

    pub fn set_speed(&self, speed: f32, keep: bool) -> Result<()> {
        Ok(self.t.call_method("setSpeed", (speed, keep))?)
    }

    pub fn know_target(&self, object: Object, typ: bool, distance: bool) -> Result<()> {
        Ok(self.t.call_method("knowTarget", (object, typ, distance))?)
    }

    pub fn is_target_detected(
        &self,
        object: Object,
        methods: BitFlags<Detection>,
    ) -> Result<DetectedTargetInfo> {
        let mut args = Variadic::new();
        args.push(object.into_lua(self.lua)?);
        for method in methods {
            args.push(method.into_lua(self.lua)?);
        }
        Ok(self.t.call_method("isTargetDetected", args)?)
    }

    pub fn get_detected_targets(
        &self,
        methods: BitFlags<Detection>,
    ) -> Result<Sequence<'lua, DetectedTarget<'lua>>> {
        let mut args = Variadic::new();
        for method in methods {
            args.push(method.into_lua(self.lua)?);
        }
        Ok(self.t.call_method("getDetectedTargets", args)?)
    }
}
