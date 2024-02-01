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

use crate::{
    cfg::{Cfg, LimitEnforceTyp, UnitTag},
    db::{
        self,
        cargo::{Cargo, Oldest, SlotStats},
        group::GroupId as DbGid,
        Db,
    },
    ewr::{self, EwrUnits},
    jtac::AdjustmentDir,
    Context,
};
use anyhow::{anyhow, bail, Context as AnyhowContext, Result};
use chrono::prelude::*;
use compact_str::{format_compact, CompactString, ToCompactString};
use dcso3::{
    as_tbl,
    coalition::Side,
    env::miz::{GroupId, Miz},
    lua_err,
    mission_commands::{CoalitionSubMenu, GroupSubMenu, MissionCommands},
    net::SlotId,
    MizLua, String,
};
use enumflags2::{BitFlag, BitFlags};
use fxhash::FxHashMap;
use log::debug;
use mlua::{prelude::*, Value};
use std::{collections::hash_map::Entry, fmt::Write};

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

fn unpakistan(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    match ctx.db.unpakistan(lua, &ctx.idx, &slot) {
        Ok(unpakistan) => {
            let player = player_name(&ctx.db, &slot);
            let msg = format_compact!("{player} {unpakistan}");
            ctx.db.ephemeral.msgs().panel_to_side(10, false, side, msg);
        }
        Err(e) => {
            let msg = format_compact!("{}", e);
            ctx.db.ephemeral.msgs().panel_to_group(10, false, gid, msg)
        }
    }
    Ok(())
}

fn load_crate(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    match ctx.db.load_nearby_crate(lua, &ctx.idx, &slot) {
        Ok(cr) => {
            let (dep_name, limit_enforce, limit) = match ctx.db.deployable_by_crate(&side, &cr.name)
            {
                Some((dep_name, dep)) => (dep_name, &dep.limit_enforce, Some(dep.limit)),
                None => (&cr.name, &LimitEnforceTyp::DenyCrate, None),
            };
            let (n, oldest) = ctx
                .db
                .number_deployed(side, dep_name.as_str())
                .with_context(|| format_compact!("getting number of {} deployed", dep_name))?;
            let enforce = match limit_enforce {
                LimitEnforceTyp::DenyCrate => {
                    format_compact!("unpacking will be denied when the limit is exceeded")
                }
                LimitEnforceTyp::DeleteOldest => match oldest {
                    Some(Oldest::Group(gid)) => {
                        format_compact!(
                            "unpacking will delete oldest, {}, when the limit is exceeded",
                            gid
                        )
                    }
                    Some(Oldest::Objective(oid)) => {
                        format_compact!(
                            "unpacking will delete oldest, {}, when the limit is exceeded",
                            oid
                        )
                    }
                    None => {
                        format_compact!("unpacking will delete oldest when the limit is exceeded")
                    }
                },
            };
            let limit = limit
                .map(|i| i.to_compact_string())
                .unwrap_or_else(|| format_compact!("unlimited"));
            let msg = format_compact!(
                "{} crate loaded\n{n} of {} {} deployed, {}",
                cr.name,
                limit,
                dep_name,
                enforce
            );
            ctx.db.ephemeral.msgs().panel_to_group(10, false, gid, msg)
        }
        Err(e) => {
            let msg = format_compact!("crate could not be loaded: {}", e);
            ctx.db.ephemeral.msgs().panel_to_group(10, false, gid, msg)
        }
    }
    Ok(())
}

fn unload_crate(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    match ctx.db.unload_crate(lua, &ctx.idx, &slot) {
        Ok(cr) => {
            let msg = format_compact!("{} crate unloaded", cr.name);
            ctx.db.ephemeral.msgs().panel_to_group(10, false, gid, msg)
        }
        Err(e) => {
            let msg = format_compact!("{}", e);
            ctx.db.ephemeral.msgs().panel_to_group(10, false, gid, msg)
        }
    }
    Ok(())
}

