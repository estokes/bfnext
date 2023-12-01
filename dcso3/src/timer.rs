use crate::{as_tbl, cvt_err, wrapped_table, LuaEnv, MizLua, Time};
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
    pub fn singleton(lua: MizLua<'lua>) -> LuaResult<Self> {
        lua.inner().globals().raw_get("timer")
    }

    pub fn get_time(&self) -> LuaResult<Time> {
        self.t.call_function("getTime", ())
    }

    pub fn get_abs_time(&self) -> LuaResult<Time> {
        self.t.call_function("getAbsTime", ())
    }

    pub fn get_time0(&self) -> LuaResult<Time> {
        self.t.call_function("getTime0", ())
    }

    pub fn schedule_function<T, F>(&self, when: Time, arg: T, f: F) -> LuaResult<FunId>
    where
        F: Fn(MizLua, T, Time) -> LuaResult<Option<Time>> + 'static,
        T: IntoLua<'lua> + FromLua<'lua>,
    {
        let f =
            self.lua
                .create_function(move |lua, (arg, time)| match f(MizLua(lua), arg, time) {
                    Ok(r) => Ok(r),
                    Err(e) => {
                        println!("error in scheduled function: {:?}", e);
                        Ok(None)
                    }
                })?;
        self.t.call_function("scheduleFunction", (f, arg, when))
    }

    pub fn remove_function(&self, id: FunId) -> LuaResult<()> {
        self.t.call_function("removeFunction", id)
    }

    pub fn set_function_fime(&self, id: FunId, when: f64) -> LuaResult<()> {
        self.t.call_function("removeFunction", (id, when))
    }
}
