use dcso3::{coalition::Side, err, String};
use fxhash::FxHashMap;
use log::error;
use mlua::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::{
    borrow::Borrow,
    fmt,
    fs::File,
    io,
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vehicle(String);

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
    /// the maximum speed in km/h that the user can be going when they drop this
    /// cargo
    pub max_drop_speed: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Deployable {
    /// The full menu path of the deployable in the menu
    pub path: Vec<String>,
    /// The template used to spawn the deployable
    pub template: String,
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
    /// Does this deployable provide logistics services, if so, what is it's
    /// exclusion zone size
    pub logistics: Option<u32>,
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
    /// Can laser designate and scout
    pub jtac: bool,
    /// How many simultaneous instances of the group are allowed
    pub limit: u32,
    /// How to deal with it when the max number of instances are deployed and the user
    /// wants to deploy an additional instance
    pub limit_enforce: LimitEnforceTyp,
    /// How much weight does the group add to the carrier unit
    pub weight: u32,
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
pub struct Cfg {
    /// how often a base will repair if it has full logistics (Seconds)
    pub repair_time: u32,
    /// The base repair crate
    pub repair_crate: FxHashMap<Side, Crate>,
    /// how far must you fly from an objective to spawn deployables
    /// without penalty (Meters)
    pub logistics_exclusion: u32,
    /// how long you have to wait to unpack when inside the logistics exclusion zone of
    /// an objective. None, you can't ever unpack there. (Seconds)
    pub inside_logistics_unpack_delay: Option<u32>,
    /// an objective will cull it's units if there are no enemy units
    /// within this distance (Meters)
    pub unit_cull_distance: u32,
    /// how often to check whether we need to cull or respawn units (Seconds)
    pub unit_cull_freq: u32,
    /// how far can a crate be from the player and still be
    /// loadable (Meters)
    pub crate_load_distance: u32,
    /// how far crates apart crates can be and still unpack (Meters)
    pub crate_spread: u32,
    /// how many times a user may switch sides in a given round,
    /// or None for unlimited side switches
    pub side_switches: Option<u8>,
    /// the life types different vehicles use
    pub life_types: FxHashMap<Vehicle, LifeType>,
    /// the life reset configuration for each life type. A pair
    /// of number of lives per reset, and reset time in seconds.
    pub default_lives: FxHashMap<LifeType, (u8, u32)>,
    /// vehicle cargo configuration
    pub cargo: FxHashMap<Vehicle, CargoConfig>,
    /// The name of the crate group for each side
    pub crate_template: FxHashMap<Side, String>,
    /// deployables configuration for each side
    pub deployables: FxHashMap<Side, Vec<Deployable>>,
    /// deployable troops configuration for each side
    pub troops: FxHashMap<Side, Vec<Troop>>,
}

impl Cfg {
    pub fn load(miz_state_path: &Path) -> LuaResult<Self> {
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
        let file = loop {
            match File::open(&path) {
                Ok(f) => break f,
                Err(e) => match e.kind() {
                    io::ErrorKind::NotFound => {
                        let file = File::create(&path).map_err(|e| {
                            error!("could not create default config {}", e);
                            err("creating cfg")
                        })?;
                        serde_json::to_writer_pretty(file, &Cfg::default()).map_err(|e| {
                            error!("could not write default config {}", e);
                            err("writing default cfg")
                        })?;
                    }
                    e => {
                        error!("could not open config file {}", e);
                        return Err(err("opening config"));
                    }
                },
            }
        };
        let cfg: Self = serde_json::from_reader(file).map_err(|e| {
            error!("failed to decode cfg file {:?}, {:?}", path, e);
            err("cfg decode error")
        })?;
        Ok(cfg)
    }
}

fn default_red_troops() -> Vec<Troop> {
    vec![
        Troop {
            name: "Standard".into(),
            template: "RSTANDARDTROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: true,
            jtac: true,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 800,
        },
        Troop {
            name: "Anti Tank".into(),
            template: "RATTROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: true,
            jtac: true,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 1000,
        },
        Troop {
            name: "Mortar".into(),
            template: "RMORTARTROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: true,
            jtac: true,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 1200,
        },
        Troop {
            name: "Igla".into(),
            template: "RIGLATROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: false,
            jtac: true,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 500,
        },
    ]
}

fn default_blue_troops() -> Vec<Troop> {
    vec![
        Troop {
            name: "Standard".into(),
            template: "BSTANDARDTROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: true,
            jtac: true,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 800,
        },
        Troop {
            name: "Anti Tank".into(),
            template: "BATTROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: true,
            jtac: true,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 1000,
        },
        Troop {
            name: "Mortar".into(),
            template: "BMORTARTROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: true,
            jtac: true,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 1200,
        },
        Troop {
            name: "Stinger".into(),
            template: "BSTINGERTROOP".into(),
            persist: PersistTyp::Forever,
            can_capture: false,
            jtac: false,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            weight: 500,
        },
    ]
}

fn default_red_deployables() -> Vec<Deployable> {
    vec![
        Deployable {
            path: vec!["Radar SAMs".into(), "SA 6 Kub".into()],
            template: "DEPSA6".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![
                Crate {
                    name: "Kub Launcher".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("Kub 2P25 ln".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
                Crate {
                    name: "Kub Radar".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("Kub 1S91 str".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
            ],
            repair_crate: Some(Crate {
                name: "Kub Repair".into(),
                weight: 1200,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }),
            logistics: None,
        },
        Deployable {
            path: vec!["Radar SAMs".into(), "SA 11 Buk".into()],
            template: "DEPSA11".into(),
            persist: PersistTyp::Forever,
            limit: 2,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![
                Crate {
                    name: "SA11 Launcher".into(),
                    weight: 1000,
                    required: 2,
                    pos_unit: Some("SA-11 Buk LN 9A310M1".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
                Crate {
                    name: "SA11 Search Radar".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("SA-11 Buk SR 9S18M1".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
                Crate {
                    name: "SA11 CC".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("SA-11 Buk CC 9S470M1".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
            ],
            repair_crate: Some(Crate {
                name: "Buk Repair".into(),
                weight: 1200,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }),
            logistics: None,
        },
        Deployable {
            path: vec!["Radar SAMs".into(), "SA15 Tor".into()],
            template: "DEPSA15".into(),
            persist: PersistTyp::Forever,
            limit: 2,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "SA15 Tor".into(),
                weight: 1000,
                required: 3,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["Radar SAMs".into(), "SA8 Osa".into()],
            template: "DEPSA8".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "SA8 Osa".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["AAA".into(), "ZU23 Emplacement".into()],
            template: "DEPZU23".into(),
            persist: PersistTyp::Forever,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "ZU23 Emplacement".into(),
                weight: 500,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["AAA".into(), "Shilka".into()],
            template: "DEPSHILKA".into(),
            persist: PersistTyp::Forever,
            limit: 6,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "Shilka Crate".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["AAA".into(), "Tunguska".into()],
            template: "DEPTUNGUSKA".into(),
            persist: PersistTyp::Forever,
            limit: 6,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "Tunguska Crate".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["IR SAMs".into(), "SA13 Strela".into()],
            template: "DEPSA13".into(),
            persist: PersistTyp::Forever,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "SA13 Strela Crate".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["Ground Units".into(), "M109".into()],
            template: "DEPM109R".into(),
            persist: PersistTyp::Forever,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "M109 Crate".into(),
                weight: 1000,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["FARP".into()],
            template: "DEPFARPR".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "FARP Crate".into(),
                weight: 1000,
                required: 4,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: Some(Crate {
                name: "FARP Repair".into(),
                weight: 1000,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }),
            logistics: Some(2000),
        },
    ]
}

fn default_blue_deployables() -> Vec<Deployable> {
    vec![
        Deployable {
            path: vec!["Radar SAMs".into(), "Roland ADS".into()],
            template: "DEPROLAND".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "Roland".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["Radar SAMs".into(), "Hawk System".into()],
            template: "DEPHAWK".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![
                Crate {
                    name: "Hawk Launcher".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("Hawk ln".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
                Crate {
                    name: "Hawk Search Radar".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("Hawk sr".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
                Crate {
                    name: "Hawk Track Radar".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("Hawk tr".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
                Crate {
                    name: "Hawk CC".into(),
                    weight: 1000,
                    required: 1,
                    pos_unit: Some("Hawk pcp".into()),
                    max_drop_height_agl: 10,
                    max_drop_speed: 50,
                },
            ],
            repair_crate: Some(Crate {
                name: "Hawk Repair".into(),
                weight: 1200,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }),
            logistics: None,
        },
        Deployable {
            path: vec!["IR SAMs".into(), "Avenger".into()],
            template: "DEPAVENGER".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "Avenger Crate".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["IR SAMs".into(), "Linebacker".into()],
            template: "DEPLINEBACKER".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "Linebacker Crate".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["AAA".into(), "Flakpanzergepard".into()],
            template: "DEPGEPARD".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "Flakpanzergepard Crate".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["AAA".into(), "Vulkan".into()],
            template: "DEPVULKAN".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "Vulkan Crate".into(),
                weight: 1000,
                required: 2,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["Ground Units".into(), "M109".into()],
            template: "DEPM109B".into(),
            persist: PersistTyp::Forever,
            limit: 10,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "M109 Crate".into(),
                weight: 1000,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: None,
            logistics: None,
        },
        Deployable {
            path: vec!["FARP".into()],
            template: "DEPFARPB".into(),
            persist: PersistTyp::Forever,
            limit: 4,
            limit_enforce: LimitEnforceTyp::DeleteOldest,
            crates: vec![Crate {
                name: "FARP Crate".into(),
                weight: 1000,
                required: 4,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }],
            repair_crate: Some(Crate {
                name: "FARP Repair".into(),
                weight: 1000,
                required: 1,
                pos_unit: None,
                max_drop_height_agl: 10,
                max_drop_speed: 50,
            }),
            logistics: Some(2000),
        },
    ]
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            repair_time: 1800,
            repair_crate: FxHashMap::from_iter([
                (
                    Side::Blue,
                    Crate {
                        name: "Repair Crate".into(),
                        weight: 1500,
                        required: 1,
                        pos_unit: None,
                        max_drop_height_agl: 10,
                        max_drop_speed: 50,
                    },
                ),
                (
                    Side::Red,
                    Crate {
                        name: "Repair Crate".into(),
                        weight: 2000,
                        required: 1,
                        pos_unit: None,
                        max_drop_height_agl: 10,
                        max_drop_speed: 50,
                    },
                ),
            ]),
            logistics_exclusion: 10000,
            inside_logistics_unpack_delay: Some(900),
            unit_cull_distance: 70000,
            unit_cull_freq: 10,
            crate_load_distance: 50,
            crate_spread: 250,
            side_switches: Some(1),
            default_lives: FxHashMap::from_iter([
                (LifeType::Standard, (3, 21600)),
                (LifeType::Intercept, (4, 21600)),
                (LifeType::Attack, (4, 21600)),
                (LifeType::Logistics, (6, 21600)),
                (LifeType::Recon, (6, 21600)),
            ]),
            life_types: FxHashMap::from_iter([
                ("FA-18C_hornet".into(), LifeType::Standard),
                ("F-14A-135-GR".into(), LifeType::Standard),
                ("F-14B".into(), LifeType::Standard),
                ("F-15C".into(), LifeType::Standard),
                ("F-15ESE".into(), LifeType::Standard),
                ("MiG-29S".into(), LifeType::Standard),
                ("M-2000C".into(), LifeType::Standard),
                ("F-16C_50".into(), LifeType::Standard),
                ("MiG-29A".into(), LifeType::Standard),
                ("Su-27".into(), LifeType::Standard),
                ("AH-64D_BLK_II".into(), LifeType::Attack),
                ("Mi-24P".into(), LifeType::Attack),
                ("Ka-50_3".into(), LifeType::Attack),
                ("A-10C".into(), LifeType::Attack),
                ("A-10A".into(), LifeType::Attack),
                ("Su-25".into(), LifeType::Attack),
                ("Su-25T".into(), LifeType::Attack),
                ("AJS37".into(), LifeType::Attack),
                ("Ka-50".into(), LifeType::Attack),
                ("AV8BNA".into(), LifeType::Attack),
                ("A-10C_2".into(), LifeType::Attack),
                ("JF-17".into(), LifeType::Attack),
                ("SA342L".into(), LifeType::Logistics),
                ("UH-1H".into(), LifeType::Logistics),
                ("Mi-8MT".into(), LifeType::Logistics),
                ("SA342M".into(), LifeType::Logistics),
                ("L-39C".into(), LifeType::Recon),
                ("L-39ZA".into(), LifeType::Recon),
                ("TF-51D".into(), LifeType::Recon),
                ("Yak-52".into(), LifeType::Recon),
                ("C-101CC".into(), LifeType::Recon),
                ("MB-339A".into(), LifeType::Recon),
                ("F-5E-3".into(), LifeType::Intercept),
                ("MiG-21Bis".into(), LifeType::Intercept),
                ("MiG-19P".into(), LifeType::Intercept),
                ("Mirage-F1EE".into(), LifeType::Intercept),
                ("Mirage-F1CE".into(), LifeType::Intercept),
            ]),
            cargo: FxHashMap::from_iter([
                (
                    "UH-1H".into(),
                    CargoConfig {
                        troop_slots: 1,
                        crate_slots: 1,
                        total_slots: 2,
                    },
                ),
                (
                    "Mi-8MT".into(),
                    CargoConfig {
                        troop_slots: 2,
                        crate_slots: 1,
                        total_slots: 2,
                    },
                ),
                (
                    "SA342L".into(),
                    CargoConfig {
                        troop_slots: 1,
                        crate_slots: 1,
                        total_slots: 1,
                    },
                ),
                (
                    "SA342M".into(),
                    CargoConfig {
                        troop_slots: 1,
                        crate_slots: 1,
                        total_slots: 1,
                    },
                ),
                (
                    "Mi-24P".into(),
                    CargoConfig {
                        troop_slots: 1,
                        crate_slots: 1,
                        total_slots: 1,
                    },
                ),
            ]),
            crate_template: FxHashMap::from_iter([
                (Side::Red, "RCRATE".into()),
                (Side::Blue, "BCRATE".into()),
            ]),
            deployables: FxHashMap::from_iter([
                (Side::Red, default_red_deployables()),
                (Side::Blue, default_blue_deployables()),
            ]),
            troops: FxHashMap::from_iter([
                (Side::Red, default_red_troops()),
                (Side::Blue, default_blue_troops()),
            ]),
        }
    }
}
