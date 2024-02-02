//shell script -> pass in config (gets theatre/era from base miz) -> create both missions(clones) -> set server config
//start server

//on mission load end: crack open ~other~ mission, apply (all?) templates, resave

//save mission values in a struct

//crack open miz

//deserialize mission table

//edit mission table (crack open templates 1 at a time)

//repack miz
use anyhow::{bail, Context, Result};
use log::{info, warn};
use mlua::{Lua, Table, Value};
use serde_derive::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufWriter},
    path::{Path, PathBuf},
};
use zip::{read::ZipArchive, write::FileOptions, ZipWriter};

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub base_miz_path: PathBuf,
    weapon_template: PathBuf,
    warehouse_template: PathBuf,
    option_template: PathBuf,
    weather_template_folder: PathBuf,
}

impl Config {
    pub fn from_file(file_path: &Path) -> Result<Self> {
        let file = File::open(file_path).context("opening file")?;
        Ok(serde_json::from_reader(file).context("deserializing")?)
    }
}

struct TriggerZone {
    x: f32,
    y: f32,
    radius: f32,
    objective_name: String,
    spawn_count: HashMap<String, isize>,
}

impl TriggerZone {
    pub fn new(zone: &Table) -> Result<Option<Self>> {
        let name: String = zone.get("name")?;
        let x: f32 = zone.get("x")?;
        let y: f32 = zone.get("y")?;
        let radius: f32 = zone.get("radius")?;
        if name.len() >= 5 {
            if name.starts_with('O') {
                let t = TriggerZone {
                    objective_name: name[4..].to_string(),
                    x,
                    y,
                    radius,
                    spawn_count: HashMap::new(),
                };
                info!("added objective {}", &name[4..]);
                Ok(Some(t))
            } else {
                Ok(None)
            }
        } else {
            bail!("trigger name {name} too short")
        }
    }

    pub fn vec2_in_zone(&self, x: f32, y: f32) -> bool {
        let dist = ((self.x - x).powi(2) + (self.y - y).powi(2)).sqrt();
        if dist <= self.radius {
            return true;
        };
        false
    }
}

struct UnpackedMiz {
    root: PathBuf,
    files: HashMap<String, PathBuf>,
}

impl Drop for UnpackedMiz {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

impl UnpackedMiz {
    fn new(path: &Path) -> Result<Self> {
        if !path.is_absolute() {
            bail!("you must specify the absolute path to the mission you want to unpack {path:?}")
        }
        let mut files: HashMap<String, PathBuf> = HashMap::new();
        let mut archive = ZipArchive::new(File::open(path).context("opening miz file")?)
            .context("unzipping miz")?;
        let mut root = PathBuf::from(path);
        root.set_extension("");
        info!("cracking open: {path:?}");
        for i in 0..archive.len() {
            let mut file = archive
                .by_index(i)
                .with_context(|| format!("getting file {i}"))?;
            let dump_path = root.join(file.name());
            let dump_root = dump_path.parent().unwrap();
            fs::create_dir_all(dump_root).with_context(|| format!("creating {dump_root:?}"))?;
            let mut extracted_file =
                File::create(&dump_path).with_context(|| format!("creating {dump_path:?}"))?;
            io::copy(&mut file, &mut extracted_file)
                .with_context(|| format!("copying {i} to {dump_path:?}"))?;
            files.insert(file.name().to_string(), dump_path);
        }
        Ok(Self { root, files })
    }

