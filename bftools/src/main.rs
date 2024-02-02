use anyhow::{bail, Result};
use clap::Parser;
use log::info;
use serde_derive::Serialize;
use std::path::{Path, PathBuf};

mod mission_edit;

#[derive(clap::ValueEnum, Clone, Debug, Serialize)]
enum Tools {
    MissionEdit,
}

#[derive(Parser)]
struct BftoolsArgs {
    #[clap(short, long)]
    tool: Tools,
    #[clap(short, long)]
    mission_path: PathBuf,
    #[clap(short, long)]
    editable_mission_path: PathBuf,
}

fn verify_files(path: PathBuf) -> Result<PathBuf> {
    match path.extension() {
        Some(extension) => {
            if extension != Path::new("miz") {
                bail!("not a valid miz file!")
            }
        }
        None => bail!("{path:?} not a .miz!"),
    };
    let json_path = Path::new(&format!(
        "{}\\{}{}",
        path.parent().unwrap().display(),
        path.file_stem().unwrap().to_str().unwrap(),
        ".json"
    ))
    .to_path_buf();
    if !json_path.is_file() {
        bail!("config for {path:?} doesnt exist!");
    }
    info!("{path:?} is valid and has config, continuing");
    Ok(json_path)
}

fn main() -> Result<()> {
    let bftools_args = BftoolsArgs::parse();
    env_logger::init();

    match bftools_args.tool {
        Tools::MissionEdit => {
            let config = dbg!(verify_files(bftools_args.mission_path)?);
            mission_edit::process_mission(config, bftools_args.editable_mission_path)?;
        }
    };
    Ok(())
}
