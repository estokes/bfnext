/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use super::{as_tbl, String};
use crate::{wrapped_table, LuaEnv};
use anyhow::Result;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(Lfs, None);

impl<'lua> Lfs<'lua> {
    pub fn singleton<L: LuaEnv<'lua>>(lua: L) -> Result<Self> {
        Ok(lua.inner().globals().raw_get("lfs")?)
    }

    pub fn writedir(&self) -> Result<String> {
        Ok(self.t.call_function("writedir", ())?)
    }

    pub fn tempdir(&self) -> Result<String> {
        Ok(self.t.call_function("tempdir", ())?)
    }
}
