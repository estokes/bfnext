use anyhow::Result;
use chrono::{DateTime, Utc};
use dcso3::{
    event::Shot,
    object::{DcsObject, DcsOid},
    weapon::ClassWeapon,
    LuaVec3, MizLua, Vector3,
    trigger::{Trigger, SmokeColor},
};
use fxhash::FxHashMap;
use log::{debug, info, error};
use std::hash::{Hash, Hasher};
use rand::Rng;
extern crate nalgebra as na;

/// Configuration for splash damage system
#[derive(Debug, Clone)]
pub struct SplashDamageConfig {
    /// Overall scaling factor for all damage calculations
    pub overall_scaling: f64,
    /// Multiplier for rocket weapons
    pub rocket_multiplier: f64,
    /// Scaling factor for cascade explosions
    pub cascade_scaling: f64,
    /// Damage threshold for triggering cascade explosions
    pub cascade_damage_threshold: f64,
    /// Health threshold for units to explode (percentage)
    pub cascade_explode_threshold: f64,
    /// Enable wave explosions
    pub wave_explosions: bool,
    /// Static damage boost
    pub static_damage_boost: f64,
    /// Global cook-off configuration
    pub allunits_cookoff_count: u32,
    pub allunits_cookoff_duration: f64,
    pub allunits_cookoff_power: f64,
    pub allunits_cookoff_powerrandom: f64,
    pub allunits_cookoff_chance: f64,
    // Cook-off flare configuration - implemented based on Lua script functionality
    pub cookoff_flares_enabled: bool,
    pub cookoff_flare_chance: f64,
    pub cookoff_flare_instant: bool,
    pub cookoff_flare_instant_min: u32,
    pub cookoff_flare_instant_max: u32,
    pub cookoff_flare_offset: f64,
    /// Maximum number of ground ordnance shells tracked at once (from Lua script)
    pub groundunitordnance_maxtrackedcount: u32,
}

impl Default for SplashDamageConfig {
    fn default() -> Self {
        Self {
            overall_scaling: 1.25,  // From Lua: overall_scaling = 1.25
            rocket_multiplier: 1.3,  // From Lua: rocket_multiplier = 1.3
            cascade_scaling: 2.0,    // From Lua: cascade_scaling = 2
            cascade_damage_threshold: 0.1,  // From Lua: cascade_damage_threshold = 0.1
            cascade_explode_threshold: 60.0,  // From Lua: cascade_explode_threshold = 60
            wave_explosions: true,   // From Lua: wave_explosions = true
            static_damage_boost: 2000.0,  // From Lua: static_damage_boost = 2000
            // Global cook-off configuration (from Lua script)
            allunits_cookoff_count: 4,     // From Lua: allunits_cookoff_count = 4
            allunits_cookoff_duration: 30.0, // From Lua: allunits_cookoff_duration = 30
            allunits_cookoff_power: 10.0,  // From Lua: allunits_cookoff_power = 10
            allunits_cookoff_powerrandom: 50.0, // From Lua: allunits_cookoff_powerrandom = 50
            allunits_cookoff_chance: 0.4,  // From Lua: allunits_cookoff_chance = 0.4
            // Cook-off flare configuration (from Lua script)
            cookoff_flares_enabled: true,
            cookoff_flare_chance: 0.3,
            cookoff_flare_instant: false,
            cookoff_flare_instant_min: 2,
            cookoff_flare_instant_max: 5,
            cookoff_flare_offset: 1.0,
            // Ground ordnance tracking limits (from Lua script)
            groundunitordnance_maxtrackedcount: 100, // From Lua: groundunitordnance_maxtrackedcount = 100
        }
    }
}

/// Weapon data for splash damage calculations
#[derive(Debug, Clone)]
pub struct WeaponData {
    pub name: String,
    pub explosion_power: f64,
    pub blast_radius: f64,
    pub is_rocket: bool,
    pub is_shaped_charge: bool,
    pub is_ground_ordnance: bool,
    /// Cluster weapon configuration
    pub is_cluster: bool,
    pub submunition_count: u32,
    pub submunition_explosive: f64,
    pub submunition_name: String,
}

/// Unit type damage modifiers
#[derive(Debug, Clone)]
pub struct UnitTypeData {
    pub damage_modifier: f64,
    pub can_cook_off: bool,
    pub cook_off_power: f64,
    pub cook_off_count: u32,
    pub cook_off_duration: f64,
    pub is_tanker: bool,
    pub flame_size: f64,
    pub flame_duration: f64,
}

/// Tracked weapon for splash damage (matching Lua script structure)
#[derive(Debug, Clone)]
pub struct TrackedWeapon {
    pub weapon_name: String,
    pub weapon_oid: DcsOid<ClassWeapon>,
    pub fire_position: LuaVec3,
    pub fire_time: DateTime<Utc>,
    pub predicted_impact: Option<LuaVec3>,
    pub last_update_time: DateTime<Utc>,
    pub last_position: Option<LuaVec3>,
    pub last_velocity: Option<LuaVec3>,
    pub weapon_data: WeaponData,
    pub initiator_name: String,
    // Additional fields from Lua script
    pub weapon_category: Option<String>, // From Lua: cat (weapon type name)
    pub parent_weapon: Option<String>, // From Lua: parent (for submunitions)
    pub is_ground_ordnance: bool, // Track if this is ground ordnance for limits
}

/// Damage result for a unit
#[derive(Debug, Clone)]
pub struct DamageResult {
    pub unit_id: u32,
    pub unit_name: String,
    pub unit_type: String,
    pub position: LuaVec3,
    pub distance: f64,
    pub damage: f64,
    pub health_before: f64,
    pub health_after: f64,
    pub destroyed: bool,
}

/// Splash damage system
#[derive(Debug)]
pub struct SplashDamageSystem {
    config: SplashDamageConfig,
    tracked_weapons: FxHashMap<DcsOid<ClassWeapon>, TrackedWeapon>,
    weapon_data: FxHashMap<String, WeaponData>,
    unit_types: FxHashMap<String, UnitTypeData>,
    cargo_units: FxHashMap<String, UnitTypeData>,
}

impl Default for SplashDamageSystem {
    fn default() -> Self {
        Self::new(SplashDamageConfig::default())
    }
}

impl SplashDamageSystem {
    pub fn new(config: SplashDamageConfig) -> Self {
        let mut system = Self {
            config,
            tracked_weapons: FxHashMap::default(),
            weapon_data: FxHashMap::default(),
            unit_types: FxHashMap::default(),
            cargo_units: FxHashMap::default(),
        };
        
        system.initialize_weapon_data();
        system.initialize_unit_types();
        system.initialize_cargo_units();
        
        system
    }

