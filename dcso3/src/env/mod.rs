/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use crate::{as_tbl, wrapped_table, LuaEnv, String};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

pub mod miz;
pub mod warehouse;

wrapped_table!(Env, None);

impl<'lua> Env<'lua> {
    pub fn singleton<L: LuaEnv<'lua>>(lua: L) -> Result<Self> {
        Ok(lua.inner().globals().raw_get("env")?)
    }

    pub fn get_value_dict_by_key(&self, key: String) -> Result<String> {
        Ok(self.t.call_function("getValueDictByKey", key)?)
    }
}
