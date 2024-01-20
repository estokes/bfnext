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

use super::{
    cargo::Cargo,
    group::{GroupId, SpawnedGroup, SpawnedUnit, UnitId},
    objective::{Objective, ObjectiveId, ObjectiveKind},
    persisted::Persisted,
};
use crate::{
    cfg::{Cfg, Crate, Deployable, DeployableLogistics, Troop, Vehicle, WarehouseConfig},
    maybe,
    msgq::MsgQ,
    spawnctx::{Despawn, SpawnCtx},
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{
    airbase::ClassAirbase,
    centroid2d,
    coalition::Side,
    env::miz::{GroupKind, Miz, MizIndex},
    net::{SlotId, Ucid},
    object::{DcsObject, DcsOid},
    trigger::{ArrowSpec, CircleSpec, LineType, MarkId, RectSpec, SideFilter, TextSpec},
    unit::{ClassUnit, Unit},
    Color, LuaVec3, MizLua, Position3, String, Vector2, Vector3,
};
use fxhash::{FxHashMap, FxHashSet};
use log::info;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::max,
    collections::{hash_map::Entry, BTreeMap, VecDeque},
    mem,
    sync::Arc,
};

#[derive(Debug, Clone, Default)]
pub(super) struct ObjectiveMarkup {
    side: Side,
    threatened: bool,
    health: u8,
    logi: u8,
    supply: u8,
    fuel: u8,
    owner_ring: MarkId,
    threatened_ring: MarkId,
    name: MarkId,
    health_label: MarkId,
    healthbar: [MarkId; 5],
    logi_label: MarkId,
    logibar: [MarkId; 5],
    supply_label: MarkId,
    supplybar: [MarkId; 5],
    supply_connections: SmallVec<[MarkId; 8]>,
    fuel_label: MarkId,
    fuelbar: [MarkId; 5],
}

fn text_color(side: Side, a: f32) -> Color {
    match side {
        Side::Red => Color::red(a),
        Side::Blue => Color::blue(a),
        Side::Neutral => Color::white(a),
    }
}

impl ObjectiveMarkup {
    fn remove(self, msgq: &mut MsgQ) {
        let ObjectiveMarkup {
            side: _,
            threatened: _,
            health: _,
            logi: _,
            supply: _,
            fuel: _,
            owner_ring,
            threatened_ring,
            name,
            health_label,
            healthbar,
            logi_label,
            logibar,
            supply_label,
            supplybar,
            supply_connections,
            fuel_label,
            fuelbar,
        } = self;
        msgq.delete_mark(owner_ring);
        msgq.delete_mark(threatened_ring);
        msgq.delete_mark(name);
        msgq.delete_mark(health_label);
        for id in healthbar {
            msgq.delete_mark(id)
        }
        msgq.delete_mark(logi_label);
        for id in logibar {
            msgq.delete_mark(id)
        }
        msgq.delete_mark(supply_label);
        for id in supplybar {
            msgq.delete_mark(id)
        }
        msgq.delete_mark(fuel_label);
        for id in fuelbar {
            msgq.delete_mark(id)
        }
        for id in supply_connections {
            msgq.delete_mark(id)
        }
    }

