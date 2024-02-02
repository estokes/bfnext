use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde_derive::Serialize;
use std::path::PathBuf;

mod mission_edit;

#[derive(Args, Clone, Debug, Serialize)]
struct Miz {
    /// the final miz file to output
    #[clap(short, long)]
    output: PathBuf,
    /// the base mission file
    #[clap(short, long)]
    base: PathBuf,
    /// the weapon template
    #[clap(short, long)]
    weapon: PathBuf,
    /// the options template
    #[clap(short, long)]
    options: PathBuf,
    /// the warehouse template
    #[clap(short, long)]
    warehouse: Option<PathBuf>,
}

#[derive(Subcommand, Clone, Debug, Serialize)]
enum Tools {
    Miz(Miz),
}

#[derive(Parser)]
struct BftoolsArgs {
    #[clap(subcommand)]
    tool: Tools,
}

fn main() -> Result<()> {
    let bftools_args = BftoolsArgs::parse();
    env_logger::init();

    match bftools_args.tool {
        Tools::Miz(cfg) => mission_edit::run(&cfg)?,
    };
    Ok(())
}
