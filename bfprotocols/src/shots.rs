use crate::db::group::{GroupId, UnitId};
use chrono::prelude::*;
use dcso3::{
    coalition::Side, net::Ucid, object::DcsOid, unit::ClassUnit, weapon::ClassWeapon, String,
};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Who {
    AI {
        unit: DcsOid<ClassUnit>,
        gid: GroupId,
        uid: UnitId,
        ucid: Option<Ucid>
    },
    Player {
        unit: DcsOid<ClassUnit>,
        ucid: Ucid
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dead {
    pub victim: Who,
    pub victim_side: Side,
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
