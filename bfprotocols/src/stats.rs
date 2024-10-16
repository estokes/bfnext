use crate::{
    cfg::{Cfg, LifeType, UnitTags, Vehicle},
    db::{
        group::{GroupId, UnitId},
        objective::{ObjectiveId, ObjectiveKind},
    },
    perf::PerfInner,
    shots::Dead,
};
use chrono::prelude::*;
use dcso3::{
    atomic_id,
    coalition::Side,
    coord::LLPos,
    net::{SlotId, Ucid},
    perf::{HistogramSer, PerfInner as ApiPerfInner},
    warehouse::LiquidType,
    String, Vector3,
};
use enumflags2::bitflags;
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{
    fmt,
    sync::atomic::{AtomicU64, Ordering},
};

atomic_id!(SeqId);

pub type MapS<K, V> = immutable_chunkmap::map::Map<K, V, 16>;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EnId {
    Player(Ucid),
    Unit(UnitId),
}

impl fmt::Display for EnId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub typ: Vehicle,
    pub tags: UnitTags,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Pos {
    pub pos: LLPos,
    pub velocity: Vector3,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[bitflags]
#[repr(u8)]
pub enum DetectionSource {
    EWR,
    Jtac,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StatKind {
    NewRound {
        sortie: String,
    },
    RoundEnd {
        winner: Option<Side>,
    },
    SessionStart {
        stop: Option<DateTime<Utc>>,
        cfg: Box<Cfg>,
    },
    SessionEnd {
        api_perf: ApiPerfInner,
        perf: PerfInner,
        frame: HistogramSer,
    },
    Objective {
        name: String,
        id: ObjectiveId,
        pos: LLPos,
        owner: Side,
        kind: ObjectiveKind,
    },
    Capture {
        id: ObjectiveId,
        by: SmallVec<[Ucid; 1]>,
        side: Side,
    },
    Repair {
        id: ObjectiveId,
        by: Ucid,
    },
    SupplyTransfer {
        from: ObjectiveId,
        to: ObjectiveId,
        by: Ucid,
    },
    EquipmentInventory {
        id: ObjectiveId,
        item: String,
        amount: u32,
    },
    LiquidInventory {
        id: ObjectiveId,
        item: LiquidType,
        amount: u32,
    },
    Action {
        by: Ucid,
        gid: Option<GroupId>,
        action: String,
    },
    DeployTroop {
        by: Ucid,
        pos: LLPos,
        troop: String,
        gid: GroupId,
    },
    DeployGroup {
        by: Ucid,
        pos: LLPos,
        gid: GroupId,
        deployable: String,
    },
    DeployFarp {
        by: Ucid,
        pos: LLPos,
        oid: ObjectiveId,
        deployable: String,
    },
    ObjectiveHealth {
        id: ObjectiveId,
        last_change: DateTime<Utc>,
        health: u8,
        logi: u8,
    },
    ObjectiveSupply {
        id: ObjectiveId,
        supply: u8,
        fuel: u8,
    },
    ObjectiveDestroyed {
        id: ObjectiveId,
    },
    Register {
        name: String,
        id: Ucid,
        side: Side,
        initial_points: i32,
    },
    Sideswitch {
        id: Ucid,
        side: Side,
    },
    Connect {
        id: Ucid,
        addr: String,
        name: String,
    },
    Disconnect {
        id: Ucid,
    },
    Slot {
        id: Ucid,
        slot: SlotId,
        typ: Option<Unit>,
    },
    Deslot {
        id: Ucid,
    },
    Unit {
        id: EnId,
        gid: Option<GroupId>,
        owner: Side,
        typ: Unit,
        pos: Pos,
    },
    GroupDeleted {
        id: GroupId,
    },
    Position {
        id: EnId,
        pos: Pos,
    },
    Detected {
        id: EnId,
        detected: bool,
        source: DetectionSource,
    },
    Takeoff {
        id: Ucid,
    },
    Land {
        id: Ucid,
    },
    Life {
        id: Ucid,
        lives: MapS<LifeType, (DateTime<Utc>, u8)>,
    },
    Kill(Dead),
    Points {
        id: Ucid,
        points: i32,
        reason: String,
    },
    PointsTransfer {
        from: Ucid,
        to: Ucid,
        points: u32,
    },
    Bind {
        id: Ucid,
        token: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stat {
    pub seq: SeqId,
    pub time: DateTime<Utc>,
    #[serde(flatten)]
    pub kind: StatKind,
}

impl Stat {
    pub fn new(kind: StatKind) -> Self {
        let time = Utc::now();
        Self {
            time,
            seq: SeqId::new(),
            kind,
        }
    }
}
