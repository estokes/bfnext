use super::{as_tbl, attribute::Attributes, cvt_err, object::Object, String};
use crate::{simple_enum, string_enum, wrapped_table};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
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

string_enum!(AltitudeKind, u8, [
    Radio => "Radio",
    Baro => "Baro"
], [
    Radio => "RADIO",
    Baro => "BARO"
]);

wrapped_table!(Controller, Some("Controller"));

impl<'lua> Controller<'lua> {
    pub fn set_task(&self, task: Task) -> LuaResult<()> {
        self.t.call_method("setTask", task)
    }

    pub fn reset_task(&self) -> LuaResult<()> {
        self.t.call_method("resetTask", ())
    }

    pub fn push_task(&self, task: Task) -> LuaResult<()> {
        self.t.call_method("pushTask", task)
    }

    pub fn pop_task(&self) -> LuaResult<()> {
        self.t.call_method("popTask", ())
    }

    pub fn has_task(&self) -> LuaResult<bool> {
        self.t.call_method("hasTask", ())
    }

    pub fn set_command(&self, command: Command) -> LuaResult<()> {
        self.t.call_method("setCommand", command)
    }

    pub fn set_option(&self, option: AirOption<'lua>) -> LuaResult<()> {
        self.t.call_method("setOption", (option.tag(),))
    }

    pub fn set_on_off(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("setOnOff", on)
    }

    pub fn set_altitude(
        &self,
        altitude: f32,
        keep: bool,
        kind: Option<AltitudeKind>,
    ) -> LuaResult<()> {
        match kind {
            None => self.t.call_method("setAltitude", (altitude, keep)),
            Some(kind) => self.t.call_method("setAltitude", (altitude, keep, kind)),
        }
    }

    pub fn set_speed(&self, speed: f32, keep: bool) -> LuaResult<()> {
        self.t.call_method("setSpeed", (speed, keep))
    }

    pub fn know_target(&self, object: Object, typ: bool, distance: bool) -> LuaResult<()> {
        self.t.call_function("knowTarget", (object, typ, distance))
    }
}
