/*
Copyright 2024 Eric Stokes.

This file is part of bflib.

bflib is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your
option) any later version.

bflib is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
for more details.
*/

use crate::{
    cfg::{UnitTag, UnitTags},
    db::{
        group::{GroupId, SpawnedUnit, UnitId},
        objective::ObjectiveId,
        Db,
    },
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    controller::Task,
    cvt_err, err,
    group::Group,
    land::Land,
    net::SlotId,
    normal2,
    object::{DcsObject, DcsOid},
    radians_to_degrees, simple_enum,
    spot::{ClassSpot, Spot},
    trigger::{MarkId, SmokeColor, Trigger},
    unit::{ClassUnit, Unit},
    LuaVec2, LuaVec3, MizLua, String, Vector2, Vector3,
};
use enumflags2::BitFlags;
use fxhash::{FxHashMap, FxHashSet};
use indexmap::IndexMap;
use log::{info, warn};
use mlua::{prelude::LuaResult, FromLua, IntoLua, Lua, Table, Value};
use rand::{thread_rng, Rng};
use serde::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::{collections::hash_map::Entry, fmt};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum JtId {
    Group(GroupId),
    Slot(SlotId),
}

impl<'lua> FromLua<'lua> for JtId {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl: Table = FromLua::from_lua(value, lua)?;
        match tbl.raw_get::<_, i64>("kind")? {
            0 => Ok(Self::Group(tbl.raw_get("id")?)),
            1 => Ok(Self::Slot(tbl.raw_get("id")?)),
            n => Err(err(&format_compact!("invalid jtid {n}"))),
        }
    }
}

impl<'lua> IntoLua<'lua> for JtId {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let tbl = lua.create_table()?;
        match self {
            Self::Group(id) => {
                tbl.raw_set("kind", 0)?;
                tbl.raw_set("id", id)?
            }
            Self::Slot(id) => {
                tbl.raw_set("kind", 1)?;
                tbl.raw_set("id", id)?;
            }
        }
        Ok(Value::Table(tbl))
    }
}

impl fmt::Display for JtId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Group(id) => write!(f, "{id}"),
            Self::Slot(id) => write!(f, "sl{id}"),
        }
    }
}

fn ui_jtac_dead(db: &mut Db, side: Side, gid: JtId) {
    db.ephemeral.msgs().panel_to_side(
        10,
        false,
        side,
        format_compact!("JTAC {gid} is no longer available"),
    )
}

simple_enum!(AdjustmentDir, u8, [
    Short => 0,
    Long => 1,
    Left => 2,
    Right => 3
]);

#[derive(Debug, Clone, Copy, Default)]
pub struct ArtilleryAdjustment {
    pub short_long: i16,
    pub left_right: i16,
}

impl ArtilleryAdjustment {
    fn compute_final_solution(&self, ip: Vector2, tp: Vector2) -> Vector2 {
        let v = (tp - ip).normalize();
        let normal = normal2(v) * self.left_right.signum() as f64;
        tp + (v * self.short_long as f64) + (normal * self.left_right as f64)
    }
}

type LocByCode = FxHashMap<ObjectiveId, FxHashMap<u16, FxHashSet<JtId>>>;

