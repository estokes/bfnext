use super::{as_tbl, String};
use crate::wrapped_table;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

wrapped_table!(Lfs, None);

impl<'lua> Lfs<'lua> {
    pub fn singleton(lua: &'lua Lua) -> LuaResult<Self> {
        lua.globals().raw_get("lfs")
    }

    pub fn writedir(&self) -> LuaResult<String> {
        self.t.call_function("writedir", ())
    }

    pub fn tempdir(&self) -> LuaResult<String> {
        self.t.call_function("tempdir", ())
    }
}