pub(super) fn list_cargo_for_slot(lua: MizLua, ctx: &mut Context, slot: &SlotId) -> Result<()> {
    let cargo = Cargo::default();
    let cargo = ctx.db.list_cargo(&slot).unwrap_or(&cargo);
    let uinfo = db::cargo::slot_miz_unit(lua, &ctx.idx, &slot).context("getting slot miz unit")?;
    let capacity = ctx
        .db
        .cargo_capacity(&uinfo.unit)
        .context("getting unit cargo capacity")?;
    let mut msg = CompactString::new("Current Cargo\n----------------------------\n");
    msg.push_str(&format_compact!(
        "troops: {} of {}\n",
        cargo.num_troops(),
        capacity.troop_slots
    ));
    msg.push_str(&format_compact!(
        "crates: {} of {}\n",
        cargo.num_crates(),
        capacity.crate_slots
    ));
    msg.push_str(&format_compact!(
        "total : {} of {}\n",
        cargo.num_total(),
        capacity.total_slots
    ));
    msg.push_str("----------------------------\n");
    let mut total = 0;
    for (_, cr) in &cargo.crates {
        msg.push_str(&format_compact!(
            "{} crate weighing {} kg\n",
            cr.name,
            cr.weight
        ));
        total += cr.weight
    }
    for tr in &cargo.troops {
        msg.push_str(&format_compact!(
            "{} troop weiging {} kg\n",
            tr.name,
            tr.weight
        ));
        total += tr.weight
    }
    if total > 0 {
        msg.push_str("----------------------------\n");
    }
    msg.push_str(&format_compact!("total cargo weight: {} kg", total as u32));
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_unit(15, false, slot.as_unit_id().unwrap(), msg);
    Ok(())
}

pub fn list_current_cargo(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    list_cargo_for_slot(lua, ctx, &slot)
}

fn list_nearby_crates(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    let st = SlotStats::get(&ctx.db, lua, &slot).context("getting slot stats")?;
    let nearby = ctx
        .db
        .list_nearby_crates(&st)
        .context("listing nearby crates")?;
    if nearby.len() > 0 {
        let mut msg = CompactString::new("");
        for nc in nearby {
            msg.push_str(&format_compact!(
                "{} crate, bearing {}, {} meters away\n",
                nc.crate_def.name,
                nc.heading as u32,
                nc.distance as u32
            ));
        }
        ctx.db.ephemeral.msgs().panel_to_group(10, false, gid, msg)
    } else {
        drop(nearby);
        ctx.db
            .ephemeral
            .msgs()
            .panel_to_group(10, false, gid, "No nearby crates")
    }
    Ok(())
}

fn destroy_nearby_crate(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    if let Err(e) = ctx.db.destroy_nearby_crate(lua, &slot) {
        ctx.db
            .ephemeral
            .msgs()
            .panel_to_group(10, false, gid, format_compact!("{}", e))
    }
    Ok(())
}

fn spawn_crate(lua: MizLua, arg: ArgTuple<GroupId, String>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_side, slot) = slot_for_group(lua, ctx, &arg.fst).context("getting slot for group")?;
    match ctx.db.spawn_crate(lua, &ctx.idx, &slot, &arg.snd) {
        Err(e) => {
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_group(10, false, arg.fst, format_compact!("{e}"))
        }
        Ok(st) => {
            if let Some(max_crates) = ctx.db.ephemeral.cfg.max_crates {
                let (n, oldest) = ctx
                    .db
                    .number_crates_deployed(&st)
                    .context("getting number of deployed crates")?;
                let msg = match oldest {
                    None => format_compact!("{n} of {max_crates} crates spawned"),
                    Some(gid) => format_compact!(
                        "{n} of {max_crates} crates spawned, {gid} will be deleted if the limit is exceeded"
                    ),
                };
                ctx.db
                    .ephemeral
                    .msgs()
                    .panel_to_group(10, false, arg.fst, msg)
            }
        }
    }
    Ok(())
}

