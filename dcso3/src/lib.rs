extern crate nalgebra as na;
use anyhow::{anyhow, bail, Result};
use compact_str::CompactString;
use fxhash::FxHashMap;
use log::error;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::{
    backtrace::Backtrace,
    borrow::Borrow,
    collections::hash_map::Entry,
    marker::PhantomData,
    ops::{Add, AddAssign, Deref, DerefMut, Sub},
    panic::{self, AssertUnwindSafe},
};

pub mod airbase;
pub mod attribute;
pub mod coalition;
pub mod controller;
pub mod coord;
pub mod country;
pub mod env;
pub mod event;
pub mod group;
pub mod hooks;
pub mod land;
pub mod lfs;
pub mod mission_commands;
pub mod net;
pub mod object;
pub mod spot;
pub mod static_object;
pub mod timer;
pub mod trigger;
pub mod unit;
pub mod warehouse;
pub mod weapon;
pub mod world;

#[macro_export]
macro_rules! atomic_id {
    ($name:ident) => {
        paste::paste! {
            #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
            pub struct $name(i64);

            impl serde::Serialize for $name {
                fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error> where S: serde::Serializer {
                    serializer.serialize_i64(self.0)
                }
            }

            pub struct [<$name Visitor>];

            impl<'de> serde::de::Visitor<'de> for [<$name Visitor>] {
                type Value = $name;

                fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
                    write!(formatter, "a i64")
                }

                fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    $name::update_max(v);
                    Ok($name(v))
                }

                fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
                where
                    E: serde::de::Error,
                {
                    let v = v as i64;
                    $name::update_max(v);
                    Ok($name(v))
                }
            }

            impl<'de> serde::Deserialize<'de> for $name {
                fn deserialize<D>(deserializer: D) -> Result<Self, D::Error> where D: serde::Deserializer<'de> {
                    deserializer.deserialize_i64([<$name Visitor>])
                }
            }

            static [<MAX_ $name:upper _ID>]: std::sync::atomic::AtomicI64 = std::sync::atomic::AtomicI64::new(0);

            impl std::fmt::Display for $name {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, "{}", self.0)
                }
            }

            impl std::default::Default for $name {
                fn default() -> Self {
                    Self::new()
                }
            }

            impl<'lua> mlua::FromLua<'lua> for $name {
                fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
                    let i = i64::from_lua(value, lua)?;
                    Ok($name(i))
                }
            }

            impl<'lua> mlua::IntoLua<'lua> for $name {
                fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
                    self.0.into_lua(lua)
                }
            }

            impl $name {
                pub fn new() -> Self {
                    Self([<MAX_ $name:upper _ID>].fetch_add(1, std::sync::atomic::Ordering::Relaxed))
                }

                fn update_max(n: i64) {
                    const O: std::sync::atomic::Ordering = std::sync::atomic::Ordering::Relaxed;
                    let _: Result<_, _> = [<MAX_ $name:upper _ID>].fetch_update(O, O, |cur| {
                        if n >= cur {
                            Some(n.wrapping_add(1))
                        } else {
                            None
                        }
                    });
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Quad2 {
    pub p0: LuaVec2,
    pub p1: LuaVec2,
    pub p2: LuaVec2,
    pub p3: LuaVec2,
}

impl<'lua> FromLua<'lua> for Quad2 {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let verts = as_tbl("Quad", None, value).map_err(lua_err)?;
        Ok(Self {
            p0: verts.raw_get(1)?,
            p1: verts.raw_get(2)?,
            p2: verts.raw_get(3)?,
            p3: verts.raw_get(4)?,
        })
    }
}

impl Quad2 {
    pub fn contains(&self, p: LuaVec2) -> bool {
        fn horizontal_ray_intersects_edge(p: &LuaVec2, v0: &LuaVec2, v1: &LuaVec2) -> bool {
            if (v0.y > p.y) == (v1.y > p.y) {
                // we're casting horizontally so we don't need to consider the case where
                // there couldn't be a horizontal intersection
                false
            } else {
                let int_x = v0.x + (p.y - v0.y) * (v1.x - v0.x) / (v1.y - v0.y);
                int_x > p.x
            }
        }
        if p == self.p0 || p == self.p1 || p == self.p2 || p == self.p3 {
            return true;
        }
        let mut intersections = 0;
        macro_rules! check_edge {
            ($v0:expr, $v1:expr) => {
                if $v0.y != $v1.y && horizontal_ray_intersects_edge(&p, &$v0, &$v1) {
                    // if the ray passes through a vertex only count it if
                    // it passes through the upper vertex (to avoid counting it twice)
                    if $v0.y == p.y || $v1.y == p.y {
                        let to_check = if $v0.y == p.y { $v1 } else { $v0 };
                        if to_check.y < p.y {
                            intersections += 1;
                        }
                    } else {
                        intersections += 1
                    }
                }
            };
        }
        check_edge!(self.p0, self.p1);
        check_edge!(self.p1, self.p2);
        check_edge!(self.p2, self.p3);
        check_edge!(self.p3, self.p0);
        intersections % 2 == 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32,
}

impl Color {
    pub fn black(a: f32) -> Color {
        Color {
            r: 0.,
            g: 0.,
            b: 0.,
            a,
        }
    }

    pub fn red(a: f32) -> Color {
        Color {
            r: 1.,
            g: 0.,
            b: 0.,
            a,
        }
    }

    pub fn white(a: f32) -> Color {
        Color {
            r: 1.,
            g: 1.,
            b: 1.,
            a,
        }
    }

    pub fn blue(a: f32) -> Color {
        Color {
            r: 0.,
            g: 0.,
            b: 1.,
            a,
        }
    }

    pub fn gray(a: f32) -> Color {
        Color {
            r: 0.25,
            g: 0.25,
            b: 0.25,
            a,
        }
    }

    pub fn green(a: f32) -> Color {
        Color {
            r: 0.,
            g: 1.,
            b: 0.,
            a,
        }
    }
}

impl<'lua> FromLua<'lua> for Color {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Color", None, value).map_err(lua_err)?;
        Ok(Self {
            r: tbl.raw_get(1)?,
            g: tbl.raw_get(2)?,
            b: tbl.raw_get(3)?,
            a: tbl.raw_get(4)?,
        })
    }
}

impl<'lua> IntoLua<'lua> for Color {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let tbl = lua.create_table()?;
        tbl.set(1, self.r)?;
        tbl.set(2, self.g)?;
        tbl.set(3, self.b)?;
        tbl.set(4, self.a)?;
        Ok(Value::Table(tbl))
    }
}

