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
    group::{DeployKind, GroupId, SpawnedGroup, SpawnedUnit, UnitId},
    markup::ObjectiveMarkup,
    objective::{Objective, ObjectiveId},
    persisted::Persisted,
};
use crate::{
    cfg::{
        ActionKind, AiPlaneCfg, BomberCfg, Cfg, Crate, Deployable, DeployableCfg,
        DeployableLogistics, DroneCfg, Troop, Vehicle, WarehouseConfig,
    },
    maybe,
    msgq::MsgQ,
    spawnctx::{Despawn, SpawnCtx, SpawnLoc, Spawned},
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{
    airbase::ClassAirbase,
    centroid2d,
    coalition::Side,
    controller::{MissionPoint, PointType, Task},
    env::miz::{GroupKind, Miz, MizIndex},
    net::{SlotId, Ucid},
    object::{DcsObject, DcsOid},
    pointing_towards2,
    static_object::ClassStatic,
    trigger::MarkId,
    unit::{ClassUnit, Unit},
    warehouse::{LiquidType, WSCategory},
    LuaVec2, MizLua, Position3, String, Vector2,
};
use fxhash::{FxBuildHasher, FxHashMap, FxHashSet};
use indexmap::IndexSet;
use log::info;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::max,
    collections::{hash_map::Entry, BTreeMap, VecDeque},
    mem,
    sync::Arc,
};

