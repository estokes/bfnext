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
        ephemeral::Ephemeral,
        group::{GroupId, SpawnedGroup, SpawnedUnit, UnitId},
        Db,
    },
    menu,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::{prelude::*, Duration};
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    controller::Task,
    group::Group,
    land::Land,
    object::{DcsObject, DcsOid},
    radians_to_degrees,
    spot::{ClassSpot, Spot},
    trigger::{MarkId, SmokeColor, Trigger},
    unit::{ClassUnit, Unit},
    LuaVec2, LuaVec3, MizLua, String, Vector2, Vector3,
};
use enumflags2::BitFlags;
use fxhash::{FxHashMap, FxHashSet};
use indexmap::IndexMap;
use log::{error, info, warn};
use rand::{thread_rng, Rng};
use smallvec::{smallvec, SmallVec};

fn ui_jtac_dead(db: &mut Ephemeral, lua: MizLua, side: Side, gid: GroupId) {
    db.msgs().panel_to_side(
        10,
        false,
        side,
        format_compact!("JTAC {gid} is no longer available"),
    );
    if let Err(e) = menu::remove_menu_for_jtac(lua, side, gid) {
        warn!("could not remove menu for jtac {gid} {e}")
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AdjustmentDir {
    Short,
    Long,
    Left,
    Right
}

#[derive(Debug, Clone, Copy)]
struct ArtilleryAdjustment {
    short_long: i16,
    left_right: i16
}

#[derive(Debug, Clone, Default)]
struct Contact {
    pos: Vector3,
    typ: String,
    tags: UnitTags,
    last_move: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct JtacTarget {
    uid: UnitId,
    source: DcsOid<ClassUnit>,
    spot: DcsOid<ClassSpot>,
    ir_pointer: Option<DcsOid<ClassSpot>>,
    mark: Option<MarkId>,
    artillery_mission: FxHashSet<GroupId>,
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

#[derive(Debug, Clone)]
struct Jtac {
    gid: GroupId,
    side: Side,
    contacts: IndexMap<UnitId, Contact>,
    artillery_adjustment: FxHashMap<GroupId, ArtilleryAdjustment>,
    filter: BitFlags<UnitTag>,
    priority: Vec<UnitTags>,
    target: Option<JtacTarget>,
    autolase: bool,
    ir_pointer: bool,
    code: u16,
    last_smoke: DateTime<Utc>,
}

impl Jtac {
    fn new(gid: GroupId, side: Side, priority: Vec<UnitTags>) -> Self {
        Self {
            gid,
            side,
            contacts: IndexMap::default(),
            artillery_adjustment: FxHashMap::default(),
            filter: BitFlags::default(),
            priority,
            target: None,
            autolase: true,
            ir_pointer: false,
            code: 1688,
            last_smoke: DateTime::<Utc>::default(),
        }
    }

    fn status(&self, db: &Db) -> Result<CompactString> {
        use std::fmt::Write;
        let jtac_pos = db.group_center(&self.gid)?;
        let (dist, heading, obj) = db.objective_near_point(jtac_pos);
        let dist = dist / 1000.;
        let mut msg = CompactString::new("");
        write!(msg, "JTAC {} status\n", self.gid)?;
        match &self.target {
            None => write!(msg, "no target\n")?,
            Some(target) => {
                let unit_typ = db.unit(&target.uid)?.typ.clone();
                let mid = match target.mark {
                    None => format_compact!("none"),
                    Some(mid) => format_compact!("{mid}"),
                };
                write!(msg, "lasing {unit_typ} code {} marker {mid}\n", self.code)?
            }
        }
        write!(
            msg,
            "position bearing {} for {:.1}km from {}\n\n",
            radians_to_degrees(heading) as u32,
            dist,
            obj.name()
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
            "\n\nautolase: {}, ir_pointer: {}",
            self.autolase, self.ir_pointer
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

    fn remove_target(&mut self, db: &Db, lua: MizLua) -> Result<()> {
        if let Err(e) = self.cancel_artillery_missions(db, lua) {
            warn!(
                "could not cancel artillery mission for jtac {} {:?}",
                self.gid, e
            )
        }
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
        match &self.target {
            Some(target) if target.uid == uid => Ok(false),
            Some(_) | None => {
                self.remove_target(db, lua)?;
                let jtid = db
                    .first_living_unit(&self.gid)
                    .context("getting jtac beam source")?
                    .clone();
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
                    Some(LuaVec3(Vector3::new(0., 5., 0.))),
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
                    artillery_mission: FxHashSet::default(),
                });
                let arty = self.artillery_near_target(db).unwrap_or_default();
                menu::add_artillery_menu_for_jtac(lua, self.side, self.gid, &arty)
                    .context("adding arty menu")?;
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
        if self.autolase && !self.contacts.is_empty() {
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

    fn cancel_artillery_mission(&mut self, db: &Db, lua: MizLua, gid: &GroupId) -> Result<()> {
        if let Some(target) = self.target.as_mut() {
            if target.artillery_mission.remove(gid) {
                // the jtac might BE the artillery and it might be gone
                if let Ok(group) = db.group(&gid) {
                    let con = Group::get_by_name(lua, &group.name)
                        .with_context(|| format_compact!("getting group {}", group.name))?
                        .get_controller()
                        .context("getting controller")?;
                    con.reset_task().context("resetting task")?
                }
            }
        }
        Ok(())
    }

    fn cancel_artillery_missions(&mut self, db: &Db, lua: MizLua) -> Result<()> {
        let cancel = |gid: GroupId| -> Result<()> {
            // the jtac might BE the artillery and it might be gone
            if let Ok(group) = db.group(&gid) {
                let con = Group::get_by_name(lua, &group.name)
                    .with_context(|| format_compact!("getting group {}", group.name))?
                    .get_controller()
                    .context("getting controller")?;
                con.reset_task().context("resetting task")?
            }
            Ok(())
        };
        if let Some(target) = self.target.as_mut() {
            for gid in target.artillery_mission.drain() {
                if let Err(e) = cancel(gid) {
                    warn!("could not cancel artillery mission for {gid} {:?}", e)
                }
            }
        }
        Ok(())
    }

    fn artillery_near_target<'a>(&self, db: &'a Db) -> Option<SmallVec<[GroupId; 8]>> {
        self.target.as_ref().and_then(|target| {
            let range2 = (db.ephemeral.cfg.artillery_mission_range as f64).powi(2);
            let pos = db.unit(&target.uid).ok()?.pos;
            let artillery = db
                .deployed()
                .filter_map(|group| {
                    if group.tags.contains(UnitTag::Artillery) {
                        let center = db.group_center(&group.id).ok()?;
                        if na::distance_squared(&center.into(), &pos.into()) <= range2 {
                            Some(group.id)
                        } else {
                            None
                        }
                    } else {
                        None
                    }
                })
                .collect::<SmallVec<[GroupId; 8]>>();
            if artillery.is_empty() {
                None
            } else {
                Some(artillery)
            }
        })
    }

    fn artillery_mission(&mut self, db: &Db, lua: MizLua, gid: &GroupId, n: u8) -> Result<()> {
        self.cancel_artillery_mission(db, lua, gid)
            .context("canceling artillery mission")?;
        match self.target.as_mut() {
            None => bail!("no target"),
            Some(target) => {
                let group = db.group(gid)?;
                let adjustment = self
                    .artillery_adjustment
                    .get(gid)
                    .unwrap_or(&Vector2::new(0., 0.));
                let pos = db.unit(&target.uid)?.pos + adjustment;
                let con = Group::get_by_name(lua, &group.name)
                    .with_context(|| format_compact!("getting group {}", group.name))?
                    .get_controller()
                    .context("getting controller")?;
                con.set_task(Task::FireAtPoint {
                    point: LuaVec2(pos),
                    radius: Some(10.),
                    expend_qty: Some(n as i64),
                    weapon_type: None,
                    altitude: None,
                    altitude_type: None,
                })
                .context("setting task")?;
                target.artillery_mission.insert(*gid);
            }
        }
        Ok(())
    }

    fn adjust_artillery_solution(&mut self, gid: &GroupId, dir: ArtilleryAdjustment, mag: u16) {
        *self
            .artillery_adjustment
            .entry(*gid)
            .or_insert(Vector2::new(0., 0.)) += v;
    }
}

#[derive(Debug, Clone, Default)]
pub struct Jtacs(FxHashMap<Side, FxHashMap<GroupId, Jtac>>);

impl Jtacs {
    fn get(&self, gid: &GroupId) -> Result<&Jtac> {
        self.0
            .iter()
            .find_map(|(_, jtx)| jtx.get(gid))
            .ok_or_else(|| anyhow!("no such jtac {gid}"))
    }

    fn get_mut(&mut self, gid: &GroupId) -> Result<&mut Jtac> {
        self.0
            .iter_mut()
            .find_map(|(_, jtx)| jtx.get_mut(gid))
            .ok_or_else(|| anyhow!("no such jtac"))
    }

    pub fn artillery_mission(
        &mut self,
        lua: MizLua,
        db: &Db,
        jtac: &GroupId,
        arty: &GroupId,
        n: u8,
    ) -> Result<()> {
        self.get_mut(jtac)?.artillery_mission(db, lua, arty, n)
    }

    pub fn cancel_artillery_mission(
        &mut self,
        lua: MizLua,
        db: &Db,
        jtac: &GroupId,
        arty: &GroupId,
    ) -> Result<()> {
        self.get_mut(jtac)?.cancel_artillery_mission(db, lua, arty)
    }

    pub fn jtac_status(&self, db: &Db, gid: &GroupId) -> Result<CompactString> {
        self.get(gid)?.status(db)
    }

    pub fn toggle_auto_laser(&mut self, db: &Db, lua: MizLua, gid: &GroupId) -> Result<()> {
        let jtac = self.get_mut(gid)?;
        jtac.autolase = !jtac.autolase;
        if jtac.autolase {
            jtac.shift(db, lua)?;
        } else {
            jtac.remove_target(db, lua)?
        }
        Ok(())
    }

    pub fn toggle_ir_pointer(&mut self, db: &Db, lua: MizLua, gid: &GroupId) -> Result<()> {
        let jtac = self.get_mut(gid)?;
        jtac.ir_pointer = !jtac.ir_pointer;
        jtac.reset_target(db, lua).context("resetting target")?;
        Ok(())
    }

    pub fn smoke_target(&mut self, lua: MizLua, gid: &GroupId) -> Result<()> {
        self.get_mut(gid)?.smoke_target(lua)
    }

    pub fn shift(&mut self, db: &Db, lua: MizLua, gid: &GroupId) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.autolase = false;
        jtac.shift(db, lua)
    }

    pub fn clear_filter(&mut self, db: &Db, lua: MizLua, gid: &GroupId) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.filter = BitFlags::empty();
        jtac.sort_contacts(db, lua)
    }

    pub fn add_filter(
        &mut self,
        db: &Db,
        lua: MizLua,
        gid: &GroupId,
        tag: UnitTag,
    ) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.filter |= tag;
        jtac.sort_contacts(db, lua)
    }

    /// set part of the laser code, defined by the scale of the passed in number. For example,
    /// passing 600 sets the hundreds part of the code to 6. passing 8 sets the ones part of the code to 8.
    /// other parts of the existing code are left alone.
    pub fn set_code_part(&mut self, lua: MizLua, gid: &GroupId, code_part: u16) -> Result<()> {
        self.get_mut(gid)?.set_code(lua, code_part)
    }

    pub fn jtac_targets<'a>(&'a self) -> impl Iterator<Item = UnitId> + 'a {
        self.0.values().flat_map(|j| {
            j.values()
                .filter_map(|jt| jt.target.as_ref().map(|target| target.uid))
        })
    }

    pub fn unit_dead(&mut self, lua: MizLua, db: &mut Db, id: &DcsOid<ClassUnit>) -> Result<()> {
        if let Some(uid) = db.ephemeral.get_uid_by_object_id(id) {
            let uid = *uid;
            for (side, jtx) in self.0.iter_mut() {
                jtx.retain(|gid, jt| {
                    if let Ok(spu) = db.unit(&uid) {
                        let group = spu.group;
                        if &group == gid {
                            let alive = db.group_health(gid).unwrap_or((0, 0)).0;
                            if alive <= 1 {
                                if let Err(e) = jt.remove_target(db, lua) {
                                    warn!("0 could not remove jtac target {:?}", e)
                                }
                                ui_jtac_dead(&mut db.ephemeral, lua, *side, *gid);
                                return false;
                            } else if let Some(target) = &jt.target {
                                if &target.source == id {
                                    if let Err(e) = jt.reset_target(db, lua) {
                                        warn!("could not reset jtac target {:?}", e)
                                    }
                                }
                            }
                        }
                    }
                    let dead = match &jt.target {
                        None => false,
                        Some(target) => target.uid == uid,
                    };
                    if dead {
                        if let Err(e) = jt.remove_target(db, lua) {
                            warn!("1 could not remove jtac target {:?}", e)
                        }
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
        for jtx in self.0.values_mut() {
            for jt in jtx.values_mut() {
                if let Some(target) = &jt.target {
                    let unit = db.unit(&target.uid)?;
                    if jt.contacts[&target.uid].pos != unit.position.p.0 {
                        let v = db
                            .ephemeral
                            .get_object_id_by_uid(&target.uid)
                            .and_then(|oid| Unit::get_instance(lua, oid).ok())
                            .and_then(|unit| unit.get_velocity().ok())
                            .unwrap_or(LuaVec3(Vector3::default()));
                        jt.contacts[&target.uid].pos = unit.position.p.0 + v.0;
                        let spot = Spot::get_instance(lua, &target.spot)
                            .context("getting the spot instance")?;
                        spot.set_point(unit.position.p)
                            .context("setting the spot position")?;
                        jt.mark_target(lua).context("marking moved target")?
                    }
                }
            }
        }
        Ok(dead)
    }

    pub fn update_contacts(&mut self, lua: MizLua, db: &mut Db) -> Result<()> {
        let land = Land::singleton(lua)?;
        let mut saw_jtacs: SmallVec<[GroupId; 32]> = smallvec![];
        let mut saw_units: FxHashSet<UnitId> = FxHashSet::default();
        let mut lost_targets: SmallVec<[(Side, GroupId, Option<UnitId>); 64]> = smallvec![];
        for (mut pos, group, ifo) in db.jtacs() {
            if !saw_jtacs.contains(&group.id) {
                saw_jtacs.push(group.id)
            }
            let range = (ifo.range as f64).powi(2);
            pos.y += 10.;
            let jtac = self
                .0
                .entry(group.side)
                .or_default()
                .entry(group.id)
                .or_insert_with(|| {
                    if let Err(e) = menu::add_menu_for_jtac(lua, group.side, group.id) {
                        error!("could not add menu for jtac {} {e}", group.id)
                    }
                    Jtac::new(group.id, group.side, db.ephemeral.cfg.jtac_priority.clone())
                });
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
        for (side, jtx) in self.0.iter_mut() {
            jtx.retain(|gid, jt| {
                saw_jtacs.contains(gid) || {
                    if let Err(e) = jt.remove_target(db, lua) {
                        warn!("2 could not remove jtac target {:?}", e)
                    }
                    ui_jtac_dead(&mut db.ephemeral, lua, *side, *gid);
                    false
                }
            })
        }
        for (side, jtx) in self.0.iter_mut() {
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
                Some(uid) => {
                    if let Err(e) = self.get_mut(&gid)?.remove_contact(lua, db, &uid) {
                        warn!("3 could not remove jtac target {uid} {:?}", e)
                    }
                }
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
        for j in self.0.values_mut() {
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
                .status(db)
                .with_context(|| format_compact!("generating jtac status for {}", jtac.gid))?;
            msgs.push((jtac.side, msg))
        }
        for (side, msg) in msgs {
            db.ephemeral.msgs().panel_to_side(10, false, side, msg);
        }
        Ok(())
    }
}
