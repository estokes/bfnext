/*
Copyright 2024 Eric Stokes.

This file is part of dcso3.

dcso3 is free software: you can redistribute it and/or modify it under
the terms of the MIT License.

dcso3 is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
*/

extern crate nalgebra as na;
use crate::{
    coalition::Side,
    net::{PlayerId, SlotId, Ucid},
    wrap_f, HooksLua, LuaEnv, String,
};
use anyhow::Result;
use mlua::prelude::*;

#[derive(Debug)]
pub struct UserHooks<'lua> {
    on_mission_load_begin: Option<mlua::Function<'lua>>,
    on_mission_load_progress: Option<mlua::Function<'lua>>,
    on_mission_load_end: Option<mlua::Function<'lua>>,
    on_simulation_start: Option<mlua::Function<'lua>>,
    on_simulation_stop: Option<mlua::Function<'lua>>,
    on_simulation_frame: Option<mlua::Function<'lua>>,
    on_simulation_pause: Option<mlua::Function<'lua>>,
    on_simulation_resume: Option<mlua::Function<'lua>>,
    on_player_connect: Option<mlua::Function<'lua>>,
    on_player_disconnect: Option<mlua::Function<'lua>>,
    on_player_start: Option<mlua::Function<'lua>>,
    on_player_stop: Option<mlua::Function<'lua>>,
    on_player_change_slot: Option<mlua::Function<'lua>>,
    on_player_try_connect: Option<mlua::Function<'lua>>,
    on_player_try_send_chat: Option<mlua::Function<'lua>>,
    on_player_try_change_slot: Option<mlua::Function<'lua>>,
    lua: &'lua Lua,
}

impl<'lua> UserHooks<'lua> {
    pub fn new(lua: HooksLua<'lua>) -> Self {
        Self {
            on_mission_load_begin: None,
            on_mission_load_progress: None,
            on_mission_load_end: None,
            on_simulation_start: None,
            on_simulation_stop: None,
            on_simulation_frame: None,
            on_simulation_pause: None,
            on_simulation_resume: None,
            on_player_connect: None,
            on_player_disconnect: None,
            on_player_start: None,
            on_player_stop: None,
            on_player_change_slot: None,
            on_player_try_change_slot: None,
            on_player_try_connect: None,
            on_player_try_send_chat: None,
            lua: lua.inner(),
        }
    }

    pub fn register(&mut self) -> Result<()> {
        let Self {
            on_mission_load_begin,
            on_mission_load_progress,
            on_mission_load_end,
            on_simulation_start,
            on_simulation_stop,
            on_simulation_frame,
            on_simulation_pause,
            on_simulation_resume,
            on_player_connect,
            on_player_disconnect,
            on_player_start,
            on_player_stop,
            on_player_change_slot,
            on_player_try_connect,
            on_player_try_send_chat,
            on_player_try_change_slot,
            lua: _,
        } = self;
        let tbl = self.lua.create_table()?;
        if let Some(f) = on_mission_load_begin.take() {
            tbl.set("onMissionLoadBegin", f)?;
        }
        if let Some(f) = on_mission_load_progress.take() {
            tbl.set("onMissionLoadProgress", f)?;
        }
        if let Some(f) = on_mission_load_end.take() {
            tbl.set("onMissionLoadEnd", f)?;
        }
        if let Some(f) = on_simulation_start.take() {
            tbl.set("onSimulationStart", f)?;
        }
        if let Some(f) = on_simulation_stop.take() {
            tbl.set("onSimulationStop", f)?;
        }
        if let Some(f) = on_simulation_frame.take() {
            tbl.set("onSimulationFrame", f)?;
        }
        if let Some(f) = on_simulation_pause.take() {
            tbl.set("onSimulationPause", f)?;
        }
        if let Some(f) = on_simulation_resume.take() {
            tbl.set("onSimulationResume", f)?;
        }
        if let Some(f) = on_player_connect.take() {
            tbl.set("onPlayerConnect", f)?;
        }
        if let Some(f) = on_player_disconnect.take() {
            tbl.set("onPlayerDisconnect", f)?;
        }
        if let Some(f) = on_player_start.take() {
            tbl.set("onPlayerStart", f)?;
        }
        if let Some(f) = on_player_stop.take() {
            tbl.set("onPlayerStop", f)?;
        }
        if let Some(f) = on_player_change_slot.take() {
            tbl.set("onPlayerChangeSlot", f)?;
        }
        if let Some(f) = on_player_try_connect.take() {
            tbl.set("onPlayerTryConnect", f)?;
        }
        if let Some(f) = on_player_try_send_chat.take() {
            tbl.set("onPlayerTrySendChat", f)?;
        }
        if let Some(f) = on_player_try_change_slot.take() {
            tbl.set("onPlayerTryChangeSlot", f)?;
        }
        let dcs: mlua::Table = self.lua.globals().get("DCS")?;
        Ok(dcs.call_function("setUserCallbacks", tbl)?)
    }

