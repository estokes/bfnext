use crate::{cfg::UnitTag, db::UnitId};
use dcso3::{coalition::Side, Vector2};
use enumflags2::BitFlags;
use fxhash::FxHashMap;

#[derive(Debug, Clone, Default)]
struct Jtac {
    contacts: Vec<(BitFlags<UnitTag>, UnitId)>,
    filter: BitFlags<UnitTag>,
    priority: Vec<UnitTag>,
    target: Option<UnitId>,
    autolase: bool,
    smoketarget: bool,
    code: u16,
    pos: Vector2,
}

struct Jtacs {
    jtacs: FxHashMap<Side, FxHashMap<UnitId, Jtac>>,
}
