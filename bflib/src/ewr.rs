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

use crate::{
    db::{
        player::{InstancedPlayer, Player},
        Db,
    },
    landcache::LandCache,
};
use anyhow::Result;
use bfprotocols::stats::{DetectionSource, EnId, Stat};
use chrono::prelude::*;
use dcso3::{
    azumith2d_to, azumith3d, coalition::Side, land::Land, net::Ucid, radians_to_degrees, MizLua,
    Position3, Vector2, Vector3,
};
use fxhash::FxHashMap;
use smallvec::{smallvec, SmallVec};
use std::fmt;

#[derive(Debug, Clone, Copy)]
pub struct GibBraa {
    pub bearing: u16,
    pub range: u32,
    pub altitude: u32,
    pub heading: u16,
    pub speed: u16,
    pub age: u16,
    pub units: EwrUnits,
    converted: bool,
}

pub const HEADER: &'static str = "BRG      RNG      ALT      SPD        HDG      AGE";

impl fmt::Display for GibBraa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let (range_u, altitude_u, _u) = match self.units {
            EwrUnits::Imperial => ("nm", "ft", "kts "),
            EwrUnits::Metric => ("km", "m ", "km/h"),
        };
        write!(
            f,
            "{:>6} {:>6}{} {:>6}{} {:>6}{} {:>6} {:>6}s",
            self.bearing,
            self.range,
            range_u,
            self.altitude,
            altitude_u,
            self.,
            _u,
            self.heading,
            self.age
        )
    }
}

