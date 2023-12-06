use crate::{as_tbl, coalition::Side, wrapped_table};
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
    pub fn add_submenu(
        &self,
        name: String,
        parent: Option<SubMenu>,
    ) -> LuaResult<SubMenu> {
        self.call_function("addSubMenu", (name, parent))
    }

    pub fn add_command<F, A>(
        &self,
        name: String,
        parent: Option<SubMenu>,
        f: F,
        arg: Option<A>,
    ) -> LuaResult<CommandItem>
    where
        F: Fn(&Lua, Option<A>) -> LuaResult<()> + 'static,
        A: IntoLua<'lua> + FromLua<'lua>,
    {
        let f = self.lua.create_function(f)?;
        self.call_function("addCommand", (name, parent, f, arg))
    }

    pub fn remove_submenu(&self, menu: SubMenu) -> LuaResult<()> {
        self.call_function("removeItem", menu)
    }

    pub fn remove_command(&self, item: CommandItem) -> LuaResult<()> {
        self.call_function("removeItem", item)
    }

    pub fn add_submenu_for_coalition(
        &self,
        side: Side,
        name: String,
        parent: Option<CoalitionSubMenu>,
    ) -> LuaResult<CoalitionSubMenu> {
        self.call_function("addSubMenuForCoalition", (side, name, parent))
    }

    pub fn add_command_for_coalition<F, A>(
        &self,
        side: Side,
        name: String,
        parent: Option<CoalitionSubMenu>,
        f: F,
        arg: Option<A>,
    ) -> LuaResult<CoalitionCommandItem>
    where
        F: Fn(&Lua, Option<A>) -> LuaResult<()> + 'static,
        A: IntoLua<'lua> + FromLua<'lua>,
    {
        let f = self.lua.create_function(f)?;
        self.call_function("addCommandForCoalition", (side, name, parent, f, arg))
    }

    pub fn remove_submenu_for_coalition(&self, menu: CoalitionSubMenu) -> LuaResult<()> {
        self.call_function("removeItemForCoalition", menu)
    }

    pub fn remove_command_for_coalition(&self, item: CoalitionCommandItem) -> LuaResult<()> {
        self.call_function("removeItemForCoalition", item)
    }

    pub fn add_submenu_for_group(
        &self,
        group: i64,
        name: String,
        parent: Option<GroupSubMenu>,
    ) -> LuaResult<GroupSubMenu> {
        self.call_function("addSubMenuForGroup", (group, name, parent))
    }

    pub fn add_command_for_group<F, A>(
        &self,
        group: i64,
        name: String,
        parent: Option<GroupSubMenu>,
        f: F,
        arg: Option<A>,
    ) -> LuaResult<GroupCommandItem>
    where
        F: Fn(&Lua, Option<A>) -> LuaResult<()> + 'static,
        A: IntoLua<'lua> + FromLua<'lua>,
    {
        let f = self.lua.create_function(f)?;
        self.call_function("addCommandForGroup", (group, name, parent, f, arg))
    }

    pub fn remove_submenu_for_group(&self, menu: GroupSubMenu) -> LuaResult<()> {
        self.call_function("removeItemForGroup", menu)
    }

    pub fn remove_command_for_group(&self, item: GroupCommandItem) -> LuaResult<()> {
        self.call_function("removeItemForGroup", item)
    }

    pub fn clear_all_menus(&self) -> LuaResult<()> {
        self.call_function("removeItem", ())
    }
}
