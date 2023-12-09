use crate::{
    as_tbl, coalition::Side, country, cvt_err, is_hooks_env, wrapped_prim, wrapped_table, Color,
    DcsTableExt, LuaEnv, LuaVec2, Path, Quad2, Sequence, String,
};
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::{collections::hash_map::Entry, ops::Deref};

wrapped_table!(Weather, None);

wrapped_prim!(UnitId, i64, Hash, Copy);
wrapped_prim!(GroupId, i64, Hash, Copy);

pub enum TriggerZoneTyp {
    Circle { radius: f64 },
    Quad(Quad2),
}

wrapped_table!(TriggerZone, None);

impl<'lua> TriggerZone<'lua> {
    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn pos(&self) -> LuaResult<na::base::Vector2<f64>> {
        Ok(na::base::Vector2::new(
            self.raw_get("x")?,
            self.raw_get("y")?,
        ))
    }

    pub fn typ(&self) -> LuaResult<TriggerZoneTyp> {
        Ok(match self.raw_get("type")? {
            0 => TriggerZoneTyp::Circle {
                radius: self.raw_get("radius")?,
            },
            2 => TriggerZoneTyp::Quad(self.raw_get("vertices")?),
            _ => return Err(cvt_err("TriggerZoneTyp")),
        })
    }

    pub fn color(&self) -> LuaResult<Color> {
        self.raw_get("color")
    }
}

wrapped_table!(NavPoint, None);

wrapped_table!(Task, None);

wrapped_table!(Point, None);

wrapped_table!(Route, None);

impl<'lua> Route<'lua> {
    pub fn points(&self) -> LuaResult<Sequence<Point>> {
        self.raw_get("points")
    }
}

wrapped_table!(Unit, None);

impl<'lua> Unit<'lua> {
    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn id(&self) -> LuaResult<UnitId> {
        self.raw_get("unitId")
    }

    pub fn set_name(&self, name: String) -> LuaResult<()> {
        self.raw_set("name", name)
    }

    pub fn pos(&self) -> LuaResult<na::base::Vector2<f64>> {
        Ok(na::base::Vector2::new(
            self.raw_get("x")?,
            self.raw_get("y")?,
        ))
    }

    pub fn set_pos(&self, pos: na::base::Vector2<f64>) -> LuaResult<()> {
        self.raw_set("x", pos.x)?;
        self.raw_set("y", pos.y)
    }

    pub fn typ(&self) -> LuaResult<String> {
        self.raw_get("type")
    }
}

wrapped_table!(Group, None);

impl<'lua> Group<'lua> {
    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn set_name(&self, name: String) -> LuaResult<()> {
        self.raw_set("name", name)
    }

    pub fn pos(&self) -> LuaResult<na::base::Vector2<f64>> {
        Ok(na::base::Vector2::new(
            self.t.raw_get("x")?,
            self.t.raw_get("y")?,
        ))
    }

    pub fn set_pos(&self, pos: na::base::Vector2<f64>) -> LuaResult<()> {
        self.t.raw_set("x", pos.x)?;
        self.t.raw_set("y", pos.y)
    }

    pub fn frequency(&self) -> LuaResult<f64> {
        self.raw_get("frequency")
    }

    pub fn modulation(&self) -> LuaResult<i64> {
        self.raw_get("modulation")
    }

    pub fn late_activation(&self) -> bool {
        self.raw_get("lateActivation").unwrap_or(false)
    }

    pub fn id(&self) -> LuaResult<GroupId> {
        self.raw_get("groupId")
    }

    pub fn tasks(&self) -> LuaResult<Sequence<Task>> {
        self.raw_get("tasks")
    }

    pub fn route(&self) -> LuaResult<Route> {
        self.raw_get("route")
    }

    pub fn hidden(&self) -> bool {
        self.raw_get("hidden").unwrap_or(false)
    }

    pub fn units(&self) -> LuaResult<Sequence<Unit>> {
        self.raw_get("units")
    }

    pub fn uncontrolled(&self) -> bool {
        self.raw_get("uncontrolled").unwrap_or(true)
    }
}

wrapped_table!(Country, None);

