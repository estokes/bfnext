use super::{as_tbl, attribute::Attributes, cvt_err, object::Object, LuaVec3, String};
use crate::{bitflags_enum, simple_enum, string_enum, wrapped_table, Sequence, lua_err};
use anyhow::Result;
use enumflags2::{bitflags, BitFlags};
use mlua::{prelude::*, Value, Variadic};
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;

wrapped_table!(Task, None);
wrapped_table!(Command, None);

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
    ) -> Result<Sequence<DetectedTarget>> {
        let mut args = Variadic::new();
        for method in methods {
            args.push(method.into_lua(self.lua)?);
        }
        Ok(self.t.call_method("getDetectedTargets", args)?)
    }
}
