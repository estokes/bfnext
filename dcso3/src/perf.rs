/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use chrono::prelude::*;
use hdrhistogram::Histogram;
use log::info;
use std::{ops::Deref, sync::Arc};

pub struct Snap<'a> {
    ts: DateTime<Utc>,
    perf: Option<&'a mut Histogram<u64>>,
}

impl<'a> Drop for Snap<'a> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<'a> Snap<'a> {
    pub fn new(perf: &'a mut Histogram<u64>) -> Self {
        Self {
            ts: Utc::now(),
            perf: Some(perf),
        }
    }

    pub fn commit(&mut self) {
        if let Some(h) = self.perf.take() {
            if let Some(ns) = (Utc::now() - self.ts).num_nanoseconds() {
                if ns >= 1 && ns <= 1_000_000_000 {
                    *h += ns as u64;
                }
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct PerfInner {
    pub get_position: Histogram<u64>,
    pub get_point: Histogram<u64>,
    pub get_velocity: Histogram<u64>,
    pub in_air: Histogram<u64>,
    pub get_ammo: Histogram<u64>,
    pub add_group: Histogram<u64>,
    pub add_static_object: Histogram<u64>,
    pub unit_is_exist: Histogram<u64>,
    pub unit_get_by_name: Histogram<u64>,
    pub unit_get_desc: Histogram<u64>,
    pub land_is_visible: Histogram<u64>,
    pub land_get_height: Histogram<u64>,
}

#[derive(Debug)]
pub struct Perf(pub Arc<PerfInner>);

static mut PERF: Option<Perf> = None;

impl Deref for Perf {
    type Target = PerfInner;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl Clone for Perf {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}

impl Default for Perf {
    fn default() -> Self {
        Self(Arc::new(PerfInner {
            get_position: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            get_point: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            get_velocity: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            in_air: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            get_ammo: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            add_group: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            add_static_object: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            unit_is_exist: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            unit_get_by_name: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            unit_get_desc: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            land_is_visible: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
            land_get_height: Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap(),
        }))
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
        log_histogram(&self.get_position, "Unit.getPosition:          ", false);
        log_histogram(&self.get_velocity, "Unit.getVelocity:          ", false);
        log_histogram(&self.unit_get_by_name, "Unit.getByName:            ", false);
        log_histogram(&self.unit_get_desc, "Unit.getDesc:              ", false);
        log_histogram(&self.unit_is_exist, "Unit.isExist:              ", false);
        log_histogram(&self.in_air, "Unit.inAir:                ", false);
        log_histogram(&self.get_ammo, "Unit.getAmmo:              ", false);
        log_histogram(&self.add_group, "Coalition.addGroup:        ", false);
        log_histogram(
            &self.add_static_object,
            "Coalition.addStaticObject: ",
            false,
        );
        log_histogram(&self.add_group, "Land.isVisible:            ", false);
        log_histogram(&self.add_group, "Land.getHeight:            ", false);
    }
}

pub fn log_histogram(h: &Histogram<u64>, name: &str, ns: bool) {
    let d = if ns { 1 } else { 1000 };
    let unit = if ns { "ns" } else { "us" };
    let n = h.len();
    let mean = h.mean().trunc() as u64 / d;
    let twenty_five = h.value_at_quantile(0.25) / d;
    let fifty = h.value_at_quantile(0.5) / d;
    let ninety = h.value_at_quantile(0.9) / d;
    let ninety_nine = h.value_at_quantile(0.99) / d;
    let ninety_nine_nine = h.value_at_quantile(0.999) / d;
    info!(
        "{name} n: {n:>6}, mean: {mean:>5}{unit}, 25th: {twenty_five:>5}{unit}, 50th: {fifty:>5}{unit}, 90th: {ninety:>5}{unit}, 99th: {ninety_nine:>6}{unit}, 99.9th: {ninety_nine_nine:>6}{unit}"
    );
}

#[cfg(feature = "perf")]
#[macro_export]
macro_rules! record_perf {
    ($key:ident, $e:expr) => {{
        use crate::perf::{Perf, Snap};
        use std::sync::Arc;
        let t = unsafe { Perf::get_mut() };
        let t = Arc::make_mut(&mut t.0); // takes about 20ns if we don't need to clone
        let mut snap = Snap::new(&mut t.$key);
        let res = $e;
        snap.commit();
        res
    }};
}

#[cfg(not(feature = "perf"))]
#[macro_export]
macro_rules! record_perf {
    ($key:ident, $e:expr) => {
        $e
    };
}
