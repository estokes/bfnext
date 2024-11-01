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

mod logpub;
mod perf;
mod rpcs;

use crate::{admin::AdminCommand, db::persisted::Persisted};
use anyhow::{anyhow, bail, Result};
use bfprotocols::{
    cfg::Cfg,
    perf::{Perf, PerfStat},
    stats::{Stat, StatKind},
};
use bytes::{BufMut, Bytes, BytesMut};
use chrono::prelude::*;
use compact_str::{format_compact, CompactString};
use crossbeam::queue::SegQueue;
use dcso3::perf::{Perf as ApiPerf, PerfStat as ApiPerfStat};
use fxhash::FxHashMap;
use log::error;
use logpub::LogPublisher;
use netidx::{
    chars::Chars,
    config::Config,
    path::Path as NetIdxPath,
    publisher::{Publisher, PublisherBuilder, Value},
};
use once_cell::sync::OnceCell;
use parking_lot::{Condvar, Mutex};
use perf::PubPerf;
use rpcs::Rpcs;
use serde::Serialize;
use simplelog::{LevelFilter, WriteLogger};
use std::{
    cell::RefCell,
    env,
    ffi::OsStr,
    fs, io,
    path::{Path, PathBuf},
    sync::Arc,
    thread,
};
use tokio::{
    fs::File,
    io::AsyncWriteExt,
    runtime::Builder,
    sync::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    task,
};

thread_local! {
    static LOGBUF: RefCell<BytesMut> = RefCell::new(BytesMut::new());
}

struct LogHandle(UnboundedSender<Task>);

impl io::Write for LogHandle {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        LOGBUF.with_borrow_mut(|lbuf| {
            lbuf.extend_from_slice(buf);
            if lbuf.len() > 0 && lbuf[lbuf.len() - 1] == 0xA {
                self.0
                    .send(Task::WriteLog(lbuf.split().freeze()))
                    .map_err(|_| io::Error::new(io::ErrorKind::Other, "backend dead"))?;
            }
            Ok(buf.len())
        })
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn encode<T: Serialize>(db: &T) -> Result<BytesMut> {
    thread_local! {
        static BUF: RefCell<BytesMut> = RefCell::new(BytesMut::new());
    }
    BUF.with(|buf| {
        let mut buf = buf.borrow_mut();
        serde_json::to_writer((&mut *buf).writer(), db)?;
        Ok(buf.split())
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
                if let Some(path) = paths.pop() {
                    fs::remove_file(path.1)?;
                }
            }
        }
    }
    Ok(())
}

async fn save(path: PathBuf, encoded: Bytes) -> Result<()> {
    task::spawn_blocking(move || {
        use std::fs::File;
        let mut tmp = PathBuf::from(&path);
        tmp.set_extension("tmp");
        let file = File::options()
            .write(true)
            .truncate(true)
            .create(true)
            .open(&tmp)?;
        let mut file = zstd::stream::Encoder::new(file, 9)?.auto_finish();
        io::copy(&mut &*encoded, &mut file)?;
        drop(file);
        if let Err(e) = rotate_state(&path) {
            error!("failed to rotate backup files {e:?}")
        }
        fs::rename(tmp, path)?;
        Ok(())
    })
    .await?
}

fn rotate_log(path: &Path) {
    if path.exists() {
        let ext = path
            .extension()
            .unwrap_or(&OsStr::new("ext"))
            .to_str()
            .unwrap_or("inv");
        let mut rotate_path = path.to_path_buf();
        rotate_path.set_extension("");
        let name = rotate_path
            .file_name()
            .unwrap_or(&OsStr::new("nameless"))
            .to_str()
            .unwrap_or("invalid");
        let ts = Utc::now()
            .to_rfc3339_opts(SecondsFormat::Secs, true)
            .chars()
            .filter(|c| c != &'-' && c != &':')
            .collect::<CompactString>();
        rotate_path.set_file_name(format_compact!("{name}{ts}.{ext}"));
        if let Err(e) = fs::rename(&path, &rotate_path) {
            println!(
                "could not rotate log file {:?} to {:?} {:?}",
                path, rotate_path, e
            )
        }
    }
}

