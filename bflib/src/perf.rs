use hdrhistogram::Histogram;
use log::info;
use std::sync::Arc;
use chrono::prelude::*;

#[derive(Debug, Clone)]
pub struct Perf {
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

static mut PERF: Option<Arc<Perf>> = None;

impl Default for Perf {
    fn default() -> Self {
        Perf {
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
    pub unsafe fn get_mut() -> &'static mut Arc<Perf> {
        match PERF.as_mut() {
            Some(perf) => perf,
            None => {
                PERF = Some(Arc::new(Perf::default()));
                PERF.as_mut().unwrap()
            }
        }
    }

    pub fn log(&self) {
        fn log_histogram(h: &Histogram<u64>, name: &str) {
            let n = h.len();
            let twenty_five = h.value_at_quantile(0.25) / 1000;
            let fifty = h.value_at_quantile(0.5) / 1000;
            let ninety = h.value_at_quantile(0.9) / 1000;
            let ninety_nine = h.value_at_quantile(0.99) / 1000;
            info!(
                "{name} n: {:>7}, 25th: {:>7}, 50th: {:>7}, 90th: {:>7}, 99th: {:>8}",
                n, twenty_five, fifty, ninety, ninety_nine
            );
        }
        log_histogram(&self.timed_events, "timed events:      ");
        log_histogram(&self.slow_timed, "slow timed events: ");
        log_histogram(&self.dcs_events, "dcs events:        ");
        log_histogram(&self.dcs_hooks, "dcs hooks:         ");
        log_histogram(&self.unit_positions, "unit positions:    ");
        log_histogram(&self.player_positions, "player positions:  ");
        log_histogram(&self.ewr_tracks, "ewr tracks:        ");
        log_histogram(&self.ewr_reports, "ewr reports:       ");
        log_histogram(&self.unit_culling, "unit culling:      ");
        log_histogram(&self.remark_objectives, "remark objectives: ");
        log_histogram(&self.update_jtac_contacts, "update jtacs:      ");
        log_histogram(&self.do_repairs, "do repairs:        ");
        log_histogram(&self.spawn_queue, "spawn queue:       ");
        log_histogram(&self.advise_captured, "advise captured:   ");
        log_histogram(&self.advise_capturable, "advise capturable: ");
        log_histogram(&self.jtac_target_positions, "jtac target pos:   ");
        log_histogram(&self.process_messages, "process messages:  ");
        log_histogram(&self.snapshot, "snapshot:          ");
        log_histogram(&self.logistics, "logistics:         ")
    }
}
