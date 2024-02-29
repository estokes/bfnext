/*
Copyright 2024 Eric Stokes.

This file is part of bflib.

bflib is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your
option) any later version.

bflib is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
for more details.
*/

pub(crate) mod cargo;
mod ewr;
mod jtac;
mod troop;

use std::sync::Arc;

use crate::{cfg::Cfg, db::Db, Context};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use compact_str::format_compact;
use dcso3::{
    as_tbl,
    coalition::Side,
    env::miz::{GroupId, Miz},
    lua_err,
    mission_commands::{GroupCommandItem, GroupSubMenu, MissionCommands},
    net::SlotId,
    MizLua, String,
};
use log::debug;
use mlua::{prelude::*, Value};

#[derive(Debug)]
struct ArgTuple<T, U> {
    fst: T,
    snd: U,
}

impl<'lua, T, U> IntoLua<'lua> for ArgTuple<T, U>
where
    T: IntoLua<'lua>,
    U: IntoLua<'lua>,
{
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let tbl = lua.create_table()?;
        tbl.raw_set(1, self.fst)?;
        tbl.raw_set(2, self.snd)?;
        Ok(Value::Table(tbl))
    }
}

impl<'lua, T, U> FromLua<'lua> for ArgTuple<T, U>
where
    T: FromLua<'lua>,
    U: FromLua<'lua>,
{
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("ArgTuple", None, value).map_err(lua_err)?;
        Ok(Self {
            fst: tbl.raw_get(1)?,
            snd: tbl.raw_get(2)?,
        })
    }
}

#[derive(Debug)]
struct ArgTriple<T, U, V> {
    fst: T,
    snd: U,
    trd: V,
}

impl<'lua, T, U, V> IntoLua<'lua> for ArgTriple<T, U, V>
where
    T: IntoLua<'lua>,
    U: IntoLua<'lua>,
    V: IntoLua<'lua>,
{
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let tbl = lua.create_table()?;
        tbl.raw_set(1, self.fst)?;
        tbl.raw_set(2, self.snd)?;
        tbl.raw_set(3, self.trd)?;
        Ok(Value::Table(tbl))
    }
}

impl<'lua, T, U, V> FromLua<'lua> for ArgTriple<T, U, V>
where
    T: FromLua<'lua>,
    U: FromLua<'lua>,
    V: FromLua<'lua>,
{
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("ArgTriple", None, value).map_err(lua_err)?;
        Ok(Self {
            fst: tbl.raw_get(1)?,
            snd: tbl.raw_get(2)?,
            trd: tbl.raw_get(3)?,
        })
    }
}

#[derive(Debug)]
struct ArgQuad<T, U, V, W> {
    fst: T,
    snd: U,
    trd: V,
    fth: W,
}

impl<'lua, T, U, V, W> IntoLua<'lua> for ArgQuad<T, U, V, W>
where
    T: IntoLua<'lua>,
    U: IntoLua<'lua>,
    V: IntoLua<'lua>,
    W: IntoLua<'lua>,
{
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let tbl = lua.create_table()?;
        tbl.raw_set(1, self.fst)?;
        tbl.raw_set(2, self.snd)?;
        tbl.raw_set(3, self.trd)?;
        tbl.raw_set(4, self.fth)?;
        Ok(Value::Table(tbl))
    }
}

impl<'lua, T, U, V, W> FromLua<'lua> for ArgQuad<T, U, V, W>
where
    T: FromLua<'lua>,
    U: FromLua<'lua>,
    V: FromLua<'lua>,
    W: FromLua<'lua>,
{
    fn from_lua(value: LuaValue<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("ArgQuad", None, value).map_err(lua_err)?;
        Ok(Self {
            fst: tbl.raw_get(1)?,
            snd: tbl.raw_get(2)?,
            trd: tbl.raw_get(3)?,
            fth: tbl.raw_get(4)?,
        })
    }
}

