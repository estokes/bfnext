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

use crate::{db::persisted::Persisted, Perf};
use anyhow::{anyhow, Result};
use bfprotocols::{
    cfg::Cfg,
    stats::{Stat, StatKind},
};
use bytes::{BufMut, Bytes, BytesMut};
use chrono::prelude::*;
use compact_str::{format_compact, CompactString};
use fxhash::FxHashMap;
use log::error;
use once_cell::sync::OnceCell;
use parking_lot::{Condvar, Mutex};
use simplelog::{LevelFilter, WriteLogger};
use std::{
    cell::RefCell,
    env,
    ffi::OsStr,
    fs,
    io,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    runtime::Builder,
    sync::mpsc::{self, UnboundedReceiver, UnboundedSender},
};

struct LogHandle(UnboundedSender<Task>);

thread_local! {
    static LOGBUF: RefCell<BytesMut> = RefCell::new(BytesMut::new());
}

fn encode(db: &Persisted) -> Result<Bytes> {
    thread_local! {
        static BUF: RefCell<BytesMut> = RefCell::new(BytesMut::new());
    }
    BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        serde_json::to_writer((&mut *buf).writer(), db)?;
        Ok(buf.split().freeze())
    })
}

fn rotate_state(path: &Path) -> Result<()> {
    if path.exists() {
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| anyhow!("save file with no name"))?;
        use std::fmt::Write;
        let now = Utc::now();
        let mut with_ts = PathBuf::from(path);
        let mut backup = CompactString::from(name);
        write!(backup, "{}", now.timestamp()).unwrap();
        with_ts.set_file_name(backup);
        fs::rename(path, with_ts)?;
        let dir = path
            .parent()
            .ok_or_else(|| anyhow!("path has no parent dir"))?;
        let mut by_age: FxHashMap<i64, Vec<(i64, PathBuf)>> = FxHashMap::default();
        for file in fs::read_dir(dir)? {
            let file = file?;
            let fname = file.file_name();
            let fname = match fname.to_str() {
                Some(s) => s,
                None => continue,
            };
            let now = now.timestamp();
            let onemin = 60;
            let tenmin = 600;
            let hour = 3600;
            let day = 86400;
            let week = day * 7;
            let month = week * 4;
            if file.file_type()?.is_file() {
                if let Some(ts) = fname.strip_prefix(name) {
                    if let Ok(ts) = ts.parse::<i64>() {
                        let age = now - ts;
                        let file = PathBuf::from(file.path());
                        if age > month {
                            by_age
                                .entry((age / month) * month)
                                .or_default()
                                .push((ts, file));
                        } else if age > week {
                            by_age
                                .entry((age / week) * week)
                                .or_default()
                                .push((ts, file));
                        } else if age > day {
                            by_age
                                .entry((age / day) * day)
                                .or_default()
                                .push((ts, file));
                        } else if age > hour {
                            by_age
                                .entry((age / hour) * hour)
                                .or_default()
                                .push((ts, file));
                        } else if age > tenmin {
                            by_age
                                .entry((age / tenmin) * tenmin)
                                .or_default()
                                .push((ts, file));
                        } else if age > onemin {
                            by_age
                                .entry((age / onemin) * onemin)
                                .or_default()
                                .push((ts, file));
                        }
                    }
                }
            }
        }
        for (_, mut paths) in by_age {
            paths.sort_by_key(|(ts, _)| *ts);
            paths.reverse();
            while paths.len() > 1 {
                fs::remove_file(paths.pop().unwrap().1)?;
            }
        }
    }
    Ok(())
}

fn save(path: &Path, encoded: Bytes) -> Result<()> {
    use std::fs::File;
    let mut tmp = PathBuf::from(path);
    tmp.set_extension("tmp");
    let file = File::options()
        .write(true)
        .truncate(true)
        .create(true)
        .open(&tmp)?;
    let mut file = zstd::stream::Encoder::new(file, 9)?.auto_finish();
    io::copy(&mut &*encoded, &mut file)?;
    drop(file);
    if let Err(e) = rotate_state(path) {
        error!("failed to rotate backup files {e:?}")
    }
    fs::rename(tmp, path)?;
    Ok(())
}