#[derive(Debug)]
pub(super) enum Task {
    SaveState(PathBuf, Persisted),
    ResetState(PathBuf),
    CfgLoaded {
        sortie: dcso3::String,
        cfg: Arc<Cfg>,
        admin_channel: Arc<SegQueue<(AdminCommand, oneshot::Sender<Value>)>>,
    },
    SaveConfig(PathBuf, Arc<Cfg>),
    WriteLog(Bytes),
    LogPerf {
        players: usize,
        perf: Perf,
        api_perf: ApiPerf,
    },
    Shutdown(Arc<(Mutex<bool>, Condvar)>),
    Stat(StatKind),
    RotateStats,
}

enum Logs {
    Netidx {
        publisher: Publisher,
        base: NetIdxPath,
        perf: PubPerf,
        stats_path: PathBuf,
        stats: LogPublisher,
        log: LogPublisher,
    },
    Files {
        log_path: PathBuf,
        log_file: Option<File>,
        stats_path: PathBuf,
        stats_file: Option<File>,
    },
}

impl Logs {
    async fn open_files(&mut self) -> Result<()> {
        match self {
            Self::Netidx { .. } => Ok(()),
            Self::Files {
                log_path,
                log_file,
                stats_path,
                stats_file,
            } => {
                *log_file = Some(
                    File::options()
                        .create(true)
                        .write(true)
                        .open(&log_path)
                        .await?,
                );
                *stats_file = Some(
                    File::options()
                        .create(true)
                        .append(true)
                        .open(&stats_path)
                        .await?,
                );
                Ok(())
            }
        }
    }

    async fn new(write_dir: &Path) -> Result<Self> {
        let stats_path = write_dir.join("Logs").join("bfstats.txt");
        let log_path = write_dir.join("Logs").join("bfnext.txt");
        rotate_log(&log_path);
        let mut t = Self::Files {
            log_file: None,
            stats_file: None,
            log_path,
            stats_path,
        };
        t.open_files().await?;
        Ok(t)
    }

    async fn write_log(&mut self, buf: Chars) -> Result<()> {
        match self {
            Self::Netidx { log, .. } => log.append(buf),
            Self::Files {
                log_file: Some(log_file),
                ..
            } => Ok(log_file.write_all_buf(&mut buf.as_bytes()).await?),
            Self::Files { .. } => bail!("log file is closed"),
        }
    }

    async fn write_stat(&mut self, stat: StatKind) -> Result<()> {
        let mut buf = encode(&Stat::new(stat))?;
        buf.put_u8(0xA);
        match self {
            Self::Netidx { stats, .. } => {
                let buf = Chars::from_bytes(buf.freeze())?;
                stats.append(buf)
            }
            Self::Files {
                stats_file: Some(stats_file),
                ..
            } => Ok(stats_file.write_all_buf(&mut buf).await?),
            Self::Files { .. } => bail!("stats file is closed"),
        }
    }

    async fn rotate_stats(&mut self) -> Result<()> {
        match self {
            Self::Netidx {
                publisher,
                base,
                stats_path,
                stats,
                ..
            } => {
                if let Err(e) = stats.close().await {
                    error!("failed to close stats {e:?}")
                }
                rotate_log(&stats_path);
                *stats = LogPublisher::new(publisher.clone(), stats_path, base.append("stats"))?;
                Ok(())
            }
            Self::Files {
                stats_path,
                stats_file,
                ..
            } => {
                drop(stats_file.take());
                rotate_log(&stats_path);
                *stats_file = Some(
                    File::options()
                        .create(true)
                        .append(true)
                        .open(&stats_path)
                        .await?,
                );
                Ok(())
            }
        }
    }

    async fn log_perf(&self, players: usize, perf_stat: &PerfStat, api_perf_stat: &ApiPerfStat) {
        perf_stat.log();
        api_perf_stat.log();
        match self {
            Self::Files { .. } => (),
            Self::Netidx {
                publisher, perf, ..
            } => {
                let mut batch = publisher.start_batch();
                perf.update(&mut batch, players, perf_stat, api_perf_stat);
                batch.commit(None).await
            }
        }
    }

