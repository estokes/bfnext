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
use std::{borrow::{Borrow, BorrowMut}, cell::RefCell, ops::{Deref, DerefMut}, sync::Arc};

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
        log_histogram(&self.land_is_visible, "Land.isVisible:            ", false);
        log_histogram(&self.land_get_height, "Land.getHeight:            ", false);
        log_histogram(
            &self.timer_schedule_function,
            "Timer.scheduleFunction:    ",
            false,
        );
        log_histogram(
            &self.timer_remove_function,
            "Timer.removeFunction:      ",
            false,
        );
        log_histogram(&self.timer_get_time, "Timer.getTime:             ", false);
        log_histogram(
            &self.timer_get_abs_time,
            "Timer.getAbsTime:          ",
            false,
        );
        log_histogram(&self.timer_get_time0, "Timer.getTime0:            ", false);
    }
}

pub fn log_histogram(h: &Histogram<u64>, name: &str, ns: bool) {
    let n = h.len();
    if n == 0 {
        return;
    }
    let d = if ns { 1 } else { 1000 };
    let unit = if ns { "ns" } else { "us" };
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

pub fn record_perf(h: &mut HistogramSer, start_ts: DateTime<Utc>) {
    if let Some(ns) = (Utc::now() - start_ts).num_nanoseconds() {
        if ns >= 1 && ns <= 1_000_000_000 {
            **h += ns as u64;
        }
    }
}
