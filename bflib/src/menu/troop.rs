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

use super::{cargo, player_name, slot_for_group, ArgTuple};
use crate::{
    cfg::{Cfg, LimitEnforceTyp},
    Context,
};
use anyhow::{Context as ErrContext, Result};
use compact_str::format_compact;
use dcso3::{
    coalition::Side, env::miz::GroupId, mission_commands::MissionCommands, MizLua, String,
};

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

pub(super) fn add_troops_menu_for_group(
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
            cargo::list_current_cargo,
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
