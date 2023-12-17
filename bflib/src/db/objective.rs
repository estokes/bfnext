use anyhow::{Result, anyhow};
use dcso3::{coalition::Side, env::miz::MizIndex, MizLua};
use log::debug;
use crate::{group, unit, objective, objective_mut, spawnctx::SpawnCtx};
use super::{Db, Objective, ObjGroupClass, ObjectiveId, Set, GroupId, Map, UnitId};
use chrono::{prelude::*, Duration};

impl Db {
    fn compute_objective_status(&self, obj: &Objective) -> Result<(u8, u8)> {
        obj.groups
            .get(&obj.owner)
            .map(|groups| {
                let mut total = 0;
                let mut alive = 0;
                let mut logi_total = 0;
                let mut logi_alive = 0;
                for (_, gid) in groups {
                    let group = group!(self, gid)?;
                    let logi = match ObjGroupClass::from(group.template_name.as_str()) {
                        ObjGroupClass::Logi => true,
                        _ => false,
                    };
                    for uid in &group.units {
                        total += 1;
                        if logi {
                            logi_total += 1;
                        }
                        if !unit!(self, uid)?.dead {
                            alive += 1;
                            if logi {
                                logi_alive += 1;
                            }
                        }
                    }
                }
                let health = ((alive as f32 / total as f32) * 100.).trunc() as u8;
                let logi = ((logi_alive as f32 / logi_total as f32) * 100.).trunc() as u8;
                Ok((health, logi))
            })
            .unwrap_or(Ok((100, 100)))
    }

    pub(super) fn update_objective_status(&mut self, oid: &ObjectiveId, now: DateTime<Utc>) -> Result<()> {
        let obj = objective!(self, oid)?;
        let (health, logi) = self.compute_objective_status(obj)?;
        let obj = objective_mut!(self, oid)?;
        obj.health = health;
        obj.logi = logi;
        obj.last_change_ts = now;
        if obj.health == 0 {
            obj.owner = Side::Neutral;
        }
        debug!("objective {oid} health: {}, logi: {}", obj.health, obj.logi);
        Ok(())
    }

    fn repair_objective(
        &mut self,
        idx: &MizIndex,
        spctx: &SpawnCtx,
        oid: ObjectiveId,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let obj = self
            .persisted
            .objectives
            .get(&oid)
            .ok_or_else(|| anyhow!("no such objective {:?}", oid))?;
        if let Some(groups) = obj.groups.get(&obj.owner) {
            let damaged_by_class: Map<ObjGroupClass, Set<GroupId>> = groups.into_iter().fold(
                Ok(Map::new()),
                |m: Result<Map<ObjGroupClass, Set<GroupId>>>, (name, id)| {
                    let mut m = m?;
                    let class = ObjGroupClass::from(name.template());
                    let mut damaged = false;
                    for uid in &group!(self, id)?.units {
                        damaged |= unit!(self, uid)?.dead;
                    }
                    if damaged {
                        m.get_or_default_cow(class).insert_cow(*id);
                        Ok(m)
                    } else {
                        Ok(m)
                    }
                },
            )?;
            for class in [
                ObjGroupClass::Logi,
                ObjGroupClass::Sr,
                ObjGroupClass::Aaa,
                ObjGroupClass::Lr,
                ObjGroupClass::Armor,
                ObjGroupClass::Other,
            ] {
                if let Some(groups) = damaged_by_class.get(&class) {
                    for gid in groups {
                        let group = &self.persisted.groups[gid];
                        for uid in &group.units {
                            self.persisted.units[uid].dead = false;
                        }
                        self.respawn_group(idx, spctx, group)?;
                        self.update_objective_status(&oid, now)?;
                        self.ephemeral.dirty = true;
                        return Ok(());
                    }
                }
            }
        }
        Ok(())
    }

    pub fn maybe_do_repairs(
        &mut self,
        lua: MizLua,
        idx: &MizIndex,
        now: DateTime<Utc>,
    ) -> Result<()> {
        let spctx = SpawnCtx::new(lua)?;
        let to_repair = self
            .persisted
            .objectives
            .into_iter()
            .filter_map(|(oid, obj)| {
                let logi = obj.logi as f32 / 100.;
                let repair_time = self.ephemeral.cfg.repair_time as f32 / logi;
                if repair_time < i64::MAX as f32 {
                    let repair_time = Duration::seconds(repair_time as i64);
                    if obj.health < 100 && (now - obj.last_change_ts) >= repair_time {
                        Some(*oid)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for oid in to_repair {
            self.repair_objective(idx, &spctx, oid, now)?
        }
        Ok(())
    }

    pub fn unit_dead(&mut self, id: UnitId, dead: bool, now: DateTime<Utc>) -> Result<()> {
        if let Some(unit) = self.persisted.units.get_mut_cow(&id) {
            unit.dead = dead;
            if let Some(oid) = self.persisted.objectives_by_group.get(&unit.group).copied() {
                self.update_objective_status(&oid, now)?
            }
        }
        self.ephemeral.dirty = true;
        Ok(())
    }

}