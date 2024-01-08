use super::{
    objective::{ObjGroupClass, ObjectiveId},
    Db, Set,
};
use crate::{
    cfg::{Crate, Deployable, Troop, UnitTags},
    group, group_by_name, unit, unit_by_name,
};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use dcso3::{
    atomic_id, centroid2d, coalition::Side, group::GroupCategory, net::Ucid, Position3, String,
    Vector2, object::DcsOid, unit::ClassUnit,
};
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};

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

impl Db {
    pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &SpawnedGroup)> {
        self.persisted.groups.into_iter()
    }

    pub fn group(&self, id: &GroupId) -> Result<&SpawnedGroup> {
        group!(self, id)
    }

    pub fn group_center(&self, id: &GroupId) -> Result<Vector2> {
        let group = group!(self, id)?;
        Ok(centroid2d(
            group
                .units
                .into_iter()
                .filter_map(|uid| self.persisted.units.get(uid))
                .filter_map(|unit| if unit.dead { None } else { Some(unit.pos) }),
        ))
    }

    pub fn group_by_name(&self, name: &str) -> Result<&SpawnedGroup> {
        group_by_name!(self, name)
    }

    pub fn unit(&self, id: &UnitId) -> Result<&SpawnedUnit> {
        unit!(self, id)
    }

    pub fn unit_by_name(&self, name: &str) -> Result<&SpawnedUnit> {
        unit_by_name!(self, name)
    }

    pub fn instanced_units(&self) -> impl Iterator<Item = (&SpawnedUnit, &DcsOid<ClassUnit>)> {
        self.persisted
            .units
            .into_iter()
            .filter_map(|(uid, sp)| self.ephemeral.object_id_by_uid.get(uid).map(|id| (sp, id)))
    }
}