#[derive(Debug, Clone, Default)]
pub(super) struct DeployableIndex {
    pub(super) deployables_by_name: FxHashMap<String, Deployable>,
    pub(super) deployables_by_crates: FxHashMap<String, String>,
    pub(super) deployables_by_repair: FxHashMap<String, String>,
    pub(super) crates_by_name: FxHashMap<String, Crate>,
    pub(super) squads_by_name: FxHashMap<String, Troop>,
    pub(super) pad_templates: FxHashSet<String>,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Equipment {
    pub(super) category: WSCategory,
    pub(super) production: u32,
}

#[derive(Debug, Clone, Default)]
pub(super) struct Production {
    pub(super) equipment: FxHashMap<String, Equipment>,
    pub(super) liquids: FxHashMap<LiquidType, u32>,
}

#[derive(Debug, Default)]
pub struct Ephemeral {
    pub(super) dirty: bool,
    pub cfg: Arc<Cfg>,
    pub(super) players_by_slot: FxHashMap<SlotId, Ucid>,
    pub(super) cargo: FxHashMap<SlotId, Cargo>,
    pub(super) deployable_idx: FxHashMap<Side, Arc<DeployableIndex>>,
    pub(super) group_marks: FxHashMap<GroupId, SmallVec<[MarkId; 2]>>,
    objective_markup: FxHashMap<ObjectiveId, ObjectiveMarkup>,
    pub(super) object_id_by_uid: FxHashMap<UnitId, DcsOid<ClassUnit>>,
    pub(super) uid_by_object_id: FxHashMap<DcsOid<ClassUnit>, UnitId>,
    pub(super) object_id_by_slot: FxHashMap<SlotId, DcsOid<ClassUnit>>,
    pub(super) slot_by_object_id: FxHashMap<DcsOid<ClassUnit>, SlotId>,
    pub(super) uid_by_static: FxHashMap<DcsOid<ClassStatic>, UnitId>,
    pub(super) airbase_by_oid: FxHashMap<ObjectiveId, DcsOid<ClassAirbase>>,
    used_pad_templates: FxHashSet<String>,
    force_to_spectators: BTreeMap<DateTime<Utc>, SmallVec<[Ucid; 1]>>,
    pub(super) units_able_to_move: IndexSet<UnitId, FxBuildHasher>,
    pub(super) groups_with_move_missions: FxHashMap<GroupId, Vector2>,
    pub(super) units_potentially_close_to_enemies: FxHashSet<UnitId>,
    pub(super) production_by_side: FxHashMap<Side, Arc<Production>>,
    pub(super) actions_taken: FxHashMap<Side, FxHashMap<String, u32>>,
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
            for _ in 0..max(1, dlen >> 3) {
                if let Some((gid, name)) = self.despawnq.pop_front() {
                    if let Some(group) = persisted.groups.get(&gid) {
                        for uid in &group.units {
                            self.units_able_to_move.swap_remove(uid);
                            self.units_potentially_close_to_enemies.remove(uid);
                            if let Some(id) = self.object_id_by_uid.remove(uid) {
                                self.uid_by_object_id.remove(&id);
                            }
                        }
                    }
                    spctx.despawn(name)?
                }
            }
        } else if slen > 0 {
            for _ in 0..max(1, slen >> 3) {
                if let Some(gid) = self.spawnq.pop_front() {
                    let group = maybe!(persisted.groups, gid, "group")?;
                    spawn_group(persisted, idx, spctx, group)?;
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
        global_pad_templates: &mut FxHashSet<String>,
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
            miz.get_group_by_name(mizidx, GroupKind::Any, side, &dep.template)?
                .ok_or_else(|| anyhow!("missing deployable template {:?} {:?}", side, dep))?;
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

    pub(super) fn player_deslot(&mut self, slot: &SlotId, kick: bool) -> Option<(UnitId, Ucid)> {
        if let Some(ucid) = self.players_by_slot.remove(slot) {
            info!("deslotting player {ucid} from dead unit");
            if kick {
                info!("queuing force player {ucid} to spectators");
                self.force_to_spectators
                    .entry(Utc::now())
                    .or_default()
                    .push(ucid.clone());
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
        }
        None
    }

    pub(super) fn unit_dead(&mut self, id: &DcsOid<ClassUnit>) -> Option<(UnitId, Option<Ucid>)> {
        let (uid, ucid) = match self.slot_by_object_id.remove(&id) {
            Some(slot) => match self.player_deslot(&slot, true) {
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
        self.players_by_slot.get(&slot)
    }

    pub fn player_in_unit(&self, id: &DcsOid<ClassUnit>) -> Option<&Ucid> {
        self.slot_by_object_id
            .get(id)
            .and_then(|slot| self.players_by_slot.get(slot))
    }

    pub fn panel_to_player<S: Into<String>>(&mut self, persisted: &Persisted, ucid: &Ucid, msg: S) {
        if let Some(player) = persisted.players.get(ucid) {
            let ifo = player.current_slot.as_ref().and_then(|(s, _)| {
                persisted
                    .objectives_by_slot
                    .get(s)
                    .and_then(|i| persisted.objectives.get(i).and_then(|o| o.slots.get(s)))
            });
            if let Some(ifo) = ifo {
                let miz_id = ifo.miz_gid;
                self.msgs().panel_to_group(5, false, miz_id, msg);
            }
        }
    }

    pub(super) fn set_cfg(&mut self, miz: &Miz, mizidx: &MizIndex, cfg: Cfg) -> Result<()> {
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
        for (side, template) in cfg.crate_template.iter() {
            miz.get_group_by_name(mizidx, GroupKind::Any, *side, template)?
                .ok_or_else(|| anyhow!("missing crate template {:?} {template}", side))?;
        }
        let mut global_pad_templates = FxHashSet::default();
        let points = cfg.points.is_some();
        for (side, deployables) in cfg.deployables.iter() {
            let repair_crate = maybe!(cfg.repair_crate, side, "side repair crate")?.clone();
            self.index_deployables_for_side(
                &mut global_pad_templates,
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
                    ActionKind::Awacs(AiPlaneCfg { template, .. })
                    | ActionKind::Bomber(BomberCfg {
                        plane: AiPlaneCfg { template, .. },
                        ..
                    })
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
                    ActionKind::Deployable(DeployableCfg {
                        name,
                        plane: AiPlaneCfg { template, .. },
                    }) => {
                        miz.get_group_by_name(mizidx, GroupKind::Any, *side, template.as_str())?
                            .ok_or_else(|| anyhow!("missing template for action {act:?}"))?;
                        self.deployable_idx
                            .get(side)
                            .and_then(|idx| idx.deployables_by_name.get(name))
                            .ok_or_else(|| anyhow!("missing deployable for action {act:?}"))?;
                    }
                    ActionKind::Paratrooper(DeployableCfg {
                        name,
                        plane: AiPlaneCfg { template, .. },
                    }) => {
                        miz.get_group_by_name(mizidx, GroupKind::Any, *side, template.as_str())?
                            .ok_or_else(|| anyhow!("missing template for action {act:?}"))?;
                        self.deployable_idx
                            .get(side)
                            .and_then(|idx| idx.squads_by_name.get(name))
                            .ok_or_else(|| anyhow!("missing troop for action {act:?}"))?;
                    }
                    ActionKind::AwacsWaypoint
                    | ActionKind::TankerWaypoint
                    | ActionKind::DroneWaypoint
                    | ActionKind::FighersWaypoint
                    | ActionKind::AttackersWaypoint
                    | ActionKind::Move(_)
                    | ActionKind::Nuke(_) => (),
                }
            }
        }
        self.cfg = Arc::new(cfg);
        Ok(())
    }
}

pub(super) fn spawn_group<'lua>(
    persisted: &Persisted,
    idx: &MizIndex,
    spctx: &SpawnCtx<'lua>,
    group: &SpawnedGroup,
) -> Result<Option<Spawned<'lua>>> {
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
    if let DeployKind::Action { loc, .. } = &group.origin {
        match loc {
            SpawnLoc::AtPos { .. }
            | SpawnLoc::AtPosWithCenter { .. }
            | SpawnLoc::AtPosWithComponents { .. }
            | SpawnLoc::AtTrigger { .. } => (),
            SpawnLoc::InAir {
                pos,
                heading,
                altitude,
            } => {
                let dst = pos + pointing_towards2(*heading) * 10_000.;
                let route = template.group.route()?;
                macro_rules! pt {
                    ($pos:expr) => {
                        MissionPoint {
                            action: None,
                            typ: PointType::TurningPoint,
                            airdrome_id: None,
                            time_re_fu_ar: None,
                            helipad: None,
                            link_unit: None,
                            pos: LuaVec2($pos),
                            alt: *altitude,
                            alt_typ: None,
                            speed: 200.,
                            speed_locked: None,
                            eta: None,
                            eta_locked: None,
                            name: None,
                            task: Box::new(Task::ComboTask(vec![])),
                        }
                    };
                }
                route.set_points(vec![pt!(*pos), pt!(dst)])?;
                template.group.set_route(route)?;
                template.group.set("heading", *heading)?;
            }
        }
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
        let units = template.group.units()?;
        let mut i = 1;
        while i as usize <= units.len() {
            let unit = units.get(i)?;
            match by_tname.get(unit.name()?.as_str()) {
                None => units.remove(i)?,
                Some(su) => {
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
    if alive {
        let point = centroid2d(points.iter().map(|p| *p));
        template.group.set_pos(point)?;
        let radius = points
            .iter()
            .map(|p: &Vector2| na::distance_squared(&(*p).into(), &point.into()))
            .fold(0., |acc, d| if d > acc { d } else { acc });
        let radius = radius.sqrt();
        spctx.remove_junk(point, radius * 1.10).with_context(|| {
            format_compact!("removing junk before spawn of {}", group.template_name)
        })?;
        Ok(Some(spctx.spawn(template).with_context(|| {
            format_compact!("spawning template {}", group.template_name)
        })?))
    } else {
        Ok(None)
    }
}