    fn update(&mut self, msgq: &mut MsgQ, obj: &Objective) {
        if obj.owner != self.side {
            let text_color = |a| text_color(obj.owner, a);
            self.side = obj.owner;
            msgq.set_markup_color(self.name, text_color(0.75));
            msgq.set_markup_color(self.owner_ring, text_color(1.));
            msgq.set_markup_color(self.health_label, text_color(0.75));
            msgq.set_markup_color(self.logi_label, text_color(0.75));
            msgq.set_markup_color(self.supply_label, text_color(0.75));
            msgq.set_markup_color(self.fuel_label, text_color(0.75));
            for id in self.supply_connections.drain(..) {
                msgq.delete_mark(id);
            }
        }
        if obj.threatened != self.threatened {
            self.threatened = obj.threatened;
            msgq.set_markup_color(
                self.threatened_ring,
                Color::yellow(if self.threatened { 0.75 } else { 0. }),
            );
        }
        if self.health != obj.health {
            self.health = obj.health;
            for (i, id) in self.healthbar.iter().enumerate() {
                let i = (i + 1) as u8;
                let a = if (obj.health / (i * 20)) > 0 { 0.5 } else { 0. };
                msgq.set_markup_fill_color(*id, Color::green(a));
            }
        }
        if self.logi != obj.logi {
            self.logi = obj.logi;
            for (i, id) in self.logibar.iter().enumerate() {
                let i = (i + 1) as u8;
                let a = if (obj.logi / (i * 20)) > 0 { 0.5 } else { 0. };
                msgq.set_markup_fill_color(*id, Color::green(a));
            }
        }
        if self.supply != obj.supply {
            self.supply = obj.supply;
            for (i, id) in self.supplybar.iter().enumerate() {
                let i = (i + 1) as u8;
                let a = if (obj.supply / (i * 20)) > 0 { 0.5 } else { 0. };
                msgq.set_markup_fill_color(*id, Color::green(a));
            }
        }
        if self.fuel != obj.fuel {
            self.fuel = obj.fuel;
            for (i, id) in self.fuelbar.iter().enumerate() {
                let i = (i + 1) as u8;
                let a = if (obj.fuel / (i * 20)) > 0 { 0.5 } else { 0. };
                msgq.set_markup_fill_color(*id, Color::green(a));
            }
        }
    }

