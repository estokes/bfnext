extern crate nalgebra as na;
use dcso3::{String, Vector2};
use immutable_chunkmap::map::MapM as Map;
use netidx_core::atomic_id;
use serde_derive::{Deserialize, Serialize};

atomic_id!(GroupId);
atomic_id!(UnitId);

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpawnedUnit {
    name: String,
    pos: Vector2,
    dead: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpawnedGroup {
    name: String,
    units: Map<UnitId, SpawnedUnit>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Db {
    groups_by_id: Map<GroupId, SpawnedGroup>,
    groups_by_name: Map<String, GroupId>,
    groups_by_unit_id: Map<UnitId, GroupId>,
    groups_by_unit_name: Map<String, GroupId>,
}
