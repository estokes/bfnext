use crate::db::{LifeType, Vehicle};
use dcso3::err;
use mlua::prelude::*;
use serde_derive::{Deserialize, Serialize};
use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
};

type Map<K, V> = immutable_chunkmap::map::Map<K, V, 32>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Cfg {
    pub repair_time: f32, // seconds
    pub life_types: Map<Vehicle, LifeType>,
    pub default_lives: Map<LifeType, (u8, f32)>, // lives per time (seconds)
}

impl Cfg {
    pub fn load(miz_state_path: &Path) -> LuaResult<Self> {
        let mut path = PathBuf::from(miz_state_path);
        path.set_extension("cfg");
        let file = loop {
            match File::open(&path) {
                Ok(f) => break f,
                Err(e) => match e.kind() {
                    io::ErrorKind::NotFound => {
                        let file = File::create(&path).map_err(|e| {
                            println!("could not create default config {}", e);
                            err("creating cfg")
                        })?;
                        serde_json::to_writer_pretty(file, &Cfg::default()).map_err(|e| {
                            println!("could not write default config {}", e);
                            err("writing default cfg")
                        })?;
                    }
                    e => {
                        println!("could not open config file {}", e);
                        return Err(err("opening config"));
                    }
                },
            }
        };
        let cfg: Self = serde_json::from_reader(file).map_err(|e| {
            println!("failed to decode cfg file {:?}, {:?}", path, e);
            err("cfg decode error")
        })?;
        Ok(cfg)
    }
}

impl Default for Cfg {
    fn default() -> Self {
        Self {
            repair_time: 1800.,
            default_lives: Map::from_iter([
                (LifeType::Standard, (3, 21600.)),
                (LifeType::Intercept, (4, 21600.)),
                (LifeType::Attack, (4, 21600.)),
                (LifeType::Logistics, (6, 21600.)),
                (LifeType::Recon, (6, 21600.)),
            ]),
            life_types: Map::from_iter([
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
        }
    }
}