    async fn switch_to_netidx(&mut self, publisher: Publisher, base: NetIdxPath) -> Result<()> {
        match self {
            Self::Netidx { .. } => Ok(()),
            Self::Files {
                log_path,
                log_file,
                stats_path,
                stats_file,
            } => {
                drop((log_file.take(), stats_file.take()));
                let go = || async {
                    let perf = PubPerf::new(
                        &publisher,
                        &base,
                        0,
                        &PerfStat::default(),
                        &ApiPerfStat::default(),
                    )?;
                    let stats =
                        LogPublisher::new(publisher.clone(), stats_path, base.append("stats"))?;
                    let log = LogPublisher::new(publisher.clone(), log_path, base.append("log"))?;
                    Ok::<_, anyhow::Error>(Self::Netidx {
                        publisher: publisher.clone(),
                        base,
                        stats_path: stats_path.clone(),
                        perf,
                        stats,
                        log,
                    })
                };
                match go().await {
                    Ok(t) => {
                        *self = t;
                        Ok(())
                    }
                    Err(e) => {
                        if let Err(e) = self.open_files().await {
                            eprintln!("netidx init failed and reopening files also failed {e:?}")
                        }
                        return Err(e);
                    }
                }
            }
        }
    }

    async fn shutdown(&mut self) {
        match self {
            Self::Files { .. } => (),
            Self::Netidx { publisher, log, stats, .. } => {
                let _ = log.close().await;
                let _ = stats.close().await;
                publisher.clone().shutdown().await
            },
        }
    }
}

async fn background_loop(write_dir: PathBuf, mut rx: UnboundedReceiver<Task>) {
    let mut logs = Logs::new(&write_dir)
        .await
        .expect("could not open log files");
    let mut _rpcs: Option<Rpcs> = None;
    while let Some(msg) = rx.recv().await {
        match msg {
            Task::CfgLoaded {
                sortie,
                cfg,
                admin_channel,
            } => {
                if let Some(base) = cfg.netidx_base.as_ref() {
                    let base = base.append(&sortie);
                    let cfg = match Config::load_default() {
                        Ok(c) => c,
                        Err(e) => {
                            error!("failed to load netidx config {e:?}");
                            continue;
                        }
                    };
                    let publisher = match PublisherBuilder::new(cfg).build().await {
                        Ok(p) => p,
                        Err(e) => {
                            error!("failed to init netidx publisher {e:?}");
                            continue;
                        }
                    };
                    _rpcs = match Rpcs::new(&publisher, &admin_channel, &base).await {
                        Ok(r) => Some(r),
                        Err(e) => {
                            error!("failed to init rpcs {e:?}");
                            None
                        }
                    };
                    if let Err(e) = logs.switch_to_netidx(publisher.clone(), base.clone()).await {
                        eprintln!("failed to initialize netidx logs {e:?}")
                    }
                }
            }
            Task::SaveState(path, db) => {
                let encoded = match encode(&db) {
                    Ok(encoded) => encoded.freeze(),
                    Err(e) => {
                        error!("failed to encode save state {e:?}");
                        continue;
                    }
                };
                drop(db); // don't hold the db reference any longer than necessary
                match save(path.clone(), encoded).await {
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
            Task::WriteLog(buf) => match Chars::from_bytes(buf) {
                Err(e) => eprintln!("invalid unicode log {e:?}"),
                Ok(buf) => {
                    if let Err(e) = logs.write_log(buf).await {
                        eprintln!("could not write log line {e:?}")
                    }
                }
            },
            Task::LogPerf {
                players,
                perf,
                api_perf,
            } => {
                logs.log_perf(players, &perf.stat(), &api_perf.stat()).await;
            }
            Task::Shutdown(a) => {
                println!("starting netidx shutdown");
                logs.shutdown().await;
                println!("netidx shutdown complete");
                let &(ref lock, ref cvar) = &*a;
                let mut synced = lock.lock();
                *synced = true;
                cvar.notify_all();
                println!("condvar signaled, exiting background loop");
                break
            },
            Task::Stat(st) => {
                if let Err(e) = logs.write_stat(st).await {
                    eprintln!("could not write stat {e:?}")
                }
            }
            Task::RotateStats => {
                if let Err(e) = logs.rotate_stats().await {
                    eprintln!("failed to rotate stats {e:?}")
                }
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
    WriteLogger::init(level, simplelog::Config::default(), LogHandle(tx))
        .expect("could not init logger")
}

pub(super) fn init(write_dir: PathBuf) -> UnboundedSender<Task> {
    match TXCOM.get() {
        Some(tx) => tx.clone(),
        None => {
            let (tx, rx) = mpsc::unbounded_channel();
            TXCOM.set(tx.clone()).expect("txcom is already set");
            setup_logger(tx.clone());
            thread::spawn(move || {
                let rt = Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .expect("could not initialize async runtime");
                rt.block_on(background_loop(write_dir, rx));
                println!("background thread exiting")
            });
            tx
        }
    }
}