fn slot_for_group(lua: MizLua, ctx: &Context, gid: &GroupId) -> Result<(Side, SlotId)> {
    let miz = Miz::singleton(lua)?;
    let group = miz
        .get_group(&ctx.idx, gid)
        .with_context(|| format_compact!("getting group {:?} from miz", gid))?
        .ok_or_else(|| anyhow!("no such group {:?}", gid))?;
    let units = group.group.units().context("getting units")?;
    if units.len() > 1 {
        bail!(
            "groups with more than one member can't spawn crates {:?}",
            gid
        )
    }
    let unit = units.first().context("getting first unit")?;
    Ok((group.side, unit.slot().context("getting unit slot")?))
}

fn player_name(db: &Db, slot: &SlotId) -> String {
    db.ephemeral
        .player_in_slot(&slot)
        .and_then(|ucid| db.player(ucid).map(|p| p.name.clone()))
        .unwrap_or_default()
}

#[derive(Debug, Clone, Copy, Default)]
struct CarryCap {
    troops: bool,
    crates: bool,
}

impl CarryCap {
    fn from_typ(cfg: &Cfg, typ: &str) -> CarryCap {
        cfg.cargo
            .get(&*typ)
            .map(|c| CarryCap {
                troops: c.troop_slots > 0 && c.total_slots > 0,
                crates: c.crate_slots > 0 && c.total_slots > 0,
            })
            .unwrap_or_default()
    }
}

pub(super) fn init_jtac_menu_for_slot(ctx: &mut Context, lua: MizLua, slot: &SlotId) -> Result<()> {
    let ucid = match ctx.db.ephemeral.player_in_slot(slot) {
        Some(ucid) => ucid,
        None => return Ok(()),
    };
    let mc = MissionCommands::singleton(lua)?;
    let si = ctx.db.info_for_slot(slot).context("getting slot info")?;
    ctx.subscribed_jtac_menus.remove(slot);
    mc.remove_command_for_group(si.miz_gid, GroupCommandItem::from(vec!["JTAC".into()]))?;
    mc.remove_submenu_for_group(si.miz_gid, GroupSubMenu::from(vec!["JTAC".into()]))?;
    mc.add_command_for_group(
        si.miz_gid,
        "JTAC".into(),
        None,
        jtac::add_jtac_locations,
        ArgTuple {
            fst: *ucid,
            snd: si.miz_gid,
        },
    )?;
    Ok(())
}

pub(super) fn init_for_slot(ctx: &mut Context, lua: MizLua, slot: &SlotId) -> Result<()> {
    debug!("initializing menus for {slot:?}");
    let cfg = Arc::clone(&ctx.db.ephemeral.cfg);
    let mc = MissionCommands::singleton(lua)?;
    match slot {
        SlotId::Spectator => Ok(()),
        SlotId::ArtilleryCommander(_, _)
        | SlotId::ForwardObserver(_, _)
        | SlotId::Instructor(_, _)
        | SlotId::Observer(_, _) => Ok(()),
        SlotId::Unit(_) | SlotId::MultiCrew(_, _) => {
            let ucid = match ctx.db.ephemeral.player_in_slot(slot) {
                Some(ucid) => *ucid,
                None => return Ok(()),
            };
            let si = ctx.db.info_for_slot(slot).context("getting slot info")?;
            mc.remove_submenu_for_group(si.miz_gid, GroupSubMenu::from(vec!["EWR".into()]))?;
            mc.remove_submenu_for_group(si.miz_gid, GroupSubMenu::from(vec!["Cargo".into()]))?;
            mc.remove_submenu_for_group(si.miz_gid, GroupSubMenu::from(vec!["Troops".into()]))?;
            ewr::add_ewr_menu_for_group(&mc, si.miz_gid)?;
            let cap = CarryCap::from_typ(&cfg, si.typ.as_str());
            if cap.crates && ctx.db.ephemeral.cfg.rules.cargo.check(&ucid) {
                cargo::add_cargo_menu_for_group(&cfg, &mc, &si.side, si.miz_gid)?
            }
            if cap.troops && ctx.db.ephemeral.cfg.rules.troops.check(&ucid) {
                troop::add_troops_menu_for_group(&cfg, &mc, &si.side, si.miz_gid)?
            }
            if ctx.db.ephemeral.cfg.rules.jtac.check(&ucid) {
                init_jtac_menu_for_slot(ctx, lua, slot)?
            }
            Ok(())
        }
    }
}
