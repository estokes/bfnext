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

use crate::db::objective::ObjectiveId;
use dcso3::{
    perf::{HistStat, HistogramSer},
    String,
};
use fxhash::FxHashSet;
use log::info;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

#[derive(Debug, Clone, Copy)]
pub struct PerfStat {
    pub frame: HistStat,
    pub timed_events: HistStat,
    pub slow_timed: HistStat,
    pub dcs_events: HistStat,
    pub dcs_hooks: HistStat,
    pub unit_positions: HistStat,
    pub player_positions: HistStat,
    pub ewr_tracks: HistStat,
    pub ewr_reports: HistStat,
    pub unit_culling: HistStat,
    pub remark_objectives: HistStat,
    pub update_jtac_contacts: HistStat,
    pub do_repairs: HistStat,
    pub spawn_queue: HistStat,
    pub spawn: HistStat,
    pub despawn: HistStat,
    pub advise_captured: HistStat,
    pub advise_capturable: HistStat,
    pub jtac_target_positions: HistStat,
    pub process_messages: HistStat,
    pub snapshot: HistStat,
    pub logistics: HistStat,
    pub logistics_distribute: HistStat,
    pub logistics_deliver: HistStat,
    pub logistics_sync_from: HistStat,
    pub logistics_sync_to: HistStat,
    pub logistics_items: u64,
}

impl PerfStat {
    pub fn log(&self) {
        let Self {
            frame,
            timed_events,
            slow_timed,
            dcs_events,
            dcs_hooks,
            unit_positions,
            player_positions,
            ewr_tracks,
            ewr_reports,
            unit_culling,
            remark_objectives,
            update_jtac_contacts,
            do_repairs,
            spawn_queue,
            spawn,
            despawn,
            advise_captured,
            advise_capturable,
            jtac_target_positions,
            process_messages,
            snapshot,
            logistics,
            logistics_distribute,
            logistics_deliver,
            logistics_sync_from,
            logistics_sync_to,
            logistics_items,
        } = self;
        let stats = [
            frame,
            timed_events,
            slow_timed,
            dcs_events,
            dcs_hooks,
            unit_positions,
            player_positions,
            ewr_tracks,
            ewr_reports,
            unit_culling,
            remark_objectives,
            update_jtac_contacts,
            do_repairs,
            spawn_queue,
            spawn,
            despawn,
            advise_captured,
            advise_capturable,
            jtac_target_positions,
            process_messages,
            snapshot,
            logistics,
            logistics_distribute,
            logistics_deliver,
            logistics_sync_from,
            logistics_sync_to,
        ];
        let max_len = stats
            .iter()
            .fold(0, |l, st| std::cmp::max(l, st.name.len()));
        for st in stats {
            st.log(max_len);
        }
        info!("logistics_items: {logistics_items}")
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerfInner {
    pub timed_events: HistogramSer,
    pub slow_timed: HistogramSer,
    pub dcs_events: HistogramSer,
    pub dcs_hooks: HistogramSer,
    pub unit_positions: HistogramSer,
    pub player_positions: HistogramSer,
    pub ewr_tracks: HistogramSer,
    pub ewr_reports: HistogramSer,
    pub unit_culling: HistogramSer,
    pub remark_objectives: HistogramSer,
    pub update_jtac_contacts: HistogramSer,
    pub do_repairs: HistogramSer,
    pub spawn_queue: HistogramSer,
    pub spawn: HistogramSer,
    pub despawn: HistogramSer,
    pub advise_captured: HistogramSer,
    pub advise_capturable: HistogramSer,
    pub jtac_target_positions: HistogramSer,
    pub process_messages: HistogramSer,
    pub snapshot: HistogramSer,
    pub logistics: HistogramSer,
    pub logistics_distribute: HistogramSer,
    pub logistics_deliver: HistogramSer,
    pub logistics_sync_from: HistogramSer,
    pub logistics_sync_to: HistogramSer,
    // CR evilkipper: remove this once the warehouse client/server desync bug is fixed
    pub logistics_items: FxHashSet<(String, ObjectiveId)>,
}

impl PerfInner {
    fn stat(&self, frame: &HistogramSer) -> PerfStat {
        let Self {
            timed_events,
            slow_timed,
            dcs_events,
            dcs_hooks,
            unit_positions,
            player_positions,
            ewr_tracks,
            ewr_reports,
            unit_culling,
            remark_objectives,
            update_jtac_contacts,
            do_repairs,
            spawn_queue,
            spawn,
            despawn,
            advise_captured,
            advise_capturable,
            jtac_target_positions,
            process_messages,
            snapshot,
            logistics,
            logistics_distribute,
            logistics_deliver,
            logistics_sync_from,
            logistics_sync_to,
            logistics_items,
        } = self;
        PerfStat {
            frame: HistStat::new(frame, "frame", false),
            timed_events: HistStat::new(timed_events, "timed_events", false),
            slow_timed: HistStat::new(slow_timed, "slow_timed", false),
            dcs_events: HistStat::new(dcs_events, "dcs_events", false),
            dcs_hooks: HistStat::new(dcs_hooks, "dcs_hooks", false),
            unit_positions: HistStat::new(unit_positions, "unit_positions", false),
            player_positions: HistStat::new(player_positions, "player_positions", false),
            ewr_tracks: HistStat::new(ewr_tracks, "ewr_tracks", false),
            ewr_reports: HistStat::new(ewr_reports, "ewr_reports", false),
            unit_culling: HistStat::new(unit_culling, "unit_culling", false),
            remark_objectives: HistStat::new(remark_objectives, "remark_objectives", false),
            update_jtac_contacts: HistStat::new(
                update_jtac_contacts,
                "update_jtac_contacts",
                false,
            ),
            do_repairs: HistStat::new(do_repairs, "do_repairs", false),
            spawn_queue: HistStat::new(spawn_queue, "spawn_queue", false),
            spawn: HistStat::new(spawn, "spawn", false),
            despawn: HistStat::new(despawn, "despawn", false),
            advise_captured: HistStat::new(advise_captured, "advise_captured", false),
            advise_capturable: HistStat::new(advise_capturable, "advise_capturable", false),
            jtac_target_positions: HistStat::new(
                jtac_target_positions,
                "jtac_target_positions",
                false,
            ),
            process_messages: HistStat::new(process_messages, "process_messages", false),
            snapshot: HistStat::new(snapshot, "snapshot", false),
            logistics: HistStat::new(logistics, "logistics", false),
            logistics_distribute: HistStat::new(
                logistics_distribute,
                "logistics_distribute",
                false,
            ),
            logistics_deliver: HistStat::new(logistics_deliver, "logistics_deliver", false),
            logistics_sync_from: HistStat::new(logistics_sync_from, "logistics_sync_from", false),
            logistics_sync_to: HistStat::new(logistics_sync_to, "logistics_sync_to", false),
            logistics_items: logistics_items.len() as u64,
        }
    }
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Perf {
    pub inner: Arc<PerfInner>,
    pub frame: Arc<HistogramSer>,
}

static mut PERF: Option<Perf> = None;

impl Clone for Perf {
    fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
            frame: Arc::clone(&self.frame),
        }
    }
}

impl Perf {
    pub unsafe fn get_mut() -> &'static mut Perf {
        match PERF.as_mut() {
            Some(perf) => perf,
            None => {
                PERF = Some(Perf::default());
                PERF.as_mut().unwrap()
            }
        }
    }

    pub unsafe fn reset() {
        PERF = None;
    }

    pub fn stat(&self) -> PerfStat {
        self.inner.stat(&self.frame)
    }
}
