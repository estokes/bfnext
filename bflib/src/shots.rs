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

use std::collections::hash_map::Entry;

/// Lets not bicker and argue about oo killed oo
use crate::db::Db;
use anyhow::Result;
use chrono::{prelude::*, Duration};
use dcso3::{
    coalition::Side,
    event::Shot as ShotEvent,
    net::Ucid,
    object::{DcsObject, DcsOid},
    unit::{ClassUnit, Unit},
    weapon::ClassWeapon,
    String,
};
use fxhash::FxHashMap;
use smallvec::SmallVec;

#[derive(Debug, Clone)]
pub struct Dead {
    pub victim: DcsOid<ClassUnit>,
    pub victim_ucid: Option<Ucid>,
    pub victim_side: Side,
    pub time: DateTime<Utc>,
    pub shots: Vec<Shot>,
}

#[derive(Debug, Clone)]
pub struct Shot {
    pub weapon_name: Option<String>,
    pub weapon: Option<DcsOid<ClassWeapon>>,
    pub shooter: DcsOid<ClassUnit>,
    pub shooter_ucid: Ucid,
    pub target: DcsOid<ClassUnit>,
    pub target_side: Side,
    pub target_ucid: Option<Ucid>,
    pub target_typ: String,
    pub time: DateTime<Utc>,
    pub hit: bool,
}

#[derive(Debug, Clone, Default)]
pub struct ShotDb {
    by_target: FxHashMap<DcsOid<ClassUnit>, SmallVec<[Shot; 8]>>,
    dead: FxHashMap<DcsOid<ClassUnit>, DateTime<Utc>>,
    recently_dead: FxHashMap<DcsOid<ClassUnit>, DateTime<Utc>>,
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
    pub fn dead(&mut self, target: DcsOid<ClassUnit>, time: DateTime<Utc>) {
        if let Entry::Vacant(e) = self.dead.entry(target) {
            e.insert(time);
        }
    }

    pub fn shot(&mut self, db: &Db, now: DateTime<Utc>, e: ShotEvent) -> Result<()> {
        let target = ok!(some!(e.weapon.get_target()?).as_unit());
        let target_oid = target.object_id()?;
        if self.dead.contains_key(&target_oid) || self.recently_dead.contains_key(&target_oid) {
            return Ok(());
        }
        let shooter = e.initiator.object_id()?;
        let shooter_ucid = some!(db.player_in_unit(true, &shooter));
        let target_typ = target.get_type_name()?;
        let target = target_oid;
        let target_side = db
            .ephemeral
            .get_uid_by_object_id(&target)
            .and_then(|uid| db.unit(uid).ok())
            .map(|u| u.side)
            .unwrap_or(Side::Neutral);
        self.by_target
            .entry(target.clone())
            .or_default()
            .push(Shot {
                weapon_name: Some(e.weapon_name),
                weapon: Some(e.weapon.object_id()?),
                shooter: shooter.clone(),
                shooter_ucid,
                target_ucid: db.player_in_unit(false, &target),
                target,
                target_typ,
                target_side,
                time: now,
                hit: false,
            });
        Ok(())
    }

    pub fn hit(
        &mut self,
        db: &Db,
        now: DateTime<Utc>,
        dead: bool,
        target: &Unit,
        shooter: &Unit,
        weapon_name: String,
    ) -> Result<()> {
        let target_oid = target.object_id()?;
        if self.dead.contains_key(&target_oid) || self.recently_dead.contains_key(&target_oid) {
            return Ok(());
        }
        let target_typ = target.get_type_name()?;
        let shooter = shooter.object_id()?;
        let shooter_ucid = some!(db.player_in_unit(true, &shooter));
        let target_side = db
            .ephemeral
            .get_uid_by_object_id(&target_oid)
            .and_then(|uid| db.unit(uid).ok())
            .map(|u| u.side)
            .unwrap_or(Side::Neutral);
        let target = target_oid;
        self.by_target
            .entry(target.clone())
            .or_default()
            .push(Shot {
                weapon_name: Some(weapon_name),
                weapon: None,
                shooter: shooter.clone(),
                shooter_ucid,
                target: target.clone(),
                target_ucid: db.player_in_unit(false, &target),
                target_typ,
                target_side,
                time: now,
                hit: true,
            });
        if dead {
            self.dead.insert(target, now);
        }
        Ok(())
    }

    pub fn bring_out_your_dead(&mut self, now: DateTime<Utc>) -> Vec<Dead> {
        let mut dead = Vec::with_capacity(self.dead.len());
        for (target, time) in self.dead.drain() {
            dead.push(Dead {
                victim: target.clone(),
                victim_ucid: None,
                victim_side: Side::Neutral,
                time,
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
                    kill.victim_side = shot.target_side;
                    kill.shots.push(shot);
                }
            }
            self.recently_dead.insert(target, time);
        }
        let five_min = Duration::minutes(5);
        self.recently_dead.retain(|_, t| now - *t <= five_min);
        let thirty_min = Duration::minutes(30);
        if now - self.last_gc >= thirty_min {
            self.last_gc = now;
            self.by_target.retain(|_, shots| {
                shots.retain(|shot| now - shot.time <= thirty_min);
                !shots.is_empty()
            });
        }
        dead
    }
}