    pub fn on_mission_load_begin<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua) -> Result<()> + 'static,
    {
        self.on_mission_load_begin =
            Some(self.lua.create_function(move |lua, ()| {
                wrap_f("on_mission_load_begin", HooksLua(lua), &f)
            })?);
        Ok(self)
    }

    /// f(progress, message)
    pub fn on_mission_load_progress<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, String, String) -> Result<()> + 'static,
    {
        self.on_mission_load_progress = Some(self.lua.create_function(
            move |lua, (progress, message): (String, String)| {
                wrap_f("on_mission_load_progress", HooksLua(lua), |lua| {
                    f(lua, progress, message)
                })
            },
        )?);
        Ok(self)
    }

    pub fn on_mission_load_end<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua) -> Result<()> + 'static,
    {
        self.on_mission_load_end =
            Some(self.lua.create_function(move |lua, ()| {
                wrap_f("on_mission_load_end", HooksLua(lua), &f)
            })?);
        Ok(self)
    }

    pub fn on_simulation_start<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua) -> Result<()> + 'static,
    {
        self.on_simulation_start =
            Some(self.lua.create_function(move |lua, ()| {
                wrap_f("on_simulation_start", HooksLua(lua), &f)
            })?);
        Ok(self)
    }

    pub fn on_simulation_stop<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua) -> Result<()> + 'static,
    {
        self.on_simulation_stop = Some(
            self.lua
                .create_function(move |lua, ()| wrap_f("on_simulation_stop", HooksLua(lua), &f))?,
        );
        Ok(self)
    }

    pub fn on_simulation_frame<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua) -> Result<()> + 'static,
    {
        self.on_simulation_frame =
            Some(self.lua.create_function(move |lua, ()| {
                wrap_f("on_simulation_frame", HooksLua(lua), &f)
            })?);
        Ok(self)
    }

    pub fn on_simulation_pause<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua) -> Result<()> + 'static,
    {
        self.on_simulation_pause =
            Some(self.lua.create_function(move |lua, ()| {
                wrap_f("on_simulation_pause", HooksLua(lua), &f)
            })?);
        Ok(self)
    }

    pub fn on_simulation_resume<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua) -> Result<()> + 'static,
    {
        self.on_simulation_resume =
            Some(self.lua.create_function(move |lua, ()| {
                wrap_f("on_simulation_resume", HooksLua(lua), &f)
            })?);
        Ok(self)
    }

    pub fn on_player_connect<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, PlayerId) -> Result<()> + 'static,
    {
        self.on_player_connect = Some(self.lua.create_function(move |lua, id| {
            wrap_f("on_player_connect", HooksLua(lua), |lua| f(lua, id))
        })?);
        Ok(self)
    }

    pub fn on_player_disconnect<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, PlayerId) -> Result<()> + 'static,
    {
        self.on_player_disconnect = Some(self.lua.create_function(move |lua, id| {
            wrap_f("on_player_disconnect", HooksLua(lua), |lua| f(lua, id))
        })?);
        Ok(self)
    }

    pub fn on_player_start<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, PlayerId) -> Result<()> + 'static,
    {
        self.on_player_start = Some(self.lua.create_function(move |lua, id| {
            wrap_f("on_player_start", HooksLua(lua), |lua| f(lua, id))
        })?);
        Ok(self)
    }

    pub fn on_player_stop<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, PlayerId) -> Result<()> + 'static,
    {
        self.on_player_stop = Some(self.lua.create_function(move |lua, id| {
            wrap_f("on_player_stop", HooksLua(lua), |lua| f(lua, id))
        })?);
        Ok(self)
    }

    pub fn on_player_change_slot<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, PlayerId) -> Result<()> + 'static,
    {
        self.on_player_change_slot = Some(self.lua.create_function(move |lua, id| {
            wrap_f("on_player_change_slot", HooksLua(lua), |lua| f(lua, id))
        })?);
        Ok(self)
    }

    /// f(addr, ucid, name, id), return `None` to accept the player,
    /// return `Some("reason for rejection")` to reject the player.
    pub fn on_player_try_connect<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, String, String, Ucid, PlayerId) -> Result<Option<String>> + 'static,
    {
        self.on_player_try_connect = Some(self.lua.create_function(
            move |lua, (addr, name, ucid, id): (String, String, Ucid, PlayerId)| {
                wrap_f("on_player_try_connect", HooksLua(lua), |lua| {
                    let mut rval = LuaMultiValue::new();
                    match f(lua, addr, name, ucid, id) {
                        Err(e) => return Err(e),
                        Ok(None) => rval.push_front(LuaValue::Boolean(true)),
                        Ok(Some(reason)) => {
                            rval.push_front(reason.into_lua(lua.inner())?);
                            rval.push_front(LuaValue::Boolean(false));
                        }
                    }
                    Ok(rval)
                })
            },
        )?);
        Ok(self)
    }

    /// f(id, message, all)
    pub fn on_player_try_send_chat<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, PlayerId, String, bool) -> Result<String> + 'static,
    {
        self.on_player_try_send_chat = Some(self.lua.create_function(
            move |lua, (id, msg, all): (PlayerId, String, bool)| {
                wrap_f("on_player_try_send_chat", HooksLua(lua), |lua| {
                    f(lua, id, msg, all)
                })
            },
        )?);
        Ok(self)
    }

    /// f(id, message, all)
    pub fn on_player_try_change_slot<F>(&mut self, f: F) -> Result<&mut Self>
    where
        F: Fn(HooksLua, PlayerId, Side, SlotId) -> Result<Option<bool>> + 'static,
    {
        self.on_player_try_change_slot = Some(self.lua.create_function(
            move |lua, (id, side, slot): (PlayerId, Side, SlotId)| {
                wrap_f("on_player_try_change_slot", HooksLua(lua), |lua| {
                    f(lua, id, side, slot)
                })
            },
        )?);
        Ok(self)
    }
}
