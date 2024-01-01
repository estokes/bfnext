use crate::{
    cfg::UnitTag,
    db::{Db, GroupId, SpawnedUnit, UnitId},
};
use anyhow::Result;
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    land::Land,
    object::{DcsObject, DcsOid},
    spot::{ClassSpot, Spot},
    LuaVec3, MizLua, Vector2, Vector3,
};
use enumflags2::BitFlags;
use fxhash::{FxHashMap, FxHashSet};
use indexmap::IndexMap;

#[derive(Debug, Clone, Copy, Default)]
struct Contact {
    pos: Vector3,
    tags: BitFlags<UnitTag>,
    last_move: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Default)]
struct Jtac {
    contacts: IndexMap<UnitId, Contact>,
    filter: BitFlags<UnitTag>,
    priority: Vec<UnitTag>,
    target: Option<(DcsOid<ClassSpot>, UnitId)>,
    autolase: bool,
    smoketarget: bool,
    code: u16,
    pos: Vector2,
}

impl Jtac {
    fn add_contact(&mut self, unit: &SpawnedUnit) {
        let ct = self.contacts.entry(unit.id).or_default();
        ct.pos = unit.position.p.0;
        ct.last_move = unit.moved;
        ct.tags = unit.tags;
    }

    fn remove_target(&mut self, lua: MizLua) -> Result<()> {
        if let Some((id, _)) = &self.target.take() {
            let spot = Spot::get_instance(lua, id)?;
            spot.destroy()?;
        }
        Ok(())
    }

    fn remove_contact(&mut self, lua: MizLua, uid: &UnitId) -> Result<()> {
        if let Some(_) = self.contacts.swap_remove(uid) {
            if let Some((id, tuid)) = &self.target {
                if tuid == uid {
                    self.remove_target(lua)?
                }
            }
        }
        Ok(())
    }

    fn sort_contacts(&mut self, plist: &[BitFlags<UnitTag>]) -> Result<()> {
        fn priority(plist: &[BitFlags<UnitTag>], tags: BitFlags<UnitTag>) -> usize {
            plist
                .iter()
                .enumerate()
                .find(|(_, p)| tags.contains(**p))
                .map(|(i, _)| i)
                .unwrap_or(plist.len())
        }
        self.contacts.sort_by(|_, ct0, _, ct1| {
            let p0 = priority(plist, ct0.tags);
            let p1 = priority(plist, ct1.tags);
            p0.cmp(&p1)
        });
        unimplemented!()
    }
}

#[derive(Debug, Clone, Default)]
pub struct Jtacs {
    jtacs: FxHashMap<Side, FxHashMap<GroupId, Jtac>>,
}

impl Jtacs {
    pub fn update_contacts(&mut self, lua: MizLua, db: &Db) -> Result<()> {
        let land = Land::singleton(lua)?;
        let mut saw = FxHashSet::default();
        for (unit, _) in db.instanced_units() {
            for (pos, group, ifo) in db.jtacs() {
                saw.insert(group.id);
                let range = (ifo.range as f64).powi(2);
                let jtac = self
                    .jtacs
                    .entry(group.side)
                    .or_default()
                    .entry(group.id)
                    .or_default();
                if let Some(ct) = jtac.contacts.get(&unit.id) {
                    if unit.moved == ct.last_move {
                        continue;
                    }
                }
                let dist = na::distance_squared(&pos.into(), &unit.position.p.0.into());
                if dist <= range && land.is_visible(LuaVec3(pos), unit.position.p)? {
                    jtac.add_contact(unit)
                } else {
                    jtac.remove_contact(lua, &unit.id)?
                }
            }
        }
        for j in self.jtacs.values_mut() {
            j.retain(|uid, jt| {
                saw.contains(uid) || {
                    let _ = jt.remove_target(lua);
                    false
                }
            })
        }
        Ok(())
    }
}