fn load_troops(lua: MizLua, arg: ArgTuple<GroupId, String>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot) = slot_for_group(lua, ctx, &arg.fst).context("getting slot for group")?;
    match ctx.db.load_troops(lua, &ctx.idx, &slot, &arg.snd) {
        Ok(tr) => {
            let (n, oldest) = ctx
                .db
                .number_troops_deployed(side, &tr.name)
                .context("getting number of deployed troops")?;
            let player = player_name(&ctx.db, &slot);
            let enforce = match tr.limit_enforce {
                LimitEnforceTyp::DenyCrate => {
                    format_compact!("unloading will be denied when the limit is exceeded")
                }
                LimitEnforceTyp::DeleteOldest => match oldest {
                    Some(gid) => {
                        format_compact!(
                            "unloading will delete oldest, {gid}, when the limit is exceeded"
                        )
                    }
                    None => {
                        format_compact!("unloading will delete oldest when the limit is exceeded")
                    }
                },
            };
            let msg = format_compact!(
                "{player} loaded {}\n{n} of {} {} deployed, {}",
                tr.name,
                tr.limit,
                tr.name,
                enforce
            );
            ctx.db.ephemeral.msgs().panel_to_side(10, false, side, msg)
        }
        Err(e) => {
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_group(10, false, arg.fst, format_compact!("{e}"))
        }
    }
    Ok(())
}

fn unload_troops(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    match ctx.db.unload_troops(lua, &ctx.idx, &slot) {
        Ok(tr) => {
            let player = player_name(&ctx.db, &slot);
            let msg = format_compact!("{player} dropped {} troops into the field", tr.name);
            ctx.db.ephemeral.msgs().panel_to_side(10, false, side, msg)
        }
        Err(e) => ctx
            .db
            .ephemeral
            .msgs()
            .panel_to_group(10, false, gid, format_compact!("{e}")),
    }
    Ok(())
}

fn extract_troops(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    match ctx.db.extract_troops(lua, &ctx.idx, &slot) {
        Ok(tr) => {
            let player = player_name(&ctx.db, &slot);
            let msg = format_compact!("{player} extracted {} troops from the field", tr.name);
            ctx.db.ephemeral.msgs().panel_to_side(10, false, side, msg)
        }
        Err(e) => ctx
            .db
            .ephemeral
            .msgs()
            .panel_to_group(10, false, gid, format_compact!("{e}")),
    }
    Ok(())
}

fn return_troops(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    match ctx.db.return_troops(lua, &ctx.idx, &slot) {
        Ok(tr) => {
            let player = player_name(&ctx.db, &slot);
            let msg = format_compact!("{player} returned {} troops", tr.name);
            ctx.db.ephemeral.msgs().panel_to_side(10, false, side, msg)
        }
        Err(e) => ctx
            .db
            .ephemeral
            .msgs()
            .panel_to_group(10, false, gid, format_compact!("{e}")),
    }
    Ok(())
}

fn toggle_ewr(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    if let Some(ucid) = ctx.db.ephemeral.player_in_slot(&slot) {
        let st = if ctx.ewr.toggle(ucid) {
            "enabled"
        } else {
            "disabled"
        };
        ctx.db.ephemeral.msgs().panel_to_group(
            5,
            false,
            gid,
            format_compact!("ewr reports are {st}"),
        )
    }
    Ok(())
}

fn ewr_report(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    let mut report = format_compact!("Bandits BRAA\n");
    if let Some(ucid) = ctx.db.ephemeral.player_in_slot(&slot) {
        if let Some(player) = ctx.db.player(ucid) {
            if let Some((_, Some(inst))) = &player.current_slot {
                let chickens = ctx
                    .ewr
                    .where_chicken(Utc::now(), false, true, ucid, player, inst);
                write!(report, "{}\n", ewr::HEADER)?;
                for braa in chickens {
                    write!(report, "{braa}\n")?;
                }
            }
        }
    }
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_group(10, false, gid, report);
    Ok(())
}

fn friendly_ewr_report(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    let mut report = format_compact!("Friendlies BRAA\n");
    if let Some(ucid) = ctx.db.ephemeral.player_in_slot(&slot) {
        if let Some(player) = ctx.db.player(ucid) {
            if let Some((_, Some(inst))) = &player.current_slot {
                let friendlies = ctx
                    .ewr
                    .where_chicken(Utc::now(), true, true, ucid, player, inst);
                write!(report, "{}\n", ewr::HEADER)?;
                for braa in friendlies {
                    write!(report, "{braa}\n")?;
                }
            }
        }
    }
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_group(10, false, gid, report);
    Ok(())
}

fn ewr_units_imperial(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    if let Some(ucid) = ctx.db.ephemeral.player_in_slot(&slot) {
        ctx.ewr.set_units(ucid, EwrUnits::Imperial);
        ctx.db
            .ephemeral
            .msgs()
            .panel_to_group(5, false, gid, "EWR units are now Imperial");
    }
    Ok(())
}

