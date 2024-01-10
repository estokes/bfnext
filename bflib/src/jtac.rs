use crate::{
    cfg::{UnitTag, UnitTags},
    db::{
        ephemeral::Ephemeral,
        group::{GroupId, SpawnedUnit, UnitId},
        Db,
    },
    menu,
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::prelude::*;
use compact_str::{format_compact, CompactString};
use dcso3::{
    coalition::Side,
    land::Land,
    object::{DcsObject, DcsOid},
    radians_to_degrees,
    spot::{ClassSpot, Spot},
    trigger::{MarkId, Trigger},
    unit::{ClassUnit, Unit},
    LuaVec3, MizLua, String, Vector3,
};
use enumflags2::BitFlags;
use fxhash::{FxHashMap, FxHashSet};
use indexmap::IndexMap;
use log::{error, warn};
use smallvec::{smallvec, SmallVec};

fn ui_jtac_dead(db: &mut Ephemeral, lua: MizLua, side: Side, gid: GroupId) {
    db.msgs().panel_to_side(
        10,
        false,
        side,
        format_compact!("JTAC {gid} is no longer available"),
    );
    if let Err(e) = menu::remove_menu_for_jtac(lua, side, gid) {
        warn!("could not remove menu for jtac {gid} {e}")
    }
}

#[derive(Debug, Clone, Default)]
struct Contact {
    pos: Vector3,
    typ: String,
    tags: UnitTags,
    last_move: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone)]
struct Jtac {
    gid: GroupId,
    side: Side,
    contacts: IndexMap<UnitId, Contact>,
    id: DcsOid<ClassUnit>,
    filter: BitFlags<UnitTag>,
    priority: Vec<UnitTags>,
    target: Option<(DcsOid<ClassSpot>, Option<MarkId>, UnitId)>,
    autolase: bool,
    smoketarget: bool,
    code: u16,
}

