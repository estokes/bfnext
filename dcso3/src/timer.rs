use crate::{wrapped_table, cvt_err, as_tbl};
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
    pub fn singleton(lua: &'lua Lua) -> LuaResult<Self> {
        lua.globals().raw_get("timer")
    }

    pub fn get_time(&self) -> LuaResult<f64> {
        self.t.call_method("getTime", ())
    }

    pub fn get_abs_time(&self) -> LuaResult<u32> {
        self.t.call_method("getAbsTime", ())
    }

    pub fn get_time0(&self) -> LuaResult<u32> {
        self.t.call_method("getTime0", ())
    }

    pub fn schedule_function<T, F>(&self, when: f64, arg: T, f: F) -> LuaResult<FunId>
    where
        F: Fn(&'lua Lua, T, f64) -> LuaResult<Option<f64>> + 'static,
        T: IntoLua<'lua> + FromLua<'lua>,
    {
        let f = self
            .lua
            .create_function(move |lua, (arg, time)| match f(lua, arg, time) {
                Ok(r) => Ok(r),
                Err(e) => {
                    println!("error in scheduled function: {:?}", e);
                    Ok(None)
                }
            })?;
        self.t.call_method("scheduleFunction", (f, arg, when))
    }

    pub fn remove_function(&self, id: FunId) -> LuaResult<()> {
        self.t.call_method("removeFunction", id)
    }

    pub fn set_function_fime(&self, id: FunId, when: f64) -> LuaResult<()> {
        self.t.call_method("removeFunction", (id, when))
    }
}
