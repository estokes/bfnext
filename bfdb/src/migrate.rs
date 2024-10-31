use std::{env::args, fs::File, io::{BufRead, BufReader}};
use serde_json::{json, Value};

fn migrate_value(mut v: Value) -> Value {
    let o = v.as_object_mut().expect("expected object");
    let typ = o.remove("type").expect("expected a type").as_str().expect("type is a string").to_string();
    let seq = o.remove("seq").expect("expected a seq");
    let time = o.remove("time").expect("expected a time");
    json!({
        "seq": seq,
        "time": time,
        "kind": {
            typ: v
        }
    })
}

fn main() {
    let file = args().nth(1).expect("expected a filename");
    let file = File::open(file).expect("opening file");
    let mut file = BufReader::new(file);
    let mut buf = String::new();
    loop {
        buf.clear();
        if file.read_line(&mut buf).expect("reading line") == 0 {
            break
        }
        let v: Value = serde_json::from_str(&buf).expect("reading json value");
        println!("{}", serde_json::to_string(&migrate_value(v)).expect("printing value"))
    }
}
