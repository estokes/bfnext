use crate::{
    cfg::UnitTag,
    db::{Db, GroupId, SpawnedUnit, UnitId},
};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    land::Land,
    mission_commands::{CoalitionSubMenu, MissionCommands},
    object::{DcsObject, DcsOid},
    spot::{ClassSpot, Spot},
    unit::{ClassUnit, Unit},
    LuaVec3, MizLua, Vector3,
};
use enumflags2::BitFlags;
use fxhash::FxHashMap;
use indexmap::IndexMap;
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone, Copy, Default)]
struct Contact {
    pos: Vector3,
    tags: BitFlags<UnitTag>,
    last_move: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct Jtac {
    contacts: IndexMap<UnitId, Contact>,
    id: DcsOid<ClassUnit>,
    filter: BitFlags<UnitTag>,
    priority: Vec<BitFlags<UnitTag>>,
    target: Option<(DcsOid<ClassSpot>, UnitId)>,
    autolase: bool,
    smoketarget: bool,
    code: u16,
}

impl Jtac {
    fn new(id: DcsOid<ClassUnit>, priority: Vec<BitFlags<UnitTag>>) -> Self {
        Self {
            contacts: IndexMap::default(),
            id,
            filter: BitFlags::default(),
            priority,
            target: None,
            autolase: true,
            smoketarget: false,
            code: 1688,
        }
    }

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

    fn set_target(&mut self, lua: MizLua, i: usize) -> Result<Option<UnitId>> {
        let (uid, ct) = self
            .contacts
            .get_index(i)
            .ok_or_else(|| anyhow!("no such target"))?;
        let uid = *uid;
        let pos = ct.pos;
        match &self.target {
            Some((_, tuid)) if tuid == &uid => Ok(None),
            Some(_) | None => {
                self.remove_target(lua)?;
                let jt = Unit::get_instance(lua, &self.id)?;
                let spot = Spot::create_laser(
                    lua,
                    jt.as_object()?,
                    Some(LuaVec3(Vector3::new(0., 1., 0.))),
                    LuaVec3(pos),
                    self.code,
                )?;
                self.target = Some((spot.object_id()?, uid));
                Ok(Some(uid))
            }
        }
    }

    fn shift(&mut self, lua: MizLua) -> Result<Option<UnitId>> {
        let i = match &self.target {
            None => 0,
            Some((_, uid)) => match self.contacts.get_index_of(uid) {
                None => 0,
                Some(i) => {
                    if i < self.contacts.len() - 1 {
                        i + 1
                    } else {
                        0
                    }
                }
            },
        };
        self.set_target(lua, i)
    }

    fn remove_contact(&mut self, lua: MizLua, uid: &UnitId) -> Result<()> {
        if let Some(_) = self.contacts.swap_remove(uid) {
            if let Some((_, tuid)) = &self.target {
                if tuid == uid {
                    self.remove_target(lua)?
                }
            }
        }
        Ok(())
    }

    fn sort_contacts(&mut self, lua: MizLua) -> Result<Option<UnitId>> {
        let plist = &self.priority;
        let priority = |tags: BitFlags<UnitTag>| {
            plist
                .iter()
                .enumerate()
                .find(|(_, p)| tags.contains(**p))
                .map(|(i, _)| i)
                .unwrap_or(plist.len())
        };
        self.contacts
            .sort_by(|_, ct0, _, ct1| priority(ct0.tags).cmp(&priority(ct1.tags)));
        if self.autolase && !self.contacts.is_empty() {
            return self.set_target(lua, 0);
        }
        Ok(None)
    }
}

fn jtac_msg(db: &mut Db, gid: GroupId, uid: UnitId) -> Result<CompactString> {
    let unit_typ = db.unit(&uid)?.typ.clone();
    let jtac_pos = db.group_center(&gid)?;
    let (dist, heading, obj) = db.objective_near_point(jtac_pos);
    Ok(format_compact!(
        "JTAC {gid} bearing {} for {} from {} now lasing {unit_typ}",
        dist as u32,
        heading as u32,
        obj.name()
    ))
}

#[derive(Debug, Clone, Default)]
pub struct Jtacs(FxHashMap<Side, FxHashMap<GroupId, Jtac>>);

impl Jtacs {
    pub fn jtac_targets<'a>(&'a self) -> impl Iterator<Item = UnitId> + 'a {
        self.0.values().flat_map(|j| {
            j.values()
                .filter_map(|jt| jt.target.as_ref().map(|(_, uid)| *uid))
        })
    }

    pub fn update_target_positions(&mut self, lua: MizLua, db: &Db) -> Result<()> {
        for jtx in self.0.values_mut() {
            for jt in jtx.values_mut() {
                if let Some((spotid, uid)) = &jt.target {
                    let unit = db.instance_unit(lua, uid)?;
                    let pos = unit.get_point()?;
                    if jt.contacts[uid].pos != pos.0 {
                        jt.contacts[uid].pos = pos.0;
                        let spot = Spot::get_instance(lua, spotid)?;
                        spot.set_point(pos)?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn update_contacts(&mut self, lua: MizLua, db: &mut Db) -> Result<()> {
        let land = Land::singleton(lua)?;
        let mut saw: SmallVec<[GroupId; 32]> = smallvec![];
        for (unit, _) in db.instanced_units() {
            for (pos, jtid, group, ifo) in db.jtacs() {
                if !saw.contains(&group.id) {
                    saw.push(group.id)
                }
                let range = (ifo.range as f64).powi(2);
                let jtac = self
                    .0
                    .entry(group.side)
                    .or_default()
                    .entry(group.id)
                    .or_insert_with(|| Jtac::new(jtid.clone(), db.cfg().jtac_priority.clone()));
                if !unit.tags.contains(jtac.filter) {
                    jtac.remove_contact(lua, &unit.id)?;
                    continue;
                }
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
        for j in self.0.values_mut() {
            j.retain(|uid, jt| {
                saw.contains(uid) || {
                    let _ = jt.remove_target(lua);
                    false
                }
            })
        }
        let mut new_contacts: SmallVec<[(GroupId, UnitId); 32]> = smallvec![];
        for j in self.0.values_mut() {
            for (gid, jtac) in j.iter_mut() {
                if let Some(uid) = jtac.sort_contacts(lua)? {
                    new_contacts.push((*gid, uid));
                }
            }
        }
        let mut msgs: SmallVec<[(GroupId, CompactString); 32]> = smallvec![];
        for (gid, uid) in new_contacts {
            msgs.push((gid, jtac_msg(db, gid, uid)?))
        }
        for (gid, msg) in msgs {
            let side = db.group(&gid)?.side;
            db.msgs().panel_to_side(10, false, side, msg);
        }
        Ok(())
    }
}
