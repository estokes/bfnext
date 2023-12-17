extern crate nalgebra as na;
use self::cargo::Cargo;
use crate::{
    cfg::{Cfg, Crate, Deployable, LifeType, Troop, Vehicle},
    spawnctx::{Despawn, SpawnCtx, SpawnLoc},
};
use anyhow::{anyhow, bail, Result};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{
    atomic_id,
    coalition::Side,
    cvt_err,
    env::miz::{GroupInfo, GroupKind, Miz, MizIndex, UnitInfo},
    group::GroupCategory,
    net::{SlotId, Ucid},
    unit::Unit,
    MizLua, Position3, String, Vector2,
};
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::{
    collections::hash_map::Entry,
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
};

pub mod cargo;
pub mod mizinit;
pub mod objective;
pub mod player;

type Map<K, V> = immutable_chunkmap::map::Map<K, V, 32>;
type Set<K> = immutable_chunkmap::set::Set<K, 32>;

atomic_id!(GroupId);
atomic_id!(UnitId);
atomic_id!(ObjectiveId);

#[macro_export]
macro_rules! unit {
    ($t:expr, $id:expr) => {
        $t.persisted
            .units
            .get(&$id)
            .ok_or_else(|| anyhow!("no such unit {:?}", $id))
    };
}

#[macro_export]
macro_rules! unit_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .units
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such unit {:?}", $id))
    };
}

#[macro_export]
macro_rules! unit_by_name {
    ($t:expr, $name:expr) => {
        $t.persisted
            .units_by_name
            .get($name)
            .and_then(|id| $t.persisted.units.get(id))
            .ok_or_else(|| anyhow!("no such unit {}", $name))
    };
}

#[macro_export]
macro_rules! group {
    ($t:expr, $id:expr) => {
        $t.persisted
            .groups
            .get(&$id)
            .ok_or_else(|| anyhow!("no such group {:?}", $id))
    };
}

#[macro_export]
macro_rules! group_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .groups
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such group {:?}", $id))
    };
}

#[macro_export]
macro_rules! group_by_name {
    ($t:expr, $name:expr) => {
        $t.persisted
            .groups_by_name
            .get($name)
            .and_then(|id| $t.persisted.groups.get(id))
            .ok_or_else(|| anyhow!("no such group {}", $name))
    };
}

#[macro_export]
macro_rules! objective {
    ($t:expr, $id:expr) => {
        $t.persisted
            .objectives
            .get(&$id)
            .ok_or_else(|| anyhow!("no such objective {:?}", $id))
    };
}

