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
    group::{SpawnedGroup, SpawnedUnit},
    logistics::LogiStage,
    markup::ObjectiveMarkup,
    objective::Objective,
    persisted::Persisted,
};
use crate::{
    bg::Task,
    maybe,
    msgq::MsgQ,
    spawnctx::{Despawn, SpawnCtx, Spawned},
};
use anyhow::{Context, Result, anyhow, bail};
use bfprotocols::{
    cfg::{
        ActionKind, AiPlaneCfg, AwacsCfg, BomberCfg, Cfg, Crate, Deployable, DeployableCfg,
        DeployableKind, DeployableObjective, DroneCfg, Troop, UnitTag, Vehicle, VictoryCondition,
        WarehouseConfig,
    },
    db::{
        group::{GroupId, UnitId},
        objective::ObjectiveId,
    },
    perf::PerfInner,
    stats::Stat,
};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{
    MizLua, Position3, String, Vector2,
    airbase::ClassAirbase,
    centroid2d,
    coalition::Side,
    controller::MissionPoint,
    env::miz::{self, GroupKind, Miz, MizIndex},
    group::ClassGroup,
    net::{SlotId, Ucid},
    object::{DcsObject, DcsOid},
    perf::record_perf,
    static_object::ClassStatic,
    trigger::MarkId,
    unit::{ClassUnit, Unit},
    warehouse::LiquidType,
};
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use indexmap::{IndexMap, IndexSet};
use log::{error, info};
use mlua::prelude::*;
use smallvec::{SmallVec, smallvec};
use std::{
    cmp::max,
    collections::{BTreeMap, VecDeque, hash_map::Entry},
    mem,
    sync::Arc,
};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone)]
pub struct SlotInfo {
    pub unit_name: String,
    pub typ: Vehicle,
    pub objective: ObjectiveId,
    pub ground_start: bool,
    pub miz_gid: miz::GroupId,
    pub side: Side,
}

