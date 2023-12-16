extern crate nalgebra as na;
use crate::cfg::{CargoConfig, Cfg, Crate, Deployable, Troop};
use anyhow::{anyhow, bail, Result};
use chrono::{prelude::*, Duration};
use compact_str::format_compact;
use dcso3::{
    atomic_id,
    coalition::{Coalition, Side},
    cvt_err,
    env::miz::{Group, GroupInfo, GroupKind, Miz, MizIndex, TriggerZone, TriggerZoneTyp, UnitInfo},
    err,
    group::GroupCategory,
    land::Land,
    net::{SlotId, SlotIdKind, Ucid},
    trigger::Trigger,
    unit::Unit,
    DeepClone, LuaEnv, LuaVec2, MizLua, Position3, String, Vector2,
};
use fxhash::FxHashMap;
use log::debug;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::{
    borrow::Borrow,
    collections::{btree_map, hash_map::Entry, BTreeMap},
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
};

type Map<K, V> = immutable_chunkmap::map::Map<K, V, 32>;
type Set<K> = immutable_chunkmap::set::Set<K, 32>;

macro_rules! or_msg {
    ($e:expr, $msg:expr) => {
        match $e {
            Some(res) => res,
            None => bail!($msg),
        }
    };
}

atomic_id!(GroupId);
atomic_id!(UnitId);
atomic_id!(ObjectiveId);

#[derive(Debug, Clone, Copy)]
pub enum SlotAuth {
    Yes,
    ObjectiveNotOwned,
    NoLives,
    NotRegistered(Side),
}

