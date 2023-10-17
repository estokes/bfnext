extern crate nalgebra as na;
use compact_str::format_compact;
use dcso3::{
    coalition::{Coalition, Side},
    env::miz::{GroupInfo, GroupKind, Miz, MizIndex, TriggerZone},
    err,
    group::GroupCategory,
    DeepClone, String, Vector2,
};
use fxhash::FxHashMap;
use immutable_chunkmap::{map::MapM as Map, set::SetM as Set};
use mlua::prelude::*;
use netidx_core::atomic_id;
use serde_derive::{Deserialize, Serialize};
use std::fmt::Display;

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
    template_name: String,
    pos: Vector2,
    dead: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedGroup {
    name: String,
    template_name: String,
    side: Side,
    kind: GroupKind,
    units: Map<UnitId, SpawnedUnit>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnLoc {
    AtPos(Vector2),
    AtTrigger { name: String, offset: Vector2 },
}

pub struct SpawnCtx<'lua> {
    coalition: Coalition<'lua>,
    miz: Miz<'lua>,
    lua: &'lua Lua,
}

impl<'lua> SpawnCtx<'lua> {
    pub fn new(lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            coalition: Coalition::singleton(lua)?,
            miz: Miz::singleton(lua)?,
            lua,
        })
    }

    fn get_template(
        &self,
        idx: &MizIndex,
        kind: GroupKind,
        side: Side,
        template_name: &str,
    ) -> LuaResult<GroupInfo> {
        let mut template = self
            .miz
            .get_group(idx, kind, side, template_name)?
            .ok_or_else(|| err("no such template"))?;
        template.group = template.group.deep_clone(self.lua)?;
        Ok(template)
    }

    fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> LuaResult<TriggerZone> {
        Ok(self
            .miz
            .get_trigger_zone(idx, name)?
            .ok_or_else(|| err("no such trigger zone"))?)
    }

    fn spawn(&self, template: GroupInfo) -> LuaResult<()> {
        match GroupCategory::from_kind(template.category) {
            None => self
                .coalition
                .add_static_object(template.country, template.group),
            Some(category) => self
                .coalition
                .add_group(template.country, category, template.group),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Db {
    #[serde(skip)]
    dirty: bool,
    groups_by_id: Map<GroupId, SpawnedGroup>,
    groups_by_name: Map<String, GroupId>,
    groups_by_unit_id: Map<UnitId, GroupId>,
    groups_by_unit_name: Map<String, GroupId>,
    groups_by_side: Map<Side, Set<GroupId>>,
}

impl Db {
    pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &SpawnedGroup)> {
        self.groups_by_id.into_iter()
    }

    pub fn respawn_group<'lua>(
        &self,
        idx: &MizIndex,
        spctx: &SpawnCtx,
        group: &SpawnedGroup,
    ) -> LuaResult<()> {
        let template =
            spctx.get_template(idx, group.kind, group.side, group.template_name.as_str())?;
        template.group.set("lateActivation", false)?;
        template.group.set_name(group.name.clone())?;
        let by_tname: FxHashMap<&str, &SpawnedUnit> = group
            .units
            .into_iter()
            .filter_map(|(_, u)| {
                if u.dead {
                    None
                } else {
                    Some((u.template_name.as_str(), u))
                }
            })
            .collect();
        let alive = {
            let units = template.group.units()?;
            let mut i = 1;
            while i as usize <= units.len() {
                let unit = units.get(i)?;
                match by_tname.get(unit.name()?.as_str()) {
                    None => units.remove(i)?,
                    Some(su) => {
                        template.group.set_pos(su.pos)?;
                        unit.set_pos(su.pos)?;
                        i += 1;
                    }
                }
            }
            units.len() > 0
        };
        if alive {
            spctx.spawn(template)
        } else {
            Ok(())
        }
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
        let mut t = self.clone();
        let spctx = SpawnCtx::new(lua)?;
        let template_name = String::from(template_name);
        let template = spctx.get_template(idx, kind, side, template_name.as_str())?;
        let pos = match location {
            SpawnLoc::AtPos(pos) => *pos,
            SpawnLoc::AtTrigger { name, offset } => {
                spctx.get_trigger_zone(idx, name.as_str())?.pos()? + offset
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
            side,
            kind,
            units: Map::new(),
        };
        for unit in template.group.units()? {
            let uid = UnitId::new();
            let unit = unit?;
            let template_name = unit.name()?;
            let unit_name = String::from(format_compact!("{}-{}", group_name, uid));
            let unit_pos_offset = orig_group_pos - unit.pos()?;
            let pos = pos + unit_pos_offset;
            unit.raw_remove("unitId")?;
            unit.set_pos(pos)?;
            unit.set_name(unit_name.clone())?;
            let spawned_unit = SpawnedUnit {
                name: unit_name.clone(),
                template_name,
                pos,
                dead: false,
            };
            spawned.units.insert_cow(uid, spawned_unit);
            t.groups_by_unit_id.insert_cow(uid, gid);
            t.groups_by_unit_name.insert_cow(unit_name, gid);
        }
        t.groups_by_id.insert_cow(gid, spawned);
        t.groups_by_name.insert_cow(group_name, gid);
        spctx.spawn(template)?;
        *self = t;
        self.dirty = true;
        Ok(gid)
    }
}
