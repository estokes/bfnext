use crate::db_id;
use anyhow::{anyhow, bail, Result};
use arrayvec::ArrayVec;
use bfprotocols::{
    cfg::{Cfg, LifeType, UnitTags, Vehicle},
    db::{
        group::{GroupId, UnitId},
        objective::{ObjectiveId, ObjectiveKind},
    },
    perf::PerfInner,
    shots::Dead,
    stats::{DetectionSource, EnId, Pos, SeqId, Stat, StatKind},
};
use chrono::prelude::*;
use dcso3::{
    coalition::Side,
    coord::LLPos,
    net::{SlotId, Ucid},
    perf::{HistogramSer, PerfInner as ApiPerfInner},
    warehouse::LiquidType,
    String,
};
use enumflags2::BitFlags;
use serde::{Deserialize, Serialize};
use sled::{
    transaction::{ConflictableTransactionError, TransactionError},
    Db,
};
use sled_typed::{transaction::Transactional, Prefix, Tree};
use smallvec::SmallVec;

db_id!(KillId);
db_id!(RoundId);
db_id!(SortieId);

// lives: SmallVec<[(LifeType, DateTime<Utc>, u8); 6]>,

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Aggregates {
    air_kills: u32,
    ground_kills: u32,
    captures: u32,
    repairs: u32,
    supply_transfers: u32,
    troops: u32,
    farps: u32,
    deploys: u32,
    actions: u32,
    deaths: u32,
    hours: f32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pilot {
    name: ArrayVec<String, 8>,
    total: Aggregates,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sortie {
    vehicle: Vehicle,
    pos: Pos,
    takeoff: DateTime<Utc>,
    land: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Unit {
    group: GroupId,
    typ: Vehicle,
    tags: UnitTags,
    pos: Pos,
    dead: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Slot {
    id: SlotId,
    vehicle: Option<Vehicle>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Objective {
    name: String,
    pos: LLPos,
    kind: ObjectiveKind,
    by: Option<Ucid>,
    owner: Side,
    last_change: DateTime<Utc>,
    health: u8,
    logi: u8,
    supply: u8,
    fuel: u8,
}

struct Pilots {
    db: Db,
    pilots: Tree<Ucid, Pilot>,
    aggregates: Tree<(Ucid, Vehicle, RoundId), Aggregates>,
    by_name: Tree<String, Ucid>,
    side: Tree<(Ucid, RoundId), (DateTime<Utc>, Side)>,
    slot: Tree<(Ucid, RoundId), (DateTime<Utc>, Slot)>,
    sortie: Tree<(Ucid, RoundId, SortieId), Sortie>,
    lives: Tree<(Ucid, RoundId), SmallVec<[(LifeType, DateTime<Utc>, u8); 5]>>,
}

impl Pilots {
    fn new(db: &Db) -> Result<Self> {
        Ok(Self {
            db: db.clone(),
            pilots: Tree::open(db, "pilots")?,
            aggregates: Tree::open(db, "aggregates")?,
            by_name: Tree::open(db, "by_name")?,
            side: Tree::open(db, "side")?,
            slot: Tree::open(db, "slot")?,
            sortie: Tree::open(db, "sortie")?,
            lives: Tree::open(db, "lives")?,
        })
    }

    fn with_pilot<F: FnMut(&mut Pilot)>(&self, k: Ucid, mut f: F) -> Result<()> {
        self.pilots
            .fetch_and_update(&k, |o| match o {
                None => None,
                Some(mut p) => {
                    f(&mut p);
                    Some(p)
                }
            })?
            .ok_or_else(|| anyhow!("pilot {k:?} is missing"))?;
        Ok(())
    }

    fn with_aggregates<F: FnMut(&mut Aggregates)>(
        &self,
        k: (Ucid, Vehicle, RoundId),
        mut f: F,
    ) -> Result<()> {
        self.aggregates
            .fetch_and_update(&k, |a| match a {
                None => None,
                Some(mut a) => {
                    f(&mut a);
                    Some(a)
                }
            })?
            .ok_or_else(|| anyhow!("aggregates {k:?} is missing"))?;
        Ok(())
    }

    fn with_pilot_and_aggregates<F, G>(&self, ucid: Ucid, round: RoundId, f: F, g: G) -> Result<()>
    where
        F: FnMut(&mut Pilot),
        G: FnMut(&mut Aggregates),
    {
        let vehicle = self.slot.get(&(ucid, round))?.and_then(|(_, s)| s.vehicle);
        self.with_pilot(ucid, f)?;
        if let Some(vehicle) = vehicle {
            self.with_aggregates((ucid, vehicle, round), g)?
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
struct Round {
    start: DateTime<Utc>,
    end: Option<DateTime<Utc>>,
    winner: Option<Side>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct SessionEnd {
    time: DateTime<Utc>,
    frame: HistogramSer,
    api: ApiPerfInner,
    engine: PerfInner,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Session {
    stop_time: Option<DateTime<Utc>>,
    end: Option<SessionEnd>,
    cfg: Cfg,
}

type Scenario = String;

#[derive(Debug, Clone, Serialize, Deserialize)]
enum GroupKind {
    Deployed { name: String, by: Ucid },
    Troop { name: String, by: Ucid },
    Action { name: String, by: Ucid },
    Objective,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Group {
    units: SmallVec<[UnitId; 16]>,
    kind: GroupKind,
}

#[derive(Debug, Clone)]
struct StatCtxInner {
    sortie: String,
    seq: SeqId,
    round: RoundId,
}

#[derive(Debug, Clone, Default)]
struct StatCtx(Option<StatCtxInner>);

impl StatCtx {
    fn get(&self) -> Result<&StatCtxInner> {
        match &self.0 {
            Some(t) => Ok(t),
            None => bail!("expected to see NewSession before stats"),
        }
    }

    fn get_mut(&mut self) -> Result<&mut StatCtxInner> {
        match &mut self.0 {
            Some(t) => Ok(t),
            None => bail!("expected to see NewSession before stats"),
        }
    }
}

struct StatsDb {
    db: Db,
    pilots: Pilots,
    seq: Tree<(Scenario, RoundId), SeqId>,
    round: Tree<(Scenario, RoundId), Round>,
    session: Tree<(RoundId, DateTime<Utc>), Session>,
    kills: Tree<(EnId, RoundId, SortieId, KillId), Dead>,
    units: Tree<(RoundId, UnitId), Unit>,
    groups: Tree<(RoundId, GroupId), Group>,
    detected: Tree<(RoundId, Side, EnId), BitFlags<DetectionSource>>,
    objectives: Tree<(RoundId, ObjectiveId), Objective>,
    equipment: Tree<(RoundId, ObjectiveId, String), u32>,
    liquids: Tree<(RoundId, ObjectiveId, LiquidType), u32>,
}

macro_rules! abort {
    ($e:expr) => {
        return Err(ConflictableTransactionError::Abort(anyhow!($e)))
    };
}
fn txn_err(e: TransactionError<anyhow::Error>) -> anyhow::Error {
    match e {
        TransactionError::Abort(e) => e,
        TransactionError::Storage(e) => e.into(),
    }
}

impl StatsDb {
    fn new(db: &Db) -> Result<Self> {
        Ok(Self {
            db: db.clone(),
            pilots: Pilots::new(db)?,
            seq: Tree::open(db, "seq")?,
            round: Tree::open(db, "round")?,
            session: Tree::open(db, "session")?,
            kills: Tree::open(db, "kills")?,
            units: Tree::open(db, "units")?,
            groups: Tree::open(db, "groups")?,
            detected: Tree::open(db, "detected")?,
            objectives: Tree::open(db, "objectives")?,
            equipment: Tree::open(db, "equipment")?,
            liquids: Tree::open(db, "liquids")?,
        })
    }

    fn new_round(
        &self,
        ctx: &mut StatCtx,
        start: DateTime<Utc>,
        sortie: String,
        seqnum: SeqId,
    ) -> Result<()> {
        let id = RoundId::new(&self.db)?;
        let key = (sortie.clone(), id);
        let r = Round {
            start,
            end: None,
            winner: None,
        };
        self.seq.insert(&key, &seqnum)?;
        self.round.insert(&key, &r)?;
        ctx.0 = Some(StatCtxInner {
            sortie,
            round: id,
            seq: seqnum,
        });
        Ok(())
    }

    fn round_end(
        &self,
        ctx: &mut StatCtx,
        time: DateTime<Utc>,
        winner: Option<Side>,
    ) -> Result<()> {
        let inner = ctx.get_mut()?;
        let key = (inner.sortie.clone(), inner.round);
        let mut round = self
            .round
            .get(&key)?
            .ok_or_else(|| anyhow!("round not found"))?;
        round.end = Some(time);
        round.winner = winner;
        let _ = self.round.insert(&key, &round)?;
        ctx.0 = None;
        Ok(())
    }

    fn with_objective<F: FnMut(&mut Objective)>(
        &self,
        k: (RoundId, ObjectiveId),
        mut f: F,
    ) -> Result<()> {
        self.objectives
            .fetch_and_update(&k, |o| match o {
                None => None,
                Some(mut o) => {
                    f(&mut o);
                    Some(o)
                }
            })?
            .ok_or_else(|| anyhow!("objective {k:?} is missing"))?;
        Ok(())
    }

    fn with_group<F: FnMut(&mut Group)>(&self, k: (RoundId, GroupId), mut f: F) -> Result<()> {
        self.groups
            .fetch_and_update(&k, |g| match g {
                None => None,
                Some(mut g) => {
                    f(&mut g);
                    Some(g)
                }
            })?
            .ok_or_else(|| anyhow!("group {k:?} is missing"))?;
        Ok(())
    }

    pub fn add_stat(&self, ctx: &mut StatCtx, stat: Stat) -> Result<()> {
        if let Some(ctx) = &ctx.0 {
            if stat.seq <= ctx.seq {
                return Ok(());
            }
        }
        let ctx = match stat.kind {
            StatKind::NewRound { sortie } => {
                if ctx.0.is_some() {
                    bail!("NewRound should only appear at the beginning of the stats or after RoundEnd")
                }
                match self.seq.scan_prefix(&sortie)?.next_back().transpose()? {
                    None => return self.new_round(ctx, stat.time, sortie.clone(), stat.seq),
                    Some(((_, round), seq)) => match self.round.get(&(sortie.clone(), round))? {
                        Some(r) if r.end.is_none() => {
                            ctx.0 = Some(StatCtxInner {
                                round,
                                seq,
                                sortie: sortie.clone(),
                            });
                            return Ok(());
                        }
                        Some(_) | None => {
                            return self.new_round(ctx, stat.time, sortie.clone(), stat.seq)
                        }
                    },
                }
            }
            StatKind::RoundEnd { winner } => return self.round_end(ctx, stat.time, winner),
            StatKind::SessionStart { stop, cfg } => {
                let ctx = ctx.get_mut()?;
                self.session.insert(
                    &(ctx.round, stat.time),
                    &Session {
                        cfg: (*cfg).clone(),
                        stop_time: stop,
                        end: None,
                    },
                )?;
                ctx
            }
            StatKind::SessionEnd {
                api_perf,
                perf,
                frame,
            } => {
                let ctx = ctx.get_mut()?;
                match self
                    .session
                    .scan_prefix(&ctx.round)?
                    .next_back()
                    .transpose()?
                {
                    None => bail!("no session for {} is in progress", &ctx.sortie),
                    Some((k, mut session)) => {
                        session.end = Some(SessionEnd {
                            api: api_perf,
                            engine: perf,
                            frame,
                            time: stat.time,
                        });
                        self.session.insert(&k, &session)?;
                        ctx
                    }
                }
            }
            StatKind::Objective {
                name,
                id,
                pos,
                owner,
                kind,
            } => {
                let ctx = ctx.get_mut()?;
                self.objectives.insert(
                    &(ctx.round, id),
                    &Objective {
                        name,
                        pos,
                        kind,
                        owner,
                        by: None,
                        last_change: stat.time,
                        health: 100,
                        logi: 100,
                        supply: 100,
                        fuel: 100,
                    },
                )?;
                ctx
            }
            StatKind::ObjectiveDestroyed { id } => {
                let ctx = ctx.get_mut()?;
                self.objectives.remove(&(ctx.round, id))?;
                ctx
            }
            StatKind::ObjectiveHealth {
                id,
                last_change,
                health,
                logi,
            } => {
                let ctx = ctx.get_mut()?;
                self.with_objective((ctx.round, id), |o| {
                    o.last_change = last_change;
                    o.health = health;
                    o.logi = logi
                })?;
                ctx
            }
            StatKind::ObjectiveSupply { id, supply, fuel } => {
                let ctx = ctx.get_mut()?;
                self.with_objective((ctx.round, id), |o| {
                    o.supply = supply;
                    o.logi = fuel
                })?;
                ctx
            }
            StatKind::Capture { id, by, side } => {
                let ctx = ctx.get_mut()?;
                self.with_objective((ctx.round, id), |o| o.owner = side)?;
                for ucid in by {
                    self.pilots.with_pilot_and_aggregates(
                        ucid,
                        ctx.round,
                        |pilot| pilot.total.captures += 1,
                        |agg| agg.captures += 1,
                    )?
                }
                ctx
            }
            StatKind::Repair { id: _, by } => {
                let ctx = ctx.get_mut()?;
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |pilot| pilot.total.repairs += 1,
                    |agg| agg.repairs += 1,
                )?;
                ctx
            }
            StatKind::SupplyTransfer { from: _, to: _, by } => {
                let ctx = ctx.get_mut()?;
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |pilot| pilot.total.supply_transfers += 1,
                    |agg| agg.supply_transfers += 1,
                )?;
                ctx
            }
            StatKind::EquipmentInventory { id, item, amount } => {
                let ctx = ctx.get_mut()?;
                self.equipment
                    .fetch_and_update(&(ctx.round, id, item), |_| Some(amount))?;
                ctx
            }
            StatKind::LiquidInventory { id, item, amount } => {
                let ctx = ctx.get_mut()?;
                self.liquids
                    .fetch_and_update(&(ctx.round, id, item), |_| Some(amount))?;
                ctx
            }
            StatKind::Action { by, gid, action } => {
                let ctx = ctx.get_mut()?;
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |p| p.total.actions += 1,
                    |a| a.actions += 1,
                )?;
                if let Some(gid) = gid {
                    self.with_group((ctx.round, gid), |group| {
                        group.kind = GroupKind::Action {
                            by,
                            name: action.clone(),
                        }
                    })?;
                }
                ctx
            }
            StatKind::DeployTroop {
                by,
                pos: _,
                troop,
                gid,
            } => {
                let ctx = ctx.get_mut()?;
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |p| p.total.troops += 1,
                    |a| a.troops += 1,
                )?;
                self.with_group((ctx.round, gid), |group| {
                    group.kind = GroupKind::Troop {
                        by,
                        name: troop.clone(),
                    }
                })?;
                ctx
            }
            StatKind::DeployGroup {
                by,
                pos: _,
                gid,
                deployable,
            } => {
                let ctx = ctx.get_mut()?;
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |p| p.total.troops += 1,
                    |a| a.troops += 1,
                )?;
                self.with_group((ctx.round, gid), |group| {
                    group.kind = GroupKind::Deployed {
                        by,
                        name: deployable.clone(),
                    }
                })?;
                ctx
            }
            StatKind::DeployFarp {
                by,
                pos: _,
                oid,
                deployable: _,
            } => {
                let ctx = ctx.get_mut()?;
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |p| p.total.farps += 1,
                    |a| a.farps += 1,
                )?;
                self.with_objective((ctx.round, oid), |o| o.by = Some(by))?;
                ctx
            }
            _ => ctx.get_mut()?,
        };
        self.seq
            .insert(&(ctx.sortie.clone(), ctx.round), &stat.seq)?;
        ctx.seq = stat.seq;
        Ok(())
    }
}
