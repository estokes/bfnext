use super::{cargo::Cargo, group::{GroupId, UnitId}, objective::ObjectiveId};
use crate::{
    cfg::{Cfg, Crate, Deployable, DeployableLogistics, Troop},
    maybe,
    msgq::MsgQ,
    spawnctx::Despawn,
};
use anyhow::{anyhow, bail, Result};
use chrono::prelude::*;
use dcso3::{
    airbase::ClassAirbase,
    coalition::Side,
    env::miz::{GroupKind, Miz, MizIndex},
    net::{SlotId, Ucid},
    object::{DcsObject, DcsOid},
    trigger::MarkId,
    unit::{ClassUnit, Unit},
    warehouse::ClassWarehouse,
    MizLua, Position3, String,
};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use std::{
    collections::{hash_map::Entry, BTreeMap, VecDeque},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub(super) struct ObjLogi {
    pub(super) airbase: DcsOid<ClassAirbase>,
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
    pub(super) units_able_to_move: FxHashSet<UnitId>,
    pub(super) units_potentially_close_to_enemies: FxHashSet<UnitId>,
    pub(super) units_potentially_on_walkabout: FxHashSet<UnitId>,
    pub(super) delayspawnq: BTreeMap<DateTime<Utc>, SmallVec<[GroupId; 8]>>,
    pub(super) spawnq: VecDeque<GroupId>,
    pub(super) despawnq: VecDeque<(GroupId, Despawn)>,
    pub(super) msgs: MsgQ,
}

impl Ephemeral {
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

    pub(super) fn player_deslot(&mut self, slot: &SlotId) {
        self.players_by_slot.remove(slot);
        self.cargo.remove(slot);
        if let Some(id) = self.object_id_by_slot.remove(slot) {
            self.slot_by_object_id.remove(&id);
            if let Some(uid) = self.uid_by_object_id.remove(&id) {
                self.object_id_by_uid.remove(&uid);
                self.units_able_to_move.remove(&uid);
            }
        }
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
