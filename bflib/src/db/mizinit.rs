/*
Copyright 2024 Eric Stokes.

This file is part of bflib.

bflib is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your
option) any later version.

bflib is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
for more details.
*/

use super::{
    group::DeployKind,
    objective::{ObjGroup, SlotInfo},
    Db, Map,
};
use crate::{
    cfg::{Cfg, Vehicle},
    db::{
        logistics::Warehouse,
        objective::{Objective, ObjectiveId, ObjectiveKind},
    },
    group, objective_mut,
    spawnctx::{SpawnCtx, SpawnLoc},
};
use anyhow::{anyhow, bail, Context, Result};
use chrono::prelude::*;
use compact_str::CompactString;
use dcso3::{
    coalition::Side, controller::PointType, env::miz::{Group, Miz, MizIndex, Skill, TriggerZone, TriggerZoneTyp}, MizLua, String, Vector2
};
use enumflags2::BitFlags;
use fxhash::FxHashSet;
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
            supply: 0,
            fuel: 0,
            last_change_ts: Utc::now(),
            last_threatened_ts: Utc::now(),
            warehouse: Warehouse::default(),
            last_cull: DateTime::<Utc>::default(),
        };
        if let ObjectiveKind::Logistics = obj.kind {
            self.persisted.logistics_hubs.insert_cow(id);
        }
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
            BitFlags::empty()
        )?;
        objective_mut!(self, obj)?
            .groups
            .get_or_default_cow(side)
            .insert_cow(gid);
        self.persisted.objectives_by_group.insert_cow(gid, obj);
        Ok(())
    }

    pub fn init_objective_slots(&mut self, side: Side, slot: Group) -> Result<()> {
        let mut ground_start = false;
        for point in slot.route()?.points()? {
            let point = point?;
            match point.typ {
                PointType::TakeOffGround | PointType::TakeOffGroundHot => ground_start = true,
                PointType::Land
                | PointType::TakeOff
                | PointType::Custom(_)
                | PointType::Nil
                | PointType::TakeOffParking
                | PointType::TurningPoint => (),
            }
        }
        for unit in slot.units()? {
            let unit = unit?;
            let vehicle = Vehicle::from(unit.typ()?);
            match self.ephemeral.cfg.threatened_distance.get(&vehicle) {
                Some(_) => (),
                None => bail!(
                    "vehicle {:?} doesn't have a configured theatened distance",
                    vehicle
                ),
            }
            if unit.skill()? != Skill::Client {
                continue;
            }
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
            objective_mut!(self, obj)?.slots.insert_cow(
                id,
                SlotInfo {
                    typ: vehicle,
                    ground_start,
                    miz_gid: slot.id()?,
                    side,
                },
            );
        }
        Ok(())
    }

    pub fn init(lua: MizLua, cfg: Cfg, idx: &MizIndex, miz: &Miz) -> Result<Self> {
        let spctx = SpawnCtx::new(lua)?;
        let mut t = Self::default();
        t.ephemeral.set_cfg(miz, idx, cfg)?;
        let mut objective_names = FxHashSet::default();
        for zone in miz.triggers()? {
            let zone = zone?;
            let name = zone.name()?;
            if name.starts_with('O') {
                if name.len() > 4 {
                    if !objective_names.insert(CompactString::from(&name[3..])) {
                        bail!("duplicate objective name {name}")
                    }
                } else {
                    bail!("malformed objective name {name}")
                }
                let name = name.strip_prefix("O").unwrap();
                t.init_objective(zone, name)?
            }
        }
        for side in Side::ALL {
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
                    t.init_objective_slots(side, plane)?
                }
                for heli in country.helicopters()? {
                    let heli = heli?;
                    t.init_objective_slots(side, heli)?
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
        t.init_warehouses(lua).context("initializing warehouses")?;
        t.ephemeral.dirty();
        Ok(t)
    }

    pub fn respawn_after_load(&mut self, idx: &MizIndex, spctx: &SpawnCtx) -> Result<()> {
        let mut spawn_deployed_and_logistics = || -> Result<()> {
            for gid in &self.persisted.deployed {
                self.ephemeral.push_spawn(*gid);
            }
            for gid in &self.persisted.crates {
                self.ephemeral.push_spawn(*gid);
            }
            for gid in &self.persisted.troops {
                self.ephemeral.push_spawn(*gid);
            }
            for (_, obj) in &self.persisted.objectives {
                if let ObjectiveKind::Farp {
                    spec: _,
                    pad_template,
                } = &obj.kind
                {
                    spctx
                        .move_farp_pad(idx, obj.owner, &pad_template, obj.pos)
                        .context("moving farp pad")?;
                }
                if let Some(groups) = obj.groups.get(&obj.owner) {
                    for gid in groups {
                        let group = group!(self, gid)?;
                        if obj.kind.is_farp()
                            || (group.class.is_services() && !obj.kind.is_airbase())
                        {
                            self.ephemeral.push_spawn(*gid)
                        }
                    }
                }
            }
            Ok(())
        };
        spawn_deployed_and_logistics().context("spawning deployed and logistics")?;
        self.setup_warehouses_after_load(spctx.lua())
            .context("setting up warehouses")?;
        let mut mark_deployed_and_logistics = || -> Result<()> {
            let groups = self
                .persisted
                .groups
                .into_iter()
                .map(|(gid, _)| *gid)
                .collect::<Vec<_>>();
            for gid in groups {
                self.mark_group(&gid)?
            }
            for (_, obj) in &self.persisted.objectives {
                self.ephemeral.create_objective_markup(obj, &self.persisted)
            }
            Ok(())
        };
        mark_deployed_and_logistics().context("marking deployed and logistics")?;
        let mut queue_check_walkabout_and_close_enemies = || -> Result<()> {
            for (uid, unit) in &self.persisted.units {
                let group = group!(self, unit.group)?;
                match group.origin {
                    DeployKind::Crate { .. } => (),
                    DeployKind::Deployed { .. } | DeployKind::Troop { .. } | DeployKind::Action { .. } => {
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
                        }
                    }
                }
            }
            Ok(())
        };
        // return lives to pilots who were airborne on the last restart
        let airborne_players = self
            .persisted
            .players
            .into_iter()
            .filter_map(|(ucid, p)| p.airborne.and_then(|lt| Some((ucid.clone(), lt))))
            .collect::<Vec<_>>();
        for (ucid, lt) in airborne_players {
            let player = &mut self.persisted.players[&ucid];
            player.airborne = None;
            if let Some((_, lives)) = player.lives.get_mut_cow(&lt) {
                *lives += 1;
                if *lives >= self.ephemeral.cfg.default_lives[&lt].0 {
                    player.lives.remove_cow(&lt);
                }
                self.ephemeral.dirty = true;
            }
        }
        queue_check_walkabout_and_close_enemies().context("queuing unit pos checks")?;
        self.cull_or_respawn_objectives(spctx.lua(), Utc::now())
            .context("initial cull or respawn")?;
        Ok(())
    }
}