impl<'lua> Country<'lua> {
    pub fn id(&self) -> LuaResult<country::Country> {
        self.raw_get("id")
    }

    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn planes(&self) -> LuaResult<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("plane")?;
        g.map(|g| g.raw_get("group"))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn helicopters(&self) -> LuaResult<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("helicopter")?;
        g.map(|g| g.raw_get("group"))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn ships(&self) -> LuaResult<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("ship")?;
        g.map(|g| g.raw_get("group"))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn vehicles(&self) -> LuaResult<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("vehicle")?;
        g.map(|g| g.raw_get("group"))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn statics(&self) -> LuaResult<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("static")?;
        g.map(|g| g.raw_get("group"))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }
}

wrapped_table!(Coalition, None);

impl<'lua> Coalition<'lua> {
    pub fn bullseye(&self) -> LuaResult<LuaVec2> {
        self.t.raw_get("bullseye")
    }

    pub fn nav_points(&self) -> LuaResult<Sequence<NavPoint>> {
        self.t.raw_get("nav_points")
    }

    pub fn name(&self) -> LuaResult<String> {
        self.t.raw_get("name")
    }

    pub fn countries(&self) -> LuaResult<Sequence<Country>> {
        self.t.raw_get("country")
    }

    fn index(&self, base: Path) -> LuaResult<CoalitionIndex> {
        let base = base.append(["country"]);
        let mut idx = CoalitionIndex::default();
        for (i, country) in self.countries()?.into_iter().enumerate() {
            let country = country?;
            let cid = country.id()?;
            let base = base.append([i + 1]);
            macro_rules! index_group {
                ($name:literal, $cat:expr, $tbl:ident) => {
                    for (i, group) in country.$tbl()?.into_iter().enumerate() {
                        let group = group?;
                        let name = group.name()?;
                        let gid = group.id()?;
                        let base = base.append([$name, "group"]).append([i + 1]);
                        match idx.groups.entry(gid) {
                            Entry::Occupied(_) => return Err(cvt_err($name)),
                            Entry::Vacant(e) => {
                                e.insert(IndexedGroup {
                                    country: cid,
                                    category: $cat,
                                    path: base.clone(),
                                });
                            }
                        }
                        match idx.groups_by_name.entry(name.clone()) {
                            Entry::Occupied(_) => return Err(cvt_err($name)),
                            Entry::Vacant(e) => e.insert(gid),
                        };
                        match idx.$tbl.entry(name) {
                            Entry::Occupied(_) => return Err(cvt_err($name)),
                            Entry::Vacant(e) => e.insert(gid),
                        };
                        for (i, unit) in group.units()?.into_iter().enumerate() {
                            let unit = unit?;
                            let base = base.append(["units"]).append([i + 1]);
                            let name = unit.name()?;
                            let uid = unit.id()?;
                            match idx.units.entry(uid) {
                                Entry::Occupied(_) => return Err(cvt_err($name)),
                                Entry::Vacant(e) => e.insert(base.clone()),
                            };
                            match idx.units_by_name.entry(name) {
                                Entry::Occupied(_) => return Err(cvt_err($name)),
                                Entry::Vacant(e) => e.insert(uid),
                            };
                            match idx.groups_by_unit.entry(uid) {
                                Entry::Occupied(_) => return Err(cvt_err($name)),
                                Entry::Vacant(e) => e.insert(gid),
                            };
                        }
                    }
                };
            }
            index_group!("plane", GroupKind::Plane, planes);
            index_group!("helicopter", GroupKind::Helicopter, helicopters);
            index_group!("ship", GroupKind::Ship, ships);
            index_group!("vehicle", GroupKind::Vehicle, vehicles);
            index_group!("static", GroupKind::Static, statics);
        }
        Ok(idx)
    }
}

#[derive(Debug, Clone, Serialize)]
struct IndexedGroup {
    country: country::Country,
    category: GroupKind,
    path: Path,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CoalitionIndex {
    units: FxHashMap<UnitId, Path>,
    units_by_name: FxHashMap<String, UnitId>,
    groups: FxHashMap<GroupId, IndexedGroup>,
    groups_by_name: FxHashMap<String, GroupId>,
    groups_by_unit: FxHashMap<UnitId, GroupId>,
    planes: FxHashMap<String, GroupId>,
    helicopters: FxHashMap<String, GroupId>,
    ships: FxHashMap<String, GroupId>,
    vehicles: FxHashMap<String, GroupId>,
    statics: FxHashMap<String, GroupId>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct MizIndex {
    by_side: FxHashMap<Side, CoalitionIndex>,
    triggers: FxHashMap<String, Path>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum GroupKind {
    Any,
    Plane,
    Helicopter,
    Ship,
    Vehicle,
    Static,
}

#[derive(Debug, Clone, Serialize)]
pub struct GroupInfo<'lua> {
    pub country: country::Country,
    pub category: GroupKind,
    pub group: Group<'lua>,
}

wrapped_table!(Miz, None);

impl<'lua> Miz<'lua> {
    pub fn singleton<L: LuaEnv<'lua> + Copy>(lua: L) -> LuaResult<Self> {
        if is_hooks_env(lua.inner()) {
            let current: mlua::Table = lua.inner().globals().get("_current_mission")?;
            current.get("mission")
        } else {
            let env: mlua::Table = lua.inner().globals().get("env")?;
            env.get("mission")
        }
    }

    pub fn coalition(&self, side: Side) -> LuaResult<Coalition<'lua>> {
        let coa: mlua::Table = self.raw_get("coalition")?;
        coa.raw_get(side.to_str())
    }

