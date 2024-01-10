extern crate nalgebra as na;
use self::{
    group::{DeployKind, SpawnedGroup},
    persisted::Persisted,
};
use crate::{
    cfg::{Cfg, Deployable, DeployableEwr, DeployableJtac, Troop},
    db::ephemeral::Ephemeral,
    spawnctx::SpawnCtx,
};
use anyhow::{anyhow, Result};
use chrono::prelude::*;
use dcso3::{
    centroid3d,
    coalition::Side,
    env::miz::{Miz, MizIndex},
    object::DcsOid,
    unit::ClassUnit,
    Vector3,
};
use std::{cmp::max, fs::File, path::Path};

pub mod cargo;
pub mod ephemeral;
pub mod group;
pub mod logistics;
pub mod mizinit;
pub mod objective;
pub mod persisted;
pub mod player;

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
        $t.get_mut(&$id)
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

    pub fn jtacs(
        &self,
    ) -> impl Iterator<Item = (Vector3, &DcsOid<ClassUnit>, &SpawnedGroup, &DeployableJtac)> {
        self.persisted.jtacs.into_iter().filter_map(|gid| {
            let group = self.persisted.groups.get(gid)?;
            let id = group
                .units
                .into_iter()
                .find_map(|gid| self.ephemeral.object_id_by_uid.get(gid))?;
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
                    Some((pos, id, group, jtac))
                }
                DeployKind::Crate { .. }
                | DeployKind::Objective
                | DeployKind::Troop { .. }
                | DeployKind::Deployed { .. } => None,
            }
        })
    }

    pub fn process_spawn_queue(
        &mut self,
        now: DateTime<Utc>,
        idx: &MizIndex,
        spctx: &SpawnCtx,
    ) -> Result<()> {
        while let Some((at, gids)) = self.ephemeral.delayspawnq.first_key_value() {
            if now < *at {
                break;
            } else {
                for gid in gids {
                    self.ephemeral.spawnq.push_back(*gid);
                }
                let at = *at;
                self.ephemeral.delayspawnq.remove(&at);
            }
        }
        let dlen = self.ephemeral.despawnq.len();
        let slen = self.ephemeral.spawnq.len();
        if dlen > 0 {
            for _ in 0..max(2, dlen >> 2) {
                if let Some((gid, name)) = self.ephemeral.despawnq.pop_front() {
                    if let Some(group) = self.persisted.groups.get(&gid) {
                        for uid in &group.units {
                            self.ephemeral.units_able_to_move.remove(uid);
                            self.ephemeral
                                .units_potentially_close_to_enemies
                                .remove(uid);
                            self.ephemeral.units_potentially_on_walkabout.remove(uid);
                            if let Some(id) = self.ephemeral.object_id_by_uid.remove(uid) {
                                self.ephemeral.uid_by_object_id.remove(&id);
                            }
                        }
                    }
                    spctx.despawn(name)?
                }
            }
        } else if slen > 0 {
            for _ in 0..max(2, slen >> 2) {
                if let Some(gid) = self.ephemeral.spawnq.pop_front() {
                    self.spawn_group(idx, spctx, group!(self, gid)?)?
                }
            }
        }
        Ok(())
    }
}
