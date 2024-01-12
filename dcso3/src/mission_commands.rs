/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

use crate::{
    as_tbl, coalition::Side, env::miz::GroupId, wrap_f, wrapped_table, LuaEnv, MizLua, String,
};
use anyhow::Result;
use compact_str::format_compact;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::ops::Deref;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
struct ItemPath(Vec<String>);

impl<'lua> IntoLua<'lua> for ItemPath {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let tbl = lua.create_table()?;
        for s in self.0 {
            tbl.raw_push(s)?
        }
        Ok(Value::Table(tbl))
    }
}

impl<'lua> FromLua<'lua> for ItemPath {
    fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = LuaTable::from_lua(value, lua)?;
        let mut res = Vec::new();
        for v in tbl.sequence_values() {
            let v = v?;
            res.push(String::from_lua(v, lua)?);
        }
        Ok(Self(res))
    }
}

macro_rules! item {
    ($name:ident) => {
        #[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
        pub struct $name(ItemPath);

        impl<'lua> IntoLua<'lua> for $name {
            fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
                self.0.into_lua(lua)
            }
        }

        impl<'lua> FromLua<'lua> for $name {
            fn from_lua(value: Value<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
                Ok(Self(ItemPath::from_lua(value, lua)?))
            }
        }

        impl From<Vec<String>> for $name {
            fn from(v: Vec<String>) -> Self {
                Self(ItemPath(v))
            }
        }
    };
}

item!(SubMenu);
item!(CoalitionSubMenu);
item!(GroupSubMenu);
item!(CommandItem);
item!(CoalitionCommandItem);
item!(GroupCommandItem);

wrapped_table!(MissionCommands, None);

impl<'lua> MissionCommands<'lua> {
    pub fn singleton(lua: MizLua<'lua>) -> Result<Self> {
        Ok(lua.inner().globals().raw_get("missionCommands")?)
    }

    pub fn add_submenu(&self, name: String, parent: Option<SubMenu>) -> Result<SubMenu> {
        Ok(self.call_function("addSubMenu", (name, parent))?)
    }

    pub fn add_command<F, A>(
        &self,
        name: String,
        parent: Option<SubMenu>,
        f: F,
        arg: A,
    ) -> Result<CommandItem>
    where
        F: Fn(MizLua, A) -> Result<()> + 'static,
        A: IntoLua<'lua> + FromLua<'lua>,
    {
        let msg = format_compact!("command {:?},{name}", parent);
        let f = self.lua.create_function(move |lua, arg: A| {
            wrap_f(msg.as_str(), MizLua(lua), |lua| f(lua, arg))
        })?;
        Ok(self.call_function("addCommand", (name, parent, f, arg))?)
    }

    pub fn remove_submenu(&self, menu: SubMenu) -> Result<()> {
        Ok(self.call_function("removeItem", menu)?)
    }

    pub fn remove_command(&self, item: CommandItem) -> Result<()> {
        Ok(self.call_function("removeItem", item)?)
    }

    pub fn add_submenu_for_coalition(
        &self,
        side: Side,
        name: String,
        parent: Option<CoalitionSubMenu>,
    ) -> Result<CoalitionSubMenu> {
        Ok(self.call_function("addSubMenuForCoalition", (side, name, parent))?)
    }

    pub fn add_command_for_coalition<F, A>(
        &self,
        side: Side,
        name: String,
        parent: Option<CoalitionSubMenu>,
        f: F,
        arg: A,
    ) -> Result<CoalitionCommandItem>
    where
        F: Fn(MizLua, A) -> Result<()> + 'static,
        A: IntoLua<'lua> + FromLua<'lua>,
    {
        let msg = format_compact!("coa cmd {:?},{name}", parent);
        let f = self.lua.create_function(move |lua, arg: A| {
            wrap_f(msg.as_str(), MizLua(lua), |lua| f(lua, arg))
        })?;
        Ok(self.call_function("addCommandForCoalition", (side, name, parent, f, arg))?)
    }

    pub fn remove_submenu_for_coalition(&self, side: Side, menu: CoalitionSubMenu) -> Result<()> {
        Ok(self.call_function("removeItemForCoalition", (side, menu))?)
    }

    pub fn remove_command_for_coalition(&self, side: Side, item: CoalitionCommandItem) -> Result<()> {
        Ok(self.call_function("removeItemForCoalition", (side, item))?)
    }

    pub fn add_submenu_for_group(
        &self,
        group: GroupId,
        name: String,
        parent: Option<GroupSubMenu>,
    ) -> Result<GroupSubMenu> {
        Ok(self.call_function("addSubMenuForGroup", (group, name, parent))?)
    }

    pub fn add_command_for_group<F, A>(
        &self,
        group: GroupId,
        name: String,
        parent: Option<GroupSubMenu>,
        f: F,
        arg: A,
    ) -> Result<GroupCommandItem>
    where
        F: Fn(MizLua, A) -> Result<()> + 'static,
        A: IntoLua<'lua> + FromLua<'lua>,
    {
        let msg = format_compact!("grp cmd {:?}, {name}", parent);
        let f = self.lua.create_function(move |lua, arg: A| {
            wrap_f(msg.as_str(), MizLua(lua), |lua| f(lua, arg))
        })?;
        Ok(self.call_function("addCommandForGroup", (group, name, parent, f, arg))?)
    }

    pub fn remove_submenu_for_group(&self, group: GroupId, menu: GroupSubMenu) -> Result<()> {
        Ok(self.call_function("removeItemForGroup", (group, menu))?)
    }

    pub fn remove_command_for_group(&self, group: GroupId, item: GroupCommandItem) -> Result<()> {
        Ok(self.call_function("removeItemForGroup", (group, item))?)
    }

    pub fn clear_all_menus(&self) -> Result<()> {
        Ok(self.call_function("removeItem", ())?)
    }
}
