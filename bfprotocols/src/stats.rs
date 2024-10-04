use crate::{
    cfg::{Action, Deployable, LifeType, Troop, UnitTags, Vehicle},
    db::{
        group::{GroupId, UnitId},
        objective::{ObjectiveId, ObjectiveKind},
    },
    shots::Dead,
};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    coord::LLPos,
    net::{SlotId, Ucid},
    warehouse::LiquidType,
    String, Vector3,
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Unit {
    pub typ: Vehicle,
    pub tags: UnitTags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pos {
    pub pos: LLPos,
    pub altitude: f32,
    pub velocity: Vector3,
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
    },
    SessionEnd,
    Objective {
        id: ObjectiveId,
        pos: LLPos,
        owner: Side,
        kind: ObjectiveKind,
    },
    Capture {
        id: ObjectiveId,
        ucids: SmallVec<[Ucid; 1]>,
        side: Side,
    },
    Repair {
        id: ObjectiveId,
        ucid: Ucid,
    },
    SupplyTransfer {
        from: ObjectiveId,
        to: ObjectiveId,
        ucid: Ucid,
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
        action: Action,
    },
    DeployTroop {
        ucid: Ucid,
        pos: LLPos,
        troop: Troop,
        gid: GroupId,
    },
    DeployGroup {
        ucid: Ucid,
        pos: LLPos,
        deployable: Deployable,
        gid: GroupId,
    },
    DeployFarp {
        ucid: Ucid,
        pos: LLPos,
        deployable: Deployable,
        oid: ObjectiveId,
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
    PlayerRegister {
        name: String,
        ucid: Ucid,
        side: Side,
        initial_points: i32,
    },
    PlayerSideswitch {
        ucid: Ucid,
        side: Side,
    },
    Slot {
        ucid: Ucid,
        slot: SlotId,
        typ: Option<Unit>,
    },
    Deslot {
        ucid: Ucid,
    },
    Unit {
        id: UnitId,
        gid: GroupId,
        typ: Unit,
    },
    GroupDeleted {
        id: GroupId,
    },
    PlayerPosition {
        id: SlotId,
        pos: Pos,
    },
    UnitPosition {
        id: UnitId,
        pos: Pos,
    },
    PlayerDetected {
        id: SlotId,
        detected: bool,
    },
    UnitDetected {
        id: UnitId,
        detected: bool,
    },
    Takeoff {
        id: SlotId,
    },
    Land {
        id: SlotId,
    },
    Life {
        ucid: Ucid,
        typ: LifeType,
        n: i8,
    },
    Kill(Dead),
    Points {
        ucid: Ucid,
        points: i32,
        reason: String,
    },
    PointsTransfer {
        from: Ucid,
        to: Ucid,
        points: u32,
    },
    Bind {
        ucid: Ucid,
        id: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Stat {
    pub seq: u64,
    pub time: DateTime<Utc>,
    #[serde(flatten)]
    pub kind: StatKind,
}

static SEQ: AtomicU64 = AtomicU64::new(0);

impl Stat {
    pub fn new(kind: StatKind) -> Self {
        let time = Utc::now();
        let seq = SEQ.fetch_add(1, Ordering::Relaxed);
        Self { time, seq, kind }
    }

    pub fn setseq(seq: u64) {
        SEQ.store(seq, Ordering::Relaxed);
    }

    pub fn seq() -> u64 {
        SEQ.load(Ordering::Relaxed)
    }
}
