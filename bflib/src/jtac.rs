use crate::{
    cfg::UnitTag,
    db::{Db, GroupId, SpawnedUnit, UnitId},
    menu,
};
use anyhow::{anyhow, bail, Result};
use chrono::prelude::*;
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    land::Land,
    object::{DcsObject, DcsOid},
    spot::{ClassSpot, Spot},
    trigger::{MarkId, Trigger},
    unit::{ClassUnit, Unit},
    LuaVec3, MizLua, String, Vector3,
};
use enumflags2::BitFlags;
use fxhash::{FxHashMap, FxHashSet};
use indexmap::IndexMap;
use log::{debug, error};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone, Default)]
struct Contact {
    pos: Vector3,
    typ: String,
    tags: BitFlags<UnitTag>,
    last_move: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct Jtac {
    gid: GroupId,
    side: Side,
    contacts: IndexMap<UnitId, Contact>,
    id: DcsOid<ClassUnit>,
    filter: BitFlags<UnitTag>,
    priority: Vec<BitFlags<UnitTag>>,
    target: Option<(DcsOid<ClassSpot>, MarkId, UnitId)>,
    autolase: bool,
    smoketarget: bool,
    code: u16,
}

impl Jtac {
    fn new(
        gid: GroupId,
        side: Side,
        id: DcsOid<ClassUnit>,
        priority: Vec<BitFlags<UnitTag>>,
    ) -> Self {
        Self {
            gid,
            side,
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

    fn status(&self, db: &Db) -> Result<CompactString> {
        use std::fmt::Write;
        let jtac_pos = db.group_center(&self.gid)?;
        let (dist, heading, obj) = db.objective_near_point(jtac_pos);
        let mut msg = format_compact!(
            "JTAC {} bearing {} for {} from {}, ",
            self.gid,
            dist as u32,
            heading as u32,
            obj.name()
        );
        match self.target {
            None => write!(msg, "no target")?,
            Some((_, mid, uid)) => {
                let unit_typ = db.unit(&uid)?.typ.clone();
                write!(msg, "now lasing {unit_typ} code {} marker {mid}", self.code)?
            }
        }
        write!(msg, "\n")?;
        if self.contacts.is_empty() {
            write!(msg, "No enemies in sight")?;
        } else {
            write!(msg, "Visual On: ")?;
            for (i, uid) in self.contacts.keys().enumerate() {
                if i == self.contacts.len() - 1 {
                    write!(msg, "{}", db.unit(uid)?.typ)?;
                } else {
                    write!(msg, "{}, ", db.unit(uid)?.typ)?;
                }
            }
        }
        write!(
            msg,
            "\nautolase: {}, smoke: {}",
            self.autolase, self.smoketarget
        )?;
        write!(msg, "\nfilter: [")?;
        for tag in self.filter.iter() {
            write!(msg, "{:?}", tag)?;
        }
        write!(msg, "]")?;
        Ok(msg)
    }

    fn add_contact(&mut self, unit: &SpawnedUnit) {
        let ct = self.contacts.entry(unit.id).or_default();
        ct.pos = unit.position.p.0;
        ct.last_move = unit.moved;
        ct.tags = unit.tags;
        ct.typ = unit.typ.clone();
    }

    fn remove_target(&mut self, lua: MizLua) -> Result<()> {
        if let Some((id, mid, _)) = self.target.take() {
            let spot = Spot::get_instance(lua, &id)?;
            spot.destroy()?;
            Trigger::singleton(lua)?.action()?.remove_mark(mid)?
        }
        Ok(())
    }

    fn mark_target(&self, lua: MizLua, mid: MarkId, pos: Vector3, typ: String) -> Result<()> {
        let act = Trigger::singleton(lua)?.action()?;
        let _ = act.remove_mark(mid);
        let msg = format_compact!(
            "JTAC {} target {} marked by code {}",
            self.gid,
            typ,
            self.code
        );
        act.mark_to_coalition(mid, msg.into(), LuaVec3(pos), self.side, true, None)
    }

    fn set_target(&mut self, lua: MizLua, i: usize) -> Result<bool> {
        let (uid, ct) = self
            .contacts
            .get_index(i)
            .ok_or_else(|| anyhow!("no such target"))?;
        let uid = *uid;
        let pos = ct.pos;
        let typ = ct.typ.clone();
        match &self.target {
            Some((_, _, tuid)) if tuid == &uid => Ok(false),
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
                let mid = MarkId::new();
                self.target = Some((spot.object_id()?, mid, uid));
                self.mark_target(lua, mid, pos, typ)?;
                Ok(true)
            }
        }
    }

    fn set_code(&mut self, lua: MizLua, code_part: u16) -> Result<()> {
        let hundreds = code_part / 100;
        let tens = code_part / 10;
        if hundreds > 9 || (hundreds > 0 && code_part % 100 > 0) || (tens > 0 && code_part % 10 > 0)
        {
            bail!("invalid code part {code_part}, mixed scales")
        }
        if hundreds > 0 {
            let tens_ones = self.code % 100;
            self.code = 1000 + code_part + tens_ones;
        } else if tens > 0 {
            let hundreds = self.code / 100;
            let ones = self.code % 10;
            self.code = hundreds + code_part + ones;
        } else {
            let c = self.code / 10;
            self.code = c + code_part;
        }
        if let Some((id, mid, uid)) = &self.target {
            let spot = Spot::get_instance(lua, id)?;
            spot.set_code(self.code)?;
            let ct = &self.contacts[uid];
            self.mark_target(lua, *mid, ct.pos, ct.typ.clone())?
        }
        Ok(())
    }

    fn shift(&mut self, lua: MizLua) -> Result<bool> {
        let i = match &self.target {
            None => 0,
            Some((_, _, uid)) => match self.contacts.get_index_of(uid) {
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

    fn remove_contact(&mut self, lua: MizLua, uid: &UnitId) -> Result<bool> {
        if let Some(_) = self.contacts.swap_remove(uid) {
            if let Some((_, _, tuid)) = &self.target {
                if tuid == uid {
                    self.remove_target(lua)?;
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    fn sort_contacts(&mut self, lua: MizLua) -> Result<bool> {
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
        Ok(false)
    }
}

#[derive(Debug, Clone, Default)]
pub struct Jtacs(FxHashMap<Side, FxHashMap<GroupId, Jtac>>);

impl Jtacs {
    fn get(&self, gid: &GroupId) -> Result<&Jtac> {
        self.0
            .iter()
            .find_map(|(_, jtx)| jtx.get(gid))
            .ok_or_else(|| anyhow!("no such jtac {gid}"))
    }

    fn get_mut(&mut self, gid: &GroupId) -> Result<&mut Jtac> {
        self.0
            .iter_mut()
            .find_map(|(_, jtx)| jtx.get_mut(gid))
            .ok_or_else(|| anyhow!("no such jtac"))
    }

    pub fn jtac_status(&self, db: &Db, gid: &GroupId) -> Result<CompactString> {
        self.get(gid)?.status(db)
    }

    pub fn toggle_auto_laser(&mut self, lua: MizLua, gid: &GroupId) -> Result<()> {
        let jtac = self.get_mut(gid)?;
        jtac.autolase = !jtac.autolase;
        if jtac.autolase {
            jtac.shift(lua)?;
        } else {
            jtac.remove_target(lua)?
        }
        Ok(())
    }

    pub fn toggle_smoke_target(&mut self, gid: &GroupId) -> Result<()> {
        let jtac = self.get_mut(gid)?;
        jtac.smoketarget = !jtac.smoketarget;
        Ok(())
    }

    pub fn shift(&mut self, lua: MizLua, gid: &GroupId) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.shift(lua)
    }

    pub fn clear_filter(&mut self, lua: MizLua, gid: &GroupId) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.filter = BitFlags::empty();
        jtac.sort_contacts(lua)
    }

    pub fn add_filter(&mut self, lua: MizLua, gid: &GroupId, tag: UnitTag) -> Result<bool> {
        let jtac = self.get_mut(gid)?;
        jtac.filter |= tag;
        jtac.sort_contacts(lua)
    }

    /// set part of the laser code, defined by the scale of the passed in number. For example,
    /// passing 600 sets the hundreds part of the code to 6. passing 8 sets the ones part of the code to 8.
    /// other parts of the existing code are left alone.
    pub fn set_code_part(&mut self, lua: MizLua, gid: &GroupId, code_part: u16) -> Result<()> {
        self.get_mut(gid)?.set_code(lua, code_part)
    }

    pub fn jtac_targets<'a>(&'a self) -> impl Iterator<Item = UnitId> + 'a {
        self.0.values().flat_map(|j| {
            j.values()
                .filter_map(|jt| jt.target.as_ref().map(|(_, _, uid)| *uid))
        })
    }

    pub fn update_target_positions(&mut self, lua: MizLua, db: &Db) -> Result<()> {
        for jtx in self.0.values_mut() {
            for jt in jtx.values_mut() {
                if let Some((spotid, _, uid)) = &jt.target {
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
        let mut saw_jtacs: SmallVec<[GroupId; 32]> = smallvec![];
        let mut saw_units: FxHashSet<UnitId> = FxHashSet::default();
        for (unit, _) in db.instanced_units() {
            saw_units.insert(unit.id);
            for (pos, jtid, group, ifo) in db.jtacs() {
                debug!("jtac {:?} considering {:?}", group, unit);
                if !saw_jtacs.contains(&group.id) {
                    saw_jtacs.push(group.id)
                }
                let range = (ifo.range as f64).powi(2);
                let jtac = self
                    .0
                    .entry(group.side)
                    .or_default()
                    .entry(group.id)
                    .or_insert_with(|| {
                        if let Err(e) = menu::add_menu_for_jtac(lua, group.side, group.id) {
                            error!("could not add menu for jtac {} {e}", group.id)
                        }
                        Jtac::new(
                            group.id,
                            group.side,
                            jtid.clone(),
                            db.cfg().jtac_priority.clone(),
                        )
                    });
                if unit.side == jtac.side {
                    continue;
                }
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
                    jtac.remove_contact(lua, &unit.id)?;
                }
            }
        }
        for (side, jtx) in self.0.iter_mut() {
            jtx.retain(|gid, jt| {
                saw_jtacs.contains(gid) || {
                    let _ = jt.remove_target(lua);
                    db.msgs().panel_to_side(
                        10,
                        false,
                        *side,
                        format_compact!("JTAC {gid} is no longer available"),
                    );
                    if let Err(e) = menu::remove_menu_for_jtac(lua, *side, *gid) {
                        error!("could not remove menu for jtac {gid} {e}")
                    }
                    false
                }
            })
        }
        let mut killed_targets: SmallVec<[(Side, GroupId, UnitId); 16]> = smallvec![];
        for (side, jtx) in self.0.iter_mut() {
            for jtac in jtx.values_mut() {
                for (uid, _) in &jtac.contacts {
                    if !saw_units.contains(uid) {
                        killed_targets.push((*side, jtac.gid, *uid));
                    }
                }
            }
        }
        for (side, gid, uid) in killed_targets {
            self.get_mut(&gid)?.remove_contact(lua, &uid)?;
            db.msgs().panel_to_side(
                10,
                false,
                side,
                format_compact!("JTAC {gid} target destroyed"),
            );
        }
        let mut new_contacts: SmallVec<[&Jtac; 32]> = smallvec![];
        for j in self.0.values_mut() {
            for (_, jtac) in j.iter_mut() {
                if jtac.sort_contacts(lua)? {
                    new_contacts.push(jtac);
                }
            }
        }
        let mut msgs: SmallVec<[(GroupId, CompactString); 32]> = smallvec![];
        for jtac in new_contacts {
            msgs.push((jtac.gid, jtac.status(db)?))
        }
        for (gid, msg) in msgs {
            let side = db.group(&gid)?.side;
            db.msgs().panel_to_side(10, false, side, msg);
        }
        Ok(())
    }
}