    fn new(cfg: &Cfg, msgq: &mut MsgQ, obj: &Objective, persisted: &Persisted) -> Self {
        let text_color = |a| text_color(obj.owner, a);
        let all_spec = match obj.kind {
            ObjectiveKind::Airbase | ObjectiveKind::Fob | ObjectiveKind::Logistics => {
                SideFilter::All
            }
            ObjectiveKind::Farp { .. } => obj.owner.into(),
        };
        let bar_with_label = |msgq: &mut MsgQ,
                              pos3: Vector3,
                              label: MarkId,
                              text: &str,
                              marks: &[MarkId; 5],
                              val: u8| {
            msgq.text_to_all(
                obj.owner.into(),
                label,
                TextSpec {
                    pos: LuaVec3(Vector3::new(pos3.x + 200., 0., pos3.z)),
                    color: text_color(0.75),
                    fill_color: Color::black(0.),
                    font_size: 12,
                    read_only: true,
                    text: text.into(),
                },
            );
            for (i, id) in marks.iter().enumerate() {
                let j = (i + 1) as u8;
                let i = i as f64;
                let a = if (val / (j * 20)) > 0 { 0.5 } else { 0. };
                msgq.rect_to_all(
                    obj.owner.into(),
                    *id,
                    RectSpec {
                        start: LuaVec3(Vector3::new(pos3.x, 0., pos3.z + i * 500.)),
                        end: LuaVec3(Vector3::new(pos3.x - 400., 0., pos3.z + i * 500. + 400.)),
                        color: Color::black(1.),
                        fill_color: Color::green(a),
                        line_type: LineType::Solid,
                        read_only: true,
                    },
                    None,
                );
            }
        };
        let mut t = ObjectiveMarkup::default();
        t.side = obj.owner;
        t.threatened = obj.threatened;
        t.health = obj.health;
        t.logi = obj.logi;
        t.supply = obj.supply;
        let mut pos3 = Vector3::new(obj.pos.x, 0., obj.pos.y);
        msgq.circle_to_all(
            all_spec,
            t.owner_ring,
            CircleSpec {
                center: LuaVec3(pos3),
                radius: obj.radius,
                color: text_color(1.),
                fill_color: Color::white(0.),
                line_type: LineType::Dashed,
                read_only: true,
            },
            None,
        );
        msgq.circle_to_all(
            obj.owner.into(),
            t.threatened_ring,
            CircleSpec {
                center: LuaVec3(pos3),
                radius: cfg.logistics_exclusion as f64,
                color: Color::yellow(if obj.threatened { 0.75 } else { 0. }),
                fill_color: Color::white(0.),
                line_type: LineType::Solid,
                read_only: true,
            },
            None,
        );
        msgq.text_to_all(
            all_spec,
            t.name,
            TextSpec {
                pos: LuaVec3(Vector3::new(pos3.x + 1500., 1., pos3.z + 1500.)),
                color: text_color(1.),
                fill_color: Color::black(0.),
                font_size: 14,
                read_only: true,
                text: obj.name.clone(),
            },
        );
        pos3.x += 5000.;
        pos3.z -= 5000.;
        bar_with_label(
            msgq,
            pos3,
            t.health_label,
            "Health",
            &t.healthbar,
            obj.health,
        );
        pos3.x -= 1500.;
        bar_with_label(msgq, pos3, t.logi_label, "Logi", &t.logibar, obj.logi);
        pos3.x -= 1500.;
        bar_with_label(
            msgq,
            pos3,
            t.supply_label,
            "Supply",
            &t.supplybar,
            obj.supply,
        );
        pos3.x -= 1500.;
        bar_with_label(msgq, pos3, t.fuel_label, "Fuel", &t.fuelbar, obj.fuel);
        match obj.kind {
            ObjectiveKind::Airbase | ObjectiveKind::Farp { .. } | ObjectiveKind::Fob => (),
            ObjectiveKind::Logistics => {
                let pos = obj.pos;
                for oid in &obj.warehouse.destination {
                    let id = MarkId::new();
                    let dobj = &persisted.objectives[oid];
                    let dir = (dobj.pos - pos).normalize();
                    let spos = pos + dir * obj.radius * 1.1;
                    let rdir = (pos - dobj.pos).normalize();
                    let dpos = dobj.pos + rdir * dobj.radius * 1.1;
                    msgq.arrow_to_all(
                        obj.owner.into(),
                        id,
                        ArrowSpec {
                            start: LuaVec3(Vector3::new(dpos.x, 0., dpos.y)),
                            end: LuaVec3(Vector3::new(spos.x, 0., spos.y)),
                            color: Color::gray(0.5),
                            fill_color: Color::gray(0.5),
                            line_type: LineType::NoLine,
                            read_only: true,
                        },
                        None,
                    );
                    t.supply_connections.push(id);
                }
            }
        }
        t
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct DeployableIndex {
    pub(super) deployables_by_name: FxHashMap<String, Deployable>,
    pub(super) deployables_by_crates: FxHashMap<String, String>,
    pub(super) deployables_by_repair: FxHashMap<String, String>,
    pub(super) crates_by_name: FxHashMap<String, Crate>,
    pub(super) squads_by_name: FxHashMap<String, Troop>,
    pub(super) pad_templates: FxHashSet<String>,
}

#[derive(Debug, Default)]
pub struct Ephemeral {
    dirty: bool,
    pub(super) cfg: Cfg,
    pub(super) players_by_slot: FxHashMap<SlotId, Ucid>,
    pub(super) cargo: FxHashMap<SlotId, Cargo>,
    pub(super) deployable_idx: FxHashMap<Side, Arc<DeployableIndex>>,
    pub(super) group_marks: FxHashMap<GroupId, MarkId>,
    objective_markup: FxHashMap<ObjectiveId, ObjectiveMarkup>,
    pub(super) object_id_by_uid: FxHashMap<UnitId, DcsOid<ClassUnit>>,
    pub(super) uid_by_object_id: FxHashMap<DcsOid<ClassUnit>, UnitId>,
    pub(super) object_id_by_slot: FxHashMap<SlotId, DcsOid<ClassUnit>>,
    pub(super) slot_by_object_id: FxHashMap<DcsOid<ClassUnit>, SlotId>,
    pub(super) airbase_by_oid: FxHashMap<ObjectiveId, DcsOid<ClassAirbase>>,
    used_pad_templates: FxHashSet<String>,
    force_to_spectators: FxHashSet<Ucid>,
    pub(super) units_able_to_move: FxHashSet<UnitId>,
    pub(super) units_potentially_close_to_enemies: FxHashSet<UnitId>,
    pub(super) units_potentially_on_walkabout: FxHashSet<UnitId>,
    pub(super) delayspawnq: BTreeMap<DateTime<Utc>, SmallVec<[GroupId; 8]>>,
    spawnq: VecDeque<GroupId>,
    despawnq: VecDeque<(GroupId, Despawn)>,
    sync_warehouse: Vec<(ObjectiveId, Vehicle)>,
    pub(super) msgs: MsgQ,
}

impl Ephemeral {
    pub fn create_objective_markup(&mut self, obj: &Objective, persisted: &Persisted) {
        if let Some(mk) = self.objective_markup.remove(&obj.id) {
            mk.remove(&mut self.msgs)
        }
        self.objective_markup.insert(
            obj.id,
            ObjectiveMarkup::new(&self.cfg, &mut self.msgs, obj, persisted),
        );
    }

    pub fn update_objective_markup(&mut self, obj: &Objective) {
        if let Some(mk) = self.objective_markup.get_mut(&obj.id) {
            mk.update(&mut self.msgs, obj)
        }
    }

    pub fn remove_objective_markup(&mut self, oid: &ObjectiveId) {
        if let Some(mk) = self.objective_markup.remove(oid) {
            mk.remove(&mut self.msgs)
        }
    }

    pub fn push_sync_warehouse(&mut self, oid: ObjectiveId, vehicle: Vehicle) {
        self.sync_warehouse.push((oid, vehicle));
    }

    pub fn warehouses_to_sync(&mut self) -> Vec<(ObjectiveId, Vehicle)> {
        mem::take(&mut self.sync_warehouse)
    }

    pub fn push_despawn(&mut self, gid: GroupId, ds: Despawn) {
        let mut queued_spawn = false;
        self.spawnq.retain(|sp_gid| {
            let qs = &gid == sp_gid;
            queued_spawn |= qs;
            !qs
        });
        let e = (gid, ds);
        if !queued_spawn && !self.despawnq.contains(&e) {
            self.despawnq.push_back(e)
        }
    }

    pub fn push_spawn(&mut self, gid: GroupId) {
        let mut queued_despawn = false;
        self.despawnq.retain(|(ds_gid, _)| {
            let qs = &gid == ds_gid;
            queued_despawn |= qs;
            !qs
        });
        if !queued_despawn && !self.spawnq.contains(&gid) {
            self.spawnq.push_back(gid)
        }
    }

    pub fn process_spawn_queue(
        &mut self,
        persisted: &Persisted,
        now: DateTime<Utc>,
        idx: &MizIndex,
        spctx: &SpawnCtx,
    ) -> Result<()> {
        let mut delayed: SmallVec<[GroupId; 16]> = smallvec![];
        while let Some((at, gids)) = self.delayspawnq.first_key_value() {
            if now < *at {
                break;
            } else {
                for gid in gids {
                    delayed.push(*gid);
                }
                let at = *at;
                self.delayspawnq.remove(&at);
            }
        }
        for gid in delayed {
            self.push_spawn(gid)
        }
        let dlen = self.despawnq.len();
        let slen = self.spawnq.len();
        if dlen > 0 {
            for _ in 0..max(2, dlen >> 2) {
                if let Some((gid, name)) = self.despawnq.pop_front() {
                    if let Some(group) = persisted.groups.get(&gid) {
                        for uid in &group.units {
                            self.units_able_to_move.remove(uid);
                            self.units_potentially_close_to_enemies.remove(uid);
                            self.units_potentially_on_walkabout.remove(uid);
                            if let Some(id) = self.object_id_by_uid.remove(uid) {
                                self.uid_by_object_id.remove(&id);
                            }
                        }
                    }
                    spctx.despawn(name)?
                }
            }
        } else if slen > 0 {
            for _ in 0..max(2, slen >> 2) {
                if let Some(gid) = self.spawnq.pop_front() {
                    let group = maybe!(persisted.groups, gid, "group")?;
                    spawn_group(persisted, idx, spctx, group)?
                }
            }
        }
        Ok(())
    }

    pub fn take_pad_template(&mut self, side: Side) -> Option<String> {
        self.deployable_idx.get(&side).and_then(|idx| {
            for pad in &idx.pad_templates {
                if self.used_pad_templates.insert(pad.clone()) {
                    return Some(pad.clone());
                }
            }
            None
        })
    }

    pub fn return_pad_template(&mut self, pad: &str) {
        self.used_pad_templates.remove(pad);
    }

    pub fn msgs(&mut self) -> &mut MsgQ {
        &mut self.msgs
    }

    pub fn cfg(&self) -> &Cfg {
        &self.cfg
    }

    pub fn get_uid_by_object_id(&self, id: &DcsOid<ClassUnit>) -> Option<&UnitId> {
        self.uid_by_object_id.get(id)
    }

    pub fn get_object_id_by_uid(&self, id: &UnitId) -> Option<&DcsOid<ClassUnit>> {
        self.object_id_by_uid.get(id)
    }

    fn index_deployables_for_side(
        &mut self,
        global_pad_templates: &mut FxHashSet<String>,
        miz: &Miz,
        mizidx: &MizIndex,
        side: Side,
        repair_crate: Crate,
        whcfg: &Option<WarehouseConfig>,
        deployables: &[Deployable],
    ) -> Result<()> {
        let idx = Arc::make_mut(self.deployable_idx.entry(side).or_default());
        idx.crates_by_name
            .insert(repair_crate.name.clone(), repair_crate);
        if let Some(whcfg) = whcfg.as_ref() {
            match whcfg.supply_transfer_crate.get(&side) {
                None => bail!("missing supply transfer crate for {side}"),
                Some(cr) => match idx.crates_by_name.entry(cr.name.clone()) {
                    Entry::Occupied(_) => bail!("multiple {} crates for side {side}", cr.name),
                    Entry::Vacant(e) => {
                        e.insert(cr.clone());
                    }
                },
            };
        }
        for dep in deployables.iter() {
            miz.get_group_by_name(mizidx, GroupKind::Any, side, &dep.template)?
                .ok_or_else(|| anyhow!("missing deployable template {:?} {:?}", side, dep))?;
            let name = match dep.path.last() {
                None => bail!("deployable with empty path {:?}", dep),
                Some(name) => name,
            };
            match idx.deployables_by_name.entry(name.clone()) {
                Entry::Occupied(_) => bail!("deployable with duplicate name {name}"),
                Entry::Vacant(e) => e.insert(dep.clone()),
            };
            if let Some(rep) = dep.repair_crate.as_ref() {
                match idx.deployables_by_repair.entry(rep.name.clone()) {
                    Entry::Occupied(_) => {
                        bail!(
                            "multiple deployables use the same repair crate {}",
                            rep.name
                        )
                    }
                    Entry::Vacant(e) => {
                        if idx.deployables_by_crates.contains_key(&rep.name) {
                            bail!(
                                "deployable {} uses repair crate of {}",
                                &idx.deployables_by_crates[&rep.name],
                                name
                            )
                        }
                        e.insert(name.clone())
                    }
                };
            }
            for cr in dep.crates.iter() {
                match idx.deployables_by_crates.entry(cr.name.clone()) {
                    Entry::Occupied(_) => bail!("multiple deployables use crate {}", cr.name),
                    Entry::Vacant(e) => {
                        if idx.deployables_by_repair.contains_key(&cr.name) {
                            bail!(
                                "deployable repair {} uses crate of {}",
                                &idx.deployables_by_repair[&cr.name],
                                name
                            )
                        }
                        e.insert(name.clone())
                    }
                };
            }
            for c in dep.crates.iter().chain(dep.repair_crate.iter()) {
                match idx.crates_by_name.entry(c.name.clone()) {
                    Entry::Occupied(_) => bail!("duplicate crate name {}", c.name),
                    Entry::Vacant(e) => e.insert(c.clone()),
                };
            }
            if let Some(DeployableLogistics {
                pad_templates,
                ammo_template,
                fuel_template,
                barracks_template,
            }) = &dep.logistics
            {
                let mut names = FxHashSet::default();
                for name in [
                    &dep.template,
                    ammo_template,
                    fuel_template,
                    barracks_template,
                ]
                .into_iter()
                .chain(pad_templates.iter())
                {
                    miz.get_group_by_name(mizidx, GroupKind::Any, side, name)?
                        .ok_or_else(|| anyhow!("missing farp template {:?} {:?}", side, name))?;
                    if !names.insert(name) {
                        bail!("deployables with logistics must use unique templates for each part {name} is reused")
                    }
                }
                for pad in pad_templates {
                    if !idx.pad_templates.insert(pad.clone()) {
                        bail!("{:?} has a duplicate pad template {pad}", dep)
                    }
                    if !global_pad_templates.insert(pad.clone()) {
                        bail!("pad template names must be globally unique {pad} is used more than once")
                    }
                }
                if dep.limit as usize > pad_templates.len() {
                    bail!(
                        "{:?} does not have enough pad templates {} are required {} are provided",
                        dep,
                        dep.limit,
                        pad_templates.len()
                    )
                }
            }
        }
        Ok(())
    }

    pub(super) fn dirty(&mut self) {
        self.dirty = true
    }

    pub(super) fn take_dirty(&mut self) -> bool {
        let cur = self.dirty;
        self.dirty = false;
        cur
    }

    pub fn slot_instance_unit<'lua>(&self, lua: MizLua<'lua>, slot: &SlotId) -> Result<Unit<'lua>> {
        self.object_id_by_slot
            .get(slot)
            .ok_or_else(|| anyhow!("unit {:?} not currently in the mission", slot))
            .and_then(|id| Unit::get_instance(lua, id))
    }

