use crate::db::group::{GroupId, UnitId};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    net::{SlotId, Ucid},
    object::DcsOid,
    unit::ClassUnit,
    weapon::ClassWeapon,
    String,
};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Who {
    AI {
        unit: DcsOid<ClassUnit>,
        side: Side,
        gid: GroupId,
        uid: UnitId,
        ucid: Option<Ucid>, // deployed
    },
    Player {
        unit: DcsOid<ClassUnit>,
        side: Side,
        ucid: Ucid,
        slot: SlotId,
    },
}

impl Who {
    pub fn side(&self) -> &Side {
        match self {
            Self::AI { side, .. } => side,
            Self::Player { side, .. } => side,
        }
    }

    pub fn ucid(&self) -> Option<&Ucid> {
        match self {
            Self::AI { ucid, .. } => ucid.as_ref(),
            Self::Player { ucid, .. } => Some(ucid),
        }
    }

    pub fn gid(&self) -> Option<GroupId> {
        match self {
            Self::AI { gid, .. } => Some(*gid),
            Self::Player { .. } => None,
        }
    }

    pub fn uid(&self) -> Option<UnitId> {
        match self {
            Self::AI { uid, .. } => Some(*uid),
            Self::Player { .. } => None,
        }
    }

    pub fn uid_gid(&self) -> Option<(GroupId, UnitId)> {
        match self {
            Self::AI { gid, uid, .. } => Some((*gid, *uid)),
            Self::Player { .. } => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dead {
    pub victim: Who,
    pub time: DateTime<Utc>,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub weapon_name: Option<String>,
    pub weapon: Option<DcsOid<ClassWeapon>>,
    pub shooter: Who,
    pub target: Who,
    pub target_typ: String,
    pub time: DateTime<Utc>,
    pub hit: bool,
}