pub fn wrap_f<'lua, L: LuaEnv<'lua>, R: Default, F: FnOnce(L) -> Result<R>>(
    name: &str,
    lua: L,
    f: F,
) -> LuaResult<R> {
    let r = panic::catch_unwind(AssertUnwindSafe(|| wrap(name, f(lua))));
    match r {
        Ok(r) => r,
        Err(e) => {
            match e.downcast::<anyhow::Error>() {
                Ok(e) => error!("{} panicked {:?} {}", name, e, Backtrace::capture()),
                Err(_) => error!("{} panicked {}", name, Backtrace::capture()),
            }
            Ok(R::default())
        }
    }
}

pub fn wrap<'lua, R: Default>(name: &str, res: Result<R>) -> LuaResult<R> {
    match res {
        Ok(r) => Ok(r),
        Err(e) => {
            error!("{}: {:?}", name, e);
            Ok(R::default())
        }
    }
}

pub fn lua_err(err: anyhow::Error) -> LuaError {
    LuaError::RuntimeError(format!("{:?}", err))
}

pub trait LuaEnv<'a> {
    fn inner(self) -> &'a Lua;
}

impl<'lua> LuaEnv<'lua> for &'lua Lua {
    fn inner(self) -> &'lua Lua {
        self
    }
}

#[derive(Debug, Clone, Copy)]
pub struct HooksLua<'lua>(&'lua Lua);

impl<'lua> LuaEnv<'lua> for HooksLua<'lua> {
    fn inner(self) -> &'lua Lua {
        self.0
    }
}

#[derive(Debug, Clone, Copy)]
pub struct MizLua<'lua>(&'lua Lua);

impl<'lua> LuaEnv<'lua> for MizLua<'lua> {
    fn inner(self) -> &'lua Lua {
        self.0
    }
}

pub fn create_root_module<H, M>(lua: &Lua, init_hooks: H, init_miz: M) -> LuaResult<LuaTable>
where
    H: Fn(HooksLua) -> Result<()> + 'static,
    M: Fn(MizLua) -> Result<()> + 'static,
{
    let exports = lua.create_table()?;
    exports.set(
        "initHooks",
        lua.create_function(move |lua, _: ()| wrap_f("init_hooks", HooksLua(lua), &init_hooks))?,
    )?;
    exports.set(
        "initMiz",
        lua.create_function(move |lua, _: ()| wrap_f("init_miz", MizLua(lua), &init_miz))?,
    )?;
    Ok(exports)
}