#[derive(Debug, Clone, Copy)]
pub struct NearbyCrate<'a> {
    pub group: &'a SpawnedGroup,
    pub crate_def: &'a Crate,
    pub heading: f64,
    pub distance: f64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub enum DeployKind {
    Objective,
    Deployed(Deployable),
    Troop(Troop),
    Crate(ObjectiveId, Crate),
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cargo {
    pub troops: SmallVec<[Troop; 1]>,
    pub crates: SmallVec<[Crate; 1]>,
}

impl Cargo {
    pub fn num_troops(&self) -> usize {
        self.troops.len()
    }

    pub fn num_crates(&self) -> usize {
        self.crates.len()
    }

    pub fn num_total(&self) -> usize {
        self.num_crates() + self.num_troops()
    }

    pub fn weight(&self) -> i64 {
        let cr = self.crates.iter().fold(0, |acc, cr| acc + cr.weight as i64);
        self.troops
            .iter()
            .fold(cr, |acc, tr| acc + tr.weight as i64)
    }
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnLoc {
    AtPos(Vector2),
    AtTrigger { name: String, offset: Vector2 },
}

pub struct SpawnCtx<'lua> {
    coalition: Coalition<'lua>,
    miz: Miz<'lua>,
    lua: MizLua<'lua>,
}

pub enum Despawn<'a> {
    Group(&'a str),
    Static(&'a str),
}

impl<'lua> SpawnCtx<'lua> {
    pub fn new(lua: MizLua<'lua>) -> Result<Self> {
        Ok(Self {
            coalition: Coalition::singleton(lua)?,
            miz: Miz::singleton(lua)?,
            lua,
        })
    }

    pub fn get_template(
        &self,
        idx: &MizIndex,
        kind: GroupKind,
        side: Side,
        template_name: &str,
    ) -> Result<GroupInfo> {
        let mut template = self
            .miz
            .get_group_by_name(idx, kind, side, template_name)?
            .ok_or_else(|| err("no such template"))?;
        template.group = template.group.deep_clone(self.lua.inner())?;
        Ok(template)
    }

    pub fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> Result<TriggerZone> {
        Ok(self
            .miz
            .get_trigger_zone(idx, name)?
            .ok_or_else(|| anyhow!("no such trigger zone {name}"))?)
    }

    pub fn spawn(&self, template: GroupInfo) -> Result<()> {
        match GroupCategory::from_kind(template.category) {
            None => {
                // static objects are not fed to addStaticObject as groups
                let unit = template.group.units()?.first()?;
                self.coalition.add_static_object(template.country, unit)?;
            }
            Some(category) => {
                dbg!(self
                    .coalition
                    .add_group(template.country, category, template.group))?;
            }
        }
        Ok(())
    }

    pub fn despawn(&self, name: Despawn) -> Result<()> {
        match name {
            Despawn::Group(name) => {
                let group = dcso3::group::Group::get_by_name(self.lua, name)?;
                Ok(group.destroy()?)
            }
            Despawn::Static(name) => {
                let obj = dcso3::static_object::StaticObject::get_by_name(self.lua, name)?;
                Ok(obj.as_object()?.destroy()?)
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum ObjectiveKind {
    Airbase,
    Fob,
    Fuelbase,
    Samsite,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, Hash)]
pub enum LifeType {
    Standard,
    Intercept,
    Logistics,
    Attack,
    Recon,
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

#[derive(Debug, Clone, Serialize, Deserialize, Hash, PartialEq, Eq, PartialOrd, Ord)]
pub struct Vehicle(String);

impl<'a> From<&'a str> for Vehicle {
    fn from(value: &'a str) -> Self {
        Self(value.into())
    }
}

impl Borrow<str> for Vehicle {
    fn borrow(&self) -> &str {
        &*self.0
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
pub struct Ephemeral {
    dirty: bool,
    cfg: Cfg,
    players_by_slot: FxHashMap<SlotId, Ucid>,
    cargo: FxHashMap<SlotId, Cargo>,
    deployable_idx: FxHashMap<Side, DeployableIndex>,
}

impl Ephemeral {
    fn set_cfg(&mut self, cfg: Cfg) -> Result<()> {
        for (side, deployables) in cfg.deployables.iter() {
            let idx = self.deployable_idx.entry(*side).or_default();
            for dep in deployables.iter() {
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

    pub fn load(path: &Path) -> Result<Self> {
        let file = File::open(&path)
            .map_err(|e| anyhow!("failed to open save file {:?}, {:?}", path, e))?;
        let persisted: Persisted = serde_json::from_reader(file)
            .map_err(|e| anyhow!("failed to decode save file {:?}, {:?}", path, e))?;
        let mut db = Db {
            persisted,
            ephemeral: Ephemeral::default(),
        };
        db.ephemeral.set_cfg(Cfg::load(path)?)?;
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

    /// objectives are just trigger zones named according to type codes
    /// the first caracter is the type of the zone
    /// O - Objective
    /// G - Group within an objective
    /// T - Generic trigger zone, ignored by the engine
    ///
    /// Then a 2 character type code
    /// - AB: Airbase
    /// - FO: Fob
    /// - SA: Sam site
    /// - FB: Fuel base
    ///
    /// Then a 1 character code for the default owner
    /// followed by the display name
    /// - R: Red
    /// - B: Blue
    /// - N: Neutral
    ///
    /// So e.g. Tblisi would be OABBTBLISI -> Objective, Airbase, Default to Blue, named Tblisi
    fn init_objective(&mut self, zone: TriggerZone, name: &str) -> Result<()> {
        fn side_and_name(s: &str) -> Result<(Side, String)> {
            if let Some(name) = s.strip_prefix("R") {
                Ok((Side::Red, String::from(name)))
            } else if let Some(name) = s.strip_prefix("B") {
                Ok((Side::Blue, String::from(name)))
            } else if let Some(name) = s.strip_prefix("N") {
                Ok((Side::Neutral, String::from(name)))
            } else {
                bail!("invalid default coalition {s} expected B, R, or N prefix")
            }
        }
        let (kind, owner, name) = if let Some(name) = name.strip_prefix("AB") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Airbase, side, name)
        } else if let Some(name) = name.strip_prefix("FO") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Fob, side, name)
        } else if let Some(name) = name.strip_prefix("FB") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Fuelbase, side, name)
        } else if let Some(name) = name.strip_prefix("SA") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Samsite, side, name)
        } else {
            bail!("invalid objective type for {name}, expected AB, FO, FB, or SA prefix")
        };
        let id = ObjectiveId::new();
        let radius = match zone.typ()? {
            TriggerZoneTyp::Quad(_) => bail!("zone volume type quad isn't supported yet"),
            TriggerZoneTyp::Circle { radius } => radius,
        };
        let pos = zone.pos()?;
        let obj = Objective {
            id,
            spawned: false,
            trigger_name: zone.name()?,
            pos,
            radius,
            name: name.clone(),
            kind,
            owner,
            slots: Map::new(),
            groups: Map::new(),
            health: 0,
            logi: 0,
            last_change_ts: Utc::now(),
        };
        self.persisted.objectives.insert_cow(id, obj);
        self.persisted.objectives_by_name.insert_cow(name, id);
        Ok(())
    }

    /// Objective groups are trigger zones with the first character set to G. They are then a template
    /// name, followed by # and a number. They are associated with an objective by proximity.
    /// e.g. GRIRSRAD#001 would be the 1st instantiation of the template RIRSRAD, which must
    /// correspond to a group in the miz file. There is one special template name called (R|B|N)LOGI
    /// which corresponds to the logistics template for objectives
    fn init_objective_group(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        _miz: &Miz,
        zone: TriggerZone,
        name: &str,
    ) -> Result<()> {
        let name = name.parse::<ObjGroup>()?;
        let pos = zone.pos()?;
        let (obj, side) = {
            let mut iter = self.persisted.objectives.into_iter();
            loop {
                match iter.next() {
                    None => bail!("group {:?} isn't associated with an objective", name),
                    Some((id, obj)) => {
                        // this is inefficent; look into an orthographic database
                        if na::distance(&pos.into(), &obj.pos.into()) <= obj.radius {
                            break (*id, obj.owner);
                        }
                    }
                }
            }
        };
        let (gid, _) = self.init_template(
            spctx,
            idx,
            side,
            &SpawnLoc::AtPos(pos),
            name.template(),
            DeployKind::Objective,
        )?;
        self.persisted.objectives[&obj]
            .groups
            .get_or_default_cow(side)
            .insert_cow(name.clone(), gid);
        self.persisted.objectives_by_group.insert_cow(gid, obj);
        Ok(())
    }

    pub fn init_objective_slots(&mut self, slot: Group) -> Result<()> {
        for unit in slot.units()? {
            let unit = unit?;
            let id = SlotId::from(unit.id()?);
            let pos = slot.pos()?;
            let obj = {
                let mut iter = self.persisted.objectives.into_iter();
                loop {
                    match iter.next() {
                        None => bail!("slot {:?} not associated with an objective", slot),
                        Some((id, obj)) => {
                            if na::distance(&pos.into(), &obj.pos.into()) <= obj.radius {
                                break *id;
                            }
                        }
                    }
                }
            };
            let vehicle = Vehicle(unit.typ()?);
            match self.ephemeral.cfg.life_types.get(&vehicle) {
                None => bail!("vehicle {:?} doesn't have a configured life type", vehicle),
                Some(typ) => match self.ephemeral.cfg.default_lives.get(&typ) {
                    Some((n, f)) if *n > 0 && *f > 0 => (),
                    None => bail!("vehicle {:?} has no configured life type", vehicle),
                    Some((n, f)) => {
                        bail!(
                            "vehicle {:?} life type {:?} has no configured lives ({n}) or negative reset time ({f})",
                            vehicle, typ
                        )
                    }
                },
            }
            self.persisted
                .objectives_by_slot
                .insert_cow(id.clone(), obj);
            self.persisted.objectives[&obj]
                .slots
                .insert_cow(id, vehicle);
        }
        Ok(())
    }

    pub fn init(lua: MizLua, cfg: Cfg, idx: &MizIndex, miz: &Miz) -> Result<Self> {
        let spctx = SpawnCtx::new(lua)?;
        let mut t = Self::default();
        t.ephemeral.set_cfg(cfg)?;
        // first init all the objectives
        for zone in miz.triggers()? {
            let zone = zone?;
            let name = zone.name()?;
            if let Some(name) = name.strip_prefix("O") {
                t.init_objective(zone, name)?
            }
        }
        // now associate groups with objectives
        for zone in miz.triggers()? {
            let zone = zone?;
            let name = zone.name()?;
            if let Some(name) = name.strip_prefix("G") {
                t.init_objective_group(&spctx, idx, miz, zone, name)?
            } else if name.starts_with("T") || name.starts_with("O") {
                () // ignored
            } else {
                bail!("invalid trigger zone type code {name}, expected O, G, or T prefix")
            }
        }
        // now associate slots with objectives
        for coa in [
            miz.coalition(Side::Blue)?,
            miz.coalition(Side::Red)?,
            miz.coalition(Side::Neutral)?,
        ] {
            for country in coa.countries()? {
                let country = country?;
                for plane in country.planes()? {
                    let plane = plane?;
                    t.init_objective_slots(plane)?
                }
                for heli in country.helicopters()? {
                    let heli = heli?;
                    t.init_objective_slots(heli)?
                }
            }
        }
        let now = Utc::now();
        let ids = t
            .persisted
            .objectives
            .into_iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in ids {
            t.update_objective_status(&id, now)
        }
        t.ephemeral.dirty = true;
        Ok(t)
    }

    fn compute_objective_status(&self, obj: &Objective) -> (u8, u8) {
        obj.groups
            .get(&obj.owner)
            .map(|groups| {
                let mut total = 0;
                let mut alive = 0;
                let mut logi_total = 0;
                let mut logi_alive = 0;
                for (_, gid) in groups {
                    let group = &self.persisted.groups[gid];
                    let logi = match ObjGroupClass::from(group.template_name.as_str()) {
                        ObjGroupClass::Logi => true,
                        _ => false,
                    };
                    for uid in &group.units {
                        total += 1;
                        if logi {
                            logi_total += 1;
                        }
                        if !self.persisted.units[uid].dead {
                            alive += 1;
                            if logi {
                                logi_alive += 1;
                            }
                        }
                    }
                }
                let health = ((alive as f32 / total as f32) * 100.).trunc() as u8;
                let logi = ((logi_alive as f32 / logi_total as f32) * 100.).trunc() as u8;
                (health, logi)
            })
            .unwrap_or((100, 100))
    }

    fn update_objective_status(&mut self, oid: &ObjectiveId, now: DateTime<Utc>) {
        let (health, logi) = self.compute_objective_status(&self.persisted.objectives[oid]);
        let obj = &mut self.persisted.objectives[oid];
        obj.health = health;
        obj.logi = logi;
        obj.last_change_ts = now;
        if obj.health == 0 {
            obj.owner = Side::Neutral;
        }
        debug!("objective {oid} health: {}, logi: {}", obj.health, obj.logi);
    }

    fn repair_objective(
        &mut self,
        idx: &MizIndex,
        spctx: &SpawnCtx,
        oid: ObjectiveId,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let obj = &self.persisted.objectives[&oid];
        if let Some(groups) = obj.groups.get(&obj.owner) {
            let damaged_by_class: Map<ObjGroupClass, Set<GroupId>> =
                groups.into_iter().fold(Map::new(), |mut m, (name, id)| {
                    let class = ObjGroupClass::from(name.template());
                    let mut damaged = false;
                    for uid in &self.persisted.groups[id].units {
                        damaged |= self.persisted.units[uid].dead;
                    }
                    if damaged {
                        m.get_or_default_cow(class).insert_cow(*id);
                        m
                    } else {
                        m
                    }
                });
            for class in [
                ObjGroupClass::Logi,
                ObjGroupClass::Sr,
                ObjGroupClass::Aaa,
                ObjGroupClass::Lr,
                ObjGroupClass::Armor,
                ObjGroupClass::Other,
            ] {
                if let Some(groups) = damaged_by_class.get(&class) {
                    for gid in groups {
                        let group = &self.persisted.groups[gid];
                        for uid in &group.units {
                            self.persisted.units[uid].dead = false;
                        }
                        self.respawn_group(idx, spctx, group)?;
                        self.update_objective_status(&oid, now);
                        self.ephemeral.dirty = true;
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn maybe_do_repairs(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let spctx = SpawnCtx::new(lua)?;
        let to_repair = self
            .persisted
            .objectives
            .into_iter()
            .filter_map(|(oid, obj)| {
                let logi = obj.logi as f32 / 100.;
                let repair_time = self.ephemeral.cfg.repair_time as f32 / logi;
                if repair_time < i64::MAX as f32 {
                    let repair_time = Duration::seconds(repair_time as i64);
                    if obj.health < 100 && (now - obj.last_change_ts) >= repair_time {
                        Some(*oid)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for oid in to_repair {
            self.repair_objective(idx, &spctx, oid, now)?
        }
        Ok(())
    }

    pub fn unit_dead(&mut self, id: UnitId, dead: bool, now: DateTime<Utc>) {
        if let Some(unit) = self.persisted.units.get_mut_cow(&id) {
            unit.dead = dead;
            if let Some(oid) = self.persisted.objectives_by_group.get(&unit.group).copied() {
                self.update_objective_status(&oid, now)
            }
        }
        self.ephemeral.dirty = true;
    }

    pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &SpawnedGroup)> {
        self.persisted.groups.into_iter()
    }

    pub fn get_group(&self, id: &GroupId) -> Option<&SpawnedGroup> {
        self.persisted.groups.get(id)
    }

    pub fn get_group_by_name(&self, name: &str) -> Option<&SpawnedGroup> {
        self.persisted
            .groups_by_name
            .get(name)
            .and_then(|gid| self.persisted.groups.get(gid))
    }

    pub fn get_unit(&self, id: &UnitId) -> Option<&SpawnedUnit> {
        self.persisted.units.get(id)
    }

    pub fn get_unit_by_name(&self, name: &str) -> Option<&SpawnedUnit> {
        self.persisted
            .units_by_name
            .get(name)
            .and_then(|uid| self.get_unit(uid))
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
        location: &SpawnLoc,
        template_name: &str,
        origin: DeployKind,
    ) -> Result<(GroupId, GroupInfo<'lua>)> {
        let template_name = String::from(template_name);
        let template = spctx.get_template(idx, GroupKind::Any, side, template_name.as_str())?;
        let kind = GroupCategory::from_kind(template.category);
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
            let unit_pos_offset = orig_group_pos - unit.pos()?;
            let pos = pos + unit_pos_offset;
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
        location: &SpawnLoc,
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

    pub fn player_in_slot(&self, slot: &SlotId) -> Option<&Ucid> {
        self.ephemeral.players_by_slot.get(&slot)
    }

    pub fn takeoff(&mut self, time: DateTime<Utc>, slot: SlotId) {
        let objective = match self
            .persisted
            .objectives_by_slot
            .get(&slot)
            .and_then(|id| self.persisted.objectives.get(&id))
        {
            Some(objective) => objective,
            None => return,
        };
        let player = match self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid))
        {
            Some(player) => player,
            None => return,
        };
        let life_type = self.ephemeral.cfg.life_types[&objective.slots[&slot]];
        let (_, player_lives) = player.lives.get_or_insert_cow(life_type, || {
            (time, self.ephemeral.cfg.default_lives[&life_type].0)
        });
        if *player_lives > 0 {
            // paranoia
            *player_lives -= 1;
        }
        self.ephemeral.dirty = true;
    }

    pub fn land(&mut self, slot: SlotId, position: Vector2) -> bool {
        let objective = match self
            .persisted
            .objectives_by_slot
            .get(&slot)
            .and_then(|id| self.persisted.objectives.get(&id))
        {
            Some(objective) => objective,
            None => return true,
        };
        let player = match self
            .ephemeral
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.persisted.players.get_mut_cow(ucid))
        {
            Some(player) => player,
            None => return true,
        };
        let life_type = self.ephemeral.cfg.life_types[&objective.slots[&slot]];
        let (_, player_lives) = match player.lives.get_mut_cow(&life_type) {
            Some(l) => l,
            None => return true,
        };
        let is_on_owned_objective = self
            .persisted
            .objectives
            .into_iter()
            .fold(false, |res, (_, obj)| {
                res || (obj.owner == player.side && obj.is_in_circle(position))
            });
        if is_on_owned_objective {
            *player_lives += 1;
            if *player_lives >= self.ephemeral.cfg.default_lives[&life_type].0 {
                player.lives.remove_cow(&life_type);
            }
            self.ephemeral.dirty = true;
            true
        } else {
            false
        }
    }

    pub fn try_occupy_slot(
        &mut self,
        time: DateTime<Utc>,
        slot_side: Side,
        slot: SlotId,
        ucid: &Ucid,
    ) -> SlotAuth {
        if slot_side == Side::Neutral && slot == SlotId::spectator() {
            return SlotAuth::Yes;
        }
        let player = match self.persisted.players.get_mut_cow(ucid) {
            Some(player) => player,
            None => return SlotAuth::NotRegistered(slot_side),
        };
        if slot_side != player.side {
            return SlotAuth::ObjectiveNotOwned;
        }
        match slot.classify() {
            SlotIdKind::ArtilleryCommander
            | SlotIdKind::ForwardObserver
            | SlotIdKind::Instructor
            | SlotIdKind::Observer => {
                // CR estokes: add permissions for game master
                SlotAuth::Yes
            }
            SlotIdKind::Normal => {
                let objective = match self
                    .persisted
                    .objectives_by_slot
                    .get(&slot)
                    .and_then(|id| self.persisted.objectives.get(id))
                {
                    Some(o) if o.owner != Side::Neutral => o,
                    Some(_) | None => return SlotAuth::ObjectiveNotOwned,
                };
                if objective.owner != player.side {
                    return SlotAuth::ObjectiveNotOwned;
                }
                let life_type = &self.ephemeral.cfg.life_types[&objective.slots[&slot]];
                macro_rules! yes {
                    () => {
                        player.current_slot = Some(slot.clone());
                        self.ephemeral.players_by_slot.insert(slot, ucid.clone());
                        break SlotAuth::Yes;
                    };
                }
                loop {
                    match player.lives.get(life_type).map(|t| *t) {
                        None => {
                            yes!();
                        }
                        Some((reset, n)) => {
                            let reset_after = Duration::seconds(
                                self.ephemeral.cfg.default_lives[life_type].1 as i64,
                            );
                            if time - reset >= reset_after {
                                player.lives.remove_cow(life_type);
                                self.ephemeral.dirty = true;
                            } else if n == 0 {
                                break SlotAuth::NoLives;
                            } else {
                                yes!();
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn register_player(
        &mut self,
        ucid: Ucid,
        name: String,
        side: Side,
    ) -> Result<(), (Option<u8>, Side)> {
        match self.persisted.players.get(&ucid) {
            Some(p) if p.side != side => Err((p.side_switches, p.side)),
            Some(_) => Ok(()),
            None => {
                self.persisted.players.insert_cow(
                    ucid,
                    Player {
                        name,
                        side,
                        side_switches: self.ephemeral.cfg.side_switches,
                        lives: Map::new(),
                        current_slot: None,
                    },
                );
                self.ephemeral.dirty = true;
                Ok(())
            }
        }
    }

    pub fn sideswitch_player(&mut self, ucid: &Ucid, side: Side) -> Result<(), &'static str> {
        match self.persisted.players.get_mut_cow(ucid) {
            None => Err("You are not registered. Type blue or red to join a side"),
            Some(player) => {
                if side == player.side {
                    Err("you are already on the requested side")
                } else if let Some(0) = player.side_switches {
                    Err("you can't switch sides again this round")
                } else if side == Side::Neutral {
                    Err("you can't switch to neutral")
                } else {
                    match &mut player.side_switches {
                        Some(n) => {
                            *n -= 1;
                        }
                        None => (),
                    }
                    player.side = side;
                    self.ephemeral.dirty = true;
                    Ok(())
                }
            }
        }
    }

    pub fn spawn_crate(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
        name: &str,
    ) -> Result<()> {
        debug!("db spawning crate");
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let pos = self.slot_instance_pos(lua, idx, slot)?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let (oid, obj) = self
            .persisted
            .objectives
            .into_iter()
            .find_map(|(oid, obj)| {
                if obj.owner == side
                    && obj.logi() > 0
                    && na::distance(&obj.pos.into(), &point.into()) <= obj.radius
                {
                    return Some((oid, obj));
                }
                None
            });
        if let None = &obj {
            bail!("not near friendly logistics");
        }
        let crate_cfg = or_msg!(
            self.ephemeral
                .deployable_idx
                .get(&side)
                .and_then(|idx| idx.crates_by_name.get(name)),
            "no such crate"
        )
        .clone();
        let template = self.ephemeral.cfg.crate_template[&side].clone();
        let spawnpos = 20. * pos.x.0 + pos.p.0; // spawn it 20 meters in front of the player
        let spawnpos = SpawnLoc::AtPos(Vector2::new(spawnpos.x, spawnpos.z));
        let dk = DeployKind::Crate(oid, crate_cfg.clone());
        self.spawn_template_as_new(lua, idx, side, &spawnpos, &template, dk)?;
        Ok(())
    }

    pub fn list_nearby_crates<'a>(
        &'a self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<SmallVec<[NearbyCrate<'a>; 2]>> {
        let pos = self.slot_instance_pos(lua, idx, slot)?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let max_dist = self.ephemeral.cfg.crate_load_distance as f64;
        let mut res: SmallVec<[NearbyCrate; 2]> = smallvec![];
        for gid in &self.persisted.crates {
            let group = &self.persisted.groups[gid];
            let crate_def = match &group.origin {
                DeployKind::Crate(_, crt) => crt,
                DeployKind::Deployed(_) | DeployKind::Troop(_) | DeployKind::Objective => {
                    bail!("group {:?} is listed in crates but isn't a crate", gid)
                }
            };
            for uid in &group.units {
                let unit = &self.persisted.units[uid];
                let distance = na::distance(&point.into(), &unit.pos.into());
                if distance <= max_dist {
                    let v = unit.pos - point;
                    let heading = v.y.atan2(v.x) * (180. / std::f64::consts::PI);
                    res.push(NearbyCrate {
                        group,
                        crate_def,
                        heading,
                        distance,
                    })
                }
            }
        }
        res.sort_by_key(|nc| (nc.distance * 1000.) as u32);
        Ok(res)
    }

    pub fn list_cargo(&self, slot: &SlotId) -> Option<&Cargo> {
        self.ephemeral.cargo.get(slot)
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
        let uid = or_msg!(slot.as_unit_id(), "player is in jtac");
        let uifo = or_msg!(miz.get_unit(idx, &uid)?, "unit not in mission");
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
    pub fn cargo_capacity(&self, unit: &dcso3::env::miz::Unit) -> Result<CargoConfig> {
        let vehicle = Vehicle(unit.typ()?);
        let cargo_capacity = self
            .ephemeral
            .cfg
            .cargo
            .get(&vehicle)
            .ok_or_else(|| anyhow!("{:?} can't carry cargo", vehicle))
            .map(|c| *c)?;
        Ok(cargo_capacity)
    }

    pub fn unpakistan(&mut self, lua: MizLua, idx: &MizIndex, slot: &SlotId) -> Result<()> {
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let nearby = self
            .list_nearby_crates(lua, idx, slot)?
            .into_iter()
            .map(|nc| (nc.group.id, nc.crate_def.clone()))
            .collect::<SmallVec<[(GroupId, Crate); 2]>>();
        let didx = or_msg!(
            self.ephemeral.deployable_idx.get(&side),
            "your side can't deploy anything"
        );
        let mut candidates: FxHashMap<String, FxHashMap<String, Vec<GroupId>>> =
            FxHashMap::default();
        for (gid, cr) in &nearby {
            if let Some(dep) = didx.deployables_by_crates.get(&cr.name) {
                candidates
                    .entry(dep.clone())
                    .or_default()
                    .entry(cr.name.clone())
                    .or_default()
                    .push(*gid);
            }
        }
        let mut reasons = CompactString::new();
        candidates.retain(|dep, have| {
            let spec = &didx.deployables_by_name[dep];
            for req in &spec.crates {
                match have.get_mut(&req.name) {
                    Some(ids) if ids.len() >= req.required as usize => {
                        while ids.len() > req.required as usize {
                            ids.pop();
                        }
                    }
                    Some(_) | None => {
                        reasons
                            .push_str(format_compact!("can't spawn {dep} missing {}\n", req.name));
                        return false;
                    }
                }
            }
            true
        });
        let (dep, have) = match candidates.drain().next() {
            Some((dep, have)) => (dep, have),
            None => bail!(reasons),
        };
        let too_close = self.persisted.objectives.into_iter().any(|(oid, obj)| {
            let mut check = false;
            for gid in have.iter().flat_map(|(_, gids)| gids.iter()) {
                if let DeployKind::Crate(source, _) = &self.persisted.groups[gid].origin {
                    check |= oid == source;
                }
            }
            check |= obj.owner == side;
            check && {
                let dist = na::distance(&obj.pos.into(), &point.into());
                let excl_dist = self.ephemeral.cfg.logistics_exclusion as f64;
                dist <= excl_dist
            }
        });
        if too_close {
            bail!("too close to friendly logistics or crate origin");
        }
        let spctx = SpawnCtx::new(lua)?;
        let mut pos_by_typ = FxHashMap::default();
        for gid in have.iter().flat_map(|(_, gids)| gids.iter()) {
            let group = &self.persisted.groups[gid];
            if let DeployKind::Crate(_, spec) = &group.origin {
                if let Some(typ) = spec.pos_unit.as_ref() {
                    let uid = group.units.iter().next().unwrap();
                    let unit = Unit::get_by_name(lua, &*self.persisted.units[&uid].name)?;
                    let pos = unit.as_object()?.get_point();
                    pos_by_typ.insert(typ.clone(), Vector2::new(pos.x, pos.z));
                }
            }
            self.delete_group(&spctx, gid)?
        }

        unimplemented!()
    }

    pub fn unload_crate(&mut self, lua: MizLua, idx: &MizIndex, slot: &SlotId) -> Result<Crate> {
        let cargo = self.ephemeral.cargo.get(slot);
        if cargo.map(|c| c.crates.is_empty()).unwrap_or(true) {
            bail!("no crates onboard")
        }
        let unit = self.slot_instance_unit(lua, idx, slot)?;
        let unit_name = unit.as_object()?.get_name()?;
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let pos = unit.as_object()?.get_position()?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let ground_alt = Land::singleton(lua)?.get_height(LuaVec2(point))?;
        let agl = pos.p.y - ground_alt;
        let speed = unit.as_object()?.get_velocity()?.0.magnitude();
        let cargo = self.ephemeral.cargo.get_mut(slot).unwrap();
        let crate_cfg = cargo.crates.pop().unwrap();
        let weight = cargo.weight();
        if speed > crate_cfg.max_drop_speed as f64 {
            bail!("you are going too fast to unload your cargo")
        }
        if agl > crate_cfg.max_drop_height_agl as f64 {
            bail!("you are too high to unload your cargo")
        }
        let template = self.ephemeral.cfg.crate_template[&side].clone();
        let spawnpos = 20. * pos.x.0 + pos.p.0; // spawn it 20 meters in front of the player
        let spawnpos = SpawnLoc::AtPos(Vector2::new(spawnpos.x, spawnpos.z));
        let dk = DeployKind::Crate(crate_cfg.clone());
        self.spawn_template_as_new(lua, idx, side, &spawnpos, &template, dk)?;
        Trigger::singleton(lua)?
            .action()?
            .set_unit_internal_cargo(unit_name, weight)?;
        Ok(crate_cfg)
    }

    pub fn load_nearby_crate(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<Crate> {
        let (cargo_capacity, side, unit_name) = {
            let uifo = self.slot_miz_unit(lua, idx, slot)?;
            let side = uifo.side;
            let unit_name = uifo.unit.name()?;
            let cargo_capacity = self.cargo_capacity(&uifo.unit)?;
            (cargo_capacity, side, unit_name)
        };
        let cargo = self.ephemeral.cargo.entry(slot.clone()).or_default();
        if cargo_capacity.crate_slots as usize <= cargo.num_crates()
            || cargo_capacity.total_slots as usize <= cargo.num_total()
        {
            bail!("you already have a full load onboard")
        }
        let (gid, crate_def) = {
            let mut nearby = self.list_nearby_crates(lua, idx, slot)?;
            nearby.retain(|nc| nc.group.side == side);
            if nearby.is_empty() {
                bail!(
                    "no friendly crates within {} meters",
                    self.ephemeral.cfg.crate_load_distance
                );
            }
            let the_crate = nearby.first().unwrap();
            let gid = the_crate.group.id;
            let crate_def = the_crate.crate_def.clone();
            (gid, crate_def)
        };
        let cargo = self.ephemeral.cargo.get_mut(slot).unwrap();
        cargo.crates.push(crate_def.clone());
        let weight = cargo.weight();
        self.delete_group(&SpawnCtx::new(lua)?, &gid)?;
        Trigger::singleton(lua)?
            .action()?
            .set_unit_internal_cargo(unit_name, weight as i64)?;
        Ok(crate_def)
    }
}