fn ewr_units_metric(lua: MizLua, gid: GroupId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (_, slot) = slot_for_group(lua, ctx, &gid).context("getting slot for group")?;
    if let Some(ucid) = ctx.db.ephemeral.player_in_slot(&slot) {
        ctx.ewr.set_units(ucid, EwrUnits::Imperial);
        ctx.db
            .ephemeral
            .msgs()
            .panel_to_group(5, false, gid, "EWR units are now Metric");
    }
    Ok(())
}

fn add_troops_menu_for_group(
    cfg: &Cfg,
    mc: &MissionCommands,
    side: &Side,
    group: GroupId,
) -> Result<()> {
    if let Some(squads) = cfg.troops.get(side) {
        let root = mc.add_submenu_for_group(group, "Troops".into(), None)?;
        mc.add_command_for_group(
            group,
            "Unload".into(),
            Some(root.clone()),
            unload_troops,
            group,
        )?;
        mc.add_command_for_group(
            group,
            "Extract".into(),
            Some(root.clone()),
            extract_troops,
            group,
        )?;
        mc.add_command_for_group(
            group,
            "List".into(),
            Some(root.clone()),
            list_current_cargo,
            group,
        )?;
        mc.add_command_for_group(
            group,
            "Return".into(),
            Some(root.clone()),
            return_troops,
            group,
        )?;
        let root = mc.add_submenu_for_group(group, "Squads".into(), Some(root))?;
        for sq in squads {
            mc.add_command_for_group(
                group,
                format_compact!("Load {} squad", sq.name).into(),
                Some(root.clone()),
                load_troops,
                ArgTuple {
                    fst: group,
                    snd: sq.name.clone(),
                },
            )?;
        }
    }
    Ok(())
}

fn add_cargo_menu_for_group(
    cfg: &Cfg,
    mc: &MissionCommands,
    side: &Side,
    group: GroupId,
) -> Result<()> {
    let root = mc.add_submenu_for_group(group, "Cargo".into(), None)?;
    mc.add_command_for_group(
        group,
        "Unpack Nearby Crate(s)".into(),
        Some(root.clone()),
        unpakistan,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Load Nearby Crate".into(),
        Some(root.clone()),
        load_crate,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Unload Crate".into(),
        Some(root.clone()),
        unload_crate,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "List Nearby Crates".into(),
        Some(root.clone()),
        list_nearby_crates,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "List Cargo".into(),
        Some(root.clone()),
        list_current_cargo,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Destroy Nearby Crate".into(),
        Some(root.clone()),
        destroy_nearby_crate,
        group,
    )?;
    let root = mc.add_submenu_for_group(group, "Crates".into(), Some(root.clone()))?;
    let rep = &cfg.repair_crate[side];
    let logi = mc.add_submenu_for_group(group, "Logistics".into(), Some(root.clone()))?;
    mc.add_command_for_group(
        group,
        rep.name.clone(),
        Some(logi.clone()),
        spawn_crate,
        ArgTuple {
            fst: group,
            snd: rep.name.clone(),
        },
    )?;
    if let Some(whcfg) = &cfg.warehouse {
        let cr = &whcfg.supply_transfer_crate[&side];
        mc.add_command_for_group(
            group,
            cr.name.clone(),
            Some(logi.clone()),
            spawn_crate,
            ArgTuple {
                fst: group,
                snd: cr.name.clone(),
            },
        )?;
    }
    let mut created_menus: FxHashMap<String, GroupSubMenu> = FxHashMap::default();
    for dep in cfg.deployables.get(side).unwrap_or(&vec![]) {
        let root = dep
            .path
            .iter()
            .fold(Ok(root.clone()), |root: Result<_>, p| {
                let root = root?;
                match created_menus.entry(p.clone()) {
                    Entry::Occupied(e) => Ok(e.get().clone()),
                    Entry::Vacant(e) => Ok(e
                        .insert(mc.add_submenu_for_group(group, p.clone(), Some(root))?)
                        .clone()),
                }
            })?;
        for cr in dep.crates.iter().chain(dep.repair_crate.iter()) {
            let title = if cr.required > 1 {
                String::from(format_compact!("{}({})", cr.name, cr.required))
            } else {
                cr.name.clone()
            };
            mc.add_command_for_group(
                group,
                title,
                Some(root.clone()),
                spawn_crate,
                ArgTuple {
                    fst: group,
                    snd: cr.name.clone(),
                },
            )?;
        }
    }
    Ok(())
}

