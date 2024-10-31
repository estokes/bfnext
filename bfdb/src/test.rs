use bfprotocols::stats::Stat;
use std::{env::args, fs::File};

fn main() {
    let file = args().nth(1).expect("expected a filename");
    let file = File::open(file).expect("opening file");
    let st: Stat = serde_json::from_reader(&file).expect("failed to parse stat");
    println!("{st:?}")
}
