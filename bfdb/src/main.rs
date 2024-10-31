use anyhow::Result;
use clap::Parser;
use db::StatsDb;
use netidx::{config::Config, path::Path as NetidxPath, subscriber::SubscriberBuilder};
use std::{future, path::PathBuf};

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
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> Result<()> {
    env_logger::init();
    let args = Args::parse();
    let subscriber = SubscriberBuilder::new()
        .config(Config::load_default()?)
        .build()?;
    let db = StatsDb::new(subscriber.clone(), args.db, args.base)?;
    future::pending().await
}
