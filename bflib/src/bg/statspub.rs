use anyhow::{Context, Result, anyhow};
use arcstr::ArcStr;
use bfprotocols::stats::{PATH, Stat};
use chrono::prelude::*;
use netidx::{
    chars::Chars,
    config::Config,
    path::Path,
    publisher::{Publisher, Value},
    resolver_client::GlobSet,
    subscriber::Event,
};
use netidx_archive::{
    config::{ConfigBuilder, PublishConfigBuilder, RecordConfigBuilder},
    logfile::{BATCH_POOL, BatchItem, Id},
    logfile_collection::ArchiveCollectionWriter,
    recorder::Recorder,
};
use std::path::PathBuf;
use tokio::task;

use super::encode;

pub(super) struct Statspub {
    id: Id,
    log: ArchiveCollectionWriter,
    _recorder: Recorder,
}

impl Statspub {
    pub(super) async fn new(
        publisher: Publisher,
        cfg: &Config,
        write_dir: PathBuf,
        base: Path,
    ) -> Result<Self> {
        let shard = ArcStr::from("0");
        let config = ConfigBuilder::default()
            .archive_directory(write_dir)
            .publish(
                PublishConfigBuilder::default()
                    .base(base)
                    .bind_from_cfg(&cfg)
                    .build()
                    .context("publish config builder")?,
            )
            .record([(
                shard.clone(),
                RecordConfigBuilder::default()
                    .spec(GlobSet::new(true, vec![])?)
                    .build()
                    .context("record config builder")?,
            )])
            .netidx_config(cfg.clone())
            .build()?;
        let recorder = Recorder::start_with(config, Some(publisher), None)
            .await
            .context("starting recorder tasks")?;
        let shard_id = *recorder
            .shards
            .by_name
            .get(&shard)
            .ok_or_else(|| anyhow!("no shard id for shard {shard}"))?;
        let mut log = recorder
            .shards
            .writers
            .lock()
            .remove(&shard_id)
            .ok_or_else(|| anyhow!("no writer for shard"))?;
        if log.len()? > 4096 {
            let now = Utc::now();
            log.rotate_and_compress(now, None).await?;
            let reader = log
                .current_reader()
                .context("get current reader after rotate")?;
            recorder
                .shards
                .notify_rotated(shard_id, now, reader)
                .context("notify rotate")?
        }
        let id = match log.id_for_path(&Path::from(PATH)) {
            Some(id) => id,
            None => {
                log.add_paths([&Path::from(PATH)])?;
                task::block_in_place(|| log.flush_pathindex())?;
                log.id_for_path(&Path::from(PATH))
                    .ok_or_else(|| anyhow!("no id after adding id"))?
            }
        };
        Ok(Self {
            id,
            log,
            _recorder: recorder,
        })
    }

    /// This will not block
    pub(super) fn append(&mut self, ts: DateTime<Utc>, stat: &Stat) -> Result<()> {
        task::block_in_place(|| {
            let mut batch = BATCH_POOL.take();
            let buf = Chars::from_bytes(encode(&stat)?.freeze())?;
            batch.push(BatchItem(self.id, Event::Update(Value::String(buf))));
            self.log.add_batch(false, ts, &batch)
        })
    }

    /// Flush the log
    pub(super) fn flush(&mut self) -> Result<()> {
        self.log.flush_current()
    }
}
