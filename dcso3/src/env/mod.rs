use crate::{as_tbl, wrapped_table, LuaEnv, String};
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

pub mod miz;
pub mod warehouse;

wrapped_table!(Env, None);

impl<'lua> Env<'lua> {
    pub fn singleton<L: LuaEnv<'lua>>(lua: L) -> LuaResult<Self> {
        lua.inner().globals().raw_get("env")
    }

    pub fn get_value_dict_by_key(&self, key: String) -> LuaResult<String> {
        self.t.call_function("getValueDictByKey", key)
    }
}