#[derive(Debug, Clone, Default)]
pub struct Contact {
    pub pos: Vector3,
    pub typ: String,
    pub tags: UnitTags,
    pub last_move: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct JtacTarget {
    uid: UnitId,
    source: DcsOid<ClassUnit>,
    spot: DcsOid<ClassSpot>,
    ir_pointer: Option<DcsOid<ClassSpot>>,
    mark: Option<MarkId>,
}

impl JtacTarget {
    fn destroy(self, lua: MizLua) -> Result<()> {
        Spot::get_instance(lua, &self.spot)
            .context("getting laser spot")?
            .destroy()
            .context("destroying laser spot")?;
        if let Some(ir_pointer) = self.ir_pointer {
            Spot::get_instance(lua, &ir_pointer)
                .context("getting ir pointer")?
                .destroy()
                .context("destroying ir pointer")?
        }
        if let Some(id) = self.mark {
            Trigger::singleton(lua)?
                .action()?
                .remove_mark(id)
                .context("removing mark")?
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy)]
pub struct JtacLocation {
    pub pos: Vector2,
    pub oid: ObjectiveId,
    pub bearing: f64,
    pub distance: f64,
}

impl JtacLocation {
    fn new(db: &Db, pos: Vector3) -> Self {
        let pos = Vector2::new(pos.x, pos.z);
        let (distance, bearing, obj) =
            Db::objective_near_point(&db.persisted.objectives, pos, |_| true).unwrap();
        Self {
            pos,
            oid: obj.id,
            bearing,
            distance,
        }
    }
}

pub struct ContactsIter<'a> {
    contacts: Vec<indexmap::map::Iter<'a, UnitId, Contact>>,
    i: usize,
}

impl<'a> Iterator for ContactsIter<'a> {
    type Item = (&'a UnitId, &'a Contact);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.contacts.len() == 0 {
                break None;
            }
            if self.i < self.contacts.len() {
                match self.contacts[self.i].next() {
                    Some(item) => {
                        self.i += 1;
                        break Some(item);
                    }
                    None => {
                        self.contacts.remove(self.i);
                        self.i += 1;
                    }
                }
            } else {
                self.i = 0;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct Jtac {
    pub gid: JtId,
    pub side: Side,
    contacts: IndexMap<UnitId, Contact>,
    filter: BitFlags<UnitTag>,
    pub location: JtacLocation,
    priority: Vec<UnitTags>,
    target: Option<JtacTarget>,
    autoshift: bool,
    ir_pointer: bool,
    code: u16,
    last_smoke: DateTime<Utc>,
    pub nearby_artillery: SmallVec<[GroupId; 8]>,
    menu_dirty: bool,
}

impl Jtac {
    fn new(db: &Db, gid: JtId, side: Side, priority: Vec<UnitTags>, pos: Vector3) -> Self {
        Self {
            gid,
            side,
            contacts: IndexMap::default(),
            filter: BitFlags::default(),
            priority,
            location: JtacLocation::new(db, pos),
            target: None,
            autoshift: true,
            ir_pointer: false,
            code: 1688,
            last_smoke: DateTime::<Utc>::default(),
            nearby_artillery: smallvec![],
            menu_dirty: false,
        }
    }

    fn status(&self, db: &Db, loc_by_code: &LocByCode) -> Result<CompactString> {
        use std::fmt::Write;
        let mut msg = CompactString::new("");
        write!(msg, "JTAC {} status\n", self.gid)?;
        match &self.target {
            None => {
                write!(msg, "no target\n")?;
            }
            Some(target) => {
                let unit_typ = db.unit(&target.uid)?.typ.clone();
                let mid = match target.mark {
                    None => format_compact!("none"),
                    Some(mid) => format_compact!("{mid}"),
                };
                let conflicts = loc_by_code
                    .get(&self.location.oid)
                    .and_then(|by_code| by_code.get(&self.code))
                    .and_then(|gids| {
                        let len = gids.len();
                        if len <= 1 {
                            None
                        } else {
                            let mut msg = CompactString::new("(code conflicts with [");
                            for (i, gid) in gids.iter().filter(|gid| **gid != self.gid).enumerate()
                            {
                                if i < len - 2 {
                                    write!(msg, "{gid}, ").unwrap()
                                } else {
                                    write!(msg, "{gid}").unwrap()
                                }
                            }
                            write!(msg, "])").unwrap();
                            Some(String::from(msg))
                        }
                    })
                    .unwrap_or(String::from(""));
                write!(
                    msg,
                    "lasing {unit_typ} code {}{} marker {mid}\n",
                    self.code, conflicts
                )?;
            }
        };
        write!(
            msg,
            "position bearing {} for {:.1}km from {}\n\n",
            radians_to_degrees(self.location.bearing) as u32,
            self.location.distance / 1000.,
            db.objective(&self.location.oid)?.name
        )?;
        if self.contacts.is_empty() {
            write!(msg, "No enemies in sight")?;
        } else {
            write!(msg, "Visual On: ")?;
            for (i, uid) in self.contacts.keys().enumerate() {
                if i == self.contacts.len() - 1 {
                    write!(msg, "{}", db.unit(uid)?.typ)?;
                } else {
                    write!(msg, "{}, ", db.unit(uid)?.typ)?;
                }
            }
        }
        write!(
            msg,
            "\n\nautoshift: {}, ir_pointer: {}",
            self.autoshift, self.ir_pointer
        )?;
        write!(msg, "\nfilter: [")?;
        let len = self.filter.len();
        for (i, tag) in self.filter.iter().enumerate() {
            if i < len - 1 {
                write!(msg, "{:?}, ", tag)?;
            } else {
                write!(msg, "{:?}", tag)?;
            }
        }
        write!(msg, "]\n")?;
        write!(msg, "available artillery: [")?;
        let len = self.nearby_artillery.len();
        for (i, gid) in self.nearby_artillery.iter().enumerate() {
            if i < len - 1 {
                write!(msg, "{gid},")?;
            } else {
                write!(msg, "{gid}")?;
            }
        }
        write!(msg, "]")?;
        Ok(msg)
    }

    fn add_contact(&mut self, unit: &SpawnedUnit) {
        let ct = self.contacts.entry(unit.id).or_default();
        ct.pos = unit.position.p.0;
        ct.last_move = unit.moved;
        ct.tags = unit.tags;
        ct.typ = unit.typ.clone();
    }

    fn remove_target(&mut self, _db: &Db, lua: MizLua) -> Result<()> {
        if let Some(target) = self.target.take() {
            target
                .destroy(lua)
                .with_context(|| format_compact!("destroying target for jtac {}", self.gid))?
        }
        Ok(())
    }

    fn mark_target(&mut self, lua: MizLua) -> Result<()> {
        if let Some(target) = &mut self.target {
            let act = Trigger::singleton(lua)?.action()?;
            if let Some(mid) = target.mark.take() {
                act.remove_mark(mid).context("removing mark")?;
            }
            let new_mid = MarkId::new();
            let ct = &self.contacts[&target.uid];
            let msg = format_compact!(
                "JTAC {} target {} marked by code {}",
                self.gid,
                ct.typ,
                self.code
            );
            act.mark_to_coalition(new_mid, msg.into(), LuaVec3(ct.pos), self.side, true, None)
                .context("marking target")?;
            target.mark = Some(new_mid);
        }
        Ok(())
    }

    fn set_target(&mut self, db: &Db, lua: MizLua, i: usize) -> Result<bool> {
        let (uid, ct) = self
            .contacts
            .get_index(i)
            .ok_or_else(|| anyhow!("no such target"))?;
        let uid = *uid;
        let pos = ct.pos;
        let prev_arty = self.nearby_artillery.clone();
        match &self.target {
            Some(target) if target.uid == uid => {
                let arty = db.artillery_near_point(self.side, Vector2::new(pos.x, pos.z));
                if prev_arty != arty {
                    self.menu_dirty = true;
                    self.nearby_artillery = arty;
                }
                Ok(false)
            }
            Some(_) | None => {
                self.remove_target(db, lua)?;
                let jtid = match &self.gid {
                    JtId::Group(gid) => db
                        .first_living_unit(gid)
                        .context("getting jtac beam source")?
                        .clone(),
                    JtId::Slot(sl) => db
                        .ephemeral
                        .get_object_id_by_slot(sl)
                        .ok_or_else(|| anyhow!("no unit for slot {sl}"))?
                        .clone(),
                };
                let jt = match Unit::get_instance(lua, &jtid) {
                    Ok(jt) => jt,
                    Err(_) => {
                        info!("jtac unit died while setting target {:?}", jtid);
                        return Ok(true);
                    }
                };
                let spot = Spot::create_laser(
                    lua,
                    jt.as_object()?,
                    Some(LuaVec3(Vector3::new(0., 10., 0.))),
                    LuaVec3(pos),
                    self.code,
                )
                .context("creating laser spot")?
                .object_id()?;
                let ir_pointer = if self.ir_pointer {
                    Some(
                        Spot::create_infra_red(
                            lua,
                            jt.as_object()?,
                            Some(LuaVec3(Vector3::new(0., 5., 0.))),
                            LuaVec3(pos),
                        )
                        .context("creating ir pointer spot")?
                        .object_id()?,
                    )
                } else {
                    None
                };
                self.target = Some(JtacTarget {
                    spot,
                    source: jtid,
                    ir_pointer,
                    mark: None,
                    uid,
                });
                self.nearby_artillery =
                    db.artillery_near_point(self.side, Vector2::new(pos.x, pos.z));
                self.menu_dirty |= prev_arty == self.nearby_artillery;
                self.mark_target(lua).context("marking target")?;
                Ok(true)
            }
        }
    }

    fn set_code(&mut self, lua: MizLua, code_part: u16) -> Result<()> {
        let hundreds = code_part / 100;
        let tens = code_part / 10;
        if hundreds > 9 || (hundreds > 0 && code_part % 100 > 0) || (tens > 0 && code_part % 10 > 0)
        {
            bail!("invalid code part {code_part}, mixed scales")
        }
        if hundreds > 0 {
            let tens_ones = self.code % 100;
            self.code = 1000 + code_part + tens_ones;
        } else if tens > 0 {
            let hundreds = self.code / 100;
            let ones = self.code % 10;
            self.code = 100 * hundreds + code_part + ones;
        } else {
            let c = self.code / 10;
            self.code = 10 * c + code_part;
        }
        if let Some(target) = &self.target {
            let spot = Spot::get_instance(lua, &target.spot).context("getting laser spot")?;
            spot.set_code(self.code).context("setting laser code")?;
            self.mark_target(lua).context("marking target")?
        }
        Ok(())
    }

    fn shift(&mut self, db: &Db, lua: MizLua) -> Result<bool> {
        if self.contacts.is_empty() {
            return Ok(false);
        }
        let i = match &self.target {
            None => 0,
            Some(target) => match self.contacts.get_index_of(&target.uid) {
                None => 0,
                Some(i) => {
                    if i < self.contacts.len() - 1 {
                        i + 1
                    } else {
                        0
                    }
                }
            },
        };
        self.set_target(db, lua, i).context("setting target")
    }

    fn remove_contact(&mut self, lua: MizLua, db: &Db, uid: &UnitId) -> Result<bool> {
        if let Some(_) = self.contacts.swap_remove(uid) {
            if let Some(target) = &self.target {
                if &target.uid == uid {
                    self.remove_target(db, lua).context("removing target")?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn sort_contacts(&mut self, db: &Db, lua: MizLua) -> Result<bool> {
        let plist = &self.priority;
        let priority = |tags: UnitTags| {
            plist
                .iter()
                .enumerate()
                .find(|(_, p)| tags.contains(p.0))
                .map(|(i, _)| i)
                .unwrap_or(plist.len())
        };
        self.contacts
            .sort_by(|_, ct0, _, ct1| priority(ct0.tags).cmp(&priority(ct1.tags)));
        if self.autoshift && !self.contacts.is_empty() {
            return self.set_target(db, lua, 0).context("setting target");
        }
        Ok(false)
    }

    fn smoke_target(&mut self, lua: MizLua) -> Result<()> {
        if let Some(target) = &self.target {
            if let Some(ct) = self.contacts.get(&target.uid) {
                let now = Utc::now();
                if now - self.last_smoke < Duration::seconds(150) {
                    let rdy = (now - self.last_smoke).num_seconds();
                    bail!("smoke will not be ready for another {}s", rdy)
                }
                self.last_smoke = now;
                let mut rng = thread_rng();
                let act = Trigger::singleton(lua)?.action()?;
                let land = Land::singleton(lua)?;
                let pos = Vector2::new(
                    ct.pos.x + rng.gen_range(0. ..10.),
                    ct.pos.z + rng.gen_range(0. ..10.),
                );
                let pos = Vector3::new(pos.x, land.get_height(LuaVec2(pos))?, pos.y);
                let color = match self.side {
                    Side::Blue => SmokeColor::Red,
                    Side::Red => SmokeColor::Blue,
                    Side::Neutral => SmokeColor::Green,
                };
                act.smoke(LuaVec3(pos), color).context("creating smoke")?;
            }
        }
        Ok(())
    }

    fn reset_target(&mut self, db: &Db, lua: MizLua) -> Result<()> {
        if let Some(target) = &self.target {
            if let Some(i) = self.contacts.get_index_of(&target.uid) {
                self.remove_target(db, lua)?;
                self.set_target(db, lua, i).context("setting jtac target")?;
            }
        }
        Ok(())
    }

    fn artillery_mission(
        &mut self,
        db: &Db,
        lua: MizLua,
        adjustment: ArtilleryAdjustment,
        gid: &GroupId,
        n: u8,
    ) -> Result<()> {
        match self.target.as_mut() {
            None => bail!("no target"),
            Some(target) => {
                let name = db.group(gid)?.name.clone();
                let apos = db.group_center(gid)?;
                let pos = db.unit(&target.uid)?.pos;
                let pos = adjustment.compute_final_solution(apos, pos);
                let task = Task::FireAtPoint {
                    point: LuaVec2(pos),
                    radius: None,
                    expend_qty: Some(n as i64),
                    weapon_type: None,
                    altitude: None,
                    altitude_type: None,
                };
                let group = Group::get_by_name(lua, &name)
                    .with_context(|| format_compact!("getting group {}", name))?;
                let con = group.get_controller().context("getting controller")?;
                con.set_task(task)?;
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct Jtacs {
    jtacs: FxHashMap<Side, FxHashMap<JtId, Jtac>>,
    artillery_adjustment: FxHashMap<GroupId, ArtilleryAdjustment>,
    code_by_location: LocByCode,
    menu_dirty: FxHashMap<Side, bool>,
}

impl Jtacs {
    pub fn get(&self, gid: &JtId) -> Result<&Jtac> {
        self.jtacs
            .iter()
            .find_map(|(_, jtx)| jtx.get(gid))
            .ok_or_else(|| anyhow!("no such jtac {gid}"))
    }

    fn get_mut(&mut self, gid: &JtId) -> Result<&mut Jtac> {
        self.jtacs
            .iter_mut()
            .find_map(|(_, jtx)| jtx.get_mut(gid))
            .ok_or_else(|| anyhow!("no such jtac"))
    }

    pub fn jtacs(&self) -> impl Iterator<Item = &Jtac> {
        self.jtacs.values().flat_map(|jtx| jtx.values())
    }

    pub fn artillery_mission(
        &mut self,
        lua: MizLua,
        db: &Db,
        jtac: &JtId,
        arty: &GroupId,
        n: u8,
    ) -> Result<()> {
        let adjustment = self
            .artillery_adjustment
            .get(arty)
            .map(|a| *a)
            .unwrap_or_default();
        self.get_mut(jtac)?
            .artillery_mission(db, lua, adjustment, arty, n)
    }

    pub fn adjust_artillery_solution(&mut self, arty: &GroupId, dir: AdjustmentDir, mag: u16) {
        let adj = self.artillery_adjustment.entry(*arty).or_default();
        match dir {
            AdjustmentDir::Long => adj.short_long -= mag as i16,
            AdjustmentDir::Short => adj.short_long += mag as i16,
            AdjustmentDir::Left => adj.left_right -= mag as i16,
            AdjustmentDir::Right => adj.left_right += mag as i16,
        }
    }

    pub fn get_artillery_adjustment(&mut self, arty: &GroupId) -> ArtilleryAdjustment {
        self.artillery_adjustment
            .get(arty)
            .map(|a| *a)
            .unwrap_or_default()
    }

    pub fn jtac_status(&self, db: &Db, gid: &JtId) -> Result<CompactString> {
        self.get(gid)?.status(db, &self.code_by_location)
    }

    pub fn toggle_auto_shift(&mut self, db: &Db, lua: MizLua, gid: &JtId) -> Result<()> {
        let jtac = self.get_mut(gid)?;
        jtac.autoshift = !jtac.autoshift;
        if jtac.autoshift {
            jtac.shift(db, lua)?;
        } else {
            jtac.remove_target(db, lua)?
        }
        Ok(())
    }

    pub fn toggle_ir_pointer(&mut self, db: &Db, lua: MizLua, gid: &JtId) -> Result<()> {
        let jtac = self.get_mut(gid)?;
        jtac.ir_pointer = !jtac.ir_pointer;
        jtac.reset_target(db, lua).context("resetting target")?;
        Ok(())
    }

    pub fn smoke_target(&mut self, lua: MizLua, gid: &JtId) -> Result<()> {
        self.get_mut(gid)?.smoke_target(lua)
    }

    pub fn shift(&mut self, db: &Db, lua: MizLua, gid: &JtId) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.autoshift = false;
        jtac.shift(db, lua)
    }

    pub fn clear_filter(&mut self, db: &Db, lua: MizLua, gid: &JtId) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.filter = BitFlags::empty();
        jtac.sort_contacts(db, lua)
    }

    pub fn add_filter(&mut self, db: &Db, lua: MizLua, gid: &JtId, tag: UnitTag) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.filter |= tag;
        jtac.sort_contacts(db, lua)
    }

    /// set part of the laser code, defined by the scale of the passed in number. For example,
    /// passing 600 sets the hundreds part of the code to 6. passing 8 sets the ones part of the code to 8.
    /// other parts of the existing code are left alone.
    pub fn set_code_part(&mut self, lua: MizLua, gid: &JtId, code_part: u16) -> Result<()> {
        let jt = self.get_mut(gid)?;
        let prev_code = jt.code;
        let oid = jt.location.oid;
        jt.set_code(lua, code_part)?;
        let code = jt.code;
        Self::remove_code_by_location(&mut self.code_by_location, oid, prev_code, *gid);
        Self::add_code_by_location(&mut self.code_by_location, oid, code, *gid);
        Ok(())
    }

    pub fn jtac_targets<'a>(&'a self) -> impl Iterator<Item = UnitId> + 'a {
        self.jtacs.values().flat_map(|j| {
            j.values()
                .filter_map(|jt| jt.target.as_ref().map(|target| target.uid))
        })
    }

    pub fn contacts_near_point(&self, side: Side, point: Vector2, dist: f64) -> ContactsIter {
        let dist = dist.powi(2);
        let contacts = self
            .jtacs()
            .filter_map(|jt| {
                if jt.side == side
                    && na::distance_squared(&jt.location.pos.into(), &point.into()) <= dist
                {
                    Some(jt.contacts.iter())
                } else {
                    None
                }
            })
            .collect();
        ContactsIter { i: 0, contacts }
    }

    pub fn add_code_by_location(t: &mut LocByCode, oid: ObjectiveId, code: u16, gid: JtId) {
        t.entry(oid)
            .or_default()
            .entry(code)
            .or_default()
            .insert(gid);
    }

    pub fn remove_code_by_location(t: &mut LocByCode, oid: ObjectiveId, code: u16, gid: JtId) {
        match t.entry(oid).or_default().entry(code) {
            Entry::Vacant(_) => (),
            Entry::Occupied(mut e) => {
                let set = e.get_mut();
                set.remove(&gid);
                if set.is_empty() {
                    e.remove();
                }
            }
        }
    }

    pub fn unit_dead(&mut self, lua: MizLua, db: &mut Db, id: &DcsOid<ClassUnit>) -> Result<()> {
        let uid = db.ephemeral.get_uid_by_object_id(id).map(|uid| *uid);
        let jtid = {
            let sl = db.ephemeral.get_slot_by_object_id(id).map(|sl| *sl);
            match uid {
                Some(uid) => db.unit(&uid).ok().map(|spu| JtId::Group(spu.group)),
                None => sl.map(|sl| JtId::Slot(sl)),
            }
        };
        if let Some(jtid) = jtid {
            for (side, jtx) in self.jtacs.iter_mut() {
                jtx.retain(|gid, jt| {
                    if &jtid == gid {
                        macro_rules! dead {
                            () => {{
                                if let Err(e) = jt.remove_target(db, lua) {
                                    warn!("0 could not remove jtac target {:?}", e)
                                }
                                ui_jtac_dead(db, *side, jtid);
                                Self::remove_code_by_location(
                                    &mut self.code_by_location,
                                    jt.location.oid,
                                    jt.code,
                                    jt.gid,
                                );
                                *self.menu_dirty.entry(jt.side).or_default() = true;
                                return false;
                            }};
                        }
                        match jtid {
                            JtId::Slot(_) => dead!(),
                            JtId::Group(gid) => {
                                if db.group_health(&gid).unwrap_or((0, 0)).0 <= 1 {
                                    dead!()
                                }
                            }
                        }
                        if let Some(target) = &jt.target {
                            if &target.source == id {
                                if let Err(e) = jt.reset_target(db, lua) {
                                    warn!("could not reset jtac target {:?}", e)
                                }
                            }
                        }
                    }
                    let dead = match &jt.target {
                        None => false,
                        Some(target) => match uid {
                            None => false,
                            Some(uid) => target.uid == uid,
                        },
                    };
                    if dead {
                        if let Err(e) = jt.remove_target(db, lua) {
                            warn!("1 could not remove jtac target {:?}", e)
                        }
                        db.ephemeral.msgs().panel_to_side(
                            10,
                            false,
                            jt.side,
                            format_compact!("{} target destroyed", jt.gid),
                        );
                    }
                    true
                })
            }
        }
        Ok(())
    }

    pub fn update_target_positions(
        &mut self,
        lua: MizLua,
        db: &mut Db,
    ) -> Result<Vec<DcsOid<ClassUnit>>> {
        let targets: SmallVec<[UnitId; 16]> = self.jtac_targets().collect();
        let dead = db
            .update_unit_positions(lua, &targets)
            .context("updating the position of jtac targets")?;
        for jtx in self.jtacs.values_mut() {
            for jt in jtx.values_mut() {
                if let Some(target) = &jt.target {
                    let unit = db.unit(&target.uid)?;
                    if (jt.contacts[&target.uid].pos - unit.position.p.0).magnitude() > 1. {
                        let v = db
                            .ephemeral
                            .get_object_id_by_uid(&target.uid)
                            .and_then(|oid| Unit::get_instance(lua, oid).ok())
                            .and_then(|unit| unit.get_velocity().ok())
                            .unwrap_or(LuaVec3(Vector3::default()));
                        let pos = &mut jt.contacts[&target.uid].pos;
                        *pos = unit.position.p.0 + v.0;
                        let spot = Spot::get_instance(lua, &target.spot)
                            .context("getting the spot instance")?;
                        spot.set_point(LuaVec3(*pos))
                            .context("setting the spot position")?;
                        jt.mark_target(lua).context("marking moved target")?
                    }
                }
            }
        }
        Ok(dead)
    }

    pub fn update_contacts(&mut self, lua: MizLua, db: &mut Db) -> Result<SmallVec<[Side; 2]>> {
        let land = Land::singleton(lua)?;
        let mut saw_jtacs: SmallVec<[JtId; 32]> = smallvec![];
        let mut saw_units: FxHashSet<UnitId> = FxHashSet::default();
        let mut lost_targets: SmallVec<[(Side, JtId, Option<UnitId>); 64]> = smallvec![];
        for (mut pos, group, side, ifo) in db.jtacs() {
            if !saw_jtacs.contains(&group) {
                saw_jtacs.push(group)
            }
            let range = (ifo.range as f64).powi(2);
            let jtac = self
                .jtacs
                .entry(side)
                .or_default()
                .entry(group)
                .or_insert_with(|| {
                    *self.menu_dirty.entry(side).or_default() = true;
                    let jt =
                        Jtac::new(db, group, side, db.ephemeral.cfg.jtac_priority.clone(), pos);
                    Self::add_code_by_location(
                        &mut self.code_by_location,
                        jt.location.oid,
                        jt.code,
                        jt.gid,
                    );
                    jt
                });
            let prev_loc = jtac.location;
            jtac.location = JtacLocation::new(db, pos);
            if prev_loc.oid != jtac.location.oid {
                Self::add_code_by_location(
                    &mut self.code_by_location,
                    prev_loc.oid,
                    jtac.code,
                    jtac.gid,
                );
                Self::add_code_by_location(
                    &mut self.code_by_location,
                    jtac.location.oid,
                    jtac.code,
                    jtac.gid,
                );
                *self.menu_dirty.entry(jtac.side).or_default() = true;
            }
            pos.y += 10.;
            for (unit, _) in db.instanced_units() {
                saw_units.insert(unit.id);
                if unit.side == jtac.side {
                    continue;
                }
                if !unit.tags.contains(jtac.filter) {
                    if let Err(e) = jtac.remove_contact(lua, db, &unit.id) {
                        warn!("could not filter jtac contact {} {:?}", unit.name, e)
                    }
                    continue;
                }
                if let Some(ct) = jtac.contacts.get(&unit.id) {
                    if unit.moved == ct.last_move {
                        continue;
                    }
                };
                let dist = na::distance_squared(&pos.into(), &unit.position.p.0.into());
                if dist <= range && land.is_visible(LuaVec3(pos), unit.position.p)? {
                    jtac.add_contact(unit)
                } else {
                    match jtac.remove_contact(lua, db, &unit.id) {
                        Err(e) => warn!("could not remove jtac contact {} {:?}", unit.name, e),
                        Ok(false) => (),
                        Ok(true) => lost_targets.push((jtac.side, jtac.gid, None)),
                    }
                }
            }
        }
        for (side, jtx) in self.jtacs.iter_mut() {
            jtx.retain(|gid, jt| {
                saw_jtacs.contains(gid) || {
                    if let Err(e) = jt.remove_target(db, lua) {
                        warn!("2 could not remove jtac target {:?}", e)
                    }
                    ui_jtac_dead(db, *side, *gid);
                    Self::remove_code_by_location(
                        &mut self.code_by_location,
                        jt.location.oid,
                        jt.code,
                        jt.gid,
                    );
                    *self.menu_dirty.entry(*side).or_default() = true;
                    false
                }
            })
        }
        for (side, jtx) in self.jtacs.iter_mut() {
            for jtac in jtx.values_mut() {
                for uid in jtac.contacts.keys() {
                    if !saw_units.contains(&uid) {
                        lost_targets.push((*side, jtac.gid, Some(*uid)));
                    }
                }
            }
        }
        for (side, gid, uid) in lost_targets {
            match uid {
                Some(uid) => match self.get_mut(&gid)?.remove_contact(lua, db, &uid) {
                    Ok(_) => (),
                    Err(e) => warn!("3 could not remove jtac target {uid} {:?}", e),
                },
                None => {
                    db.ephemeral.msgs().panel_to_side(
                        10,
                        false,
                        side,
                        format_compact!("JTAC {gid} target lost"),
                    );
                }
            }
        }
        let mut new_contacts: SmallVec<[&Jtac; 32]> = smallvec![];
        for j in self.jtacs.values_mut() {
            for (_, jtac) in j.iter_mut() {
                match jtac.sort_contacts(db, lua) {
                    Ok(false) => (),
                    Ok(true) => new_contacts.push(jtac),
                    Err(e) => warn!("could not sort contacts for jtac {}, {:?}", jtac.gid, e),
                }
            }
        }
        let mut msgs: SmallVec<[(Side, CompactString); 32]> = smallvec![];
        for jtac in new_contacts {
            let msg = jtac
                .status(db, &self.code_by_location)
                .with_context(|| format_compact!("generating jtac status for {}", jtac.gid))?;
            msgs.push((jtac.side, msg))
        }
        for (side, msg) in msgs {
            db.ephemeral.msgs().panel_to_side(10, false, side, msg);
        }
        let mut side_menus = smallvec![];
        for (side, dirty) in self.menu_dirty.iter_mut() {
            if *dirty {
                side_menus.push(*side);
                *dirty = false;
            }
        }
        for (side, jtx) in self.jtacs.iter_mut() {
            for jt in jtx.values_mut() {
                if jt.menu_dirty {
                    if !side_menus.contains(side) {
                        side_menus.push(*side)
                    }
                    jt.menu_dirty = false;
                }
            }
        }
        Ok(side_menus)
    }
}