#[derive(Debug, Clone, Default)]
pub(super) struct DeployableIndex {
    pub(super) deployables_by_name: FxHashMap<String, Deployable>,
    pub(super) deployables_by_crates: FxHashMap<String, String>,
    pub(super) deployables_by_repair: FxHashMap<String, String>,
    pub(super) crates_by_name: FxHashMap<String, Crate>,
    pub(super) squads_by_name: FxHashMap<String, Troop>,
    pub(super) pad_templates: FxHashMap<String, FxHashSet<String>>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Equipment {
    pub(super) production: u32,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Production {
    pub(super) equipment: FxHashMap<String, Equipment>,
    pub(super) liquids: FxHashMap<LiquidType, u32>,
}

#[derive(Debug)]
pub struct Ephemeral {
    pub(super) dirty: bool,
    pub cfg: Arc<Cfg>,
    pub(super) to_bg: Option<UnboundedSender<Task>>,
    pub(super) players_by_slot: IndexMap<SlotId, Ucid, FxBuildHasher>,
    pub(super) cargo: FxHashMap<SlotId, Cargo>,
    pub(super) deployable_idx: FxHashMap<Side, Arc<DeployableIndex>>,
    pub(super) group_marks: FxHashMap<GroupId, MarkId>,
    objective_markup: FxHashMap<ObjectiveId, ObjectiveMarkup>,
    pub(super) object_id_by_uid: FxHashMap<UnitId, DcsOid<ClassUnit>>,
    pub(super) uid_by_object_id: FxHashMap<DcsOid<ClassUnit>, UnitId>,
    pub(super) object_id_by_slot: FxHashMap<SlotId, DcsOid<ClassUnit>>,
    pub(super) slot_by_object_id: FxHashMap<DcsOid<ClassUnit>, SlotId>,
    pub(super) object_id_by_gid: FxHashMap<GroupId, DcsOid<ClassGroup>>,
    pub(super) gid_by_object_id: FxHashMap<DcsOid<ClassGroup>, GroupId>,
    pub(super) uid_by_static: FxHashMap<DcsOid<ClassStatic>, UnitId>,
    pub(super) slot_by_miz_gid: FxHashMap<miz::GroupId, SlotId>,
    pub(super) airbase_by_oid: FxHashMap<ObjectiveId, DcsOid<ClassAirbase>>,
    pub(super) slot_info: FxHashMap<SlotId, SlotInfo>,
    used_pad_templates: FxHashSet<String>,
    pub(super) global_pad_templates: FxHashSet<String>,
    force_to_spectators: BTreeMap<DateTime<Utc>, SmallVec<[Ucid; 1]>>,
    pub(super) units_able_to_move: IndexSet<UnitId, FxBuildHasher>,
    pub(super) groups_with_move_missions: FxHashMap<GroupId, Vector2>,
    pub(super) units_potentially_close_to_enemies: FxHashSet<UnitId>,
    pub(super) production_by_side: FxHashMap<Side, Arc<Production>>,
    pub(super) actions_taken: FxHashMap<Side, FxHashMap<String, u32>>,
    pub(super) delayspawnq: BTreeMap<DateTime<Utc>, SmallVec<[GroupId; 8]>>,
    pub(super) awacs_stn: u32,
    pub(super) logistics_stage: LogiStage,
    spawnq: VecDeque<GroupId>,
    despawnq: VecDeque<(GroupId, Despawn)>,
    sync_warehouse: Vec<(ObjectiveId, Vehicle)>,
    pub(super) msgs: MsgQ,
    pub(super) victory: Option<(DateTime<Utc>, Side)>,
}

impl Default for Ephemeral {
    fn default() -> Self {
        Self {
            dirty: false,
            cfg: Arc::new(Cfg::default()),
            to_bg: None,
            players_by_slot: IndexMap::default(),
            cargo: FxHashMap::default(),
            deployable_idx: FxHashMap::default(),
            group_marks: FxHashMap::default(),
            objective_markup: FxHashMap::default(),
            object_id_by_uid: FxHashMap::default(),
            uid_by_object_id: FxHashMap::default(),
            object_id_by_slot: FxHashMap::default(),
            slot_by_object_id: FxHashMap::default(),
            slot_by_miz_gid: FxHashMap::default(),
            object_id_by_gid: FxHashMap::default(),
            gid_by_object_id: FxHashMap::default(),
            uid_by_static: FxHashMap::default(),
            airbase_by_oid: FxHashMap::default(),
            slot_info: FxHashMap::default(),
            used_pad_templates: FxHashSet::default(),
            global_pad_templates: FxHashSet::default(),
            force_to_spectators: BTreeMap::default(),
            units_able_to_move: IndexSet::default(),
            groups_with_move_missions: FxHashMap::default(),
            units_potentially_close_to_enemies: FxHashSet::default(),
            production_by_side: FxHashMap::default(),
            actions_taken: FxHashMap::default(),
            delayspawnq: BTreeMap::default(),
            awacs_stn: 0o77777,
            spawnq: VecDeque::default(),
            despawnq: VecDeque::default(),
            sync_warehouse: Vec::default(),
            msgs: MsgQ::default(),
            logistics_stage: LogiStage::default(),
            victory: None,
        }
    }
}

impl Ephemeral {
    fn do_bg(&self, task: Task) {
        if let Some(to_bg) = &self.to_bg {
            match to_bg.send(task) {
                Ok(()) => (),
                Err(_) => panic!("background thread is dead"),
            }
        }
    }

    pub fn stat(&self, stat: Stat) {
        self.do_bg(Task::Stat(stat))
    }

    pub fn get_slot_info(&self, slot: &SlotId) -> Option<&SlotInfo> {
        self.slot_info.get(slot)
    }

    pub fn get_slot_info_by_miz_gid(&self, gid: &miz::GroupId) -> Option<(SlotId, &SlotInfo)> {
        self.slot_by_miz_gid
            .get(gid)
            .and_then(|sl| self.slot_info.get(sl).map(|s| (*sl, s)))
    }

    pub fn create_objective_markup(&mut self, persisted: &Persisted, obj: &Objective) {
        if let Some(mk) = self.objective_markup.remove(&obj.id) {
            mk.remove(&mut self.msgs);
        }
        self.objective_markup.insert(
            obj.id,
            ObjectiveMarkup::new(&self.cfg, &mut self.msgs, obj, persisted),
        );
    }

    pub fn update_objective_markup(
        &mut self,
        persisted: &Persisted,
        obj: &Objective,
        moved: &[ObjectiveId],
    ) {
        match self.objective_markup.entry(obj.id) {
            Entry::Occupied(mut e) => e.get_mut().update(persisted, &mut self.msgs, obj, moved),
            Entry::Vacant(e) => {
                e.insert(ObjectiveMarkup::new(
                    &self.cfg,
                    &mut self.msgs,
                    obj,
                    persisted,
                ));
            }
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

    pub fn spawnq_len(&self) -> usize {
        self.spawnq.len()
    }

    pub fn process_spawn_queue(
        &mut self,
        perf: &mut PerfInner,
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
            for _ in 0..max(1, dlen >> 4) {
                if let Some((gid, despawn)) = self.despawnq.pop_front() {
                    if let Some(group) = persisted.groups.get(&gid) {
                        if let Some(id) = self.object_id_by_gid.remove(&gid) {
                            self.gid_by_object_id.remove(&id);
                        }
                        for uid in &group.units {
                            self.units_able_to_move.swap_remove(uid);
                            self.units_potentially_close_to_enemies.remove(uid);
                            if let Some(id) = self.object_id_by_uid.remove(uid) {
                                self.uid_by_object_id.remove(&id);
                            }
                        }
                    }
                    spctx.despawn(perf, despawn)?;
                }
            }
        } else if slen > 0 {
            for _ in 0..max(1, slen >> 4) {
                if let Some(gid) = self.spawnq.pop_front() {
                    let group = maybe!(persisted.groups, gid, "group")?;
                    self.spawn_group(perf, persisted, idx, spctx, group, vec![])?;
                }
            }
        }
        Ok(())
    }

    pub fn take_pad_template(&mut self, side: Side, name: &String) -> Option<String> {
        self.deployable_idx.get(&side).and_then(|idx| {
            if let Some(templates) = idx.pad_templates.get(name) {
                for pad in templates {
                    if self.used_pad_templates.insert(pad.clone()) {
                        return Some(pad.clone());
                    }
                }
            }
            None
        })
    }

    pub fn return_pad_template(&mut self, pad: &str) {
        self.used_pad_templates.remove(pad);
    }

    pub fn set_pad_template_used(&mut self, pad: String) {
        self.used_pad_templates.insert(pad);
    }

    pub fn msgs(&mut self) -> &mut MsgQ {
        &mut self.msgs
    }

    pub fn get_uid_by_object_id(&self, id: &DcsOid<ClassUnit>) -> Option<&UnitId> {
        self.uid_by_object_id.get(id)
    }

    pub fn get_object_id_by_uid(&self, id: &UnitId) -> Option<&DcsOid<ClassUnit>> {
        self.object_id_by_uid.get(id)
    }

    pub fn get_slot_by_object_id(&self, id: &DcsOid<ClassUnit>) -> Option<&SlotId> {
        self.slot_by_object_id.get(id)
    }

    pub fn get_object_id_by_slot(&self, id: &SlotId) -> Option<&DcsOid<ClassUnit>> {
        self.object_id_by_slot.get(id)
    }

    fn index_deployables_for_side(
        &mut self,
        miz: &Miz,
        mizidx: &MizIndex,
        side: Side,
        repair_crate: Crate,
        whcfg: &Option<WarehouseConfig>,
        points: bool,
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
            if let DeployableKind::Group { template } = &dep.kind {
                miz.get_group_by_name(mizidx, GroupKind::Any, side, template)?
                    .ok_or_else(|| anyhow!("missing deployable template {:?} {:?}", side, dep))?;
            }
            if !points && dep.cost > 0 {
                bail!(
                    "the points system is disabled, but {:?} costs points",
                    dep.path
                )
            }
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
            if let DeployableKind::Objective(DeployableObjective {
                pad_templates,
                defenses_template,
                ammo_template,
                fuel_template,
                barracks_template,
            }) = &dep.kind
            {
                let mut names = FxHashSet::default();
                for name in defenses_template
                    .iter()
                    .chain(ammo_template.iter())
                    .chain(fuel_template.iter())
                    .chain(barracks_template.iter())
                    .chain(pad_templates.iter())
                {
                    miz.get_group_by_name(mizidx, GroupKind::Any, side, name)?
                        .ok_or_else(|| anyhow!("missing farp template {:?} {:?}", side, name))?;
                    if !names.insert(name) {
                        bail!(
                            "deployables with logistics must use unique templates for each part {name} is reused"
                        )
                    }
                }
                for pad in pad_templates {
                    if !idx
                        .pad_templates
                        .entry(name.clone())
                        .or_default()
                        .insert(pad.clone())
                    {
                        bail!("{:?} has a duplicate pad template {pad}", dep)
                    }
                    if !self.global_pad_templates.insert(pad.clone()) {
                        bail!(
                            "pad template names must be globally unique {pad} is used more than once"
                        )
                    }
                    let gifo = miz
                        .get_group_by_name(mizidx, GroupKind::Any, side, pad)?
                        .ok_or_else(|| anyhow!("missing pad template {:?} {:?}", side, pad))?;
                    for unit in gifo.group.units()? {
                        let unit = unit?;
                        let uname = unit.name()?;
                        if &uname != pad {
                            bail!(
                                "pad template groups and units must be named the same thing {uname} != {pad}"
                            )
                        }
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

    pub fn dirty(&mut self) {
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

    #[allow(dead_code)]
    pub fn instance_unit<'lua>(&self, lua: MizLua<'lua>, uid: &UnitId) -> Result<Unit<'lua>> {
        self.object_id_by_uid
            .get(uid)
            .ok_or_else(|| anyhow!("unit {:?} not currently in the mission", uid))
            .and_then(|id| Unit::get_instance(lua, id))
    }

    pub fn slot_instance_pos(&self, lua: MizLua, slot: &SlotId) -> Result<Position3> {
        self.slot_instance_unit(lua, slot)?.get_position()
    }

    pub fn players_to_force_to_spectators<'a>(
        &'a mut self,
        now: DateTime<Utc>,
    ) -> BTreeMap<DateTime<Utc>, SmallVec<[Ucid; 1]>> {
        let keep = self.force_to_spectators.split_off(&now);
        mem::replace(&mut self.force_to_spectators, keep)
    }

    pub fn cancel_force_to_spectators(&mut self, ucid: &Ucid) {
        info!("canceling force to spectators for {ucid}");
        self.force_to_spectators.retain(|_, ids| {
            ids.retain(|pucid| pucid != ucid);
            !ids.is_empty()
        })
    }

    pub fn force_player_to_spectators(&mut self, ucid: &Ucid) {
        self.force_to_spectators
            .entry(Utc::now())
            .or_default()
            .push(ucid.clone())
    }

    pub fn force_player_to_spectators_at(&mut self, ucid: &Ucid, ts: DateTime<Utc>) {
        self.force_to_spectators
            .entry(ts)
            .or_default()
            .push(ucid.clone())
    }

    pub(super) fn player_deslot(
        &mut self,
        per: &Persisted,
        slot: &SlotId,
        expected_ucid: Option<Ucid>,
    ) -> Option<(UnitId, Ucid)> {
        if let Some(ucid) = self.players_by_slot.swap_remove(slot) {
            if let Some(expected_ucid) = expected_ucid {
                if expected_ucid != ucid {
                    error!("players_by_slot ucid mismatch {expected_ucid} vs {ucid} in slot {slot}")
                }
            }
            info!("deslotting player {ucid}");
            if let Some(player) = per.players.get(&ucid) {
                if !player.changing_slots && !player.jtac_or_spectators {
                    info!("queuing force player {ucid} to spectators");
                    self.force_to_spectators
                        .entry(Utc::now())
                        .or_default()
                        .push(ucid.clone());
                }
            }
            self.cargo.remove(slot);
            if let Some(id) = self.object_id_by_slot.remove(slot) {
                self.slot_by_object_id.remove(&id);
                if let Some(uid) = self.uid_by_object_id.remove(&id) {
                    self.object_id_by_uid.remove(&uid);
                    self.units_able_to_move.swap_remove(&uid);
                    return Some((uid, ucid));
                }
            }
            error!("have ucid but no unitid for dead slot {slot} {ucid}");
        }
        None
    }

    pub(super) fn unit_dead(
        &mut self,
        per: &Persisted,
        id: &DcsOid<ClassUnit>,
    ) -> Option<(UnitId, Option<Ucid>)> {
        let (uid, ucid) = match self.slot_by_object_id.remove(id) {
            Some(slot) => match self.player_deslot(per, &slot, None) {
                Some((uid, ucid)) => (uid, Some(ucid)),
                None => return None,
            },
            None => match self.uid_by_object_id.remove(id) {
                Some(uid) => {
                    self.object_id_by_uid.remove(&uid);
                    (uid, None)
                }
                None => {
                    info!("no uid for object id {:?}", id);
                    return None;
                }
            },
        };
        self.units_potentially_close_to_enemies.remove(&uid);
        self.units_able_to_move.swap_remove(&uid);
        Some((uid, ucid))
    }

    pub fn player_in_slot(&self, slot: &SlotId) -> Option<&Ucid> {
        self.players_by_slot.get(slot)
    }

    pub fn player_in_unit(&self, id: &DcsOid<ClassUnit>) -> Option<&Ucid> {
        self.slot_by_object_id
            .get(id)
            .and_then(|slot| self.players_by_slot.get(slot))
    }

    pub fn panel_to_player<S: Into<String>>(
        &mut self,
        persisted: &Persisted,
        duration: i64,
        ucid: &Ucid,
        msg: S,
    ) {
        if let Some(player) = persisted.players.get(ucid) {
            if let Some(ifo) = player
                .current_slot
                .as_ref()
                .and_then(|(s, _)| self.slot_info.get(s))
            {
                let miz_id = ifo.miz_gid;
                self.msgs().panel_to_group(duration, false, miz_id, msg);
            }
        }
    }

    pub(super) fn set_cfg(
        &mut self,
        miz: &Miz,
        mizidx: &MizIndex,
        cfg: Arc<Cfg>,
        to_bg: UnboundedSender<Task>,
    ) -> Result<()> {
        self.to_bg = Some(to_bg);
        let check_unit_classification = || -> Result<()> {
            let mut not_classified = FxHashSet::default();
            for side in Side::ALL {
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
        if let Some(VictoryCondition::MapOwned { fraction }) = cfg.auto_reset.map(|vc| vc.condition)
        {
            if fraction > 1. || fraction < 0. {
                bail!("auto_reset fraction must be between 0 and 1")
            }
        }
        for (side, template) in cfg.crate_template.iter() {
            miz.get_group_by_name(mizidx, GroupKind::Any, *side, template)?
                .ok_or_else(|| anyhow!("missing crate template {:?} {template}", side))?;
        }
        let points = cfg.points.is_some();
        for (side, deployables) in cfg.deployables.iter() {
            let repair_crate = maybe!(cfg.repair_crate, side, "side repair crate")?.clone();
            self.index_deployables_for_side(
                miz,
                mizidx,
                *side,
                repair_crate,
                &cfg.warehouse,
                points,
                deployables,
            )?
        }
        for (side, troops) in cfg.troops.iter() {
            let idx = Arc::make_mut(self.deployable_idx.entry(*side).or_default());
            for troop in troops {
                miz.get_group_by_name(mizidx, GroupKind::Any, *side, &troop.template)?
                    .ok_or_else(|| anyhow!("missing troop template {:?} {:?}", side, troop.name))?;
                if !points && troop.cost > 0 {
                    bail!(
                        "the points system is disabled but {} troops cost points",
                        troop.name
                    )
                }
                match idx.squads_by_name.entry(troop.name.clone()) {
                    Entry::Occupied(_) => bail!("duplicate squad name {}", troop.name),
                    Entry::Vacant(e) => e.insert(troop.clone()),
                };
            }
        }
        for (side, actions) in &cfg.actions {
            for (_, act) in actions {
                if !points && (act.cost > 0 || act.penalty.unwrap_or(0) > 0) {
                    bail!("the points system is disabled but {act:?} costs points")
                }
                match &act.kind {
                    ActionKind::Awacs(AwacsCfg {
                        plane: AiPlaneCfg { template, .. },
                        ..
                    })
                    | ActionKind::Bomber(BomberCfg {
                        plane: AiPlaneCfg { template, .. },
                        ..
                    })
                    | ActionKind::CruiseMissileSpawn(AiPlaneCfg { template, .. })
                    | ActionKind::Tanker(AiPlaneCfg { template, .. })
                    | ActionKind::Drone(DroneCfg {
                        plane: AiPlaneCfg { template, .. },
                        ..
                    })
                    | ActionKind::Fighters(AiPlaneCfg { template, .. })
                    | ActionKind::Attackers(AiPlaneCfg { template, .. })
                    | ActionKind::LogisticsRepair(AiPlaneCfg { template, .. })
                    | ActionKind::LogisticsTransfer(AiPlaneCfg { template, .. }) => {
                        miz.get_group_by_name(mizidx, GroupKind::Any, *side, template.as_str())?
                            .ok_or_else(|| anyhow!("missing template for action {act:?}"))?;
                    }
                    ActionKind::Deployable(DeployableCfg { name, plane }) => {
                        if let Some(AiPlaneCfg { template, .. }) = plane {
                            miz.get_group_by_name(
                                mizidx,
                                GroupKind::Any,
                                *side,
                                template.as_str(),
                            )?
                            .ok_or_else(|| anyhow!("missing template for action {act:?}"))?;
                        }
                        self.deployable_idx
                            .get(side)
                            .and_then(|idx| idx.deployables_by_name.get(name))
                            .ok_or_else(|| anyhow!("missing deployable for action {act:?}"))?;
                    }
                    ActionKind::Paratrooper(DeployableCfg {
                        name,
                        plane: Some(AiPlaneCfg { template, .. }),
                    }) => {
                        miz.get_group_by_name(mizidx, GroupKind::Any, *side, template.as_str())?
                            .ok_or_else(|| anyhow!("missing template for action {act:?}"))?;
                        self.deployable_idx
                            .get(side)
                            .and_then(|idx| idx.squads_by_name.get(name))
                            .ok_or_else(|| anyhow!("missing troop for action {act:?}"))?;
                    }
                    ActionKind::Paratrooper(DeployableCfg { name, plane: None }) => {
                        bail!("patroop mission {name} does not include an ai plane config")
                    }
                    ActionKind::AwacsWaypoint
                    | ActionKind::TankerWaypoint
                    | ActionKind::DroneWaypoint
                    | ActionKind::CruiseMissileWaypoint                    
                    | ActionKind::FighersWaypoint
                    | ActionKind::AttackersWaypoint
                    | ActionKind::Move(_)
                    | ActionKind::Nuke(_) => (),
                }
            }
        }
        self.cfg = cfg;
        Ok(())
    }

    pub(super) fn spawn_group<'lua>(
        &mut self,
        perf: &mut PerfInner,
        persisted: &Persisted,
        idx: &MizIndex,
        spctx: &SpawnCtx<'lua>,
        group: &SpawnedGroup,
        mission: Vec<MissionPoint<'lua>>,
    ) -> Result<Option<Spawned<'lua>>> {
        let ts = Utc::now();
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
        if mission.len() > 0 {
            template
                .group
                .route()
                .context("getting route")?
                .set_points(mission)
                .context("setting points")?;
        }
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
            let units = template.group.units().context("getting units")?;
            let mut i = 1;
            while i as usize <= units.len() {
                let unit = units.get(i)?;
                match by_tname.get(unit.name()?.as_str()) {
                    None => units.remove(i)?,
                    Some(su) => {
                        if su.tags.contains(UnitTag::AWACS) {
                            let stn = String::from(format_compact!("{:005o}", self.awacs_stn));
                            if let Ok(props) = unit.raw_get::<_, LuaTable>("AddPropAircraft") {
                                self.awacs_stn -= 1;
                                props.raw_set("STN_L16", stn)?;
                            }
                        }
                        unit.raw_remove("unitId")?;
                        unit.set_pos(su.pos)?;
                        unit.set_alt(su.position.p.y)?;
                        unit.set_heading(su.heading)?;
                        unit.set_name(su.name.clone())?;
                        i += 1;
                    }
                }
            }
            units.len() > 0
        };
        if !alive {
            record_perf(&mut perf.spawn, ts);
            Ok(None)
        } else {
            let point = centroid2d(points.iter().map(|p| *p));
            template.group.set_pos(point)?;
            /*
            let radius = points
                .iter()
                .map(|p: &Vector2| na::distance_squared(&(*p).into(), &point.into()))
                .fold(0., |acc, d| if d > acc { d } else { acc });
            let radius = radius.sqrt();
            spctx.remove_junk(point, radius * 1.10).with_context(|| {
                format_compact!("removing junk before spawn of {}", group.template_name)
            })?;
            */
            let spawned = spctx
                .spawn(template)
                .with_context(|| format_compact!("spawning template {}", group.template_name))?;
            match &spawned {
                Spawned::Static => (),
                Spawned::Group(g) => {
                    let oid = g.object_id()?;
                    self.object_id_by_gid.insert(group.id, oid.clone());
                    self.gid_by_object_id.insert(oid, group.id);
                }
            }
            record_perf(&mut perf.spawn, ts);
            Ok(Some(spawned))
        }
    }
}