#[macro_export]
macro_rules! wrapped_table {
    ($name:ident, $class:expr) => {
        #[derive(Clone, Serialize)]
        pub struct $name<'lua> {
            t: mlua::Table<'lua>,
            #[allow(dead_code)]
            #[serde(skip)]
            lua: &'lua Lua,
        }

        impl<'lua> std::fmt::Debug for $name<'lua> {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                match self.t.raw_get::<&str, Value>("id_") {
                    Ok(Value::Nil) => {
                        let mut tbl = fxhash::FxHashMap::default();
                        let v = crate::value_to_json(&mut tbl, None, &Value::Table(self.t.clone()));
                        write!(f, "{v}")
                    },
                    Ok(v) => {
                        let class: String = self
                            .t
                            .get_metatable()
                            .and_then(|mt| mt.raw_get("className_").ok())
                            .unwrap_or(String::from("unknown"));
                        write!(f, "{{ class: {}, id: {:?} }}", class, v)
                    }
                    Err(_) => write!(f, "{:?}", self.t),
                }
            }
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
                    t: as_tbl(stringify!($name), $class, value).map_err(crate::lua_err)?,
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        #[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
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
        #[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize)]
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

#[macro_export]
macro_rules! wrapped_prim {
    ($name:ident, $type:ty) => {
        wrapped_prim!($name, $type, );
    };
    ($name:ident, $type:ty, $($extra_derives:ident),*) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, $($extra_derives),*)]
        pub struct $name($type);

        impl $name {
            pub fn inner(self) -> $type {
                self.0
            }
        }

        impl From<$type> for $name {
            fn from(t: $type) -> Self {
                Self(t)
            }
        }

        impl<'lua> FromLua<'lua> for $name {
            fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
                Ok(Self(FromLua::from_lua(value, lua)?))
            }
        }

        impl<'lua> IntoLua<'lua> for $name {
            fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
                Ok(self.0.into_lua(lua)?)
            }
        }
    };
}

pub fn cvt_err(to: &'static str) -> LuaError {
    LuaError::FromLuaConversionError {
        from: "value",
        to,
        message: None,
    }
}

pub fn err(msg: &str) -> LuaError {
    LuaError::runtime(msg)
}

pub fn as_tbl_ref<'a: 'lua, 'lua>(
    to: &'static str,
    value: &'a Value<'lua>,
) -> Result<&'a mlua::Table<'lua>> {
    value
        .as_table()
        .ok_or_else(|| anyhow!("can't convert {:?} to {}", value, to))
}

fn check_implements(tbl: &mlua::Table, class: &str) -> bool {
    let mut parent = None;
    loop {
        let tbl = match parent.as_ref() {
            None => tbl,
            Some(tbl) => tbl,
        };
        match tbl.raw_get::<_, String>("className_") {
            Err(_) => break false,
            Ok(s) if s.as_str() == class => break true,
            Ok(_) => match tbl.raw_get::<_, mlua::Table>("parentClass_") {
                Err(_) => break false,
                Ok(t) => {
                    parent = Some(t);
                }
            },
        }
    }
}

pub fn as_tbl<'lua>(
    to: &'static str,
    objtyp: Option<&'static str>,
    value: Value<'lua>,
) -> Result<mlua::Table<'lua>> {
    match value {
        Value::Table(tbl) => match objtyp {
            None => Ok(tbl),
            Some(typ) => match tbl.get_metatable() {
                None => bail!(
                    "to: {to}. not an object, expected object of type {} got {:?}",
                    typ,
                    tbl
                ),
                Some(meta) => {
                    if check_implements(&meta, typ) {
                        Ok(tbl)
                    } else {
                        bail!("to: {to}. expected object of type {}, got {:?}", typ, tbl)
                    }
                }
            },
        },
        _ => bail!("expected a table, got {:?}", value),
    }
}

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

