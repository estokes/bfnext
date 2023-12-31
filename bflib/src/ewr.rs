use std::fmt;

use crate::db::{Db, InstancedPlayer, Player};
use anyhow::Result;
use chrono::prelude::*;
use dcso3::{
    coalition::Side, land::Land, net::Ucid, LuaVec2, LuaVec3, MizLua, Position3, Vector2, Vector3,
};
use fxhash::FxHashMap;
use smallvec::{smallvec, SmallVec};

#[derive(Debug, Clone, Copy)]
pub struct GibBraa {
    pub bearing: u16,
    pub range: u32,
    pub altitude: u32,
    pub heading: u16,
    pub age: u16,
    pub units: EwrUnits,
}

impl fmt::Display for GibBraa {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{:03} {:03} {:>5} {:03} {:03}s",
            self.bearing, self.range, self.altitude, self.heading, self.age
        )
    }
}

impl GibBraa {
    fn convert(&mut self, unit: EwrUnits) {
        match unit {
            EwrUnits::Metric => (),
            EwrUnits::Imperial => {
                self.range = self.range / 1852;
                self.altitude = (self.altitude as f64 * 3.38084) as u32;
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
    tracks: FxHashMap<Side, FxHashMap<Ucid, Track>>,
    player_state: FxHashMap<Ucid, PlayerState>,
}

impl Ewr {
    pub fn update_tracks(&mut self, lua: MizLua, db: &Db, now: DateTime<Utc>) -> Result<()> {
        let land = Land::singleton(lua)?;
        let players: SmallVec<[_; 64]> = db.airborne_players().collect();
        for (ewr_pos, side, ewr) in db.ewrs() {
            let ewr_pos3 = LuaVec3(Vector3::new(
                ewr_pos.x,
                land.get_height(LuaVec2(ewr_pos))?,
                ewr_pos.y,
            ));
            let range = (ewr.range as f64).powi(2);
            let tracks = self.tracks.entry(side).or_default();
            for (ucid, player, inst) in &players {
                let track = tracks.entry((*ucid).clone()).or_default();
                if track.last != now {
                    let player_pos = Vector2::new(inst.position.p.x, inst.position.p.z);
                    let dist = na::distance_squared(&ewr_pos.into(), &player_pos.into());
                    if dist <= range && land.is_visible(ewr_pos3, inst.position.p)? {
                        track.pos = inst.position;
                        track.velocity = inst.velocity;
                        track.last = now;
                        track.side = player.side;
                    }
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
        ucid: &Ucid,
        player: &Player,
        inst: &InstancedPlayer,
    ) -> SmallVec<[GibBraa; 64]> {
        let side = player.side;
        let pos = Vector2::new(inst.position.p.x, inst.position.p.z);
        let mut reports: SmallVec<[GibBraa; 64]> = smallvec![];
        let tracks = match self.tracks.get(&side) {
            Some(t) => t,
            None => return smallvec![],
        };
        let state = self.player_state.entry(ucid.clone()).or_default();
        if !state.enabled {
            return reports;
        }
        for track in tracks.values() {
            let age = (now - track.last).num_seconds();
            let include = (friendly && track.side == side) || (!friendly && track.side != side);
            if age <= 120 && include {
                let cpos = Vector2::new(track.pos.p.x, track.pos.p.z);
                let range = na::distance(&pos.into(), &cpos.into());
                let v = cpos - pos;
                let bearing = v.y.atan2(v.x);
                let heading = cpos.y.atan2(cpos.x);
                let altitude = track.pos.p.y / 1000.;
                reports.push(GibBraa {
                    range: range as u32,
                    heading: heading as u16,
                    altitude: altitude as u32,
                    bearing: bearing as u16,
                    age: age as u16,
                    units: EwrUnits::Metric,
                })
            }
        }
        if reports.is_empty() {
            return reports;
        }
        reports.sort_by_key(|r| r.range);
        while reports.len() > 10 {
            reports.pop();
        }
        let since_last = (now - state.last).num_seconds();
        if since_last >= 60
            || reports[0].range <= 20000
            || (reports[0].range <= 40000 && since_last >= 30)
        {
            state.last = now;
            reports.iter_mut().for_each(|r| r.convert(state.units));
            reports
        } else {
            smallvec![]
        }
    }
}
