//shell script -> pass in config (gets theatre/era from base miz) -> create both missions(clones) -> set server config
//start server

//on mission load end: crack open ~other~ mission, apply (all?) templates, resave

//save mission values in a struct

//crack open miz

//deserialize mission table

//edit mission table (crack open templates 1 at a time)

//repack miz
use crate::MizCmd;
use anyhow::{anyhow, bail, Context, Result};
use compact_str::{format_compact, CompactStringExt};
use dcso3::{
    azumith2d, coalition::Side, controller::{MissionPoint, PointType}, country::Country, env::miz::{Group, Miz, Property, TriggerZoneTyp}, normal2, LuaVec2, Quad2, Sequence, String, Vector2
};
use log::{info, warn};
use mlua::{FromLua, IntoLua, Lua, Table, Value};
use std::{
    collections::HashMap, fs::{self, File}, io::{self, BufWriter}, ops::Deref, path::{Path, PathBuf}, str::FromStr
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
                objective_name: String::from(&name[4..]),
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
                .with_context(|| format_compact!("getting file {i}"))?;
            let dump_path = root.join(file.name());
            let dump_root = dump_path.parent().unwrap();
            fs::create_dir_all(dump_root)
                .with_context(|| format_compact!("creating {dump_root:?}"))?;
            let mut extracted_file = File::create(&dump_path)
                .with_context(|| format_compact!("creating {dump_path:?}"))?;
            io::copy(&mut file, &mut extracted_file)
                .with_context(|| format_compact!("copying {i} to {dump_path:?}"))?;
            files.insert(String::from(file.name()), dump_path);
        }
        Ok(Self { root, files })
    }

    fn pack(&self, destination_file: &Path) -> Result<()> {
        info!("repacking current miz to: {destination_file:?}");
        let file = File::create(&destination_file)
            .with_context(|| format_compact!("creating {:?}", destination_file))?;
        let zip_file = BufWriter::new(file);
        let mut zip_writer = ZipWriter::new(zip_file);
        for (_, file_path) in &self.files {
            if file_path.is_dir() {
                continue;
            }
            let mut file = File::open(file_path)
                .with_context(|| format_compact!("opening file {:?}", file_path))?;
            let relative_path = file_path.strip_prefix(&self.root).with_context(|| {
                format_compact!("stripping {:?} from file {file_path:?}", self.root)
            })?;
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
        Value::Integer(i) => String::from(format_compact!("{i}")),
        Value::Number(n) => String::from(format_compact!("{n}")),
        Value::Boolean(b) => String::from(format_compact!("{b}")),
        Value::String(s) => String::from(format_compact!("{:?}", s.to_str().unwrap())),
        _ => String::from(""),
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
        serialized.push(String::from(format_compact!("{} = ", name)));
        if value.type_name() == "number"
            || value.type_name() == "integer"
            || value.type_name() == "string"
            || value.type_name() == "boolean"
        {
            serialized.push(String::from(format_compact!("{}\n", key)));
        } else {
            if saved.contains_key(key) {
                serialized.push(String::from(format_compact!("{}\n", saved[key])));
            } else {
                saved.insert(name.clone(), basic_serialize(&value));
                serialized.push(String::from("{}\n"));

                match value {
                    Value::Table(t) => {
                        for r in t.pairs::<Value, Value>() {
                            let (k, v) = r.unwrap();
                            let field_name =
                                String::from(format_compact!("{}[{}]", name, basic_serialize(&k)));
                            serialized.push(serialize_with_cycles(field_name, v, saved));
                        }
                    }
                    _ => (),
                }
            }
        }
        String::from(serialized.concat_compact())
    } else {
        String::from("")
    }
}

struct LoadedMiz {
    miz: UnpackedMiz,
    mission: Miz<'static>,
    #[allow(dead_code)]
    options: Table<'static>,
    #[allow(dead_code)]
    warehouses: Table<'static>,
}

