/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use crate::{as_tbl, cvt_err, record_perf, wrap_f, wrapped_table, LuaEnv, MizLua, Time};
use anyhow::Result;
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
        Ok(record_perf!(
            timer_get_time,
            self.t.call_function("getTime", ())?
        ))
    }

    pub fn get_abs_time(&self) -> Result<Time> {
        Ok(record_perf!(
            timer_get_abs_time,
            self.t.call_function("getAbsTime", ())?
        ))
    }

    pub fn get_time0(&self) -> Result<Time> {
        Ok(record_perf!(
            timer_get_time0,
            self.t.call_function("getTime0", ())?
        ))
    }

    pub fn schedule_function<T, F>(&self, when: Time, arg: T, f: F) -> Result<FunId>
    where
        F: Fn(MizLua, T, Time) -> Result<Option<Time>> + 'static,
        T: IntoLua<'lua> + FromLua<'lua>,
    {
        let f = self
            .lua
            .create_function(move |lua, (arg, time): (T, Time)| {
                wrap_f("scheduled function", MizLua(lua), |lua| f(lua, arg, time))
            })?;
        Ok(record_perf!(
            timer_schedule_function,
            self.t.call_function("scheduleFunction", (f, arg, when))?
        ))
    }

    pub fn remove_function(&self, id: FunId) -> Result<()> {
        Ok(record_perf!(
            timer_remove_function,
            self.t.call_function("removeFunction", id)?
        ))
    }

    pub fn set_function_time(&self, id: FunId, when: f64) -> Result<()> {
        Ok(record_perf!(
            timer_remove_function,
            self.t.call_function("removeFunction", (id, when))?
        ))
    }
}