    fn pack(&self, destination_file: &Path) -> Result<()> {
        info!("repacking current miz to: {destination_file:?}");
        let file = File::create(&destination_file)
            .with_context(|| format!("creating {:?}", destination_file))?;
        let zip_file = BufWriter::new(file);
        let mut zip_writer = ZipWriter::new(zip_file);
        for (_, file_path) in &self.files {
            if file_path.is_dir() {
                continue;
            }
            let mut file =
                File::open(file_path).with_context(|| format!("opening file {:?}", file_path))?;
            let relative_path = file_path
                .strip_prefix(&self.root)
                .with_context(|| format!("stripping {:?} from file {file_path:?}", self.root))?;
            zip_writer
                .start_file(relative_path.to_string_lossy(), FileOptions::default())
                .context("starting zip file")?;
            io::copy(&mut file, &mut zip_writer).context("writing to zip file")?;
            info!("added {file_path:?} to archive");
        }
        info!("{destination_file:?} good to go!");
        Ok(())
    }
}

fn basic_serialize(value: &Value<'_>) -> String {
    match value {
        Value::Integer(i) => i.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::String(s) => format!("{:?}", s.to_str().unwrap()),
        _ => "".to_string(),
    }
}

fn serialize_with_cycles<'lua>(
    name: String,
    value: Value<'lua>,
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
                    Value::Table(t) => {
                        for r in t.pairs::<Value, Value>() {
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

struct LoadedMiz {
    #[allow(dead_code)]
    lua: &'static Lua,
    miz: UnpackedMiz,
    mission: Table<'static>,
    #[allow(dead_code)]
    options: Table<'static>,
    #[allow(dead_code)]
    warehouses: Table<'static>,
}

impl LoadedMiz {
    fn new(path: &Path) -> Result<Self> {
        let miz = UnpackedMiz::new(path).with_context(|| format!("unpacking {path:?}"))?;
        let lua = Box::leak(Box::new(Lua::new()));
        let mut mission = lua.create_table()?;
        let mut options = lua.create_table()?;
        let mut warehouses = lua.create_table()?;
        for (file_name, file) in &miz.files {
            if file_name != "mission" || file_name != "warehouses" || file_name != "options" {
                continue;
            }
            info!("processing {file_name}");
            let file_content =
                fs::read_to_string(file).with_context(|| format!("error reading file {file:?}"))?;
            lua.load(&file_content)
                .exec()
                .with_context(|| format!("loading {file_name} into lua"))?;
            if file_name == "mission" {
                mission = lua
                    .globals()
                    .raw_get("mission")
                    .context("extracting mission")?;
            }
            if file_name == "warehouses" {
                warehouses = lua
                    .globals()
                    .raw_get("warehouses")
                    .context("extracting warehouses")?;
            }
            if file_name == "options" {
                options = lua
                    .globals()
                    .raw_get("options")
                    .context("extracting options")?;
            }
        }
        Ok(Self {
            lua,
            miz,
            mission,
            options,
            warehouses,
        })
    }
}

pub fn process_mission(config_path: PathBuf, target_miz: PathBuf) -> Result<()> {
    let mut objective_triggers: Vec<TriggerZone> = Vec::new();
    let mission_config: Config = Config::from_file(&config_path)
        .with_context(|| format!("loading config file {:?}", config_path))?;
    let base = LoadedMiz::new(&mission_config.base_miz_path).context("loading base mission")?;
    let weapon_template =
        LoadedMiz::new(&mission_config.weapon_template).context("loading weapon template")?;
    let mut payload_templates: HashMap<String, Table> = HashMap::new();
    let mut add_prop_aircraft_templates: HashMap<String, Table> = HashMap::new();
    let mut radio_templates: HashMap<String, Table> = HashMap::new();
    fn vehicle(
        country: &Table<'static>,
        name: &str,
    ) -> Result<impl Iterator<Item = Result<Table<'static>>>> {
        Ok(country
            .raw_get::<_, Table>(name)?
            .raw_get::<_, Table>("group")?
            .pairs::<Value, Table>()
            .map(|r| Ok(r?.1)))
    }
    fn increment_key(map: &mut HashMap<String, isize>, key: &str) -> isize {
        let n = map.entry(key.to_string()).or_default();
        *n += 1;
        *n
    }
    //gather payloads/APA from the template file for helis and planes
    for coa in weapon_template
        .mission
        .raw_get::<_, Table>("coalition")?
        .pairs::<Value, Table>()
    {
        let coa = coa?.1;
        for country in coa.raw_get::<_, Table>("country")?.pairs::<Value, Table>() {
            let country = country?.1;
            for group in vehicle(&country, "plane")?.chain(vehicle(&country, "helicopter")?) {
                let group = group?;
                for unit in group.raw_get::<_, Table>("units")?.pairs::<Value, Table>() {
                    let unit = unit?.1;
                    let unit_type: String = unit.raw_get("type")?;
                    info!("adding payload template: {unit_type}");
                    if let Ok(w) = unit.raw_get("payload") {
                        payload_templates.insert(unit_type.clone(), w);
                    }
                    if let Ok(w) = unit.raw_get("AddPropAircraft") {
                        add_prop_aircraft_templates.insert(unit_type.clone(), w);
                    }
                    if let Ok(w) = unit.raw_get("Radio") {
                        radio_templates.insert(unit_type, w);
                    }
                }
            }
        }
    }
    //compile trigger zones
    for zone in base
        .mission
        .raw_get::<_, Table>("triggers")?
        .raw_get::<_, Table>("zones")?
        .pairs::<Value, Table>()
    {
        let zone = zone?.1;
        if let Some(t) = TriggerZone::new(&zone)? {
            objective_triggers.push(t);
        }
    }

    let mut replace_count: HashMap<String, isize> = HashMap::new();
    //apply weapon/APA templates to mission table in self
    info!("replacing slots with template payloads");
    for coa in base
        .mission
        .raw_get::<_, Table>("coalition")?
        .pairs::<Value, Table>()
    {
        let coa = coa?.1;
        for country in coa.raw_get::<_, Table>("country")?.pairs::<Value, Table>() {
            let country = country?.1;
            if !country.contains_key("plane")? {
                continue;
            }
            for group in vehicle(&country, "plane")?.chain(vehicle(&country, "helicopter")?) {
                let group = group?;
                for unit in group.raw_get::<_, Table>("units")?.pairs::<Value, Table>() {
                    let unit = unit?.1;
                    let unit_type: String = unit.raw_get("type")?;
                    match payload_templates.get(&unit_type) {
                        Some(w) => unit.set("payload", w.clone())?,
                        None => warn!("no payload table for {unit_type}"),
                    }
                    if let Some(w) = add_prop_aircraft_templates.get(&unit_type) {
                        unit.set("AddPropAircraft", w.clone())?
                    }
                    if let Some(w) = radio_templates.get(&unit_type) {
                        unit.set("Radio", w.clone())?
                    }
                    increment_key(&mut replace_count, &unit_type);
                    let x = unit.get("x")?;
                    let y = unit.get("y")?;
                    for trigger_zone in &mut objective_triggers {
                        if trigger_zone.vec2_in_zone(x, y) {
                            let count = increment_key(&mut trigger_zone.spawn_count, &unit_type);
                            let new_name =
                                format!("{} {} {}", trigger_zone.objective_name, &unit_type, count);
                            unit.set("name", new_name.clone())?;
                            group.set("name", new_name)?;
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
        Value::Table(base.mission.clone()),
        &mut HashMap::new(),
    );
    fs::write(base.miz.files.get("mission").unwrap(), s).context("writing mission file")?;
    info!("wrote serialized mission to mission file.");
    //replace options file
    let options_template =
        UnpackedMiz::new(&mission_config.option_template).context("loading options template")?;
    let source_options_path = options_template.files.get("options").unwrap();
    let destination_options_path = base.miz.files.get("options").unwrap();
    fs::rename(source_options_path, destination_options_path)
        .context("replacing the options file")?;
    info!(
        "replaced options file from {:?}",
        &mission_config.option_template
    );
    let mut output = base.miz.root.clone();
    let mut name = output.file_name().unwrap().to_string_lossy().into_owned();
    name.push_str("_result");
    output.set_file_name(name);
    output.set_extension("miz");
    info!("saving finalized mission to {output:?}");
    base.miz.pack(&output).context("repacking mission")?;
    Ok(())
}
