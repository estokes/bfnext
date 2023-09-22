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
    value.as_table().ok_or_else(|| cvt_err(to))
}

fn check_implements(mut tbl: mlua::Table, class: &str) -> bool {
    loop {
        match tbl.raw_get::<_, String>("className_") {
            Err(_) => break false,
            Ok(s) if s.as_str() == class => break true,
            Ok(_) => match tbl.raw_get::<_, mlua::Table>("parentClass_") {
                Err(_) => break false,
                Ok(t) => {
                    tbl = t;
                }
            },
        }
    }
}

fn as_tbl<'lua>(
    to: &'static str,
    objtyp: Option<&'static str>,
    value: Value<'lua>,
) -> LuaResult<mlua::Table<'lua>> {
    match value {
        Value::Table(tbl) => match objtyp {
            None => Ok(tbl),
            Some(typ) => match tbl.get_metatable() {
                None => Err(LuaError::FromLuaConversionError {
                    from: "table",
                    to: typ,
                    message: Some(format!("table is not an object")),
                }),
                Some(meta) => {
                    if check_implements(meta, typ) {
                        Ok(tbl)
                    } else {
                        Err(LuaError::FromLuaConversionError {
                            from: "table",
                            to: typ,
                            message: Some(format!("object or super expected to have type {}", typ)),
                        })
                    }
                }
            },
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
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
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
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
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
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
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
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
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
        Ok(Self(value.as_f32().ok_or_else(|| cvt_err("Time"))?))
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

#[derive(Debug, Clone, Serialize)]
pub struct Object<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Object<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Object", Some("Object"), value)?,
            lua,
        })
    }
}

impl<'lua> IntoLua<'lua> for Object<'lua> {
    fn into_lua(self, _: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Table(self.t))
    }
}

impl<'lua> Object<'lua> {
    pub fn destroy(&self) -> LuaResult<()> {
        self.t.call_method("destroy", ())
    }

    pub fn get_category(&self) -> LuaResult<ObjectCategory> {
        Ok(match self.t.call_method("getCategory", ())? {
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

    pub fn get_desc(&self) -> LuaResult<mlua::Table<'lua>> {
        self.t.call_method("getDesc", ())
    }

    pub fn get_name(&self) -> LuaResult<String> {
        self.t.call_method("getName", ())
    }

    pub fn get_point(&self) -> LuaResult<Vec3> {
        self.t.call_method("getPoint", ())
    }

    pub fn get_position(&self) -> LuaResult<Position3> {
        self.t.call_method("getPosition", ())
    }

    pub fn get_velocity(&self) -> LuaResult<Vec3> {
        self.t.call_method("getPosition", ())
    }

    pub fn in_air(&self) -> LuaResult<bool> {
        self.t.call_method("inAir", ())
    }

    pub fn as_unit(&self) -> LuaResult<Unit> {
        Unit::from_lua(Value::Table(self.t.clone()), self.lua)
    }

    pub fn as_weapon(&self) -> LuaResult<Weapon> {
        Weapon::from_lua(Value::Table(self.t.clone()), self.lua)
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
    _lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Controller<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Controller", Some("Controller"), value)?,
            _lua: lua,
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
    Contested,
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
            _ => return Err(cvt_err("GroupCategory")),
        })
    }

    pub fn get_coalition(&self) -> LuaResult<Coalition> {
        Ok(match self.t.call_method("getCoalition", ())? {
            0 => Coalition::Neutral,
            1 => Coalition::Red,
            2 => Coalition::Blue,
            3 => Coalition::Contested,
            _ => return Err(cvt_err("Coalition")),
        })
    }

    pub fn get_name(&self) -> LuaResult<String> {
        self.t.call_method("getName", ())
    }

    pub fn get_id(&self) -> LuaResult<u32> {
        self.t.call_method("getID", ())
    }

    pub fn get_size(&self) -> LuaResult<u32> {
        self.t.call_method("getSize", ())
    }

    pub fn get_initial_size(&self) -> LuaResult<u32> {
        self.t.call_method("getInitialSize", ())
    }

    pub fn get_unit(&self, index: usize) -> LuaResult<Unit> {
        Unit::from_lua(self.t.call_method("getUnit", index)?, self.lua)
    }

    pub fn get_units(&self) -> LuaResult<impl Iterator<Item = LuaResult<Unit>>> {
        Ok(as_tbl("Units", None, self.t.call_method("getUnits", ())?)?.sequence_values())
    }

    pub fn get_controller(&self) -> LuaResult<Controller> {
        Ok(Controller::from_lua(
            self.t.call_method("getController", ())?,
            self.lua,
        )?)
    }

    pub fn enable_emission(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("enableEmission", on)
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

    pub fn as_object(&self) -> LuaResult<Object<'lua>> {
        Object::from_lua(Value::Table(self.t.clone()), self.lua)
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

    pub fn get_group(&self) -> LuaResult<Group<'lua>> {
        Group::from_lua(self.t.call_method("getGroup", ())?, self.lua)
    }

    pub fn get_callsign(&self) -> LuaResult<String> {
        self.t.call_method("getCallsign", ())
    }

    pub fn get_life(&self) -> LuaResult<i32> {
        self.t.call_method("getLife", ())
    }

    pub fn get_life0(&self) -> LuaResult<i32> {
        self.t.call_method("getLife0", ())
    }

    pub fn get_fuel(&self) -> LuaResult<f32> {
        self.t.call_method("getFuel", ())
    }

    pub fn enable_emission(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("enableEmission", on)
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Weapon<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Weapon<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Weapon", Some("Weapon"), value)?,
            lua,
        })
    }
}

impl<'lua> Weapon<'lua> {
    pub fn as_object(&self) -> LuaResult<Object<'lua>> {
        Object::from_lua(Value::Table(self.t.clone()), self.lua)
    }

    pub fn get_launcher(&self) -> LuaResult<Unit<'lua>> {
        Unit::from_lua(self.t.call_method("getLauncher", ())?, self.lua)
    }

    pub fn get_target(&self) -> LuaResult<Option<Object<'lua>>> {
        match self.t.call_method("getTarget", ())? {
            Value::Nil => Ok(None),
            v => Ok(Some(Object::from_lua(v, self.lua)?)),
        }
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
    Birth(AtPlace<'lua>),
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
            _ => return Err(cvt_err("Event")),
        };
        Ok(ev)
    }
}
