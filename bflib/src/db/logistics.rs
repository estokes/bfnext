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
    objective::{Objective, ObjectiveId},
    Db, Map, Set,
};
use crate::{
    db::{ephemeral::ObjLogi, objective::ObjectiveKind},
    maybe, objective, objective_mut,
};
use anyhow::{anyhow, bail, Context, Result};
use compact_str::format_compact;
use dcso3::{
    airbase::Airbase,
    coalition::Side,
    object::DcsObject,
    warehouse::{self, LiquidType},
    world::World,
    MizLua, String, Vector2,
};
use log::debug;
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::{
    cmp::{max, min},
    ops::{Add, AddAssign, Sub, SubAssign},
};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Inventory<N> {
    stored: N,
    capacity: N,
}

impl<N> AddAssign<N> for Inventory<N>
where
    N: Add<Output = N> + PartialOrd + Copy,
{
    fn add_assign(&mut self, rhs: N) {
        let qty = self.stored + rhs;
        if qty > self.capacity {
            self.stored = self.capacity
        } else {
            self.stored = qty
        }
    }
}

impl<N> SubAssign<N> for Inventory<N>
where
    N: Sub<Output = N> + PartialOrd + Copy + Default,
{
    fn sub_assign(&mut self, rhs: N) {
        if rhs > self.stored {
            self.stored = N::default()
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
            TransferItem::Equipment(name) => dst.warehouse.equipment[name].stored += self.amount,
            TransferItem::Liquid(name) => dst.warehouse.liquids[name].stored += self.amount,
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
    pub(super) base_equipment: Map<String, Inventory<u32>>,
    pub(super) equipment: Map<String, Inventory<u32>>,
    pub(super) liquids: Map<LiquidType, Inventory<u32>>,
    pub(super) supplier: Option<ObjectiveId>,
    pub(super) destination: Set<ObjectiveId>,
}

fn sync_from_obj(obj: &Objective, warehouse: &warehouse::Warehouse) -> Result<()> {
    let inventory = warehouse.get_inventory(None).context("getting inventory")?;
    let weapons = inventory.weapons().context("getting weapons")?;
    let aircraft = inventory.aircraft().context("getting aircraft")?;
    let liquids = inventory.liquids().context("getting liquids")?;
    macro_rules! zero {
        ($src:ident, $dst:ident, $set:ident) => {
            $src.for_each(|name, _| match obj.warehouse.$dst.get(&name) {
                Some(_) => Ok(()),
                None => warehouse.$set(name, 0),
            })
            .context("zeroing")?;
        };
    }
    zero!(weapons, equipment, set_item);
    zero!(aircraft, equipment, set_item);
    zero!(liquids, liquids, set_liquid_amount);
    for (name, inv) in &obj.warehouse.equipment {
        warehouse
            .set_item(name.clone(), inv.stored)
            .context("setting item")?
    }
    for (name, inv) in &obj.warehouse.liquids {
        warehouse
            .set_liquid_amount(*name, inv.stored)
            .context("setting liquid")?
    }
    debug!("{} warehouse: {:?}", obj.name, warehouse.get_inventory(None)?);
    Ok(())
}

fn sync_to_obj(obj: &mut Objective, warehouse: &warehouse::Warehouse) -> Result<()> {
    let inventory = warehouse.get_inventory(None).context("getting inventory")?;
    let weapons = inventory.weapons().context("getting weapons")?;
    let aircraft = inventory.aircraft().context("getting aircraft")?;
    let liquids = inventory.liquids().context("getting liquids")?;
    macro_rules! sync {
        ($src:ident, $dst:ident) => {
            $src.for_each(|name, qty| {
                let inv = obj.warehouse.$dst.get_or_default_cow(name);
                inv.stored = qty;
                Ok(())
            })
            .context("syncing")?;
        };
    }
    sync!(weapons, equipment);
    sync!(aircraft, equipment);
    sync!(liquids, liquids);
    Ok(())
}

impl Db {
    pub(super) fn init_warehouses(&mut self, lua: MizLua) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()),
        };
        for side in [Side::Red, Side::Blue, Side::Neutral] {
            let oids: SmallVec<[ObjectiveId; 64]> = self
                .persisted
                .objectives
                .into_iter()
                .filter_map(|(oid, obj)| if obj.owner == side { Some(*oid) } else { None })
                .collect();
            let template = match whcfg.supply_source.get(&side) {
                Some(tmpl) => tmpl,
                None => continue, // side didn't produce anything, bummer
            };
            let w = Airbase::get_by_name(lua, template.clone())
                .with_context(|| format_compact!("getting airbase {}", template))?
                .get_warehouse()
                .context("getting warehouse")?
                .get_inventory(None)
                .context("getting inventory")?;
            macro_rules! dist {
                ($src:ident, $dst:ident) => {{
                    w.$src()
                        .with_context(|| format_compact!("getting {}", stringify!($src)))?
                        .for_each(|name, qty| {
                            for oid in &oids {
                                let hub = self.persisted.logistics_hubs.contains(oid);
                                let obj = objective_mut!(self, oid)?;
                                let capacity = qty
                                    * if hub {
                                        whcfg.hub_max
                                    } else {
                                        whcfg.airbase_max
                                    };
                                let inv = Inventory {
                                    stored: capacity,
                                    capacity,
                                };
                                obj.warehouse.$dst.insert_cow(name.clone(), inv);
                            }
                            Ok(())
                        })
                        .context("distributing")?;
                }};
            }
            dist!(weapons, equipment);
            dist!(aircraft, equipment);
            dist!(liquids, liquids);
        }
        self.ephemeral.dirty();
        Ok(())
    }

    pub(super) fn setup_warehouses_after_load(&mut self, lua: MizLua) -> Result<()> {
        if self.ephemeral.cfg.warehouse.is_none() {
            return Ok(()); // warehouse system disabled
        }
        let world = World::singleton(lua).context("getting world")?;
        let mut load_and_sync_airbases = || -> Result<()> {
            for airbase in world.get_airbases().context("getting airbases")? {
                let airbase = airbase.context("getting airbase")?;
                let pos3 = airbase.get_point().context("getting airbase position")?;
                let pos = Vector2::new(pos3.x, pos3.z);
                airbase
                    .auto_capture(false)
                    .context("setting airbase autocapture")?;
                let oid = self.persisted.objectives.into_iter().find(|(_, obj)| {
                    let radius2 = obj.radius.powi(2);
                    na::distance_squared(&pos.into(), &obj.pos.into()) <= radius2
                });
                let (oid, _) = match oid {
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
                        continue;
                    }
                };
                self.ephemeral.logistics_by_oid.insert(
                    oid,
                    ObjLogi {
                        airbase: airbase.object_id().context("getting airbase object_id")?,
                    },
                );
            }
            let mut missing = vec![];
            for (oid, obj) in &self.persisted.objectives {
                if !self.ephemeral.logistics_by_oid.contains_key(oid) {
                    missing.push(obj.name.clone());
                }
            }
            if !missing.is_empty() {
                bail!("objectives missing a warehouse {:?}", missing)
            }
            Ok(())
        };
        load_and_sync_airbases().context("loading and syncing airbases")?;
        self.deliver_production(lua)
            .context("delivering production")?;
        self.ephemeral.dirty();
        self.sync_warehouses_from_objectives(lua)
            .context("syncing warehouses from objectives")
    }

    pub fn deliver_production(&mut self, lua: MizLua) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()), // warehouse system disabled
        };
        let mut setup_supply_lines = || -> Result<()> {
            let mut suppliers: SmallVec<[(ObjectiveId, Option<ObjectiveId>); 64]> = smallvec![];
            for (oid, obj) in &self.persisted.objectives {
                match obj.kind {
                    ObjectiveKind::Logistics => (),
                    ObjectiveKind::Airbase | ObjectiveKind::Farp(_) | ObjectiveKind::Fob => {
                        let supplier = self
                            .persisted
                            .logistics_hubs
                            .into_iter()
                            .fold(None, |acc, id| {
                                let logi = &self.persisted.objectives[id];
                                if logi.owner != obj.owner {
                                    acc
                                } else {
                                    let dist =
                                        na::distance_squared(&obj.pos.into(), &logi.pos.into());
                                    match acc {
                                        None => Some((dist, *id)),
                                        Some((pdist, _)) if dist < pdist => Some((dist, *id)),
                                        Some((dist, id)) => Some((dist, id)),
                                    }
                                }
                            })
                            .map(|(_, id)| id);
                        suppliers.push((*oid, supplier));
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
        };
        setup_supply_lines().context("setting up supplylines")?;
        let mut deliver_produced_supplies = || -> Result<()> {
            for side in [Side::Red, Side::Blue, Side::Neutral] {
                macro_rules! dlvr {
                    ($dest:ident, $name:expr, $qty:expr) => {{
                        for oid in &self.persisted.logistics_hubs {
                            let logi = objective_mut!(self, oid)?;
                            if logi.owner == side {
                                *logi.warehouse.$dest.get_or_default_cow($name.clone()) += $qty;
                            }
                        }
                        Ok(())
                    }};
                }
                let template = match whcfg.supply_source.get(&side) {
                    Some(tmpl) => tmpl,
                    None => continue, // side didn't produce anything, bummer
                };
                let w = Airbase::get_by_name(lua, template.clone())
                    .with_context(|| format_compact!("getting airbase {}", template))?
                    .get_warehouse()
                    .context("getting warehouse")?
                    .get_inventory(None)
                    .context("getting inventory")?;
                w.weapons()
                    .context("getting weapons")?
                    .for_each(|n, q| dlvr!(equipment, n, q))
                    .context("delivering weapons")?;
                w.aircraft()
                    .context("getting aircraft")?
                    .for_each(|n, q| dlvr!(equipment, n, q))
                    .context("delivering aircraft")?;
                w.liquids()
                    .context("getting liquids")?
                    .for_each(|n, q| dlvr!(liquids, n, q))
                    .context("delivering liquids")?
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
            let airbase = &maybe!(self.ephemeral.logistics_by_oid, oid, "airbase")?.airbase;
            let warehouse = Airbase::get_instance(lua, airbase)
                .context("getting airbase")?
                .get_warehouse()
                .context("getting warehouse")?;
            sync_to_obj(obj, &warehouse).context("syncing warehouse to objective")?
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
            let airbase = &maybe!(self.ephemeral.logistics_by_oid, oid, "airbase")?.airbase;
            let warehouse = Airbase::get_instance(lua, airbase)
                .context("getting airbase")?
                .get_warehouse()
                .context("getting warehouse")?;
            sync_from_obj(obj, &warehouse).context("syncing warehouse from objective")?
        }
        self.ephemeral.dirty();
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
                            let demanded = inv.capacity - inv.stored;
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
        for side in [Side::Blue, Side::Red, Side::Neutral] {
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
        Ok(())
    }
}
