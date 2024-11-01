use chrono::prelude::*;
use serde_json::{json, Value};
use std::{
    env::args,
    fs::File,
    io::{BufRead, BufReader},
};

fn migrate_value(mut v: Value) -> Option<Value> {
    let o = v.as_object_mut().expect("expected object");
    let typ = o
        .remove("type")
        .expect("expected a type")
        .as_str()
        .expect("type is a string")
        .to_string();
    let seq = o.remove("seq").expect("expected a seq");
    let time = o.remove("time").expect("expected a time");
    if &typ == "Objective" && !o.contains_key("name") {
        o.insert("name".into(), "unknown".into());
    }
    if &typ == "Unit" {
        if o.get("id").expect("missing id").is_i64() {
            let id = o.remove("id").unwrap();
            o.insert("id".into(), json!({ "Unit": id }));
        }
        if !o.contains_key("owner") {
            o.insert("owner".into(), "Neutral".into());
        }
    }
    if &typ == "Kill" && !o.contains_key("time") {
        o.insert("time".into(), Utc::now().to_string().into());
    }
    if &typ == "SessionEnd" && !o.contains_key("api_perf") {
        return None;
    }
    Some(json!({
        "seq": seq,
        "time": time,
        "kind": {
            typ: v
        }
    }))
}

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
        let v: Value = serde_json::from_str(&buf).expect("reading json value");
        if let Some(v) = migrate_value(v) {
            println!("{}", serde_json::to_string(&v).expect("printing value"))
        }
    }
}
