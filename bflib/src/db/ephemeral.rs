use super::{
    cargo::Cargo,
    group::{GroupId, SpawnedGroup, SpawnedUnit, UnitId},
    objective::ObjectiveId,
    persisted::Persisted,
};
use crate::{
    cfg::{Cfg, Crate, Deployable, DeployableLogistics, Troop},
    maybe,
    msgq::MsgQ,
    spawnctx::{Despawn, SpawnCtx},
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{
    centroid2d,
    coalition::Side,
    env::miz::{GroupKind, Miz, MizIndex},
    net::{SlotId, Ucid},
    object::{DcsObject, DcsOid},
    trigger::MarkId,
    unit::{ClassUnit, Unit},
    warehouse::ClassWarehouse,
    MizLua, Position3, String, Vector2,
};
use fxhash::{FxHashMap, FxHashSet};
use log::info;
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::max,
    collections::{hash_map::Entry, BTreeMap, VecDeque},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub(super) struct ObjLogi {
    pub(super) warehouse: DcsOid<ClassWarehouse>,
}

#[derive(Debug, Clone, Default)]
pub(super) struct DeployableIndex {
    pub(super) deployables_by_name: FxHashMap<String, Deployable>,
    pub(super) deployables_by_crates: FxHashMap<String, String>,
    pub(super) deployables_by_repair: FxHashMap<String, String>,
    pub(super) crates_by_name: FxHashMap<String, Crate>,
    pub(super) squads_by_name: FxHashMap<String, Troop>,
}

#[derive(Debug, Default)]
pub struct Ephemeral {
    dirty: bool,
    pub(super) cfg: Cfg,
    pub(super) players_by_slot: FxHashMap<SlotId, Ucid>,
    pub(super) cargo: FxHashMap<SlotId, Cargo>,
    pub(super) deployable_idx: FxHashMap<Side, Arc<DeployableIndex>>,
    pub(super) group_marks: FxHashMap<GroupId, MarkId>,
    pub(super) objective_marks: FxHashMap<ObjectiveId, (MarkId, Option<MarkId>)>,
    pub(super) object_id_by_uid: FxHashMap<UnitId, DcsOid<ClassUnit>>,
    pub(super) uid_by_object_id: FxHashMap<DcsOid<ClassUnit>, UnitId>,
    pub(super) object_id_by_slot: FxHashMap<SlotId, DcsOid<ClassUnit>>,
    pub(super) slot_by_object_id: FxHashMap<DcsOid<ClassUnit>, SlotId>,
    pub(super) logistics_by_oid: FxHashMap<ObjectiveId, ObjLogi>,
    force_to_spectators: FxHashSet<Ucid>,
    pub(super) units_able_to_move: FxHashSet<UnitId>,
    pub(super) units_potentially_close_to_enemies: FxHashSet<UnitId>,
    pub(super) units_potentially_on_walkabout: FxHashSet<UnitId>,
    pub(super) delayspawnq: BTreeMap<DateTime<Utc>, SmallVec<[GroupId; 8]>>,
    spawnq: VecDeque<GroupId>,
    despawnq: VecDeque<(GroupId, Despawn)>,
    pub(super) msgs: MsgQ,
}

impl Ephemeral {
    pub fn push_despawn(&mut self, gid: GroupId, ds: Despawn) {
        let mut queued_spawn = false;
        self.spawnq.retain(|sp_gid| {
            let qs = &gid == sp_gid;
            queued_spawn |= qs;
            !qs
        });
        if !queued_spawn {
            self.despawnq.push_back((gid, ds))
        }
    }

    pub fn push_spawn(&mut self, gid: GroupId) {
        let mut queued_despawn = false;
        self.despawnq.retain(|(ds_gid, _)| {
            let qs = &gid == ds_gid;
            queued_despawn |= qs;
            !qs
        });
        if !queued_despawn {
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
        miz: &Miz,
        mizidx: &MizIndex,
        side: Side,
        repair_crate: Crate,
        deployables: &[Deployable],
    ) -> Result<()> {
        let idx = Arc::make_mut(self.deployable_idx.entry(side).or_default());
        idx.crates_by_name
            .insert(repair_crate.name.clone(), repair_crate);
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
                pad_template,
                ammo_template,
                fuel_template,
                barracks_template,
            }) = &dep.logistics
            {
                let mut names = FxHashSet::default();
                for name in [
                    &dep.template,
                    &ammo_template,
                    &pad_template,
                    &fuel_template,
                    &barracks_template,
                ] {
                    miz.get_group_by_name(mizidx, GroupKind::Any, side, name)?
                        .ok_or_else(|| anyhow!("missing farp template {:?} {:?}", side, name))?;
                    if !names.insert(name) {
                        bail!("deployables with logistics must use unique templates for each part {name} is reused")
                    }
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
        for (side, deployables) in cfg.deployables.iter() {
            let repair_crate = maybe!(cfg.repair_crate, side, "side repair crate")?.clone();
            self.index_deployables_for_side(miz, mizidx, *side, repair_crate, deployables)?
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

fn spawn_group<'lua>(
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