pub fn is_hooks_env(lua: &Lua) -> bool {
    lua.globals().contains_key("DCS").unwrap_or(false)
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub enum PathElt {
    Integer(i64),
    String(String),
}

impl From<&str> for PathElt {
    fn from(value: &str) -> Self {
        PathElt::String(String::from(value))
    }
}

impl From<String> for PathElt {
    fn from(value: String) -> Self {
        PathElt::String(value)
    }
}

impl From<std::string::String> for PathElt {
    fn from(value: std::string::String) -> Self {
        PathElt::String(String::from(value))
    }
}

impl From<usize> for PathElt {
    fn from(value: usize) -> Self {
        PathElt::Integer(value as i64)
    }
}

impl From<u64> for PathElt {
    fn from(value: u64) -> Self {
        PathElt::Integer(value as i64)
    }
}

impl From<u32> for PathElt {
    fn from(value: u32) -> Self {
        PathElt::Integer(value as i64)
    }
}

impl From<i64> for PathElt {
    fn from(value: i64) -> Self {
        PathElt::Integer(value)
    }
}

impl From<i32> for PathElt {
    fn from(value: i32) -> Self {
        PathElt::Integer(value as i64)
    }
}

impl From<u8> for PathElt {
    fn from(value: u8) -> Self {
        PathElt::Integer(value as i64)
    }
}

impl<'lua> IntoLua<'lua> for &PathElt {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(match self {
            PathElt::Integer(i) => Value::Integer(*i),
            PathElt::String(s) => Value::String(lua.create_string(s.as_bytes())?),
        })
    }
}

impl<'lua> FromLua<'lua> for PathElt {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(match value {
            Value::Integer(n) => PathElt::Integer(n),
            Value::String(_) => PathElt::String(String::from_lua(value, lua)?),
            _ => return Err(cvt_err("String")),
        })
    }
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct Path(Vec<PathElt>);

impl Deref for Path {
    type Target = [PathElt];

    fn deref(&self) -> &Self::Target {
        &self.0[..]
    }
}

impl<'a> IntoIterator for &'a Path {
    type IntoIter = std::slice::Iter<'a, PathElt>;
    type Item = &'a PathElt;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl Path {
    pub fn push<T: Into<PathElt>>(&mut self, t: T) {
        self.0.push(t.into())
    }

    pub fn pop(&mut self) -> Option<PathElt> {
        self.0.pop()
    }

    pub fn append<T: Into<PathElt>, I: IntoIterator<Item = T>>(&self, elts: I) -> Self {
        let mut new_t = self.clone();
        for elt in elts {
            new_t.push(elt)
        }
        new_t
    }

    pub fn get(&self, i: usize) -> Option<&PathElt> {
        self.0.get(i)
    }
}

pub trait DcsTableExt<'lua> {
    fn raw_get_path<T>(&self, path: &Path) -> Result<T>
    where
        T: FromLua<'lua>;

    fn get_path<T>(&self, path: &Path) -> Result<T>
    where
        T: FromLua<'lua>;
}

fn table_raw_get_path<'lua, T>(tbl: &mlua::Table<'lua>, path: &[PathElt]) -> Result<T>
where
    T: FromLua<'lua>,
{
    match path {
        [] => bail!("path not found"),
        [elt] => Ok(tbl.raw_get(elt)?),
        [elt, path @ ..] => {
            let tbl: mlua::Table = tbl.raw_get(elt)?;
            table_raw_get_path(&tbl, path)
        }
    }
}

fn table_get_path<'lua, T>(tbl: &mlua::Table<'lua>, path: &[PathElt]) -> Result<T>
where
    T: FromLua<'lua>,
{
    match path {
        [] => bail!("path not found"),
        [elt] => Ok(tbl.get(elt)?),
        [elt, path @ ..] => {
            let tbl: mlua::Table = tbl.get(elt)?;
            table_get_path(&tbl, path)
        }
    }
}

impl<'lua> DcsTableExt<'lua> for mlua::Table<'lua> {
    fn raw_get_path<T>(&self, path: &Path) -> Result<T>
    where
        T: FromLua<'lua>,
    {
        table_raw_get_path(self, &**path)
    }

    fn get_path<T>(&self, path: &Path) -> Result<T>
    where
        T: FromLua<'lua>,
    {
        table_get_path(self, &**path)
    }
}

pub type Vector2 = na::base::Vector2<f64>;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct LuaVec2(pub na::base::Vector2<f64>);

