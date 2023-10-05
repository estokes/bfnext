use crate::{as_tbl, coalition::Side, cvt_err, wrapped_table, Sequence, String, Vec2};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(Weather, None);

pub struct Quad {
    pub p0: Vec2,
    pub p1: Vec2,
    pub p2: Vec2,
    pub p3: Vec2
}

impl<'lua> FromLua<'lua> for Quad {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let verts = as_tbl("Quad", None, value)?;
        Ok(Self {
            p0: verts.raw_get(1)?,
            p1: verts.raw_get(2)?,
            p2: verts.raw_get(3)?,
            p3: verts.raw_get(4)?
        })
    }
}

pub enum TriggerZoneTyp {
    Circle { radius: f64 },
    Quad(Quad)
}

#[derive(Clone, Copy, PartialEq, Serialize)]
pub struct Color {
    r: f32,
    g: f32,
    b: f32,
    a: f32
}

impl<'lua> FromLua<'lua> for Color {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("Color", None, value)?;
        Ok(Self {
            r: tbl.raw_get(1)?,
            g: tbl.raw_get(2)?,
            b: tbl.raw_get(3)?,
            a: tbl.raw_get(4)?
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

wrapped_table!(TriggerZone, None);

impl<'lua> TriggerZone<'lua> {
    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn pos(&self) -> LuaResult<Vec2> {
        Ok(Vec2 {
            x: self.raw_get("x")?,
            y: self.raw_get("y")?,
        })
    }

    pub fn typ(&self) -> LuaResult<TriggerZoneTyp> {
        Ok(match self.raw_get("type")? {
            0 => TriggerZoneTyp::Circle { radius: self.raw_get("radius")? },
            2 => TriggerZoneTyp::Quad(self.raw_get("vertices")?),
            _ => return Err(cvt_err("TriggerZoneTyp"))
        })
    }

    pub fn color(&self) -> LuaResult<Color> {
        self.raw_get("color")
    }
}

wrapped_table!(NavPoint, None);

wrapped_table!(Task, None);

wrapped_table!(Point, None);

wrapped_table!(Route, None);

impl<'lua> Route<'lua> {
    pub fn points(&self) -> LuaResult<Sequence<Point>> {
        self.raw_get("points")
    }
}

wrapped_table!(Unit, None);

wrapped_table!(Group, None);

impl<'lua> Group<'lua> {
    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn frequency(&self) -> LuaResult<f64> {
        self.raw_get("frequency")
    }

    pub fn modulation(&self) -> LuaResult<i64> {
        self.raw_get("modulation")
    }

    pub fn late_activation(&self) -> bool {
        self.raw_get("lateActivation").unwrap_or(false)
    }

    pub fn group_id(&self) -> LuaResult<i64> {
        self.raw_get("groupId")
    }

    pub fn tasks(&self) -> LuaResult<Sequence<Task>> {
        self.raw_get("tasks")
    }

    pub fn route(&self) -> LuaResult<Route> {
        self.raw_get("route")
    }

    pub fn hidden(&self) -> bool {
        self.raw_get("hidden").unwrap_or(false)
    }

    pub fn units(&self) -> LuaResult<Sequence<Unit>> {
        self.raw_get("units")
    }

    pub fn uncontrollable(&self) -> bool {
        self.raw_get("uncontrollable").unwrap_or(true)
    }
}

wrapped_table!(Country, None);

impl<'lua> Country<'lua> {
    pub fn id(&self) -> LuaResult<Country> {
        self.raw_get("id")
    }

    pub fn name(&self) -> LuaResult<String> {
        self.raw_get("name")
    }

    pub fn planes(&self) -> LuaResult<Sequence<Group>> {
        let g: mlua::Table = self.raw_get("plane")?;
        g.raw_get("group")
    }

    pub fn helicopters(&self) -> LuaResult<Sequence<Group>> {
        let g: mlua::Table = self.raw_get("helicopter")?;
        g.raw_get("group")
    }

    pub fn ships(&self) -> LuaResult<Sequence<Group>> {
        let g: mlua::Table = self.raw_get("ship")?;
        g.raw_get("group")
    }

    pub fn vehicles(&self) -> LuaResult<Sequence<Group>> {
        let g: mlua::Table = self.raw_get("vehicle")?;
        g.raw_get("group")
    }

    pub fn statics(&self) -> LuaResult<Sequence<Group>> {
        let g: mlua::Table = self.raw_get("static")?;
        g.raw_get("group")
    }
}

wrapped_table!(Coalition, None);

impl<'lua> Coalition<'lua> {
    pub fn bullseye(&self) -> LuaResult<Vec2> {
        self.t.raw_get("bullseye")
    }

    pub fn nav_points(&self) -> LuaResult<Sequence<NavPoint>> {
        self.t.raw_get("nav_points")
    }

    pub fn name(&self) -> LuaResult<String> {
        self.t.raw_get("name")
    }

    pub fn countries(&self) -> LuaResult<Sequence<Country>> {
        self.t.raw_get("country")
    }
}

wrapped_table!(Miz, None);

impl<'lua> Miz<'lua> {
    pub fn coalition(&self, side: Side) -> LuaResult<Coalition<'lua>> {
        match side {
            Side::Blue => self.t.raw_get("blue"),
            Side::Red => self.t.raw_get("red"),
            Side::Neutral => self.t.raw_get("neutral"),
        }
    }

    pub fn triggers(&self) -> LuaResult<Sequence<TriggerZone>> {
        let triggers: mlua::Table = self.t.raw_get("triggers")?;
        triggers.raw_get("zones")
    }

    pub fn weather(&self) -> LuaResult<Weather<'lua>> {
        self.t.raw_get("weather")
    }
}