#[macro_export]
macro_rules! objective_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .objectives
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such objective {:?}", $id))
    };
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DeployKind {
    Objective,
    Deployed(Deployable),
    Troop(Troop),
    Crate(ObjectiveId, Crate),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpawnedUnit {
    pub name: String,
    pub id: UnitId,
    pub group: GroupId,
    pub template_name: String,
    pub pos: Vector2,
    pub dead: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SpawnedGroup {
    pub id: GroupId,
    pub name: String,
    pub template_name: String,
    pub side: Side,
    pub kind: Option<GroupCategory>,
    pub origin: DeployKind,
    pub units: Set<UnitId>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ObjectiveKind {
    Airbase,
    Fob,
    Fuelbase,
    Samsite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ObjGroupClass {
    Logi,
    Aaa,
    Lr,
    Sr,
    Armor,
    Other,
}

impl From<&str> for ObjGroupClass {
    fn from(value: &str) -> Self {
        match value {
            "BLOGI" | "RLOGI" | "NLOGI" => ObjGroupClass::Logi,
            s => {
                if s.starts_with("BAAA") || s.starts_with("RAAA") || s.starts_with("NAAA") {
                    ObjGroupClass::Aaa
                } else if s.starts_with("BLR") || s.starts_with("RLR") || s.starts_with("NLR") {
                    ObjGroupClass::Lr
                } else if s.starts_with("BSR") || s.starts_with("RSR") || s.starts_with("NSR") {
                    ObjGroupClass::Sr
                } else if s.starts_with("BARMOR")
                    || s.starts_with("RARMOR")
                    || s.starts_with("NARMOR")
                {
                    ObjGroupClass::Armor
                } else {
                    ObjGroupClass::Other
                }
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ObjGroup(String);

impl FromStr for ObjGroup {
    type Err = LuaError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self(String::from(s)))
    }
}

impl<'lua> FromLua<'lua> for ObjGroup {
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::String(s) => s.to_str()?.parse(),
            _ => Err(cvt_err("ObjGroup")),
        }
    }
}

impl ObjGroup {
    fn template(&self) -> &str {
        match self.0.rsplit_once("-") {
            Some((l, _)) => l,
            None => self.0.as_str(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Objective {
    id: ObjectiveId,
    spawned: bool,
    trigger_name: String,
    name: String,
    pos: Vector2,
    radius: f64,
    owner: Side,
    kind: ObjectiveKind,
    slots: Map<SlotId, Vehicle>,
    groups: Map<Side, Map<ObjGroup, GroupId>>,
    health: u8,
    logi: u8,
    last_change_ts: DateTime<Utc>,
}

impl Objective {
    pub fn is_in_circle(&self, pos: Vector2) -> bool {
        na::distance(&self.pos.into(), &pos.into()) <= self.radius
    }
}

impl Objective {
    pub fn health(&self) -> u8 {
        self.health
    }

    pub fn logi(&self) -> u8 {
        self.logi
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Player {
    name: String,
    side: Side,
    side_switches: Option<u8>,
    lives: Map<LifeType, (DateTime<Utc>, u8)>,
    #[serde(skip)]
    current_slot: Option<SlotId>,
}

impl Player {
    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn lives(&self) -> &Map<LifeType, (DateTime<Utc>, u8)> {
        &self.lives
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Persisted {
    groups: Map<GroupId, SpawnedGroup>,
    units: Map<UnitId, SpawnedUnit>,
    groups_by_name: Map<String, GroupId>,
    units_by_name: Map<String, UnitId>,
    groups_by_side: Map<Side, Set<GroupId>>,
    deployed: Set<GroupId>,
    crates: Set<GroupId>,
    troops: Set<GroupId>,
    objectives: Map<ObjectiveId, Objective>,
    objectives_by_slot: Map<SlotId, ObjectiveId>,
    objectives_by_name: Map<String, ObjectiveId>,
    objectives_by_group: Map<GroupId, ObjectiveId>,
    players: Map<Ucid, Player>,
}

impl Persisted {
    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let mut tmp = PathBuf::from(path);
        tmp.set_extension("tmp");
        let file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&tmp)?;
        serde_json::to_writer(file, &self)?;
        fs::rename(tmp, path)?;
        Ok(())
    }
}

#[derive(Debug, Default)]
struct DeployableIndex {
    deployables_by_name: FxHashMap<String, Deployable>,
    deployables_by_crates: FxHashMap<String, String>,
    crates_by_name: FxHashMap<String, Crate>,
}

#[derive(Debug, Default)]
struct Ephemeral {
    dirty: bool,
    cfg: Cfg,
    players_by_slot: FxHashMap<SlotId, Ucid>,
    cargo: FxHashMap<SlotId, Cargo>,
    deployable_idx: FxHashMap<Side, DeployableIndex>,
}

impl Ephemeral {
    fn set_cfg(&mut self, miz: &Miz, mizidx: &MizIndex, cfg: Cfg) -> Result<()> {
        for (side, template) in cfg.crate_template.iter() {
            miz.get_group_by_name(mizidx, GroupKind::Any, *side, template)?
                .ok_or_else(|| anyhow!("missing crate template {:?} {template}", side))?;
        }
        for (side, deployables) in cfg.deployables.iter() {
            let idx = self.deployable_idx.entry(*side).or_default();
            for dep in deployables.iter() {
                miz.get_group_by_name(mizidx, GroupKind::Any, *side, &dep.template)?
                    .ok_or_else(|| anyhow!("missing deployable template {:?} {:?}", side, dep))?;
                let name = match dep.path.last() {
                    None => bail!("deployable with empty path {:?}", dep),
                    Some(name) => name,
                };
                match idx.deployables_by_name.entry(name.clone()) {
                    Entry::Occupied(_) => bail!("deployable with duplicate name {name}"),
                    Entry::Vacant(e) => e.insert(dep.clone()),
                };
                for cr in dep.crates.iter() {
                    match idx.deployables_by_crates.entry(cr.name.clone()) {
                        Entry::Occupied(_) => bail!("multiple deployables use crate {}", cr.name),
                        Entry::Vacant(e) => e.insert(name.clone()),
                    };
                }
                for c in dep.crates.iter() {
                    match idx.crates_by_name.entry(c.name.clone()) {
                        Entry::Occupied(_) => bail!("duplicate crate name {}", c.name),
                        Entry::Vacant(e) => e.insert(c.clone()),
                    };
                }
            }
        }
        self.cfg = cfg;
        Ok(())
    }
}

#[derive(Debug, Default)]
pub struct Db {
    persisted: Persisted,
    ephemeral: Ephemeral,
}

impl Db {
    pub fn cfg(&self) -> &Cfg {
        &self.ephemeral.cfg
    }

    pub fn load(miz: &Miz, idx: &MizIndex, path: &Path) -> Result<Self> {
        let file = File::open(&path)
            .map_err(|e| anyhow!("failed to open save file {:?}, {:?}", path, e))?;
        let persisted: Persisted = serde_json::from_reader(file)
            .map_err(|e| anyhow!("failed to decode save file {:?}, {:?}", path, e))?;
        let mut db = Db {
            persisted,
            ephemeral: Ephemeral::default(),
        };
        db.ephemeral.set_cfg(miz, idx, Cfg::load(path)?)?;
        Ok(db)
    }

    pub fn maybe_snapshot(&mut self) -> Option<Persisted> {
        if self.ephemeral.dirty {
            self.ephemeral.dirty = false;
            Some(self.persisted.clone())
        } else {
            None
        }
    }

    pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &SpawnedGroup)> {
        self.persisted.groups.into_iter()
    }

    pub fn group(&self, id: &GroupId) -> Result<&SpawnedGroup> {
        group!(self, id)
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

    pub fn player_in_slot(&self, slot: &SlotId) -> Option<&Ucid> {
        self.ephemeral.players_by_slot.get(&slot)
    }

    pub fn player(&self, ucid: &Ucid) -> Option<&Player> {
        self.persisted.players.get(ucid)
    }

    pub fn respawn_group<'lua>(
        &self,
        idx: &MizIndex,
        spctx: &SpawnCtx,
        group: &SpawnedGroup,
    ) -> Result<()> {
        let template = spctx.get_template(
            idx,
            GroupKind::Any,
            group.side,
            group.template_name.as_str(),
        )?;
        template.group.set("lateActivation", false)?;
        template.group.set_name(group.name.clone())?;
        let by_tname: FxHashMap<&str, &SpawnedUnit> = group
            .units
            .into_iter()
            .filter_map(|uid| {
                self.persisted.units.get(uid).and_then(|u| {
                    if u.dead {
                        None
                    } else {
                        Some((u.template_name.as_str(), u))
                    }
                })
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
                        unit.set_name(su.name.clone())?;
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

    pub fn delete_group<'lua>(&mut self, spctx: &'lua SpawnCtx<'lua>, gid: &GroupId) -> Result<()> {
        let group = self
            .persisted
            .groups
            .remove_cow(gid)
            .ok_or_else(|| anyhow!("no such group {:?}", gid))?;
        self.persisted.groups_by_name.remove_cow(&group.name);
        self.persisted
            .groups_by_side
            .get_mut_cow(&group.side)
            .map(|m| m.remove_cow(gid));
        match &group.origin {
            DeployKind::Objective => (),
            DeployKind::Crate(_, _) => {
                self.persisted.crates.remove_cow(gid);
            }
            DeployKind::Deployed(_) => {
                self.persisted.deployed.remove_cow(gid);
            }
            DeployKind::Troop(_) => {
                self.persisted.troops.remove_cow(gid);
            }
        }
        let mut units: SmallVec<[String; 16]> = smallvec![];
        for uid in &group.units {
            if let Some(unit) = self.persisted.units.remove_cow(uid) {
                self.persisted.units_by_name.remove_cow(&unit.name);
                units.push(unit.name);
            }
        }
        self.ephemeral.dirty = true;
        match group.kind {
            None => {
                // it's a static, we have to get it's units
                for unit in &units {
                    spctx.despawn(Despawn::Static(&*unit))?
                }
            }
            Some(_) => {
                // it's a normal group
                spctx.despawn(Despawn::Group(&*group.name))?
            }
        }
        Ok(())
    }

    /// add the units to the db, but don't actually spawn them
    fn init_template<'lua>(
        &mut self,
        spctx: &'lua SpawnCtx<'lua>,
        idx: &MizIndex,
        side: Side,
        location: SpawnLoc,
        template_name: &str,
        origin: DeployKind,
    ) -> Result<(GroupId, GroupInfo<'lua>)> {
        let template_name = String::from(template_name);
        let template = spctx.get_template(idx, GroupKind::Any, side, template_name.as_str())?;
        let kind = GroupCategory::from_kind(template.category);
        let (pos, pos_by_typ) = match location {
            SpawnLoc::AtPos(pos) => (pos, FxHashMap::default()),
            SpawnLoc::AtPosWithComponents(pos, tbl) => (pos, tbl),
            SpawnLoc::AtTrigger { name, offset } => (
                spctx.get_trigger_zone(idx, name.as_str())?.pos()? + offset,
                FxHashMap::default(),
            ),
        };
        let gid = GroupId::new();
        let group_name = String::from(format_compact!("{}-{}", template_name, gid));
        template.group.set("lateActivation", false)?;
        template.group.raw_remove("groupId")?;
        let orig_group_pos = template.group.pos()?;
        let orig_pos_by_typ: FxHashMap<String, Vector2> = if pos_by_typ.is_empty() {
            FxHashMap::default()
        } else {
            let mut tbl = FxHashMap::default();
            for unit in template.group.units()? {
                let unit = unit?;
                let pos = unit.pos()?;
                let typ = unit.typ()?;
                if pos_by_typ.contains_key(&**typ) {
                    let (n, v) = tbl
                        .entry(typ.clone())
                        .or_insert_with(|| (0, Vector2::new(0., 0.)));
                    *v += pos;
                    *n += 1;
                }
            }
            tbl.into_iter()
                .map(|(k, (n, v))| (k, v / (n as f64)))
                .collect()
        };
        template.group.set_pos(pos)?;
        template.group.set_name(group_name.clone())?;
        let mut spawned = SpawnedGroup {
            id: gid,
            name: group_name.clone(),
            template_name: template_name.clone(),
            side,
            kind,
            origin,
            units: Set::new(),
        };
        for unit in template.group.units()? {
            let uid = UnitId::new();
            let unit = unit?;
            let template_name = unit.name()?;
            let unit_name = String::from(format_compact!("{}-{}", group_name, uid));
            let unit_typ = unit.typ()?;
            let orig_group_pos = match orig_pos_by_typ.get(&unit_typ) {
                None => orig_group_pos,
                Some(pos) => *pos,
            };
            let unit_pos_offset = orig_group_pos - unit.pos()?;
            let pos = match pos_by_typ.get(&unit_typ) {
                None => pos + unit_pos_offset,
                Some(pos) => pos + unit_pos_offset,
            };
            unit.raw_remove("unitId")?;
            unit.set_pos(pos)?;
            unit.set_name(unit_name.clone())?;
            let spawned_unit = SpawnedUnit {
                id: uid,
                group: gid,
                name: unit_name.clone(),
                template_name,
                pos,
                dead: false,
            };
            spawned.units.insert_cow(uid);
            self.persisted.units.insert_cow(uid, spawned_unit);
            self.persisted.units_by_name.insert_cow(unit_name, uid);
        }
        match &spawned.origin {
            DeployKind::Objective => (),
            DeployKind::Crate(_, _) => {
                self.persisted.crates.insert_cow(gid);
            }
            DeployKind::Deployed(_) => {
                self.persisted.deployed.insert_cow(gid);
            }
            DeployKind::Troop(_) => {
                self.persisted.troops.insert_cow(gid);
            }
        }
        self.persisted.groups.insert_cow(gid, spawned);
        self.persisted.groups_by_name.insert_cow(group_name, gid);
        self.persisted
            .groups_by_side
            .get_or_default_cow(side)
            .insert_cow(gid);
        Ok((gid, template))
    }

    pub fn spawn_template_as_new<'lua>(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        side: Side,
        location: SpawnLoc,
        template_name: &str,
        origin: DeployKind,
    ) -> Result<GroupId> {
        let spctx = SpawnCtx::new(lua)?;
        let (gid, template) =
            self.init_template(&spctx, idx, side, location, template_name, origin)?;
        self.ephemeral.dirty = true;
        spctx.spawn(template)?;
        Ok(gid)
    }

    pub fn slot_miz_unit<'lua>(
        &self,
        lua: MizLua<'lua>,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<UnitInfo<'lua>> {
        let miz = Miz::singleton(lua)?;
        let uid = slot
            .as_unit_id()
            .ok_or_else(|| anyhow!("player is in jtac"))?;
        miz.get_unit(idx, &uid)?
            .ok_or_else(|| anyhow!("unknown slot"))
    }

    pub fn slot_instance_unit<'lua>(
        &self,
        lua: MizLua<'lua>,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<Unit<'lua>> {
        let miz = Miz::singleton(lua)?;
        let uid = slot
            .as_unit_id()
            .ok_or_else(|| anyhow!("player is in {:?}", slot))?;
        let uifo = miz
            .get_unit(idx, &uid)?
            .ok_or_else(|| anyhow!("unit {:?} not in mission", uid))?;
        Unit::get_by_name(lua, &*uifo.unit.name()?)
    }

    pub fn slot_instance_pos(
        &self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<Position3> {
        let unit = self.slot_instance_unit(lua, idx, slot)?;
        unit.as_object()?.get_position()
    }
}
