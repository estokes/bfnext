extern crate nalgebra as na;
use crate::cfg::Cfg;
use chrono::{prelude::*, Duration};
use compact_str::format_compact;
use dcso3::{
    coalition::{Coalition, Side},
    cvt_err,
    env::miz::{Group, GroupInfo, GroupKind, Miz, MizIndex, TriggerZone, TriggerZoneTyp},
    err,
    group::GroupCategory,
    net::{SlotId, SlotIdKind, Ucid},
    DeepClone, LuaEnv, MizLua, String, Vector2,
};
use fxhash::FxHashMap;
use log::{error, debug};
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::{
    fmt::Display,
    fs::{self, File},
    path::{Path, PathBuf},
    str::FromStr,
    sync::atomic::{AtomicU64, Ordering},
};

type Map<K, V> = immutable_chunkmap::map::Map<K, V, 32>;
type Set<K> = immutable_chunkmap::set::Set<K, 32>;

macro_rules! atomic_id {
    ($name:ident) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
            pub struct $name(u64);

            static [<MAX_ $name:upper _ID>]: AtomicU64 = AtomicU64::new(0);

            impl Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }

            impl Default for $name {
                fn default() -> Self {
                    Self(0)
                }
            }

            impl $name {
                pub fn new() -> Self {
                    Self([<MAX_ $name:upper _ID>].fetch_add(1, Ordering::Relaxed))
                }

                fn update_max(id: Self) {
                    // not strictly thread safe, but it doesn't matter in this context
                    if id.0 >= [<MAX_ $name:upper _ID>].load(Ordering::Relaxed) {
                        [<MAX_ $name:upper _ID>].store(id.0 + 1, Ordering::Relaxed)
                    }
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SlotAuth {
    Yes,
    ObjectiveNotOwned,
    NoLives,
    NotRegistered(Side),
}

atomic_id!(GroupId);
atomic_id!(UnitId);
atomic_id!(ObjectiveId);

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
    pub kind: GroupKind,
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

impl<'lua> SpawnCtx<'lua> {
    pub fn new(lua: MizLua<'lua>) -> LuaResult<Self> {
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
    ) -> LuaResult<GroupInfo> {
        let mut template = self
            .miz
            .get_group(idx, kind, side, template_name)?
            .ok_or_else(|| err("no such template"))?;
        template.group = template.group.deep_clone(self.lua.inner())?;
        Ok(template)
    }

    pub fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> LuaResult<TriggerZone> {
        Ok(self
            .miz
            .get_trigger_zone(idx, name)?
            .ok_or_else(|| err("no such trigger zone"))?)
    }

    pub fn spawn(&self, template: GroupInfo) -> LuaResult<()> {
        match GroupCategory::from_kind(template.category) {
            None => self
                .coalition
                .add_static_object(template.country, template.group),
            Some(category) => self
                .coalition
                .add_group(template.country, category, template.group),
        }
    }

    pub fn despawn(&self, name: &str) -> LuaResult<()> {
        let group = dcso3::group::Group::get_by_name(self.lua, name)?;
        group.destroy()
    }
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum LifeType {
    Standard,
    Intercept,
    Logistics,
    Attack,
    Recon,
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
pub struct Db {
    #[serde(skip)]
    dirty: bool,
    #[serde(skip)]
    cfg: Cfg,
    groups: Map<GroupId, SpawnedGroup>,
    units: Map<UnitId, SpawnedUnit>,
    groups_by_name: Map<String, GroupId>,
    units_by_name: Map<String, UnitId>,
    groups_by_side: Map<Side, Set<GroupId>>,
    objectives: Map<ObjectiveId, Objective>,
    objectives_by_slot: Map<SlotId, ObjectiveId>,
    objectives_by_name: Map<String, ObjectiveId>,
    objectives_by_group: Map<GroupId, ObjectiveId>,
    players: Map<Ucid, Player>,
    #[serde(skip)]
    players_by_slot: Map<SlotId, Ucid>,
}

impl Db {
    pub fn load(path: &Path) -> LuaResult<Self> {
        let file = File::open(&path).map_err(|e| {
            error!("failed to open save file {:?}, {:?}", path, e);
            err("io error")
        })?;
        let mut db: Self = serde_json::from_reader(file).map_err(|e| {
            error!("failed to decode save file {:?}, {:?}", path, e);
            err("decode error")
        })?;
        db.cfg = Cfg::load(path)?;
        for (id, _) in &db.groups {
            GroupId::update_max(*id)
        }
        for (id, _) in &db.units {
            UnitId::update_max(*id)
        }
        for (id, _) in &db.objectives {
            ObjectiveId::update_max(*id)
        }
        Ok(db)
    }

    pub fn save(&self, path: &Path) -> anyhow::Result<()> {
        let mut tmp = PathBuf::from(path);
        tmp.set_extension("tmp");
        let file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&tmp)?;
        serde_json::to_writer(file, self)?;
        fs::rename(tmp, path)?;
        Ok(())
    }

    pub fn maybe_snapshot(&mut self) -> Option<Self> {
        if self.dirty {
            self.dirty = false;
            Some(self.clone())
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
    fn init_objective(&mut self, zone: TriggerZone, name: &str) -> LuaResult<()> {
        fn side_and_name(s: &str) -> LuaResult<(Side, String)> {
            if let Some(name) = s.strip_prefix("R") {
                Ok((Side::Red, String::from(name)))
            } else if let Some(name) = s.strip_prefix("B") {
                Ok((Side::Blue, String::from(name)))
            } else if let Some(name) = s.strip_prefix("N") {
                Ok((Side::Neutral, String::from(name)))
            } else {
                Err(err("invalid default coalition, expected B, R, or N"))
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
            return Err(err("invalid objective type"));
        };
        let id = ObjectiveId::new();
        let radius = match zone.typ()? {
            TriggerZoneTyp::Quad(_) => return Err(err("invalid zone volume type")),
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
        self.objectives.insert_cow(id, obj);
        self.objectives_by_name.insert_cow(name, id);
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
    ) -> LuaResult<()> {
        let name = name.parse::<ObjGroup>()?;
        let pos = zone.pos()?;
        let (obj, side) = {
            let mut iter = self.objectives.into_iter();
            loop {
                match iter.next() {
                    None => return Err(err("group isn't associated with an objective")),
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
            GroupKind::Any,
            &SpawnLoc::AtPos(pos),
            name.template(),
        )?;
        self.objectives[&obj]
            .groups
            .get_or_default_cow(side)
            .insert_cow(name.clone(), gid);
        self.objectives_by_group.insert_cow(gid, obj);
        Ok(())
    }

    pub fn init_objective_slots(&mut self, slot: Group) -> LuaResult<()> {
        for unit in slot.units()? {
            let unit = unit?;
            let id = SlotId::from(unit.unit_id()?);
            let pos = slot.pos()?;
            let obj = {
                let mut iter = self.objectives.into_iter();
                loop {
                    match iter.next() {
                        None => return Err(err("slot not associated with an objective")),
                        Some((id, obj)) => {
                            if na::distance(&pos.into(), &obj.pos.into()) <= obj.radius {
                                break *id;
                            }
                        }
                    }
                }
            };
            let vehicle = Vehicle(unit.typ()?);
            match self.cfg.life_types.get(&vehicle) {
                None => {
                    error!("vehicle {:?} doesn't have a configured life type", vehicle);
                    return Err(err("vehicle missing life type"));
                }
                Some(typ) => match self.cfg.default_lives.get(&typ) {
                    Some((n, f)) if *n > 0 && *f > 0 => (),
                    None => {
                        error!("vehicle {:?} has no configured life type", vehicle);
                        return Err(err("vehicle has no configured life type"));
                    }
                    Some((n, f)) => {
                        error!(
                            "vehicle {:?} life type {:?} has no configured lives ({n}) or negative reset time ({f})",
                            vehicle, typ
                        );
                        return Err(err("vehicle's life type has no default lives"));
                    }
                },
            }
            self.objectives_by_slot.insert_cow(id.clone(), obj);
            self.objectives[&obj].slots.insert_cow(id, vehicle);
        }
        Ok(())
    }

    pub fn init(lua: MizLua, cfg: Cfg, idx: &MizIndex, miz: &Miz) -> LuaResult<Self> {
        let spctx = SpawnCtx::new(lua)?;
        let mut t = Self::default();
        t.cfg = cfg;
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
                return Err(err("invalid trigger zone type code, expected O, G, or T"));
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
        t.dirty = true;
        debug!("{:#?}", &t);
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
                    let group = &self.groups[gid];
                    let logi = match ObjGroupClass::from(group.template_name.as_str()) {
                        ObjGroupClass::Logi => true,
                        _ => false,
                    };
                    for uid in &group.units {
                        total += 1;
                        if logi {
                            logi_total += 1;
                        }
                        if !self.units[uid].dead {
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
        let (health, logi) = self.compute_objective_status(&self.objectives[oid]);
        let obj = &mut self.objectives[oid];
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
    ) -> LuaResult<()> {
        let obj = &self.objectives[&oid];
        if let Some(groups) = obj.groups.get(&obj.owner) {
            let damaged_by_class: Map<ObjGroupClass, Set<GroupId>> =
                groups.into_iter().fold(Map::new(), |mut m, (name, id)| {
                    let class = ObjGroupClass::from(name.template());
                    let mut damaged = false;
                    for uid in &self.groups[id].units {
                        damaged |= self.units[uid].dead;
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
                        let group = &self.groups[gid];
                        for uid in &group.units {
                            self.units[uid].dead = false;
                        }
                        self.respawn_group(idx, spctx, group)?;
                        self.update_objective_status(&oid, now);
                        self.dirty = true;
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
    ) -> LuaResult<()> {
        let spctx = SpawnCtx::new(lua)?;
        let to_repair = self
            .objectives
            .into_iter()
            .filter_map(|(oid, obj)| {
                let logi = obj.logi as f32 / 100.;
                let repair_time = self.cfg.repair_time as f32 / logi;
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
        if let Some(unit) = self.units.get_mut_cow(&id) {
            unit.dead = dead;
            if let Some(oid) = self.objectives_by_group.get(&unit.group).copied() {
                self.update_objective_status(&oid, now)
            }
        }
        self.dirty = true;
    }

    pub fn groups(&self) -> impl Iterator<Item = (&GroupId, &SpawnedGroup)> {
        self.groups.into_iter()
    }

    pub fn get_group(&self, id: &GroupId) -> Option<&SpawnedGroup> {
        self.groups.get(id)
    }

    pub fn get_group_by_name(&self, name: &str) -> Option<&SpawnedGroup> {
        self.groups_by_name
            .get(name)
            .and_then(|gid| self.groups.get(gid))
    }

    pub fn get_unit(&self, id: &UnitId) -> Option<&SpawnedUnit> {
        self.units.get(id)
    }

    pub fn get_unit_by_name(&self, name: &str) -> Option<&SpawnedUnit> {
        self.units_by_name
            .get(name)
            .and_then(|uid| self.get_unit(uid))
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
            .filter_map(|uid| {
                self.units.get(uid).and_then(|u| {
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

    /// add the units to the db, but don't actually spawn them
    fn init_template<'lua>(
        &mut self,
        spctx: &'lua SpawnCtx<'lua>,
        idx: &MizIndex,
        side: Side,
        kind: GroupKind,
        location: &SpawnLoc,
        template_name: &str,
    ) -> LuaResult<(GroupId, GroupInfo<'lua>)> {
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
            id: gid,
            name: group_name.clone(),
            template_name: template_name.clone(),
            side,
            kind,
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
            self.units.insert_cow(uid, spawned_unit);
            self.units_by_name.insert_cow(unit_name, uid);
        }
        self.groups.insert_cow(gid, spawned);
        self.groups_by_name.insert_cow(group_name, gid);
        self.groups_by_side.get_or_default_cow(side).insert_cow(gid);
        Ok((gid, template))
    }

    pub fn spawn_template_as_new<'lua>(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        side: Side,
        kind: GroupKind,
        location: &SpawnLoc,
        template_name: &str,
    ) -> LuaResult<GroupId> {
        let spctx = SpawnCtx::new(lua)?;
        let (gid, template) =
            self.init_template(&spctx, idx, side, kind, location, template_name)?;
        self.dirty = true;
        spctx.spawn(template)?;
        Ok(gid)
    }

    pub fn player_in_slot(&self, slot: &SlotId) -> Option<&Ucid> {
        self.players_by_slot.get(&slot)
    }

    pub fn takeoff(&mut self, time: DateTime<Utc>, slot: SlotId) {
        let objective = match self
            .objectives_by_slot
            .get(&slot)
            .and_then(|id| self.objectives.get(&id))
        {
            Some(objective) => objective,
            None => return,
        };
        let player = match self
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.players.get_mut_cow(ucid))
        {
            Some(player) => player,
            None => return,
        };
        let life_type = self.cfg.life_types[&objective.slots[&slot]];
        let (_, player_lives) = player
            .lives
            .get_or_insert_cow(life_type, || (time, self.cfg.default_lives[&life_type].0));
        if *player_lives > 0 {
            // paranoia
            *player_lives -= 1;
        }
        self.dirty = true;
    }

    pub fn land(&mut self, slot: SlotId, position: Vector2) -> bool {
        let objective = match self
            .objectives_by_slot
            .get(&slot)
            .and_then(|id| self.objectives.get(&id))
        {
            Some(objective) => objective,
            None => return true,
        };
        let player = match self
            .players_by_slot
            .get(&slot)
            .and_then(|ucid| self.players.get_mut_cow(ucid))
        {
            Some(player) => player,
            None => return true,
        };
        let life_type = self.cfg.life_types[&objective.slots[&slot]];
        let (_, player_lives) = match player.lives.get_mut_cow(&life_type) {
            Some(l) => l,
            None => return true,
        };
        let is_on_owned_objective = self.objectives.into_iter().fold(false, |res, (_, obj)| {
            res || (obj.owner == player.side && obj.is_in_circle(position))
        });
        if is_on_owned_objective {
            *player_lives += 1;
            if *player_lives >= self.cfg.default_lives[&life_type].0 {
                player.lives.remove_cow(&life_type);
            }
            self.dirty = true;
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
        let player = match self.players.get_mut_cow(ucid) {
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
                    .objectives_by_slot
                    .get(&slot)
                    .and_then(|id| self.objectives.get(id))
                {
                    Some(o) if o.owner != Side::Neutral => o,
                    Some(_) | None => return SlotAuth::ObjectiveNotOwned,
                };
                if objective.owner != player.side {
                    return SlotAuth::ObjectiveNotOwned;
                }
                let life_type = &self.cfg.life_types[&objective.slots[&slot]];
                loop {
                    match player.lives.get(life_type).map(|t| *t) {
                        Some((reset, 0)) => {
                            let reset_after =
                                Duration::seconds(self.cfg.default_lives[life_type].1 as i64);
                            if time - reset >= reset_after {
                                player.lives.remove_cow(life_type);
                                self.dirty = true;
                            } else {
                                break SlotAuth::NoLives;
                            }
                        }
                        None | Some(_) => {
                            player.current_slot = Some(slot.clone());
                            self.players_by_slot.insert_cow(slot, ucid.clone());
                            break SlotAuth::Yes;
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
        match self.players.get(&ucid) {
            Some(p) if p.side != side => Err((p.side_switches, p.side)),
            Some(_) => Ok(()),
            None => {
                self.players.insert_cow(
                    ucid,
                    Player {
                        name,
                        side,
                        side_switches: self.cfg.side_switches,
                        lives: Map::new(),
                        current_slot: None,
                    },
                );
                self.dirty = true;
                Ok(())
            }
        }
    }

    pub fn sideswitch_player(&mut self, ucid: &Ucid, side: Side) -> Result<(), &'static str> {
        match self.players.get_mut_cow(ucid) {
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
                    self.dirty = true;
                    Ok(())
                }
            }
        }
    }
}
