/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use bytes::{BufMut, BytesMut};
use chrono::prelude::*;
use compact_str::format_compact;
use hdrhistogram::{
    serialization::{Deserializer, Serializer, V2DeflateSerializer},
    Histogram,
};
use log::info;
use serde::{de, ser, Deserialize, Serialize};
use std::{
    borrow::{Borrow, BorrowMut},
    cell::RefCell,
    ops::{Deref, DerefMut},
    sync::Arc,
};

#[derive(Debug, Clone)]
pub struct HistogramSer(Histogram<u64>);

impl Borrow<Histogram<u64>> for HistogramSer {
    fn borrow(&self) -> &Histogram<u64> {
        &self.0
    }
}

impl BorrowMut<Histogram<u64>> for HistogramSer {
    fn borrow_mut(&mut self) -> &mut Histogram<u64> {
        &mut self.0
    }
}

impl AsRef<Histogram<u64>> for HistogramSer {
    fn as_ref(&self) -> &Histogram<u64> {
        &self.0
    }
}

impl Deref for HistogramSer {
    type Target = Histogram<u64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for HistogramSer {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl Default for HistogramSer {
    fn default() -> Self {
        Self(Histogram::new_with_bounds(1, 1_000_000_000, 3).unwrap())
    }
}

impl Serialize for HistogramSer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        use base64::prelude::*;
        use ser::Error;
        thread_local! {
            static SER: RefCell<(V2DeflateSerializer, BytesMut, String)> = RefCell::new(
                (V2DeflateSerializer::default(), BytesMut::new(), String::new()));
        }
        SER.with_borrow_mut(|(ser, buf, sbuf)| {
            buf.clear();
            sbuf.clear();
            let mut w = buf.writer();
            ser.serialize(&self.0, &mut w)
                .map_err(|e| S::Error::custom(format_compact!("{e:?}")))?;
            BASE64_STANDARD.encode_string(buf, sbuf);
            serializer.serialize_str(&sbuf)
        })
    }
}

struct HistogramSerVisitor;

impl<'de> de::Visitor<'de> for HistogramSerVisitor {
    type Value = HistogramSer;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(formatter, "expecting a string")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        use base64::prelude::*;
        thread_local! {
            static SER: RefCell<(Deserializer, Vec<u8>)> = RefCell::new(
                (Deserializer::default(), Vec::new()));
        }
        SER.with_borrow_mut(|(des, buf)| {
            buf.clear();
            BASE64_STANDARD
                .decode_vec(v, buf)
                .map_err(|e| E::custom(format_compact!("{e:?}")))?;
            let h = des
                .deserialize(&mut &buf[..])
                .map_err(|e| E::custom(format_compact!("{e:?}")))?;
            Ok(HistogramSer(h))
        })
    }

    fn visit_string<E>(self, v: String) -> Result<Self::Value, E>
    where
        E: de::Error,
    {
        self.visit_str(&v)
    }
}

impl<'de> Deserialize<'de> for HistogramSer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: de::Deserializer<'de>,
    {
        deserializer.deserialize_str(HistogramSerVisitor)
    }
}

pub struct Snap<'a> {
    ts: DateTime<Utc>,
    perf: Option<&'a mut HistogramSer>,
}

impl<'a> Drop for Snap<'a> {
    fn drop(&mut self) {
        self.commit();
    }
}

impl<'a> Snap<'a> {
    pub fn new(perf: &'a mut HistogramSer) -> Self {
        Self {
            ts: Utc::now(),
            perf: Some(perf),
        }
    }