impl Jtac {
    fn new(gid: GroupId, side: Side, id: DcsOid<ClassUnit>, priority: Vec<UnitTags>) -> Self {
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
        let dist = dist / 1000.;
        let mut msg = CompactString::new("");
        write!(msg, "JTAC {} status\n", self.gid)?;
        match self.target {
            None => write!(msg, "no target\n")?,
            Some((_, mid, uid)) => {
                let unit_typ = db.unit(&uid)?.typ.clone();
                let mid = match mid {
                    None => format_compact!("none"),
                    Some(mid) => format_compact!("{mid}"),
                };
                write!(msg, "lasing {unit_typ} code {} marker {mid}\n", self.code)?
            }
        }
        write!(
            msg,
            "position bearing {} for {:.1}km from {}\n\n",
            radians_to_degrees(heading) as u32,
            dist,
            obj.name()
        )?;
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
            "\n\nautolase: {}, smoke: {}",
            self.autolase, self.smoketarget
        )?;
        write!(msg, "\nfilter: [")?;
        let len = self.filter.len();
        for (i, tag) in self.filter.iter().enumerate() {
            if i < len - 1 {
                write!(msg, "{:?}, ", tag)?;
            } else {
                write!(msg, "{:?}", tag)?;
            }
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
            if let Some(mid) = mid {
                Trigger::singleton(lua)?.action()?.remove_mark(mid)?
            }
        }
        Ok(())
    }

    fn mark_target(&mut self, lua: MizLua) -> Result<()> {
        if let Some((_, mid, uid)) = &mut self.target {
            let act = Trigger::singleton(lua)?.action()?;
            if let Some(mid) = mid.take() {
                act.remove_mark(mid)?;
            }
            let new_mid = MarkId::new();
            let ct = &self.contacts[&*uid];
            let msg = format_compact!(
                "JTAC {} target {} marked by code {}",
                self.gid,
                ct.typ,
                self.code
            );
            act.mark_to_coalition(new_mid, msg.into(), LuaVec3(ct.pos), self.side, true, None)?;
            *mid = Some(new_mid);
        }
        Ok(())
    }

    fn set_target(&mut self, lua: MizLua, i: usize) -> Result<bool> {
        let (uid, ct) = self
            .contacts
            .get_index(i)
            .ok_or_else(|| anyhow!("no such target"))?;
        let uid = *uid;
        let pos = ct.pos;
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
                self.target = Some((spot.object_id()?, None, uid));
                self.mark_target(lua)?;
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
            self.code = 100 * hundreds + code_part + ones;
        } else {
            let c = self.code / 10;
            self.code = 10 * c + code_part;
        }
        if let Some((id, _, _)) = &self.target {
            let spot = Spot::get_instance(lua, id)?;
            spot.set_code(self.code)?;
            self.mark_target(lua)?
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
        let priority = |tags: UnitTags| {
            plist
                .iter()
                .enumerate()
                .find(|(_, p)| tags.contains(p.0))
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

    pub fn unit_dead(&mut self, lua: MizLua, db: &mut Db, id: &DcsOid<ClassUnit>) -> Result<()> {
        if let Some(uid) = db.ephemeral.get_uid_by_object_id(id) {
            let uid = *uid;
            for (side, jtx) in self.0.iter_mut() {
                jtx.retain(|gid, jt| {
                    if let Ok(spu) = db.unit(&uid) {
                        let group = spu.group;
                        if &group == gid {
                            if let Err(e) = jt.remove_target(lua) {
                                warn!("0 could not remove jtac target {:?}", e)
                            }
                            ui_jtac_dead(&mut db.ephemeral, lua, *side, *gid);
                            return false;
                        }
                    }
                    let dead = match &jt.target {
                        None => false,
                        Some((_, _, tuid)) => tuid == &uid,
                    };
                    if dead {
                        if let Err(e) = jt.remove_target(lua) {
                            warn!("1 could not remove jtac target {:?}", e)
                        }
                    }
                    true
                })
            }
        }
        Ok(())
    }

    pub fn update_target_positions(
        &mut self,
        lua: MizLua,
        db: &mut Db,
    ) -> Result<Vec<DcsOid<ClassUnit>>> {
        let dead = db
            .update_unit_positions(lua, Some(self.jtac_targets()))
            .context("updating the position of jtac targets")?;
        for jtx in self.0.values_mut() {
            for jt in jtx.values_mut() {
                if let Some((spotid, _, uid)) = &jt.target {
                    let unit = db.unit(uid)?;
                    if jt.contacts[uid].pos != unit.position.p.0 {
                        jt.contacts[uid].pos = unit.position.p.0;
                        let spot =
                            Spot::get_instance(lua, spotid).context("getting the spot instance")?;
                        spot.set_point(unit.position.p)
                            .context("setting the spot position")?;
                    }
                }
            }
        }
        Ok(dead)
    }

    pub fn update_contacts(&mut self, lua: MizLua, db: &mut Db) -> Result<()> {
        let land = Land::singleton(lua)?;
        let mut saw_jtacs: SmallVec<[GroupId; 32]> = smallvec![];
        let mut saw_units: FxHashSet<UnitId> = FxHashSet::default();
        for (mut pos, jtid, group, ifo) in db.jtacs() {
            if !saw_jtacs.contains(&group.id) {
                saw_jtacs.push(group.id)
            }
            let range = (ifo.range as f64).powi(2);
            pos.y += 5.;
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
                        db.ephemeral.cfg().jtac_priority.clone(),
                    )
                });
            for (unit, _) in db.instanced_units() {
                saw_units.insert(unit.id);
                if unit.side == jtac.side {
                    continue;
                }
                if !unit.tags.contains(jtac.filter) {
                    if let Err(e) = jtac.remove_contact(lua, &unit.id) {
                        warn!("could not filter jtac contact {} {:?}", unit.name, e)
                    }
                    continue;
                }
                if let Some(ct) = jtac.contacts.get(&unit.id) {
                    if unit.moved == ct.last_move {
                        continue;
                    }
                };
                let dist = na::distance_squared(&pos.into(), &unit.position.p.0.into());
                if dist <= range && land.is_visible(LuaVec3(pos), unit.position.p)? {
                    jtac.add_contact(unit)
                } else {
                    if let Err(e) = jtac.remove_contact(lua, &unit.id) {
                        warn!("could not remove jtac contact {} {:?}", unit.name, e)
                    }
                }
            }
        }
        for (side, jtx) in self.0.iter_mut() {
            jtx.retain(|gid, jt| {
                saw_jtacs.contains(gid) || {
                    if let Err(e) = jt.remove_target(lua) {
                        warn!("2 could not remove jtac target {:?}", e)
                    }
                    ui_jtac_dead(&mut db.ephemeral, lua, *side, *gid);
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
            if let Err(e) = self.get_mut(&gid)?.remove_contact(lua, &uid) {
                warn!("3 could not remove jtac target {uid} {:?}", e)
            }
            db.ephemeral.msgs().panel_to_side(
                10,
                false,
                side,
                format_compact!("JTAC {gid} target destroyed"),
            );
        }
        let mut new_contacts: SmallVec<[&Jtac; 32]> = smallvec![];
        for j in self.0.values_mut() {
            for (_, jtac) in j.iter_mut() {
                match jtac.sort_contacts(lua) {
                    Ok(false) => (),
                    Ok(true) => new_contacts.push(jtac),
                    Err(e) => warn!("could not sort contacts for jtac {}, {:?}", jtac.gid, e),
                }
            }
        }
        let mut msgs: SmallVec<[(Side, CompactString); 32]> = smallvec![];
        for jtac in new_contacts {
            let msg = jtac
                .status(db)
                .with_context(|| format_compact!("generating jtac status for {}", jtac.gid))?;
            msgs.push((jtac.side, msg))
        }
        for (side, msg) in msgs {
            db.ephemeral.msgs().panel_to_side(10, false, side, msg);
        }
        Ok(())
    }
}
