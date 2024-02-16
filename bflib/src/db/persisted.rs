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
use dcso3::{
    coalition::Side,
    net::{SlotId, Ucid},
    String
};
use serde_derive::{Deserialize, Serialize};
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
    pub actions: Set<GroupId>,
    pub objectives: Map<ObjectiveId, Objective>,
    pub objectives_by_slot: Map<SlotId, ObjectiveId>,
    pub objectives_by_name: Map<String, ObjectiveId>,
    pub objectives_by_group: Map<GroupId, ObjectiveId>,
    pub players: Map<Ucid, Player>,
    #[serde(default)]
    pub logistics_hubs: Set<ObjectiveId>,
}

impl Persisted {
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
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
