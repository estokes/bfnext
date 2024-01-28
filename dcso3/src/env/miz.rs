/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use crate::{
    as_tbl, coalition::Side, country, is_hooks_env, wrapped_prim, wrapped_table, Color,
    DcsTableExt, LuaEnv, LuaVec2, Path, Quad2, Sequence, String, net::SlotId, string_enum,
};
use anyhow::{bail, Result};
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::{collections::hash_map::Entry, ops::Deref};

wrapped_table!(Weather, None);

wrapped_prim!(UnitId, i64, Hash, Copy);
wrapped_prim!(GroupId, i64, Hash, Copy);

string_enum!(PointType, u8, [
    TakeOffGround => "TakeOffGround",
    TakeOffGroundHot => "TakeOffGroundHot",
    TurningPoint => "Turning Point",
    TakeOffParking => "TakeOffParking",
    TakeOff => "TakeOff",
    Land => "Land",
    Nil => ""
]);

string_enum!(Skill, u8, [
    Client => "Client",
    Excellant => "Excellant"
]);

#[derive(Debug, Clone, Copy)]
pub enum TriggerZoneTyp {
    Circle { radius: f64 },
    Quad(Quad2),
}

wrapped_prim!(TriggerZoneId, i64, Copy, Hash);

wrapped_table!(TriggerZone, None);

impl<'lua> TriggerZone<'lua> {
    pub fn name(&self) -> Result<String> {
        Ok(self.raw_get("name")?)
    }

    pub fn pos(&self) -> Result<na::base::Vector2<f64>> {
        Ok(na::base::Vector2::new(
            self.raw_get("x")?,
            self.raw_get("y")?,
        ))
    }

    pub fn typ(&self) -> Result<TriggerZoneTyp> {
        Ok(match self.raw_get("type")? {
            0 => TriggerZoneTyp::Circle {
                radius: self.raw_get("radius")?,
            },
            2 => TriggerZoneTyp::Quad(self.raw_get("verticies")?),
            n => bail!("unknown trigger zone type {}", n),
        })
    }

    pub fn color(&self) -> Result<Color> {
        Ok(self.raw_get("color")?)
    }

    pub fn id(&self) -> Result<TriggerZoneId> {
        Ok(self.raw_get("zoneId")?)
    }
}

wrapped_table!(NavPoint, None);

wrapped_table!(Task, None);

wrapped_table!(Point, None);

impl<'lua> Point<'lua> {
    pub fn typ(&self) -> Result<PointType> {
        Ok(self.t.raw_get("type")?)
    }
}

wrapped_table!(Route, None);

impl<'lua> Route<'lua> {
    pub fn points(&self) -> Result<Sequence<Point>> {
        Ok(self.raw_get("points")?)
    }
}

wrapped_table!(Unit, None);

impl<'lua> Unit<'lua> {
    pub fn name(&self) -> Result<String> {
        Ok(self.raw_get("name")?)
    }

    pub fn id(&self) -> Result<UnitId> {
        Ok(self.raw_get("unitId")?)
    }

    pub fn slot(&self) -> Result<SlotId> {
        Ok(SlotId::from(self.id()?))
    }

    pub fn set_name(&self, name: String) -> Result<()> {
        Ok(self.raw_set("name", name)?)
    }

    pub fn pos(&self) -> Result<na::base::Vector2<f64>> {
        Ok(na::base::Vector2::new(
            self.raw_get("x")?,
            self.raw_get("y")?,
        ))
    }

    pub fn set_pos(&self, pos: na::base::Vector2<f64>) -> Result<()> {
        self.raw_set("x", pos.x)?;
        self.raw_set("y", pos.y)?;
        Ok(())
    }

    pub fn heading(&self) -> Result<f64> {
        Ok(self.raw_get("heading")?)
    }

    pub fn set_heading(&self, h: f64) -> Result<()> {
        Ok(self.raw_set("heading", h)?)
    }

    pub fn typ(&self) -> Result<String> {
        Ok(self.raw_get("type")?)
    }

    pub fn skill(&self) -> Result<Skill> {
        Ok(self.raw_get("skill")?)
    }
}

wrapped_table!(Group, None);

impl<'lua> Group<'lua> {
    pub fn name(&self) -> Result<String> {
        Ok(self.raw_get("name")?)
    }

    pub fn set_name(&self, name: String) -> Result<()> {
        Ok(self.raw_set("name", name)?)
    }

    pub fn pos(&self) -> Result<na::base::Vector2<f64>> {
        Ok(na::base::Vector2::new(
            self.t.raw_get("x")?,
            self.t.raw_get("y")?,
        ))
    }

    pub fn set_pos(&self, pos: na::base::Vector2<f64>) -> Result<()> {
        self.t.raw_set("x", pos.x)?;
        self.t.raw_set("y", pos.y)?;
        Ok(())
    }

    pub fn frequency(&self) -> Result<f64> {
        Ok(self.raw_get("frequency")?)
    }

