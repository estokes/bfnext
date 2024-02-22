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

use super::{ArgTriple, ArgTuple};
use crate::{
    cfg::{ActionKind, UnitTag},
    db::{
        actions::{ActionArgs, ActionCmd, WithJtac},
        group::GroupId as DbGid,
        Db,
    },
    jtac::{AdjustmentDir, JtId, Jtac},
    spawnctx::SpawnCtx,
    Context,
};
use anyhow::{anyhow, bail, Context as ErrContext, Result};
use compact_str::format_compact;
use dcso3::{
    coalition::Side,
    env::miz::GroupId,
    mission_commands::{GroupSubMenu, MissionCommands},
    net::Ucid,
    MizLua, String,
};
use enumflags2::{BitFlag, BitFlags};

fn jtac_status(_: MizLua, gid: JtId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let side = ctx.jtac.get(&gid)?.side;
    let msg = ctx
        .jtac
        .jtac_status(&ctx.db, &gid)
        .context("getting jtac status")?;
    ctx.db.ephemeral.msgs().panel_to_side(10, false, side, msg);
    Ok(())
}

fn jtac_toggle_auto_shift(lua: MizLua, gid: JtId) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .toggle_auto_shift(&ctx.db, lua, &gid)
            .context("toggling jtac auto laser")?;
    }
    jtac_status(lua, gid)
}

fn jtac_toggle_ir_pointer(lua: MizLua, gid: JtId) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .toggle_ir_pointer(&ctx.db, lua, &gid)
            .context("toggling ir pointer")?
    }
    jtac_status(lua, gid)
}

fn jtac_smoke_target(lua: MizLua, gid: JtId) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .smoke_target(lua, &gid)
            .context("smoking jtac target")?;
    }
    jtac_status(lua, gid)
}

fn jtac_shift(lua: MizLua, gid: JtId) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .shift(&ctx.db, lua, &gid)
            .context("shifting jtac target")?;
    }
    jtac_status(lua, gid)
}

fn jtac_artillery_mission(lua: MizLua, arg: ArgTriple<JtId, DbGid, u8>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let side = ctx.jtac.get(&arg.fst)?.side;
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

fn jtac_clear_filter(lua: MizLua, gid: JtId) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    ctx.jtac
        .clear_filter(&ctx.db, lua, &gid)
        .context("clearing jtac target filter")?;
    Ok(())
}

fn jtac_filter(lua: MizLua, arg: ArgTuple<JtId, u64>) -> Result<()> {
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

fn jtac_set_code(lua: MizLua, arg: ArgTuple<JtId, u16>) -> Result<()> {
    {
        let ctx = unsafe { Context::get_mut() };
        ctx.jtac
            .set_code_part(lua, &arg.fst, arg.snd)
            .context("setting jtac laser code")?;
    }
    jtac_status(lua, arg.fst)
}

fn add_artillery_menu_for_jtac(
    lua: MizLua,
    mizgid: GroupId,
    root: GroupSubMenu,
    jtac: JtId,
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
            "40".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgTriple {
                fst: jtac,
                snd: *gid,
                trd: 40,
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

fn call_bomber(lua: MizLua, arg: ArgTriple<JtId, Ucid, String>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let spctx = SpawnCtx::new(lua)?;
    let side = ctx.jtac.get(&arg.fst)?.side;
    let name = ctx
        .db
        .player(&arg.snd)
        .ok_or_else(|| anyhow!("no such player"))?
        .name
        .clone();
    let action = ctx
        .db
        .ephemeral
        .cfg
        .actions
        .get(&side)
        .and_then(|acts| acts.get(&arg.trd))
        .ok_or_else(|| anyhow!("no such action {}", arg.trd))?;
    let cfg = match &action.kind {
        ActionKind::Bomber(cfg) => cfg.clone(),
        _ => bail!("not a bomber action"),
    };
    match ctx.db.start_action(
        &spctx,
        &ctx.idx,
        &ctx.jtac,
        side,
        Some(arg.snd.clone()),
        ActionCmd {
            name: arg.trd,
            action: action.clone(),
            args: ActionArgs::Bomber(WithJtac { jtac: arg.fst, cfg }),
        },
    ) {
        Ok(()) => ctx.db.ephemeral.msgs().panel_to_side(
            15,
            false,
            side,
            format_compact!(
                "Bomber mission started by {name} targeting by jtac {}",
                arg.fst
            ),
        ),
        Err(e) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            &arg.snd,
            format_compact!("bomber mission could not start {e:?}"),
        ),
    }
    Ok(())
}

pub(super) fn add_menu_for_jtac(
    db: &Db,
    side: Side,
    root: GroupSubMenu,
    lua: MizLua,
    mizgid: GroupId,
    jtac: &Jtac,
    ucid: &Ucid,
) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    let root =
        mc.add_submenu_for_group(mizgid, format_compact!("{}", jtac.gid).into(), Some(root))?;
    mc.add_command_for_group(
        mizgid,
        "Status".into(),
        Some(root.clone()),
        jtac_status,
        jtac.gid,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Toggle Auto Shift".into(),
        Some(root.clone()),
        jtac_toggle_auto_shift,
        jtac.gid,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Toggle IR Pointer".into(),
        Some(root.clone()),
        jtac_toggle_ir_pointer,
        jtac.gid,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Smoke Current Target".into(),
        Some(root.clone()),
        jtac_smoke_target,
        jtac.gid,
    )?;
    mc.add_command_for_group(
        mizgid,
        "Shift".into(),
        Some(root.clone()),
        jtac_shift,
        jtac.gid,
    )?;
    let mut filter_root = mc.add_submenu_for_group(mizgid, "Filter".into(), Some(root.clone()))?;
    mc.add_command_for_group(
        mizgid,
        "Clear".into(),
        Some(filter_root.clone()),
        jtac_clear_filter,
        jtac.gid,
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
                fst: jtac.gid,
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
                    fst: jtac.gid,
                    snd: n * scale,
                },
            )?;
        }
    }
    add_artillery_menu_for_jtac(lua, mizgid, root.clone(), jtac.gid, &jtac.nearby_artillery)?;
    let bomber_missions = db.ephemeral.cfg.actions.get(&side);
    let bomber_missions = bomber_missions.iter().flat_map(|acts| {
        acts.iter().filter_map(|(n, a)| match a.kind {
            ActionKind::Bomber(_) => Some(n.clone()),
            _ => None,
        })
    });
    for name in bomber_missions {
        mc.add_command_for_group(
            mizgid,
            "Bomber Mission".into(),
            Some(root.clone()),
            call_bomber,
            ArgTriple {
                fst: jtac.gid,
                snd: ucid.clone(),
                trd: name,
            },
        )?;
    }
    Ok(())
}