    pub fn commit(&mut self) {
        if let Some(h) = self.perf.take() {
            if let Some(ns) = (Utc::now() - self.ts).num_nanoseconds() {
                if ns >= 1 && ns <= 1_000_000_000 {
                    **h += ns as u64;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HistStat {
    pub name: &'static str,
    pub unit: &'static str,
    pub n: u64,
    pub mean: u64,
    pub twenty_five: u64,
    pub fifty: u64,
    pub ninety: u64,
    pub ninety_nine: u64,
    pub ninety_nine_nine: u64,
}

impl HistStat {
    pub fn empty(name: &'static str, ns: bool) -> Self {
        Self {
            name,
            unit: if ns { "ns" } else { "us" },
            n: 0,
            mean: 0,
            twenty_five: 0,
            fifty: 0,
            ninety: 0,
            ninety_nine: 0,
            ninety_nine_nine: 0,
        }
    }

    pub fn new(h: &Histogram<u64>, name: &'static str, ns: bool) -> Self {
        let n = h.len();
        let d = if ns { 1 } else { 1000 };
        let unit = if ns { "ns" } else { "us" };
        let mean = h.mean().trunc() as u64 / d;
        let twenty_five = h.value_at_quantile(0.25) / d;
        let fifty = h.value_at_quantile(0.5) / d;
        let ninety = h.value_at_quantile(0.9) / d;
        let ninety_nine = h.value_at_quantile(0.99) / d;
        let ninety_nine_nine = h.value_at_quantile(0.999) / d;
        Self {
            name,
            unit,
            n,
            mean,
            twenty_five,
            fifty,
            ninety,
            ninety_nine,
            ninety_nine_nine,
        }
    }

    pub fn log(&self, pad: usize) {
        let Self {
            name,
            unit,
            n,
            mean,
            twenty_five,
            fifty,
            ninety,
            ninety_nine,
            ninety_nine_nine,
        } = self;
        if *n > 0 {
            info!(
                "{name:pad$}: n: {n:>6}, mean: {mean:>5}{unit}, 25th: {twenty_five:>5}{unit}, 50th: {fifty:>5}{unit}, 90th: {ninety:>5}{unit}, 99th: {ninety_nine:>6}{unit}, 99.9th: {ninety_nine_nine:>6}{unit}"
            );
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PerfInner {
    pub get_position: HistogramSer,
    pub get_point: HistogramSer,
    pub get_velocity: HistogramSer,
    pub in_air: HistogramSer,
    pub get_ammo: HistogramSer,
    pub add_group: HistogramSer,
    pub add_static_object: HistogramSer,
    pub unit_is_exist: HistogramSer,
    pub unit_get_by_name: HistogramSer,
    pub unit_get_desc: HistogramSer,
    pub land_is_visible: HistogramSer,
    pub land_get_height: HistogramSer,
    pub timer_schedule_function: HistogramSer,
    pub timer_remove_function: HistogramSer,
    pub timer_get_time: HistogramSer,
    pub timer_get_abs_time: HistogramSer,
    pub timer_get_time0: HistogramSer,
}

#[derive(Debug, Default, Serialize, Deserialize)]
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

impl Perf {
    pub unsafe fn get_mut() -> &'static mut Perf {
        #[allow(static_mut_refs)]
        let perf = PERF.as_mut();
        match perf {
            Some(perf) => perf,
            None => {
                PERF = Some(Perf::default());
                #[allow(static_mut_refs)]
                PERF.as_mut().unwrap()
            }
        }
    }

    pub unsafe fn reset() {
        PERF = None;
    }

    pub fn stat(&self) -> PerfStat {
        let PerfInner {
            get_position,
            get_point,
            get_velocity,
            in_air,
            get_ammo,
            add_group,
            add_static_object,
            unit_is_exist,
            unit_get_by_name,
            unit_get_desc,
            land_is_visible,
            land_get_height,
            timer_schedule_function,
            timer_remove_function,
            timer_get_time,
            timer_get_abs_time,
            timer_get_time0,
        } = &*self.0;
        PerfStat {
            get_position: HistStat::new(&get_position, "Unit.getPosition", false),
            add_group: HistStat::new(&add_group, "Coalition.addGroup", false),
            add_static_object: HistStat::new(
                &add_static_object,
                "Coalition.addStaticObject",
                false,
            ),
            get_ammo: HistStat::new(&get_ammo, "Unit.getAmmo", false),
            get_point: HistStat::new(&get_point, "Unit.getPoint", false),
            get_velocity: HistStat::new(&get_velocity, "Unit.getVelocity", false),
            in_air: HistStat::new(&in_air, "Unit.inAir", false),
            land_get_height: HistStat::new(&land_get_height, "Land.getHeight", false),
            land_is_visible: HistStat::new(&land_is_visible, "Land.isVisible", false),
            timer_get_abs_time: HistStat::new(&timer_get_abs_time, "Timer.getAbsTime", false),
            timer_get_time: HistStat::new(&timer_get_time, "Timer.getTime", false),
            timer_get_time0: HistStat::new(&timer_get_time0, "Timer.getTime0", false),
            timer_remove_function: HistStat::new(
                &timer_remove_function,
                "Timer.removeFunction",
                false,
            ),
            timer_schedule_function: HistStat::new(
                &timer_schedule_function,
                "Timer.scheduleFunction",
                false,
            ),
            unit_get_by_name: HistStat::new(&unit_get_by_name, "Unit.getByName", false),
            unit_get_desc: HistStat::new(&unit_get_desc, "Unit.getDesc", false),
            unit_is_exist: HistStat::new(&unit_is_exist, "Unit.isExist", false),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct PerfStat {
    pub get_position: HistStat,
    pub get_point: HistStat,
    pub get_velocity: HistStat,
    pub in_air: HistStat,
    pub get_ammo: HistStat,
    pub add_group: HistStat,
    pub add_static_object: HistStat,
    pub unit_is_exist: HistStat,
    pub unit_get_by_name: HistStat,
    pub unit_get_desc: HistStat,
    pub land_is_visible: HistStat,
    pub land_get_height: HistStat,
    pub timer_schedule_function: HistStat,
    pub timer_remove_function: HistStat,
    pub timer_get_time: HistStat,
    pub timer_get_abs_time: HistStat,
    pub timer_get_time0: HistStat,
}

impl Default for PerfStat {
    fn default() -> Self {
        PerfStat {
            get_position: HistStat::empty("Unit.getPosition", false),
            add_group: HistStat::empty("Coalition.addGroup", false),
            add_static_object: HistStat::empty("Coalition.addStaticObject", false),
            get_ammo: HistStat::empty("Unit.getAmmo", false),
            get_point: HistStat::empty("Unit.getPoint", false),
            get_velocity: HistStat::empty("Unit.getVelocity", false),
            in_air: HistStat::empty("Unit.inAir", false),
            land_get_height: HistStat::empty("Land.getHeight", false),
            land_is_visible: HistStat::empty("Land.isVisible", false),
            timer_get_abs_time: HistStat::empty("Timer.getAbsTime", false),
            timer_get_time: HistStat::empty("Timer.getTime", false),
            timer_get_time0: HistStat::empty("Timer.getTime0", false),
            timer_remove_function: HistStat::empty("Timer.removeFunction", false),
            timer_schedule_function: HistStat::empty("Timer.scheduleFunction", false),
            unit_get_by_name: HistStat::empty("Unit.getByName", false),
            unit_get_desc: HistStat::empty("Unit.getDesc", false),
            unit_is_exist: HistStat::empty("Unit.isExist", false),
        }
    }
}

impl PerfStat {
    pub fn log(&self) {
        use std::cmp::max;
        let Self {
            get_position,
            get_point,
            get_velocity,
            in_air,
            get_ammo,
            add_group,
            add_static_object,
            unit_is_exist,
            unit_get_by_name,
            unit_get_desc,
            land_is_visible,
            land_get_height,
            timer_schedule_function,
            timer_remove_function,
            timer_get_time,
            timer_get_abs_time,
            timer_get_time0,
        } = self;
        let stats = [
            get_position,
            get_point,
            get_velocity,
            in_air,
            get_ammo,
            add_group,
            add_static_object,
            unit_is_exist,
            unit_get_by_name,
            unit_get_desc,
            land_is_visible,
            land_get_height,
            timer_schedule_function,
            timer_remove_function,
            timer_get_time,
            timer_get_abs_time,
            timer_get_time0,
        ];
        let max_len = stats.iter().fold(0, |l, st| max(l, st.name.len()));
        for st in stats {
            st.log(max_len);
        }
    }
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

pub fn record_perf(h: &mut HistogramSer, start_ts: DateTime<Utc>) {
    if let Some(ns) = (Utc::now() - start_ts).num_nanoseconds() {
        if ns >= 1 && ns <= 1_000_000_000 {
            **h += ns as u64;
        }
    }
}