impl Deref for LuaVec2 {
    type Target = na::base::Vector2<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LuaVec2 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'lua> IntoLua<'lua> for LuaVec2 {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let tbl = lua.create_table()?;
        tbl.set("x", self.0.x)?;
        tbl.set("y", self.0.y)?;
        Ok(Value::Table(tbl))
    }
}

impl<'lua> FromLua<'lua> for LuaVec2 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Vec2", None, value).map_err(lua_err)?;
        Ok(Self(na::base::Vector2::new(
            tbl.raw_get("x")?,
            tbl.raw_get("y")?,
        )))
    }
}

impl LuaVec2 {
    pub fn new(x: f64, y: f64) -> Self {
        LuaVec2(na::base::Vector2::new(x, y))
    }
}

pub type Vector3 = na::base::Vector3<f64>;

#[derive(Debug, Clone, Copy, PartialEq, PartialOrd, Default, Serialize, Deserialize)]
pub struct LuaVec3(pub na::base::Vector3<f64>);

impl Deref for LuaVec3 {
    type Target = na::base::Vector3<f64>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl DerefMut for LuaVec3 {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<'lua> FromLua<'lua> for LuaVec3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Vec3", None, value).map_err(lua_err)?;
        Ok(Self(na::base::Vector3::new(
            tbl.raw_get("x")?,
            tbl.raw_get("y")?,
            tbl.raw_get("z")?,
        )))
    }
}

impl<'lua> IntoLua<'lua> for LuaVec3 {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let tbl = lua.create_table()?;
        tbl.raw_set("x", self.0.x)?;
        tbl.raw_set("y", self.0.y)?;
        tbl.raw_set("z", self.0.z)?;
        Ok(Value::Table(tbl))
    }
}

impl LuaVec3 {
    pub fn new(x: f64, y: f64, z: f64) -> Self {
        Self(na::base::Vector3::new(x, y, z))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Default, Serialize, Deserialize)]
pub struct Position3 {
    pub p: LuaVec3,
    pub x: LuaVec3,
    pub y: LuaVec3,
    pub z: LuaVec3,
}

impl<'lua> FromLua<'lua> for Position3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Position3", None, value).map_err(lua_err)?;
        Ok(Self {
            p: tbl.raw_get("p")?,
            x: tbl.raw_get("x")?,
            y: tbl.raw_get("y")?,
            z: tbl.raw_get("z")?,
        })
    }
}

impl<'lua> IntoLua<'lua> for Position3 {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let tbl = lua.create_table()?;
        tbl.raw_set("p", self.p)?;
        tbl.raw_set("x", self.x)?;
        tbl.raw_set("y", self.y)?;
        tbl.raw_set("z", self.z)?;
        Ok(Value::Table(tbl))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct Box3 {
    pub min: LuaVec3,
    pub max: LuaVec3,
}

impl<'lua> FromLua<'lua> for Box3 {
    fn from_lua(value: Value<'lua>, _: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Box3", None, value).map_err(lua_err)?;
        Ok(Self {
            min: tbl.raw_get("min")?,
            max: tbl.raw_get("max")?,
        })
    }
}

#[derive(Debug, Clone, Default, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
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

impl AsRef<str> for String {
    fn as_ref(&self) -> &str {
        self.0.as_ref()
    }
}

impl Borrow<str> for String {
    fn borrow(&self) -> &str {
        self.0.borrow()
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

#[derive(Debug, Clone, Copy, Default, PartialEq, PartialOrd, Serialize, Deserialize)]
pub struct Time(pub f32);

impl<'lua> IntoLua<'lua> for Time {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        self.0.into_lua(lua)
    }
}

impl<'lua> FromLua<'lua> for Time {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self(f32::from_lua(value, lua)?))
    }
}

impl Add<f32> for Time {
    type Output = Self;

    fn add(self, rhs: f32) -> Self::Output {
        Time(self.0 + rhs)
    }
}

impl AddAssign<f32> for Time {
    fn add_assign(&mut self, rhs: f32) {
        self.0 += rhs
    }
}

impl Sub for Time {
    type Output = f32;

    fn sub(self, rhs: Self) -> f32 {
        self.0 - rhs.0
    }
}

#[derive(Debug, Clone, Serialize)]
pub enum VolumeType {
    Segment,
    Box,
    Sphere,
    Pyramid,
}

#[derive(Debug, Clone, Serialize)]
pub struct Sequence<'lua, T> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    _lua: &'lua Lua,
    ph: PhantomData<T>,
}

