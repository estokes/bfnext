use anyhow::Result;
use bytes::{Buf, Bytes, BytesMut};
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    select, StreamExt,
};
use fxhash::FxHashSet;
use netidx::{
    path::Path,
    publisher::{Event, Publisher, Value},
};
use std::{io::SeekFrom, path::PathBuf};
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
    task,
};
use log::error;

enum ToLogger {
    Log(Bytes),
    Close(oneshot::Sender<()>),
}

async fn logger_loop(
    publisher: Publisher,
    file_path: PathBuf,
    netidx_path: Path,
    mut input: UnboundedReceiver<ToLogger>,
) -> Result<()> {
    let sep = [b'\n'];
    let mut file = OpenOptions::new()
        .append(true)
        .create(true)
        .read(true)
        .write(true)
        .open(&file_path)
        .await?;
    let (tx, mut events) = mpsc::unbounded();
    let contents = publisher.publish(netidx_path, Value::Null)?;
    publisher.events_for_id(contents.id(), tx);
    let mut subs = FxHashSet::default();
    let mut batch = publisher.start_batch();
    let mut buf = String::new();
    let mut bytes = BytesMut::new();
    loop {
        select! {
            e = events.select_next_some() => match e {
                Event::Destroyed(_) => return Ok(()),
                Event::Unsubscribe(_, cl) => {
                    subs.remove(&cl);
                }
                Event::Subscribe(_, cl) => {
                    subs.insert(cl);
                    file.seek(SeekFrom::Start(0)).await?;
                    let mut bufreader = BufReader::new(file);
                    buf.clear();
                    loop {
                        if bufreader.read_line(&mut buf).await? == 0 {
                            break
                        }
                        bytes.extend_from_slice(buf.as_bytes());
                        buf.clear();
                        contents.update_subscriber(&mut batch, cl, Value::Bytes(bytes.split().freeze()));
                    }
                    file = bufreader.into_inner();
                    batch.commit(None).await;
                    batch = publisher.start_batch();
                },
            },
            e = input.select_next_some() => match e {
                ToLogger::Log(b) => {
                    file.write_all_buf(&mut (&*b).chain(&sep[..])).await?;
                    for cl in &subs {
                        contents.update_subscriber(&mut batch, *cl, Value::Bytes(b.clone()))
                    }
                    batch.commit(None).await;
                    batch = publisher.start_batch();
                }
                ToLogger::Close(ch) => {
                    drop(contents);
                    drop(file);
                    let _ = ch.send(());
                    return Ok(())
                }
            },
            complete => return Ok(())
        }
    }
}

#[derive(Debug, Clone)]
pub struct LogPublisher(UnboundedSender<ToLogger>);

impl LogPublisher {
    pub fn new(publisher: Publisher, file_path: PathBuf, netidx_path: Path) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded();
        task::spawn(async move {
            match logger_loop(publisher, file_path.clone(), netidx_path, rx).await {
                Ok(()) => (),
                Err(e) => error!("{file_path:?} logger failed {e:?}")
            }
        });
        Ok(Self(tx))
    }

    pub fn append(&self, m: Bytes) -> Result<()> {
        Ok(self.0.unbounded_send(ToLogger::Log(m))?)
    }

    pub async fn close(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0.unbounded_send(ToLogger::Close(tx))?;
        Ok(rx.await?)
    }
}