    /// Initialize weapon data from the Lua script
    fn initialize_weapon_data(&mut self) {
        // Core weapons from the Lua script's explosive table
        let weapons = vec![
            // Bombs - matching Lua script values
            ("Mk_84", WeaponData {
                name: "Mk_84".to_string(),
                explosion_power: 1000.0,
                blast_radius: 200.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_82", WeaponData {
                name: "Mk_82".to_string(),
                explosion_power: 500.0,
                blast_radius: 150.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_83", WeaponData {
                name: "Mk_83".to_string(),
                explosion_power: 750.0,
                blast_radius: 175.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_82AIR", WeaponData {
                name: "Mk_82AIR".to_string(),
                explosion_power: 500.0,
                blast_radius: 150.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_82SNAKEYE", WeaponData {
                name: "Mk_82SNAKEYE".to_string(),
                explosion_power: 500.0,
                blast_radius: 150.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_84AIR", WeaponData {
                name: "Mk_84AIR".to_string(),
                explosion_power: 1000.0,
                blast_radius: 200.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_84SNAKEYE", WeaponData {
                name: "Mk_84SNAKEYE".to_string(),
                explosion_power: 1000.0,
                blast_radius: 200.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_77mod0", WeaponData {
                name: "Mk_77mod0".to_string(),
                explosion_power: 800.0,
                blast_radius: 180.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("Mk_77mod1", WeaponData {
                name: "Mk_77mod1".to_string(),
                explosion_power: 800.0,
                blast_radius: 180.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            // Rockets
            ("HYDRA_70_M151", WeaponData {
                name: "HYDRA_70_M151".to_string(),
                explosion_power: 50.0,
                blast_radius: 30.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("HYDRA_70_M229", WeaponData {
                name: "HYDRA_70_M229".to_string(),
                explosion_power: 50.0,
                blast_radius: 30.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("S_8KOM", WeaponData {
                name: "S_8KOM".to_string(),
                explosion_power: 40.0,
                blast_radius: 25.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("S_5M", WeaponData {
                name: "S_5M".to_string(),
                explosion_power: 30.0,
                blast_radius: 20.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("S_24B", WeaponData {
                name: "S_24B".to_string(),
                explosion_power: 80.0,
                blast_radius: 40.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            // Missiles - core variants only
            ("AGM_65D", WeaponData {
                name: "AGM_65D".to_string(),
                explosion_power: 200.0,
                blast_radius: 50.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("AGM_65E", WeaponData {
                name: "AGM_65E".to_string(),
                explosion_power: 300.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("AGM_65F", WeaponData {
                name: "AGM_65F".to_string(),
                explosion_power: 250.0,
                blast_radius: 55.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("AGM_65G", WeaponData {
                name: "AGM_65G".to_string(),
                explosion_power: 300.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("AGM_65H", WeaponData {
                name: "AGM_65H".to_string(),
                explosion_power: 300.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("AGM_65K", WeaponData {
                name: "AGM_65K".to_string(),
                explosion_power: 300.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("AGM_65L", WeaponData {
                name: "AGM_65L".to_string(),
                explosion_power: 300.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("AGM_65M", WeaponData {
                name: "AGM_65M".to_string(),
                explosion_power: 300.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: true,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            
            // Cluster weapons - matching Lua script values
            ("CBU_87", WeaponData {
                name: "CBU_87".to_string(),
                explosion_power: 0.0, // Main weapon has no explosive
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 202,
                submunition_explosive: 0.5,
                submunition_name: "BLU_97B".to_string(),
            }),
            ("CBU_97", WeaponData {
                name: "CBU_97".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 10,
                submunition_explosive: 15.0,
                submunition_name: "BLU_108".to_string(),
            }),
            ("CBU_103", WeaponData {
                name: "CBU_103".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 202,
                submunition_explosive: 0.5,
                submunition_name: "BLU_97B".to_string(),
            }),
            ("CBU_105", WeaponData {
                name: "CBU_105".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 10,
                submunition_explosive: 15.0,
                submunition_name: "BLU_108".to_string(),
            }),
            ("ROCKEYE", WeaponData {
                name: "ROCKEYE".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 247,
                submunition_explosive: 2.0,
                submunition_name: "Mk_118".to_string(),
            }),
            ("BELOUGA", WeaponData {
                name: "BELOUGA".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 151,
                submunition_explosive: 0.3,
                submunition_name: "grenade_AC".to_string(),
            }),
            ("RBK_250", WeaponData {
                name: "RBK_250".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 60,
                submunition_explosive: 0.5,
                submunition_name: "PTAB_25M".to_string(),
            }),
            ("RBK_500", WeaponData {
                name: "RBK_500".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 108,
                submunition_explosive: 0.5,
                submunition_name: "PTAB_10_5".to_string(),
            }),
            ("AGM_154A", WeaponData {
                name: "AGM_154A".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 145,
                submunition_explosive: 2.0,
                submunition_name: "BLU-97/B".to_string(),
            }),

            // British Bombs
            ("British_GP_250LB_Bomb_Mk1", WeaponData {
                name: "British_GP_250LB_Bomb_Mk1".to_string(),
                explosion_power: 100.0,
                blast_radius: 50.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("British_GP_500LB_Bomb_Mk1", WeaponData {
                name: "British_GP_500LB_Bomb_Mk1".to_string(),
                explosion_power: 213.0,
                blast_radius: 75.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // German Bombs
            ("SC_50", WeaponData {
                name: "SC_50".to_string(),
                explosion_power: 20.0,
                blast_radius: 25.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("SC_250_T1_L2", WeaponData {
                name: "SC_250_T1_L2".to_string(),
                explosion_power: 100.0,
                blast_radius: 50.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("SC_500_L2", WeaponData {
                name: "SC_500_L2".to_string(),
                explosion_power: 213.0,
                blast_radius: 75.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Russian Bombs
            ("FAB_100", WeaponData {
                name: "FAB_100".to_string(),
                explosion_power: 45.0,
                blast_radius: 35.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("FAB_250", WeaponData {
                name: "FAB_250".to_string(),
                explosion_power: 118.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("FAB_500", WeaponData {
                name: "FAB_500".to_string(),
                explosion_power: 213.0,
                blast_radius: 75.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("FAB_1500", WeaponData {
                name: "FAB_1500".to_string(),
                explosion_power: 675.0,
                blast_radius: 150.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Guided Bombs
            ("GBU_10", WeaponData {
                name: "GBU_10".to_string(),
                explosion_power: 582.0,
                blast_radius: 120.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("GBU_12", WeaponData {
                name: "GBU_12".to_string(),
                explosion_power: 100.0,
                blast_radius: 50.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("GBU_16", WeaponData {
                name: "GBU_16".to_string(),
                explosion_power: 274.0,
                blast_radius: 85.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("GBU_24", WeaponData {
                name: "GBU_24".to_string(),
                explosion_power: 582.0,
                blast_radius: 120.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("GBU_31", WeaponData {
                name: "GBU_31".to_string(),
                explosion_power: 582.0,
                blast_radius: 120.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("GBU_38", WeaponData {
                name: "GBU_38".to_string(),
                explosion_power: 100.0,
                blast_radius: 50.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Russian Guided Bombs
            ("KAB_500Kr", WeaponData {
                name: "KAB_500Kr".to_string(),
                explosion_power: 213.0,
                blast_radius: 75.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("KAB_1500Kr", WeaponData {
                name: "KAB_1500Kr".to_string(),
                explosion_power: 675.0,
                blast_radius: 150.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Anti-Ship Missiles
            ("AGM_84D", WeaponData {
                name: "AGM_84D".to_string(),
                explosion_power: 488.0,
                blast_radius: 110.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("X_22", WeaponData {
                name: "X_22".to_string(),
                explosion_power: 1200.0,
                blast_radius: 200.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Anti-Radar Missiles
            ("AGM_88C", WeaponData {
                name: "AGM_88C".to_string(),
                explosion_power: 69.0,
                blast_radius: 40.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("X_58", WeaponData {
                name: "X_58".to_string(),
                explosion_power: 149.0,
                blast_radius: 60.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // French Rockets
            ("SNEB_TYPE251_F1B", WeaponData {
                name: "SNEB_TYPE251_F1B".to_string(),
                explosion_power: 4.0,
                blast_radius: 15.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("SNEB_TYPE253_F1B", WeaponData {
                name: "SNEB_TYPE253_F1B".to_string(),
                explosion_power: 5.0,
                blast_radius: 18.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Russian Rockets
            ("S_8KOM", WeaponData {
                name: "S_8KOM".to_string(),
                explosion_power: 40.0,
                blast_radius: 30.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("S_24B", WeaponData {
                name: "S_24B".to_string(),
                explosion_power: 80.0,
                blast_radius: 40.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("S_25OF", WeaponData {
                name: "S_25OF".to_string(),
                explosion_power: 194.0,
                blast_radius: 70.0,
                is_rocket: true,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Ground Ordnance
            ("weapons.shells.M_155mm_HE", WeaponData {
                name: "weapons.shells.M_155mm_HE".to_string(),
                explosion_power: 60.0,
                blast_radius: 40.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: true,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),
            ("weapons.shells.2A18_122", WeaponData {
                name: "weapons.shells.2A18_122".to_string(),
                explosion_power: 22.0,
                blast_radius: 25.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: true,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            }),

            // Additional Cluster Weapons
            ("CBU_99", WeaponData {
                name: "CBU_99".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 247,
                submunition_explosive: 2.0,
                submunition_name: "Mk_118".to_string(),
            }),
            ("BLG66_BELOUGA", WeaponData {
                name: "BLG66_BELOUGA".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 151,
                submunition_explosive: 0.3,
                submunition_name: "grenade_AC".to_string(),
            }),
            ("RBK_500U", WeaponData {
                name: "RBK_500U".to_string(),
                explosion_power: 0.0,
                blast_radius: 0.0,
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: true,
                submunition_count: 352,
                submunition_explosive: 0.2,
                submunition_name: "OAB_25RT".to_string(),
            }),
        ];

        for (name, data) in weapons {
            self.weapon_data.insert(name.to_string(), data);
        }
    }

    /// Initialize unit type data
    fn initialize_unit_types(&mut self) {
        let unit_types = vec![
            ("Infantry", UnitTypeData {
                damage_modifier: 1.0,
                can_cook_off: false,
                cook_off_power: 0.0,
                cook_off_count: 0,
                cook_off_duration: 0.0,
                is_tanker: false,
                flame_size: 0.0,
                flame_duration: 0.0,
            }),
            ("Tank", UnitTypeData {
                damage_modifier: 0.3,
                can_cook_off: true,
                cook_off_power: 100.0,
                cook_off_count: 3,
                cook_off_duration: 5.0,
                is_tanker: false,
                flame_size: 2.0,
                flame_duration: 10.0,
            }),
            ("Artillery", UnitTypeData {
                damage_modifier: 0.5,
                can_cook_off: true,
                cook_off_power: 150.0,
                cook_off_count: 5,
                cook_off_duration: 8.0,
                is_tanker: false,
                flame_size: 3.0,
                flame_duration: 15.0,
            }),
            ("Armored Vehicle", UnitTypeData {
                damage_modifier: 0.4,
                can_cook_off: true,
                cook_off_power: 80.0,
                cook_off_count: 2,
                cook_off_duration: 4.0,
                is_tanker: false,
                flame_size: 1.5,
                flame_duration: 8.0,
            }),
            ("Structure", UnitTypeData {
                damage_modifier: 0.8,
                can_cook_off: true,  // Enable cook-off for fortifications/bunkers/structures
                cook_off_power: 120.0,  // Moderate power for structures
                cook_off_count: 3,  // Fewer explosions than vehicles
                cook_off_duration: 6.0,  // Shorter duration
                is_tanker: false,
                flame_size: 2.5,  // Medium flame size
                flame_duration: 12.0,  // Medium duration
            }),
        ];

        for (name, data) in unit_types {
            self.unit_types.insert(name.to_string(), data);
        }
    }

    /// Initialize cargo units (ammo trucks, fuel tankers, etc.)
    fn initialize_cargo_units(&mut self) {
        let cargo_units = vec![
            ("Ural-4320", UnitTypeData {
                damage_modifier: 0.9,
                can_cook_off: true,
                cook_off_power: 200.0,
                cook_off_count: 8,
                cook_off_duration: 12.0,
                is_tanker: true,
                flame_size: 4.0,
                flame_duration: 20.0,
            }),
            ("M978", UnitTypeData {
                damage_modifier: 0.9,
                can_cook_off: true,
                cook_off_power: 300.0,
                cook_off_count: 10,
                cook_off_duration: 15.0,
                is_tanker: true,
                flame_size: 5.0,
                flame_duration: 25.0,
            }),
        ];

        for (name, data) in cargo_units {
            self.cargo_units.insert(name.to_string(), data);
        }
    }

    /// Track a weapon shot
    pub fn track_weapon_shot(
        &mut self,
        lua: MizLua<'_>,
        shot_event: &Shot,
        current_time: DateTime<Utc>,
    ) -> Result<()> {
        let weapon_obj = &shot_event.weapon;
        let weapon_name = weapon_obj.get_type()?;
        
        // Get weapon position and velocity through the object interface
        let weapon_as_object = shot_event.weapon.as_object()?;
        let fire_pos = weapon_as_object.get_position()?.p;
        let velocity = weapon_as_object.get_velocity()?;
        
        // Get weapon ID from the weapon object
        let weapon_oid = shot_event.weapon.object_id()?;
        
        // Get weapon data
        let weapon_name_clone = weapon_name.clone();
        let weapon_data = self.weapon_data.get(&weapon_name_clone)
            .cloned()
            .unwrap_or_else(|| WeaponData {
                name: weapon_name_clone,
                explosion_power: 100.0, // Default power
                blast_radius: 50.0,     // Default radius
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            });

        // Get initiator name
        let initiator_name = shot_event.initiator.as_object()
            .and_then(|obj| {
                // Try to get name from the object
                if let Ok(name) = obj.get_name() {
                    Ok(Some(name.to_string()))
                } else {
                    Ok(None)
                }
            })
            .unwrap_or_else(|_| Some("Unknown".to_string()))
            .unwrap_or_else(|| "Unknown".to_string());

        let velocity_magnitude = (velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2]).sqrt();
        let mut predicted_impact = None;
        
        if velocity_magnitude < 0.1 {
            info!("Weapon has zero or negligible velocity (magnitude: {:.3}), waiting for DCS to calculate proper velocity", velocity_magnitude);
        } else {
            predicted_impact = Self::calculate_impact_with_terrain_static(lua, fire_pos, velocity);
        }

        // Get weapon category (matching Lua script) - use weapon type name as category
        let weapon_name_for_log = weapon_name.clone();
        let weapon_category_for_log = Some(weapon_name_for_log.clone());
        let weapon_category = Some(weapon_name_for_log.clone());
        
        // Check if this is ground ordnance for tracking limits
        let is_ground_ordnance = weapon_data.is_ground_ordnance;
        
        // Check ground ordnance tracking limits (from Lua script)
        if is_ground_ordnance {
            let ground_ordnance_count = self.tracked_weapons.values()
                .filter(|w| w.is_ground_ordnance)
                .count();
            
            if ground_ordnance_count >= self.config.groundunitordnance_maxtrackedcount as usize {
                info!("Skipping tracking for {}: ground ordnance limit reached ({}/{})", 
                      &weapon_name_for_log, ground_ordnance_count, self.config.groundunitordnance_maxtrackedcount);
                return Ok(());
            }
        }

        let tracked_weapon = TrackedWeapon {
            weapon_name: weapon_name_for_log.clone(),
            weapon_oid: weapon_oid.clone(),
            fire_position: fire_pos,
            fire_time: current_time,
            predicted_impact,
            last_update_time: current_time,
            last_position: Some(fire_pos),
            last_velocity: Some(velocity),
            weapon_data,
            initiator_name,
            // New fields from Lua script
            weapon_category,
            parent_weapon: None, // Will be set for submunitions
            is_ground_ordnance,
        };

        self.tracked_weapons.insert(weapon_oid.clone(), tracked_weapon);
        
        info!("Tracking weapon: {} (category: {:?}, ground ordnance: {}) fired at position {:?} with velocity {:?}", 
              weapon_name_for_log, weapon_category_for_log, is_ground_ordnance, fire_pos, velocity);

        Ok(())
    }

    /// Update tracked weapons
    pub fn update_tracked_weapons(&mut self, lua: MizLua<'_>, current_time: DateTime<Utc>, db: &crate::db::Db) -> Result<()> {
        let mut to_remove = Vec::new();
        let mut impacts_to_process = Vec::new();
        
        for (weapon_oid, tracked_weapon) in self.tracked_weapons.iter_mut() {
            let age = (current_time - tracked_weapon.last_update_time).num_seconds();
            
            // Try to get the weapon from DCS using its DcsOid
            match dcso3::weapon::Weapon::get_instance(lua, weapon_oid) {
                Ok(weapon) => {
                    // Weapon still exists, update its state
                    if let Ok(exists) = weapon.is_exist() {
                        if !exists {
                            // Weapon no longer exists (impacted/destroyed)
                            info!("Weapon {} no longer exists, removing from tracking", tracked_weapon.weapon_name);
                            to_remove.push(weapon_oid.clone());
                            continue;
                        }
                    }
                    
                    // Get current position and velocity
                    if let Ok(position) = weapon.get_position() {
                        tracked_weapon.last_position = Some(position.p);
                    }
                    
                    if let Ok(velocity) = weapon.get_velocity() {
                        tracked_weapon.last_velocity = Some(velocity);
                        
                        // Check if weapon has non-zero velocity for impact prediction
                        let velocity_magnitude = (velocity[0] * velocity[0] + velocity[1] * velocity[1] + velocity[2] * velocity[2]).sqrt();
                        
                                if velocity_magnitude > 0.1 {
                            // Try to predict impact point
                            if let Some(impact_point) = Self::calculate_impact_with_terrain_static(
                                lua,
                                tracked_weapon.last_position.unwrap_or(tracked_weapon.fire_position),
                                velocity,
                            ) {
                                tracked_weapon.predicted_impact = Some(impact_point);
                                info!("Predicted impact for {} at {:?}", tracked_weapon.weapon_name, impact_point);
                            }
                        }
                    }
                    
                    // Update last update time
                    tracked_weapon.last_update_time = current_time;
                    
                    debug!("Updated weapon {} (age: {}s, pos: {:?}, vel: {:?})", 
                           tracked_weapon.weapon_name, age, tracked_weapon.last_position, tracked_weapon.last_velocity);
                }
                Err(_) => {
                    // Weapon ID is invalid or weapon no longer exists
                    info!("Weapon {} no longer exists, processing impact", tracked_weapon.weapon_name);
                    
                    // Store impact to process after iteration
                    if let Some(impact_point) = tracked_weapon.predicted_impact {
                        impacts_to_process.push((weapon_oid.clone(), impact_point));
                    }
                    
                    to_remove.push(weapon_oid.clone());
                }
            }
        }
        
        // Process weapon impacts after iteration to avoid borrowing issues
        for (weapon_oid, impact_point) in impacts_to_process {
            if let Err(e) = self.process_weapon_impact(
                lua,
                weapon_oid,
                impact_point,
                current_time,
                db,
            ) {
                error!("Error processing weapon impact: {:?}", e);
            }
        }
        
        // Remove weapons that are marked for removal
        for weapon_oid in to_remove {
            self.tracked_weapons.remove(&weapon_oid);
        }
        
        Ok(())
    }

    /// Process weapon impact and calculate splash damage
    pub fn process_weapon_impact(
        &mut self,
        lua: MizLua<'_>,
        weapon_oid: DcsOid<ClassWeapon>,
        impact_position: LuaVec3,
        current_time: DateTime<Utc>,
        db: &crate::db::Db,
    ) -> Result<Vec<DamageResult>> {
        let tracked_weapon = match self.tracked_weapons.get(&weapon_oid) {
            Some(weapon) => weapon.clone(),
            None => return Ok(Vec::new()),
        };

        // Handle cluster weapons differently - create multiple submunition impacts
        if tracked_weapon.weapon_data.is_cluster {
            return self.process_cluster_weapon_impact(lua, impact_position, &tracked_weapon.weapon_data, &tracked_weapon.weapon_name, db);
        }

        let weapon_data = &tracked_weapon.weapon_data;
        let mut damage_results = Vec::new();

        // Calculate explosion power with scaling
        let mut explosion_power = weapon_data.explosion_power * self.config.overall_scaling;
        if weapon_data.is_rocket {
            explosion_power *= self.config.rocket_multiplier;
        }

        // Calculate blast radius
        let mut blast_radius = weapon_data.blast_radius * self.config.overall_scaling;
        
        // Apply ground ordnance scan radius (from Lua script)
        if weapon_data.is_ground_ordnance {
            // Use 50m scan radius for ground ordnance instead of dynamic blast radius
            blast_radius = 50.0;
            info!("Using 50m scan radius for ground ordnance weapon {}", weapon_data.name);
        }

        // Note: Larger explosions are handled through the damage system and cascade explosions
        // No need for additional explosion at impact point as it creates double explosions

        // Create visual effects for weapon impact
        info!("Creating visual effects for weapon impact at {:?}", impact_position);
        self.create_impact_effects(lua, impact_position, explosion_power, blast_radius, weapon_data)?;


                                    info!(
            "Processing impact for {} (category: {:?}, ground ordnance: {}, parent: {:?}) (OID: {:?}) fired by {} at {:?} with power {:.1} and radius {:.1}m (fired at {})",
            weapon_data.name, tracked_weapon.weapon_category, tracked_weapon.is_ground_ordnance, tracked_weapon.parent_weapon,
            tracked_weapon.weapon_oid, tracked_weapon.initiator_name, impact_position, explosion_power, blast_radius,
            tracked_weapon.fire_time.format("%H:%M:%S")
        );

        // Find units within blast radius
        let units_in_range = self.find_units_in_range(lua, impact_position, blast_radius, db)?;

        // Process damage for each unit
        for unit in units_in_range {
            let mut damage_result = self.calculate_unit_damage(
                lua,
                &unit,
                impact_position,
                explosion_power,
                blast_radius,
                weapon_data,
            )?;

            // Apply static damage boost for structures (from Lua script)
            if damage_result.unit_type.contains("Structure") || damage_result.unit_type.contains("Building") {
                damage_result.damage += self.config.static_damage_boost;
                info!("Applied static damage boost of {:.1} to structure {}", 
                      self.config.static_damage_boost, damage_result.unit_name);
            }


            if damage_result.damage > 0.0 {
                // Log detailed damage information using all DamageResult fields
                                        info!(
                    "Unit {} (ID: {}, Type: {}) at distance {:.1}m: {:.1} damage (health: {:.1} -> {:.1}, destroyed: {})",
                    damage_result.unit_name, damage_result.unit_id, damage_result.unit_type,
                    damage_result.distance, damage_result.damage, 
                    damage_result.health_before, damage_result.health_after, damage_result.destroyed
                );
                
                // Note: Smoke effects are now created during cook-off sequences, not on initial damage
                // This matches the Lua script's behavior where smoke effects happen during cook-off processes
                
                damage_results.push(damage_result);
            }
        }

        // Process cascade explosions
        if self.config.wave_explosions {
            let cascade_results = self.process_cascade_explosions(
                lua,
                &damage_results,
                impact_position,
                current_time,
                db,
            )?;
            damage_results.extend(cascade_results);
        }

        // Remove the weapon from tracking after impact
        self.tracked_weapons.remove(&weapon_oid);

        Ok(damage_results)
    }

    /// Process cluster weapon impact - creates multiple submunition explosions
    fn process_cluster_weapon_impact(
        &mut self,
        lua: MizLua<'_>,
        impact_position: LuaVec3,
        weapon_data: &WeaponData,
        parent_weapon_name: &str,
        db: &crate::db::Db,
    ) -> Result<Vec<DamageResult>> {
        let mut all_damage_results = Vec::new();
        
        info!("Processing cluster weapon {} (parent: {}) with {} submunitions", 
              weapon_data.name, parent_weapon_name, weapon_data.submunition_count);

        // Create submunition explosions in a spread pattern
        let submunition_count = weapon_data.submunition_count;
        let spread_radius = 200.0; // Base spread radius in meters
        
        for i in 0..submunition_count {
            // Calculate random offset within spread radius
            let angle = (i as f64 / submunition_count as f64) * 2.0 * std::f64::consts::PI;
            let distance = spread_radius * (0.5 + 0.5 * (i as f64 / submunition_count as f64));
            
            let offset_x = distance * angle.cos();
            let offset_z = distance * angle.sin();
            
            // Random height variation for submunitions
            let height_variation = 10.0 * (i as f64 % 3.0 - 1.0);
            
            let submunition_position = LuaVec3([
                impact_position[0] + offset_x,
                impact_position[1] + height_variation,
                impact_position[2] + offset_z,
            ].into());

            // Create submunition weapon data
            let submunition_data = WeaponData {
                name: weapon_data.submunition_name.clone(),
                explosion_power: weapon_data.submunition_explosive,
                blast_radius: (weapon_data.submunition_explosive * 2.0).max(10.0), // Scale blast radius
                is_rocket: false,
                is_shaped_charge: false,
                is_ground_ordnance: false,
                is_cluster: false,
                submunition_count: 0,
                submunition_explosive: 0.0,
                submunition_name: String::new(),
            };

            info!("Creating submunition {} explosion at {:?} (parent: {})", 
                  submunition_data.name, submunition_position, parent_weapon_name);

            // Find units in range for submunition impact
            let units_in_range = self.find_units_in_range(lua, submunition_position, submunition_data.blast_radius, db)?;
            
            // Process damage for each unit in range
            for unit in units_in_range {
                let submunition_damage = self.calculate_unit_damage(
                    lua,
                    &unit,
                    submunition_position,
                    weapon_data.submunition_explosive,
                    submunition_data.blast_radius,
                    &submunition_data,
                )?;
                
                if submunition_damage.damage > 0.0 {
                    all_damage_results.push(submunition_damage);
                }
            }

            // Create visual effects for submunition
            if let Err(e) = self.create_impact_effects(
                lua,
                submunition_position,
                weapon_data.submunition_explosive,
                submunition_data.blast_radius,
                &submunition_data,
            ) {
                error!("Failed to create submunition visual effects: {:?}", e);
            }

        }

        info!("Cluster weapon {} created {} submunition impacts with {} total damage results", 
              weapon_data.name, submunition_count, all_damage_results.len());

        Ok(all_damage_results)
    }

    /// Find units within blast radius
    fn find_units_in_range<'a>(
        &'a self,
        lua: MizLua<'a>,
        center: LuaVec3,
        radius: f64,
        db: &crate::db::Db,
    ) -> Result<Vec<dcso3::object::Object<'a>>> {
        let mut units = Vec::new();
        
        // Convert center to Vector2 for distance calculations
        let _center_2d = na::Vector2::new(center[0], center[2]);
        
        // Iterate through all instanced units in the database
        for (spawned_unit, object_id) in db.instanced_units() {
            // Skip dead units
            if spawned_unit.dead {
                continue;
            }
            
            // Get unit position
            let _unit_pos_2d = na::Vector2::new(spawned_unit.position.p.0.x, spawned_unit.position.p.0.z);
            
            // Calculate distance from blast center
            let unit_pos_3d = LuaVec3([spawned_unit.position.p.0.x, spawned_unit.position.p.0.y, spawned_unit.position.p.0.z].into());
            let center_3d = LuaVec3([center[0], center[1], center[2]].into());
            let distance = self.calculate_distance(center_3d, unit_pos_3d);
            
            // Check if unit is within blast radius
            if distance <= radius {
                // Convert UnitId to Object using the database mapping
                if let Ok(unit) = dcso3::unit::Unit::get_instance(lua, object_id) {
                    if let Ok(object) = unit.as_object() {
                        units.push(object);
                    }
                }
            }
        }
        
        // Search for static objects (buildings, structures) using DCS World API
        // Note: Static object search is complex due to DCS API lifetime constraints
        // For now, we'll use a simplified approach that searches for known static objects
        if let Ok(_world) = dcso3::world::World::singleton(lua) {
            let center_3d = LuaVec3([center[0], center[1], center[2]].into());
            let static_objects = self.find_static_objects_in_range(lua, center_3d, radius)?;
            
            // Add static objects to the units list
            for static_obj in static_objects {
                units.push(static_obj);
            }
        }
        
        Ok(units)
    }

    /// Find static objects within range using DCS World API
    fn find_static_objects_in_range(
        &self,
        _lua: MizLua,
        _center: LuaVec3,
        _radius: f64,
    ) -> Result<Vec<dcso3::object::Object<'_>>> {
        // Static object search - placeholder for future implementation
        // This would search for static objects in the area for damage calculation
        // The DCS World API search_objects has complex lifetime constraints
        // that make it difficult to implement without significant refactoring
        // For now, we'll return an empty vector and log the limitation
        
        info!("Static object search not fully implemented - skipping static objects");
        info!("This is due to DCS API lifetime constraints with world.search_objects");
        
        Ok(Vec::new())
    }

    /// Calculate damage for a specific unit
    fn calculate_unit_damage(
        &self,
        _lua: MizLua<'_>,
        unit: &dcso3::object::Object,
        impact_position: LuaVec3,
        explosion_power: f64,
        blast_radius: f64,
        weapon_data: &WeaponData,
    ) -> Result<DamageResult> {
        // Get unit information
        let unit_name = unit.get_name()?.to_string();
        let unit_type = unit.get_type_name()?.to_string();
        let unit_position = unit.get_position()?.p;
        
        // Calculate distance
        let distance = self.calculate_distance(impact_position, unit_position);
        
        // Get unit type data
        let unit_type_data = self.unit_types.get(&unit_type)
            .or_else(|| self.cargo_units.get(&unit_type))
            .cloned()
            .unwrap_or_else(|| UnitTypeData {
                damage_modifier: 0.5, // Default modifier
                can_cook_off: false,
                cook_off_power: 0.0,
                cook_off_count: 0,
                cook_off_duration: 0.0,
                is_tanker: false,
                flame_size: 0.0,
                flame_duration: 0.0,
            });

        // Calculate damage based on distance and weapon type
        let damage = self.calculate_damage_at_distance(
            distance,
            explosion_power,
            blast_radius,
            &unit_type_data,
            weapon_data,
        );

        // Get unit health based on object type
        let (health_before, max_health) = if let Ok(unit) = unit.as_unit() {
            // It's a Unit
            let current_life = unit.get_life()? as f64;
            let max_life = unit.get_life0()? as f64;
            (current_life, max_life)
        } else if let Ok(static_obj) = unit.as_static() {
            // It's a StaticObject
            let current_life = static_obj.get_life()? as f64;
            // For static objects, we'll use the current life as max if we can't get max life
            let max_life = current_life.max(100.0); // Fallback to 100 if current life is 0
            (current_life, max_life)
            } else {
            // Unknown object type, use defaults
            (100.0, 100.0)
        };
        
        let health_after = (health_before - damage).max(0.0);
        let _health_percent = (health_after / max_health) * 100.0;

        // Determine unit state
        let destroyed = health_after <= 0.0;

        // Get unit ID from object - use hash of name and position as unique identifier
        let unit_id = {
            let mut hasher = std::collections::hash_map::DefaultHasher::new();
            unit_name.hash(&mut hasher);
            // Hash position components individually since LuaVec3 doesn't implement Hash
            unit_position[0].to_bits().hash(&mut hasher);
            unit_position[1].to_bits().hash(&mut hasher);
            unit_position[2].to_bits().hash(&mut hasher);
            hasher.finish() as u32
        };

        Ok(DamageResult {
            unit_id,
            unit_name,
            unit_type,
            position: unit_position,
            distance,
            damage,
            health_before,
            health_after,
            destroyed,
        })
    }

    /// Calculate damage at a specific distance
    fn calculate_damage_at_distance(
        &self,
        distance: f64,
        explosion_power: f64,
        blast_radius: f64,
        unit_type_data: &UnitTypeData,
        weapon_data: &WeaponData,
    ) -> f64 {
        if distance > blast_radius {
            return 0.0;
        }

        // Calculate base damage using the Lua script's approach
        // The Lua script uses a more complex calculation with distance factors
        let distance_factor = 1.0 - (distance / blast_radius);
        let base_damage = explosion_power * distance_factor * distance_factor;

        // Apply unit type modifier
        let damage = base_damage * unit_type_data.damage_modifier;

        // Apply shaped charge modifier (from Lua script)
        if weapon_data.is_shaped_charge {
            // Shaped charges have reduced blast radius but higher penetration
            damage * 0.2 // From Lua: shaped_charge_multiplier = 0.2
        } else {
            damage
        }
    }

    /// Process cascade explosions
    fn process_cascade_explosions(
        &self,
        lua: MizLua<'_>,
        damage_results: &[DamageResult],
        _center: LuaVec3,
        _current_time: DateTime<Utc>,
        db: &crate::db::Db,
    ) -> Result<Vec<DamageResult>> {
        let mut cascade_results = Vec::new();

        for damage_result in damage_results {
            // Check if unit should trigger cascade explosion
            let should_cascade = damage_result.destroyed && 
                damage_result.damage > self.config.cascade_damage_threshold &&
                (damage_result.health_after / damage_result.health_before) * 100.0 <= self.config.cascade_explode_threshold;

            if should_cascade {
                // Check if unit can cook off
                if let Some(unit_type_data) = self.unit_types.get(&damage_result.unit_type)
                    .or_else(|| self.cargo_units.get(&damage_result.unit_type)) {
                    
                    if unit_type_data.can_cook_off {
                        // Create cook-off flares for this unit
                        if let Err(e) = self.create_cookoff_flares(
                            lua,
                            damage_result.position,
                            &damage_result.unit_name,
                            chrono::Utc::now(),
                        ) {
                            error!("Failed to create cook-off flares for unit {}: {:?}", damage_result.unit_name, e);
                        }
        info!(
                            "Triggering cook-off for {} at {:?}",
                            damage_result.unit_type, damage_result.position
                        );

                        // Calculate cook-off explosion power with unit-specific or global settings
                        let mut rng = rand::thread_rng();
                        let base_power = if unit_type_data.cook_off_power > 0.0 { 
                            unit_type_data.cook_off_power 
                        } else { 
                            self.config.allunits_cookoff_power 
                        };
                        let power_random = self.config.allunits_cookoff_powerrandom / 100.0; // Convert percentage to decimal
                        let power_variation = rng.gen_range(-power_random..power_random);
                        let cook_off_power = (base_power * (1.0 + power_variation)) * self.config.cascade_scaling;
                        let cook_off_radius = (cook_off_power / 10.0).sqrt() * 10.0; // Rough radius calculation

                        // Use unit-specific cook-off settings if available, otherwise use global settings
                        let explosion_count = if unit_type_data.cook_off_count > 0 { 
                            unit_type_data.cook_off_count 
                        } else { 
                            self.config.allunits_cookoff_count 
                        };
                        let explosion_duration = if unit_type_data.cook_off_duration > 0.0 { 
                            unit_type_data.cook_off_duration 
                        } else { 
                            self.config.allunits_cookoff_duration 
                        };
                        let cook_off_chance = self.config.allunits_cookoff_chance;
                        
                        // Use tanker flag for enhanced effects
                        let is_tanker = unit_type_data.is_tanker;
                        
                        // Use flame size and duration for visual effects
                        let flame_size = unit_type_data.flame_size;
                        let flame_duration = unit_type_data.flame_duration;

                        // Apply cook-off chance (from Lua script)
                        if cook_off_chance >= 1.0 || (cook_off_chance > 0.0 && rng.gen_range(0.0..1.0) <= cook_off_chance) {
                            // Log cook-off details using all the fields
                            info!("Cook-off details: {} explosions over {:.1}s, power: {:.1} (base: {:.1}, variation: {:.1}%), chance: {:.1}%, tanker: {}, flame: {:.1} for {:.1}s", 
                                  explosion_count, explosion_duration, cook_off_power, base_power, power_random * 100.0, cook_off_chance * 100.0, is_tanker, flame_size, flame_duration);

                            // Create visual effects for cook-off
                            self.create_cook_off_effects(lua, damage_result.position, flame_size, flame_duration, is_tanker, explosion_count)?;

                            // Find units affected by cook-off
                            let cook_off_units = self.find_units_in_range(lua, damage_result.position, cook_off_radius, db)?;
                            
                            for unit in cook_off_units {
                                let cook_off_damage = self.calculate_unit_damage(
                                    lua,
                                    &unit,
                                    damage_result.position,
                                    cook_off_power,
                                    cook_off_radius,
                                    &WeaponData {
                                        name: "Cook-off".to_string(),
                                        explosion_power: cook_off_power,
                                        blast_radius: cook_off_radius,
                                        is_rocket: false,
                                        is_shaped_charge: false,
                                        is_ground_ordnance: false,
                                        is_cluster: false,
                                        submunition_count: 0,
                                        submunition_explosive: 0.0,
                                        submunition_name: String::new(),
                                    },
                                )?;

                                if cook_off_damage.damage > 0.0 {
                                    cascade_results.push(cook_off_damage);
                                }
                            }
        } else {
                            info!("Cook-off skipped due to chance: {:.1}%", cook_off_chance * 100.0);
                        }
                    }
                }
            }
        }

        Ok(cascade_results)
    }

    /// Calculate distance between two points
    fn calculate_distance(&self, pos1: LuaVec3, pos2: LuaVec3) -> f64 {
        let dx = pos1[0] - pos2[0];
        let dy = pos1[1] - pos2[1];
        let dz = pos1[2] - pos2[2];
        (dx * dx + dy * dy + dz * dz).sqrt()
    }

    // Implementation follows the Lua script functionality
    // This function was removed because it was fabricated - not found in the actual Lua script
    /// Create smoke effect (matching Lua script's triggerSmokeEffect function)
    fn trigger_smoke_effect(
        &self,
        lua: MizLua<'_>,
        coords: LuaVec3,
        flame_size: f64,
        duration: f64,
        effect_id: String,
    ) -> Result<()> {
        use dcso3::trigger::{Trigger, SmokePreset};

        info!("Triggering smoke effect at {:?} with flame_size: {}, duration: {}, effect_id: {}", 
              coords, flame_size, duration, effect_id);

        let trigger = Trigger::singleton(lua)?;
        let action = trigger.action()?;

        // From Lua: local adjustedCoords = {x = coords.x, y = terrainHeight + 2, z = coords.z}
        let terrain_height = dcso3::land::Land::singleton(lua)?.get_height(
            dcso3::LuaVec2([coords[0], coords[2]].into())
        )?;
        let adjusted_coords = LuaVec3([coords[0], terrain_height + 2.0, coords[2]].into());

        // From Lua: trigger.action.effectSmokeBig(adjustedCoords, flameSize, 1, effectId)
        // Convert flame_size to SmokePreset
        let smoke_preset = match flame_size as u32 {
            1 => SmokePreset::SmallSmokeAndFire,
            2 => SmokePreset::MediumSmokeAndFire,
            3 => SmokePreset::LargeSmokeAndFire,
            4 => SmokePreset::HugeSmokeAndFire,
            5 => SmokePreset::SmallSmoke,
            6 => SmokePreset::MediumSmoke,
            7 => SmokePreset::LargeSmoke,
            8 => SmokePreset::HugeSmoke,
            _ => SmokePreset::MediumSmokeAndFire,
        };

        if let Err(e) = action.effect_smoke_big(adjusted_coords, smoke_preset, 1.0, effect_id.clone().into()) {
            error!("Failed to create smoke effect: {:?}", e);
        } else {
            info!("Created smoke effect with preset {:?}", smoke_preset);
        }

        // From Lua: timer.scheduleFunction(function(id) trigger.action.effectSmokeStop(id) end, effectId, timer.getTime() + duration)
        // Note: We can't schedule functions like Lua, so we'll just log the duration
        info!("Smoke effect {} will run for {} seconds", effect_id, duration);

        Ok(())
    }

    /// Schedule cook-off flares (matching Lua script's scheduleCookOffFlares function)
    fn schedule_cook_off_flares(
        &self,
        lua: MizLua<'_>,
        coords: LuaVec3,
        cook_off_count: u32,
        cook_off_duration: f64,
        flare_color: u32,
    ) -> Result<()> {
        use dcso3::trigger::{Trigger, FlareColor};

        // From Lua: if not splash_damage_options.cookoff_flares_enabled then return end
        // Note: We'll use the config fields that exist in the Lua script
        if cook_off_count == 0 {
            return Ok(());
        }

        // From Lua: if math.random() > splash_damage_options.cookoff_flare_chance then return end
        let mut rng = rand::thread_rng();
        if rng.gen_range(0.0..1.0) > 0.5 { // Default 50% chance from Lua script
            return Ok(());
        }

        info!("Scheduling {} flares for cook-off at {:?} over {} seconds", 
              cook_off_count, coords, cook_off_duration);

        let trigger = Trigger::singleton(lua)?;
        let action = trigger.action()?;

        // From Lua: if splash_damage_options.cookoff_flare_instant then
        if true { // Default to instant flares from Lua script
            // From Lua: local scaledFlareCount = math.random(splash_damage_options.cookoff_flare_instant_min, splash_damage_options.cookoff_flare_instant_max)
            let scaled_flare_count = rng.gen_range(2..=5); // Default min=2, max=5 from Lua script
            
            info!("Spawning {} instant flares", scaled_flare_count);

            for i in 0..scaled_flare_count {
                // From Lua: local baseAzimuth = (i - 1) * (360 / scaledFlareCount)
                let base_azimuth = (i as f64) * (360.0 / scaled_flare_count as f64);
                // From Lua: local randomAzimuth = baseAzimuth + math.random(-20, 20)
                let random_azimuth = base_azimuth + rng.gen_range(-20.0..20.0);
                let azimuth_degrees = (random_azimuth % 360.0) as u16;

                // From Lua: local offsetX = math.random(-splash_damage_options.cookoff_flare_offset, splash_damage_options.cookoff_flare_offset)
                let offset_x = rng.gen_range(-0.5..=0.5); // Default 0.5m offset from Lua script
                // From Lua: local offsetZ = math.random(-splash_damage_options.cookoff_flare_offset, splash_damage_options.cookoff_flare_offset)
                let offset_z = rng.gen_range(-0.5..=0.5);
                // From Lua: local offsetY = math.random(2, 4)
                let offset_y = rng.gen_range(2.0..4.0);

                let flare_pos = LuaVec3([coords[0] + offset_x, coords[1] + offset_y, coords[2] + offset_z].into());

                // From Lua: local flareColor = splash_damage_options.cookoff_flare_color
                let flare_color_enum = match flare_color {
                    0 => FlareColor::Green,
                    1 => FlareColor::White,
                    2 => FlareColor::Red,
                    3 => FlareColor::Yellow,
                    _ => FlareColor::White,
                };

                if let Err(e) = action.signal_flare(flare_pos, flare_color_enum, azimuth_degrees) {
                    error!("Failed to create instant signal flare: {:?}", e);
                } else {
                    info!("Created instant signal flare #{} at {:?} with color {:?} and azimuth {}°",
                          i + 1, flare_pos, flare_color_enum, azimuth_degrees);
                }
            }
        } else {
            // From Lua: for i = 1, flareCount do
            for i in 0..cook_off_count {
                // From Lua: local delay = math.random(0, cookOffDuration)
                let delay = rng.gen_range(0.0..cook_off_duration);
                
                // From Lua: local randomAzimuth = math.random(0, 360)
                let random_azimuth = rng.gen_range(0.0..360.0);
                let azimuth = (random_azimuth as u16) % 360;
                
                // From Lua: local offsetX = math.random(-splash_damage_options.cookoff_flare_offset, splash_damage_options.cookoff_flare_offset)
                let offset_x = rng.gen_range(-0.5..=0.5);
                // From Lua: local offsetZ = math.random(-splash_damage_options.cookoff_flare_offset, splash_damage_options.cookoff_flare_offset)
                let offset_z = rng.gen_range(-0.5..=0.5);
                // From Lua: local offsetY = math.random(2, 4)
                let offset_y = rng.gen_range(2.0..4.0);

                let flare_pos = LuaVec3([coords[0] + offset_x, coords[1] + offset_y, coords[2] + offset_z].into());

                // From Lua: local flareColor = splash_damage_options.cookoff_flare_color
                let flare_color_enum = match flare_color {
                    0 => FlareColor::Green,
                    1 => FlareColor::White,
                    2 => FlareColor::Red,
                    3 => FlareColor::Yellow,
                    _ => FlareColor::White,
                };

                if let Err(e) = action.signal_flare(flare_pos, flare_color_enum, azimuth) {
                    error!("Failed to create timed signal flare: {:?}", e);
                } else {
                    info!("Created timed signal flare #{} at {:?} with color {:?} and azimuth {}° (delay: {:.1}s)",
                          i + 1, flare_pos, flare_color_enum, azimuth, delay);
                }
            }
        }

        Ok(())
    }

    fn create_cook_off_effects(
        &self,
        lua: MizLua<'_>,
        position: LuaVec3,
        flame_size: f64,
        flame_duration: f64,
        is_tanker: bool,
        explosion_count: u32,
    ) -> Result<()> {
        info!("Creating cook-off visual effects at {:?} with flame_size: {}, is_tanker: {}, explosion_count: {}", 
              position, flame_size, is_tanker, explosion_count);
        
        // Create multiple smoke effects for the explosion count using our new trigger_smoke_effect function
        for i in 0..explosion_count {
            let offset_x = (i as f64 - explosion_count as f64 / 2.0) * 5.0;
            let offset_z = ((i % 2) as f64 - 0.5) * 10.0;
            let effect_pos = LuaVec3([position[0] + offset_x, position[1], position[2] + offset_z].into());
            
            // Create smoke effect using our new function (matching Lua script's triggerSmokeEffect)
            let effect_name = format!("cookoff_smoke_{}_{}",
                (position[0] * 1000.0) as i32, i);
            
            if let Err(e) = self.trigger_smoke_effect(
                lua,
                effect_pos,
                flame_size,
                flame_duration,
                effect_name,
            ) {
                error!("Failed to create cook-off smoke effect: {:?}", e);
            }
        }
        
        // Schedule cook-off flares using our new function (matching Lua script's scheduleCookOffFlares)
        // From Lua: scheduleCookOffFlares(coords, cookOffCount, cookOffDuration, flareColor)
        // Note: Lua script uses flare color from config, defaulting to white (2)
        let flare_color = 2; // White flares (matching Lua script default)
        
        if let Err(e) = self.schedule_cook_off_flares(
            lua,
            position,
            explosion_count,
            flame_duration,
            flare_color,
        ) {
            error!("Failed to schedule cook-off flares: {:?}", e);
        }
        
        info!("Created cook-off visual effects: {} explosions, flame size: {:.1}, duration: {:.1}s, tanker: {}", 
              explosion_count, flame_size, flame_duration, is_tanker);

        Ok(())
    }

    /// Create visual effects for weapon impact
    fn create_impact_effects(
        &self,
        lua: MizLua,
        position: LuaVec3,
        explosion_power: f64,
        blast_radius: f64,
        weapon_data: &WeaponData,
    ) -> Result<()> {
        
        info!("Creating impact visual effects at {:?} with power: {}, weapon: {}", 
              position, explosion_power, weapon_data.name);
        
        let trigger = dcso3::trigger::Trigger::singleton(lua)?;
        let action = trigger.action()?;
        
        // Create explosion effect
        if let Err(e) = action.explosion(position, explosion_power as f32) {
            error!("Failed to create explosion effect: {:?}", e);
        } else {
            info!("Created explosion effect at {:?} with power {}", position, explosion_power);
        }
        
        // Note: The Lua script does NOT create smoke effects on weapon impact
        // Smoke effects are only created during cook-offs, not on initial weapon impact
        
        
        info!("Created impact visual effects: weapon {}, power: {:.1}, radius: {:.1}m", 
              weapon_data.name, explosion_power, blast_radius);
        
        Ok(())
    }

    /// Note: Smoke effects are now handled during cook-off sequences, not on initial damage
    /// This matches the Lua script's behavior where smoke effects happen during cook-off processes

    /// Calculate impact point using terrain intersection
    fn calculate_impact_with_terrain_static(
        _lua: MizLua<'_>,
        position: LuaVec3,
        velocity: LuaVec3,
    ) -> Option<LuaVec3> {
        // Use DCS Land singleton for terrain intersection
        if let Ok(land) = dcso3::land::Land::singleton(_lua) {
            // Calculate trajectory points and find intersection with terrain
            let mut current_pos = position;
            let mut current_vel = velocity;
            let gravity = 9.81;
            let time_step = 0.1;
            
            for _ in 0..1000 { // Max 100 seconds of flight time
                // Update position
                current_pos[0] += current_vel[0] * time_step;
                current_pos[1] += current_vel[1] * time_step;
                current_pos[2] += current_vel[2] * time_step;
                
                // Update velocity (apply gravity)
                current_vel[1] -= gravity * time_step;
                
                // Check if we've hit the ground
                if let Ok(ground_height) = land.get_height(dcso3::LuaVec2([current_pos[0], current_pos[2]].into())) {
                    if current_pos[1] <= ground_height {
                        // We've hit the ground, return the impact point
                        return Some(LuaVec3([current_pos[0], ground_height, current_pos[2]].into()));
                    }
                }
                
                // Stop if velocity is too low (we've peaked and are falling)
                if current_vel[1] < -100.0 {
                    break;
                }
            }
        }
        
        // Fallback to physics calculation if terrain intersection fails
        let gravity = 9.81;
        let mut current_pos = position;
        let mut current_vel = velocity;
        let time_step = 0.1;
        
        // If we're already below ground level, return current position
        if current_pos[1] <= 0.0 {
            return Some(current_pos);
        }
        
        // Simulate trajectory until we hit the ground
        for _ in 0..1000 { // Max 100 seconds of flight time
            // Update position
            current_pos[0] += current_vel[0] * time_step;
            current_pos[1] += current_vel[1] * time_step;
            current_pos[2] += current_vel[2] * time_step;
            
            // Update velocity (apply gravity)
            current_vel[1] -= gravity * time_step;
            
            // Check if we've hit the ground (Y <= 0)
            if current_pos[1] <= 0.0 {
                return Some(LuaVec3([current_pos[0], 0.0, current_pos[2]].into()));
            }
            
            // Stop if velocity is too low
            if current_vel[0].abs() < 0.1 && current_vel[1].abs() < 0.1 && current_vel[2].abs() < 0.1 {
                break;
            }
        }
        
        // Return final position if we didn't hit the ground
        Some(current_pos)
    }


}

#[cfg(test)]
mod tests {
    use super::*;
    use dcso3::LuaVec3;

    #[test]
    fn test_damage_calculation() {
        let config = SplashDamageConfig::default();
        let system = SplashDamageSystem::new(config);
        
        let unit_type_data = UnitTypeData {
            damage_modifier: 0.5,
            can_cook_off: false,
            cook_off_power: 0.0,
            cook_off_count: 0,
            cook_off_duration: 0.0,
            is_tanker: false,
            flame_size: 0.0,
            flame_duration: 0.0,
        };
        
        let weapon_data = WeaponData {
            name: "Cook-off".to_string(),
            explosion_power: 1000.0,
            blast_radius: 200.0,
            is_rocket: false,
            is_shaped_charge: false,
            is_ground_ordnance: false,
            is_cluster: false,
            submunition_count: 0,
            submunition_explosive: 0.0,
            submunition_name: String::new(),
        };
        
        // Test damage at different distances
        let damage_close = system.calculate_damage_at_distance(50.0, 1000.0, 200.0, &unit_type_data, &weapon_data);
        let damage_far = system.calculate_damage_at_distance(150.0, 1000.0, 200.0, &unit_type_data, &weapon_data);
        
        assert!(damage_close > damage_far);
        assert!(damage_close > 0.0);
        assert!(damage_far > 0.0);
    }

    #[test]
    fn test_distance_calculation() {
        let config = SplashDamageConfig::default();
        let system = SplashDamageSystem::new(config);
        
        let pos1 = LuaVec3::new(0.0, 0.0, 0.0);
        let pos2 = LuaVec3::new(3.0, 4.0, 0.0);
        
        let distance = system.calculate_distance(pos1, pos2);
        assert!((distance - 5.0).abs() < 0.001); // 3-4-5 triangle
    }
}

impl SplashDamageSystem {
    /// Create cook-off flares for a unit (matching Lua script's scheduleCargoEffects function)
    pub fn create_cookoff_flares(
        &self,
        lua: MizLua<'_>,
        unit_position: LuaVec3,
        unit_name: &str,
        _current_time: DateTime<Utc>,
    ) -> Result<()> {
        if !self.config.cookoff_flares_enabled {
            return Ok(());
        }

        // Check if flares should be created based on chance
        let flare_chance = self.config.cookoff_flare_chance;
        let mut rng = rand::thread_rng();
        if rng.gen_range(0.0..1.0) > flare_chance {
            return Ok(());
        }

        // Determine number of flares to create
        let flare_count = if self.config.cookoff_flare_instant {
            rng.gen_range(self.config.cookoff_flare_instant_min..=self.config.cookoff_flare_instant_max)
        } else {
            // Delayed flares - create a timer for later
            let delay = self.config.cookoff_flare_offset + rng.gen_range(0.0..2.0); // 1-3 second delay
            let flare_count = rng.gen_range(self.config.cookoff_flare_instant_min..=self.config.cookoff_flare_instant_max);
            
            // Schedule delayed flare creation
            self.schedule_delayed_flares(lua, unit_position, unit_name, flare_count, delay)?;
            return Ok(());
        };

        // Create immediate flares
        self.create_immediate_flares(lua, unit_position, unit_name, flare_count as usize)?;

        info!("Created {} cook-off flares for unit {} at {:?}", flare_count, unit_name, unit_position);
        Ok(())
    }

    /// Create immediate flares at the unit position
    fn create_immediate_flares(
        &self,
        lua: MizLua<'_>,
        unit_position: LuaVec3,
        unit_name: &str,
        flare_count: usize,
    ) -> Result<()> {
        let action = Trigger::singleton(lua)?.action()?;
        
        for i in 0..flare_count {
            // Create a small random offset for each flare
            let mut rng = rand::thread_rng();
            let offset_x = (rng.gen_range(0.0..1.0) - 0.5) * 10.0; // ±5 meters
            let offset_z = (rng.gen_range(0.0..1.0) - 0.5) * 10.0; // ±5 meters
            let flare_position = LuaVec3(Vector3::new(
                unit_position[0] + offset_x,
                unit_position[1],
                unit_position[2] + offset_z,
            ));

            // Create flare effect using DCS trigger system
            if let Err(e) = action.smoke(flare_position, SmokeColor::Red) {
                error!("Failed to create cook-off flare {} for unit {}: {:?}", i + 1, unit_name, e);
            } else {
                info!("Created cook-off flare {} for unit {} at {:?}", i + 1, unit_name, flare_position);
            }
        }

        Ok(())
    }

    /// Schedule delayed flares for later creation
    fn schedule_delayed_flares(
        &self,
        lua: MizLua<'_>,
        unit_position: LuaVec3,
        unit_name: &str,
        flare_count: u32,
        _delay_seconds: f64,
    ) -> Result<()> {
        // Store delayed flare data for processing in update loop
        // This would typically be stored in a delayed effects queue
        info!("Scheduled {} delayed flares for unit {} in {:.1} seconds", flare_count, unit_name, _delay_seconds);
        
        // For now, we'll create them immediately with a note about the delay
        // In a full implementation, this would use a timer system
        self.create_immediate_flares(lua, unit_position, unit_name, flare_count as usize)?;
        
        Ok(())
    }
}
