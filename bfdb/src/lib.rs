use arrayvec::ArrayVec;
use bfprotocols::{
    cfg::{LifeType, UnitTags, Vehicle},
    db::group::{GroupId, UnitId},
    shots::Dead,
    stats::{DetectionSource, EnId, Pos},
};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    net::{SlotId, Ucid},
    String,
};
use enumflags2::BitFlags;
use serde::{Deserialize, Serialize};
use sled::Db;
use smallvec::SmallVec;
use typed_sled::Tree;

mod db_id;

db_id!(KillId);
db_id!(RoundId);
db_id!(SortieId);

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
    total: Aggregates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sortie {
    vehicle: Vehicle,
    pos: Pos,
    takeoff: DateTime<Utc>,
    land: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Unit {
    group: GroupId,
    typ: Vehicle,
    tags: UnitTags,
    pos: Pos,
    dead: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Slot {
    id: SlotId,
    vehicle: Option<Vehicle>,
}

struct Pilots {
    pilots: Tree<Ucid, Pilot>,
    aggregates: Tree<(Ucid, Vehicle, RoundId), Aggregates>,
    by_name: Tree<String, Ucid>,
    side: Tree<(Ucid, RoundId), Side>,
    slot: Tree<(Ucid, RoundId), Slot>,
    sortie: Tree<(Ucid, RoundId, SortieId), Sortie>,
}

struct T {
    pilots: Pilots,
    campaign: Tree<(String, RoundId), u64>,
    kills: Tree<(EnId, RoundId, SortieId, KillId), Dead>,
    units: Tree<(RoundId, UnitId), Unit>,
    groups: Tree<(RoundId, GroupId, UnitId), ()>,
    detected: Tree<(RoundId, Side, EnId), BitFlags<DetectionSource>>,
}
