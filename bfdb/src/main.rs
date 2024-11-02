use anyhow::Result;
use clap::Parser;
use db::StatsDb;
use netidx::{config::Config, path::Path as NetidxPath, subscriber::SubscriberBuilder};
use regex::Regex;
use std::{future, net::SocketAddr, path::PathBuf};
use tokio::task;
use warp::{
    filters::BoxedFilter,
    reject::{Reject, Rejection},
    reply::{Reply, Response},
    Filter,
};

mod db;
mod db_id;

/// load stats and serve the coop web interface
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The base path to find and subscribe to the stats
    #[arg(short, long)]
    base: NetidxPath,
    /// The path to the database
    #[arg(short, long)]
    db: PathBuf,
    /// The certificate to use for TLS
    #[arg(short, long)]
    cert: Option<PathBuf>,
    /// The private key to use for TLS
    #[arg(short, long)]
    key: Option<PathBuf>,
    /// Include only scenarios that match the given regex
    #[arg(long)]
    include: Option<Regex>,
    /// Exclude scenarios that match the given regex
    #[arg(long)]
    exclude: Option<Regex>,
    /// The web address to listen on
    #[arg(long)]
    listen_address: SocketAddr,
}

#[derive(Debug)]
struct Error(anyhow::Error);

impl Reply for Error {
    fn into_response(self) -> Response {
        Response::new(format!("{:?}", self.0).into())
    }
}

impl From<anyhow::Error> for Error {
    fn from(value: anyhow::Error) -> Self {
        Self(value)
    }
}

async fn pilots(db: StatsDb) -> std::result::Result<impl warp::Reply, Error> {
    let buf = task::block_in_place(|| -> Result<String> {
        use std::fmt::Write;
        let mut buf = String::new();
        for r in db.pilots() {
            let (ucid, name) = r?;
            write!(buf, "{ucid}: {name}\n").unwrap()
        }
        Ok(buf)
    })?;
    Ok(buf)
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let subscriber = SubscriberBuilder::new()
        .config(Config::load_default()?)
        .build()?;
    let db = StatsDb::new(
        subscriber.clone(),
        args.db,
        args.base,
        args.include,
        args.exclude,
    )?;
    let pilots = warp::path("pilots").then({
        let db = db.clone();
        move || pilots(db.clone())
    });
    let routes = warp::get().and(pilots);
    match (&args.cert, &args.key) {
        (_, None) | (None, _) => warp::serve(routes).run(args.listen_address).await,
        (Some(cert), Some(key)) => {
            warp::serve(routes)
                .tls()
                .cert_path(cert)
                .key_path(key)
                .run(args.listen_address)
                .await
        }
    }
    Ok(())
}
