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

use super::{
    group::{GroupId, SpawnedGroup, SpawnedUnit, UnitId},
    objective::{Objective, ObjectiveId},
    player::Player,
    Map, Set,
};
use anyhow::{anyhow, Result};
use compact_str::CompactString;
use dcso3::{
    coalition::Side,
    net::{SlotId, Ucid},
    String,
};
use chrono::{prelude::*, Duration};
use fxhash::FxHashMap;
use serde_derive::{Deserialize, Serialize};
use smallvec::smallvec;
use std::{
    fs::{self, File},
    path::{Path, PathBuf},
};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Persisted {
    pub groups: Map<GroupId, SpawnedGroup>,
    pub units: Map<UnitId, SpawnedUnit>,
    pub groups_by_name: Map<String, GroupId>,
    pub units_by_name: Map<String, UnitId>,
    pub groups_by_side: Map<Side, Set<GroupId>>,
    pub deployed: Set<GroupId>,
    pub farps: Set<ObjectiveId>,
    pub crates: Set<GroupId>,
    pub troops: Set<GroupId>,
    pub jtacs: Set<GroupId>,
    pub ewrs: Set<GroupId>,
    #[serde(default)]
    pub actions: Set<GroupId>,
    pub objectives: Map<ObjectiveId, Objective>,
    pub objectives_by_slot: Map<SlotId, ObjectiveId>,
    pub objectives_by_name: Map<String, ObjectiveId>,
    pub objectives_by_group: Map<GroupId, ObjectiveId>,
    pub players: Map<Ucid, Player>,
    #[serde(default)]
    pub logistics_hubs: Set<ObjectiveId>,
    #[serde(default)]
    pub nukes_used: u32,
}

impl Persisted {
    pub fn save(&self, path: &Path) -> Result<()> {
        fn rotate(path: &Path) -> Result<()> {
            if path.exists() {
                let name = path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .ok_or_else(|| anyhow!("save file with no name"))?;
                use std::fmt::Write;
                let now = Utc::now();
                let mut with_ts = PathBuf::from(path);
                let mut backup = CompactString::from(name);
                write!(backup, "{}", now.timestamp()).unwrap();
                with_ts.set_file_name(backup);
                fs::rename(path, with_ts)?;
                let dir = path.parent().ok_or_else(|| anyhow!("path has no parent dir"))?;
                let mut by_age: FxHashMap<Duration, Vec<PathBuf>> = FxHashMap::default();
                for file in fs::read_dir(dir)? {
                    let file = file?;
                    let fname = match file.file_name().to_str() {
                        Some(s) => s,
                        None => continue,
                    };
                    if file.file_type()?.is_file() {
                        if let Some(ts) = fname.strip_prefix(name) {
                            if let Ok(ts) = ts.parse::<i64>() {
                                if let Some(ts) = DateTime::<Utc>::from_timestamp(ts, 0) {
                                    if now - ts > Duration::weeks(1) {
                                        to_delete.push(PathBuf::from(file.path()));
                                    }
                                }
                            }
                        }
                    }
                }
            }
            Ok(())
        }
        let mut tmp = PathBuf::from(path);
        tmp.set_extension("tmp");
        let file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&tmp)?;
        let file = zstd::stream::Encoder::new(file, 9)?.auto_finish();
        serde_json::to_writer(file, &self)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn players(&self) -> &Map<Ucid, Player> {
        &self.players
    }
}
