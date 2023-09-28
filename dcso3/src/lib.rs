use compact_str::CompactString;
use fxhash::FxHashMap;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::{
    collections::hash_map::Entry,
    ops::{Deref, DerefMut},
};

use self::coalition::Side;
pub mod airbase;
pub mod attribute;
pub mod coalition;
pub mod controller;
pub mod country;
pub mod event;
pub mod group;
pub mod object;
pub mod static_object;
pub mod unit;
pub mod warehouse;
pub mod weapon;
pub mod world;

#[macro_export]
macro_rules! wrapped_table {
    ($name:ident, $class:expr) => {
        #[derive(Debug, Clone, Serialize)]
        pub struct $name<'lua> {
            t: mlua::Table<'lua>,
            #[allow(dead_code)]
            #[serde(skip)]
            lua: &'lua Lua,
        }

        impl<'lua> Deref for $name<'lua> {
            type Target = mlua::Table<'lua>;

            fn deref(&self) -> &Self::Target {
                &self.t
            }
        }

        impl<'lua> FromLua<'lua> for $name<'lua> {
            fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
                Ok(Self {
                    t: as_tbl(stringify!($name), $class, value)?,
                    lua,
                })
            }
        }

        impl<'lua> IntoLua<'lua> for $name<'lua> {
            fn into_lua(self, _lua: &'lua Lua) -> LuaResult<Value<'lua>> {
                Ok(Value::Table(self.t))
            }
        }
    };
}

#[macro_export]
macro_rules! simple_enum {
    ($name:ident, $repr:ident, [$($case:ident => $num:literal),+]) => {
        #[derive(Debug, Clone, Copy, Serialize)]
        #[allow(non_camel_case_types)]
        #[repr($repr)]
        pub enum $name {
            $($case = $num),+
        }

        impl<'lua> FromLua<'lua> for $name {
            fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
                Ok(match $repr::from_lua(value, lua)? {
                    $($num => Self::$case),+,
                    _ => return Err(cvt_err(stringify!($name)))
                })
            }
        }

        impl<'lua> IntoLua<'lua> for $name {
            fn into_lua(self, _lua: &'lua Lua) -> LuaResult<Value<'lua>> {
                Ok(Value::Integer(self as i64))
            }
        }
    };
}

#[macro_export]
macro_rules! bitflags_enum {
    ($name:ident, $repr:ident, [$($case:ident => $num:literal),+]) => {
        #[bitflags]
        #[derive(Debug, Clone, Copy, Serialize)]
        #[allow(non_camel_case_types)]
        #[repr($repr)]
        pub enum $name {
            $($case = $num),+
        }

        impl<'lua> FromLua<'lua> for $name {
            fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
                Ok(match $repr::from_lua(value, lua)? {
                    $($num => Self::$case),+,
                    _ => return Err(cvt_err(stringify!($name)))
                })
            }
        }

        impl<'lua> IntoLua<'lua> for $name {
            fn into_lua(self, _lua: &'lua Lua) -> LuaResult<Value<'lua>> {
                Ok(Value::Integer(self as i64))
            }
        }
    };
}

#[macro_export]
macro_rules! string_enum {
    ($name:ident, $repr:ident, [$($case:ident => $str:literal),+]) => {
        string_enum!($name, $repr, [$($case => $str),+], []);
    };
    ($name:ident,
     $repr:ident,
     [$($case:ident => $str:literal),+],
     [$($altcase:ident => $altstr:literal),*]) => {
        #[derive(Debug, Clone, Serialize)]
        #[allow(non_camel_case_types)]
        #[repr($repr)]
        pub enum $name {
            $($case),+,
            Custom(String)
        }

        impl<'lua> FromLua<'lua> for $name {
            fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
                let s = String::from_lua(value, lua)?;
                Ok(match s.as_str() {
                    $($str => Self::$case,)+
                    $($altstr => Self::$altcase,)*
                    _ => Self::Custom(s)
                })
            }
        }

        impl<'lua> IntoLua<'lua> for $name {
            fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
                Ok(Value::String(match self {
                    $(Self::$case => lua.create_string($str)?),+,
                    Self::Custom(s) => lua.create_string(s.as_str())?
                }))
            }
        }
    };
}

fn cvt_err(to: &'static str) -> LuaError {
    LuaError::FromLuaConversionError {
        from: "value",
        to,
        message: None,
    }
}

fn as_tbl_ref<'a: 'lua, 'lua>(
    to: &'static str,
    value: &'a Value<'lua>,
) -> LuaResult<&'a mlua::Table<'lua>> {
    value.as_table().ok_or_else(|| cvt_err(to))
}