    pub fn modulation(&self) -> Result<i64> {
        Ok(self.raw_get("modulation")?)
    }

    pub fn late_activation(&self) -> bool {
        self.raw_get("lateActivation").unwrap_or(false)
    }

    pub fn id(&self) -> Result<GroupId> {
        Ok(self.raw_get("groupId")?)
    }

    pub fn tasks(&self) -> Result<Sequence<Task>> {
        Ok(self.raw_get("tasks")?)
    }

    pub fn route(&self) -> Result<Route> {
        Ok(self.raw_get("route")?)
    }

    pub fn hidden(&self) -> bool {
        self.raw_get("hidden").unwrap_or(false)
    }

    pub fn units(&self) -> Result<Sequence<Unit>> {
        Ok(self.raw_get("units")?)
    }

    pub fn uncontrolled(&self) -> bool {
        self.raw_get("uncontrolled").unwrap_or(true)
    }
}

wrapped_table!(Country, None);

impl<'lua> Country<'lua> {
    pub fn id(&self) -> Result<country::Country> {
        Ok(self.raw_get("id")?)
    }

    pub fn name(&self) -> Result<String> {
        Ok(self.raw_get("name")?)
    }

    pub fn planes(&self) -> Result<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("plane")?;
        g.map(|g| Ok(g.raw_get("group")?))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn helicopters(&self) -> Result<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("helicopter")?;
        g.map(|g| Ok(g.raw_get("group")?))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn ships(&self) -> Result<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("ship")?;
        g.map(|g| Ok(g.raw_get("group")?))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn vehicles(&self) -> Result<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("vehicle")?;
        g.map(|g| Ok(g.raw_get("group")?))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }

    pub fn statics(&self) -> Result<Sequence<Group>> {
        let g: Option<mlua::Table> = self.raw_get("static")?;
        g.map(|g| Ok(g.raw_get("group")?))
            .unwrap_or_else(|| Sequence::empty(self.lua))
    }
}

wrapped_table!(Coalition, None);

impl<'lua> Coalition<'lua> {
    pub fn bullseye(&self) -> Result<LuaVec2> {
        Ok(self.t.raw_get("bullseye")?)
    }

    pub fn nav_points(&self) -> Result<Sequence<NavPoint>> {
        Ok(self.t.raw_get("nav_points")?)
    }

    pub fn name(&self) -> Result<String> {
        Ok(self.t.raw_get("name")?)
    }

    pub fn countries(&self) -> Result<Sequence<Country>> {
        Ok(self.t.raw_get("country")?)
    }

    fn index(&self, side: Side, base: Path) -> Result<CoalitionIndex> {
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
                            Entry::Occupied(_) => bail!("duplicate group id {:?}", gid),
                            Entry::Vacant(e) => {
                                e.insert(IndexedGroup {
                                    side,
                                    country: cid,
                                    category: $cat,
                                    path: base.clone(),
                                });
                            }
                        }
                        match idx.groups_by_name.entry(name.clone()) {
                            Entry::Occupied(_) => bail!("duplicate group name {name}"),
                            Entry::Vacant(e) => e.insert(gid),
                        };
                        match idx.$tbl.entry(name.clone()) {
                            Entry::Occupied(_) => bail!("duplicate group name {name}"),
                            Entry::Vacant(e) => e.insert(gid),
                        };
                        for (i, unit) in group.units()?.into_iter().enumerate() {
                            let unit = unit?;
                            let base = base.append(["units"]).append([i + 1]);
                            let name = unit.name()?;
                            let uid = unit.id()?;
                            match idx.units.entry(uid) {
                                Entry::Occupied(_) => bail!("duplicate unit id {:?}", uid),
                                Entry::Vacant(e) => e.insert(IndexedUnit {
                                    side,
                                    country: cid,
                                    path: base.clone(),
                                }),
                            };
                            match idx.units_by_name.entry(name.clone()) {
                                Entry::Occupied(_) => bail!("duplicate unit name {name}"),
                                Entry::Vacant(e) => e.insert(uid),
                            };
                            match idx.groups_by_unit.entry(uid) {
                                Entry::Occupied(_) => bail!("guplicate unit id {:?}", uid),
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

#[derive(Debug, Clone, Copy)]
pub struct Role {
    pub neutrals: u8,
    pub red: u8,
    pub blue: u8
}

impl<'lua> FromLua<'lua> for Role {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: LuaTable = FromLua::from_lua(value, lua)?;
        Ok(Self {
            neutrals: tbl.raw_get("neutrals")?,
            red: tbl.raw_get("red")?,
            blue: tbl.raw_get("blue")?
        })
    }
}

wrapped_table!(Roles, None);

impl<'lua> Roles<'lua> {
    pub fn artillery_commander(&self) -> Result<Role> {
        Ok(self.raw_get("artillery_commander")?)
    }

    pub fn instructor(&self) -> Result<Role> {
        Ok(self.raw_get("instructor")?)
    }

    pub fn observer(&self) -> Result<Role> {
        Ok(self.raw_get("observer")?)
    }

    pub fn forward_observer(&self) -> Result<Role> {
        Ok(self.raw_get("forward_observer")?)
    }
}

wrapped_table!(GroundControl, None);

impl<'lua> GroundControl<'lua> {
    pub fn roles(&self) -> Result<Roles<'lua>> {
        Ok(self.raw_get("roles")?)
    }
}

#[derive(Debug, Clone, Serialize)]
struct IndexedGroup {
    side: Side,
    country: country::Country,
    category: GroupKind,
    path: Path,
}

#[derive(Debug, Clone, Serialize)]
struct IndexedUnit {
    side: Side,
    country: country::Country,
    path: Path,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct CoalitionIndex {
    units: FxHashMap<UnitId, IndexedUnit>,
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
    pub side: Side,
    pub country: country::Country,
    pub category: GroupKind,
    pub group: Group<'lua>,
}

#[derive(Debug, Clone, Serialize)]
pub struct UnitInfo<'lua> {
    pub side: Side,
    pub country: country::Country,
    pub unit: Unit<'lua>,
}

wrapped_table!(Miz, None);

impl<'lua> Miz<'lua> {
    pub fn singleton<L: LuaEnv<'lua> + Copy>(lua: L) -> Result<Self> {
        if is_hooks_env(lua.inner()) {
            let current: mlua::Table = lua.inner().globals().get("_current_mission")?;
            Ok(current.get("mission")?)
        } else {
            let env: mlua::Table = lua.inner().globals().get("env")?;
            Ok(env.get("mission")?)
        }
    }

