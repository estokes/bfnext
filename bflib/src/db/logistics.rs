use super::{objective::{ObjectiveId, Objective}, Db, Map};
use crate::{db::ephemeral::ObjLogi, objective_mut};
use anyhow::{Result, bail};
use dcso3::{
    coalition::Side, object::DcsObject, warehouse::{self, LiquidType}, world::World, MizLua, String,
    Vector2,
};
use serde_derive::{Deserialize, Serialize};
use smallvec::smallvec;

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Inventory<N> {
    stored: N,
    capacity: N,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Warehouse {
    base_equipment: Map<String, Inventory<u32>>,
    equipment: Map<String, Inventory<u32>>,
    liquids: Map<LiquidType, Inventory<f32>>,
    supplier: Option<ObjectiveId>,
}

fn sync_from_obj(obj: &Objective, warehouse: warehouse::Warehouse) -> Result<()> {
    let inventory = warehouse.get_inventory(None)?;
    let weapons = inventory.weapons()?;
    let aircraft = inventory.aircraft()?;
    let liquids = inventory.liquids()?;
    if weapons.is_empty() || aircraft.is_empty() || liquids.is_empty() {
        bail!("objective {} has warehouse categories set to unlimited", obj.name)
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
        None => warehouse.set_liquid_amount(name, 0.)
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
    pub(super) fn init_warehouses(&mut self, lua: MizLua) -> Result<()> {
        let whcfg = match self.ephemeral.cfg.warehouse.as_ref() {
            Some(cfg) => cfg,
            None => return Ok(()), // warehouse system disabled
        };
        let world = World::singleton(lua)?;
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
                },
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
        for side in [Side::Red, Side::Blue, Side::Neutral] {
            let template = match whcfg.supply_source.get(&side) {
                Some(tmpl) => tmpl,
                None => continue,
            };
        }
        unimplemented!()
    }
}