fn check_implements(mut tbl: mlua::Table, class: &str) -> bool {
    loop {
        match tbl.raw_get::<_, String>("className_") {
            Err(_) => break false,
            Ok(s) if s.as_str() == class => break true,
            Ok(_) => match tbl.raw_get::<_, mlua::Table>("parentClass_") {
                Err(_) => break false,
                Ok(t) => {
                    tbl = t;
                }
            },
        }
    }
}

fn as_tbl<'lua>(
    to: &'static str,
    objtyp: Option<&'static str>,
    value: Value<'lua>,
) -> LuaResult<mlua::Table<'lua>> {
    match value {
        Value::Table(tbl) => match objtyp {
            None => Ok(tbl),
            Some(typ) => match tbl.get_metatable() {
                None => Err(LuaError::FromLuaConversionError {
                    from: "table",
                    to: typ,
                    message: Some(format!("table is not an object")),
                }),
                Some(meta) => {
                    if check_implements(meta, typ) {
                        Ok(tbl)
                    } else {
                        Err(LuaError::FromLuaConversionError {
                            from: "table",
                            to: typ,
                            message: Some(format!("object or super expected to have type {}", typ)),
                        })
                    }
                }
            },
        },
        _ => Err(cvt_err(to)),
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vec2 {
    x: f64,
    y: f64,
}

impl<'lua> FromLua<'lua> for Vec2 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Vec2", None, value)?;
        Ok(Self {
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Vec3 {
    pub x: f64,
    pub y: f64,
    pub z: f64,
}

impl<'lua> FromLua<'lua> for Vec3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Vec3", None, value)?;
        Ok(Self {
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
            z: tbl.raw_get("z")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Position3 {
    p: Vec3,
    x: Vec3,
    y: Vec3,
    z: Vec3,
}

impl<'lua> FromLua<'lua> for Position3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Position3", None, value)?;
        Ok(Self {
            p: tbl.raw_get("p")?,
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
            z: tbl.raw_get("z")?,
        })
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct Box3 {
    pub min: Vec3,
    pub max: Vec3,
}

impl<'lua> FromLua<'lua> for Box3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Box3", None, value)?;
        Ok(Self {
            min: tbl.raw_get("min")?,
            max: tbl.raw_get("max")?,
        })
    }
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[repr(transparent)]
pub struct String(CompactString);

impl std::fmt::Display for String {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Deref for String {
    type Target = compact_str::CompactString;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for String {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'lua> IntoLua<'lua> for String {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::String(lua.create_string(self.0)?))
    }
}

impl<'lua> FromLua<'lua> for String {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        use compact_str::format_compact;
        match value {
            Value::String(s) => Ok(Self(CompactString::from(s.to_str()?))),
            Value::Boolean(b) => Ok(Self(format_compact!("{b}"))),
            Value::Integer(n) => Ok(Self(format_compact!("{n}"))),
            Value::Number(n) => Ok(Self(format_compact!("{n}"))),
            v => Ok(Self(CompactString::from(v.to_string()?))),
        }
    }
}

impl From<&str> for String {
    fn from(value: &str) -> Self {
        Self(CompactString::from(value))
    }
}

impl From<std::string::String> for String {
    fn from(value: std::string::String) -> Self {
        Self(CompactString::from(value))
    }
}

impl From<CompactString> for String {
    fn from(value: CompactString) -> Self {
        Self(value)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd, Serialize)]
pub struct Time(f32);

impl<'lua> FromLua<'lua> for Time {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(value.as_f32().ok_or_else(|| cvt_err("Time"))?))
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum VolumeType {
    Segment,
    Box,
    Sphere,
    Pyramid,
}

pub struct UserHooks<'lua> {
    on_mission_load_begin: Option<mlua::Function<'lua>>,
    on_mission_load_progress: Option<mlua::Function<'lua>>,
    on_mission_load_end: Option<mlua::Function<'lua>>,
    on_simulation_start: Option<mlua::Function<'lua>>,
    on_simulation_stop: Option<mlua::Function<'lua>>,
    on_simulation_frame: Option<mlua::Function<'lua>>,
    on_simulation_pause: Option<mlua::Function<'lua>>,
    on_simulation_resume: Option<mlua::Function<'lua>>,
    on_player_connect: Option<mlua::Function<'lua>>,
    on_player_disconnect: Option<mlua::Function<'lua>>,
    on_player_start: Option<mlua::Function<'lua>>,
    on_player_stop: Option<mlua::Function<'lua>>,
    on_player_change_slot: Option<mlua::Function<'lua>>,
    on_player_try_connect: Option<mlua::Function<'lua>>,
    on_player_try_send_chat: Option<mlua::Function<'lua>>,
    on_player_try_change_slot: Option<mlua::Function<'lua>>,
    lua: &'lua Lua,
}

impl<'lua> UserHooks<'lua> {
    pub fn new(lua: &'lua Lua) -> Self {
        Self {
            on_mission_load_begin: None,
            on_mission_load_progress: None,
            on_mission_load_end: None,
            on_simulation_start: None,
            on_simulation_stop: None,
            on_simulation_frame: None,
            on_simulation_pause: None,
            on_simulation_resume: None,
            on_player_connect: None,
            on_player_disconnect: None,
            on_player_start: None,
            on_player_stop: None,
            on_player_change_slot: None,
            on_player_try_change_slot: None,
            on_player_try_connect: None,
            on_player_try_send_chat: None,
            lua,
        }
    }

    pub fn register(&mut self) -> LuaResult<()> {
        let Self {
            on_mission_load_begin,
            on_mission_load_progress,
            on_mission_load_end,
            on_simulation_start,
            on_simulation_stop,
            on_simulation_frame,
            on_simulation_pause,
            on_simulation_resume,
            on_player_connect,
            on_player_disconnect,
            on_player_start,
            on_player_stop,
            on_player_change_slot,
            on_player_try_connect,
            on_player_try_send_chat,
            on_player_try_change_slot,
            lua: _,
        } = self;
        let tbl = self.lua.create_table()?;
        if let Some(f) = on_mission_load_begin.take() {
            tbl.set("onMissionLoadBegin", f)?;
        }
        if let Some(f) = on_mission_load_progress.take() {
            tbl.set("onMissionLoadProgress", f)?;
        }
        if let Some(f) = on_mission_load_end.take() {
            tbl.set("onMissionLoadEnd", f)?;
        }
        if let Some(f) = on_simulation_start.take() {
            tbl.set("onSimulationStart", f)?;
        }
        if let Some(f) = on_simulation_stop.take() {
            tbl.set("onSimulationStop", f)?;
        }
        if let Some(f) = on_simulation_frame.take() {
            tbl.set("onSimulationFrame", f)?;
        }
        if let Some(f) = on_simulation_pause.take() {
            tbl.set("onSimulationPause", f)?;
        }
        if let Some(f) = on_simulation_resume.take() {
            tbl.set("onSimulationResume", f)?;
        }
        if let Some(f) = on_player_connect.take() {
            tbl.set("onPlayerConnect", f)?;
        }
        if let Some(f) = on_player_disconnect.take() {
            tbl.set("onPlayerDisconnect", f)?;
        }
        if let Some(f) = on_player_start.take() {
            tbl.set("onPlayerStart", f)?;
        }
        if let Some(f) = on_player_stop.take() {
            tbl.set("onPlayerStop", f)?;
        }
        if let Some(f) = on_player_change_slot.take() {
            tbl.set("onPlayerChangeSlot", f)?;
        }
        if let Some(f) = on_player_try_connect.take() {
            tbl.set("onPlayerTryConnect", f)?;
        }
        if let Some(f) = on_player_try_send_chat.take() {
            tbl.set("onPlayerTrySendChat", f)?;
        }
        if let Some(f) = on_player_try_change_slot.take() {
            tbl.set("onPlayerTryChangeSlot", f)?;
        }
        let dcs = as_tbl("DCS", None, self.lua.globals().raw_get("DCS")?)?;
        dcs.call_method("setUserCallbacks", tbl)
    }

    pub fn on_mission_load_begin<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua) -> LuaResult<()> + 'static,
    {
        self.on_mission_load_begin = Some(self.lua.create_function(move |lua, ()| f(lua))?);
        Ok(self)
    }

    /// f(progress, message)
    pub fn on_mission_load_progress<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, String, String) -> LuaResult<()> + 'static,
    {
        self.on_mission_load_progress = Some(
            self.lua
                .create_function(move |lua, (progress, message)| f(lua, progress, message))?,
        );
        Ok(self)
    }

    pub fn on_mission_load_end<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua) -> LuaResult<()> + 'static,
    {
        self.on_mission_load_end = Some(self.lua.create_function(move |lua, ()| f(lua))?);
        Ok(self)
    }

    pub fn on_simulation_start<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua) -> LuaResult<()> + 'static,
    {
        self.on_simulation_start = Some(self.lua.create_function(move |lua, ()| f(lua))?);
        Ok(self)
    }

    pub fn on_simulation_stop<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua) -> LuaResult<()> + 'static,
    {
        self.on_simulation_stop = Some(self.lua.create_function(move |lua, ()| f(lua))?);
        Ok(self)
    }

    pub fn on_simulation_frame<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua) -> LuaResult<()> + 'static,
    {
        self.on_simulation_frame = Some(self.lua.create_function(move |lua, ()| f(lua))?);
        Ok(self)
    }

    pub fn on_simulation_pause<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua) -> LuaResult<()> + 'static,
    {
        self.on_simulation_pause = Some(self.lua.create_function(move |lua, ()| f(lua))?);
        Ok(self)
    }

    pub fn on_simulation_resume<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua) -> LuaResult<()> + 'static,
    {
        self.on_simulation_resume = Some(self.lua.create_function(move |lua, ()| f(lua))?);
        Ok(self)
    }

    pub fn on_player_connect<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, u32) -> LuaResult<()> + 'static,
    {
        self.on_player_connect = Some(self.lua.create_function(move |lua, id| f(lua, id))?);
        Ok(self)
    }

    pub fn on_player_disconnect<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, u32) -> LuaResult<()> + 'static,
    {
        self.on_player_disconnect = Some(self.lua.create_function(move |lua, id| f(lua, id))?);
        Ok(self)
    }

    pub fn on_player_start<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, u32) -> LuaResult<()> + 'static,
    {
        self.on_player_start = Some(self.lua.create_function(move |lua, id| f(lua, id))?);
        Ok(self)
    }

    pub fn on_player_stop<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, u32) -> LuaResult<()> + 'static,
    {
        self.on_player_stop = Some(self.lua.create_function(move |lua, id| f(lua, id))?);
        Ok(self)
    }

    pub fn on_player_change_slot<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, u32) -> LuaResult<()> + 'static,
    {
        self.on_player_change_slot = Some(self.lua.create_function(move |lua, id| f(lua, id))?);
        Ok(self)
    }

    /// f(addr, ucid, name, id)
    pub fn on_player_try_connect<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, String, String, String, u32) -> LuaResult<bool> + 'static,
    {
        self.on_player_try_connect = Some(
            self.lua
                .create_function(move |lua, (addr, ucid, name, id)| f(lua, addr, ucid, name, id))?,
        );
        Ok(self)
    }

    /// f(id, message, all)
    pub fn on_player_try_send_chat<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, u32, String, bool) -> LuaResult<String> + 'static,
    {
        self.on_player_try_send_chat = Some(
            self.lua
                .create_function(move |lua, (id, msg, all)| f(lua, id, msg, all))?,
        );
        Ok(self)
    }

    /// f(id, message, all)
    pub fn on_player_try_change_slot<F>(&mut self, f: F) -> LuaResult<&mut Self>
    where
        F: Fn(&Lua, u32, Side, String) -> LuaResult<bool> + 'static,
    {
        self.on_player_try_change_slot = Some(
            self.lua
                .create_function(move |lua, (id, side, slot)| f(lua, id, side, slot))?,
        );
        Ok(self)
    }
}

