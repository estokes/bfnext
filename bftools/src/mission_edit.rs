//shell script -> pass in config (gets theatre/era from base miz) -> create both missions(clones) -> set server config
//start server

//on mission load end: crack open ~other~ mission, apply (all?) templates, resave

//save mission values in a struct

//crack open miz

//deserialize mission table

//edit mission table (crack open templates 1 at a time)

//repack miz
use anyhow::{anyhow, bail, Result};
use log::{info, warn};
use rlua::prelude::*;
use rlua::{Table, Value};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{BufReader, BufWriter, Read, Write},
    path::PathBuf,
};
use zip::{read::ZipArchive, write::FileOptions, CompressionMethod, ZipWriter};

#[derive(serde::Serialize, serde::Deserialize)]
pub struct MissionEditConfig {
    pub base_miz_path: PathBuf,
    weapon_template: PathBuf,
    warehouse_template: PathBuf,
    option_template: PathBuf,
    weather_template_folder: PathBuf,
}

struct TriggerZone {
    x: f32,
    y: f32,
    radius: f32,
    objective_name: String,
    spawn_count: HashMap<String, i32>,
}

impl TriggerZone {
    pub fn new(trigger_name: String, x: f32, y: f32, radius: f32) -> Result<Self> {
        if trigger_name.len() >= 5 {
            if trigger_name.starts_with('O') {
                let t = TriggerZone {
                    objective_name: trigger_name[4..].to_string(),
                    x,
                    y,
                    radius,
                    spawn_count: HashMap::new(),
                };
                info!("added objective {}", trigger_name[4..].to_string());
                return Ok(t);
            } else {
                bail!("invalid trigger name");
            }
        };
        Err(anyhow!("trigger name too short"))
    }

    pub fn vec2_in_zone(&self, x: f32, y: f32) -> bool {
        let dist = ((self.x - x).powi(2) + (self.y - y).powi(2)).sqrt();
        if dist <= self.radius {
            return true;
        };
        false
    }
}

fn dump_miz_contents(miz_path: &PathBuf) -> Result<HashMap<String, PathBuf>> {
    let mut map: HashMap<String, PathBuf> = HashMap::new();
    let mut archive = ZipArchive::new(File::open(miz_path)?)?;
    let dump_path = miz_path.parent().unwrap();
    info!("cracking open: {miz_path:?}");
    for i in 0..archive.len() {
        let mut file = archive.by_index(i)?;
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer)?;
        fs::create_dir_all(dump_path.join(file.name()).parent().unwrap())?;
        let mut extracted_file = File::create(dump_path.join(file.name()))?;
        extracted_file.write_all(&buffer)?;
        map.insert(file.name().to_string(), dump_path.join(file.name()));
    }

    Ok(map)
}

fn add_file_to_zip<W: Write + std::io::Seek>(
    zip_writer: &mut ZipWriter<W>,
    file_path: &PathBuf,
    source_directory: &PathBuf,
) -> Result<()> {
    let file = File::open(file_path)?;
    let mut file_reader = BufReader::new(file);
    let options = FileOptions::default().compression_method(CompressionMethod::Stored);
    let relative_path = file_path.strip_prefix(source_directory)?;
    zip_writer.start_file(relative_path.to_string_lossy(), options)?;
    std::io::copy(&mut file_reader, zip_writer)?;
    Ok(())
}

fn repack_miz(
    destination_file: PathBuf,
    map: HashMap<String, PathBuf>,
    source_directory: &PathBuf,
) -> Result<()> {
    info!("repacking current miz to: {destination_file:?}");
    let zip_file = BufWriter::new(File::create(&destination_file)?);
    let mut zip_writer = ZipWriter::new(zip_file);

    for (_, file_path) in map {
        if file_path.is_dir() {
            continue;
        }
        add_file_to_zip(&mut zip_writer, &file_path, source_directory)?;
        info!("added {file_path:?} to archive");
    }
    info!("{destination_file:?} good to go!");
    Ok(())
}

fn clean_files(map: HashMap<String, PathBuf>) -> Result<()> {
    let binding = map.clone();
    for (_, file_path) in map {
        let base_path = binding.get("mission").unwrap().parent().unwrap();
        let stripped = file_path.strip_prefix(base_path);

        match stripped {
            Ok(p) => {
                let components: Vec<_> = p.components().collect();
                let _ = fs::remove_dir_all(base_path.join(components[0].as_os_str()));
            }
            Err(_) => (),
        };

        match fs::remove_file(&file_path) {
            Ok(_) => info!("removed {file_path:?}"),
            Err(_) => continue,
        };
    }
    Ok(())
}