impl LoadedMiz {
    fn new(lua: &'static Lua, path: &Path) -> Result<Self> {
        let miz = UnpackedMiz::new(path).with_context(|| format_compact!("unpacking {path:?}"))?;
        let mut mission = lua.create_table()?;
        let mut options = lua.create_table()?;
        let mut warehouses = lua.create_table()?;
        for (file_name, file) in &miz.files {
            if **file_name != "mission" && **file_name != "warehouses" && **file_name != "options" {
                continue;
            }
            info!("processing {file_name}");
            let file_content = fs::read_to_string(file)
                .with_context(|| format_compact!("error reading file {file:?}"))?;
            lua.load(&file_content)
                .exec()
                .with_context(|| format_compact!("loading {file_name} into lua"))?;
            if **file_name == "mission" {
                mission = lua
                    .globals()
                    .raw_get("mission")
                    .context("extracting mission")?;
            }
            if **file_name == "warehouses" {
                warehouses = lua
                    .globals()
                    .raw_get("warehouses")
                    .context("extracting warehouses")?;
            }
            if **file_name == "options" {
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
            mission: Miz::from_lua(Value::Table(mission), lua)?,
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
    let n = map.entry(String::from(key)).or_default();
    *n += 1;
    *n
}

struct SlotSpec(HashMap<Side, HashMap<String, usize>>);

impl Deref for SlotSpec {
    type Target = HashMap<Side, HashMap<String, usize>>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl SlotSpec {
    fn new(props: Sequence<Property>) -> Result<Self> {
        let mut spec: HashMap<Side, HashMap<String, usize>> = HashMap::default();
        let mut side = None;
        for prop in props {
            let prop = prop?;
            match Side::from_str(&prop.key) {
                Ok(s) => side = Some(s),
                Err(_) => match side {
                    None => bail!("expected Blue or Red before airframe declarations"),
                    Some(side) => {
                        *spec.entry(side).or_default().entry(prop.key).or_default() +=
                            prop.value.parse::<usize>()?
                    }
                },
            }
        }
        Ok(Self(spec))
    }
}

trait PosGenerator {
    fn next(&mut self) -> Result<Vector2>;
    fn azumith(&self) -> f64;
}

struct SlotGrid {
    quad: Quad2,
    cr: Vector2,
    row_az: f64,
    row: Vector2,
    column: Vector2,
    current: Vector2,
}

impl SlotGrid {
    fn new(quad: Quad2) -> Result<SlotGrid> {
        let (p0, p1, _) = quad.longest_edge();
        let column = (p0 - p1).normalize();
        let row = normal2(column).normalize();
        // unit vectors pointing along the row and column axis of the grid that starts
        // at p0 and ends at p1
        let (row, column) = if quad.contains(LuaVec2(p0 + column + row)) {
            (row, column)
        } else if quad.contains(LuaVec2(p0 + column - row)) {
            (-row, column)
        } else if quad.contains(LuaVec2(p0 - column + row)) {
            (row, -column)
        } else if quad.contains(LuaVec2(p0 - column - row)) {
            (-row, -column)
        } else {
            bail!("the area is too thin")
        };
        Ok(Self {
            quad,
            cr: p0,
            row_az: azumith2d(row),
            row,
            column,
            current: p0,
        })
    }
}

impl PosGenerator for SlotGrid {
    fn next(&mut self) -> Result<Vector2> {
        let p = self.current + self.column * 25.;
        if self.quad.contains(LuaVec2(p)) {
            self.current = p;
            Ok(p)
        } else {
            let cr = self.cr + self.row * 25.;
            let p = cr + self.column * 25.;
            if self.quad.contains(LuaVec2(p)) {
                self.cr = cr;
                self.current = p;
                Ok(p)
            } else {
                bail!("zone is full")
            }
        }
    }

    fn azumith(&self) -> f64 {
        self.row_az
    }
}

#[derive(Clone, Copy)]
enum SlotType {
    Plane,
    Helicopter,
}

struct VehicleTemplates {
    plane_slots: HashMap<String, Group<'static>>,
    helicopter_slots: HashMap<String, Group<'static>>,
    payload: HashMap<String, Table<'static>>,
    prop_aircraft: HashMap<String, Table<'static>>,
    radio: HashMap<String, Table<'static>>,
    frequency: HashMap<String, Value<'static>>,
}

impl VehicleTemplates {
    fn new(wep: &LoadedMiz) -> Result<Self> {
        let mut plane_slots: HashMap<String, Group> = HashMap::new();
        let mut helicopter_slots: HashMap<String, Group> = HashMap::new();
        let mut payload: HashMap<String, Table> = HashMap::new();
        let mut prop_aircraft: HashMap<String, Table> = HashMap::new();
        let mut radio: HashMap<String, Table> = HashMap::new();
        let mut frequency: HashMap<String, Value> = HashMap::new();
        for coa in [Side::Blue, Side::Red]
            .into_iter()
            .map(|side| wep.mission.coalition(side))
        {
            let coa = coa?;
            for country in coa.countries()? {
                let country = country?;
                for (st, group) in country
                    .planes()
                    .context("getting planes")?
                    .into_iter()
                    .map(|p| (SlotType::Plane, p))
                    .chain(
                        country
                            .helicopters()
                            .context("getting helicopters")?
                            .into_iter()
                            .map(|p| (SlotType::Helicopter, p)),
                    )
                {
                    let group = group?;
                    for unit in group
                        .raw_get::<_, Table>("units")
                        .context("getting units")?
                        .pairs::<Value, Table>()
                    {
                        let unit = unit?.1;
                        let unit_type: String = unit.raw_get("type").context("getting units")?;
                        match st {
                            SlotType::Helicopter => &mut helicopter_slots,
                            SlotType::Plane => &mut plane_slots,
                        }
                        .insert(unit_type.clone(), group.clone());
                        info!("adding payload template: {unit_type}");
                        if let Ok(w) = unit.raw_get("payload") {
                            payload.insert(unit_type.clone(), w);
                        }
                        if let Ok(w) = unit.raw_get("AddPropAircraft") {
                            prop_aircraft.insert(unit_type.clone(), w);
                        }
                        if let Ok(w) = unit.raw_get("Radio") {
                            radio.insert(unit_type.clone(), w);
                        }
                        if let Ok(v) = unit.raw_get("frequency") {
                            frequency.insert(unit_type, v);
                        }
                    }
                }
            }
        }
        Ok(Self {
            plane_slots,
            helicopter_slots,
            payload,
            prop_aircraft,
            radio,
            frequency,
        })
    }

    fn generate_slots(&self, lua: &Lua, base: &mut LoadedMiz) -> Result<()> {
        let idx = base.mission.index()?;
        let mut next_uid = idx.max_uid().next();
        let mut next_gid = idx.max_gid().next();
        for zone in base.mission.triggers()? {
            let zone = zone?;
            if !zone.name()?.starts_with("TS") {
                continue;
            }
            for (side, slots) in &*SlotSpec::new(zone.properties()?)? {
                let mut posgen: Box<dyn PosGenerator> = match zone.typ()? {
                    TriggerZoneTyp::Circle { radius: _ } => {
                        unimplemented!()
                    }
                    TriggerZoneTyp::Quad(quad) => Box::new(SlotGrid::new(quad)?),
                };
                let coa = base.mission.coalition(*side)?;
                let cname = match side {
                    Side::Blue => Country::CJTF_BLUE,
                    Side::Red => Country::CJTF_RED,
                    Side::Neutral => unreachable!(),
                };
                let country = coa
                    .country(cname)?
                    .ok_or_else(|| anyhow!("you must have CJTF_BLUE and CJTF_RED in your miz"))?;
                let helicopters = country.helicopters()?;
                let planes = country.planes()?;
                for (vehicle, n) in slots {
                    let (seq, tmpl) = match self.plane_slots.get(vehicle) {
                        Some(t) => (&planes, t),
                        None => match self.helicopter_slots.get(vehicle) {
                            Some(t) => (&helicopters, t),
                            None => bail!("missing required slot template {vehicle}"),
                        },
                    };
                    for _ in 0..*n {
                        let tmpl = tmpl.deep_clone(lua)?;
                        let pos = posgen.next()?;
                        let route = tmpl.route()?;
                        let mut has_ground_start = false;
                        route.set_points(
                            route
                                .points()?
                                .into_iter()
                                .map(|p| {
                                    let mut p = p?;
                                    match p.typ {
                                        PointType::TakeOffGround | PointType::TakeOffGroundHot => {
                                            has_ground_start = true;
                                            p.pos = LuaVec2(pos);
                                        }
                                        _ => (),
                                    }
                                    Ok(p)
                                })
                                .collect::<Result<Vec<MissionPoint>>>()?,
                        )?;
                        if !has_ground_start {
                            bail!("slot template aircraft must be ground starts")
                        }
                        tmpl.set_route(route)?;
                        tmpl.set_id(next_gid)?;
                        for u in tmpl.units()? {
                            let u = u?;
                            u.set_id(next_uid)?;
                            u.set_heading(posgen.azumith())?;
                            next_uid = next_uid.next();
                        }
                        next_gid = next_gid.next();
                        seq.push(tmpl)?;
                    }
                }
            }
        }
        Ok(())
    }

    fn apply(
        &self,
        lua: &Lua,
        objectives: &mut Vec<TriggerZone>,
        base: &mut LoadedMiz,
    ) -> Result<()> {
        let mut slots: HashMap<String, HashMap<String, usize>> = HashMap::default();
        let mut replace_count: HashMap<String, isize> = HashMap::new();
        let mut stn = 1u64;
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
                            continue;
                        }
                        let unit_type: String = unit.raw_get("type")?;
                        match self.payload.get(&unit_type) {
                            Some(w) => unit.set("payload", w.deep_clone(lua)?)?,
                            None => warn!("no payload table for {unit_type}"),
                        }
                        let stn_string = match self.prop_aircraft.get(&unit_type) {
                            None => String::from(""),
                            Some(tmpl) => {
                                let tmpl = tmpl.deep_clone(lua)?;
                                let stn = if tmpl.contains_key("STN_L16")? {
                                    tmpl.raw_set(
                                        "STN_L16",
                                        String::from(format_compact!("{:005o}", stn)),
                                    )?;
                                    let s = String::from(format_compact!(" STN#{:005o}", stn));
                                    stn += 1;
                                    s
                                } else {
                                    String::from("")
                                };
                                unit.set("AddPropAircraft", tmpl)?;
                                stn
                            }
                        };
                        if let Some(w) = self.radio.get(&unit_type) {
                            unit.set("Radio", w.deep_clone(lua)?)?
                        }
                        if let Some(v) = self.frequency.get(&unit_type) {
                            unit.set("frequency", v.deep_clone(lua)?)?
                        }
                        increment_key(&mut replace_count, &unit_type);
                        let x = unit.get("x")?;
                        let y = unit.get("y")?;
                        for trigger_zone in &mut *objectives {
                            if trigger_zone.vec2_in_zone(x, y) {
                                let count =
                                    increment_key(&mut trigger_zone.spawn_count, &unit_type);
                                let new_name = String::from(format_compact!(
                                    "{} {} {}{}",
                                    trigger_zone.objective_name,
                                    &unit_type,
                                    count,
                                    stn_string
                                ));
                                unit.set("name", new_name.clone())?;
                                group.set("name", new_name)?;
                                if let Some(cnt) = slots
                                    .entry(trigger_zone.objective_name.clone())
                                    .or_insert_with(|| {
                                        HashMap::from_iter(
                                            self.payload.keys().map(|typ| (typ.clone(), 0)),
                                        )
                                    })
                                    .get_mut(&unit_type)
                                {
                                    *cnt += 1;
                                }
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
        for (obj, slots) in slots {
            info!("objective {obj} slots:");
            let mut slots = Vec::from_iter(slots);
            slots.sort_by(|(_, c0), (_, c1)| c0.cmp(c1));
            for (typ, cnt) in slots {
                info!("    {typ}: {cnt}")
            }
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
    fn new(wht: &LoadedMiz, cfg: &MizCmd) -> Result<Self> {
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
                        if *unit.raw_get::<_, String>("type")? == "Invisible FARP" {
                            let name = unit.raw_get::<_, String>("name")?;
                            let id = unit.raw_get::<_, i64>("unitId")?;
                            if *name == "DEFAULT" {
                                default_id = id;
                            } else if *name == cfg.blue_production_template {
                                blue_inventory_id = id;
                            } else if *name == cfg.red_production_template {
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

    fn apply(&self, lua: &Lua, cfg: &MizCmd, base: &mut LoadedMiz) -> Result<()> {
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
                            if *typ == "FARP"
                                || *typ == "SINGLE_HELIPAD"
                                || *typ == "FARP_SINGLE_01"
                                || *typ == "Invisible FARP"
                            {
                                if *name == cfg.blue_production_template {
                                    blue_inventory = id;
                                } else if *name == cfg.red_production_template {
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
                .with_context(|| format_compact!("setting airport {id}"))?;
        }
        for id in whids {
            warehouses
                .set(id, self.default.deep_clone(lua)?)
                .with_context(|| format_compact!("setting warehouse {id}"))?
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

pub fn run(cfg: &MizCmd) -> Result<()> {
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
        .generate_slots(lua, &mut base)
        .context("generating slots")?;
    vehicle_templates
        .apply(lua, &mut objectives, &mut base)
        .context("applying vehicle templates")?;
    let s = serialize_with_cycles(
        "mission".into(),
        Value::Table((&*base.mission).clone()),
        &mut HashMap::new(),
    );
    fs::write(&base.miz.files["mission"], &*s).context("writing mission file")?;
    info!("wrote serialized mission to mission file.");
    if let Some(wht) = warehouse_template {
        wht.apply(lua, &cfg, &mut base)
            .context("applying warehouse template")?;
        let s = serialize_with_cycles(
            "warehouses".into(),
            Value::Table(base.warehouses.clone()),
            &mut HashMap::new(),
        );
        fs::write(&base.miz.files["warehouses"], &*s).context("writing warehouse file")?;
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
