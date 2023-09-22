use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use super::{object::Object, as_tbl};

#[derive(Debug, Clone, Serialize)]
pub enum AltitudeKind {
    Radio,
    Baro,
}

impl<'lua> IntoLua<'lua> for AltitudeKind {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::String(lua.create_string(match self {
            Self::Radio => "RADIO",
            Self::Baro => "BARO",
        })?))
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct Controller<'lua> {
    t: mlua::Table<'lua>,
    #[serde(skip)]
    _lua: &'lua Lua,
}

impl<'lua> FromLua<'lua> for Controller<'lua> {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        Ok(Self {
            t: as_tbl("Controller", Some("Controller"), value)?,
            _lua: lua,
        })
    }
}

impl<'lua> Controller<'lua> {
    pub fn has_task(&self) -> LuaResult<bool> {
        self.t.call_method("hasTask", ())
    }

    pub fn set_on_off(&self, on: bool) -> LuaResult<()> {
        self.t.call_method("setOnOff", on)
    }

    pub fn set_altitude(
        &self,
        altitude: f32,
        keep: bool,
        kind: Option<AltitudeKind>,
    ) -> LuaResult<()> {
        match kind {
            None => self.t.call_method("setAltitude", (altitude, keep)),
            Some(kind) => self.t.call_method("setAltitude", (altitude, keep, kind)),
        }
    }

    pub fn set_speed(&self, speed: f32, keep: bool) -> LuaResult<()> {
        self.t.call_method("setSpeed", (speed, keep))
    }

    pub fn know_target(&self, object: Object, typ: bool, distance: bool) -> LuaResult<()> {
        self.t.call_function("knowTarget", (object, typ, distance))
    }
}