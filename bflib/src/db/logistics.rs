use super::{
    objective::{Objective, ObjectiveId},
    Db, Map, Set,
};
use crate::{
    db::{ephemeral::ObjLogi, objective::ObjectiveKind},
    objective_mut,
};
use anyhow::{anyhow, bail, Result};
use dcso3::{
    coalition::Side,
    object::DcsObject,
    warehouse::{self, LiquidType},
    world::World,
    MizLua, String, Vector2,
};
use fxhash::FxHashMap;
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};
use std::ops::{Add, AddAssign, Sub, SubAssign};

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

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Warehouse {
    base_equipment: Map<String, Inventory<u32>>,
    equipment: Map<String, Inventory<u32>>,
    liquids: Map<LiquidType, Inventory<f32>>,
    supplier: Option<ObjectiveId>,
    destination: Set<ObjectiveId>,
}

fn sync_from_obj(obj: &Objective, warehouse: warehouse::Warehouse) -> Result<()> {
    let inventory = warehouse.get_inventory(None)?;
    let weapons = inventory.weapons()?;
    let aircraft = inventory.aircraft()?;
    let liquids = inventory.liquids()?;
    if weapons.is_empty() || aircraft.is_empty() || liquids.is_empty() {
        bail!(
            "objective {} has warehouse categories set to unlimited",
            obj.name
        )
    }
    weapons.for_each(|name, _| match obj.warehouse.equipment.get(name.as_str()) {
        Some(_) => Ok(()),
        None => warehouse.set_item(name, 0),
    })?;
    aircraft.for_each(|name, _| match obj.warehouse.equipment.get(name.as_str()) {
        Some(_) => Ok(()),
        None => warehouse.set_item(name, 0),
    })?;
    liquids.for_each(|name, _| match obj.warehouse.liquids.get(&name) {
        Some(_) => Ok(()),
        None => warehouse.set_liquid_amount(name, 0.),
    })?;
    for (name, inv) in &obj.warehouse.equipment {
        warehouse.set_item(name.clone(), inv.stored)?
    }
    for (name, inv) in &obj.warehouse.liquids {
        warehouse.set_liquid_amount(*name, inv.stored)?
    }
    Ok(())
}

impl Db {
    pub(super) fn setup_warehouses_after_load(&mut self, lua: MizLua) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()), // warehouse system disabled
        };
        let world = World::singleton(lua)?;
        let mut load_and_sync_airbases = || -> Result<()> {
            for airbase in world.get_airbases()? {
                let airbase = airbase?;
                let pos3 = airbase.get_point()?;
                let pos = Vector2::new(pos3.x, pos3.z);
                airbase.auto_capture(false)?;
                let oid = self.persisted.objectives.into_iter().find(|(_, obj)| {
                    let radius2 = obj.radius.powi(2);
                    na::distance_squared(&pos.into(), &obj.pos.into()) <= radius2
                });
                let (oid, obj) = match oid {
                    Some((oid, obj)) => {
                        airbase.set_coalition(obj.owner)?;
                        (*oid, obj)
                    }
                    None => {
                        airbase.set_coalition(Side::Neutral)?;
                        continue;
                    }
                };
                let warehouse = airbase.get_warehouse()?;
                self.ephemeral.logistics_by_oid.insert(
                    oid,
                    ObjLogi {
                        airbase: airbase.object_id()?,
                        warehouse: warehouse.object_id()?,
                    },
                );
                sync_from_obj(obj, warehouse)?;
            }
            Ok(())
        };
        load_and_sync_airbases()?;
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
        setup_supply_lines()?;
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
                let w = warehouse::Warehouse::get_by_name(lua, template.clone())?
                    .get_inventory(None)?;
                w.weapons()?.for_each(|n, q| dlvr!(equipment, n, q))?;
                w.aircraft()?.for_each(|n, q| dlvr!(equipment, n, q))?;
                w.liquids()?.for_each(|n, q| dlvr!(liquids, n, q))?;
            }
            unimplemented!()
        };
        deliver_produced_supplies()?;
        self.ephemeral.dirty();
        self.deliver_supplies_from_logistics_hubs()
    }

    pub fn deliver_supplies_from_logistics_hubs(&mut self) -> Result<()> {
        let mut equipment: FxHashMap<Side, SmallVec<[String; 128]>> = FxHashMap::default();
        let mut liquids: FxHashMap<Side, SmallVec<[LiquidType; 8]>> = FxHashMap::default();
        for lid in &self.persisted.logistics_hubs {
            let logi = objective_mut!(self, lid)?;
            let equipment = equipment.entry(logi.owner).or_insert_with(|| {
                logi.warehouse
                    .equipment
                    .into_iter()
                    .map(|(id, _)| id.clone())
                    .collect::<SmallVec<_>>()
            });
            let liquids = liquids.entry(logi.owner).or_insert_with(|| {
                logi.warehouse
                    .liquids
                    .into_iter()
                    .map(|(id, _)| *id)
                    .collect::<SmallVec<_>>()
            });
            let mut needed: SmallVec<[ObjectiveId; 64]> = smallvec![];
            for name in equipment {

            }
        }
        unimplemented!()
    }
}
