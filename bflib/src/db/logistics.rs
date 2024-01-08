use dcso3::{warehouse::LiquidType, String};
use serde_derive::{Serialize, Deserialize};
use super::{Db, Map, objective::ObjectiveId};

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
pub struct Inventory<N> {
    stored: N,
    capacity: N,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Warehouse {
    base_equipment: Map<String, Inventory<u16>>,
    equipment: Map<String, Inventory<u16>>,
    liquids: Map<LiquidType, Inventory<u16>>,
    supplier: Option<ObjectiveId>,
}

impl Db {

}