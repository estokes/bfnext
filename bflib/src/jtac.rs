use crate::{cfg::UnitTag, db::{UnitId, Db}};
use anyhow::Result;
use dcso3::{coalition::Side, Vector2, spot::ClassSpot, object::DcsOid, MizLua};
use enumflags2::BitFlags;
use fxhash::FxHashMap;

#[derive(Debug, Clone, Default)]
struct Jtac {
    contacts: Vec<(BitFlags<UnitTag>, UnitId)>,
    filter: BitFlags<UnitTag>,
    priority: Vec<UnitTag>,
    target: Option<(DcsOid<ClassSpot>, UnitId)>,
    autolase: bool,
    smoketarget: bool,
    code: u16,
    pos: Vector2,
}

#[derive(Debug, Clone, Default)]
struct Jtacs {
    jtacs: FxHashMap<Side, FxHashMap<UnitId, Jtac>>,
}

impl Jtacs {
    pub fn update_contacts(&mut self, lua: MizLua, db: &Db) -> Result<()> {
        unimplemented!()
    }
}
