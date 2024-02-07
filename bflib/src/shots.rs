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

/// Lets not bicker and argue about who killed who
use anyhow::Result;
use chrono::{prelude::*, Duration};
use dcso3::{
    event::{Shot as ShotEvent, WeaponUse},
    net::Ucid,
    object::{DcsObject, DcsOid},
    unit::ClassUnit,
    weapon::ClassWeapon,
    String,
};
use fxhash::{FxHashMap, FxHashSet};
use smallvec::SmallVec;
use crate::db::Db;

#[derive(Debug, Clone)]
pub struct Dead {
    pub victim: DcsOid<ClassUnit>,
    pub victim_ucid: Option<Ucid>,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Clone)]
pub struct Shot {
    pub weapon_name: Option<String>,
    pub weapon: Option<DcsOid<ClassWeapon>>,
    pub shooter: DcsOid<ClassUnit>,
    pub shooter_ucid: Ucid,
    pub target: DcsOid<ClassUnit>,
    pub target_ucid: Option<Ucid>,
    pub time: DateTime<Utc>,
    pub hit: bool, // is this an actual hit
}

#[derive(Debug, Clone, Default)]
pub struct ShotDb {
    by_target: FxHashMap<DcsOid<ClassUnit>, SmallVec<[Shot; 8]>>,
    dead: FxHashSet<DcsOid<ClassUnit>>,
    last_gc: DateTime<Utc>,
}

macro_rules! ok {
    ($r:expr) => {
        match $r {
            Ok(u) => u,
            Err(_) => return Ok(()),
        }
    };
}

macro_rules! some {
    ($o:expr) => {
        match $o {
            Some(u) => u,
            None => return Ok(()),
        }
    };
}

impl ShotDb {
    pub fn dead(&mut self, target: DcsOid<ClassUnit>) {
        self.dead.insert(target);
    }

    pub fn shot(&mut self, db: &Db, now: DateTime<Utc>, e: ShotEvent) -> Result<()> {
        let target = ok!(some!(e.weapon.get_target()?).as_unit()).object_id()?;
        let shooter = e.initiator.object_id()?;
        let shooter_ucid = some!(db.player_in_unit(true, &shooter));
        let shot = Shot {
            weapon_name: Some(e.weapon_name),
            weapon: Some(e.weapon.object_id()?),
            shooter: shooter.clone(),
            shooter_ucid,
            target: target.clone(),
            target_ucid: db.player_in_unit(false, &target),
            time: now,
            hit: false,
        };
        self.by_target.entry(target).or_default().push(shot);
        Ok(())
    }

    pub fn hit(&mut self, db: &Db, now: DateTime<Utc>, e: WeaponUse) -> Result<()> {
        let target = ok!(some!(e.target).as_unit());
        let target_oid = target.object_id()?;
        if self.dead.contains(&target_oid) {
            return Ok(());
        }
        let shooter = ok!(some!(e.initiator).as_unit()).object_id()?;
        let shooter_ucid = some!(db.player_in_unit(true, &shooter));
        let dead = target.get_life()? < 1;
        let target = target_oid;
        let shot = Shot {
            weapon_name: Some(e.weapon_name),
            weapon: None,
            shooter: shooter.clone(),
            shooter_ucid,
            target: target.clone(),
            target_ucid: db.player_in_unit(false, &target),
            time: now,
            hit: true,
        };
        self.by_target.entry(target.clone()).or_default().push(shot);
        if dead {
            self.dead.insert(target);
        }
        Ok(())
    }

    pub fn bring_out_your_dead(&mut self, now: DateTime<Utc>) -> Vec<Dead> {
        let mut dead = Vec::with_capacity(self.dead.len());
        for target in self.dead.drain() {
            dead.push(Dead {
                victim: target.clone(),
                victim_ucid: None,
                shots: vec![],
            });
            let kill = dead.last_mut().unwrap();
            if let Some(shots) = self.by_target.remove(&target) {
                for shot in shots {
                    if kill.victim_ucid.is_none() {
                        if let Some(ucid) = shot.target_ucid.clone() {
                            kill.victim_ucid = Some(ucid);
                        }
                    }
                    kill.shots.push(shot);
                }
            }
        }
        if now - self.last_gc >= Duration::minutes(30) {
            self.last_gc = now;
            self.by_target.retain(|_, shots| {
                shots.retain(|shot| now - shot.time <= Duration::minutes(30));
                !shots.is_empty()
            });
        }
        dead
    }
}
