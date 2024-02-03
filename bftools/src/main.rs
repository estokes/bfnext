use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use serde_derive::Serialize;
use std::path::PathBuf;

mod mission_edit;

#[derive(Args, Clone, Debug, Serialize)]
struct Miz {
    /// the final miz file to output
    #[clap(long)]
    output: PathBuf,
    /// the base mission file
    #[clap(long)]
    base: PathBuf,
    /// the weapon template
    #[clap(long)]
    weapon: PathBuf,
    /// the options template
    #[clap(long)]
    options: PathBuf,
    /// the warehouse template
    #[clap(long)]
    warehouse: Option<PathBuf>,
    #[clap(long, default_value = "BINVENTORY")]
    blue_production_template: String,
    #[clap(long, default_value = "RINVENTORY")]
    red_production_template: String
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
