use super::{Db, DeployKind, GroupId, ObjectiveId, SpawnedGroup};
use crate::{
    cfg::{CargoConfig, Crate, LimitEnforceTyp, Troop, Vehicle},
    group,
    spawnctx::{SpawnCtx, SpawnLoc},
    unit,
};
use anyhow::{anyhow, bail, Result};
use compact_str::{format_compact, CompactString};
use dcso3::{
    centroid2d, env::miz::MizIndex, land::Land, net::SlotId, trigger::Trigger, LuaVec2, MizLua,
    String, Vector2,
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
        let obj = self
            .persisted
            .objectives
            .into_iter()
            .find_map(|(oid, obj)| {
                if obj.owner == side
                    && obj.logi() > 0
                    && na::distance(&obj.pos.into(), &point.into()) <= obj.radius
                {
                    return Some((oid, obj));
                }
                None
            });
        let oid = match obj {
            Some((oid, _)) => *oid,
            None => bail!("not near friendly logistics"),
        };
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

    pub fn list_nearby_crates<'a>(
        &'a self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<SmallVec<[NearbyCrate<'a>; 2]>> {
        let pos = self.slot_instance_pos(lua, idx, slot)?;
        let point = Vector2::new(pos.p.x, pos.p.z);
        let max_dist = self.ephemeral.cfg.crate_load_distance as f64;
        let mut res: SmallVec<[NearbyCrate; 2]> = smallvec![];
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
    ) -> Result<(String, GroupId)> {
        #[derive(Clone)]
        struct Cifo {
            pos: Vector2,
            group: GroupId,
            crate_def: Crate,
        }
        let side = self.slot_miz_unit(lua, idx, slot)?.side;
        let nearby = self
            .list_nearby_crates(lua, idx, slot)?
            .into_iter()
            .map(|nc| Cifo {
                pos: nc.pos,
                group: nc.group.id,
                crate_def: nc.crate_def.clone(),
            })
            .collect::<SmallVec<[Cifo; 2]>>();
        let didx = self
            .ephemeral
            .deployable_idx
            .get(&side)
            .ok_or_else(|| anyhow!("{:?} can't deploy anything", side))?;
        let mut candidates: FxHashMap<String, FxHashMap<String, Vec<Cifo>>> = FxHashMap::default();
        for cr in nearby {
            if let Some(dep) = didx.deployables_by_crates.get(&cr.crate_def.name) {
                candidates
                    .entry(dep.clone())
                    .or_default()
                    .entry(cr.crate_def.name.clone())
                    .or_default()
                    .push(cr);
            }
        }
        let mut reasons = CompactString::new("");
        candidates.retain(|dep, have| {
            let spec = match didx.deployables_by_name.get(dep) {
                Some(spec) => spec,
                None => {
                    error!("missing deployable {dep}");
                    return false;
                }
            };
            for req in &spec.crates {
                match have.get_mut(&req.name) {
                    Some(ids) if ids.len() >= req.required as usize => {
                        while ids.len() > req.required as usize {
                            ids.pop();
                        }
                    }
                    Some(_) | None => {
                        reasons
                            .push_str(&format_compact!("can't spawn {dep} missing {}\n", req.name));
                        return false;
                    }
                }
            }
            true
        });
        let (dep, have) = match candidates.drain().next() {
            Some((dep, have)) => (dep, have),
            None => bail!(reasons),
        };
        let centroid = centroid2d(have.iter().flat_map(|(_, c)| c.iter()).map(|c| c.pos));
        let too_close = self.persisted.objectives.into_iter().any(|(oid, obj)| {
            let mut check = false;
            for cr in have.iter().flat_map(|(_, c)| c.iter()) {
                match self.persisted.groups.get(&cr.group) {
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
                let dist = na::distance(&obj.pos.into(), &centroid.into());
                let excl_dist = self.ephemeral.cfg.logistics_exclusion as f64;
                dist <= excl_dist
            }
        });
        if too_close {
            bail!("too close to friendly logistics or crate origin");
        }
        let spec = didx
            .deployables_by_name
            .get(&dep)
            .ok_or_else(|| anyhow!("missing deployable {dep}"))?
            .clone();
        let (n, oldest) = self.number_deployed(&*dep)?;
        if n >= spec.limit as usize {
            match spec.limit_enforce {
                LimitEnforceTyp::DenyCrate => {
                    bail!("the max number of {:?} are already deployed", dep)
                }
                LimitEnforceTyp::DeleteOldest => {
                    if let Some(gid) = oldest {
                        self.delete_group(&gid)?
                    }
                }
            }
        }
        let mut num_by_typ: FxHashMap<String, usize> = FxHashMap::default();
        let mut pos_by_typ: FxHashMap<String, Vector2> = FxHashMap::default();
        for cr in have.iter().flat_map(|(_, cr)| cr.iter()) {
            let group = &group!(self, cr.group)?;
            if let DeployKind::Crate(_, spec) = &group.origin {
                if let Some(typ) = spec.pos_unit.as_ref() {
                    let uid = group
                        .units
                        .into_iter()
                        .next()
                        .ok_or_else(|| anyhow!("{:?} has no units", cr.group))?;
                    *pos_by_typ.entry(typ.clone()).or_default() += unit!(self, uid)?.pos;
                    *num_by_typ.entry(typ.clone()).or_default() += 1;
                }
            }
            self.delete_group(&cr.group)?
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
        let origin = DeployKind::Deployed(spec.clone());
        let spctx = SpawnCtx::new(lua)?;
        let gid = self.add_and_queue_group(&spctx, idx, side, spawnloc, &*spec.template, origin)?;
        Ok((dep, gid))
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

    pub fn load_nearby_crate(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        slot: &SlotId,
    ) -> Result<Crate> {
        let (cargo_capacity, side, unit_name) = {
            let uifo = self.slot_miz_unit(lua, idx, slot)?;
            let side = uifo.side;
            let unit_name = uifo.unit.name()?;
            let cargo_capacity = self.cargo_capacity(&uifo.unit)?;
            (cargo_capacity, side, unit_name)
        };
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
}
