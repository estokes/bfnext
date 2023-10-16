extern crate nalgebra as na;
use compact_str::format_compact;
use dcso3::{
    coalition::{Coalition, Side},
    env::{
        self,
        miz::{GroupKind, MizIndex},
    },
    err,
    group::GroupCategory,
    String, Vector2, country::Country, DeepClone,
};
use immutable_chunkmap::{map::MapM as Map, set::SetM as Set};
use mlua::prelude::*;
use netidx_core::atomic_id;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

use crate::SpawnLoc;

atomic_id!(GroupId);

impl Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

atomic_id!(UnitId);

impl Display for UnitId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpawnedUnit {
    name: String,
    pos: Vector2,
    dead: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedGroup {
    name: String,
    template_name: String,
    country: Country,
    kind: GroupKind,
    units: Map<UnitId, SpawnedUnit>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Db {
    groups_by_id: Map<GroupId, SpawnedGroup>,
    groups_by_name: Map<String, GroupId>,
    groups_by_unit_id: Map<UnitId, GroupId>,
    groups_by_unit_name: Map<String, GroupId>,
    groups_by_side: Map<Side, Set<GroupId>>,
}

impl Db {
    fn respawn_group<'lua>(&self, lua: &'lua Lua, group: GroupId) -> LuaResult<()> {

    }

    pub fn spawn_template_as_new<'lua>(
        &mut self,
        lua: &'lua Lua,
        idx: &MizIndex,
        side: Side,
        kind: GroupKind,
        location: &SpawnLoc,
        template_name: &str,
    ) -> LuaResult<GroupId> {
        let template_name = String::from(template_name);
        let coalition = Coalition::singleton(lua)?;
        let miz = env::miz::Miz::singleton(lua)?;
        let mut template = miz
            .get_group(idx, kind, side, template_name.as_str())?
            .ok_or_else(|| err("no such template"))?;
        template.group = template.group.deep_clone(lua)?;
        let pos = match location {
            SpawnLoc::AtPos(pos) => *pos,
            SpawnLoc::AtTrigger { name, offset } => {
                let tz = miz
                    .get_trigger_zone(&idx, name.as_str())?
                    .ok_or_else(|| err("no such trigger zone"))?;
                tz.pos()? + offset
            }
        };
        let gid = GroupId::new();
        let group_name = String::from(format_compact!("{}-{}", template_name, gid));
        template.group.set("lateActivation", false)?;
        template.group.raw_remove("groupId")?;
        let orig_group_pos = template.group.pos()?;
        template.group.set_pos(pos)?;
        template.group.set_name(group_name.clone())?;
        let mut spawned = SpawnedGroup {
            name: group_name.clone(),
            template_name: template_name.clone(),
            country: template.country,
            kind,
            units: Map::new(),
        };
        for (i, unit) in template.group.units()?.into_iter().enumerate() {
            let uid = UnitId::new();
            let unit = unit?;
            let unit_name = String::from(format_compact!("{}-{}", group_name, uid));
            let unit_pos_offset = orig_group_pos - unit.pos()?;
            let pos = pos + unit_pos_offset;
            unit.raw_remove("unitId")?;
            unit.set_pos(pos)?;
            unit.set_name(unit_name.clone())?;
            let spawned_unit = SpawnedUnit {
                name: unit_name,
                pos,
                dead: false,
            };
            spawned.units.insert_cow(uid, spawned_unit);
            self.groups_by_unit_id.insert_cow(uid, gid);
            self.groups_by_unit_name.insert_cow(unit_name, gid);
        }
        self.groups_by_id.insert_cow(gid, spawned);
        self.groups_by_name.insert_cow(group_name, gid);
        match GroupCategory::from_kind(template.category) {
            None => coalition.add_static_object(template.country, template.group)?,
            Some(category) => coalition.add_group(template.country, category, template.group)?,
        }
        Ok(gid)
    }
}
