use std::fmt;

use super::{Db, DeployKind, DeployableIndex, GroupId, Objective, ObjectiveId, SpawnedGroup};
use crate::{
    cfg::{CargoConfig, Crate, Deployable, LimitEnforceTyp, Troop, Vehicle},
    group,
    spawnctx::{SpawnCtx, SpawnLoc},
    unit, unit_mut,
};
use anyhow::{anyhow, bail, Result};
use compact_str::{format_compact, CompactString};
use dcso3::{
    centroid2d, coalition::Side, env::miz::MizIndex, land::Land, net::SlotId, trigger::Trigger,
    LuaVec2, MizLua, String, Vector2,
};
use fxhash::FxHashMap;
use log::{debug, error};
use serde_derive::{Deserialize, Serialize};
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone, Copy)]
pub struct NearbyCrate<'a> {
    pub group: &'a SpawnedGroup,
    pub origin: ObjectiveId,
    pub crate_def: &'a Crate,
    pub pos: Vector2,
    pub heading: f64,
    pub distance: f64,
}

#[derive(Debug, Clone, Copy)]
pub enum Unpakistan {
    Unpacked,
    Repaired,
}

impl fmt::Display for Unpakistan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Unpacked => write!(f, "unpacked"),
            Self::Repaired => write!(f, "repaired"),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Cargo {
    pub troops: SmallVec<[Troop; 1]>,
    pub crates: SmallVec<[(ObjectiveId, Crate); 1]>,
}

impl Cargo {
    pub fn num_troops(&self) -> usize {
        self.troops.len()
    }

    pub fn num_crates(&self) -> usize {
        self.crates.len()
    }

    pub fn num_total(&self) -> usize {
        self.num_crates() + self.num_troops()
    }

    pub fn weight(&self) -> i64 {
        let cr = self
            .crates
            .iter()
            .fold(0, |acc, (_, cr)| acc + cr.weight as i64);
        self.troops
            .iter()
            .fold(cr, |acc, tr| acc + tr.weight as i64)
    }
}

impl Db {
    fn point_near_logistics(
        &self,
        side: Side,
        point: Vector2,
    ) -> Result<(ObjectiveId, &Objective)> {
        let obj = self
            .persisted
            .objectives
            .into_iter()
            .find_map(|(oid, obj)| {
                if obj.owner == side
                    && obj.logi() > 0
                    && na::distance_squared(&obj.pos.into(), &point.into()) <= obj.radius.powi(2)
                {
                    return Some((oid, obj));
                }
                None
            });
        match obj {
            Some((oid, obj)) => Ok((*oid, obj)),
            None => bail!("not near friendly logistics"),
        }
    }

    pub fn spawn_crate(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
        name: &str,
    ) -> Result<()> {
        debug!("db spawning crate");
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let pos = self.slot_instance_pos(lua, idx, slot)?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let (oid, _) = self.point_near_logistics(side, point)?;
        let crate_cfg = self
            .ephemeral
            .deployable_idx
            .get(&side)
            .and_then(|idx| idx.crates_by_name.get(name))
            .ok_or_else(|| anyhow!("no such crate {name}"))?
            .clone();
        let template = self
            .ephemeral
            .cfg
            .crate_template
            .get(&side)
            .ok_or_else(|| anyhow!("missing crate template for {:?} side", side))?
            .clone();
        let spawnpos = 20. * pos.x.0 + pos.p.0; // spawn it 20 meters in front of the player
        let spawnpos = SpawnLoc::AtPos(Vector2::new(spawnpos.x, spawnpos.z));
        let dk = DeployKind::Crate(oid, crate_cfg.clone());
        self.add_and_queue_group(&SpawnCtx::new(lua)?, idx, side, spawnpos, &template, dk)?;
        Ok(())
    }

