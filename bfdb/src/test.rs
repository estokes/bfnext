use bfprotocols::stats::Stat;
use std::{env::args, fs::File, io::{BufRead, BufReader}};

fn main() {
    let file = args().nth(1).expect("expected a filename");
    let file = File::open(file).expect("opening file");
    let mut file = BufReader::new(file);
    let mut buf = String::new();
    loop {
        buf.clear();
        if file.read_line(&mut buf).expect("reading line") == 0 {
            break;
        }
        let st: Stat = match serde_json::from_str(&buf) {
            Ok(st) => st,
            Err(e) => {
                eprintln!("failed to parse {buf} {e:?}");
                break
            }
        };
        println!("{st:?}")
    }
}
