use super::{as_tbl, cvt_err, unit::Unit, weapon::Weapon, LuaVec3, Position3, String};
use crate::{check_implements, simple_enum, wrapped_table, LuaEnv, MizLua};
use anyhow::{anyhow, bail, Result};
use core::fmt;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::{hash::Hash, marker::PhantomData, ops::Deref};

#[derive(Clone, Serialize)]
pub struct DcsOid<T> {
    pub(crate) id: u64,
    pub(crate) class: String,
    pub(crate) t: PhantomData<T>,
}

impl<T> DcsOid<T> {
    pub fn erased(&self) -> DcsOid<ClassObject> {
        DcsOid {
            id: self.id,
            class: self.class.clone(),
            t: PhantomData,
        }
    }

    pub fn check_implements(&self, lua: MizLua, class: &str) -> Result<()> {
        let m = lua.inner().globals().raw_get(&**self.class)?;
        if !check_implements(&m, class) {
            bail!("{:?} is does not implement {class}", self)
        }
        Ok(())
    }
}

impl<T> fmt::Debug for DcsOid<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{{ id: {}, class: {} }}", self.id, self.class)
    }
}

impl<T> Hash for DcsOid<T> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.id.hash(state)
    }
}

impl<T> PartialEq for DcsOid<T> {
    fn eq(&self, other: &Self) -> bool {
        self.id.eq(&other.id)
    }
}

impl<T> Eq for DcsOid<T> {}

impl<T> PartialOrd for DcsOid<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        self.id.partial_cmp(&other.id)
    }
}

impl<T> Ord for DcsOid<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.id.cmp(&other.id)
    }
}

#[derive(Debug, Clone)]
pub struct ClassObject;

pub trait DcsObject<'lua>: Sized + Deref<Target = mlua::Table<'lua>> {
    type Class: fmt::Debug + Clone;

    fn object_id(&self) -> Result<DcsOid<Self::Class>> {
        let id = self.raw_get("id_")?;
        let m = self
            .get_metatable()
            .ok_or_else(|| anyhow!("object with no metatable"))?;
        let class = m.raw_get("className_")?;
        Ok(DcsOid {
            id,
            class,
            t: PhantomData,
        })
    }

    fn change_instance(self, id: &DcsOid<Self::Class>) -> Result<Self> {
        self.raw_set("id_", id.id)?;
        Ok(self)
    }

    fn change_instance_dyn<T>(self, id: &DcsOid<T>) -> Result<Self>;
    fn get_instance(lua: MizLua<'lua>, id: &DcsOid<Self::Class>) -> Result<Self>;
    fn get_instance_dyn<T>(lua: MizLua<'lua>, id: &DcsOid<T>) -> Result<Self>;
}

simple_enum!(ObjectCategory, u8, [
    Void => 0,
    Unit => 1,
    Weapon => 2,
    Static => 3,
    Base => 4,
    Scenery => 5,
    Cargo => 6
]);

wrapped_table!(Object, Some("Object"));

impl<'lua> Object<'lua> {
    pub fn destroy(self) -> Result<()> {
        Ok(self.t.call_method("destroy", ())?)
    }

    pub fn get_category(&self) -> Result<ObjectCategory> {
        Ok(self.t.call_method("getCategory", ())?)
    }

    pub fn get_desc(&self) -> Result<mlua::Table<'lua>> {
        Ok(self.t.call_method("getDesc", ())?)
    }

    pub fn has_attribute(&self, attr: String) -> Result<bool> {
        Ok(self.t.call_method("hasAttribute", attr)?)
    }

    pub fn get_name(&self) -> Result<String> {
        Ok(self.t.call_method("getName", ())?)
    }

    pub fn get_type_name(&self) -> Result<String> {
        Ok(self.t.call_method("getTypeName", ())?)
    }

    pub fn get_point(&self) -> Result<LuaVec3> {
        Ok(self.t.call_method("getPoint", ())?)
    }

    pub fn get_position(&self) -> Result<Position3> {
        Ok(self.t.call_method("getPosition", ())?)
    }

    pub fn get_velocity(&self) -> Result<LuaVec3> {
        Ok(self.t.call_method("getVelocity", ())?)
    }

    pub fn in_air(&self) -> Result<bool> {
        Ok(self.t.call_method("inAir", ())?)
    }

    pub fn is_exist(&self) -> Result<bool> {
        Ok(self.t.call_method("isExist", ())?)
    }

    pub fn as_unit(&self) -> Result<Unit<'lua>> {
        Ok(Unit::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }

    pub fn as_weapon(&self) -> Result<Weapon<'lua>> {
        Ok(Weapon::from_lua(Value::Table(self.t.clone()), self.lua)?)
    }
}

impl<'lua> DcsObject<'lua> for Object<'lua> {
    type Class = ClassObject;

    fn get_instance(lua: MizLua<'lua>, id: &DcsOid<Self::Class>) -> Result<Self> {
        let t = lua.inner().create_table()?;
        t.set_metatable(Some(lua.inner().globals().raw_get(&**id.class)?));
        t.raw_set("id_", id.id)?;
        Ok(Object {
            t,
            lua: lua.inner(),
        })
    }

    fn get_instance_dyn<T>(lua: MizLua<'lua>, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(lua, "Object")?;
        let id = DcsOid {
            id: id.id,
            class: id.class.clone(),
            t: PhantomData,
        };
        Self::get_instance(lua, &id)
    }

    fn change_instance_dyn<T>(self, id: &DcsOid<T>) -> Result<Self> {
        id.check_implements(MizLua(self.lua), "Object")?;
        self.t.raw_set("id_", id.id)?;
        Ok(self)
    }
}
