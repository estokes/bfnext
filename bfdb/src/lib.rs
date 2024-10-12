use anyhow::Result;
use arrayvec::ArrayVec;
use bfprotocols::{
    cfg::{LifeType, UnitTags, Vehicle},
    db::{
        group::{GroupId, UnitId},
        objective::{ObjectiveId, ObjectiveKind},
    },
    shots::Dead,
    stats::{DetectionSource, EnId, Pos, SeqId, Stat},
};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    coord::LLPos,
    net::{SlotId, Ucid},
    String,
};
use enumflags2::BitFlags;
use serde::{Deserialize, Serialize};
use sled::Db;
use sled_typed::{Prefix, Tree};
use smallvec::SmallVec;

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

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Objective {
    name: String,
    pos: LLPos,
    typ: ObjectiveKind,
    health: u8,
    logi: u8,
    supply: u8,
    fuel: u8,
}

struct Pilots {
    pilots: Tree<Ucid, Pilot>,
    aggregates: Tree<(Ucid, Vehicle, RoundId), Aggregates>,
    by_name: Tree<String, Ucid>,
    side: Tree<(Ucid, RoundId), Side>,
    slot: Tree<(Ucid, RoundId), Slot>,
    sortie: Tree<(Ucid, RoundId, SortieId), Sortie>,
    lives: Tree<(Ucid, RoundId), SmallVec<[(LifeType, DateTime<Utc>, u8); 5]>>,
}

impl Pilots {
    fn new(db: &Db) -> Self {
        Self {
            pilots: Tree::open(db, "pilots"),
            aggregates: Tree::open(db, "aggregates"),
            by_name: Tree::open(db, "by_name"),
            side: Tree::open(db, "side"),
            slot: Tree::open(db, "slot"),
            sortie: Tree::open(db, "sortie"),
            lives: Tree::open(db, "lives"),
        }
    }
}

struct StatsDb {
    db: Db,
    pilots: Pilots,
    campaign: Tree<(String, RoundId), SeqId>,
    kills: Tree<(EnId, RoundId, SortieId, KillId), Dead>,
    units: Tree<(RoundId, UnitId), Unit>,
    groups: Tree<(RoundId, GroupId), SmallVec<[UnitId; 16]>>,
    detected: Tree<(RoundId, Side, EnId), BitFlags<DetectionSource>>,
    objectives: Tree<(RoundId, ObjectiveId), Objective>,
}

impl StatsDb {
    fn new(db: &Db) -> Self {
        Self {
            db: db.clone(),
            pilots: Pilots::new(db),
            campaign: Tree::open(db, "campaign"),
            kills: Tree::open(db, "kills"),
            units: Tree::open(db, "units"),
            groups: Tree::open(db, "groups"),
            detected: Tree::open(db, "detected"),
            objectives: Tree::open(db, "objectives"),
        }
    }

    fn get_seqs(&self, sortie: String) -> Result<(RoundId, SeqId)> {
        match self
            .campaign
            .scan_prefix(&sortie)?
            .next_back()
            .transpose()?
        {
            Some(((_, round), seq)) => Ok((round, seq)),
            None => {
                let rid = RoundId::new(&self.db)?;
                let sid = SeqId::zero();
                self.campaign.insert(&(sortie, rid), &sid)?;
                Ok((rid, sid))
            }
        }
    }

    fn add_stat(&self, stat: &Stat) -> Result<RoundId> {
        unimplemented!()
    }
}
