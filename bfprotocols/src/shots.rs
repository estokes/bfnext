use crate::db::group::{GroupId, UnitId};
use chrono::prelude::*;
use dcso3::{
    coalition::Side, net::Ucid, object::DcsOid, unit::ClassUnit, weapon::ClassWeapon, String,
};
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Dead {
    pub victim: DcsOid<ClassUnit>,
    pub victim_ucid: Option<Ucid>,
    pub victim_side: Side,
    pub victim_uid: Option<UnitId>,
    pub victim_gid: Option<GroupId>,
    pub time: DateTime<Utc>,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Shot {
    pub weapon_name: Option<String>,
    pub weapon: Option<DcsOid<ClassWeapon>>,
    pub shooter: DcsOid<ClassUnit>,
    pub shooter_ucid: Ucid,
    pub shooter_uid: Option<UnitId>,
    pub shooter_gid: Option<GroupId>,
    pub target: DcsOid<ClassUnit>,
    pub target_side: Side,
    pub target_ucid: Option<Ucid>,
    pub target_uid: Option<UnitId>,
    pub target_gid: Option<GroupId>,
    pub target_typ: String,
    pub time: DateTime<Utc>,
    pub hit: bool,
}
