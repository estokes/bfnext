use super::{as_tbl, cvt_err, unit::Unit, weapon::Weapon, Position3, String, Vec3};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;

#[derive(Debug, Clone, Serialize)]
pub enum ObjectCategory {
    Void,
    Unit,
    Weapon,
    Static,
    Base,
    Scenery,
    Cargo,
}

#[derive(Debug, Clone, Serialize)]
pub struct Object<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Object<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Object", Some("Object"), value)?,
            lua,
        })
    }
}

impl<'lua> IntoLua<'lua> for Object<'lua> {
    fn into_lua(self, _: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Table(self.t))
    }
}

impl<'lua> Object<'lua> {
    pub fn destroy(&self) -> LuaResult<()> {
        self.t.call_method("destroy", ())
    }

    pub fn get_category(&self) -> LuaResult<ObjectCategory> {
        Ok(match self.t.call_method("getCategory", ())? {
            0 => ObjectCategory::Void,
            1 => ObjectCategory::Unit,
            2 => ObjectCategory::Weapon,
            3 => ObjectCategory::Static,
            4 => ObjectCategory::Base,
            5 => ObjectCategory::Scenery,
            6 => ObjectCategory::Cargo,
            _ => return Err(cvt_err("ObjectCategory")),
        })
    }

    pub fn get_desc(&self) -> LuaResult<mlua::Table<'lua>> {
        self.t.call_method("getDesc", ())
    }

    pub fn get_name(&self) -> LuaResult<String> {
        self.t.call_method("getName", ())
    }

    pub fn get_point(&self) -> LuaResult<Vec3> {
        self.t.call_method("getPoint", ())
    }

    pub fn get_position(&self) -> LuaResult<Position3> {
        self.t.call_method("getPosition", ())
    }

    pub fn get_velocity(&self) -> LuaResult<Vec3> {
        self.t.call_method("getPosition", ())
    }

    pub fn in_air(&self) -> LuaResult<bool> {
        self.t.call_method("inAir", ())
    }

    pub fn as_unit(&self) -> LuaResult<Unit> {
        Unit::from_lua(Value::Table(self.t.clone()), self.lua)
    }

    pub fn as_weapon(&self) -> LuaResult<Weapon> {
        Weapon::from_lua(Value::Table(self.t.clone()), self.lua)
    }
}
