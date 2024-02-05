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
    ephemeral::{Equipment, Production},
    objective::{Objective, ObjectiveId},
    Db, Map, Set,
};
use crate::{
    admin::WarehouseKind, cfg::Vehicle, db::objective::ObjectiveKind, maybe, objective,
    objective_mut,
};
use anyhow::{anyhow, bail, Context, Result};
use compact_str::{format_compact, CompactString};
use dcso3::{
    airbase::Airbase,
    coalition::Side,
    object::DcsObject,
    warehouse::{self, LiquidType},
    world::World,
    MizLua, String, Vector2,
};
use log::warn;
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::{max, min},
    ops::{AddAssign, SubAssign},
    sync::Arc,
};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Inventory {
    pub stored: u32,
    pub capacity: u32,
}

impl Inventory {
    pub fn percent(&self) -> Option<u8> {
        if self.capacity == 0 {
            None
        } else {
            let stored: f32 = self.stored as f32;
            let capacity: f32 = self.capacity as f32;
            Some(((stored / capacity) * 100.) as u8)
        }
    }

    pub fn reduce(&mut self, percent: f32) -> u32 {
        if self.stored == 0 {
            0
        } else {
            let taken = max(1, (self.stored as f32 * percent) as u32);
            self.stored -= taken;
            taken
        }
    }
}

impl AddAssign<u32> for Inventory {
    fn add_assign(&mut self, rhs: u32) {
        let qty = self.stored + rhs;
        if qty > self.capacity {
            self.stored = self.capacity
        } else {
            self.stored = qty
        }
    }
}

impl SubAssign<u32> for Inventory {
    fn sub_assign(&mut self, rhs: u32) {
        if rhs > self.stored {
            self.stored = 0
        } else {
            self.stored = self.stored - rhs;
        }
    }
}

#[derive(Debug, Clone)]
enum TransferItem {
    Equipment(String),
    Liquid(LiquidType),
}

#[derive(Debug, Clone)]
struct Transfer {
    source: ObjectiveId,
    target: ObjectiveId,
    amount: u32,
    item: TransferItem,
}

impl Transfer {
    fn execute(&self, db: &mut Db) -> Result<()> {
        let src = objective_mut!(db, self.source)?;
        match &self.item {
            TransferItem::Equipment(name) => src.warehouse.equipment[name].stored -= self.amount,
            TransferItem::Liquid(name) => src.warehouse.liquids[name].stored -= self.amount,
        }
        let dst = objective_mut!(db, self.target)?;
        match &self.item {
            TransferItem::Equipment(name) => {
                dst.warehouse
                    .equipment
                    .get_or_default_cow(name.clone())
                    .stored += self.amount
            }
            TransferItem::Liquid(name) => {
                dst.warehouse
                    .liquids
                    .get_or_default_cow(name.clone())
                    .stored += self.amount
            }
        }
        Ok(())
    }
}

struct Needed<'a> {
    oid: &'a ObjectiveId,
    obj: &'a Objective,
    demanded: u32,
    allocated: u32,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Warehouse {
    pub(super) base_equipment: Map<String, Inventory>,
    pub(super) equipment: Map<String, Inventory>,
    pub(super) liquids: Map<LiquidType, Inventory>,
    pub(super) supplier: Option<ObjectiveId>,
    pub(super) destination: Set<ObjectiveId>,
}

fn sync_obj_to_warehouse(
    obj: &Objective,
    production: &Production,
    warehouse: &warehouse::Warehouse,
) -> Result<()> {
    for (item, _) in &production.equipment {
        match obj.warehouse.equipment.get(item) {
            Some(inv) => warehouse
                .set_item(item.clone(), inv.stored)
                .context("setting item")?,
            None => warehouse
                .set_item(item.clone(), 0)
                .context("setting item")?,
        }
    }
    for (name, _) in &production.liquids {
        match obj.warehouse.liquids.get(name) {
            None => warehouse
                .set_liquid_amount(*name, 0)
                .context("setting liquid")?,
            Some(inv) => warehouse
                .set_liquid_amount(*name, inv.stored)
                .context("setting liquid")?,
        }
    }
    Ok(())
}

