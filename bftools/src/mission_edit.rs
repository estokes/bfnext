//shell script -> pass in config (gets theatre/era from base miz) -> create both missions(clones) -> set server config
//start server

//on mission load end: crack open ~other~ mission, apply (all?) templates, resave

//save mission values in a struct

//crack open miz

//deserialize mission table

//edit mission table (crack open templates 1 at a time)

//repack miz
use crate::Miz;
use anyhow::{bail, Context, Result};
use log::{info, warn};
use mlua::{FromLua, IntoLua, Lua, Table, Value};
use std::{
    collections::HashMap,
    fs::{self, File},
    io::{self, BufWriter},
    path::{Path, PathBuf},
};
use zip::{read::ZipArchive, write::FileOptions, ZipWriter};

pub trait DeepClone<'lua>: IntoLua<'lua> + FromLua<'lua> + Clone {
    fn deep_clone(&self, lua: &'lua Lua) -> Result<Self>;
}

impl<'lua, T> DeepClone<'lua> for T
where
    T: IntoLua<'lua> + FromLua<'lua> + Clone,
{
    fn deep_clone(&self, lua: &'lua Lua) -> Result<Self> {
        let v = match self.clone().into_lua(lua)? {
            Value::Boolean(b) => Value::Boolean(b),
            Value::Error(e) => Value::Error(e),
            Value::Function(f) => Value::Function(f),
            Value::Integer(i) => Value::Integer(i),
            Value::LightUserData(d) => Value::LightUserData(d),
            Value::Nil => Value::Nil,
            Value::Number(n) => Value::Number(n),
            Value::String(s) => Value::String(lua.create_string(s)?),
            Value::Table(t) => {
                let new = lua.create_table()?;
                new.set_metatable(t.get_metatable());
                for r in t.pairs::<Value, Value>() {
                    let (k, v) = r?;
                    new.set(k.deep_clone(lua)?, v.deep_clone(lua)?)?
                }
                Value::Table(new)
            }
            Value::Thread(t) => Value::Thread(t),
            Value::UserData(d) => Value::UserData(d),
        };
        Ok(T::from_lua(v, lua)?)
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
        if name.starts_with('O') {
            if name.len() < 5 {
                bail!("trigger name {name} too short")
            }
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
    miz: UnpackedMiz,
    mission: Table<'static>,
    #[allow(dead_code)]
    options: Table<'static>,
    #[allow(dead_code)]
    warehouses: Table<'static>,
}

impl LoadedMiz {
    fn new(lua: &'static Lua, path: &Path) -> Result<Self> {
        let miz = UnpackedMiz::new(path).with_context(|| format!("unpacking {path:?}"))?;
        let mut mission = lua.create_table()?;
        let mut options = lua.create_table()?;
        let mut warehouses = lua.create_table()?;
        for (file_name, file) in &miz.files {
            if file_name != "mission" && file_name != "warehouses" && file_name != "options" {
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
        if mission.is_empty() {
            bail!("{path:?} did not contain a mission file")
        }
        if options.is_empty() {
            bail!("{path:?} did not contain an options file")
        }
        if warehouses.is_empty() {
            bail!("{path:?} did not contain a warehouses file")
        }
        Ok(Self {
            miz,
            mission,
            options,
            warehouses,
        })
    }
}

fn vehicle(
    country: &Table<'static>,
    name: &str,
) -> Result<Box<dyn Iterator<Item = Result<Table<'static>>>>> {
    if !country.contains_key(name)? {
        Ok(Box::new([].into_iter()))
    } else {
        Ok(Box::new(
            country
                .raw_get::<_, Table>(name)?
                .raw_get::<_, Table>("group")?
                .pairs::<Value, Table>()
                .map(|r| Ok(r?.1)),
        ))
    }
}

fn increment_key(map: &mut HashMap<String, isize>, key: &str) -> isize {
    let n = map.entry(key.to_string()).or_default();
    *n += 1;
    *n
}

struct VehicleTemplates {
    payload: HashMap<String, Table<'static>>,
    prop_aircraft: HashMap<String, Table<'static>>,
    radio: HashMap<String, Table<'static>>,
}

impl VehicleTemplates {
    fn new(wep: &LoadedMiz) -> Result<Self> {
        let mut payload: HashMap<String, Table> = HashMap::new();
        let mut prop_aircraft: HashMap<String, Table> = HashMap::new();
        let mut radio: HashMap<String, Table> = HashMap::new();
        for coa in wep
            .mission
            .raw_get::<_, Table>("coalition")?
            .pairs::<Value, Table>()
        {
            let coa = coa?.1;
            for country in coa
                .raw_get::<_, Table>("country")
                .context("getting countries")?
                .pairs::<Value, Table>()
            {
                let country = country?.1;
                for group in vehicle(&country, "plane")
                    .context("getting planes")?
                    .chain(vehicle(&country, "helicopter").context("getting helicopters")?)
                {
                    let group = group?;
                    for unit in group
                        .raw_get::<_, Table>("units")
                        .context("getting units")?
                        .pairs::<Value, Table>()
                    {
                        let unit = unit?.1;
                        let unit_type: String = unit.raw_get("type").context("getting units")?;
                        info!("adding payload template: {unit_type}");
                        if let Ok(w) = unit.raw_get("payload") {
                            payload.insert(unit_type.clone(), w);
                        }
                        if let Ok(w) = unit.raw_get("AddPropAircraft") {
                            prop_aircraft.insert(unit_type.clone(), w);
                        }
                        if let Ok(w) = unit.raw_get("Radio") {
                            radio.insert(unit_type, w);
                        }
                    }
                }
            }
        }
        Ok(Self {
            payload,
            prop_aircraft,
            radio,
        })
    }

    fn apply(&self, lua: &Lua, objectives: &mut Vec<TriggerZone>, base: &mut LoadedMiz) -> Result<()> {
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
                for group in vehicle(&country, "plane")
                    .context("getting planes")?
                    .chain(vehicle(&country, "helicopter").context("getting helicopters")?)
                {
                    let group = group.context("getting group")?;
                    for unit in group
                        .raw_get::<_, Table>("units")
                        .context("getting units")?
                        .pairs::<Value, Table>()
                    {
                        let unit = unit.context("getting unit")?.1;
                        // skip ai aircraft
                        if unit.raw_get::<_, String>("skill")?.as_str() != "Client" {
                            continue
                        }
                        let unit_type: String = unit.raw_get("type")?;
                        match self.payload.get(&unit_type) {
                            Some(w) => unit.set("payload", w.deep_clone(lua)?)?,
                            None => warn!("no payload table for {unit_type}"),
                        }
                        if let Some(w) = self.prop_aircraft.get(&unit_type) {
                            unit.set("AddPropAircraft", w.deep_clone(lua)?)?
                        }
                        if let Some(w) = self.radio.get(&unit_type) {
                            unit.set("Radio", w.deep_clone(lua)?)?
                        }
                        increment_key(&mut replace_count, &unit_type);
                        let x = unit.get("x")?;
                        let y = unit.get("y")?;
                        for trigger_zone in &mut *objectives {
                            if trigger_zone.vec2_in_zone(x, y) {
                                let count =
                                    increment_key(&mut trigger_zone.spawn_count, &unit_type);
                                let new_name = format!(
                                    "{} {} {}",
                                    trigger_zone.objective_name, &unit_type, count
                                );
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
        Ok(())
    }
}

struct WarehouseTemplate {
    blue_inventory: Table<'static>,
    red_inventory: Table<'static>,
    default: Table<'static>,
}

impl WarehouseTemplate {
    fn new(wht: &LoadedMiz, cfg: &Miz) -> Result<Self> {
        let mut blue_inventory_id = 0;
        let mut red_inventory_id = 0;
        let mut default_id = 0;
        for coa in wht
            .mission
            .raw_get::<_, Table>("coalition")?
            .pairs::<Value, Table>()
        {
            let coa = coa?.1;
            for country in coa.raw_get::<_, Table>("country")?.pairs::<Value, Table>() {
                let country = country?.1;
                for group in vehicle(&country, "static")? {
                    let group = group?;
                    for unit in group.raw_get::<_, Table>("units")?.pairs::<Value, Table>() {
                        let unit = unit?.1;
                        if unit.raw_get::<_, String>("type")? == "Invisible FARP" {
                            let name = unit.raw_get::<_, String>("name")?;
                            let id = unit.raw_get::<_, i64>("unitId")?;
                            if name == "DEFAULT" {
                                default_id = id;
                            } else if name == cfg.blue_production_template {
                                blue_inventory_id = id;
                            } else if name == cfg.red_production_template {
                                red_inventory_id = id;
                            } else {
                                bail!(
                                    "invalid warehouse template, unexpected {name} invisible farp"
                                )
                            }
                        }
                    }
                }
            }
        }
        if blue_inventory_id == 0 {
            bail!(
                "missing warehouse template {}",
                cfg.blue_production_template
            )
        }
        if red_inventory_id == 0 {
            bail!("missing warehouse template {}", cfg.red_production_template)
        }
        if default_id == 0 {
            bail!("missing warehouse template DEFAULT")
        }
        let warehouses = wht
            .warehouses
            .raw_get::<_, Table>("warehouses")
            .context("getting warehouses")?;
        Ok(Self {
            blue_inventory: warehouses
                .raw_get(blue_inventory_id)
                .context("getting blue inventory")?,
            red_inventory: warehouses
                .raw_get(red_inventory_id)
                .context("getting red inventory")?,
            default: warehouses
                .raw_get(default_id)
                .context("getting default inventory")?,
        })
    }

    fn apply(&self, lua: &Lua, cfg: &Miz, base: &mut LoadedMiz) -> Result<()> {
        let mut blue_inventory = 0;
        let mut red_inventory = 0;
        let mut whids = vec![];
        for coa in base
            .mission
            .raw_get::<_, Table>("coalition")?
            .pairs::<Value, Table>()
        {
            let coa = coa?.1;
            for country in coa.raw_get::<_, Table>("country")?.pairs::<Value, Table>() {
                let country = country?.1;
                if let Ok(iter) = vehicle(&country, "static") {
                    for group in iter {
                        let group = group?;
                        for unit in group.raw_get::<_, Table>("units")?.pairs::<Value, Table>() {
                            let unit = unit?.1;
                            let typ: String = unit.raw_get("type")?;
                            let name: String = unit.raw_get("name")?;
                            let id: i64 = unit.raw_get("unitId")?;
                            if typ == "FARP"
                                || typ == "SINGLE_HELIPAD"
                                || typ == "FARP_SINGLE_01"
                                || typ == "Invisible FARP"
                            {
                                if name == cfg.blue_production_template {
                                    blue_inventory = id;
                                } else if name == cfg.red_production_template {
                                    red_inventory = id;
                                } else {
                                    whids.push(id);
                                }
                            }
                        }
                    }
                }
            }
        }
        let airports = base
            .warehouses
            .raw_get::<_, Table>("airports")
            .context("getting airports")?;
        let warehouses = base
            .warehouses
            .raw_get::<_, Table>("warehouses")
            .context("getting warehouses")?;
        let mut airport_ids = vec![];
        for wh in airports.clone().pairs::<i64, Table>() {
            let (id, _) = wh?;
            airport_ids.push(id);
        }
        for id in airport_ids {
            airports
                .set(id, self.default.deep_clone(lua)?)
                .with_context(|| format!("setting airport {id}"))?;
        }
        for id in whids {
            warehouses
                .set(id, self.default.deep_clone(lua)?)
                .with_context(|| format!("setting warehouse {id}"))?
        }
        warehouses
            .set(red_inventory, self.red_inventory.deep_clone(lua)?)
            .context("setting red inventory")?;
        warehouses
            .set(blue_inventory, self.blue_inventory.deep_clone(lua)?)
            .context("setting blue inventory")?;
        base.warehouses.set("airports", airports)?;
        base.warehouses.set("warehouses", warehouses)?;
        Ok(())
    }
}

fn compile_objectives(base: &LoadedMiz) -> Result<Vec<TriggerZone>> {
    let mut objectives = Vec::new();
    for zone in base
        .mission
        .raw_get::<_, Table>("triggers")
        .context("getting triggers")?
        .raw_get::<_, Table>("zones")
        .context("getting zones")?
        .pairs::<Value, Table>()
    {
        let zone = zone?.1;
        if let Some(t) = TriggerZone::new(&zone)? {
            objectives.push(t);
        }
    }
    Ok(objectives)
}

pub fn run(cfg: &Miz) -> Result<()> {
    let lua = Box::leak(Box::new(Lua::new()));
    lua.gc_stop();
    let mut base = LoadedMiz::new(lua, &cfg.base).context("loading base mission")?;
    let mut objectives = compile_objectives(&base).context("compiling objectives")?;
    let vehicle_templates = {
        let wep = LoadedMiz::new(lua, &cfg.weapon).context("loading weapon template")?;
        VehicleTemplates::new(&wep).context("loading templates")?
    };
    let warehouse_template = match cfg.warehouse.as_ref() {
        None => None,
        Some(wh) => {
            let wht = LoadedMiz::new(lua, wh).context("loading warehouse template")?;
            Some(WarehouseTemplate::new(&wht, cfg).context("compiling warehouse template")?)
        }
    };
    vehicle_templates
        .apply(lua, &mut objectives, &mut base)
        .context("applying vehicle templates")?;
    let s = serialize_with_cycles(
        "mission".into(),
        Value::Table(base.mission.clone()),
        &mut HashMap::new(),
    );
    fs::write(&base.miz.files["mission"], s).context("writing mission file")?;
    info!("wrote serialized mission to mission file.");
    if let Some(wht) = warehouse_template {
        wht.apply(lua, &cfg, &mut base)
            .context("applying warehouse template")?;
        let s = serialize_with_cycles(
            "warehouses".into(),
            Value::Table(base.warehouses.clone()),
            &mut HashMap::new(),
        );
        fs::write(&base.miz.files["warehouses"], s).context("writing warehouse file")?;
        info!("wrote serialized warehouses to warehouse file.");
    }
    //replace options file
    let options_template = UnpackedMiz::new(&cfg.options).context("loading options template")?;
    let source_options_path = options_template.files.get("options").unwrap();
    let destination_options_path = base.miz.files.get("options").unwrap();
    fs::rename(source_options_path, destination_options_path)
        .context("replacing the options file")?;
    info!("replaced options file from {:?}", &cfg.options);
    info!("saving finalized mission to {:?}", cfg.output);
    base.miz.pack(&cfg.output).context("repacking mission")?;
    Ok(())
}
