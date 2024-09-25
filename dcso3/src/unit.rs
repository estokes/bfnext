/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::{as_tbl, controller::Controller, cvt_err, group::Group, object::Object, String};
use crate::{
    env::miz::UnitId,
    net::SlotId,
    object::{DcsObject, DcsOid},
    record_perf, simple_enum, wrapped_table, LuaEnv, LuaVec2, LuaVec3, MizLua, Position3, Sequence,
};
use anyhow::{bail, Result};
use mlua::{prelude::*, Value};
use na::Vector2;
use serde_derive::{Deserialize, Serialize};
use std::{marker::PhantomData, ops::Deref};

simple_enum!(UnitCategory, u8, [
    Airplane => 0,
    GroundUnit => 2,
    Helicopter => 1,
    Ship => 3,
    Structure => 4
]);

wrapped_table!(Ammo, None);

impl<'lua> Ammo<'lua> {
    pub fn count(&self) -> Result<u32> {
        Ok(self.t.raw_get("count")?)
    }

    pub fn type_name(&self) -> Result<String> {
        Ok(self.t.raw_get::<_, LuaTable>("desc")?.raw_get("typeName")?)
    }

    pub fn display_name(&self) -> Result<String> {
        Ok(self
            .t
            .raw_get::<_, LuaTable>("desc")?
            .raw_get("displayName")?)
    }
}

wrapped_table!(Unit, Some("Unit"));

impl<'lua> Unit<'lua> {
    pub fn get_by_name(lua: MizLua<'lua>, name: &str) -> Result<Unit<'lua>> {
        let globals = lua.inner().globals();
        let unit = as_tbl("Unit", None, globals.raw_get("Unit")?)?;
        Ok(record_perf!(
            unit_get_by_name,
            unit.call_function("getByName", name)?
        ))
    }

    pub fn is_exist(&self) -> Result<bool> {
        Ok(record_perf!(
            unit_is_exist,
            self.t.call_method("isExist", ())?
        ))
    }

    pub fn destroy(self) -> Result<()> {
        Ok(self.t.call_method("destroy", ())?)
    }

    pub fn get_desc(&self) -> Result<mlua::Table<'lua>> {
        Ok(self.t.call_method("getDesc", ())?)
    }

    pub fn as_object(&self) -> Result<Object<'lua>> {
        Ok(Object::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }

    pub fn get_type_name(&self) -> Result<String> {
        Ok(self.t.call_method("getTypeName", ())?)
    }

    pub fn get_point(&self) -> Result<LuaVec3> {
        Ok(record_perf!(get_point, self.t.call_method("getPoint", ())?))
    }

    pub fn get_position(&self) -> Result<Position3> {
        Ok(record_perf!(
            get_position,
            self.t.call_method("getPosition", ())?
        ))
    }

    pub fn get_ground_position(&self) -> Result<LuaVec2> {
        let pos = self.get_point()?;
        Ok(LuaVec2(Vector2::from(na::Vector2::new(pos.0.x, pos.0.z))))
    }

    pub fn get_velocity(&self) -> Result<LuaVec3> {
        Ok(record_perf!(
            get_velocity,
            self.t.call_method("getVelocity", ())?
        ))
    }

    pub fn in_air(&self) -> Result<bool> {
        Ok(record_perf!(in_air, self.t.call_method("inAir", ())?))
    }

    pub fn is_active(&self) -> Result<bool> {
        Ok(self.t.call_method("isActive", ())?)
    }

    pub fn get_name(&self) -> Result<String> {
        Ok(self.t.call_method("getName", ())?)
    }

    pub fn get_player_name(&self) -> Result<Option<String>> {
        Ok(self.t.call_method("getPlayerName", ())?)
    }

    pub fn id(&self) -> Result<UnitId> {
        Ok(self.t.call_method("getID", ())?)
    }

    pub fn slot(&self) -> Result<SlotId> {
        Ok(SlotId::from(self.id()?))
    }

    pub fn get_number(&self) -> Result<i64> {
        Ok(self.t.call_method("getNumber", ())?)
    }

    pub fn get_controller(&self) -> Result<Controller<'lua>> {
        Ok(self.t.call_method("getController", ())?)
    }

    pub fn get_group(&self) -> Result<Group<'lua>> {
        Ok(self.t.call_method("getGroup", ())?)
    }

    pub fn get_callsign(&self) -> Result<String> {
        Ok(self.t.call_method("getCallsign", ())?)
    }

    pub fn get_life(&self) -> Result<i32> {
        Ok(self.t.call_method("getLife", ())?)
    }

    pub fn get_life0(&self) -> Result<i32> {
        Ok(self.t.call_method("getLife0", ())?)
    }

    pub fn get_fuel(&self) -> Result<f32> {
        Ok(self.t.call_method("getFuel", ())?)
    }

    pub fn enable_emission(&self, on: bool) -> Result<()> {
        Ok(self.t.call_method("enableEmission", on)?)
    }

    pub fn get_category(&self) -> Result<UnitCategory> {
        Ok(self.t.call_method("getCategory", ())?)
    }

    pub fn get_ammo(&self) -> Result<Sequence<'lua, Ammo<'lua>>> {
        Ok(record_perf!(get_ammo, self.t.call_method("getAmmo", ())?))
    }
}

#[derive(Debug, Clone)]
pub struct ClassUnit;

impl<'lua> DcsObject<'lua> for Unit<'lua> {
    type Class = ClassUnit;

    fn object_id(&self) -> Result<DcsOid<Self::Class>> {
        Ok(DcsOid {
            id: self.raw_get("id_")?,
            class: "Unit".into(),
            t: PhantomData,
        })
    }

    fn get_instance(lua: MizLua<'lua>, id: &DcsOid<Self::Class>) -> Result<Self> {
        let t = lua.inner().create_table()?;
        t.set_metatable(Some(lua.inner().globals().raw_get(&**id.class)?));
        t.raw_set("id_", id.id)?;
        let t = Unit {
            t,
            lua: lua.inner(),
        };
        if !t.is_exist()? {
            bail!("{} is an invalid unit", id.id)
        }
        // work around DCS bug that results in isExist => true for dead units
        if t.get_life()? <= 0 {
            bail!("{} is dead", id.id)
        }
        Ok(t)
    }

    fn get_instance_dyn<T>(lua: MizLua<'lua>, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(lua, "Unit")?;
        let id = DcsOid {
            id: id.id,
            class: id.class.clone(),
            t: PhantomData,
        };
        Self::get_instance(lua, &id)
    }

    fn change_instance(self, id: &DcsOid<Self::Class>) -> Result<Self> {
        self.raw_set("id_", id.id)?;
        if !self.is_exist()? {
            bail!("{} is an invalid unit", id.id)
        }
        // work around DCS bug that results in isExist => true for dead units
        if self.get_life()? <= 0 {
            bail!("{} is dead", id.id)
        }
        Ok(self)
    }

    fn change_instance_dyn<T>(self, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(MizLua(self.lua), "Unit")?;
        self.t.raw_set("id_", id.id)?;
        if !self.is_exist()? {
            bail!("{} is an invalid unit", id.id)
        }
        // work around DCS bug that results in isExist => true for dead units
        if self.get_life()? <= 0 {
            bail!("{} is dead", id.id)
        }
        Ok(self)
    }
}