fn sync_warehouse_to_obj(
    obj: &mut Objective,
    production: &Production,
    warehouse: &warehouse::Warehouse,
) -> Result<()> {
    for (name, _) in &production.equipment {
        if let Some(inv) = obj.warehouse.equipment.get_mut_cow(name) {
            inv.stored = warehouse.get_item_count(name.clone())?;
        }
    }
    for (name, _) in &production.liquids {
        if let Some(inv) = obj.warehouse.liquids.get_mut_cow(&name) {
            inv.stored = warehouse.get_liquid_amount(*name)?;
        }
    }
    Ok(())
}

fn get_supplier<'lua>(lua: MizLua<'lua>, template: String) -> Result<warehouse::Warehouse<'lua>> {
    Airbase::get_by_name(lua, template.clone())
        .with_context(|| format_compact!("getting airbase {}", template))?
        .get_warehouse()
        .context("getting warehouse")
}

impl Db {
    fn init_resource_map(&mut self, lua: MizLua) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            None => return Ok(()),
            Some(w) => w,
        };
        if self.ephemeral.production_by_side.is_empty() {
            let map =
                warehouse::Warehouse::get_resource_map(lua).context("getting resource map")?;
            map.for_each(|name, typ| {
                for side in Side::ALL {
                    let template = match whcfg.supply_source.get(&side) {
                        Some(tmpl) => tmpl,
                        None => continue, // side didn't produce anything, bummer
                    };
                    let w = get_supplier(lua, template.clone())
                        .with_context(|| format_compact!("getting supplier {template}"))?;
                    let production =
                        Arc::make_mut(self.ephemeral.production_by_side.entry(side).or_default());
                    let qty = w
                        .get_item_count(name.clone())
                        .with_context(|| format_compact!("getting {name} from the warehouse"))?;
                    if qty > 0 {
                        production.equipment.insert(
                            name.clone(),
                            Equipment {
                                category: typ.category().context("getting category")?,
                                production: qty,
                            },
                        );
                    }
                    for name in LiquidType::ALL {
                        let qty = w.get_liquid_amount(name).context("getting liquid amount")?;
                        if qty > 0 {
                            production.liquids.insert(name, qty);
                        }
                    }
                }
                Ok(())
            })
            .context("iterating resource map")?
        }
        Ok(())
    }

    pub(super) fn init_farp_warehouse(&mut self, oid: &ObjectiveId) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()),
        };
        let obj = objective_mut!(self, oid)?;
        let production = match self.ephemeral.production_by_side.get(&obj.owner) {
            Some(q) => Arc::clone(q),
            None => return Ok(()),
        };
        for (name, equip) in &production.equipment {
            if !equip.category.is_aircraft() {
                let inv = Inventory {
                    stored: 0,
                    capacity: equip.production * whcfg.airbase_max,
                };
                obj.warehouse.equipment.insert_cow(name.clone(), inv);
            }
        }
        for (name, qty) in &production.liquids {
            let inv = Inventory {
                stored: 0,
                capacity: qty * whcfg.airbase_max,
            };
            obj.warehouse.liquids.insert_cow(*name, inv);
        }
        Ok(())
    }

    pub(super) fn init_warehouses(&mut self, lua: MizLua) -> Result<()> {
        self.init_resource_map(lua)
            .context("initializing resource map")?;
        let cfg = &self.ephemeral.cfg;
        let whcfg = match cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()),
        };
        let oids: SmallVec<[ObjectiveId; 64]> = self
            .persisted
            .objectives
            .into_iter()
            .map(|(oid, _)| *oid)
            .collect();
        for side in Side::ALL {
            let production = match self.ephemeral.production_by_side.get(&side) {
                None => continue,
                Some(q) => Arc::clone(q),
            };
            for (name, equip) in &production.equipment {
                let aircraft = equip.category.is_aircraft();
                for oid in &oids {
                    let obj = objective_mut!(self, oid)?;
                    if obj.owner == side {
                        let hub = self.persisted.logistics_hubs.contains(&oid);
                        let capacity = whcfg.capacity(hub, equip.production);
                        if aircraft {
                            let include = hub
                                || obj
                                    .slots
                                    .into_iter()
                                    .any(|(_, v)| v.typ.as_str() == name.as_str());
                            if !include {
                                continue;
                            }
                        }
                        let inv = obj.warehouse.equipment.get_or_default_cow(name.clone());
                        inv.capacity = capacity;
                        inv.stored = capacity;
                    }
                }
            }
            for (name, qty) in &production.liquids {
                for oid in &oids {
                    let obj = objective_mut!(self, oid)?;
                    if obj.owner == side {
                        let hub = self.persisted.logistics_hubs.contains(&oid);
                        let capacity = whcfg.capacity(hub, *qty);
                        let inv = obj.warehouse.liquids.get_or_default_cow(*name);
                        inv.capacity = capacity;
                        inv.stored = capacity;
                    }
                }
            }
        }
        self.ephemeral.dirty();
        Ok(())
    }

    pub(super) fn setup_warehouses_after_load(&mut self, lua: MizLua) -> Result<()> {
        self.init_resource_map(lua)
            .context("initializing resource map")?;
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()),
        };
        let map = warehouse::Warehouse::get_resource_map(lua).context("getting resource map")?;
        let world = World::singleton(lua).context("getting world")?;
        let mut load_and_sync_airbases = || -> Result<()> {
            world
                .get_airbases()
                .context("getting airbases")?
                .for_each(|airbase| {
                    let airbase = airbase.context("getting airbase")?;
                    if !airbase.is_exist()? {
                        return Ok(()); // can happen when farps get recycled
                    }
                    let pos3 = airbase.get_point().context("getting airbase position")?;
                    let pos = Vector2::new(pos3.x, pos3.z);
                    airbase
                        .auto_capture(false)
                        .context("setting airbase autocapture")?;
                    let oid = self.persisted.objectives.into_iter().find(|(_, obj)| {
                        let radius2 = obj.radius.powi(2);
                        na::distance_squared(&pos.into(), &obj.pos.into()) <= radius2
                    });
                    let w = airbase
                        .get_warehouse()
                        .context("getting airbase warehouse")?;
                    let (oid, obj) = match oid {
                        Some((oid, obj)) => {
                            airbase
                                .set_coalition(obj.owner)
                                .context("setting airbase owner")?;
                            (*oid, obj)
                        }
                        None => {
                            airbase
                                .set_coalition(Side::Neutral)
                                .context("setting airbase owner neutral")?;
                            map.for_each(|name, _| {
                                w.set_item(name, 0).context("zeroing item")?;
                                Ok(())
                            })?;
                            return Ok(());
                        }
                    };
                    self.ephemeral.airbase_by_oid.insert(
                        oid,
                        airbase.object_id().context("getting airbase object_id")?,
                    );
                    let production = match self.ephemeral.production_by_side.get(&obj.owner) {
                        Some(p) => p,
                        None => return Ok(()),
                    };
                    map.for_each(|name, _| {
                        if !production.equipment.contains_key(&name) {
                            w.set_item(name, 0).context("zeroing item")?
                        }
                        Ok(())
                    })?;
                    Ok(())
                })
        };
        load_and_sync_airbases().context("loading and syncing airbases")?;
        let mut adjust_warehouses_for_miz_changes = || -> Result<()> {
            for (oid, obj) in self.persisted.objectives.iter_mut_cow() {
                let mut del_eq: SmallVec<[String; 8]> = smallvec![];
                let mut del_l: SmallVec<[LiquidType; 4]> = smallvec![];
                if let Some(prod) = self.ephemeral.production_by_side.get(&obj.owner) {
                    let hub = self.persisted.logistics_hubs.contains(oid);
                    for (name, _) in &obj.warehouse.equipment {
                        if !prod.equipment.contains_key(name) {
                            del_eq.push(name.clone());
                        }
                    }
                    for name in del_eq {
                        obj.warehouse.equipment.remove_cow(&name);
                    }
                    for (liq, _) in &obj.warehouse.liquids {
                        if !prod.liquids.contains_key(liq) {
                            del_l.push(*liq);
                        }
                    }
                    for liq in del_l {
                        obj.warehouse.liquids.remove_cow(&liq);
                    }
                    for (name, eqip) in &prod.equipment {
                        let capacity = whcfg.capacity(hub, eqip.production);
                        if eqip.category.is_aircraft() {
                            let include = hub
                                || obj
                                    .slots
                                    .into_iter()
                                    .any(|(_, v)| v.typ.as_str() == name.as_str());
                            if !include {
                                continue;
                            }
                        }
                        let inv = obj.warehouse.equipment.get_or_default_cow(name.clone());
                        inv.capacity = capacity;
                    }
                    for (name, prod) in &prod.liquids {
                        let capacity = whcfg.capacity(hub, *prod);
                        let inv = obj.warehouse.liquids.get_or_default_cow(*name);
                        inv.capacity = capacity;
                    }
                }
            }
            Ok(())
        };
        adjust_warehouses_for_miz_changes().context("adjusting warehouses for miz changes")?;
        let mut missing = vec![];
        for (oid, obj) in &self.persisted.objectives {
            if !self.ephemeral.airbase_by_oid.contains_key(oid) {
                missing.push(obj.name.clone());
            }
        }
        if !missing.is_empty() {
            bail!("objectives missing a warehouse {:?}", missing)
        }
        self.deliver_production().context("delivering production")?;
        self.ephemeral.dirty();
        self.sync_warehouses_from_objectives(lua)
            .context("syncing warehouses from objectives")
    }

    pub(super) fn capture_warehouse(&mut self, lua: MizLua, oid: ObjectiveId) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()),
        };
        let obj = objective_mut!(self, oid)?;
        let production = match self.ephemeral.production_by_side.get(&obj.owner) {
            Some(q) => Arc::clone(q),
            None => return Ok(()),
        };
        let w = match self.ephemeral.airbase_by_oid.get(&oid) {
            None => bail!("airbase has no warehouse"),
            Some(aid) => Airbase::get_instance(lua, aid)
                .context("getting airbase")?
                .get_warehouse()
                .context("getting warehouse")?,
        };
        let map = warehouse::Warehouse::get_resource_map(lua).context("getting resource map")?;
        let hub = obj.kind.is_hub();
        map.for_each(|name, _| {
            match production.equipment.get(&name) {
                None => {
                    w.set_item(name.clone(), 0).context("clearing item")?;
                    if let Some(inv) = obj.warehouse.equipment.get_mut_cow(&name) {
                        inv.stored = 0;
                        inv.capacity = 0;
                    }
                }
                Some(equip) => {
                    let inv = obj.warehouse.equipment.get_or_default_cow(name.clone());
                    inv.capacity = whcfg.capacity(hub, equip.production);
                    inv.stored = w.get_item_count(name.clone()).context("getting item")?;
                    if hub {
                        inv.stored = max(inv.stored, equip.production * whcfg.airbase_max);
                        w.set_item(name.clone(), inv.stored)
                            .context("setting item")?;
                    }
                }
            }
            Ok(())
        })?;
        for name in LiquidType::ALL {
            match production.liquids.get(&name) {
                None => {
                    w.set_liquid_amount(name, 0).context("setting liquid")?;
                    if let Some(inv) = obj.warehouse.liquids.get_mut_cow(&name) {
                        inv.stored = 0;
                        inv.capacity = 0;
                    }
                }
                Some(qty) => {
                    let inv = obj.warehouse.liquids.get_or_default_cow(name);
                    inv.capacity = whcfg.capacity(hub, *qty);
                    inv.stored = w.get_liquid_amount(name).context("getting liquid")?;
                }
            }
        }
        Ok(())
    }

    pub(super) fn compute_supplier(&self, obj: &Objective) -> Result<Option<ObjectiveId>> {
        Ok(self
            .persisted
            .logistics_hubs
            .into_iter()
            .fold(Ok::<_, anyhow::Error>(None), |acc, id| {
                let logi = objective!(self, id)?;
                if logi.owner != obj.owner {
                    acc
                } else {
                    let dist = na::distance_squared(&obj.pos.into(), &logi.pos.into());
                    match acc {
                        Err(e) => Err(e),
                        Ok(None) => Ok(Some((dist, *id))),
                        Ok(Some((pdist, _))) if dist < pdist => Ok(Some((dist, *id))),
                        Ok(Some((dist, id))) => Ok(Some((dist, id))),
                    }
                }
            })?
            .map(|(_, id)| id))
    }

    pub fn setup_supply_lines(&mut self) -> Result<()> {
        let mut suppliers: SmallVec<[(ObjectiveId, Option<ObjectiveId>); 64]> = smallvec![];
        for (oid, obj) in &self.persisted.objectives {
            match obj.kind {
                ObjectiveKind::Logistics => (),
                ObjectiveKind::Airbase | ObjectiveKind::Farp { .. } | ObjectiveKind::Fob => {
                    suppliers.push((*oid, self.compute_supplier(obj)?));
                }
            }
        }
        for oid in &self.persisted.logistics_hubs {
            let obj = objective_mut!(self, oid)?;
            obj.warehouse.destination = Set::new();
        }
        for (oid, supplier) in suppliers {
            let obj = objective_mut!(self, oid)?;
            obj.warehouse.supplier = supplier;
            if let Some(id) = supplier {
                let logi = objective_mut!(self, id)?;
                logi.warehouse.destination.insert_cow(oid);
            }
        }
        Ok(())
    }

    pub fn deliver_production(&mut self) -> Result<()> {
        if self.ephemeral.cfg.warehouse.is_none() {
            return Ok(());
        }
        self.setup_supply_lines()
            .context("setting up supply lines")?;
        let mut deliver_produced_supplies = || -> Result<()> {
            for side in Side::ALL {
                let production = match self.ephemeral.production_by_side.get(&side) {
                    Some(e) => Arc::clone(e),
                    None => continue,
                };
                for (name, equip) in &production.equipment {
                    for oid in &self.persisted.logistics_hubs {
                        let logi = objective_mut!(self, oid)?;
                        if logi.owner == side {
                            *logi.warehouse.equipment.get_or_default_cow(name.clone()) +=
                                equip.production;
                        }
                    }
                }
                for (name, qty) in &production.liquids {
                    for oid in &self.persisted.logistics_hubs {
                        let logi = objective_mut!(self, oid)?;
                        if logi.owner == side {
                            *logi.warehouse.liquids.get_or_default_cow(*name) += *qty;
                        }
                    }
                }
            }
            Ok(())
        };
        deliver_produced_supplies().context("delivering produced supplies")?;
        self.deliver_supplies_from_logistics_hubs()
            .context("delivering supplies from logistics hubs")?;
        Ok(())
    }

    pub fn sync_objectives_from_warehouses(&mut self, lua: MizLua) -> Result<()> {
        let oids: SmallVec<[ObjectiveId; 64]> = self
            .persisted
            .objectives
            .into_iter()
            .map(|(oid, _)| *oid)
            .collect();
        for oid in &oids {
            let obj = objective_mut!(self, oid)?;
            let production = match self.ephemeral.production_by_side.get(&obj.owner) {
                Some(p) => Arc::clone(p),
                None => continue,
            };
            let airbase = &maybe!(self.ephemeral.airbase_by_oid, oid, "objective airbase")?;
            let warehouse = Airbase::get_instance(lua, airbase)
                .context("getting airbase")?
                .get_warehouse()
                .context("getting warehouse")?;
            sync_warehouse_to_obj(obj, &production, &warehouse)
                .context("syncing warehouse to objective")?
        }
        self.ephemeral.dirty();
        Ok(())
    }

    pub fn sync_warehouses_from_objectives(&mut self, lua: MizLua) -> Result<()> {
        let oids: SmallVec<[ObjectiveId; 64]> = self
            .persisted
            .objectives
            .into_iter()
            .map(|(oid, _)| *oid)
            .collect();
        for oid in &oids {
            let obj = objective_mut!(self, oid)?;
            let production = match self.ephemeral.production_by_side.get(&obj.owner) {
                Some(p) => p,
                None => continue,
            };
            let airbase = maybe!(self.ephemeral.airbase_by_oid, oid, "objective airbase")
                .with_context(|| format_compact!("getting airbase for objective {}", obj.name))?;
            let warehouse = Airbase::get_instance(lua, airbase)
                .context("getting airbase")?
                .get_warehouse()
                .context("getting warehouse")?;
            sync_obj_to_warehouse(obj, production, &warehouse)
                .context("syncing warehouse from objective")?
        }
        self.ephemeral.dirty();
        Ok(())
    }

    pub fn sync_vehicle_at_obj(
        &mut self,
        lua: MizLua,
        oid: ObjectiveId,
        typ: Vehicle,
    ) -> Result<()> {
        let obj = objective_mut!(self, oid)?;
        let id = maybe!(self.ephemeral.airbase_by_oid, oid, "airbase")?;
        let wh = Airbase::get_instance(lua, id)
            .context("getting airbase")?
            .get_warehouse()
            .context("getting warehouse")?;
        if let Some(inv) = obj.warehouse.equipment.get_mut_cow(&typ.0) {
            inv.stored = wh.get_item_count(typ.0).context("getting item")?;
            self.ephemeral.dirty();
        }
        Ok(())
    }

    pub fn deliver_supplies_from_logistics_hubs(&mut self) -> Result<()> {
        let mut transfers: Vec<Transfer> = vec![];
        for lid in &self.persisted.logistics_hubs {
            let logi = objective!(self, lid)?;
            let mut needed: SmallVec<[Needed; 64]> = logi
                .warehouse
                .destination
                .into_iter()
                .filter_map(|oid| Some((oid, self.persisted.objectives.get(oid)?)))
                .filter(|(_, obj)| logi.owner == obj.owner)
                .map(|(oid, obj)| Needed {
                    oid,
                    obj,
                    demanded: 0,
                    allocated: 0,
                })
                .collect();
            macro_rules! schedule_transfers {
                ($typ:expr, $from:ident, $get:ident) => {
                    for (name, inv) in &logi.warehouse.$from {
                        if inv.stored == 0 {
                            continue;
                        }
                        needed.sort_by(|n0, n1| {
                            let i0 = n0.obj.$get(name);
                            let i1 = n1.obj.$get(name);
                            i0.stored.cmp(&i1.stored)
                        });
                        let mut total_demanded = 0;
                        for n in &mut needed {
                            let inv = n.obj.$get(name);
                            let demanded = if inv.stored <= inv.capacity {
                                inv.capacity - inv.stored
                            } else {
                                0
                            };
                            total_demanded += demanded;
                            n.demanded = demanded;
                            n.allocated = 0;
                        }
                        let mut have = inv.stored;
                        let mut total_filled = 0;
                        while have > 0 && total_filled < total_demanded {
                            for n in &mut needed {
                                if have == 0 {
                                    break;
                                }
                                let allocation = max(1, have >> 3);
                                let amount = min(allocation, n.demanded - n.allocated);
                                n.allocated += amount;
                                total_filled += amount;
                                have -= amount;
                            }
                        }
                        for n in &needed {
                            if n.allocated > 0 {
                                transfers.push(Transfer {
                                    source: *lid,
                                    target: *n.oid,
                                    amount: n.allocated,
                                    item: $typ(name.clone()),
                                })
                            }
                        }
                    }
                };
            }
            schedule_transfers!(TransferItem::Equipment, equipment, get_equipment);
            schedule_transfers!(TransferItem::Liquid, liquids, get_liquids);
        }
        for tr in transfers.drain(..) {
            tr.execute(self)
                .with_context(|| format_compact!("executing transfer {:?}", tr))?
        }
        self.balance_logistics_hubs()
    }

    fn balance_logistics_hubs(&mut self) -> Result<()> {
        struct Needed<'a> {
            oid: &'a ObjectiveId,
            obj: &'a Objective,
            had: u32,
            have: u32,
        }
        for side in Side::ALL {
            let mut transfers: Vec<Transfer> = vec![];
            macro_rules! schedule_transfers {
                ($typ:expr, $from:ident, $get:ident) => {{
                    let mut needed: SmallVec<[Needed; 16]> = self
                        .persisted
                        .logistics_hubs
                        .into_iter()
                        .filter_map(|lid| {
                            let obj = &self.persisted.objectives[lid];
                            if obj.owner != side {
                                None
                            } else {
                                Some(Needed {
                                    oid: lid,
                                    obj,
                                    had: 0,
                                    have: 0,
                                })
                            }
                        })
                        .collect();
                    if needed.len() < 2 {
                        continue;
                    }
                    let items = needed[0].obj.warehouse.$from.clone();
                    for (name, _) in &items {
                        let mean = {
                            let sum: u32 = needed
                                .iter_mut()
                                .map(|n| {
                                    n.have = n.obj.$get(name).stored;
                                    n.had = n.have;
                                    n.had
                                })
                                .sum();
                            sum / needed.len() as u32
                        };
                        if mean >> 2 == 0 {
                            continue;
                        }
                        needed.sort_by(|n0, n1| n0.had.cmp(&n1.had));
                        let mut take = needed.len() - 1;
                        for i in 0..needed.len() {
                            if needed[i].have + 1 >= mean {
                                break;
                            }
                            while needed[i].have + 1 < mean {
                                while take > i && needed[take].have <= mean {
                                    take -= 1;
                                }
                                if take == i {
                                    break;
                                }
                                let need = mean - needed[i].have;
                                let available = needed[take].have - mean;
                                let xfer = min(need, available);
                                needed[i].have += xfer;
                                needed[take].have -= xfer;
                                transfers.push(Transfer {
                                    source: *needed[take].oid,
                                    target: *needed[i].oid,
                                    amount: xfer,
                                    item: $typ(name.clone()),
                                });
                            }
                        }
                    }
                }};
            }
            schedule_transfers!(TransferItem::Equipment, equipment, get_equipment);
            schedule_transfers!(TransferItem::Liquid, liquids, get_liquids);
            for tr in transfers.drain(..) {
                tr.execute(self)
                    .with_context(|| format_compact!("executing transfer {:?}", tr))?
            }
            self.ephemeral.dirty();
        }
        self.update_supply_status()?;
        Ok(())
    }

    fn update_supply_status(&mut self) -> Result<()> {
        let oids: SmallVec<[ObjectiveId; 64]> = self
            .persisted
            .objectives
            .into_iter()
            .map(|(id, _)| *id)
            .collect();
        for oid in oids {
            let obj = objective_mut!(self, oid)?;
            let mut n = 0;
            let mut sum: u32 = 0;
            for (_, inv) in &obj.warehouse.equipment {
                if let Some(pct) = inv.percent() {
                    sum += pct as u32;
                    n += 1;
                }
            }
            obj.supply = if n == 0 { 0 } else { (sum / n) as u8 };
            n = 0;
            sum = 0;
            for (_, inv) in &obj.warehouse.liquids {
                if let Some(pct) = inv.percent() {
                    sum += pct as u32;
                    n += 1;
                }
            }
            obj.fuel = if n == 0 { 0 } else { (sum / n) as u8 };
        }
        self.ephemeral.dirty();
        Ok(())
    }

    pub fn sync_warehouse_to_objective<'lua>(
        &mut self,
        lua: MizLua<'lua>,
        oid: ObjectiveId,
    ) -> Result<(&mut Objective, warehouse::Warehouse<'lua>)> {
        let obj = objective_mut!(self, oid)?;
        let airbase = self
            .ephemeral
            .airbase_by_oid
            .get(&oid)
            .ok_or_else(|| anyhow!("no logistics for objective {}", obj.name))?;
        let warehouse = Airbase::get_instance(lua, &airbase)
            .context("getting airbase")?
            .get_warehouse()
            .context("getting warehouse")?;
        let production = match self.ephemeral.production_by_side.get(&obj.owner) {
            None => return Ok((obj, warehouse)),
            Some(p) => p,
        };
        sync_warehouse_to_obj(obj, production, &warehouse)
            .context("syncing warehouse to objective")?;
        Ok((obj, warehouse))
    }

    pub fn sync_objective_to_warehouse<'lua>(
        &mut self,
        lua: MizLua<'lua>,
        oid: ObjectiveId,
    ) -> Result<(&mut Objective, warehouse::Warehouse<'lua>)> {
        let obj = objective_mut!(self, oid)?;
        let airbase = self
            .ephemeral
            .airbase_by_oid
            .get(&oid)
            .ok_or_else(|| anyhow!("no logistics for objective {}", obj.name))?;
        let warehouse = Airbase::get_instance(lua, &airbase)
            .context("getting airbase")?
            .get_warehouse()
            .context("getting warehouse")?;
        let production = match self.ephemeral.production_by_side.get(&obj.owner) {
            None => return Ok((obj, warehouse)),
            Some(p) => p,
        };
        sync_obj_to_warehouse(obj, production, &warehouse)
            .context("syncing warehouse to objective")?;
        Ok((obj, warehouse))
    }

    pub fn transfer_supplies(
        &mut self,
        lua: MizLua,
        from: ObjectiveId,
        to: ObjectiveId,
    ) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(whcfg) => whcfg,
            None => return Ok(()),
        };
        let size = whcfg.supply_transfer_size as f32 / 100.;
        let side = objective!(self, from)?.owner;
        if side != objective!(self, to)?.owner {
            bail!("can't transfer supply from an enemy objective")
        }
        let mut transfers: SmallVec<[Transfer; 128]> = smallvec![];
        let (_, from_wh) = self
            .sync_warehouse_to_objective(lua, from)
            .context("syncing from objective")?;
        let (_, to_wh) = self
            .sync_warehouse_to_objective(lua, to)
            .context("syncing to objective")?;
        let production = match self.ephemeral.production_by_side.get(&side) {
            Some(p) => Arc::clone(p),
            None => return Ok(()),
        };
        let from_obj = objective!(self, from)?;
        let to_obj = objective!(self, to)?;
        macro_rules! compute {
            ($src:ident, $typ:ident) => {
                for (name, inv) in &from_obj.warehouse.$src {
                    if inv.stored > 0 {
                        let needed = match to_obj.warehouse.$src.get(name) {
                            None => 0,
                            Some(inv) => {
                                if inv.capacity >= inv.stored {
                                    inv.capacity - inv.stored
                                } else {
                                    0
                                }
                            }
                        };
                        let amount = min(needed, max(1, (inv.stored as f32 * size) as u32));
                        transfers.push(Transfer {
                            amount,
                            source: from,
                            target: to,
                            item: TransferItem::$typ(name.clone()),
                        });
                    }
                }
            };
        }
        compute!(equipment, Equipment);
        compute!(liquids, Liquid);
        for tr in transfers {
            tr.execute(self)?
        }
        sync_obj_to_warehouse(objective!(self, from)?, &production, &from_wh)?;
        sync_obj_to_warehouse(objective!(self, to)?, &production, &to_wh)?;
        self.update_supply_status()
            .context("updating supply status")?;
        Ok(())
    }

    pub fn admin_transfer_supplies(&mut self, lua: MizLua, from: &str, to: &str) -> Result<()> {
        let from = self
            .persisted
            .objectives_by_name
            .get(from)
            .ok_or_else(|| anyhow!("not such objective {from}"))?;
        let to = self
            .persisted
            .objectives_by_name
            .get(to)
            .ok_or_else(|| anyhow!("no such objective {to}"))?;
        self.transfer_supplies(lua, *from, *to)
    }

    pub fn admin_reduce_inventory(&mut self, lua: MizLua, name: &str, amount: u8) -> Result<()> {
        let oid = self
            .persisted
            .objectives_by_name
            .get(name)
            .map(|oid| *oid)
            .ok_or_else(|| anyhow!("no such objective {name}"))?;
        if amount > 100 {
            bail!("enter a percentage")
        }
        let percent = amount as f32 / 100.;
        let production = match self
            .ephemeral
            .production_by_side
            .get(&objective!(self, oid)?.owner)
        {
            Some(p) => Arc::clone(p),
            None => return Ok(()),
        };
        let (obj, warehouse) = self
            .sync_warehouse_to_objective(lua, oid)
            .with_context(|| format_compact!("syncing warehouses to {name}"))?;
        for name in production.equipment.keys() {
            if let Some(inv) = obj.warehouse.equipment.get_mut_cow(name) {
                inv.reduce(percent);
            }
        }
        for liq in production.liquids.keys() {
            if let Some(inv) = obj.warehouse.liquids.get_mut_cow(&liq) {
                inv.reduce(percent);
            }
        }
        sync_obj_to_warehouse(obj, &production, &warehouse).context("syncing from warehouse")?;
        self.update_supply_status()
            .context("updating supply status")?;
        self.ephemeral.dirty();
        Ok(())
    }

    pub fn admin_log_inventory(
        &mut self,
        lua: MizLua,
        kind: WarehouseKind,
        name: &str,
    ) -> Result<()> {
        use std::fmt::Write;
        let oid = self
            .persisted
            .objectives_by_name
            .get(name)
            .map(|oid| *oid)
            .ok_or_else(|| anyhow!("no such objective {name}"))?;
        match kind {
            WarehouseKind::DCS => {
                let abid = self
                    .ephemeral
                    .airbase_by_oid
                    .get(&oid)
                    .ok_or_else(|| anyhow!("no airbase for {oid}"))?;
                let wh = Airbase::get_instance(lua, &abid)
                    .context("getting airbase")?
                    .get_warehouse()
                    .context("getting warehouse")?;
                let map =
                    warehouse::Warehouse::get_resource_map(lua).context("getting resource map")?;
                let mut msg = CompactString::new("");
                map.for_each(|name, _| {
                    let qty = wh
                        .get_item_count(name.clone())
                        .with_context(|| format_compact!("getting {name} count from warehouse"))?;
                    if qty > 0 {
                        write!(msg, "{name}, {qty}\n")?
                    }
                    Ok(())
                })?;
                for name in LiquidType::ALL {
                    let qty = wh.get_liquid_amount(name).with_context(|| {
                        format_compact!("getting liquid {:?} from warehouse", name)
                    })?;
                    if qty > 0 {
                        write!(msg, "{:?}, {qty}\n", name)?
                    }
                }
                warn!("{msg}")
            }
            WarehouseKind::Objective => {
                let obj = objective!(self, oid)?;
                let mut msg = CompactString::new("");
                for (name, inv) in &obj.warehouse.equipment {
                    write!(msg, "{name}, {}/{}\n", inv.stored, inv.capacity)?
                }
                for (name, inv) in &obj.warehouse.liquids {
                    write!(msg, "{:?}, {}/{}\n", name, inv.stored, inv.capacity)?
                }
                warn!("{msg}")
            }
        }
        Ok(())
    }
}
