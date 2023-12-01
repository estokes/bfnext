use super::{as_tbl, event::Event, unit::Unit, String};
use crate::{airbase::Airbase, wrapped_table, Sequence, MizLua, LuaEnv};
use compact_str::format_compact;
use mlua::{prelude::*, Value};
use serde_derive::Serialize;
use std::{
    ops::Deref,
    sync::atomic::{AtomicU32, Ordering},
};

#[derive(Debug, Serialize)]
pub struct HandlerId(u32);

impl HandlerId {
    fn new() -> Self {
        static NEXT: AtomicU32 = AtomicU32::new(0);
        Self(NEXT.fetch_add(1, Ordering::Relaxed))
    }

    fn key(&self) -> String {
        String(format_compact!("rustHandler{}", self.0))
    }
}

wrapped_table!(World, None);

impl<'lua> World<'lua> {
    pub fn singleton(lua: MizLua<'lua>) -> LuaResult<Self> {
        lua.inner().globals().raw_get("world")
    }

    pub fn add_event_handler<F>(&self, f: F) -> LuaResult<HandlerId>
    where
        F: Fn(MizLua<'lua>, Event) -> LuaResult<()> + 'static,
    {
        let globals = self.lua.globals();
        let id = HandlerId::new();
        let tbl = self.lua.create_table()?;
        tbl.set(
            "onEvent",
            self.lua
                .create_function(move |lua, (_, ev): (Value, Value)| {
                    match Event::from_lua(ev, lua) {
                        Err(e) => {
                            println!("error translating event: {:?}", e);
                            Ok(())
                        }
                        Ok(ev) => match f(MizLua(lua), ev) {
                            Ok(()) => Ok(()),
                            Err(e) => {
                                println!("error in event handler: {:?}", e);
                                Ok(())
                            }
                        },
                    }
                })?,
        )?;
        self.t.call_function("addEventHandler", tbl.clone())?;
        globals.raw_set(id.key(), tbl)?;
        Ok(id)
    }

    pub fn remove_event_handler(&self, id: HandlerId) -> LuaResult<()> {
        let globals = self.lua.globals();
        let key = id.key();
        let handler = globals.raw_get(key.clone())?;
        let handler = as_tbl("EventHandler", None, handler)?;
        self.t.call_function("removeEventHandler", handler)?;
        globals.raw_remove(key)?;
        Ok(())
    }

    pub fn get_player(&self) -> LuaResult<Sequence<Unit>> {
        self.t.call_function("getPlayer", ())
    }

    pub fn get_airbases(&self) -> LuaResult<Sequence<Airbase>> {
        self.t.call_function("getAirbases", ())
    }
}