    pub fn instance_unit<'lua>(&self, lua: MizLua<'lua>, uid: &UnitId) -> Result<Unit<'lua>> {
        self.object_id_by_uid
            .get(uid)
            .ok_or_else(|| anyhow!("unit {:?} not currently in the mission", uid))
            .and_then(|id| Unit::get_instance(lua, id))
    }

    pub fn slot_instance_pos(&self, lua: MizLua, slot: &SlotId) -> Result<Position3> {
        self.slot_instance_unit(lua, slot)?.get_position()
    }

    pub fn players_to_force_to_spectators<'a>(&'a mut self) -> impl Iterator<Item = Ucid> + 'a {
        self.force_to_spectators.drain()
    }

    pub fn cancel_force_to_spectators(&mut self, ucid: &Ucid) {
        self.force_to_spectators.remove(ucid);
    }

    pub(super) fn player_deslot(&mut self, slot: &SlotId) -> Option<(UnitId, Ucid)> {
        if let Some(ucid) = self.players_by_slot.remove(slot) {
            self.force_to_spectators.insert(ucid.clone());
            self.cargo.remove(slot);
            if let Some(id) = self.object_id_by_slot.remove(slot) {
                self.slot_by_object_id.remove(&id);
                if let Some(uid) = self.uid_by_object_id.remove(&id) {
                    self.object_id_by_uid.remove(&uid);
                    self.units_able_to_move.remove(&uid);
                    return Some((uid, ucid));
                }
            }
        }
        None
    }

    pub(super) fn unit_dead(&mut self, id: &DcsOid<ClassUnit>) -> Option<(UnitId, Option<Ucid>)> {
        let (uid, ucid) = match self.slot_by_object_id.remove(&id) {
            None => match self.uid_by_object_id.remove(&id) {
                Some(uid) => {
                    self.object_id_by_uid.remove(&uid);
                    (uid, None)
                }
                None => {
                    info!("no uid for object id {:?}", id);
                    return None;
                }
            },
            Some(slot) => match self.player_deslot(&slot) {
                Some((uid, ucid)) => (uid, Some(ucid)),
                None => {
                    info!("deslot player in slot {:?} failed", slot);
                    return None;
                }
            },
        };
        self.units_potentially_close_to_enemies.remove(&uid);
        self.units_potentially_on_walkabout.remove(&uid);
        self.units_able_to_move.remove(&uid);
        Some((uid, ucid))
    }

    pub fn player_in_slot(&self, slot: &SlotId) -> Option<&Ucid> {
        self.players_by_slot.get(&slot)
    }

    pub fn player_in_unit(&self, id: &DcsOid<ClassUnit>) -> Option<&Ucid> {
        self.slot_by_object_id
            .get(id)
            .and_then(|slot| self.players_by_slot.get(slot))
    }

    pub(super) fn set_cfg(&mut self, miz: &Miz, mizidx: &MizIndex, cfg: Cfg) -> Result<()> {
        let check_unit_classification = || -> Result<()> {
            let mut not_classified = FxHashSet::default();
            for side in [Side::Blue, Side::Red, Side::Neutral] {
                let coa = miz.coalition(side)?;
                for country in coa.countries()? {
                    let country = country?;
                    for group in country
                        .planes()?
                        .into_iter()
                        .chain(country.helicopters()?)
                        .chain(country.vehicles()?)
                        .chain(country.ships()?)
                        .chain(country.statics()?)
                    {
                        let group = group?;
                        for unit in group.units()? {
                            let typ = unit?.typ()?;
                            if !cfg.unit_classification.contains_key(typ.as_str()) {
                                not_classified.insert(typ);
                            }
                        }
                    }
                }
            }
            if not_classified.is_empty() {
                Ok(())
            } else {
                bail!("unit types not classified {:?}", not_classified)
            }
        };
        check_unit_classification()?;
        for (side, template) in cfg.crate_template.iter() {
            miz.get_group_by_name(mizidx, GroupKind::Any, *side, template)?
                .ok_or_else(|| anyhow!("missing crate template {:?} {template}", side))?;
        }
        let mut global_pad_templates = FxHashSet::default();
        for (side, deployables) in cfg.deployables.iter() {
            let repair_crate = maybe!(cfg.repair_crate, side, "side repair crate")?.clone();
            self.index_deployables_for_side(
                &mut global_pad_templates,
                miz,
                mizidx,
                *side,
                repair_crate,
                &cfg.warehouse,
                deployables,
            )?
        }
        for (side, troops) in cfg.troops.iter() {
            let idx = Arc::make_mut(self.deployable_idx.entry(*side).or_default());
            for troop in troops {
                miz.get_group_by_name(mizidx, GroupKind::Any, *side, &troop.template)?
                    .ok_or_else(|| anyhow!("missing troop template {:?} {:?}", side, troop.name))?;
                match idx.squads_by_name.entry(troop.name.clone()) {
                    Entry::Occupied(_) => bail!("duplicate squad name {}", troop.name),
                    Entry::Vacant(e) => e.insert(troop.clone()),
                };
            }
        }
        self.cfg = cfg;
        Ok(())
    }
}

