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
    typ: Vehicle,
    tags: UnitTags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pos {
    pos: LLPos,
    altitude: f32,
    velocity: Vector3,
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
    Inventory {
        id: ObjectiveId,
        equipment: Vec<(String, u32)>,
        liquids: Vec<(LiquidType, u32)>,
    },
    Action {
        by: Ucid,
        action: Action,
    },
    Deploy {
        ucid: Ucid,
        pos: LLPos,
        deployable: Deployable,
        gid: GroupId,
    },
    Troop {
        ucid: Ucid,
        pos: LLPos,
        troop: Troop,
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
        initial_points: usize,
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
    Kill {
        shots: Dead,
        team_kill: bool,
    },
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
