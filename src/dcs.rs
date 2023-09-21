use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::{Deref, DerefMut};

fn cvt_err(to: &'static str) -> LuaError {
    LuaError::FromLuaConversionError {
        from: "value",
        to,
        message: None,
    }
}

fn as_tbl_ref<'a: 'lua, 'lua>(
    to: &'static str,
    value: &'a Value<'lua>,
) -> LuaResult<&'a mlua::Table<'lua>> {
    value
        .as_table()
        .ok_or_else(|| cvt_err(to))
}

fn as_tbl<'lua>(
    to: &'static str,
    objtyp: Option<&'static str>,
    value: Value<'lua>,
) -> LuaResult<mlua::Table<'lua>> {
    match value {
        Value::Table(tbl) => match objtyp {
            None => Ok(tbl),
            Some(typ) => {
                let actual_typ: String = tbl.raw_get("className_")?;
                if actual_typ.as_str() == typ {
                    Ok(tbl)
                } else {
                    Err(LuaError::FromLuaConversionError {
                        from: "table",
                        to: typ,
                        message: Some(format!(
                            "object expected to have type {}, actually type {}",
                            typ, &*actual_typ
                        )),
                    })
                }
            }
        },
        _ => Err(cvt_err(to)),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vec2 {
    x: f64,
    y: f64,
}

impl<'lua> FromLua<'lua> for Vec2 {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Vec2", None, value)?;
        Ok(Self {
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl<'lua> FromLua<'lua> for Vec3 {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Vec3", None, value)?;
        Ok(Self {
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
            z: tbl.raw_get("z")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Position3 {
    p: Vec3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
}

impl<'lua> FromLua<'lua> for Position3 {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Position3", None, value)?;
        Ok(Self {
            p: tbl.raw_get("p")?,
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
            z: tbl.raw_get("z")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Box3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl<'lua> FromLua<'lua> for Box3 {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Box3", None, value)?;
        Ok(Self {
            min: tbl.raw_get("min")?,
            max: tbl.raw_get("max")?,
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
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

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize)]
pub struct Time(f32);

impl<'lua> FromLua<'lua> for Time {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(value.as_f32().ok_or_else(|| {
            cvt_err("Time")
        })?))
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum ObjectCategory {
    Void,
    Unit,
    Weapon,
    Static,
    Base,
    Scenery,
    Cargo,
}

pub trait ObjectExt {
    fn destroy(&self) -> LuaResult<()>;
    fn get_category(&self) -> LuaResult<ObjectCategory>;
    fn get_name(&self) -> LuaResult<String>;
    fn get_point(&self) -> LuaResult<Vec3>;
    fn get_position(&self) -> LuaResult<Position3>;
    fn get_velocity(&self) -> LuaResult<Vec3>;
    fn in_air(&self) -> LuaResult<bool>;
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Object<'lua>(mlua::Table<'lua>);

impl<'lua> FromLua<'lua> for Object<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(as_tbl("Object", Some("Object"), value)?))
    }
}

impl<'lua> IntoLua<'lua> for Object<'lua> {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Table(self.0))
    }
}

impl<'lua> ObjectExt for Object<'lua> {
    fn destroy(&self) -> LuaResult<()> {
        self.0.call_method("destroy", ())
    }

    fn get_category(&self) -> LuaResult<ObjectCategory> {
        Ok(match self.0.call_method("getCategory", ())? {
            0 => ObjectCategory::Void,
            1 => ObjectCategory::Unit,
            2 => ObjectCategory::Weapon,
            3 => ObjectCategory::Static,
            4 => ObjectCategory::Base,
            5 => ObjectCategory::Scenery,
            6 => ObjectCategory::Cargo,
            _ => return Err(cvt_err("ObjectCategory")),
        })
    }

    fn get_name(&self) -> LuaResult<String> {
        self.0.call_method("getName", ())
    }

    fn get_point(&self) -> LuaResult<Vec3> {
        self.0.call_method("getPoint", ())
    }

    fn get_position(&self) -> LuaResult<Position3> {
        self.0.call_method("getPosition", ())
    }

    fn get_velocity(&self) -> LuaResult<Vec3> {
        self.0.call_method("getPosition", ())
    }

    fn in_air(&self) -> LuaResult<bool> {
        self.0.call_method("inAir", ())
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum AltitudeKind {
    Radio,
    Baro,
}

impl<'lua> IntoLua<'lua> for AltitudeKind {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::String(lua.create_string(match self {
            Self::Radio => "RADIO",
            Self::Baro => "BARO",
        })?))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Controller<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Controller<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Controller", Some("Controller"), value)?,
            lua,
        })
    }
}

impl<'lua> Controller<'lua> {
    pub fn has_task(&self) -> LuaResult<bool> {
        self.t.call_method("hasTask", ())
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

#[derive(Debug, Clone, Serialize)]
pub enum GroupCategory {
    Airplane,
    Ground,
    Helicopter,
    Ship,
    Train,
}

#[derive(Debug, Clone, Serialize)]
pub enum Coalition {
    Neutral,
    Red,
    Blue,
    Contested
}

#[derive(Debug, Clone, Serialize)]
pub struct Group<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Group<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Group", Some("Group"), value)?,
            lua,
        })
    }
}

impl<'lua> Group<'lua> {
    pub fn get_by_name(lua: &'lua Lua, name: &str) -> LuaResult<Group<'lua>> {
        let globals = lua.globals();
        let unit = as_tbl("Group", Some("Group"), globals.raw_get("Group")?)?;
        Self::from_lua(unit.call_method("getByName", name)?, lua)
    }

    pub fn destroy(&self) -> LuaResult<()> {
        self.t.call_method("destroy", ())
    }

    pub fn activate(&self) -> LuaResult<()> {
        self.t.call_method("activate", ())
    }

    pub fn get_category(&self) -> LuaResult<GroupCategory> {
        Ok(match self.t.call_method("getCategory", ())? {
            0 => GroupCategory::Airplane,
            1 => GroupCategory::Ground,
            2 => GroupCategory::Helicopter,
            3 => GroupCategory::Ship,
            4 => GroupCategory::Train,
            _ => return Err(cvt_err("GroupCategory"))
        })
    }

    pub fn get_coalition(&self) -> LuaResult<Coalition> {
        Ok(match self.t.call_method("getCoalition", ())? {
            0 => Coalition::Neutral,
            1 => Coalition::Red,
            2 => Coalition::Blue,
            3 => Coalition::Contested,
            _ => return Err(cvt_err("Coalition"))
        })
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum UnitCategory {
    Airplane,
    Helicopter,
    GroundUnit,
    Ship,
    Structure,
}

#[derive(Debug, Clone, Serialize)]
pub struct Unit<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Unit<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Unit", Some("Unit"), value)?,
            lua,
        })
    }
}

impl<'lua> Unit<'lua> {
    pub fn get_by_name(lua: &'lua Lua, name: &str) -> LuaResult<Unit<'lua>> {
        let globals = lua.globals();
        let unit = as_tbl("Unit", Some("Unit"), globals.raw_get("Unit")?)?;
        Self::from_lua(unit.call_method("getByName", name)?, lua)
    }

    pub fn is_active(&self) -> LuaResult<bool> {
        self.t.call_method("isActive", ())
    }

    pub fn get_player_name(&self) -> LuaResult<String> {
        self.t.call_method("getPlayerName", ())
    }

    pub fn get_id(&self) -> LuaResult<u32> {
        self.t.call_method("getID", ())
    }

    pub fn get_number(&self) -> LuaResult<u32> {
        self.t.call_method("getNumber", ())
    }

    pub fn get_object_id(&self) -> LuaResult<u32> {
        self.t.call_method("getObjectID", ())
    }

    pub fn get_controller(&self) -> LuaResult<Controller<'lua>> {
        Controller::from_lua(self.t.call_method("getController", ())?, self.lua)
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum VolumeType {
    Segment,
    Box,
    Sphere,
    Pyramid,
}

#[derive(Debug, Clone, Serialize)]
pub enum BirthPlace {
    Air,
    Runway,
    Park,
    HeliportHot,
    HeliportCold,
}

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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

#[derive(Debug, Clone, Serialize)]
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
            subplace: tbl.raw_get("subPlace")?,
        })
    }
}

/// This is a dcs event
#[derive(Debug, Clone, Serialize)]
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
    Unknown(u32),
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