fn add_ewr_menu_for_group(mc: &MissionCommands, group: GroupId) -> Result<()> {
    let root = mc.add_submenu_for_group(group, "EWR".into(), None)?;
    mc.add_command_for_group(
        group,
        "Report".into(),
        Some(root.clone()),
        ewr_report,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "toggle".into(),
        Some(root.clone()),
        toggle_ewr,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Friendly Report".into(),
        Some(root.clone()),
        friendly_ewr_report,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Units to Imperial".into(),
        Some(root.clone()),
        ewr_units_imperial,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Units to Metric".into(),
        Some(root.clone()),
        ewr_units_metric,
        group,
    )?;
    Ok(())
}

fn jtac_status(_: MizLua, gid: DbGid) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let side = ctx.db.group(&gid)?.side;
    let msg = ctx
        .jtac
        .jtac_status(&ctx.db, &gid)
        .context("getting jtac status")?;
    ctx.db.ephemeral.msgs().panel_to_side(10, false, side, msg);
    Ok(())
}

fn jtac_toggle_auto_laser(lua: MizLua, gid: DbGid) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .toggle_auto_laser(&ctx.db, lua, &gid)
            .context("toggling jtac auto laser")?;
    }
    jtac_status(lua, gid)
}

fn jtac_toggle_ir_pointer(lua: MizLua, gid: DbGid) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .toggle_ir_pointer(&ctx.db, lua, &gid)
            .context("toggling ir pointer")?
    }
    jtac_status(lua, gid)
}

fn jtac_smoke_target(lua: MizLua, gid: DbGid) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .smoke_target(lua, &gid)
            .context("smoking jtac target")?;
    }
    jtac_status(lua, gid)
}

fn jtac_shift(lua: MizLua, gid: DbGid) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .shift(&ctx.db, lua, &gid)
            .context("shifting jtac target")?;
    }
    jtac_status(lua, gid)
}

fn jtac_artillery_mission(lua: MizLua, arg: ArgTriple<DbGid, DbGid, u8>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let side = ctx.db.group(&arg.fst)?.side;
    match ctx
        .jtac
        .artillery_mission(lua, &ctx.db, &arg.fst, &arg.snd, arg.trd)
    {
        Ok(()) => ctx.db.ephemeral.msgs().panel_to_side(
            10,
            false,
            side,
            format!(
                "jtac {} artillery fire mission started for {}",
                arg.fst, arg.snd
            ),
        ),
        Err(e) => ctx.db.ephemeral.msgs().panel_to_side(
            10,
            false,
            side,
            format!("jtac {} could not start artillery mission {:?}", arg.fst, e),
        ),
    }
    Ok(())
}

fn jtac_adjust_solution(_lua: MizLua, arg: ArgTriple<DbGid, AdjustmentDir, u16>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let side = ctx.db.group(&arg.fst)?.side;
    ctx.jtac
        .adjust_artillery_solution(&arg.fst, arg.snd, arg.trd);
    let a = ctx.jtac.get_artillery_adjustment(&arg.fst);
    ctx.db.ephemeral.msgs().panel_to_side(
        10,
        false,
        side,
        format_compact!("artillery solution for {} adjusted now {:?}", arg.fst, a),
    );
    Ok(())
}

fn jtac_show_adjustment(_lua: MizLua, arg: DbGid) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let side = ctx.db.group(&arg)?.side;
    let a = ctx.jtac.get_artillery_adjustment(&arg);
    ctx.db.ephemeral.msgs().panel_to_side(
        10,
        false,
        side,
        format_compact!("adjustment for {} is {:?}", arg, a),
    );
    Ok(())
}

fn jtac_clear_filter(lua: MizLua, gid: DbGid) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    ctx.jtac
        .clear_filter(&ctx.db, lua, &gid)
        .context("clearing jtac target filter")?;
    Ok(())
}

