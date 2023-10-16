use crate::{
    as_tbl, coalition::Side, country, cvt_err, is_hooks_env, wrapped_table, DcsTableExt, Path,
    Sequence, String, LuaVec2,
};
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use serde_derive::{Serialize, Deserialize};
use std::{collections::hash_map::Entry, ops::Deref};

wrapped_table!(Weather, None);

pub struct Quad {
    pub p0: LuaVec2,
    pub p1: LuaVec2,
    pub p2: LuaVec2,
    pub p3: LuaVec2,
}

impl<'lua> FromLua<'lua> for Quad {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let verts = as_tbl("Quad", None, value)?;
        Ok(Self {
            p0: verts.raw_get(1)?,
            p1: verts.raw_get(2)?,
            p2: verts.raw_get(3)?,
            p3: verts.raw_get(4)?,
        })
    }
}

pub enum TriggerZoneTyp {
    Circle { radius: f64 },
    Quad(Quad),
}

#[derive(Clone, Copy, PartialEq, Serialize)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl<'lua> FromLua<'lua> for Color {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Color", None, value)?;
        Ok(Self {
            r: tbl.raw_get(1)?,
            g: tbl.raw_get(2)?,
            b: tbl.raw_get(3)?,
            a: tbl.raw_get(4)?,
        })
    }
}

impl<'lua> IntoLua<'lua> for Color {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let tbl = lua.create_table()?;
        tbl.set(1, self.r)?;
        tbl.set(2, self.g)?;
        tbl.set(3, self.b)?;
        tbl.set(4, self.a)?;
        Ok(Value::Table(tbl))
    }
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

    pub fn group_id(&self) -> LuaResult<i64> {
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

    pub fn uncontrollable(&self) -> bool {
        self.raw_get("uncontrollable").unwrap_or(true)
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
                        let base = base.append([$name, "group"]).append([i + 1]);
                        match idx.$tbl.entry(group.name()?) {
                            Entry::Occupied(_) => return Err(cvt_err($name)),
                            Entry::Vacant(e) => {
                                e.insert(IndexedGroup {
                                    country: cid,
                                    category: $cat,
                                    path: base.clone(),
                                });
                            }
                        }
                        match idx.all.entry(group.name()?) {
                            Entry::Occupied(_) => return Err(cvt_err($name)),
                            Entry::Vacant(e) => {
                                e.insert(IndexedGroup {
                                    country: cid,
                                    category: $cat,
                                    path: base,
                                });
                            }
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
    all: FxHashMap<String, IndexedGroup>,
    planes: FxHashMap<String, IndexedGroup>,
    helicopters: FxHashMap<String, IndexedGroup>,
    ships: FxHashMap<String, IndexedGroup>,
    vehicles: FxHashMap<String, IndexedGroup>,
    statics: FxHashMap<String, IndexedGroup>,
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
    pub fn singleton(lua: &'lua Lua) -> LuaResult<Self> {
        if is_hooks_env(lua) {
            let current: mlua::Table = lua.globals().get("_current_mission")?;
            current.get("mission")
        } else {
            let env: mlua::Table = lua.globals().get("env")?;
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

    pub fn get_group(
        &self,
        idx: &MizIndex,
        kind: GroupKind,
        side: Side,
        name: &str,
    ) -> LuaResult<Option<GroupInfo>> {
        idx.by_side
            .get(&side)
            .and_then(|cidx| match kind {
                GroupKind::Any => cidx.all.get(name),
                GroupKind::Plane => cidx.planes.get(name),
                GroupKind::Helicopter => cidx.helicopters.get(name),
                GroupKind::Vehicle => cidx.vehicles.get(name),
                GroupKind::Ship => cidx.ships.get(name),
                GroupKind::Static => cidx.statics.get(name),
            })
            .map(|ifo| {
                self.raw_get_path(&ifo.path).map(|group| GroupInfo {
                    country: ifo.country,
                    category: ifo.category,
                    group,
                })
            })
            .transpose()
    }

    pub fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> LuaResult<Option<TriggerZone>> {
        idx.triggers
            .get(name)
            .map(|path| self.raw_get_path(path))
            .transpose()
    }

    pub fn index(&self) -> LuaResult<MizIndex> {
        let base = Path::default();
        let mut idx = MizIndex::default();
        {
            let base = base.append(["triggers", "zones"]);
            println!("indexing triggers");
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