impl io::Write for LogHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        LOGBUF.with(|lbuf| {
            let mut lbuf = lbuf.borrow_mut();
            lbuf.extend_from_slice(buf);
            self.0
                .send(Task::WriteLog(lbuf.split().freeze()))
                .map_err(|_| io::Error::new(io::ErrorKind::Other, "backend dead"))?;
            Ok(buf.len())
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn rotate_log(path: &Path) {
    if path.exists() {
        let mut rotate_path = path.to_path_buf();
        let name = rotate_path.file_name().unwrap_or(&OsStr::new("nameless"));
        let ext = rotate_path.extension().unwrap_or(&OsStr::new("ext"));
        let ts = Utc::now()
            .to_rfc3339_opts(SecondsFormat::Secs, true)
            .chars()
            .filter(|c| c != &'-' && c != &':')
            .collect::<CompactString>();
        rotate_path.set_file_name(format_compact!("{name:?}{ts}.{ext:?}"));
        if let Err(e) = fs::rename(&path, &rotate_path) {
            error!(
                "could not rotate log file {:?} to {:?} {:?}",
                path, rotate_path, e
            )
        }
    }
}

async fn write_stat(file: &mut File, buf: &mut BytesMut, stat: StatKind) {
    let stat = Stat::new(stat);
    if let Err(e) = serde_json::to_writer(buf.writer(), &stat) {
        error!("could not format log item {stat:?}: {e:?}");
        return;
    }
    buf.put_u8(0xA);
    if let Err(e) = file.write_all_buf(&mut buf.split()).await {
        error!("could not write stat {stat:?}: {e:?}")
    }
}

#[derive(Debug)]
pub(super) enum Task {
    SaveState(PathBuf, Persisted),
    ResetState(PathBuf),
    SaveConfig(PathBuf, Arc<Cfg>),
    WriteLog(Bytes),
    LogPerf(Perf, dcso3::perf::Perf),
    Sync(Arc<(Mutex<bool>, Condvar)>),
    Stat(StatKind),
    RotateStats,
}

async fn background_loop(write_dir: PathBuf, mut rx: UnboundedReceiver<Task>) {
    let mut statsbuf = BytesMut::with_capacity(4096);
    let stats_path = write_dir.join("Logs").join("bfstats.txt");
    let log_path = write_dir.join("Logs").join("bfnext.txt");
    rotate_log(&log_path);
    let mut log_file = File::options()
        .create(true)
        .write(true)
        .open(&log_path)
        .await
        .unwrap();
    let mut stats_file = File::options()
        .create(true)
        .append(true)
        .open(&stats_path)
        .await
        .unwrap();
    while let Some(msg) = rx.recv().await {
        match msg {
            Task::SaveState(path, db) => {
                let encoded = match encode(&db) {
                    Ok(encoded) => encoded,
                    Err(e) => {
                        error!("failed to encode save state {e:?}");
                        continue;
                    }
                };
                drop(db); // don't hold the db reference any longer than necessary
                match save(&path, encoded) {
                    Ok(()) => (),
                    Err(e) => error!("failed to save state to {path:?}, {e:?}"),
                }
            }
            Task::ResetState(path) => match fs::remove_file(&path) {
                Ok(()) => (),
                Err(e) => error!("failed to reset state {path:?}, {e:?}"),
            },
            Task::SaveConfig(path, cfg) => match cfg.save(&path) {
                Ok(()) => (),
                Err(e) => error!("failed to save config {e:?}"),
            },
            Task::WriteLog(mut buf) => log_file.write_all_buf(&mut buf).await.unwrap(),
            Task::LogPerf(perf, api_perf) => {
                perf.log();
                api_perf.log();
            },
            Task::Sync(a) => {
                let &(ref lock, ref cvar) = &*a;
                let mut synced = lock.lock();
                *synced = true;
                cvar.notify_all();
            }
            Task::Stat(st) => write_stat(&mut stats_file, &mut statsbuf, st).await,
            Task::RotateStats => {
                drop(stats_file);
                rotate_log(&stats_path);
                stats_file = File::options()
                    .create(true)
                    .append(true)
                    .open(&stats_path)
                    .await
                    .unwrap();
            }
        }
    }
}

static TXCOM: OnceCell<mpsc::UnboundedSender<Task>> = OnceCell::new();

fn setup_logger(tx: UnboundedSender<Task>) {
    let level = match env::var("RUST_LOG").ok().map(|s| s.to_ascii_lowercase()) {
        None => LevelFilter::Debug,
        Some(s) if &s == "trace" => LevelFilter::Trace,
        Some(s) if &s == "debug" => LevelFilter::Debug,
        Some(s) if &s == "info" => LevelFilter::Info,
        Some(s) if &s == "error" => LevelFilter::Error,
        Some(s) if &s == "warn" => LevelFilter::Warn,
        Some(s) if &s == "off" => LevelFilter::Off,
        Some(_) => LevelFilter::Debug,
    };
    WriteLogger::init(level, simplelog::Config::default(), LogHandle(tx)).unwrap()
}

pub(super) fn init(write_dir: PathBuf) -> UnboundedSender<Task> {
    match TXCOM.get() {
        Some(tx) => tx.clone(),
        None => {
            let (tx, rx) = mpsc::unbounded_channel();
            TXCOM.set(tx.clone()).unwrap();
            setup_logger(tx.clone());
            thread::spawn(move || {
                let rt = Builder::new_current_thread().enable_all().build().unwrap();
                rt.block_on(background_loop(write_dir, rx))
            });
            tx
        }
    }
}
