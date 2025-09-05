/*
Copyright 2024 Eric Stokes.

This file is part of bflib.

bflib is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your
option) any later version.

bflib is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
for more details.
*/

use anyhow::{anyhow, bail, Context, Result};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{coalition::Side, controller::AltType, net::Ucid, String};
use enumflags2::{bitflags, BitFlags};
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use indexmap::IndexMap;
use netidx::path::Path as NetIdxPath;
use regex::Regex;
use serde_derive::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    fmt,
    fs::{self, File},
    io,
    ops::{Deref, DerefMut},
    path::{Path, PathBuf},
};

mod example;

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord, Default)]
pub struct Vehicle(pub String);

impl fmt::Display for Vehicle {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl<'a> From<&'a str> for Vehicle {
    fn from(value: &'a str) -> Self {
        Self(value.into())
    }
}

impl From<String> for Vehicle {
    fn from(value: String) -> Self {
        Vehicle(value)
    }
}

impl Borrow<str> for Vehicle {
    fn borrow(&self) -> &str {
        &*self.0
    }
}

impl Vehicle {
    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Rule {
    Whitelist { allowed: FxHashMap<Ucid, String> },
    Blacklist { denied: FxHashMap<Ucid, String> },
    AlwaysAllowed,
    NeverAllowed,
}

impl Default for Rule {
    fn default() -> Self {
        Self::AlwaysAllowed
    }
}

impl Rule {
    pub fn check(&self, ucid: &Ucid) -> bool {
        match self {
            Self::Whitelist { allowed } => allowed.contains_key(ucid),
            Self::Blacklist { denied } => !denied.contains_key(&ucid),
            Self::AlwaysAllowed => true,
            Self::NeverAllowed => false,
        }
    }

    #[allow(dead_code)]
    pub fn blacklist(&mut self, ucid: Ucid, name: String) {
        match self {
            Self::Blacklist { denied } => {
                denied.insert(ucid, name);
            }
            Self::Whitelist { allowed } => {
                allowed.remove(&ucid);
            }
            Self::AlwaysAllowed => {
                let denied = FxHashMap::from_iter([(ucid, name)]);
                *self = Self::Blacklist { denied };
            }
            Self::NeverAllowed => (),
        }
    }

    #[allow(dead_code)]
    pub fn whitelist(&mut self, ucid: Ucid, name: String) {
        match self {
            Self::Blacklist { denied } => {
                denied.remove(&ucid);
            }
            Self::Whitelist { allowed } => {
                allowed.insert(ucid, name);
            }
            Self::NeverAllowed => {
                let allowed = FxHashMap::from_iter([(ucid, name)]);
                *self = Self::Whitelist { allowed };
            }
            Self::AlwaysAllowed => (),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[bitflags]
#[repr(u64)]
pub enum UnitTag {
    SAM,
    AAA,
    Armor,
    APC,
    Logistics,
    Infantry,
    EWR,
    Aircraft,
    Helicopter,
    LR,
    SR,
    MR,
    IRGuided,
    RadarGuided,
    OpticallyGuided,
    EngagesWeapons,
    Unguided,
    TrackRadar,
    SearchRadar,
    AuxRadarUnit,
    ControlUnit,
    Launcher,
    ATGM,
    Artillery,
    LightCannon,
    HeavyCannon,
    RPG,
    SmallArms,
    Unarmed,
    Invincible,
    Driveable,
    AWACS,
    Link16,
    Boat,
    NavalSpawnPoint,
}

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize,
)]
#[serde(from = "Vec<UnitTag>", into = "Vec<UnitTag>")]
pub struct UnitTags(pub BitFlags<UnitTag>);

impl fmt::Display for UnitTags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let len = self.0.len();
        write!(f, "[")?;
        for (i, tag) in self.0.iter().enumerate() {
            if i < len - 1 {
                write!(f, "{tag:?}, ")?
            } else {
                write!(f, "{tag:?}")?
            }
        }
        write!(f, "]")
    }
}

impl Deref for UnitTags {
    type Target = BitFlags<UnitTag>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for UnitTags {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl From<Vec<UnitTag>> for UnitTags {
    fn from(value: Vec<UnitTag>) -> Self {
        Self(value.into_iter().collect())
    }
}

impl From<BitFlags<UnitTag>> for UnitTags {
    fn from(value: BitFlags<UnitTag>) -> Self {
        Self(value)
    }
}

impl Into<Vec<UnitTag>> for UnitTags {
    fn into(self) -> Vec<UnitTag> {
        self.0.into_iter().collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum LifeType {
    Standard,
    Intercept,
    Logistics,
    Attack,
    Recon,
}

impl fmt::Display for LifeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Standard => "standard",
            Self::Intercept => "intercept",
            Self::Logistics => "logistics",
            Self::Attack => "attack",
            Self::Recon => "recon",
        };
        write!(f, "{s}")
    }
}

impl LifeType {
    pub fn up(&self) -> Option<LifeType> {
        match self {
            LifeType::Recon => Some(LifeType::Logistics),
            LifeType::Logistics => Some(LifeType::Intercept),
            LifeType::Intercept => Some(LifeType::Attack),
            LifeType::Attack => Some(LifeType::Standard),
            LifeType::Standard => None,
        }
    }

