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
