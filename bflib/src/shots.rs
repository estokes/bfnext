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

//! Lets not bicker and argue about oo killed oo
use crate::db::{Db, group::DeployKind};
use anyhow::Result;
use bfprotocols::shots::{Dead, Shot, Who};
use chrono::{Duration, prelude::*};
use dcso3::{
    String,
    event::Shot as ShotEvent,
    object::{DcsObject, DcsOid},
    unit::{ClassUnit, Unit},
};
use fxhash::FxHashMap;
use std::collections::hash_map::Entry;

#[derive(Debug, Clone, Default)]
pub struct ShotDb {
    by_target: FxHashMap<DcsOid<ClassUnit>, Vec<Shot>>,
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

fn who(db: &Db, id: DcsOid<ClassUnit>) -> Option<Who> {
    match db.ephemeral.get_uid_by_object_id(&id) {
        Some(uid) => db.unit(uid).ok().map(|u| Who::AI {
            side: u.side,
            gid: u.group,
            uid: *uid,
            unit: id,
            ucid: db.group(&u.group).ok().and_then(|g| match &g.origin {
                DeployKind::Action { player, .. } => *player,
                DeployKind::Deployed { player, .. } => Some(*player),
                DeployKind::Troop { player, .. } => Some(*player),
                DeployKind::Crate { .. } | DeployKind::Objective(_) => None,
            }),
        }),
        None => db
            .ephemeral
            .get_slot_by_object_id(&id)
            .and_then(|sl| db.ephemeral.player_in_slot(sl).map(|ucid| (sl, ucid)))
            .and_then(|(sl, ucid)| db.player(ucid).map(|p| (sl, ucid, p)))
            .map(|(sl, ucid, p)| Who::Player {
                side: p.side,
                slot: *sl,
                ucid: *ucid,
                unit: id,
            }),
    }
}

impl ShotDb {
    pub fn dead(&mut self, target: DcsOid<ClassUnit>, time: DateTime<Utc>) {
        if let Entry::Vacant(e) = self.dead.entry(target) {
            e.insert(time);
        }
    }

    pub fn shot(&mut self, db: &Db, now: DateTime<Utc>, e: &ShotEvent) -> Result<()> {
        let target = ok!(some!(e.weapon.get_target()?).as_unit());
        let target_oid = target.object_id()?;
        if self.dead.contains_key(&target_oid) || self.recently_dead.contains_key(&target_oid) {
            return Ok(());
        }
        let shooter = some!(who(db, e.initiator.object_id()?));
        let target_typ = target.get_type_name()?;
        let target = some!(who(db, target_oid.clone()));
        self.by_target.entry(target_oid).or_default().push(Shot {
            weapon_name: Some(e.weapon_name.clone()),
            weapon: Some(e.weapon.object_id()?),
            shooter,
            target,
            target_typ,
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
        let shooter = some!(who(db, shooter.object_id()?));
        let target = some!(who(db, target_oid.clone()));
        self.by_target
            .entry(target_oid.clone())
            .or_default()
            .push(Shot {
                weapon_name: Some(weapon_name),
                weapon: None,
                shooter,
                target,
                target_typ,
                time: now,
                hit: true,
            });
        if dead {
            self.dead.insert(target_oid, now);
        }
        Ok(())
    }

    pub fn bring_out_your_dead(&mut self, now: DateTime<Utc>) -> Vec<Dead> {
        let mut dead = Vec::with_capacity(self.dead.len());
        for (target, time) in self.dead.drain() {
            if let Some(shots) = self.by_target.remove(&target) {
                if shots.len() > 0 {
                    let victim = shots[0].target.clone();
                    dead.push(Dead {
                        victim,
                        time,
                        shots,
                    });
                }
            }
            self.recently_dead.insert(target, time);
        }
        const FIVE_MIN: Duration = Duration::minutes(5);
        const THIRTY_MIN: Duration = Duration::minutes(30);
        self.recently_dead.retain(|_, t| now - *t <= FIVE_MIN);
        if now - self.last_gc >= THIRTY_MIN {
            self.last_gc = now;
            self.by_target.retain(|_, shots| {
                shots.retain(|shot| now - shot.time <= THIRTY_MIN);
                !shots.is_empty()
            });
        }
        dead
    }
}
