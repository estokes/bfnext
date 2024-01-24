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
    group::{GroupId, SpawnedGroup, SpawnedUnit, UnitId}, objective::{Objective, ObjectiveId}, player::Player, pmc::Pmc, Map, Set
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
    pub(super) groups: Map<GroupId, SpawnedGroup>,
    pub(super) units: Map<UnitId, SpawnedUnit>,
    pub(super) groups_by_name: Map<String, GroupId>,
    pub(super) units_by_name: Map<String, UnitId>,
    pub(super) groups_by_side: Map<Side, Set<GroupId>>,
    pub(super) deployed: Set<GroupId>,
    pub(super) farps: Set<ObjectiveId>,
    pub(super) crates: Set<GroupId>,
    pub(super) troops: Set<GroupId>,
    pub(super) jtacs: Set<GroupId>,
    pub(super) ewrs: Set<GroupId>,
    pub(super) objectives: Map<ObjectiveId, Objective>,
    pub(super) objectives_by_slot: Map<SlotId, ObjectiveId>,
    pub(super) objectives_by_name: Map<String, ObjectiveId>,
    pub(super) objectives_by_group: Map<GroupId, ObjectiveId>,
    pub(super) players: Map<Ucid, Player>,
    pub(super) pmcs: Map<String, Pmc>,
    #[serde(default)]
    pub(super) logistics_hubs: Set<ObjectiveId>,
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
        serde_json::to_writer(file, &self)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn players(&self) -> &Map<Ucid, Player> {
        &self.players
    }
}
