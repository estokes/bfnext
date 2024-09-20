use crate::{
    cfg::{Action, Deployable, Troop, Vehicle},
    db::objective::{ObjectiveId, ObjectiveKind},
    shots::Dead,
};
use chrono::prelude::*;
use dcso3::{
    coalition::Side, coord::LLPos, net::{SlotId, Ucid}, String, Vector3
};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum StatKind {
    NewRound,
    RoundEnd {
        winner: Option<Side>,
    },
    SessionStart {
        stop_time: DateTime<Utc>,
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
        ucid: Ucid,
        side: Side,
        points: usize,
    },
    Repair {
        id: ObjectiveId,
        ucid: Ucid,
        points: usize,
    },
    Action {
        by: Ucid,
        action: Action,
    },
    Deploy {
        ucid: Ucid,
        pos: LLPos,
        deployable: Deployable,
    },
    Troop {
        ucid: Ucid,
        pos: LLPos,
        troop: Troop,
    },
    ObjectiveStatus {
        id: ObjectiveId,
        health: u8,
        logi: u8,
        supply: u8,
        fuel: u8,
    },
    PlayerRegister {
        name: String,
        ucid: Ucid,
        side: Side,
        points: usize,
    },
    PlayerSideswitch {
        ucid: Ucid,
        side: Side,
    },
    Slot {
        ucid: Ucid,
        slot: SlotId,
        aircraft: Option<Vehicle>,
    },
    Position {
        ucid: Ucid,
        pos: LLPos,
        altitude: f32,
        velocity: Vector3,
    },
    Takeoff {
        ucid: Ucid,
        aircraft: Vehicle,
    },
    Land {
        ucid: Ucid,
        life_returned: bool,
    },
    Kill {
        shots: Dead,
        team_kill: bool,
        points: SmallVec<[(Ucid, usize); 2]>,
    },
    Bind {
        ucid: Ucid,
        id: String,
    }
}