    pub fn triggers(&self) -> LuaResult<Sequence<TriggerZone>> {
        let triggers: mlua::Table = self.t.raw_get("triggers")?;
        triggers.raw_get("zones")
    }

    pub fn weather(&self) -> LuaResult<Weather<'lua>> {
        self.t.raw_get("weather")
    }

    pub fn get_group(&self, idx: &MizIndex, id: &GroupId) -> LuaResult<Option<GroupInfo>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.groups.get(id))
            .map(|ifo| {
                self.raw_get_path(&ifo.path).map(|group| GroupInfo {
                    country: ifo.country,
                    category: ifo.category,
                    group,
                })
            })
            .transpose()
    }

    pub fn get_group_by_name(
        &self,
        idx: &MizIndex,
        kind: GroupKind,
        side: Side,
        name: &str,
    ) -> LuaResult<Option<GroupInfo>> {
        idx.by_side
            .get(&side)
            .and_then(|cidx| match kind {
                GroupKind::Any => cidx.groups_by_name.get(name),
                GroupKind::Plane => cidx.planes.get(name),
                GroupKind::Helicopter => cidx.helicopters.get(name),
                GroupKind::Vehicle => cidx.vehicles.get(name),
                GroupKind::Ship => cidx.ships.get(name),
                GroupKind::Static => cidx.statics.get(name),
            })
            .and_then(|gid| self.get_group(idx, gid).transpose())
            .transpose()
    }

    pub fn get_unit(&self, idx: &MizIndex, id: &UnitId) -> LuaResult<Option<Unit>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.units.get(id))
            .map(|path| self.raw_get_path(path))
            .transpose()
    }

    pub fn get_unit_by_name(&self, idx: &MizIndex, name: &str) -> LuaResult<Option<Unit>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.units_by_name.get(name).and_then(|id| idx.units.get(id)))
            .map(|path| self.raw_get_path(path))
            .transpose()
    }

    pub fn get_group_by_unit(&self, idx: &MizIndex, id: &UnitId) -> LuaResult<Option<GroupInfo>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.groups_by_unit.get(id))
            .and_then(|gid| self.get_group(idx, gid).transpose())
            .transpose()
    }

    pub fn get_group_by_unit_name(
        &self,
        idx: &MizIndex,
        name: &str,
    ) -> LuaResult<Option<GroupInfo>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| {
                idx.units_by_name
                    .get(name)
                    .and_then(|uid| idx.groups_by_unit.get(uid))
            })
            .and_then(|gid| self.get_group(idx, gid).transpose())
            .transpose()
    }

    pub fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> LuaResult<Option<TriggerZone>> {
        idx.triggers
            .get(name)
            .map(|path| self.raw_get_path(path))
            .transpose()
    }

    pub fn sortie(&self) -> LuaResult<String> {
        self.raw_get("sortie")
    }

    pub fn index(&self) -> LuaResult<MizIndex> {
        let base = Path::default();
        let mut idx = MizIndex::default();
        {
            let base = base.append(["triggers", "zones"]);
            for (i, tz) in self.triggers()?.into_iter().enumerate() {
                let tz = tz?;
                let base = base.append([i + 1]);
                match idx.triggers.entry(tz.name()?) {
                    Entry::Vacant(e) => {
                        e.insert(base);
                    }
                    Entry::Occupied(_) => return Err(cvt_err("duplicate trigger zone")),
                }
            }
        }
        for side in [Side::Blue, Side::Red, Side::Neutral] {
            let base = base.append(["coalition", side.to_str()]);
            idx.by_side
                .entry(side)
                .or_insert(self.coalition(side)?.index(base)?);
        }
        Ok(idx)
    }
}