fn jtac_filter(lua: MizLua, arg: ArgTuple<DbGid, u64>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let filter =
        BitFlags::<UnitTag>::from_bits(arg.snd).map_err(|_| anyhow!("invalid filter bits"))?;
    for tag in filter.iter() {
        ctx.jtac
            .add_filter(&ctx.db, lua, &arg.fst, tag)
            .context("setting jtac target filter")?;
    }
    Ok(())
}

fn jtac_set_code(lua: MizLua, arg: ArgTuple<DbGid, u16>) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .set_code_part(lua, &arg.fst, arg.snd)
            .context("setting jtac laser code")?;
    }
    jtac_status(lua, arg.fst)
}

pub fn remove_menu_for_jtac(lua: MizLua, side: Side, group: DbGid) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    mc.remove_submenu_for_coalition(
        side,
        CoalitionSubMenu::from(vec!["JTAC".into(), format_compact!("{group}").into()]),
    )
}

fn add_artillery_menu_for_jtac(
    lua: MizLua,
    mizgid: GroupId,
    root: GroupSubMenu,
    jtac: DbGid,
    arty: &[DbGid],
) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    let root = mc.add_submenu_for_group(mizgid, "Artillery".into(), Some(root.clone()))?;
    for gid in arty {
        let root =
            mc.add_submenu_for_group(mizgid, format_compact!("{gid}").into(), Some(root.clone()))?;
        let add_adjust = |root: &GroupSubMenu, dir: AdjustmentDir| -> Result<()> {
            mc.add_command_for_group(
                mizgid,
                "10m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgTriple {
                    fst: *gid,
                    snd: dir,
                    trd: 10,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "25m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgTriple {
                    fst: *gid,
                    snd: dir,
                    trd: 25,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "50m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgTriple {
                    fst: *gid,
                    snd: dir,
                    trd: 50,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "100m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgTriple {
                    fst: *gid,
                    snd: dir,
                    trd: 100,
                },
            )?;
            Ok(())
        };
        mc.add_command_for_group(
            mizgid,
            "Fire One".into(),
            Some(root.clone()),
            jtac_artillery_mission,
            ArgTriple {
                fst: jtac,
                snd: *gid,
                trd: 1,
            },
        )?;
        let for_effect =
            mc.add_submenu_for_group(mizgid, "Fire For Effect".into(), Some(root.clone()))?;
        mc.add_command_for_group(
            mizgid,
            "5".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgTriple {
                fst: jtac,
                snd: *gid,
                trd: 5,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "10".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgTriple {
                fst: jtac,
                snd: *gid,
                trd: 10,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "20".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgTriple {
                fst: jtac,
                snd: *gid,
                trd: 20,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "Show Adjustment".into(),
            Some(root.clone()),
            jtac_show_adjustment,
            *gid,
        )?;
        let short = mc.add_submenu_for_group(mizgid, "Report Short".into(), Some(root.clone()))?;
        add_adjust(&short, AdjustmentDir::Short)?;
        let long = mc.add_submenu_for_group(mizgid, "Report Long".into(), Some(root.clone()))?;
        add_adjust(&long, AdjustmentDir::Long)?;
        let left = mc.add_submenu_for_group(mizgid, "Report Left".into(), Some(root.clone()))?;
        add_adjust(&left, AdjustmentDir::Left)?;
        let right = mc.add_submenu_for_group(mizgid, "Report Right".into(), Some(root.clone()))?;
        add_adjust(&right, AdjustmentDir::Right)?;
    }
    Ok(())
}

pub fn add_menu_for_jtac(lua: MizLua, mizgid: GroupId, group: DbGid, arty: &[DbGid]) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    let root = GroupSubMenu::from(vec!["JTAC".into()]);
    mc.remove_submenu_for_group(
        mizgid,
        GroupSubMenu::from(vec!["JTAC".into(), format_compact!("{group}").into()]),
    )?;
    let root = mc.add_submenu_for_group(mizgid, format_compact!("{group}").into(), Some(root))?;
    mc.add_command_for_group(
        mizgid,
        "Status".into(),
        Some(root.clone()),
        jtac_status,
        group,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Toggle Auto Laser".into(),
        Some(root.clone()),
        jtac_toggle_auto_laser,
        group,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Toggle IR Pointer".into(),
        Some(root.clone()),
        jtac_toggle_ir_pointer,
        group,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Smoke Current Target".into(),
        Some(root.clone()),
        jtac_smoke_target,
        group,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Shift".into(),
        Some(root.clone()),
        jtac_shift,
        group,
    )?;
    let mut filter_root = mc.add_submenu_for_group(mizgid, "Filter".into(), Some(root.clone()))?;
    mc.add_command_for_group(
        mizgid,
        "Clear".into(),
        Some(filter_root.clone()),
        jtac_clear_filter,
        group,
    )?;
    for (i, tag) in UnitTag::all().iter().enumerate() {
        if (i + 1) % 9 == 0 {
            filter_root =
                mc.add_submenu_for_group(mizgid, "Next>>".into(), Some(filter_root.clone()))?;
        }
        mc.add_command_for_group(
            mizgid,
            format_compact!("{:?}", tag).into(),
            Some(filter_root.clone()),
            jtac_filter,
            ArgTuple {
                fst: group,
                snd: BitFlags::from(tag).bits(),
            },
        )?;
    }
    let code_root = mc.add_submenu_for_group(mizgid, "Code".into(), Some(root.clone()))?;
    let hundreds_root =
        mc.add_submenu_for_group(mizgid, "Hundreds".into(), Some(code_root.clone()))?;
    let tens_root = mc.add_submenu_for_group(mizgid, "Tens".into(), Some(code_root.clone()))?;
    let ones_root = mc.add_submenu_for_group(mizgid, "Ones".into(), Some(code_root.clone()))?;
    for (scale, root) in [(100, &hundreds_root), (10, &tens_root), (1, &ones_root)] {
        let range = if scale == 100 { 0..=6 } else { 0..=8 };
        for n in range {
            mc.add_command_for_group(
                mizgid,
                format_compact!("{n}").into(),
                Some(root.clone()),
                jtac_set_code,
                ArgTuple {
                    fst: group,
                    snd: n * scale,
                },
            )?;
        }
    }
    add_artillery_menu_for_jtac(lua, mizgid, root, group, arty)?;
    Ok(())
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

pub(super) fn update_jtac_menu(
    db: &Db,
    lua: MizLua,
    jtac: DbGid,
    arty: &[DbGid],
) -> Result<()> {
    for (_, player, _) in db.instanced_players() {
        if let Some((sl, _)) = player.current_slot.as_ref() {
            let si = db.info_for_slot(sl).context("getting slot")?;
            add_menu_for_jtac(lua, si.miz_gid, jtac, arty).context("adding menu")?
        }
    }
    Ok(())
}

pub(super) fn init_for_slot(ctx: &Context, lua: MizLua, slot: &SlotId) -> Result<()> {
    debug!("initializing menus");
    let cfg = &ctx.db.ephemeral.cfg;
    let mc = MissionCommands::singleton(lua)?;
    let add_jtac = |side, gid| -> Result<()> {
        let _ = mc.add_submenu_for_group(gid, "JTAC".into(), None)?;
        for (_, group, _) in ctx.db.jtacs() {
            if group.side == side {
                ctx.jtac.add_menu(lua, gid, &group.id)?
            }
        }
        Ok(())
    };
    match slot {
        SlotId::Spectator => Ok(()),
        SlotId::ArtilleryCommander(_, _)
        | SlotId::ForwardObserver(_, _)
        | SlotId::Instructor(_, _)
        | SlotId::Observer(_, _) => Ok(()),
        SlotId::Unit(_) | SlotId::MultiCrew(_, _) => {
            let si = ctx.db.info_for_slot(slot).context("getting slot info")?;
            let cap = CarryCap::from_typ(cfg, si.typ.as_str());
            mc.remove_submenu_for_group(si.miz_gid, vec!["Cargo".into()].into())?;
            if cap.crates {
                add_cargo_menu_for_group(cfg, &mc, &si.side, si.miz_gid)?
            }
            mc.remove_submenu_for_group(si.miz_gid, vec!["Troops".into()].into())?;
            if cap.troops {
                add_troops_menu_for_group(cfg, &mc, &si.side, si.miz_gid)?
            }
            mc.remove_submenu_for_group(si.miz_gid, vec!["EWR".into()].into())?;
            add_ewr_menu_for_group(&mc, si.miz_gid)?;
            mc.remove_submenu_for_group(si.miz_gid, vec!["JTAC".into()].into())?;
            add_jtac(si.side, si.miz_gid)
        }
    }
}