impl<'lua, T: FromLua<'lua> + 'lua> FromLua<'lua> for Sequence<'lua, T> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::Table(t) => Ok(Self {
                t,
                _lua: lua,
                ph: PhantomData,
            }),
            Value::Nil => Ok(Self {
                t: lua.create_table()?,
                _lua: lua,
                ph: PhantomData,
            }),
            _ => Err(cvt_err("Sequence")),
        }
    }
}

impl<'lua, T: IntoLua<'lua> + 'lua> IntoLua<'lua> for Sequence<'lua, T> {
    fn into_lua(self, _lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Table(self.t))
    }
}

impl<'lua, T: FromLua<'lua> + 'lua> IntoIterator for Sequence<'lua, T> {
    type IntoIter = mlua::TableSequence<'lua, T>;
    type Item = LuaResult<T>;

    fn into_iter(self) -> Self::IntoIter {
        self.t.sequence_values()
    }
}

impl<'lua, T: FromLua<'lua> + 'lua> Sequence<'lua, T> {
    pub fn get(&self, i: i64) -> Result<T> {
        Ok(self.t.raw_get(i)?)
    }
}

impl<'lua, T: IntoLua<'lua> + 'lua> Sequence<'lua, T> {
    pub fn set(&self, i: i64, t: T) -> Result<()> {
        Ok(self.t.raw_set(i, t)?)
    }
}

impl<'lua, T: FromLua<'lua> + 'lua> Sequence<'lua, T> {
    pub fn empty(lua: &'lua Lua) -> Result<Self> {
        Ok(Self {
            t: lua.create_table()?,
            _lua: lua,
            ph: PhantomData,
        })
    }

    pub fn len(&self) -> usize {
        self.t.raw_len()
    }

    pub fn into_inner(self) -> mlua::Table<'lua> {
        self.t
    }

    pub fn remove(&self, i: i64) -> Result<()> {
        Ok(self.t.raw_remove(i)?)
    }

    pub fn first(&self) -> Result<T> {
        Ok(self.t.raw_get(1)?)
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

pub fn centroid2d(points: impl IntoIterator<Item = Vector2>) -> Vector2 {
    let (n, sum) = points
        .into_iter()
        .fold((0, Vector2::new(0., 0.)), |(n, c), p| (n + 1, c + p));
    sum / (n as f64)
}

pub fn centroid3d(points: impl IntoIterator<Item = Vector3>) -> Vector3 {
    let (n, sum) = points
        .into_iter()
        .fold((0, Vector3::new(0., 0., 0.)), |(n, c), p| (n + 1, c + p));
    sum / (n as f64)
}

/// Rotate a collection of points in 2d space around their center point
/// keeping the relative orientations of the points constant. The angle
/// is in radians.
pub fn rotate2d(angle: f64, points: &mut [Vector2]) {
    let centroid = centroid2d(points.into_iter().map(|p| *p));
    let sin = angle.sin();
    let cos = angle.cos();
    for p in points {
        *p -= centroid;
        let x = p.x;
        let y = p.y;
        p.x = x * cos - y * sin;
        p.y = x * sin + y * cos;
        *p += centroid
    }
}

/// Same as rotate2d, but construct and return a vec containing the rotated points
/// in the same order as the they appear in the input slice.
pub fn rotate2d_vec(angle: f64, points: &[Vector2]) -> Vec<Vector2> {
    let mut points = Vec::from_iter(points.into_iter().map(|p| *p));
    rotate2d(angle, &mut points);
    points
}

pub fn radians_to_degrees(radians: f64) -> f64 {
    radians * (180. / std::f64::consts::PI)
}

pub fn degrees_to_radians(degrees: f64) -> f64 {
    degrees * (std::f64::consts::PI / 180.)
}

pub fn azumith2d(v: Vector2) -> f64 {
    let az = v.y.atan2(v.x);
    if az < 0. {
        az + 2. * std::f64::consts::PI
    } else {
        az
    }
}

pub fn azumith2d_to(from: Vector2, to: Vector2) -> f64 {
    azumith2d(to - from)
}

pub fn azumith3d(v: Vector3) -> f64 {
    let az = v.z.atan2(v.x);
    if az < 0. {
        az + 2. * std::f64::consts::PI
    } else {
        az
    }
}

pub fn azumith3d_to(from: Vector3, to: Vector3) -> f64 {
    azumith3d(to - from)
}
