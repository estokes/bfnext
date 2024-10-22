use crate::db_id;
use anyhow::{anyhow, bail, Result};
use arrayvec::ArrayVec;
use bfprotocols::{
    cfg::{Cfg, LifeType, UnitTag, UnitTags, Vehicle},
    db::{
        group::GroupId,
        objective::{ObjectiveId, ObjectiveKind},
    },
    perf::PerfInner,
    shots::{Dead, Who},
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
use sled::{transaction::TransactionError, Db};
use sled_typed::Tree;
use smallvec::{smallvec, SmallVec};
use std::str::FromStr;
use uuid::Uuid;

db_id!(KillId);
db_id!(RoundId);
db_id!(SortieId);

#[derive(Debug, Clone, Copy, Default, Serialize, Deserialize)]
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
    donated_points: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Pilot {
    name: ArrayVec<String, 8>,
    total: Aggregates,
    token: ArrayVec<Uuid, 4>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PilotRoundInfo {
    points: i32,
    side: (DateTime<Utc>, Side),
    slot: Option<Slot>,
    lives: ArrayVec<(LifeType, DateTime<Utc>, u8), 5>,
    connected: Option<(DateTime<Utc>, String)>,
}

impl Default for PilotRoundInfo {
    fn default() -> Self {
        Self {
            points: 0,
            side: (Utc::now(), Side::Neutral),
            slot: None,
            lives: ArrayVec::new(),
            connected: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Sortie {
    vehicle: Vehicle,
    takeoff: DateTime<Utc>,
    land: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Slot {
    id: SlotId,
    time: DateTime<Utc>,
    vehicle: Option<Vehicle>,
    sortie: Option<SortieId>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Unit {
    group: Option<GroupId>,
    owner: Side,
    typ: Vehicle,
    tags: UnitTags,
    pos: Pos,
    dead: bool,
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

#[derive(Clone)]
struct Pilots {
    db: Db,
    pilots: Tree<Ucid, Pilot>,
    aggregates: Tree<(Ucid, Vehicle, RoundId), Aggregates>,
    by_name: Tree<String, ArrayVec<Ucid, 8>>,
    by_token: Tree<Uuid, Ucid>,
    sortie: Tree<(Ucid, RoundId, SortieId), Sortie>,
    round_info: Tree<(Ucid, RoundId), PilotRoundInfo>,
}

impl Pilots {
    fn new(db: &Db) -> Result<Self> {
        Ok(Self {
            db: db.clone(),
            pilots: Tree::open(db, "pilots")?,
            aggregates: Tree::open(db, "aggregates")?,
            by_name: Tree::open(db, "by_name")?,
            by_token: Tree::open(db, "by_token")?,
            sortie: Tree::open(db, "sortie")?,
            round_info: Tree::open(db, "pilot_round_info")?,
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
        let vehicle = self
            .round_info
            .get(&(ucid, round))?
            .and_then(|ri| ri.slot.and_then(|s| s.vehicle));
        self.with_pilot(ucid, f)?;
        if let Some(vehicle) = vehicle {
            self.with_aggregates((ucid, vehicle, round), g)?
        }
        Ok(())
    }

    fn with_pilot_round_info<F>(&self, ucid: Ucid, round: RoundId, mut f: F) -> Result<()>
    where
        F: FnMut(&mut PilotRoundInfo),
    {
        self.round_info.fetch_and_update(&(ucid, round), |ri| {
            let mut ri = ri.unwrap_or_default();
            f(&mut ri);
            Some(ri)
        })?;
        Ok(())
    }

    fn with_sortie<F>(&self, k: (Ucid, RoundId, SortieId), mut f: F) -> Result<()>
    where
        F: FnMut(&mut Sortie),
    {
        self.sortie
            .fetch_and_update(&k, |s| match s {
                None => None,
                Some(mut s) => {
                    f(&mut s);
                    Some(s)
                }
            })?
            .ok_or_else(|| anyhow!("sortie {k:?} is missing"))?;
        Ok(())
    }

    fn saw_pilot(&self, id: Ucid, name: String) -> Result<()> {
        self.pilots.fetch_and_update(&id, |pilot| match pilot {
            None => Some(Pilot {
                name: ArrayVec::from_iter([name.clone()]),
                total: Aggregates::default(),
                token: ArrayVec::new(),
            }),
            Some(mut pilot) => match pilot.name.iter().enumerate().find(|(_, n)| name == **n) {
                Some((i, _)) => {
                    let last = pilot.name.len() - 1;
                    pilot.name.swap(i, last);
                    Some(pilot)
                }
                None => {
                    if pilot.name.is_full() {
                        let _ = pilot.name.pop_at(0);
                    }
                    pilot.name.push(name.clone());
                    Some(pilot)
                }
            },
        })?;
        self.by_name.update_and_fetch(&name, |ids| match ids {
            None => Some(ArrayVec::from_iter([id])),
            Some(mut ids) if !ids.contains(&id) => {
                if ids.is_full() {
                    ids.pop_at(0);
                }
                ids.push(id);
                Some(ids)
            }
            Some(ids) => Some(ids),
        })?;
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

impl Default for GroupKind {
    fn default() -> Self {
        GroupKind::Objective
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Group {
    owner: Side,
    units: SmallVec<[EnId; 16]>,
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

#[derive(Clone)]
struct StatsDb {
    db: Db,
    pilots: Pilots,
    seq: Tree<(Scenario, RoundId), SeqId>,
    round: Tree<(Scenario, RoundId), Round>,
    session: Tree<(RoundId, DateTime<Utc>), Session>,
    kills: Tree<(EnId, RoundId, KillId), Dead>,
    shared_kills: Tree<KillId, SmallVec<[EnId; 2]>>,
    units: Tree<(RoundId, EnId), Unit>,
    groups: Tree<(RoundId, GroupId), Group>,
    detected: Tree<(RoundId, EnId), BitFlags<DetectionSource>>,
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
            shared_kills: Tree::open(db, "shared_kills")?,
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

    fn with_unit<F: FnMut(&mut Unit)>(&self, k: (RoundId, EnId), mut f: F) -> Result<()> {
        self.units
            .fetch_and_update(&k, |g| match g {
                None => None,
                Some(mut u) => {
                    f(&mut u);
                    Some(u)
                }
            })?
            .ok_or_else(|| anyhow!("unit {k:?} is missing"))?;
        Ok(())
    }

    fn with_shared_kills<F: FnMut(&mut SmallVec<[EnId; 2]>)>(
        &self,
        k: KillId,
        mut f: F,
    ) -> Result<()> {
        self.shared_kills.update_and_fetch(&k, |sk| {
            let mut sk = sk.unwrap_or_default();
            f(&mut sk);
            Some(sk)
        })?;
        Ok(())
    }

    fn record_kill(&self, ctx: &mut StatCtxInner, dead: Dead) -> Result<()> {
        let kid = KillId::new(&self.db)?;
        let air = match &dead.victim {
            Who::Player { ucid, .. } => {
                self.pilots.with_pilot_and_aggregates(
                    *ucid,
                    ctx.round,
                    |p| p.total.deaths += 1,
                    |a| a.deaths += 1,
                )?;
                true
            }
            Who::AI { uid, .. } => {
                let tags = self
                    .units
                    .get(&(ctx.round, EnId::Unit(*uid)))?
                    .map(|u| u.tags)
                    .unwrap_or_default();
                tags.contains(UnitTag::Aircraft) || tags.contains(UnitTag::Helicopter)
            }
        };
        let no_hit = dead.shots.iter().any(|s| s.hit);
        let up = |a: &mut Aggregates| {
            if air {
                a.air_kills += 1
            } else {
                a.ground_kills += 1
            }
        };
        for shot in dead.shots.iter() {
            if no_hit && !shot.hit {
                continue;
            }
            let enid = match &shot.shooter {
                Who::AI {
                    ucid: None, uid, ..
                } => EnId::Unit(*uid),
                Who::Player { ucid, .. }
                | Who::AI {
                    ucid: Some(ucid), ..
                } => {
                    self.pilots.with_pilot_and_aggregates(
                        *ucid,
                        ctx.round,
                        |p| up(&mut p.total),
                        |a| up(a),
                    )?;
                    EnId::Player(*ucid)
                }
            };
            self.kills.insert(&(enid, ctx.round, kid), &dead)?;
            self.with_shared_kills(kid, |sk| {
                if !sk.contains(&enid) {
                    sk.push(enid)
                }
            })?;
        }
        Ok(())
    }

    pub fn add_stat(&self, ctx: &mut StatCtx, stat: Stat) -> Result<()> {
        if let Some(ctx) = &ctx.0 {
            if stat.seq <= ctx.seq {
                return Ok(());
            }
        }
        if let StatKind::NewRound { sortie } = &stat.kind {
            if ctx.0.is_some() {
                bail!("NewRound should only appear at the beginning of the stats or after RoundEnd")
            }
            match self.seq.scan_prefix(sortie)?.next_back().transpose()? {
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
        if let StatKind::RoundEnd { winner } = &stat.kind {
            return self.round_end(ctx, stat.time, *winner);
        }
        let ctx = ctx.get_mut()?;
        match stat.kind {
            StatKind::NewRound { .. } | StatKind::RoundEnd { .. } => unreachable!(),
            StatKind::SessionStart { stop, cfg } => {
                self.session.insert(
                    &(ctx.round, stat.time),
                    &Session {
                        cfg: (*cfg).clone(),
                        stop_time: stop,
                        end: None,
                    },
                )?;
            }
            StatKind::SessionEnd {
                api_perf,
                perf,
                frame,
            } => {
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
            }
            StatKind::ObjectiveDestroyed { id } => {
                self.objectives.remove(&(ctx.round, id))?;
            }
            StatKind::ObjectiveHealth {
                id,
                last_change,
                health,
                logi,
            } => {
                self.with_objective((ctx.round, id), |o| {
                    o.last_change = last_change;
                    o.health = health;
                    o.logi = logi
                })?;
            }
            StatKind::ObjectiveSupply { id, supply, fuel } => {
                self.with_objective((ctx.round, id), |o| {
                    o.supply = supply;
                    o.logi = fuel
                })?;
            }
            StatKind::Capture { id, by, side } => {
                self.with_objective((ctx.round, id), |o| o.owner = side)?;
                for ucid in by {
                    self.pilots.with_pilot_and_aggregates(
                        ucid,
                        ctx.round,
                        |pilot| pilot.total.captures += 1,
                        |agg| agg.captures += 1,
                    )?
                }
            }
            StatKind::Repair { id: _, by } => {
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |pilot| pilot.total.repairs += 1,
                    |agg| agg.repairs += 1,
                )?;
            }
            StatKind::SupplyTransfer { from: _, to: _, by } => {
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |pilot| pilot.total.supply_transfers += 1,
                    |agg| agg.supply_transfers += 1,
                )?;
            }
            StatKind::EquipmentInventory { id, item, amount } => {
                self.equipment
                    .fetch_and_update(&(ctx.round, id, item), |_| Some(amount))?;
            }
            StatKind::LiquidInventory { id, item, amount } => {
                self.liquids
                    .fetch_and_update(&(ctx.round, id, item), |_| Some(amount))?;
            }
            StatKind::Action { by, gid, action } => {
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
            }
            StatKind::DeployTroop { by, troop, gid } => {
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
            }
            StatKind::DeployGroup {
                by,
                gid,
                deployable,
            } => {
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
            }
            StatKind::DeployFarp {
                by,
                oid,
                deployable: _,
            } => {
                self.pilots.with_pilot_and_aggregates(
                    by,
                    ctx.round,
                    |p| p.total.farps += 1,
                    |a| a.farps += 1,
                )?;
                self.with_objective((ctx.round, oid), |o| o.by = Some(by))?;
            }
            StatKind::Register {
                name,
                id,
                side,
                initial_points,
            } => {
                self.pilots.saw_pilot(id, name)?;
                self.pilots.with_pilot_round_info(id, ctx.round, |ri| {
                    ri.side = (stat.time, side);
                    ri.points = initial_points;
                })?;
            }
            StatKind::Sideswitch { id, side } => {
                self.pilots
                    .with_pilot_round_info(id, ctx.round, |ri| ri.side = (stat.time, side))?;
            }
            StatKind::Connect { id, addr, name } => {
                self.pilots.saw_pilot(id, name)?;
                self.pilots.with_pilot_round_info(id, ctx.round, |ri| {
                    ri.connected = Some((stat.time, addr.clone()))
                })?;
            }
            StatKind::Disconnect { id } => {
                self.pilots
                    .with_pilot_round_info(id, ctx.round, |ri| ri.connected = None)?;
            }
            StatKind::Slot { id, slot, typ } => {
                self.pilots.with_pilot_round_info(id, ctx.round, |ri| {
                    ri.slot = Some(Slot {
                        time: stat.time,
                        id: slot,
                        vehicle: typ.as_ref().map(|u| u.typ.clone()),
                        sortie: None,
                    })
                })?;
            }
            StatKind::Deslot { id } => {
                self.pilots
                    .with_pilot_round_info(id, ctx.round, |ri| ri.slot = None)?;
                self.units.remove(&(ctx.round, EnId::Player(id)))?;
            }
            StatKind::Unit {
                id,
                gid,
                owner,
                typ,
                pos,
            } => {
                self.units.fetch_and_update(&(ctx.round, id), |_| {
                    Some(Unit {
                        dead: false,
                        group: gid,
                        owner,
                        typ: typ.typ.clone(),
                        tags: typ.tags,
                        pos,
                    })
                })?;
                if let Some(gid) = gid {
                    self.groups.fetch_and_update(&(ctx.round, gid), |g| {
                        let mut g = g.unwrap_or_default();
                        g.owner = owner;
                        if !g.units.contains(&id) {
                            g.units.push(id);
                        }
                        Some(g)
                    })?;
                }
            }
            StatKind::Position { id, pos } => {
                self.with_unit((ctx.round, id), |u| u.pos = pos)?;
            }
            StatKind::GroupDeleted { id } => {
                if let Some(group) = self.groups.remove(&(ctx.round, id))? {
                    for uid in group.units {
                        self.units.remove(&(ctx.round, uid))?;
                    }
                }
            }
            StatKind::Detected {
                id,
                detected,
                source,
            } => {
                self.detected.update_and_fetch(&(ctx.round, id), |d| {
                    let mut d = d.unwrap_or_default();
                    if detected {
                        d.insert(source);
                    } else {
                        d.remove(source);
                    }
                    if d.is_empty() {
                        None
                    } else {
                        Some(d)
                    }
                })?;
            }
            StatKind::Takeoff { id } => {
                let sid = SortieId::new(&self.db)?;
                let mut vehicle = None;
                self.pilots.with_pilot_round_info(id, ctx.round, |ri| {
                    if let Some(sl) = ri.slot.as_mut() {
                        sl.sortie = Some(sid);
                        vehicle = sl.vehicle.clone()
                    }
                })?;
                let vehicle = vehicle.ok_or_else(|| anyhow!("{id} takeoff without slotting"))?;
                self.pilots.sortie.insert(
                    &(id, ctx.round, sid),
                    &Sortie {
                        takeoff: stat.time,
                        land: None,
                        vehicle,
                    },
                )?;
            }
            StatKind::Land { id } => {
                let mut sid: Option<SortieId> = None;
                self.pilots.with_pilot_round_info(id, ctx.round, |ri| {
                    if let Some(sl) = ri.slot.as_mut() {
                        sid = sl.sortie.take();
                    }
                })?;
                let sid = sid.ok_or_else(|| anyhow!("{id} landed without taking off"))?;
                self.pilots
                    .with_sortie((id, ctx.round, sid), |s| s.land = Some(stat.time))?;
            }
            StatKind::Life { id, lives } => {
                self.pilots.with_pilot_round_info(id, ctx.round, |ri| {
                    ri.lives.clear();
                    ri.lives
                        .extend(lives.into_iter().map(|(lt, (dt, n))| (*lt, *dt, *n)));
                })?;
            }
            StatKind::Kill(dead) => self.record_kill(ctx, dead)?,
            StatKind::Points {
                id,
                points,
                reason: _,
            } => {
                self.pilots
                    .with_pilot_round_info(id, ctx.round, |ri| ri.points += points)?;
            }
            StatKind::PointsTransfer { from, to, points } => {
                self.pilots
                    .with_pilot_round_info(from, ctx.round, |ri| ri.points -= points as i32)?;
                self.pilots.with_pilot_and_aggregates(
                    from,
                    ctx.round,
                    |p| p.total.donated_points += points,
                    |a| a.donated_points += points,
                )?;
                self.pilots
                    .with_pilot_round_info(to, ctx.round, |ri| ri.points += points as i32)?;
            }
            StatKind::Bind { id, token } => {
                let token = Uuid::from_str(&token)?;
                let mut remove = None;
                self.pilots.with_pilot(id, |p| {
                    if p.token.is_full() {
                        remove = p.token.pop_at(0);
                    }
                    p.token.push(token)
                })?;
                self.pilots.by_token.insert(&token, &id)?;
                if let Some(token) = remove {
                    self.pilots.by_token.remove(&token)?;
                }
            }
        };
        self.seq
            .insert(&(ctx.sortie.clone(), ctx.round), &stat.seq)?;
        ctx.seq = stat.seq;
        Ok(())
    }
}