    fn list_crates_near_point<'a>(
        &'a self,
        point: Vector2,
        max_dist: f64,
    ) -> Result<SmallVec<[NearbyCrate<'a>; 4]>> {
        let mut res: SmallVec<[NearbyCrate; 4]> = smallvec![];
        for gid in &self.persisted.crates {
            let group = group!(self, gid)?;
            let (oid, crate_def) = match &group.origin {
                DeployKind::Crate(oid, crt) => (oid, crt),
                DeployKind::Deployed(_) | DeployKind::Troop(_) | DeployKind::Objective => {
                    bail!("group {:?} is listed in crates but isn't a crate", gid)
                }
            };
            for uid in &group.units {
                let unit = &unit!(self, uid)?;
                let distance = na::distance(&point.into(), &unit.pos.into());
                if distance <= max_dist {
                    let v = unit.pos - point;
                    let heading = v.y.atan2(v.x) * (180. / std::f64::consts::PI);
                    res.push(NearbyCrate {
                        group,
                        origin: *oid,
                        crate_def,
                        pos: unit.pos,
                        heading,
                        distance,
                    })
                }
            }
        }
        res.sort_by_key(|nc| (nc.distance * 1000.) as u32);
        Ok(res)
    }

    pub fn list_nearby_crates<'a>(
        &'a self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<SmallVec<[NearbyCrate<'a>; 4]>> {
        let pos = self.slot_instance_pos(lua, idx, slot)?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let max_dist = self.ephemeral.cfg.crate_load_distance as f64;
        self.list_crates_near_point(point, max_dist)
    }

    pub fn destroy_nearby_crate(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<()> {
        let nearby = self.list_nearby_crates(lua, idx, slot)?;
        let closest = nearby
            .into_iter()
            .next()
            .ok_or_else(|| anyhow!("no nearby crates"))?;
        let gid = closest.group.id;
        self.delete_group(&gid)
    }

    pub fn list_cargo(&self, slot: &SlotId) -> Option<&Cargo> {
        self.ephemeral.cargo.get(slot)
    }

    pub fn is_player_deployed(&self, gid: &GroupId) -> bool {
        self.persisted.deployed.contains(gid)
    }

    pub fn cargo_capacity(&self, unit: &dcso3::env::miz::Unit) -> Result<CargoConfig> {
        let vehicle = Vehicle::from(unit.typ()?);
        let cargo_capacity = self
            .ephemeral
            .cfg
            .cargo
            .get(&vehicle)
            .ok_or_else(|| anyhow!("{:?} can't carry cargo", vehicle))
            .map(|c| *c)?;
        Ok(cargo_capacity)
    }

    pub fn number_deployed(&self, name: &str) -> Result<(usize, Option<GroupId>)> {
        let mut n = 0;
        let mut oldest = None;
        for gid in &self.persisted.deployed {
            if let DeployKind::Deployed(d) = &group!(self, gid)?.origin {
                if let Some(d_name) = d.path.last() {
                    if d_name.as_str() == name {
                        if oldest.is_none() {
                            oldest = Some(*gid);
                        }
                        n += 1;
                    }
                }
            }
        }
        Ok((n, oldest))
    }

    pub fn unpakistan(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<(Unpakistan, String, GroupId)> {
        #[derive(Clone)]
        struct Cifo {
            pos: Vector2,
            group: GroupId,
            crate_def: Crate,
        }
        impl<'a> From<NearbyCrate<'a>> for Cifo {
            fn from(nc: NearbyCrate<'a>) -> Self {
                Self {
                    pos: nc.pos,
                    group: nc.group.id,
                    crate_def: nc.crate_def.clone(),
                }
            }
        }
        fn nearby(
            db: &Db,
            lua: MizLua,
            idx: &MizIndex,
            slot: &SlotId,
        ) -> Result<SmallVec<[Cifo; 8]>> {
            let nearby_player = db
                .list_nearby_crates(lua, idx, slot)?
                .into_iter()
                .map(Cifo::from)
                .collect::<SmallVec<[Cifo; 8]>>();
            if nearby_player.is_empty() {
                Ok(nearby_player)
            } else {
                let sp = db.ephemeral.cfg.crate_spread as f64;
                let mut crates = FxHashMap::default();
                for cr in &nearby_player {
                    for cr in db
                        .list_crates_near_point(cr.pos, sp)?
                        .into_iter()
                        .map(Cifo::from)
                    {
                        crates.entry(cr.group).or_insert(cr);
                    }
                }
                Ok(crates.into_iter().map(|(_, cr)| cr).collect())
            }
        }
        fn buildable(
            nearby: &SmallVec<[Cifo; 8]>,
            didx: &DeployableIndex,
        ) -> std::result::Result<FxHashMap<String, FxHashMap<String, Vec<Cifo>>>, CompactString>
        {
            let mut candidates: FxHashMap<String, FxHashMap<String, Vec<Cifo>>> =
                FxHashMap::default();
            let mut reasons = CompactString::new("");
            for cr in nearby {
                if let Some(dep) = didx.deployables_by_crates.get(&cr.crate_def.name) {
                    candidates
                        .entry(dep.clone())
                        .or_default()
                        .entry(cr.crate_def.name.clone())
                        .or_default()
                        .push(cr.clone());
                }
            }
            candidates.retain(|dep, have| {
                let spec = &didx.deployables_by_name[dep];
                for req in &spec.crates {
                    match have.get_mut(&req.name) {
                        Some(ids) if ids.len() >= req.required as usize => {
                            while ids.len() > req.required as usize {
                                ids.pop();
                            }
                        }
                        Some(_) | None => {
                            reasons.push_str(&format_compact!(
                                "can't spawn {dep} missing {}\n",
                                req.name
                            ));
                            return false;
                        }
                    }
                }
                true
            });
            if candidates.is_empty() {
                Err(reasons)
            } else {
                Ok(candidates)
            }
        }
        fn repairable(
            db: &Db,
            nearby: &SmallVec<[Cifo; 8]>,
            didx: &DeployableIndex,
            max_dist: f64,
        ) -> std::result::Result<FxHashMap<String, (GroupId, Vec<Cifo>)>, CompactString> {
            let mut repairs: FxHashMap<String, (GroupId, Vec<Cifo>)> = FxHashMap::default();
            let mut reasons = CompactString::new("");
            let max_dist = max_dist.powi(2);
            for cr in nearby {
                if let Some(dep) = didx.deployables_by_repair.get(&cr.crate_def.name) {
                    let mut group_to_repair = None;
                    for gid in &db.persisted.deployed {
                        let group = &db.persisted.groups[gid];
                        match &group.origin {
                            DeployKind::Deployed(d) if d.path.last() == Some(&dep) => {
                                for uid in &group.units {
                                    let unit_pos = db.persisted.units[uid].pos;
                                    if na::distance_squared(&unit_pos.into(), &cr.pos.into())
                                        <= max_dist
                                    {
                                        group_to_repair = Some(*gid);
                                        break;
                                    }
                                }
                                reasons
                                    .push_str(&format_compact!("not close enough to repair {dep}"));
                            }
                            DeployKind::Deployed(_)
                            | DeployKind::Crate(_, _)
                            | DeployKind::Objective
                            | DeployKind::Troop(_) => (),
                        }
                    }
                    if let Some(gid) = group_to_repair {
                        let (_, crates) =
                            repairs.entry(dep.clone()).or_insert_with(|| (gid, vec![]));
                        crates.push(cr.clone())
                    }
                }
            }
            repairs.retain(|dep, (_, have)| {
                let required = have[0].crate_def.required as usize;
                if have.len() < required {
                    reasons.push_str(&format_compact!("not enough crates to repair {dep}\n"));
                    false
                } else {
                    while have.len() > required {
                        have.pop();
                    }
                    true
                }
            });
            if repairs.is_empty() {
                Err(reasons)
            } else {
                Ok(repairs)
            }
        }
        fn too_close<'a, I: Iterator<Item = &'a Cifo>, F: Fn() -> I>(
            db: &Db,
            side: Side,
            centroid: Vector2,
            iter: F,
        ) -> bool {
            let excl_dist = (db.ephemeral.cfg.logistics_exclusion as f64).powi(2);
            db.persisted.objectives.into_iter().any(|(oid, obj)| {
                let mut check = false;
                for cr in iter() {
                    match db.persisted.groups.get(&cr.group) {
                        Some(group) => {
                            if let DeployKind::Crate(source, _) = &group.origin {
                                check |= oid == source;
                            }
                        }
                        None => error!("missing group {:?}", cr.group),
                    }
                }
                check |= obj.owner == side;
                check && {
                    let dist = na::distance_squared(&obj.pos.into(), &centroid.into());
                    dist <= excl_dist
                }
            })
        }
        fn compute_positions(
            db: &mut Db,
            have: &FxHashMap<String, Vec<Cifo>>,
            centroid: Vector2,
        ) -> Result<SpawnLoc> {
            let mut num_by_typ: FxHashMap<String, usize> = FxHashMap::default();
            let mut pos_by_typ: FxHashMap<String, Vector2> = FxHashMap::default();
            for cr in have.iter().flat_map(|(_, cr)| cr.iter()) {
                let group = &group!(db, cr.group)?;
                if let DeployKind::Crate(_, spec) = &group.origin {
                    if let Some(typ) = spec.pos_unit.as_ref() {
                        let uid = group
                            .units
                            .into_iter()
                            .next()
                            .ok_or_else(|| anyhow!("{:?} has no units", cr.group))?;
                        *pos_by_typ.entry(typ.clone()).or_default() += unit!(db, uid)?.pos;
                        *num_by_typ.entry(typ.clone()).or_default() += 1;
                    }
                }
            }
            for (typ, pos) in pos_by_typ.iter_mut() {
                if let Some(n) = num_by_typ.get(typ) {
                    *pos /= *n as f64
                }
            }
            let spawnloc = if pos_by_typ.is_empty() {
                SpawnLoc::AtPos(centroid)
            } else {
                SpawnLoc::AtPosWithComponents(centroid, pos_by_typ)
            };
            Ok(spawnloc)
        }
        fn enforce_deploy_limits(db: &mut Db, spec: &Deployable, dep: &String) -> Result<()> {
            let (n, oldest) = db.number_deployed(&**dep)?;
            if n >= spec.limit as usize {
                match spec.limit_enforce {
                    LimitEnforceTyp::DenyCrate => {
                        bail!("the max number of {:?} are already deployed", dep)
                    }
                    LimitEnforceTyp::DeleteOldest => {
                        if let Some(gid) = oldest {
                            db.delete_group(&gid)?
                        }
                    }
                }
            }
            Ok(())
        }
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let max_dist = self.ephemeral.cfg.crate_load_distance as f64;
        let nearby = nearby(self, lua, idx, slot)?;
        let didx = self
            .ephemeral
            .deployable_idx
            .get(&side)
            .ok_or_else(|| anyhow!("{:?} can't deploy anything", side))?;
        if nearby.is_empty() {
            bail!("no nearby crates")
        }
        match buildable(&nearby, didx) {
            Ok(mut candidates) => {
                let (dep, have) = candidates.drain().next().unwrap();
                let centroid = centroid2d(have.values().flat_map(|c| c.iter()).map(|c| c.pos));
                if too_close(self, side, centroid, || {
                    have.values().flat_map(|c| c.iter())
                }) {
                    bail!("too close to friendly logistics or crate origin");
                }
                let spec = didx.deployables_by_name[&dep].clone();
                enforce_deploy_limits(self, &spec, &dep)?;
                let spawnloc = compute_positions(self, &have, centroid)?;
                let origin = DeployKind::Deployed(spec.clone());
                let spctx = SpawnCtx::new(lua)?;
                for cr in have.values().flat_map(|c| c.iter()) {
                    self.delete_group(&cr.group)?
                }
                let gid =
                    self.add_and_queue_group(&spctx, idx, side, spawnloc, &*spec.template, origin)?;
                Ok((Unpakistan::Unpacked, dep, gid))
            }
            Err(build_reasons) => match repairable(self, &nearby, didx, max_dist) {
                Err(rep_reasons) => {
                    if build_reasons.is_empty() {
                        bail!(rep_reasons)
                    } else {
                        bail!("{build_reasons}\n{rep_reasons}")
                    }
                }
                Ok(mut candidates) => {
                    let (dep, (gid, have)) = candidates.drain().next().unwrap();
                    let centroid = centroid2d(have.iter().map(|c| c.pos));
                    if too_close(self, side, centroid, || have.iter()) {
                        bail!("too close to friendly logistics or crate origin")
                    }
                    let group = group!(self, gid)?;
                    for uid in &group.units {
                        let unit = unit_mut!(self, uid)?;
                        unit.dead = false;
                    }
                    for cr in &have {
                        self.delete_group(&cr.group)?
                    }
                    self.ephemeral.spawnq.push_back(gid);
                    self.ephemeral.dirty = true;
                    Ok((Unpakistan::Repaired, dep, gid))
                }
            },
        }
    }

    pub fn unload_crate(&mut self, lua: MizLua, idx: &MizIndex, slot: &SlotId) -> Result<Crate> {
        let cargo = self.ephemeral.cargo.get(slot);
        if cargo.map(|c| c.crates.is_empty()).unwrap_or(true) {
            bail!("no crates onboard")
        }
        let unit = self.slot_instance_unit(lua, idx, slot)?;
        let in_air = unit.as_object()?.in_air()?;
        let unit_name = unit.as_object()?.get_name()?;
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let pos = unit.as_object()?.get_position()?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let ground_alt = Land::singleton(lua)?.get_height(LuaVec2(point))?;
        let agl = pos.p.y - ground_alt;
        let speed = unit.as_object()?.get_velocity()?.0.magnitude();
        let cargo = self.ephemeral.cargo.get_mut(slot).unwrap();
        let (oid, crate_cfg) = cargo.crates.pop().unwrap();
        let weight = cargo.weight();
        debug!("drop speed {speed}, drop height {agl}");
        if in_air && speed > crate_cfg.max_drop_speed as f64 {
            cargo.crates.push((oid, crate_cfg));
            bail!("you are going too fast to unload your cargo")
        }
        if in_air && agl > crate_cfg.max_drop_height_agl as f64 {
            cargo.crates.push((oid, crate_cfg));
            bail!("you are too high to unload your cargo")
        }
        Trigger::singleton(lua)?
            .action()?
            .set_unit_internal_cargo(unit_name, weight)?;
        let template = self
            .ephemeral
            .cfg
            .crate_template
            .get(&side)
            .ok_or_else(|| anyhow!("missing crate template for {:?}", side))?
            .clone();
        let spawnpos = 20. * pos.x.0 + pos.p.0; // spawn it 20 meters in front of the player
        let spawnpos = SpawnLoc::AtPos(Vector2::new(spawnpos.x, spawnpos.z));
        let dk = DeployKind::Crate(oid, crate_cfg.clone());
        let spctx = SpawnCtx::new(lua)?;
        self.add_and_queue_group(&spctx, idx, side, spawnpos, &template, dk)?;
        Ok(crate_cfg)
    }

    pub fn unit_cargo_cfg(
        &self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<(CargoConfig, Side, String)> {
        let uifo = self.slot_miz_unit(lua, idx, slot)?;
        let side = uifo.side;
        let unit_name = uifo.unit.name()?;
        let cargo_capacity = self.cargo_capacity(&uifo.unit)?;
        Ok((cargo_capacity, side, unit_name))
    }

    pub fn load_nearby_crate(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<Crate> {
        let (cargo_capacity, side, unit_name) = self.unit_cargo_cfg(lua, idx, slot)?;
        let cargo = self.ephemeral.cargo.entry(slot.clone()).or_default();
        if cargo_capacity.crate_slots as usize <= cargo.num_crates()
            || cargo_capacity.total_slots as usize <= cargo.num_total()
        {
            bail!("you already have a full load onboard")
        }
        let (gid, oid, crate_def) = {
            let mut nearby = self.list_nearby_crates(lua, idx, slot)?;
            nearby.retain(|nc| nc.group.side == side);
            if nearby.is_empty() {
                bail!(
                    "no friendly crates within {} meters",
                    self.ephemeral.cfg.crate_load_distance
                );
            }
            let the_crate = nearby.first().unwrap();
            let gid = the_crate.group.id;
            let crate_def = the_crate.crate_def.clone();
            let oid = the_crate.origin;
            (gid, oid, crate_def)
        };
        let cargo = self.ephemeral.cargo.get_mut(slot).unwrap();
        cargo.crates.push((oid, crate_def.clone()));
        let weight = cargo.weight();
        self.delete_group(&gid)?;
        Trigger::singleton(lua)?
            .action()?
            .set_unit_internal_cargo(unit_name, weight as i64)?;
        Ok(crate_def)
    }

    pub fn load_troops(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
        name: &str,
    ) -> Result<Troop> {
        let (cargo_capacity, side, unit_name) = self.unit_cargo_cfg(lua, idx, slot)?;
        let pos = self.slot_instance_pos(lua, idx, slot)?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let _ = self.point_near_logistics(side, point)?;
        let troop_cfg = self
            .ephemeral
            .deployable_idx
            .get(&side)
            .and_then(|idx| idx.squads_by_name.get(name))
            .ok_or_else(|| anyhow!("no such squad {name}"))?
            .clone();
        let cargo = self.ephemeral.cargo.entry(slot.clone()).or_default();
        if cargo_capacity.troop_slots as usize <= cargo.num_troops()
            || cargo_capacity.total_slots as usize <= cargo.num_total()
        {
            bail!("you already have a full load onboard")
        }
        cargo.troops.push(troop_cfg.clone());
        Trigger::singleton(lua)?
            .action()?
            .set_unit_internal_cargo(unit_name, cargo.weight() as i64)?;
        Ok(troop_cfg)
    }

    pub fn unload_troops(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<(bool, Troop)> {
        let cargo = self.ephemeral.cargo.get(slot);
        if cargo.map(|c| c.troops.is_empty()).unwrap_or(true) {
            bail!("no troops onboard")
        }
        let unit = self.slot_instance_unit(lua, idx, slot)?;
        if unit.as_object()?.in_air()? {
            bail!("you must land to unload troops")
        }
        let unit_name = unit.as_object()?.get_name()?;
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let pos = unit.as_object()?.get_position()?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let cargo = self.ephemeral.cargo.get_mut(slot).unwrap();
        let troop_cfg = cargo.troops.pop().unwrap();
        let weight = cargo.weight();
        Trigger::singleton(lua)?
            .action()?
            .set_unit_internal_cargo(unit_name, weight)?;
        if self.point_near_logistics(side, point).is_ok() {
            Ok((true, troop_cfg))
        } else {
            let spawnpos = 20. * pos.x.0 + pos.p.0; // spawn it 20 meters in front of the player
            let spawnpos = SpawnLoc::AtPos(Vector2::new(spawnpos.x, spawnpos.z));
            let dk = DeployKind::Troop(troop_cfg.clone());
            let spctx = SpawnCtx::new(lua)?;
            self.add_and_queue_group(&spctx, idx, side, spawnpos, &*troop_cfg.template, dk)?;
            Ok((false, troop_cfg))
        }
    }

    pub fn extract_troops(&mut self, lua: MizLua, idx: &MizIndex, slot: &SlotId) -> Result<Troop> {
        let (cargo_capacity, side, unit_name) = self.unit_cargo_cfg(lua, idx, slot)?;
        let pos = self.slot_instance_pos(lua, idx, slot)?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let (gid, troop_cfg) = {
            let max_dist = (self.cfg().crate_load_distance as f64).powi(2);
            self.persisted
                .troops
                .into_iter()
                .filter_map(|gid| self.persisted.groups.get(gid).map(|g| (*gid, g)))
                .find_map(|(gid, g)| {
                    if let DeployKind::Troop(troop_cfg) = &g.origin {
                        if g.side == side {
                            let in_range = g
                                .units
                                .into_iter()
                                .filter_map(|uid| self.persisted.units.get(uid))
                                .any(|u| {
                                    na::distance_squared(&u.pos.into(), &point.into()) <= max_dist
                                });
                            if in_range {
                                return Some((gid, troop_cfg.clone()));
                            }
                        }
                    }
                    None
                })
                .ok_or_else(|| anyhow!("no troops in range"))?
        };
        let cargo = self.ephemeral.cargo.entry(slot.clone()).or_default();
        if cargo_capacity.troop_slots as usize <= cargo.num_troops()
            || cargo_capacity.total_slots as usize <= cargo.num_total()
        {
            bail!("you already have a full load onboard")
        }
        cargo.troops.push(troop_cfg.clone());
        Trigger::singleton(lua)?
            .action()?
            .set_unit_internal_cargo(unit_name, cargo.weight() as i64)?;
        self.delete_group(&gid)?;
        Ok(troop_cfg)
    }
}
