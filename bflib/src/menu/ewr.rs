use super::slot_for_group;
use crate::{
    ewr::{self, EwrUnits},
    Context,
};
use anyhow::{Context as ErrContext, Result};
use chrono::prelude::*;
use compact_str::format_compact;
use dcso3::{env::miz::GroupId, mission_commands::MissionCommands, MizLua};
use std::fmt::Write;

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

pub(super) fn add_ewr_menu_for_group(mc: &MissionCommands, group: GroupId) -> Result<()> {
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
