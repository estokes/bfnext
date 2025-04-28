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
    group::{SpawnedGroup, SpawnedUnit},
    objective::Objective,
    player::Player,
    Map, MapM, MapS, Set, SetM, SetS,
};
use bfprotocols::db::{
    group::{GroupId, UnitId},
    objective::ObjectiveId,
};
use dcso3::{coalition::Side, net::Ucid, String};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Persisted {
    pub groups: Map<GroupId, SpawnedGroup>,
    pub units: Map<UnitId, SpawnedUnit>,
    pub groups_by_name: Map<String, GroupId>,
    pub units_by_name: Map<String, UnitId>,
    pub groups_by_side: MapS<Side, Set<GroupId>>,
    pub deployed: SetM<GroupId>,
    pub farps: SetS<ObjectiveId>,
    pub crates: SetM<GroupId>,
    pub troops: SetM<GroupId>,
    pub jtacs: SetM<GroupId>,
    pub ewrs: SetS<GroupId>,
    #[serde(default)]
    pub actions: SetS<GroupId>,
    pub objectives: MapM<ObjectiveId, Objective>,
    pub objectives_by_name: MapM<String, ObjectiveId>,
    pub objectives_by_group: MapM<GroupId, ObjectiveId>,
    pub players: Map<Ucid, Player>,
    #[serde(default)]
    pub logistics_hubs: SetS<ObjectiveId>,
    #[serde(default)]
    pub nukes_used: u32,
    #[serde(default)]
    pub logistics_ticks_since_delivery: u32,
    #[serde(default)]
    pub oid: i64,
    #[serde(default)]
    pub gid: i64,
    #[serde(default)]
    pub uid: i64,
    #[serde(default)]
    pub migrated_obj_group_live: bool,
}

impl Persisted {
    pub fn players(&self) -> &Map<Ucid, Player> {
        &self.players
    }
}