impl GibBraa {
    fn convert(&mut self, unit: EwrUnits) {
        if self.converted {
            return;
        }
        self.converted = true;
        match unit {
            EwrUnits::Metric => {
                self.range = self.range / 1000;
                // Round speed to nearest 100s in metric (km/h)
                self.speed = ((self.speed as f64 / 100.0).round() * 100.0) as u16;
                // Round altitude: under 1000m to nearest 100s, 1000m+ to nearest 1000s
                if self.altitude < 1000 {
                    self.altitude = ((self.altitude as f64 / 100.0).round() * 100.0) as u32;
                } else {
                    self.altitude = ((self.altitude as f64 / 1000.0).round() * 1000.0) as u32;
                }
            }
            EwrUnits::Imperial => {
                self.range = self.range / 1852;
                self.altitude = (self.altitude as f64 * 3.38084) as u32;
                // Round speed to nearest 100s in imperial (kts)
                self.speed = ((self.speed as f64 / 100.0).round() * 100.0) as u16;
                // Round altitude: under 1000ft to nearest 100s, 1000ft+ to nearest 1000s
                if self.altitude < 1000 {
                    self.altitude = ((self.altitude as f64 / 100.0).round() * 100.0) as u32;
                } else {
                    self.altitude = ((self.altitude as f64 / 1000.0).round() * 1000.0) as u32;
                }
            }
        }
        self.units = unit;
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct Track {
    pos: Position3,
    velocity: Vector3,
    last: DateTime<Utc>,
    side: Side,
    was_detected: bool,
    detected: bool,
}

#[derive(Debug, Clone, Copy)]
pub enum EwrUnits {
    Imperial,
    Metric,
}

impl Default for EwrUnits {
    fn default() -> Self {
        Self::Metric
    }
}

#[derive(Debug, Clone, Copy)]
struct PlayerState {
    enabled: bool,
    units: EwrUnits,
    last: DateTime<Utc>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            enabled: true,
            units: EwrUnits::default(),
            last: DateTime::default(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Ewr {
    tracks: FxHashMap<Side, FxHashMap<EnId, Track>>,
    player_state: FxHashMap<Ucid, PlayerState>,
}

impl Ewr {
    pub fn update_tracks(
        &mut self,
        lua: MizLua,
        landcache: &mut LandCache,
        db: &Db,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let land = Land::singleton(lua)?;
        let aircraft: SmallVec<[(EnId, Side, Position3, Vector3); 128]> = {
            let players = db
                .instanced_players()
                .filter(|(_, _, inst)| inst.in_air)
                .map(|(ucid, player, inst)| {
                    (
                        EnId::Player(*ucid),
                        player.side,
                        inst.position,
                        inst.velocity,
                    )
                });
            let actions = db
                .persisted
                .actions
                .into_iter()
                .filter_map(|gid| db.persisted.groups.get(gid))
                .flat_map(|sg| {
                    sg.units
                        .into_iter()
                        .filter_map(|uid| db.persisted.units.get(uid).map(|u| (*uid, u)))
                        .filter_map(|(uid, su)| {
                            su.airborne_velocity
                                .map(|v| (EnId::Unit(uid), sg.side, su.position, v))
                        })
                });
            players.chain(actions).collect()
        };
        for tracks in self.tracks.values_mut() {
            for track in tracks.values_mut() {
                track.detected = false;
            }
        }
        for (mut ewr_pos, ewr_side, ewr) in db.ewrs() {
            let range = (ewr.range as f64).powi(2);
            let tracks = self.tracks.entry(ewr_side).or_default();
            ewr_pos.y += 10.; // factor in antenna height
            for (id, obj_side, pos, velocity) in &aircraft {
                let track = tracks.entry(*id).or_default();
                if track.last != now {
                    let dist = na::distance_squared(&ewr_pos.into(), &pos.p.0.into());
                    if dist <= range {
                        if landcache.is_visible(&land, dist.sqrt(), ewr_pos, pos.p.0)? {
                            track.pos = *pos;
                            track.velocity = *velocity;
                            track.last = now;
                            track.side = *obj_side;
                            track.detected |= ewr_side != *obj_side;
                        }
                    }
                }
            }
        }
        for tracks in self.tracks.values_mut() {
            for (id, track) in tracks.iter_mut() {
                if track.was_detected != track.detected {
                    track.was_detected = track.detected;
                    db.ephemeral.stat(Stat::Detected {
                        id: *id,
                        detected: track.was_detected,
                        source: DetectionSource::EWR,
                    })
                }
            }
        }
        Ok(())
    }

    pub fn toggle(&mut self, ucid: &Ucid) -> bool {
        let st = self.player_state.entry(ucid.clone()).or_default();
        st.enabled = !st.enabled;
        st.enabled
    }

    pub fn set_units(&mut self, ucid: &Ucid, units: EwrUnits) {
        self.player_state.entry(ucid.clone()).or_default().units = units;
    }

    pub fn where_chicken(
        &mut self,
        now: DateTime<Utc>,
        friendly: bool,
        force: bool,
        ucid: &Ucid,
        player: &Player,
        inst: &InstancedPlayer,
    ) -> SmallVec<[GibBraa; 64]> {
        let side = player.side;
        let pos = Vector2::new(inst.position.p.x, inst.position.p.z);
        let mut reports: SmallVec<[GibBraa; 64]> = smallvec![];
        let tracks = match self.tracks.get_mut(&side) {
            Some(t) => t,
            None => return reports,
        };
        let state = self.player_state.entry(ucid.clone()).or_default();
        if !force && !state.enabled {
            return reports;
        }
        let ownship = EnId::Player(*ucid);
        tracks.retain(|tucid, track| {
            let age = (now - track.last).num_seconds();
            let include = (friendly && track.side == side) || (!friendly && track.side != side);
            if include && age <= 120 && tucid != &ownship {
                let cpos = Vector2::new(track.pos.p.x, track.pos.p.z);
                let range = na::distance(&pos.into(), &cpos.into());
                let bearing = radians_to_degrees(azumith2d_to(pos, cpos));
                let heading = radians_to_degrees(azumith3d(track.pos.x.0));
                let speed = track.velocity.magnitude();
                let altitude = track.pos.p.y;
                reports.push(GibBraa {
                    range: range as u32,
                    heading: heading as u16,
                    altitude: altitude as u32,
                    bearing: bearing as u16,
                    age: age as u16,
                    speed: speed as u16,
                    units: EwrUnits::Metric,
                    converted: false,
                })
            }
            age <= 120
        });
        if reports.is_empty() {
            return reports;
        }
        reports.sort_by_key(|r| r.range);
        while reports.len() > 10 {
            reports.pop();
        }
        let since_last = (now - state.last).num_seconds();
        if force
            || since_last >= 60
            || (reports[0].range <= 20000 && reports[0].age <= 10)
            || (reports[0].range <= 40000 && reports[0].age <= 10 && since_last >= 30)
        {
            state.last = now;
            reports.iter_mut().for_each(|r| r.convert(state.units));
            reports
        } else {
            smallvec![]
        }
    }
}
