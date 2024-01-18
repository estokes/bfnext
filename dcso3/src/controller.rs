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
    lua_err, simple_enum,
    static_object::StaticObjectId,
    string_enum,
    trigger::Modulation,
    wrapped_table, LuaVec2, Sequence, Time,
};
use anyhow::Result;
use enumflags2::{bitflags, BitFlags};
use mlua::{prelude::*, Value, Variadic};
use serde::ser::SerializeTupleVariant;
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;

string_enum!(WaypointType, u8, [
    Takeoff => "TAKEOFF",
    TakeoffParking => "TAKEOFF_PARKING",
    TakeoffParkingHot => "TAKEOFF_PARKING_HOT",
    TurningPoint => "TURNING_POINT",
    Land => "LAND"
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
    RraceTrack => "RACE_TRACK",
    Circle => "CIRCLE"
]);

string_enum!(TurnMethod, u8, [
    FlyOverPoint => "FLY_OVER_POINT",
    FinPoint => "FIN_POINT"
]);

string_enum!(Designation, u8, [
    No => "NO",
    WP => "WP",
    IrPointer => "IR_POINTER",
    Laser => "LASER",
    Auto => "AUTO"
]);

simple_enum!(AltType, u8, [
    MSL => 0,
    AGL => 1
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
    weapon_type: Option<u64>, // weapon flag(s)
    expend: Option<WeaponExpend>,
    direction: Option<f64>, // in radians
    altitude: Option<f64>,
    attack_qty: Option<i64>,
    group_attack: Option<bool>,
}

impl AttackParams {
    fn push_tbl(&self, tbl: &LuaTable) -> LuaResult<()> {
        if let Some(wt) = self.weapon_type {
            tbl.raw_set("weaponType", wt)?
        }
        if let Some(exp) = &self.expend {
            tbl.raw_set("expend", exp.clone())?
        }
        if let Some(dir) = self.direction {
            tbl.raw_set("directionEnabled", true)?;
            tbl.raw_set("direction", dir)?
        }
        if let Some(alt) = self.altitude {
            tbl.raw_set("altitudeEnabled", true)?;
            tbl.raw_set("altitude", alt)?;
        }
        if let Some(qty) = self.attack_qty {
            tbl.raw_set("attackQtyLimit", true)?;
            tbl.raw_set("attackQty", qty)?;
        }
        if let Some(grp) = self.group_attack {
            tbl.raw_set("groupAttack", grp)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct FollowParams {
    group: GroupId,
    pos: LuaVec3,
    last_waypoint_index: Option<i64>,
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
    weapon_type: Option<u64>, // weapon flag(s),
    designation: Option<Designation>,
    datalink: Option<bool>,
    frequency: Option<f64>,
    modulation: Option<Modulation>,
    callname: Option<FACCallsign>,
    number: Option<u8>,
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
pub struct MissionPoint {
    typ: WaypointType,
    airdrome_id: Option<AirbaseId>,
    time_re_fu_ar: Option<i64>,
    helipad: Option<AirbaseId>,
    link_unit: Option<UnitId>,
    action: Option<TurnMethod>,
    pos: LuaVec2,
    alt: f64,
    alt_typ: Option<AltType>,
    speed: f64,
    speed_locked: Option<bool>,
    eta: Option<Time>,
    eta_locked: Option<bool>,
    name: String,
    task: Box<Task>,
}

impl<'lua> IntoLua<'lua> for MissionPoint {
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
pub enum Task {
    AttackGroup {
        group: GroupId,
        params: AttackParams,
    },
    AttackUnit {
        unit: UnitId,
        params: AttackParams,
    },
    Bombing {
        point: LuaVec2,
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
        route: Vec<MissionPoint>,
    },
    ComboTask(Vec<Task>),
}

impl<'lua> IntoLua<'lua> for Task {
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
            Self::Bombing { point, params: atp } => {
                root.raw_set("id", "Bombing")?;
                params.raw_set("point", point)?;
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
                for task in tasks {
                    params.push(task)?;
                }
            }
        }
        root.raw_set("params", params)?;
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
        group: GroupId,
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
                params.raw_set("groupId", group)?;
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
            Self::ActivateLink4 { unit, frequency, name } => {
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
], [
    Cone => "CONE",
    Diamond => "DIAMOND",
    EchelonLeft => "ECHELON_LEFT",
    EchelonRight => "ECHELON_RIGHT",
    OffRoad => "OFF_ROAD",
    OnRoad => "ON_ROAD",
    Rank => "RANK",
    Vee => "VEE"
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
