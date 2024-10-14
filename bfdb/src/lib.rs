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

mod db_id;

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
    troops: u32,
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
    typ: ObjectiveKind,
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
    groups: Tree<(RoundId, GroupId), SmallVec<[UnitId; 16]>>,
    detected: Tree<(RoundId, Side, EnId), BitFlags<DetectionSource>>,
    objectives: Tree<(RoundId, ObjectiveId), Objective>,
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

    fn add_stat(&self, ctx: &mut StatCtx, stat: Stat) -> Result<()> {
        if let Some(ctx) = &ctx.0 {
            if stat.seq <= ctx.seq {
                return Ok(());
            }
        }
        match stat.kind {
            StatKind::NewRound { sortie } => {
                if ctx.0.is_some() {
                    bail!("NewRound should only appear at the beginning of the stats or after RoundEnd")
                }
                match self.seq.scan_prefix(&sortie)?.next_back().transpose()? {
                    None => self.new_round(ctx, stat.time, sortie.clone(), stat.seq),
                    Some(((_, round), seq)) => match self.round.get(&(sortie.clone(), round))? {
                        Some(r) if r.end.is_none() => {
                            ctx.0 = Some(StatCtxInner {
                                round,
                                seq,
                                sortie: sortie.clone(),
                            });
                            Ok(())
                        }
                        Some(_) | None => self.new_round(ctx, stat.time, sortie.clone(), stat.seq),
                    },
                }
            }
            StatKind::RoundEnd { winner } => {
                let inner = ctx.get_mut()?;
                let key = (inner.sortie.clone(), inner.round);
                let mut round = self
                    .round
                    .get(&key)?
                    .ok_or_else(|| anyhow!("round not found"))?;
                round.end = Some(stat.time);
                round.winner = winner;
                let _ = self.round.insert(&key, &round)?;
                ctx.0 = None;
                Ok(())
            }
            StatKind::SessionStart { stop, cfg } => {
                let ctx = ctx.get_mut()?;
                self.seq.insert(&(ctx.sortie.clone(), ctx.round), &stat.seq)?;
                self.session.insert(
                    &(ctx.round, stat.time),
                    &Session {
                        cfg: (*cfg).clone(),
                        stop_time: stop,
                        end: None,
                    },
                )?;
                ctx.seq = stat.seq;
                Ok(())
            }
            StatKind::SessionEnd { api_perf, perf, frame } => {
                let ctx = ctx.get_mut()?;
                self.seq.insert(&(ctx.sortie.clone(), ctx.round), &stat.seq)?;
                match self.session.scan_prefix(&ctx.round)?.next_back().transpose()? {
                    None => bail!("no session for {} is in progress", &ctx.sortie),
                    Some((k, mut session)) => {
                        session.end = Some(SessionEnd {
                            api: api_perf,
                            engine: perf,
                            frame,
                            time: stat.time
                        });
                        self.session.insert(&k, &session)?;
                        ctx.seq = stat.seq;
                        Ok(())
                    }
                }
            }
            _ => Ok(()),
        }
    }
}
