use crate::{
    cfg::{Cfg, Vehicle},
    spawnctx::{SpawnCtx, SpawnLoc},
};

use super::{Db, DeployKind, Map, ObjGroup, Objective, ObjectiveId, ObjectiveKind};
use anyhow::{bail, Result};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    env::miz::{Group, Miz, MizIndex, TriggerZone, TriggerZoneTyp},
    net::SlotId,
    MizLua, String,
};

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
    /// - FB: Fuel base
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
        } else if let Some(name) = name.strip_prefix("FB") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Fuelbase, side, name)
        } else if let Some(name) = name.strip_prefix("SA") {
            let (side, name) = side_and_name(name)?;
            (ObjectiveKind::Samsite, side, name)
        } else {
            bail!("invalid objective type for {name}, expected AB, FO, FB, or SA prefix")
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
            trigger_name: zone.name()?,
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
        let name = name.parse::<ObjGroup>()?;
        let pos = zone.pos()?;
        let obj = {
            let mut iter = self.persisted.objectives.into_iter();
            loop {
                match iter.next() {
                    None => bail!("group {:?} isn't associated with an objective", name),
                    Some((id, obj)) => {
                        if na::distance_squared(&pos.into(), &obj.pos.into()) <= obj.radius.powi(2) {
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
            SpawnLoc::AtPos(pos),
            name.template(side).as_str(),
            DeployKind::Objective,
        )?;
        self.persisted.objectives[&obj]
            .groups
            .get_or_default_cow(side)
            .insert_cow(name.clone(), gid);
        self.persisted.objectives_by_group.insert_cow(gid, obj);
        Ok(())
    }

    pub fn init_objective_slots(&mut self, slot: Group) -> Result<()> {
        for unit in slot.units()? {
            let unit = unit?;
            let id = SlotId::from(unit.id()?);
            let pos = slot.pos()?;
            let obj = {
                let mut iter = self.persisted.objectives.into_iter();
                loop {
                    match iter.next() {
                        None => bail!("slot {:?} not associated with an objective", slot),
                        Some((id, obj)) => {
                            if na::distance(&pos.into(), &obj.pos.into()) <= obj.radius {
                                break *id;
                            }
                        }
                    }
                }
            };
            let vehicle = Vehicle::from(unit.typ()?);
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
            self.persisted.objectives[&obj]
                .slots
                .insert_cow(id, vehicle);
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
        for side in [
            Side::Blue,
            Side::Red,
            Side::Neutral,
        ] {
            let coa = miz.coalition(side)?;
            for zone in miz.triggers()? {
                let zone = zone?;
                let name = zone.name()?;
                if let Some(name) = name.strip_prefix("G") {
                    t.init_objective_group(&spctx, idx, miz, zone, side, name)?
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
        t.ephemeral.dirty = true;
        Ok(t)
    }
}
