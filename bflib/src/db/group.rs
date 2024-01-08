use super::{
    objective::{ObjGroupClass, ObjectiveId},
    Set,
};
use crate::cfg::{Crate, Deployable, Troop, UnitTags};
use chrono::prelude::*;
use dcso3::{
    atomic_id, coalition::Side, group::GroupCategory, net::Ucid, Position3, String, Vector2,
};
use serde_derive::{Deserialize, Serialize};
use mlua::{prelude::*, Value};

atomic_id!(GroupId);
atomic_id!(UnitId);

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DeployKind {
    Objective,
    Deployed {
        player: Ucid,
        spec: Deployable,
    },
    Troop {
        player: Ucid,
        spec: Troop,
    },
    Crate {
        origin: ObjectiveId,
        player: Ucid,
        spec: Crate,
    },
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpawnedUnit {
    pub name: String,
    pub id: UnitId,
    pub group: GroupId,
    pub side: Side,
    pub typ: String,
    pub tags: UnitTags,
    pub template_name: String,
    pub spawn_pos: Vector2,
    pub spawn_heading: f64,
    pub spawn_position: Position3,
    pub pos: Vector2,
    pub heading: f64,
    pub position: Position3,
    pub dead: bool,
    #[serde(skip)]
    pub moved: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedGroup {
    pub id: GroupId,
    pub name: String,
    pub template_name: String,
    pub side: Side,
    pub kind: Option<GroupCategory>,
    pub class: ObjGroupClass,
    pub origin: DeployKind,
    pub units: Set<UnitId>,
}
