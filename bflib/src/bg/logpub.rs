use anyhow::Result;
use bytes::BytesMut;
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    select_biased, StreamExt,
};
use fxhash::FxHashSet;
use log::error;
use netidx::{
    chars::Chars,
    path::Path,
    publisher::{Event, Publisher, Value},
};
use std::{io::SeekFrom, path::PathBuf};
use tokio::{
    fs::OpenOptions,
    io::{AsyncBufReadExt, AsyncSeekExt, AsyncWriteExt, BufReader},
    task,
};

enum ToLogger {
    Log(Chars),
    Close(oneshot::Sender<()>),
}

async fn logger_loop(
    publisher: Publisher,
    file_path: &PathBuf,
    netidx_path: Path,
    mut input: UnboundedReceiver<ToLogger>,
) -> Result<()> {
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
        select_biased! {
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
                    let mut n = 0;
                    loop {
                        if bufreader.read_line(&mut buf).await? == 0 {
                            break
                        }
                        bytes.extend_from_slice(buf.trim().as_bytes());
                        buf.clear();
                        let chars = Chars::from_bytes(bytes.split().freeze()).unwrap();
                        contents.update_subscriber(&mut batch, cl, Value::String(chars));
                        n += 1;
                        if n > 500 {
                            n = 0;
                            batch.commit(None).await;
                            batch = publisher.start_batch();
                        }
                    }
                    file = bufreader.into_inner();
                    batch.commit(None).await;
                    batch = publisher.start_batch();
                },
            },
            e = input.select_next_some() => match e {
                ToLogger::Log(b) => {
                    file.write_all_buf(&mut b.as_bytes()).await?;
                    bytes.extend_from_slice(b.trim().as_bytes());
                    let c = Chars::from_bytes(bytes.split().freeze()).unwrap();
                    for cl in &subs {
                        contents.update_subscriber(&mut batch, *cl, Value::String(c.clone()))
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
    pub fn new(publisher: Publisher, file_path: &PathBuf, netidx_path: Path) -> Result<Self> {
        let (tx, rx) = mpsc::unbounded();
        let file_path = file_path.clone();
        task::spawn(async move {
            match logger_loop(publisher, &file_path, netidx_path, rx).await {
                Ok(()) => (),
                Err(e) => error!("{file_path:?} logger failed {e:?}"),
            }
        });
        Ok(Self(tx))
    }

    pub fn append(&self, m: Chars) -> Result<()> {
        Ok(self.0.unbounded_send(ToLogger::Log(m))?)
    }

    pub async fn close(&self) -> Result<()> {
        let (tx, rx) = oneshot::channel();
        self.0.unbounded_send(ToLogger::Close(tx))?;
        Ok(rx.await?)
    }
}