    #[allow(dead_code)]
    pub fn down(&self) -> Option<LifeType> {
        match self {
            LifeType::Recon => None,
            LifeType::Logistics => Some(LifeType::Recon),
            LifeType::Intercept => Some(LifeType::Logistics),
            LifeType::Attack => Some(LifeType::Attack),
            LifeType::Standard => Some(LifeType::Attack),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PersistTyp {
    /// The deployable persists until it is destroyed
    Forever,
    /// The deployable doesn't persist across restarts
    UntilRestart,
    /// The deployable persists for the specified number of
    /// real world seconds
    WallTime(f32),
    /// The deployable persists for the the specified number
    /// of server restart cycles
    Restarts(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LimitEnforceTyp {
    /// Handle the limit by removing the oldest instance of the deployable when
    /// a new one is unpacked. (lifo)
    DeleteOldest,
    /// Handle the limit by refusing to spawn new construction crates for
    /// the deployable
    DenyCrate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Crate {
    /// The name of the crate in the menu
    pub name: String,
    /// The weight of the crate in kg
    pub weight: u32,
    /// The number of crates of this type required to build the deployable
    pub required: u32,
    /// The type of unit in the associated deployable group that will inherit
    /// this crate's position when the deployable is spawned. This is only
    /// needed for multi unit groups with distinct parts.
    pub pos_unit: Option<String>,
    /// the maximum height in meters agl that the user can drop this crate from
    pub max_drop_height_agl: u32,
    /// the maximum speed in m/s that the user can be going when they drop this
    /// cargo
    pub max_drop_speed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployableObjective {
    pub pad_templates: Vec<String>,
    #[serde(default)]
    pub defenses_template: Option<String>,
    #[serde(default)]
    pub ammo_template: Option<String>,
    #[serde(default)]
    pub fuel_template: Option<String>,
    #[serde(default)]
    pub barracks_template: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct DeployableEwr {
    /// range for likely detection (Meters)
    pub range: u32,
    // CR estokes: Actual radar simulation ...
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeployableJtac {
    /// jtac detection and lasing range (Meters)
    pub range: u32,
    /// if true line of sight checks are not required, the jtac will
    /// see every unit in range regardless of terrain or cover
    #[serde(default)]
    pub nolos: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum DeployableKind {
    Group { template: String },
    Objective(DeployableObjective),
}

impl DeployableKind {
    pub fn is_group(&self) -> bool {
        match self {
            Self::Group { .. } => true,
            Self::Objective(_) => false,
        }
    }

    pub fn is_objective(&self) -> bool {
        match self {
            Self::Objective(_) => true,
            Self::Group { .. } => false,
        }
    }
}

fn default_deployable_kind() -> DeployableKind {
    DeployableKind::Group {
        template: "".into(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deployable {
    /// The full menu path of the deployable in the menu
    pub path: Vec<String>,
    /// The type of deployable
    #[serde(default = "default_deployable_kind")]
    pub kind: DeployableKind,
    /// How the deployable should persist across restarts
    pub persist: PersistTyp,
    /// How many instances are allowed at the same time
    pub limit: u32,
    /// How to deal with it when the max number of instances are deployed and
    /// a player wants to deploy a new instance
    pub limit_enforce: LimitEnforceTyp,
    /// What crates are required to build the deployable
    pub crates: Vec<Crate>,
    /// Can the damaged deployable be repaired, and if so, by which crate.
    pub repair_crate: Option<Crate>,
    /// How much does the damaged deployable cost to repair
    #[serde(default)]
    pub repair_cost: u32,
    /// How many points does this deployable cost (if any)
    #[serde(default)]
    pub cost: u32,
    /// Is this unit an early warning radar
    pub ewr: Option<DeployableEwr>,
    /// Is this unit a jtac
    pub jtac: Option<DeployableJtac>,
    #[serde(default)]
    #[serde(rename = "template")]
    pub deprecated_template: Option<String>,
    #[serde(default)]
    #[serde(rename = "logistics")]
    pub deprecated_logistics: Option<DeployableObjective>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Troop {
    /// The name of the squad in the menu
    pub name: String,
    /// The name of the template used to spawn the group
    pub template: String,
    /// How the troops will persist
    pub persist: PersistTyp,
    /// Can the troops capture objectives?
    pub can_capture: bool,
    /// How many simultaneous instances of the group are allowed
    pub limit: u32,
    /// How to deal with it when the max number of instances are deployed and the user
    /// wants to deploy an additional instance
    pub limit_enforce: LimitEnforceTyp,
    /// How much weight does the group add to the carrier unit
    pub weight: u32,
    /// How many points does this troop cost
    #[serde(default)]
    pub cost: u32,
    /// Can laser designate and scout
    pub jtac: Option<DeployableJtac>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CargoConfig {
    /// How many troop slots does this vehicle have
    pub troop_slots: u8,
    /// How many crate slots does this vehicle have
    pub crate_slots: u8,
    /// How many total troops and crates can this vehicle carry.
    /// e.g. if troop_slots is 1, crate_slots is 1, and total_slots is 1
    /// then the vehicle can carry either a troop or a crate but not both.
    pub total_slots: u16,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct WarehouseConfig {
    /// Logistics hub max supply stock as a multiple of the delivery amount
    pub hub_max: u32,
    /// Airbase max supply stock as a multiple of the delivery amount
    pub airbase_max: u32,
    /// Logistics tick in minutes. Supplies move automatically every tick
    pub tick: u32,
    /// How many logistics ticks does it take before supplies are delivered
    /// from outside
    pub ticks_per_delivery: u32,
    /// The supply transfer crate
    pub supply_transfer_crate: FxHashMap<Side, Crate>,
    /// The percentage of supply that is transfered by a transfer crate
    pub supply_transfer_size: u8,
    /// The name of the warehouse that is the source of supply every
    /// restart
    pub supply_source: FxHashMap<Side, String>,
    /// Airframes that do not play nice with the warehouse that are exempt from the
    /// warehouse check
    #[serde(default)]
    pub exempt_airframes: FxHashSet<String>,
}

impl WarehouseConfig {
    pub fn capacity(&self, hub: bool, qty: u32) -> u32 {
        if hub {
            qty * self.hub_max
        } else {
            qty * self.airbase_max
        }
    }
}

fn default_tk_window() -> u32 {
    24
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PointsCfg {
    /// Bonus issued to new players when they register
    pub new_player_join: u32,
    /// Points awarded for each air kill
    pub air_kill: u32,
    /// Base points awared for each ground kill
    pub ground_kill: u32,
    /// Bonus points awarded to heavy sam kills
    pub lr_sam_bonus: u32,
    /// Points awarded for repairing base logistics
    pub logistics_repair: u32,
    /// Points awarded for logistics transfers
    pub logistics_transfer: u32,
    /// Points awarded for base capture
    pub capture: u32,
    /// How many hours before previous team kills are forgotten for
    /// the purposes of computing the penalty of a team kill.
    #[serde(default = "default_tk_window")]
    pub tk_window: u32,
    /// If provisional is true then points earned in a sortie are only
    /// committed to the player's points balance when they land at a
    /// friendly objective
    #[serde(default)]
    pub provisional: bool,
    /// If strict is true then the player cannot take off when their
    /// loadout or airframe costs more points than they have. They
    /// will be deleted on takeoff, and no points or lives will be
    /// deducted. If struct is false then the player's points will go
    /// negative if they take off with an airframe/loadout that
    /// exceeds their current balance.
    #[serde(default)]
    pub strict: bool,
    /// How many points does it cost to slot in a given airframe. This
    /// need not cover all airframes on the server, and the default is 0.
    #[serde(default)]
    pub airframe_cost: FxHashMap<Vehicle, u32>,
    /// How many points does it cost to load a given weapon. This need
    /// not cover all weapons, and the default is zero.
    #[serde(default)]
    pub weapon_cost: FxHashMap<String, u32>,
    /// How many points do connected players automatically gain per
    /// time interval. This is a pair of the number of points with the
    /// interval in seconds. The number of points CAN be negative, the
    /// interval must be positive. The default is (0, 0)
    #[serde(default)]
    pub periodic_point_gain: (i32, u32),
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum AiPlaneKind {
    FixedWing,
    Helicopter,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiPlaneCfg {
    pub kind: AiPlaneKind,
    pub duration: Option<u32>,
    pub template: String,
    pub altitude: f64,
    pub altitude_typ: AltType,
    pub speed: f64,
    #[serde(default)]
    pub freq: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AwacsCfg {
    pub ewr: DeployableEwr,
    pub plane: AiPlaneCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BomberCfg {
    pub targets: u32,
    pub power: u32,
    // in meters radius around the target point
    pub accuracy: u32,
    pub plane: AiPlaneCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeployableCfg {
    pub name: String,
    pub plane: Option<AiPlaneCfg>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DroneCfg {
    pub jtac: DeployableJtac,
    pub plane: AiPlaneCfg,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NukeCfg {
    /// using a nuke reduces the cost of nukes for everyone by this
    /// factor. e.g. cost_scale: 4, with initial cost 1000. The first
    /// nuke would cost 1000 points. The next nuke would cost 250
    /// points. The next nuke would cost 62 points, and so on until a
    /// nuke costs 1 point at which point it stops scaling.
    pub cost_scale: u8,
    /// in Kilotons of TNT
    pub power: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MoveCfg {
    /// max distance for troop moves in meters per unit cost
    pub troop: u32,
    /// max distance for deployable moves in meters per unit cost
    pub deployable: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionKind {
    Tanker(AiPlaneCfg),
    Awacs(AwacsCfg),
    Bomber(BomberCfg),
    Fighters(AiPlaneCfg),
    Attackers(AiPlaneCfg),
    Drone(DroneCfg),
    Nuke(NukeCfg),
    FighersWaypoint,
    AttackersWaypoint,
    DroneWaypoint,
    TankerWaypoint,
    AwacsWaypoint,
    Paratrooper(DeployableCfg),
    Deployable(DeployableCfg),
    LogisticsRepair(AiPlaneCfg),
    LogisticsTransfer(AiPlaneCfg),
    Move(MoveCfg),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionGeoLimit {
    Unlimited,
    /// This action can only be run within `max` in meters of a friendly objective
    NearFriendlyObjective {
        max: u32,
    },
}

impl Default for ActionGeoLimit {
    fn default() -> Self {
        Self::Unlimited
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Action {
    pub kind: ActionKind,
    pub cost: u32,
    pub penalty: Option<u32>,
    pub limit: Option<u32>,
    /// defines where this action is allowed to run
    #[serde(default)]
    pub geo_limit: ActionGeoLimit,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(deny_unknown_fields)]
pub struct Rules {
    /// who can use actions
    pub actions: Rule,
    /// who gets the cargo menu
    pub cargo: Rule,
    /// who gets the troops menu
    pub troops: Rule,
    /// who gets the jtac menu
    pub jtac: Rule,
    /// who can access the jtac slots
    pub ca: Rule,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct NameFilter(Regex);

impl TryFrom<String> for NameFilter {
    type Error = anyhow::Error;

    fn try_from(value: String) -> Result<Self> {
        Ok(Self(Regex::new(&value)?))
    }
}

impl TryFrom<&str> for NameFilter {
    type Error = anyhow::Error;

    fn try_from(value: &str) -> Result<Self> {
        Ok(Self(Regex::new(value)?))
    }
}

impl Into<String> for NameFilter {
    fn into(self) -> String {
        self.0.as_str().into()
    }
}

impl NameFilter {
    /// Check if a name is allowed
    pub fn check(&self, name: &str) -> bool {
        self.0.is_match(name)
    }

    pub fn as_str(&self) -> &str {
        self.0.as_str()
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum VictoryCondition {
    /// Victory is triggered when the specified percentage of the map
    /// is owned by a given team, or is neutral. Every objective is
    /// considered equally in this calculation. Must be between 0 and 1
    MapOwned { fraction: f64 },
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AutoResetOnVictory {
    /// What victory condition triggers an automatic reset
    pub condition: VictoryCondition,
    /// How long, in seconds, must the condition hold before reset is
    /// tiggered
    pub delay: u32,
}

fn default_msgs_per_second() -> usize {
    5
}

fn default_cull_after() -> u32 {
    1800
}

fn default_lock_sides() -> bool {
    true
}

fn default_limited_lives() -> bool {
    true
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Cfg {
    #[serde(default)]
    pub netidx_base: Option<NetIdxPath>,
    /// if specified, automatically reset the server state and record
    /// a victory in the stats when the condition is met.
    #[serde(default)]
    pub auto_reset: Option<AutoResetOnVictory>,
    /// ucids in this list are able to run admin commands
    #[serde(default)]
    pub admins: FxHashMap<Ucid, String>,
    /// ucids in this list are banned
    #[serde(default)]
    pub banned: FxHashMap<Ucid, (Option<DateTime<Utc>>, String)>,
    /// who can do what
    #[serde(default)]
    pub rules: Rules,
    /// Because DCS. Reject names that don't match this regex
    #[serde(default)]
    pub name_filter: Option<NameFilter>,
    /// The maximum number of messages, including markup, we will push to dcs
    /// per second.
    #[serde(default = "default_msgs_per_second")]
    pub max_msgs_per_second: usize,
    /// shutdown after the specified number of hours, don't shutdown
    /// if None.
    #[serde(default)]
    pub shutdown: Option<u32>,
    /// how many points are various actions worth (if any)
    #[serde(default)]
    pub points: Option<PointsCfg>,
    /// do not attempt to get the target of any weapon in this list
    #[serde(default)]
    pub weapon_target_exclusions: FxHashSet<String>,
    /// how often a base will repair if it has full logistics (Seconds)
    pub repair_time: u32,
    /// The base repair crate
    pub repair_crate: FxHashMap<Side, Crate>,
    /// If the warehouse system is to be used then this should be specified,
    /// otherwise warehouses will be ignored and you should set them to unlimited
    pub warehouse: Option<WarehouseConfig>,
    /// how far must you fly from an objective to spawn deployables
    /// without penalty (Meters)
    pub logistics_exclusion: u32,
    /// an objective will cull it's units if there are no enemy units
    /// within this distance (Meters)
    pub unit_cull_distance: u32,
    /// an objective will cull it's units if there are no enemy ground units
    /// within this distance (Meters)
    pub ground_vehicle_cull_distance: u32,
    /// If a base has been inactive for this long then cull it's units (Seconds)
    #[serde(default = "default_cull_after")]
    pub cull_after: u32,
    /// how often to do more expensive checks such as unit culling and
    /// updating unit positions (Seconds)
    pub slow_timed_events_freq: u32,
    /// how close various kinds of enemy units can be (with LOS) for an objective
    /// to be considered threatened. Threatened objectives can't spawn deployables
    /// within the exclusion zone. (Meters)
    pub threatened_distance: FxHashMap<Vehicle, u32>,
    /// how long before threatened is removed if no enemy can be seen
    pub threatened_cooldown: u32,
    /// how far can a crate be from the player and still be
    /// loadable (Meters)
    pub crate_load_distance: u32,
    /// how far crates apart crates can be and still unpack (Meters)
    pub crate_spread: u32,
    /// how close must artillery be to participate in an artillery mission
    /// (meters).
    pub artillery_mission_range: u32,
    /// If true players will be locked to the side they initially
    /// choose for the duration of the round
    #[serde(default = "default_lock_sides")]
    pub lock_sides: bool,
    /// how many times a user may switch sides in a given round,
    /// or None for unlimited side switches
    #[serde(default)]
    pub side_switches: Option<u8>,
    /// How many crates a player may spawn at the same time
    #[serde(default)]
    pub max_crates: Option<u32>,
    /// the life types different vehicles use
    pub life_types: FxHashMap<Vehicle, LifeType>,
    /// the life reset configuration for each life type. A pair
    /// of number of lives per reset, and reset time in seconds.
    pub default_lives: FxHashMap<LifeType, (u8, u32)>,
    /// If true, lives will be limited according to the default_lives
    /// and life_types specification
    #[serde(default = "default_limited_lives")]
    pub limited_lives: bool,
    /// Available actions per side
    #[serde(default)]
    pub actions: FxHashMap<Side, IndexMap<String, Action, FxBuildHasher>>,
    /// vehicle cargo configuration
    #[serde(default)]
    pub cargo: FxHashMap<Vehicle, CargoConfig>,
    /// The name of the crate group for each side
    #[serde(default)]
    pub crate_template: FxHashMap<Side, String>,
    /// deployables configuration for each side
    #[serde(default)]
    pub deployables: FxHashMap<Side, Vec<Deployable>>,
    /// deployable troops configuration for each side
    pub troops: FxHashMap<Side, Vec<Troop>>,
    /// classification of ground units in the mission
    pub unit_classification: FxHashMap<Vehicle, UnitTags>,
    /// airborne jtacs
    #[serde(default)]
    pub airborne_jtacs: FxHashMap<Vehicle, DeployableJtac>,
    /// The jtac target priority list
    pub jtac_priority: Vec<UnitTags>,
    /// Objectives that can host fixed wing even though they aren't
    /// airbases. Used by actions to choose a spawn point. E.G. You
    /// want to make an airbase a logistics hub because it's close to
    /// a port.
    #[serde(default)]
    pub extra_fixed_wing_objectives: FxHashSet<String>,
}

impl Cfg {
    fn path(miz_state_path: &Path) -> PathBuf {
        let mut path = PathBuf::from(miz_state_path);
        let file_name = path
            .file_name()
            .map(|s| {
                let mut s = s.to_string_lossy().into_owned();
                s.push_str("_CFG");
                s
            })
            .unwrap_or_else(|| "CFG".into());
        path.set_file_name(file_name);
        path
    }

    pub fn load(miz_state_path: &Path) -> Result<Self> {
        let path = Self::path(miz_state_path);
        let file = loop {
            match File::open(&path) {
                Ok(f) => break f,
                Err(e) => match e.kind() {
                    io::ErrorKind::NotFound => {
                        let file = File::create(&path)
                            .map_err(|e| anyhow!("could not create default config {}", e))?;
                        serde_json::to_writer_pretty(file, &Cfg::default())
                            .map_err(|e| anyhow!("could not write default config {}", e))?;
                    }
                    e => {
                        return Err(anyhow!("error opening config file {:?}", e));
                    }
                },
            }
        };
        let mut cfg: Self = serde_json::from_reader(file)
            .map_err(|e| anyhow!("failed to decode cfg file {:?}, {:?}", path, e))?;
        for (_, actions) in &mut cfg.actions {
            actions.sort_by(|name0, _, name1, _| name0.cmp(name1));
        }
        // translate deployables to the new format
        let mut has_deprecated = false;
        for (_, deps) in cfg.deployables.iter_mut() {
            for dep in deps.iter_mut() {
                if let Some(mut parts) = dep.deprecated_logistics.take() {
                    parts.defenses_template = dep.deprecated_template.take();
                    dep.kind = DeployableKind::Objective(parts);
                    has_deprecated = true;
                } else if let Some(template) = dep.deprecated_template.take() {
                    dep.kind = DeployableKind::Group { template };
                    has_deprecated = true;
                }
            }
        }
        if has_deprecated {
            fs::write(path, serde_json::to_string_pretty(&cfg)?)?
        }
        Ok(cfg)
    }

    pub fn save(&self, miz_state_path: &Path) -> Result<()> {
        let mut path = Self::path(miz_state_path);
        path.set_extension("bak");
        let fd = File::options()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&path)
            .with_context(|| format_compact!("opening {:?}", path))?;
        serde_json::to_writer_pretty(fd, self).context("serializing cfg")?;
        fs::rename(&path, Self::path(miz_state_path)).context("moving new file into place")?;
        Ok(())
    }

    pub fn check_vehicle_has_threat_distance(&self, vehicle: &Vehicle) -> Result<()> {
        match self.threatened_distance.get(vehicle) {
            Some(_) => (),
            None => bail!(
                "vehicle {:?} doesn't have a configured theatened distance",
                vehicle
            ),
        }
        Ok(())
    }

    pub fn check_vehicle_has_life_type(&self, vehicle: &Vehicle) -> Result<()> {
        match self.life_types.get(vehicle) {
            None => bail!("vehicle {:?} doesn't have a configured life type", vehicle),
            Some(typ) => match self.default_lives.get(&typ) {
                Some((n, f)) if *n > 0 && *f > 0 => (),
                None => bail!("vehicle {:?} has no configured life type", vehicle),
                Some((n, f)) => {
                    bail!(
                        "vehicle {:?} life type {:?} has no configured lives ({n}) or negative reset time ({f})",
                        vehicle, typ
                    )
                }
            },
        }
        Ok(())
    }
}
