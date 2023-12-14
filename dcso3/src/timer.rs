use crate::{as_tbl, cvt_err, wrapped_table, LuaEnv, MizLua, Time};
use anyhow::Result;
use log::error;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct FunId(i64);

impl<'lua> FromLua<'lua> for FunId {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        match value {
            Value::Integer(i) => Ok(FunId(i)),
            _ => Err(cvt_err("FunId")),
        }
    }
}

impl<'lua> IntoLua<'lua> for FunId {
    fn into_lua(self, _lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        Ok(Value::Integer(self.0))
    }
}

wrapped_table!(Timer, None);

impl<'lua> Timer<'lua> {
    pub fn singleton(lua: MizLua<'lua>) -> Result<Self> {
        Ok(lua.inner().globals().raw_get("timer")?)
    }

    pub fn get_time(&self) -> Result<Time> {
        Ok(self.t.call_function("getTime", ())?)
    }

    pub fn get_abs_time(&self) -> Result<Time> {
        Ok(self.t.call_function("getAbsTime", ())?)
    }

    pub fn get_time0(&self) -> Result<Time> {
        Ok(self.t.call_function("getTime0", ())?)
    }

    pub fn schedule_function<T, F>(&self, when: Time, arg: T, f: F) -> Result<FunId>
    where
        F: Fn(MizLua, T, Time) -> Result<Option<Time>> + 'static,
        T: IntoLua<'lua> + FromLua<'lua>,
    {
        let f =
            self.lua
                .create_function(move |lua, (arg, time)| match f(MizLua(lua), arg, time) {
                    Ok(r) => Ok(r),
                    Err(e) => {
                        error!("error in scheduled function: {:?}", e);
                        Ok(None)
                    }
                })?;
        Ok(self.t.call_function("scheduleFunction", (f, arg, when))?)
    }

    pub fn remove_function(&self, id: FunId) -> Result<()> {
        Ok(self.t.call_function("removeFunction", id)?)
    }

    pub fn set_function_fime(&self, id: FunId, when: f64) -> Result<()> {
        Ok(self.t.call_function("removeFunction", (id, when))?)
    }
}
