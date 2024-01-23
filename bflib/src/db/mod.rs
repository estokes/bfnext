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

extern crate nalgebra as na;
use self::{
    group::{DeployKind, SpawnedGroup},
    persisted::Persisted,
};
use crate::{
    cfg::{Cfg, Deployable, DeployableEwr, DeployableJtac, Troop},
    db::ephemeral::Ephemeral,
};
use anyhow::{anyhow, Result};
use dcso3::{
    centroid3d,
    coalition::Side,
    env::miz::{Miz, MizIndex},
    Vector3,
};
use std::{fs::File, path::Path};

pub mod cargo;
pub mod ephemeral;
pub mod group;
pub mod logistics;
pub mod mizinit;
pub mod objective;
pub mod persisted;
pub mod player;
pub mod pmc;

pub type Map<K, V> = immutable_chunkmap::map::Map<K, V, 256>;
pub type Set<K> = immutable_chunkmap::set::Set<K, 256>;

#[macro_export]
macro_rules! maybe {
    ($t:expr, $id:expr, $name:expr) => {
        $t.get(&$id)
            .ok_or_else(|| anyhow!("no such {} {:?}", $name, $id))
    };
}

#[macro_export]
macro_rules! maybe_mut {
    ($t:expr, $id:expr, $name:expr) => {
        $t.get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such {} {:?}", $name, $id))
    };
}

#[macro_export]
macro_rules! unit {
    ($t:expr, $id:expr) => {
        $t.persisted
            .units
            .get(&$id)
            .ok_or_else(|| anyhow!("no such unit {:?}", $id))
    };
}

#[macro_export]
macro_rules! unit_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .units
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such unit {:?}", $id))
    };
}

#[macro_export]
macro_rules! unit_by_name {
    ($t:expr, $name:expr) => {
        $t.persisted
            .units_by_name
            .get($name)
            .and_then(|id| $t.persisted.units.get(id))
            .ok_or_else(|| anyhow!("no such unit {}", $name))
    };
}

#[macro_export]
macro_rules! group {
    ($t:expr, $id:expr) => {
        $t.persisted
            .groups
            .get(&$id)
            .ok_or_else(|| anyhow!("no such group {:?}", $id))
    };
}

#[macro_export]
macro_rules! group_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .groups
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such group {:?}", $id))
    };
}

#[macro_export]
macro_rules! group_by_name {
    ($t:expr, $name:expr) => {
        $t.persisted
            .groups_by_name
            .get($name)
            .and_then(|id| $t.persisted.groups.get(id))
            .ok_or_else(|| anyhow!("no such group {}", $name))
    };
}

#[macro_export]
macro_rules! objective {
    ($t:expr, $id:expr) => {
        $t.persisted
            .objectives
            .get(&$id)
            .ok_or_else(|| anyhow!("no such objective {:?}", $id))
    };
}

#[macro_export]
macro_rules! objective_mut {
    ($t:expr, $id:expr) => {
        $t.persisted
            .objectives
            .get_mut_cow(&$id)
            .ok_or_else(|| anyhow!("no such objective {:?}", $id))
    };
}

#[derive(Debug, Default)]
pub struct Db {
    pub persisted: Persisted,
    pub ephemeral: Ephemeral,
}

impl Db {
    pub fn load(miz: &Miz, idx: &MizIndex, path: &Path) -> Result<Self> {
        let file = File::open(&path)
            .map_err(|e| anyhow!("failed to open save file {:?}, {:?}", path, e))?;
        let persisted: Persisted = serde_json::from_reader(file)
            .map_err(|e| anyhow!("failed to decode save file {:?}, {:?}", path, e))?;
        let mut db = Db {
            persisted,
            ephemeral: Ephemeral::default(),
        };
        db.ephemeral.set_cfg(miz, idx, Cfg::load(path)?)?;
        Ok(db)
    }

    pub fn maybe_snapshot(&mut self) -> Option<Persisted> {
        if self.ephemeral.take_dirty() {
            Some(self.persisted.clone())
        } else {
            None
        }
    }

    pub fn ewrs(&self) -> impl Iterator<Item = (Vector3, Side, &DeployableEwr)> {
        self.persisted.ewrs.into_iter().filter_map(|gid| {
            let group = self.persisted.groups.get(gid)?;
            match &group.origin {
                DeployKind::Crate { .. } | DeployKind::Objective | DeployKind::Troop { .. } => None,
                DeployKind::Deployed { spec, .. } => {
                    let ewr = spec.ewr.as_ref()?;
                    let pos = centroid3d(
                        group
                            .units
                            .into_iter()
                            .map(|u| self.persisted.units[u].position.p.0),
                    );
                    Some((pos, group.side, ewr))
                }
            }
        })
    }

    pub fn jtacs(&self) -> impl Iterator<Item = (Vector3, &SpawnedGroup, &DeployableJtac)> {
        self.persisted.jtacs.into_iter().filter_map(|gid| {
            let group = self.persisted.groups.get(gid)?;
            match &group.origin {
                DeployKind::Troop {
                    spec: Troop {
                        jtac: Some(jtac), ..
                    },
                    ..
                }
                | DeployKind::Deployed {
                    spec:
                        Deployable {
                            jtac: Some(jtac), ..
                        },
                    ..
                } => {
                    let pos = centroid3d(
                        group
                            .units
                            .into_iter()
                            .map(|u| self.persisted.units[u].position.p.0),
                    );
                    Some((pos, group, jtac))
                }
                DeployKind::Crate { .. }
                | DeployKind::Objective
                | DeployKind::Troop { .. }
                | DeployKind::Deployed { .. } => None,
            }
        })
    }
}