pub(super) fn spawn_group<'lua>(
    persisted: &Persisted,
    idx: &MizIndex,
    spctx: &SpawnCtx,
    group: &SpawnedGroup,
) -> Result<()> {
    let template = spctx
        .get_template(
            idx,
            GroupKind::Any,
            group.side,
            group.template_name.as_str(),
        )
        .with_context(|| format_compact!("getting template {}", group.template_name))?;
    template.group.set("lateActivation", false)?;
    template.group.set("hidden", false)?;
    template.group.set_name(group.name.clone())?;
    let mut points: SmallVec<[Vector2; 16]> = smallvec![];
    let by_tname: FxHashMap<&str, &SpawnedUnit> = group
        .units
        .into_iter()
        .filter_map(|uid| {
            persisted.units.get(uid).and_then(|u| {
                points.push(u.pos);
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
                    unit.raw_remove("unitId")?;
                    unit.set_pos(su.pos)?;
                    unit.set_heading(su.heading)?;
                    unit.set_name(su.name.clone())?;
                    i += 1;
                }
            }
        }
        units.len() > 0
    };
    if alive {
        let point = centroid2d(points.iter().map(|p| *p));
        template.group.set_pos(point)?;
        let radius = points
            .iter()
            .map(|p: &Vector2| na::distance_squared(&(*p).into(), &point.into()))
            .fold(0., |acc, d| if d > acc { d } else { acc });
        spctx
            .remove_junk(point, radius.sqrt() * 1.10)
            .with_context(|| {
                format_compact!("removing junk before spawn of {}", group.template_name)
            })?;
        spctx
            .spawn(template)
            .with_context(|| format_compact!("spawning template {}", group.template_name))
    } else {
        Ok(())
    }
}