fn basic_serialize(value: &LuaValue<'_>) -> String {
    match value {
        LuaValue::Integer(i) => i.to_string(),
        LuaValue::Number(n) => n.to_string(),
        LuaValue::Boolean(b) => b.to_string(),
        LuaValue::String(s) => format!("{:?}", s.to_str().unwrap()),
        _ => "".to_string(),
    }
}

fn serialize_with_cycles<'lua>(
    name: String,
    value: rlua::Value<'lua>,
    saved: &mut HashMap<String, String>,
) -> String {
    let mut serialized = Vec::new();
    let key = &basic_serialize(&value);
    if value.type_name() == "number"
        || value.type_name() == "integer"
        || value.type_name() == "string"
        || value.type_name() == "boolean"
        || value.type_name() == "table"
    {
        serialized.push(format!("{} = ", name));

        if value.type_name() == "number"
            || value.type_name() == "integer"
            || value.type_name() == "string"
            || value.type_name() == "boolean"
        {
            serialized.push(format!("{}\n", key));
        } else {
            if saved.contains_key(key) {
                serialized.push(format!("{}\n", saved[key]));
            } else {
                saved.insert(name.clone(), basic_serialize(&value));
                serialized.push("{}\n".to_string());

                match value {
                    LuaValue::Table(t) => {
                        for r in t.pairs::<rlua::Value, rlua::Value>() {
                            let (k, v) = r.unwrap();
                            let field_name = format!("{}[{}]", name, basic_serialize(&k));
                            serialized.push(serialize_with_cycles(field_name, v, saved));
                        }
                    }
                    _ => (),
                }
            }
        }

        serialized.concat()
    } else {
        "".to_string()
    }
}

pub struct MissionEditor {
    pub mission_config: MissionEditConfig,
}

impl MissionEditConfig {
    pub fn from_file(file_path: PathBuf) -> Result<Self> {
        let mut file = File::open(file_path)?;
        let mut contents = String::new();
        file.read_to_string(&mut contents)?;
        let config: MissionEditConfig = serde_json::from_str(&contents)?;
        Ok(config)
    }
}

