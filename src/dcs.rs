use compact_str::CompactString;
use mlua::{prelude::*, Value};
use netidx_derive::Pack;

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Pack)]
pub struct Time {
    pub epoch: u32,
    pub time: u32,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Pack)]
pub struct ObjectId {
    pub epoch: u32,
    pub id: u32
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Pack)]
pub struct Vec2 {
    x: f64,
    y: f64,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Pack)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Pack)]
pub struct Box3 {
    pub min: Vec3,
    pub max: Vec3,
}

#[derive(Debug, Clone, Pack)]
pub enum VolumeType {
    Segment,
    Box,
    Sphere,
    Pyramid
}

#[derive(Debug, Clone, Pack)]
pub enum BirthPlace {
    Air,
    Runway,
    Park,
    HeliportHot,
    HeliportCold
}

#[derive(Debug, Clone, Pack)]
pub enum UnitCategory {
    Airplane,
    Helicopter,
    GroundUnit,
    Ship,
    Structure
}

pub struct Unit {

}

#[derive(Debug, Clone, Pack)]
pub enum ObjectCategory {
    Unit,
    Weapon,
    Static,
    Base,
    Scenery,
    Cargo
}

#[derive(Debug, Clone, Pack)]
pub struct Object {
    id: ObjectId,
    type_name: CompactString,
    name: CompactString,
    collider: Box3,
    category: ObjectCategory,
}

pub struct Shot {
    pub time: Time,
    pub initiator: ObjectId,
    pub weapon: ObjectId,
}

pub struct Hit {
    pub time: Time,
    pub initiator: ObjectId,
    pub weapon: ObjectId,
    pub target: ObjectId,
}

/// This is a dcs event
#[derive(Debug, Clone, Pack)]
pub enum Event {
    Invalid,
    Shot(Shot),
    Hit,
    Takeoff,
    Land,
    Crash,
    Ejection,
    Refueling,
    Dead,
    PilotDead,
    BaseCaptured,
    MissionStart,
    MissionEnd,
    TookControl,
    RefuelingStop,
    Birth,
    HumanFailure,
    DetailedFailure,
    EngineStartup,
    EngineShutdown,
    PlayerEnterUnit,
    PlayerLeaveUnit,
    ShootingStart,
    ShootingEnd,
    MarkAdded,
    MarkChange,
    MarkRemoved,
    Kill,
    Score,
    UnitLost,
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