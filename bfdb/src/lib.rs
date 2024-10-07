use arrayvec::ArrayVec;
use bfprotocols::{cfg::{LifeType, UnitTags, Vehicle}, db::group::{GroupId, UnitId}, shots::Dead, stats::Pos};
use dcso3::{coalition::Side, net::Ucid, String};
use smallvec::SmallVec;
use sled::Db;
use typed_sled::Tree;
use chrono::prelude::*;
use serde::{Serialize, Deserialize};

mod atomic_id;

atomic_id!(KillId);
atomic_id!(RoundId);
atomic_id!(SortieId);

// lives: SmallVec<[(LifeType, DateTime<Utc>, u8); 6]>,

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Aggregates {
    air_kills: u32,
    ground_kills: u32,
    captures: u32,
    repairs: u32,
    troops: u32,
    deploys: u32,
    actions: u32,
    deaths: u32,
    hours: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pilot {
    name: ArrayVec<String, 8>,
    side: ArrayVec<(RoundId, Side), 8>,
    total: Aggregates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sortie {
    takeoff: DateTime<Utc>,
    land: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Unit {
    group: GroupId,
    typ: Vehicle,
    tags: UnitTags,
    pos: Pos,
    dead: bool,
}

struct T {
    campaign: Tree<(String, RoundId), ()>,
    pilots: Tree<Ucid, Pilot>,
    pilot_aggregates: Tree<(Ucid, RoundId, Vehicle), Aggregates>,
    pilot_by_name: Tree<String, Ucid>,
    pilot_side: Tree<(Ucid, RoundId), Side>,
    slotted: Tree<(Ucid, RoundId), Vehicle>,
    sortie: Tree<(Ucid, RoundId, SortieId), Sortie>,
    kills: Tree<(Ucid, RoundId, SortieId, KillId), Dead>,
    units: Tree<(RoundId, UnitId), Unit>,
    groups: Tree<(RoundId, GroupId, UnitId), ()>,
}
