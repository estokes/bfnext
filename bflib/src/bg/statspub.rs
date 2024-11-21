use anyhow::{anyhow, bail, Context, Result};
use chrono::prelude::*;
use netidx::{
    chars::Chars,
    path::Path,
    publisher::{Publisher, Value},
    subscriber::Event,
};
use netidx_archive::{
    config::{self, Config},
    logfile::{BatchItem, Id, BATCH_POOL},
    logfile_collection::ArchiveCollectionWriter,
    recorder::Recorder,
};
use std::path::PathBuf;
use tokio::task;

pub(super) struct Statspub {
    id: Id,
    log: ArchiveCollectionWriter,
    recorder: Recorder,
}

impl Statspub {
    pub(super) async fn new(publisher: Publisher, write_dir: PathBuf, base: Path) -> Result<Self> {
        let mut config = config::file::Config::default();
        let mut shard = None;
        config.archive_directory = write_dir;
        config.archive_cmds = None;
        let r = config
            .record
            .as_mut()
            .ok_or_else(|| anyhow!("missing record"))?;
        for (name, sh) in r.shards.iter_mut() {
            if shard.is_some() {
                bail!("more than one shard")
            }
            shard = Some(name.clone());
            sh.spec = vec![]; // don't start the record process
        }
        let p = config
            .publish
            .as_mut()
            .ok_or_else(|| anyhow!("no publish config"))?;
        p.base = base.append("stats");
        let base = p.base.clone();
        let shard = shard.ok_or_else(|| anyhow!("no shard"))?;
        let config = Config::try_from(config)?;
        let recorder = Recorder::start_with(config, Some(publisher), None)
            .await
            .context("starting recorder tasks")?;
        let shard_id = *recorder
            .shards
            .by_name
            .get(shard.as_str())
            .ok_or_else(|| anyhow!("no shard id for shard {shard}"))?;
        let mut log = recorder
            .shards
            .writers
            .lock()
            .remove(&shard_id)
            .ok_or_else(|| anyhow!("no writer for shard"))?;
        if log.len()? > 0 {
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
        let id = match log.id_for_path(&base) {
            Some(id) => id,
            None => {
                log.add_paths([&base])?;
                task::block_in_place(|| log.flush_pathindex())?;
                log.id_for_path(&base)
                    .ok_or_else(|| anyhow!("no id after adding id"))?
            }
        };
        Ok(Self { id, log, recorder })
    }

    /// This will block
    pub(super) fn append(&mut self, ts: DateTime<Utc>, c: Chars) -> Result<()> {
        let mut batch = BATCH_POOL.take();
        batch.push(BatchItem(self.id, Event::Update(Value::String(c))));
        self.log.add_batch(false, ts, &batch)
    }

    /// Flush the log
    pub(super) fn flush(&mut self) -> Result<()> {
        self.log.flush_current()
    }
}
