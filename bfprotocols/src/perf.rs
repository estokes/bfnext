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
use dcso3::{String, perf::HistogramSer};
use fxhash::FxHashSet;
use hdrhistogram::Histogram;
use log::info;
use std::sync::Arc;

#[derive(Debug, Clone, Default)]
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

#[derive(Debug, Default)]
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

    pub fn log(&self) {
        fn log_histogram(h: &Histogram<u64>, name: &str) {
            let n = h.len();
            let twenty_five = h.value_at_quantile(0.25) / 1000;
            let fifty = h.value_at_quantile(0.5) / 1000;
            let ninety = h.value_at_quantile(0.9) / 1000;
            let ninety_nine = h.value_at_quantile(0.99) / 1000;
            info!(
                "{name} n: {:>5}, 25th: {:>5}, 50th: {:>5}, 90th: {:>5}, 99th: {:>6}",
                n, twenty_five, fifty, ninety, ninety_nine
            );
        }
        log_histogram(&self.inner.timed_events, "timed events:      ");
        log_histogram(&self.inner.slow_timed, "slow timed events: ");
        log_histogram(&self.inner.dcs_events, "dcs events:        ");
        log_histogram(&self.inner.dcs_hooks, "dcs hooks:         ");
        log_histogram(&self.inner.unit_positions, "unit positions:    ");
        log_histogram(&self.inner.player_positions, "player positions:  ");
        log_histogram(&self.inner.ewr_tracks, "ewr tracks:        ");
        log_histogram(&self.inner.ewr_reports, "ewr reports:       ");
        log_histogram(&self.inner.unit_culling, "unit culling:      ");
        log_histogram(&self.inner.remark_objectives, "remark objectives: ");
        log_histogram(&self.inner.update_jtac_contacts, "update jtacs:      ");
        log_histogram(&self.inner.do_repairs, "do repairs:        ");
        log_histogram(&self.inner.spawn_queue, "spawn queue:       ");
        log_histogram(&self.inner.spawn, "spawn:             ");
        log_histogram(&self.inner.despawn, "despawn:           ");
        log_histogram(&self.inner.advise_captured, "advise captured:   ");
        log_histogram(&self.inner.advise_capturable, "advise capturable: ");
        log_histogram(&self.inner.jtac_target_positions, "jtac target pos:   ");
        log_histogram(&self.inner.process_messages, "process messages:  ");
        log_histogram(&self.inner.snapshot, "snapshot:          ");
        log_histogram(&self.inner.logistics, "logistics:         ");
        log_histogram(&self.inner.logistics_distribute, "logistics_distrib: ");
        log_histogram(&self.inner.logistics_deliver, "logistics_deliver: ");
        log_histogram(&self.inner.logistics_sync_from, "logistics_sfrom:   ");
        log_histogram(&self.inner.logistics_sync_to, "logistics_sto:     ");
        log_histogram(&self.frame, "frame:             ");
        info!("logistics items:   {}", self.inner.logistics_items.len());
    }
}
