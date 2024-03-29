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

use chrono::prelude::*;
use hdrhistogram::Histogram;
use log::info;
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct PerfInner {
    pub timed_events: Histogram<u64>,
    pub slow_timed: Histogram<u64>,
    pub dcs_events: Histogram<u64>,
    pub dcs_hooks: Histogram<u64>,
    pub unit_positions: Histogram<u64>,
    pub player_positions: Histogram<u64>,
    pub ewr_tracks: Histogram<u64>,
    pub ewr_reports: Histogram<u64>,
    pub unit_culling: Histogram<u64>,
    pub remark_objectives: Histogram<u64>,
    pub update_jtac_contacts: Histogram<u64>,
    pub do_repairs: Histogram<u64>,
    pub spawn_queue: Histogram<u64>,
    pub spawn: Histogram<u64>,
    pub despawn: Histogram<u64>,
    pub advise_captured: Histogram<u64>,
    pub advise_capturable: Histogram<u64>,
    pub jtac_target_positions: Histogram<u64>,
    pub process_messages: Histogram<u64>,
    pub snapshot: Histogram<u64>,
    pub logistics: Histogram<u64>,
    pub logistics_distribute: Histogram<u64>,
    pub logistics_deliver: Histogram<u64>,
    pub logistics_sync_from: Histogram<u64>,
    pub logistics_sync_to: Histogram<u64>,
}

#[derive(Debug)]
pub struct Perf {
    pub inner: Arc<PerfInner>,
    pub frame: Arc<Histogram<u64>>,
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

impl Default for Perf {
    fn default() -> Self {
        Perf {
            inner: Arc::new(PerfInner {
                timed_events: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                slow_timed: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                dcs_events: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                dcs_hooks: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                unit_positions: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                player_positions: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                ewr_tracks: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                ewr_reports: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                unit_culling: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                remark_objectives: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                update_jtac_contacts: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                do_repairs: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                spawn_queue: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                spawn: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                despawn: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                advise_captured: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                advise_capturable: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                jtac_target_positions: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                process_messages: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                snapshot: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                logistics: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                logistics_distribute: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                logistics_deliver: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                logistics_sync_from: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
                logistics_sync_to: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            }),
            frame: Arc::new(Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap()),
        }
    }
}

pub fn record_perf(h: &mut Histogram<u64>, start_ts: DateTime<Utc>) {
    if let Some(ns) = (Utc::now() - start_ts).num_nanoseconds() {
        if ns >= 1 && ns <= 1_000_000_000 {
            *h += ns as u64;
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
        log_histogram(&self.frame, "frame:             ")
    }
}