    pub fn ground_control(&self) -> Result<GroundControl> {
        Ok(self.raw_get("groundControl")?)
    }

    pub fn coalition(&self, side: Side) -> Result<Coalition<'lua>> {
        let coa: mlua::Table = self.raw_get("coalition")?;
        Ok(coa.raw_get(side.to_str())?)
    }

    pub fn triggers(&self) -> Result<Sequence<TriggerZone<'lua>>> {
        let triggers: mlua::Table = self.t.raw_get("triggers")?;
        Ok(triggers.raw_get("zones")?)
    }

    pub fn weather(&self) -> Result<Weather<'lua>> {
        Ok(self.t.raw_get("weather")?)
    }

    pub fn get_group(&self, idx: &MizIndex, id: &GroupId) -> Result<Option<GroupInfo<'lua>>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.groups.get(id))
            .map(|ifo| {
                self.raw_get_path(&ifo.path).map(|group| GroupInfo {
                    side: ifo.side,
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
    ) -> Result<Option<GroupInfo<'lua>>> {
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

    pub fn get_unit(&self, idx: &MizIndex, id: &UnitId) -> Result<Option<UnitInfo<'lua>>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.units.get(id))
            .map(|ifo| {
                self.raw_get_path(&ifo.path).map(|unit| UnitInfo {
                    side: ifo.side,
                    country: ifo.country,
                    unit,
                })
            })
            .transpose()
    }

    pub fn get_unit_by_name(&self, idx: &MizIndex, name: &str) -> Result<Option<UnitInfo<'lua>>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.units_by_name.get(name).and_then(|id| idx.units.get(id)))
            .map(|ifo| {
                self.raw_get_path(&ifo.path).map(|unit| UnitInfo {
                    side: ifo.side,
                    country: ifo.country,
                    unit,
                })
            })
            .transpose()
    }

    pub fn get_group_by_unit(&self, idx: &MizIndex, id: &UnitId) -> Result<Option<GroupInfo<'lua>>> {
        idx.by_side
            .iter()
            .find_map(|(_, idx)| idx.groups_by_unit.get(id))
            .and_then(|gid| self.get_group(idx, gid).transpose())
            .transpose()
    }

    pub fn get_group_by_unit_name(&self, idx: &MizIndex, name: &str) -> Result<Option<GroupInfo<'lua>>> {
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

    pub fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> Result<Option<TriggerZone<'lua>>> {
        idx.triggers
            .get(name)
            .map(|path| self.raw_get_path(path))
            .transpose()
    }

    pub fn sortie(&self) -> Result<String> {
        Ok(self.raw_get("sortie")?)
    }

    pub fn index(&self) -> Result<MizIndex> {
        let base = Path::default();
        let mut idx = MizIndex::default();
        {
            let base = base.append(["triggers", "zones"]);
            for (i, tz) in self.triggers()?.into_iter().enumerate() {
                let tz = tz?;
                let base = base.append([i + 1]);
                let name = tz.name()?;
                match idx.triggers.entry(name.clone()) {
                    Entry::Vacant(e) => e.insert(base),
                    Entry::Occupied(_) => bail!("duplicate trigger zone {name}"),
                };
            }
        }
        for side in Side::ALL {
            let base = base.append(["coalition", side.to_str()]);
            idx.by_side
                .entry(side)
                .or_insert(self.coalition(side)?.index(side, base)?);
        }
        Ok(idx)
    }
}