pub fn value_to_json(
    ctx: &mut FxHashMap<usize, String>,
    key: Option<&str>,
    v: &Value,
) -> serde_json::Value {
    use serde_json::{json, Map, Value as JVal};
    match v {
        Value::Nil => JVal::Null,
        Value::Boolean(b) => json!(b),
        Value::LightUserData(_) => json!("<LightUserData>"),
        Value::Integer(i) => json!(*i),
        Value::Number(i) => json!(*i),
        Value::UserData(_) => json!("<UserData>"),
        Value::String(s) => json!(s),
        Value::Function(_) => json!("<Function>"),
        Value::Thread(_) => json!("<Thread>"),
        Value::Error(e) => json!(format!("{e}")),
        Value::Table(tbl) => {
            let address = tbl.to_pointer() as usize;
            match ctx.entry(address) {
                Entry::Occupied(e) => json!(format!("<Table(0x{:x} {})>", address, e.get())),
                Entry::Vacant(e) => {
                    e.insert(String::from(key.unwrap_or("Root")));
                    let mut map = Map::new();
                    for pair in tbl.clone().pairs::<Value, Value>() {
                        let (k, v) = pair.unwrap();
                        let k = match value_to_json(ctx, None, &k) {
                            JVal::String(s) => s,
                            v => v.to_string(),
                        };
                        let v = value_to_json(ctx, Some(k.as_str()), &v);
                        map.insert(k, v);
                    }
                    JVal::Object(map)
                }
            }
        }
    }
}