impl<'lua> MissionEditor {
    pub fn do_the_thing(config_path: PathBuf, lua: rlua::Lua, target_miz: PathBuf) -> Result<()> {
        let _ = lua.context(|ctx| {
            let mut objective_triggers: Vec<TriggerZone> = Vec::new();
            let mission_config: MissionEditConfig =
                MissionEditConfig::from_file(config_path).unwrap();
            let base_contents = dump_miz_contents(&mission_config.base_miz_path).unwrap();
            let mut lua_contents: HashMap<String, rlua::Table> = HashMap::new();
            for (file_name, file) in base_contents.clone() {
                info!("processing {file_name}");
                if !file.is_file() {
                    warn!("skipping {file_name}");
                    continue;
                }
                let file_content = match std::fs::read_to_string(file) {
                    Ok(f) => f,
                    Err(_) => continue,
                };
                let _ = ctx.load(&file_content).exec();
                let table: rlua::Table = match ctx.globals().raw_get(file_name.clone()) {
                    //check to see if file names always coincide with table name
                    Ok(l) => {
                        info!("{file_name} matches! inserting table..");
                        l
                    }
                    Err(_) => {
                        info!("{file_name}'s global table isnt equad to file name!");
                        continue;
                    }
                };
                lua_contents.insert(file_name, table);
            }

            let weapon_template_path = mission_config.weapon_template;
            let weapon_contents = dump_miz_contents(&weapon_template_path).unwrap();
            let file_content =
                std::fs::read_to_string(weapon_contents.get("mission").unwrap()).unwrap();
            let mission_template: rlua::Table;
            let _ = ctx.load(&file_content).exec();
            let globals = ctx.globals();
            mission_template = globals.get("mission").unwrap();

            let mut payload_templates: HashMap<String, rlua::Table> = HashMap::new();
            let mut add_prop_aircraft_templates: HashMap<String, rlua::Table> = HashMap::new();
            let mut radio_templates: HashMap<String, rlua::Table> = HashMap::new();

            fn increment_key(map: &mut HashMap<String, isize>, key: &str) {
                *map.entry(key.to_string()).or_insert(0) += 1;
            }

            //gather payloads/APA from the template file for helis and planes
            for coalitions in mission_template
                .raw_get::<_, Table>("coalition")
                .unwrap()
                .pairs::<Value, Table>()
            {
                let coa_table = coalitions.unwrap().1;
                for countries in coa_table
                    .raw_get::<_, Table>("country")
                    .unwrap()
                    .pairs::<Value, Table>()
                {
                    let country_table = countries.unwrap().1;
                    //planes
                    for groups in country_table
                        .raw_get::<_, Table>("plane")
                        .unwrap()
                        .raw_get::<_, Table>("group")
                        .unwrap()
                        .pairs::<Value, Table>()
                    {
                        let group_table = groups.unwrap().1;
                        for units in group_table
                            .raw_get::<_, Table>("units")
                            .unwrap()
                            .pairs::<Value, Table>()
                        {
                            let unit_table = units.unwrap().1;

                            let unit_type: String = unit_table.raw_get("type").unwrap();
                            info!("adding payload template: {unit_type}");
                            match unit_table.raw_get("payload") {
                                Ok(w) => {
                                    payload_templates.insert(unit_type.clone(), w);
                                }
                                Err(_) => (),
                            };
                            match unit_table.raw_get("AddPropAircraft") {
                                Ok(w) => {
                                    add_prop_aircraft_templates.insert(unit_type.clone(), w);
                                }
                                Err(_) => (),
                            };
                            match unit_table.raw_get("Radio") {
                                Ok(w) => {
                                    radio_templates.insert(unit_type, w);
                                }
                                Err(_) => (),
                            };
                        }
                    }
                    //helis
                    for groups in country_table
                        .raw_get::<_, Table>("helicopter")
                        .unwrap()
                        .raw_get::<_, Table>("group")
                        .unwrap()
                        .pairs::<Value, Table>()
                    {
                        let group_table = groups.unwrap().1;
                        for units in group_table
                            .raw_get::<_, Table>("units")
                            .unwrap()
                            .pairs::<Value, Table>()
                        {
                            let unit_table = units.unwrap().1;

                            let unit_type: String = unit_table.raw_get("type").unwrap();
                            info!("adding payload template: {unit_type}");
                            match unit_table.raw_get("payload") {
                                Ok(w) => {
                                    payload_templates.insert(unit_type.clone(), w);
                                }
                                Err(_) => (),
                            };
                            match unit_table.raw_get("AddPropAircraft") {
                                Ok(w) => {
                                    add_prop_aircraft_templates.insert(unit_type.clone(), w);
                                }
                                Err(_) => (),
                            };
                            match unit_table.raw_get("Radio") {
                                Ok(w) => {
                                    radio_templates.insert(unit_type, w);
                                }
                                Err(_) => (),
                            };
                        }
                    }
                }
            }
            //compile trigger zones
            for trigger_zone in lua_contents
                .get("mission")
                .unwrap()
                .raw_get::<_, Table>("triggers")
                .unwrap()
                .raw_get::<_, Table>("zones")
                .unwrap()
                .pairs::<Value, Table>()
            {
                let (_index, trigger_table) = trigger_zone.unwrap();
                match TriggerZone::new(
                    trigger_table.get("name").unwrap(),
                    trigger_table.get("x").unwrap(),
                    trigger_table.get("y").unwrap(),
                    trigger_table.get("radius").unwrap(),
                ) {
                    Ok(t) => objective_triggers.push(t),
                    Err(_) => continue,
                }
            }

            let mut replace_count: HashMap<String, isize> = HashMap::new();
            //apply weapon/APA templates to mission table in self
            info!("replacing slots with template payloads");
            for coalitions in lua_contents
                .get("mission")
                .unwrap()
                .raw_get::<_, Table>("coalition")
                .unwrap()
                .pairs::<Value, Table>()
            {
                let coa_table = coalitions.unwrap().1;
                for countries in coa_table
                    .raw_get::<_, Table>("country")
                    .unwrap()
                    .pairs::<Value, Table>()
                {
                    let country_table = countries.unwrap().1;
                    match country_table.raw_get::<_, Table>("plane") {
                        Ok(_) => (),
                        Err(_) => continue,
                    };
                    //planes
                    for groups in country_table
                        .raw_get::<_, Table>("plane")
                        .unwrap()
                        .raw_get::<_, Table>("group")
                        .unwrap()
                        .pairs::<Value, Table>()
                    {
                        let group_table = groups.unwrap().1;
                        for units in group_table
                            .raw_get::<_, Table>("units")
                            .unwrap()
                            .pairs::<Value, Table>()
                        {
                            let unit_table = units.unwrap().1;
                            let unit_type: String = unit_table.raw_get("type").unwrap();
                            match payload_templates.get(&unit_type) {
                                Some(w) => unit_table.set("payload", w.clone()).unwrap(),
                                None => warn!("no payload table for {unit_type}"),
                            }
                            match add_prop_aircraft_templates.get(&unit_type) {
                                Some(w) => unit_table.set("AddPropAircraft", w.clone()).unwrap(),
                                None => (),
                            };
                            match radio_templates.get(&unit_type) {
                                Some(w) => unit_table.set("Radio", w.clone()).unwrap(),
                                None => (),
                            };
                            increment_key(&mut replace_count, &unit_type);

                            let x = unit_table.get("x").unwrap();
                            let y = unit_table.get("y").unwrap();

                            for trigger_zone in &mut objective_triggers {
                                if trigger_zone.vec2_in_zone(x, y) {
                                    let count: i32 = match trigger_zone.spawn_count.get(&unit_type)
                                    {
                                        Some(i) => i + 1,
                                        None => 1,
                                    };

                                    trigger_zone.spawn_count.insert(unit_type.clone(), count);

                                    let new_name = format!(
                                        "{} {} {}",
                                        trigger_zone.objective_name, &unit_type, count
                                    );
                                    unit_table.set("name", new_name.clone()).unwrap();
                                    group_table.set("name", new_name).unwrap();
                                    break;
                                }
                            }
                        }
                    }
                    //helis
                    for groups in country_table
                        .raw_get::<_, Table>("helicopter")
                        .unwrap()
                        .raw_get::<_, Table>("group")
                        .unwrap()
                        .pairs::<Value, Table>()
                    {
                        let group_table = groups.unwrap().1;
                        for units in group_table
                            .raw_get::<_, Table>("units")
                            .unwrap()
                            .pairs::<Value, Table>()
                        {
                            let unit_table = units.unwrap().1;
                            let unit_type: String = unit_table.raw_get("type").unwrap();
                            match payload_templates.get(&unit_type) {
                                Some(w) => unit_table.set("payload", w.clone()).unwrap(),
                                None => warn!("no payload table for {unit_type}"),
                            }
                            match add_prop_aircraft_templates.get(&unit_type) {
                                Some(w) => unit_table.set("AddPropAircraft", w.clone()).unwrap(),
                                None => (),
                            };
                            match radio_templates.get(&unit_type) {
                                Some(w) => unit_table.set("Radio", w.clone()).unwrap(),
                                None => (),
                            };
                            increment_key(&mut replace_count, &unit_type);
                            let x = unit_table.get("x").unwrap();
                            let y = unit_table.get("y").unwrap();

                            for trigger_zone in &mut objective_triggers {
                                if trigger_zone.vec2_in_zone(x, y) {
                                    let count: i32 = match trigger_zone.spawn_count.get(&unit_type)
                                    {
                                        Some(i) => i + 1,
                                        None => 1,
                                    };

                                    trigger_zone.spawn_count.insert(unit_type.clone(), count);

                                    let new_name = format!(
                                        "{} {} {}",
                                        trigger_zone.objective_name, &unit_type, count
                                    );
                                    unit_table.set("name", new_name.clone()).unwrap();
                                    group_table.set("name", new_name).unwrap();
                                    break;
                                }
                            }
                        }
                    }
                }
            }
            for (unit_type, amount) in replace_count {
                info!("replaced {amount} radio/payloads for {unit_type}");
            }

            let s = serialize_with_cycles(
                "mission".to_string(),
                rlua::Value::Table(lua_contents.get("mission").unwrap().clone()),
                &mut HashMap::new(),
            );
            fs::write(base_contents.get("mission").unwrap(), s).unwrap();

            info!("wrote serialized mission to mission file.");

            //replace options file
            let option_template_path = mission_config.option_template;
            let option_contents = dump_miz_contents(&option_template_path).unwrap();
            let source_options_path = option_contents.get("options").unwrap();
            let destination_options_path = base_contents.get("options").unwrap();
            fs::rename(source_options_path, destination_options_path).unwrap();
            info!("replaced options file from {option_template_path:?}");

            repack_miz(
                target_miz,
                base_contents.clone(),
                &mission_config.base_miz_path.parent().unwrap().to_path_buf(),
            )
            .unwrap();

            info!("cleaning files...");
            let _ = clean_files(base_contents);
            let _ = clean_files(weapon_contents);
            let _ = clean_files(option_contents);
        });

        Ok(())
    }

    pub fn _apply_objective_names_to_units(&self) -> Result<()> {
        Ok(())
    }
}
