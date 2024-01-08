use super::{group::DeployKind, objective::ObjGroup, Db, Map};
use crate::{
    cfg::{Cfg, Vehicle},
    db::{
        logistics::Warehouse,
        objective::{Objective, ObjectiveId, ObjectiveKind},
    },
    group, objective_mut,
    spawnctx::{SpawnCtx, SpawnLoc},
};
use anyhow::{anyhow, bail, Result};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    env::miz::{Group, Miz, MizIndex, TriggerZone, TriggerZoneTyp},
    MizLua, String, Vector2 
};
use log::info;

impl Db {
    /// objectives are just trigger zones named according to type codes
    /// the first caracter is the type of the zone
    /// O - Objective
    /// G - Group within an objective
    /// T - Generic trigger zone, ignored by the engine
    ///
    /// Then a 2 character type code
    /// - AB: Airbase
    /// - FO: Fob
    /// - SA: Sam site
    /// - LO: Logistics Objective
    ///
    /// Then a 1 character code for the default owner
    /// followed by the display name
    /// - R: Red
    /// - B: Blue
    /// - N: Neutral
    ///
    /// So e.g. Tblisi would be OABBTBLISI -> Objective, Airbase, Default to Blue, named Tblisi
    fn init_objective(&mut self, zone: TriggerZone, name: &str) -> Result<()> {
        fn side_and_name(s: &str) -> Result<(Side, String)> {
            if let Some(name) = s.strip_prefix("R") {
                Ok((Side::Red, String::from(name)))
            } else if let Some(name) = s.strip_prefix("B") {
                Ok((Side::Blue, String::from(name)))
            } else if let Some(name) = s.strip_prefix("N") {
                Ok((Side::Neutral, String::from(name)))
            } else {
                bail!("invalid default coalition {s} expected B, R, or N prefix")
            }
        }
        let (kind, owner, name) = if let Some(name) = name.strip_prefix("AB") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Airbase, side, name)
        } else if let Some(name) = name.strip_prefix("FO") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Fob, side, name)
        } else if let Some(name) = name.strip_prefix("LO") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Logistics, side, name)
        } else {
            bail!("invalid objective type for {name}, expected AB, FO, of LO")
        };
        let id = ObjectiveId::new();
        let radius = match zone.typ()? {
            TriggerZoneTyp::Quad(_) => bail!("zone volume type quad isn't supported yet"),
            TriggerZoneTyp::Circle { radius } => radius,
        };
        let pos = zone.pos()?;
        let obj = Objective {
            id,
            spawned: false,
            threatened: false,
            pos,
            radius,
            name: name.clone(),
            kind,
            owner,
            slots: Map::new(),
            groups: Map::new(),
            health: 0,
            logi: 0,
            last_change_ts: Utc::now(),
            last_threatened_ts: Utc::now(),
            warehouse: Warehouse::default(),
            needs_mark: false,
        };
        self.persisted.objectives.insert_cow(id, obj);
        self.persisted.objectives_by_name.insert_cow(name, id);
        Ok(())
    }

    /// Objective groups are trigger zones with the first character set to G. They are then a template
    /// name, followed by # and a number. They are associated with an objective by proximity.
    /// e.g. GRIRSRAD#001 would be the 1st instantiation of the template RIRSRAD, which must
    /// correspond to a group in the miz file. There is one special template name called (R|B|N)LOGI
    /// which corresponds to the logistics template for objectives
    fn init_objective_group(
        &mut self,
        spctx: &SpawnCtx,
        idx: &MizIndex,
        _miz: &Miz,
        zone: TriggerZone,
        side: Side,
        name: &str,
    ) -> Result<()> {
        let pos = zone.pos()?;
        let obj = {
            let mut iter = self.persisted.objectives.into_iter();
            loop {
                match iter.next() {
                    None => bail!("group {:?} isn't associated with an objective", name),
                    Some((id, obj)) => {
                        if na::distance_squared(&pos.into(), &obj.pos.into()) <= obj.radius.powi(2)
                        {
                            break *id;
                        }
                    }
                }
            }
        };
        let gid = self.add_group(
            spctx,
            idx,
            side,
            SpawnLoc::AtPos {
                pos,
                offset_direction: Vector2::default(),
                group_heading: 0.,
            },
            name,
            DeployKind::Objective,
        )?;
        objective_mut!(self, obj)?
            .groups
            .get_or_default_cow(side)
            .insert_cow(gid);
        self.persisted.objectives_by_group.insert_cow(gid, obj);
        Ok(())
    }

    pub fn init_objective_slots(&mut self, slot: Group) -> Result<()> {
        for unit in slot.units()? {
            let unit = unit?;
            let id = unit.slot()?;
            let pos = slot.pos()?;
            let obj = {
                let mut iter = self.persisted.objectives.into_iter();
                loop {
                    match iter.next() {
                        None => {
                            info!("slot {:?} not associated with an objective", slot);
                            return Ok(());
                        }
                        Some((id, obj)) => {
                            if na::distance(&pos.into(), &obj.pos.into()) <= obj.radius {
                                break *id;
                            }
                        }
                    }
                }
            };
            let vehicle = Vehicle::from(unit.typ()?);
            match self.ephemeral.cfg.threatened_distance.get(&vehicle) {
                Some(_) => (),
                None => bail!(
                    "vehicle {:?} doesn't have a configured theatened distance",
                    vehicle
                ),
            }
            match self.ephemeral.cfg.life_types.get(&vehicle) {
                None => bail!("vehicle {:?} doesn't have a configured life type", vehicle),
                Some(typ) => match self.ephemeral.cfg.default_lives.get(&typ) {
                    Some((n, f)) if *n > 0 && *f > 0 => (),
                    None => bail!("vehicle {:?} has no configured life type", vehicle),
                    Some((n, f)) => {
                        bail!(
                            "vehicle {:?} life type {:?} has no configured lives ({n}) or negative reset time ({f})",
                            vehicle, typ
                        )
                    }
                },
            }
            self.persisted
                .objectives_by_slot
                .insert_cow(id.clone(), obj);
            objective_mut!(self, obj)?.slots.insert_cow(id, vehicle);
        }
        Ok(())
    }

    pub fn init(lua: MizLua, cfg: Cfg, idx: &MizIndex, miz: &Miz) -> Result<Self> {
        let spctx = SpawnCtx::new(lua)?;
        let mut t = Self::default();
        t.ephemeral.set_cfg(miz, idx, cfg)?;
        for zone in miz.triggers()? {
            let zone = zone?;
            let name = zone.name()?;
            if let Some(name) = name.strip_prefix("O") {
                t.init_objective(zone, name)?
            }
        }
        for side in [Side::Blue, Side::Red, Side::Neutral] {
            let coa = miz.coalition(side)?;
            for zone in miz.triggers()? {
                let zone = zone?;
                let name = zone.name()?;
                if let Some(name) = name.strip_prefix("G") {
                    let (template_side, name) = name.parse::<ObjGroup>()?.template(side);
                    if template_side == side {
                        t.init_objective_group(&spctx, idx, miz, zone, side, name.as_str())?
                    }
                } else if name.starts_with("T") || name.starts_with("O") {
                    () // ignored
                } else {
                    bail!("invalid trigger zone type code {name}, expected O, G, or T prefix")
                }
            }
            for country in coa.countries()? {
                let country = country?;
                for plane in country.planes()? {
                    let plane = plane?;
                    t.init_objective_slots(plane)?
                }
                for heli in country.helicopters()? {
                    let heli = heli?;
                    t.init_objective_slots(heli)?
                }
            }
        }
        let now = Utc::now();
        let ids = t
            .persisted
            .objectives
            .into_iter()
            .map(|(id, _)| *id)
            .collect::<Vec<_>>();
        for id in ids {
            t.update_objective_status(&id, now)?
        }
        t.ephemeral.dirty();
        Ok(t)
    }

    pub fn respawn_after_load(&mut self, idx: &MizIndex, spctx: &SpawnCtx) -> Result<()> {
        for gid in &self.persisted.deployed {
            self.spawn_group(idx, spctx, group!(self, gid)?)?
        }
        for gid in &self.persisted.crates {
            self.spawn_group(idx, spctx, group!(self, gid)?)?
        }
        for gid in &self.persisted.troops {
            self.spawn_group(idx, spctx, group!(self, gid)?)?
        }
        for (_, obj) in &self.persisted.objectives {
            if let Some(groups) = obj.groups.get(&obj.owner) {
                for gid in groups {
                    let group = group!(self, gid)?;
                    if group.class.is_logi() {
                        self.spawn_group(idx, spctx, group)?
                    }
                }
            }
        }
        let groups = self
            .persisted
            .groups
            .into_iter()
            .map(|(gid, _)| *gid)
            .collect::<Vec<_>>();
        for gid in groups {
            self.mark_group(&gid)?
        }
        let objectives = self
            .persisted
            .objectives
            .into_iter()
            .map(|(oid, _)| *oid)
            .collect::<Vec<_>>();
        for oid in objectives {
            self.mark_objective(&oid)?
        }
        for (uid, unit) in &self.persisted.units {
            let group = group!(self, unit.group)?;
            match group.origin {
                DeployKind::Crate { .. } => (),
                DeployKind::Deployed { .. } | DeployKind::Troop { .. } => {
                    self.ephemeral
                        .units_potentially_close_to_enemies
                        .insert(*uid);
                }
                DeployKind::Objective => {
                    let oid = self.persisted.objectives_by_group[&unit.group];
                    let obj = &self.persisted.objectives[&oid];
                    if obj.owner == group.side {
                        self.ephemeral
                            .units_potentially_close_to_enemies
                            .insert(*uid);
                        self.ephemeral.units_potentially_on_walkabout.insert(*uid);
                    }
                }
            }
        }
        Ok(())
    }
}
