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

use super::{ArgQuad, ArgTriple, ArgTuple};
use crate::{
    Context,
    db::{
        Db,
        actions::{ActionArgs, ActionCmd, WithJtac},
        group::DeployKind,
    },
    jtac::{JtId, Jtac, Jtacs},
    spawnctx::SpawnCtx,
};
use anyhow::{Context as ErrContext, Result, anyhow, bail};
use bfprotocols::{
    cfg::{ActionKind, UnitTag, Vehicle},
    db::{group::GroupId as DbGid, objective::ObjectiveId},
    perf::Perf,
};
use compact_str::format_compact;
use dcso3::{
    MizLua, String,
    coalition::Side,
    env::miz::GroupId,
    mission_commands::{GroupCommandItem, GroupSubMenu, MissionCommands},
    net::{SlotId, Ucid},
};
use enumflags2::{BitFlag, BitFlags};
use log::error;
use smallvec::{SmallVec, smallvec};
use std::sync::Arc;

pub fn jtac_status(_: MizLua, arg: ArgTuple<Option<Ucid>, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = ctx
        .jtac
        .get(&arg.snd)
        .with_context(|| format_compact!("get jtac {}", arg.snd))?;
    let msg = jtac
        .status(&ctx.db, ctx.jtac.location_by_code())
        .context("generate jtac status")?;
    match &arg.fst {
        None => ctx
            .db
            .ephemeral
            .msgs()
            .panel_to_side(10, false, jtac.side(), msg),
        Some(ucid) => ctx
            .db
            .ephemeral
            .panel_to_player(&ctx.db.persisted, 10, ucid, msg),
    }
    Ok(())
}

fn change_info(jtac: &Jtac, db: &Db, ucid: &Ucid) -> (String, String) {
    let near = db
        .objective(&jtac.location().oid)
        .map(|o| o.name.clone())
        .unwrap_or("unknown".into());
    let name = db
        .player(ucid)
        .map(|p| p.name.clone())
        .unwrap_or("unknown".into());
    (near, name)
}

fn get_jtac_mut<'a>(jtacs: &'a mut Jtacs, id: &JtId) -> Result<&'a mut Jtac> {
    jtacs
        .get_mut(id)
        .with_context(|| format_compact!("get jtac {}", id))
}

fn get_jtac<'a>(jtacs: &'a Jtacs, id: &JtId) -> Result<&'a Jtac> {
    jtacs
        .get(id)
        .with_context(|| format_compact!("get jtac {}", id))
}

fn jtac_msg_auto_shift(db: &mut Db, jtid: JtId, jtac: &Jtac, ucid: &Ucid) {
    let (near, name) = change_info(jtac, db, ucid);
    let msg = format_compact!(
        "AUTO SHIFT NOW {}\nfor jtac {} near {}\nchanged by {}",
        jtac.autoshift(),
        jtid,
        near,
        name
    );
    db.ephemeral
        .msgs()
        .panel_to_side(10, false, jtac.side(), msg);
}

pub fn jtac_toggle_auto_shift(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    jtac.toggle_auto_shift(&ctx.db, lua)
        .with_context(|| format_compact!("toggle auto shift {}", arg.snd))?;
    jtac_msg_auto_shift(&mut ctx.db, arg.snd, jtac, &arg.fst);
    Ok(())
}

pub fn jtac_toggle_ir_pointer(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    jtac.toggle_ir_pointer(&ctx.db, lua)
        .context("toggling ir pointer")?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.fst);
    let msg = format_compact!(
        "IR POINTER NOW {}\nfor jtac {} near {}\nchanged by {}",
        jtac.ir_pointer(),
        arg.snd,
        near,
        name
    );
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_side(10, false, jtac.side(), msg);
    Ok(())
}

pub fn jtac_smoke_target(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.fst);
    let msg = match jtac.smoke_target(lua).context("smoking jtac target") {
        Err(e) => format_compact!("COULD NOT SMOKE TARGET\njtac {}\n{e:?}", arg.snd),
        Ok(()) => format_compact!(
            "SMOKE DEPLOYED ON TARGET\njtac {} near {}\nrequested by {}",
            arg.snd,
            near,
            name
        ),
    };
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_side(10, false, jtac.side(), msg);
    Ok(())
}

fn jtac_msg_shift(db: &mut Db, jtid: JtId, jtac: &Jtac, ucid: &Ucid) {
    let (near, name) = change_info(jtac, db, ucid);
    let target = jtac
        .target()
        .as_ref()
        .map(|t| t.typ.clone())
        .unwrap_or("no target".into());
    let msg = format_compact!(
        "JTAC SHIFTED NOW TARGETING {}\nauto shift is now disabled\njtac {} near {}\nrequested by {}",
        target,
        jtid,
        near,
        name
    );
    db.ephemeral
        .msgs()
        .panel_to_side(10, false, jtac.side(), msg);
}

pub fn jtac_shift(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    jtac.shift(&ctx.db, lua).context("shifting jtac target")?;
    jtac_msg_shift(&mut ctx.db, arg.snd, jtac, &arg.fst);
    Ok(())
}

pub fn jtac_artillery_mission(lua: MizLua, arg: ArgQuad<JtId, DbGid, u8, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    match ctx
        .jtac
        .artillery_mission(&ctx.db, lua, &arg.fst, &arg.snd, arg.trd)
    {
        Ok(()) => {
            let jtac = get_jtac(&ctx.jtac, &arg.fst).context("getting jtac")?;
            let (near, name) = change_info(jtac, &ctx.db, &arg.fth);
            let msg = format_compact!(
                "ARTILLERY MISSION STARTED for {}\ndirected by jtac {} near {}\nrequested by {}",
                arg.snd,
                arg.fst,
                near,
                name
            );
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_side(10, false, jtac.side(), msg)
        }
        Err(e) => {
            let msg = format!("jtac {} could not start artillery mission {:?}", arg.fst, e);
            ctx.db
                .ephemeral
                .panel_to_player(&ctx.db.persisted, 10, &arg.fth, msg);
        }
    }
    Ok(())
}

pub fn jtac_alcm_mission(lua: MizLua, arg: ArgQuad<JtId, DbGid, Vec<u8>, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    match ctx
        .jtac
        .alcm_mission(&ctx.db, lua, &arg.fst, &arg.snd, arg.trd)
    {
        Ok(()) => {
            let jtac = get_jtac(&ctx.jtac, &arg.fst).context("getting jtac")?;
            let (near, name) = change_info(jtac, &ctx.db, &arg.fth);
            let msg = format_compact!(
                "ALCM MISSION STARTED for {}\ndirected by jtac {} near {}\nrequested by {}",
                arg.snd,
                arg.fst,
                near,
                name
            );
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_side(10, false, jtac.side(), msg)
        }
        Err(e) => {
            let msg = format!("jtac {} could not start ALCM mission {:?}", arg.fst, e);
            ctx.db
                .ephemeral
                .panel_to_player(&ctx.db.persisted, 10, &arg.fth, msg);
        }
    }
    Ok(())
}

fn jtac_relay_target(lua: MizLua, arg: ArgTriple<JtId, DbGid, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.fst)?;
    match jtac.relay_target(&ctx.db, lua, &arg.snd) {
        Ok(()) => {
            let (near, name) = change_info(jtac, &ctx.db, &arg.trd);
            let msg = format_compact!(
                "TARGET RELAYED to {}\ndirected by jtac {} near {}\nrequested by {}",
                arg.snd,
                arg.fst,
                near,
                name
            );
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_side(10, false, jtac.side(), msg)
        }
        Err(e) => {
            let msg = format!("jtac {} could not relay target {:?}", arg.fst, e);
            ctx.db
                .ephemeral
                .panel_to_player(&ctx.db.persisted, 10, &arg.trd, msg);
        }
    }
    Ok(())
}

fn jtac_clear_filter(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    jtac.clear_filter(&ctx.db, lua)
        .context("clearing jtac target filter")?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.fst);
    let msg = format_compact!(
        "JTAC FILTER CLEARED\njtac {} near {}\ncleared by {}",
        arg.snd,
        near,
        name
    );
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_side(10, false, jtac.side(), msg);
    Ok(())
}

fn jtac_filter(lua: MizLua, arg: ArgTriple<JtId, u64, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let filter =
        BitFlags::<UnitTag>::from_bits(arg.snd).map_err(|_| anyhow!("invalid filter bits"))?;
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.fst)?;
    jtac.add_filter(&ctx.db, lua, filter)
        .context("setting jtac target filter")?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.trd);
    let msg = format_compact!(
        "JTAC FILTER CHANGED TO {}\njtac {} near {}\nchanged by {}",
        jtac.filter(),
        arg.fst,
        near,
        name
    );
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_side(10, false, jtac.side(), msg);
    Ok(())
}

pub fn jtac_set_code(lua: MizLua, arg: ArgTriple<JtId, u16, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    ctx.jtac
        .set_code_part(lua, &arg.fst, arg.snd)
        .context("setting jtac laser code")?;
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.fst)?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.trd);
    let msg = format_compact!(
        "JTAC CODE CHANGED TO {}\njtac {} near {}\nchanged by {}",
        jtac.code(),
        arg.fst,
        near,
        name
    );
    ctx.db
        .ephemeral
        .msgs()
        .panel_to_side(10, false, jtac.side(), msg);
    Ok(())
}

fn add_artillery_menu_for_jtac(
    lua: MizLua,
    mizgid: GroupId,
    ucid: Ucid,
    root: GroupSubMenu,
    jtac: JtId,
    arty: &[DbGid],
) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    let root = mc.add_submenu_for_group(mizgid, "Artillery".into(), Some(root.clone()))?;
    for gid in arty {
        let root =
            mc.add_submenu_for_group(mizgid, format_compact!("{gid}").into(), Some(root.clone()))?;
        mc.add_command_for_group(
            mizgid,
            "Relay Target".into(),
            Some(root.clone()),
            jtac_relay_target,
            ArgTriple {
                fst: jtac,
                snd: *gid,
                trd: ucid,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "Fire One".into(),
            Some(root.clone()),
            jtac_artillery_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 1,
                fth: ucid,
            },
        )?;
        let for_effect =
            mc.add_submenu_for_group(mizgid, "Fire For Effect".into(), Some(root.clone()))?;
        mc.add_command_for_group(
            mizgid,
            "5".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 5,
                fth: ucid,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "10".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 10,
                fth: ucid,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "20".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 20,
                fth: ucid,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "40".into(),
            Some(for_effect.clone()),
            jtac_artillery_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 40,
                fth: ucid,
            },
        )?;
    }
    Ok(())
}

fn add_alcm_menu_for_jtac(
    lua: MizLua,
    mizgid: GroupId,
    ucid: Ucid,
    root: GroupSubMenu,
    jtac: JtId,
    alcm: &[DbGid],
) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;

    let root = mc.add_submenu_for_group(mizgid, "ALCM".into(), Some(root.clone()))?;
    for gid in alcm {
        let root =
            mc.add_submenu_for_group(mizgid, format_compact!("{gid}").into(), Some(root.clone()))?;

        let quarter =
            mc.add_submenu_for_group(mizgid, "Fire Quarter".into(), Some(root.clone()))?;
        let half = mc.add_submenu_for_group(mizgid, "Fire Half".into(), Some(root.clone()))?;
        let all = mc.add_submenu_for_group(mizgid, "Fire All".into(), Some(root.clone()))?;

        for (submenu, n) in vec![(quarter, 1), (half, 2), (all, 4)] {
            mc.add_command_for_group(
                mizgid,
                "Fire One per target".into(),
                Some(submenu.clone()),
                jtac_alcm_mission,
                ArgQuad {
                    fst: jtac,
                    snd: *gid,
                    trd: vec![n, 1],
                    fth: ucid,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "Fire Two per target".into(),
                Some(submenu.clone()),
                jtac_alcm_mission,
                ArgQuad {
                    fst: jtac,
                    snd: *gid,
                    trd: vec![n, 2],
                    fth: ucid,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "Fire Four per target".into(),
                Some(submenu.clone()),
                jtac_alcm_mission,
                ArgQuad {
                    fst: jtac,
                    snd: *gid,
                    trd: vec![n, 4],
                    fth: ucid,
                },
            )?;
        }
    }

    Ok(())
}

pub fn call_bomber(lua: MizLua, arg: ArgTriple<JtId, Ucid, String>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let perf = Arc::make_mut(&mut unsafe { Perf::get_mut() }.inner);
    let spctx = SpawnCtx::new(lua)?;
    let jtac = get_jtac(&ctx.jtac, &arg.fst)?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.snd);
    let action = ctx
        .db
        .ephemeral
        .cfg
        .actions
        .get(&jtac.side())
        .and_then(|acts| acts.get(&arg.trd))
        .ok_or_else(|| anyhow!("no such action {}", arg.trd))?;
    let cfg = match &action.kind {
        ActionKind::Bomber(cfg) => cfg.clone(),
        _ => bail!("not a bomber action"),
    };
    match ctx.db.start_action(
        lua,
        perf,
        &spctx,
        &ctx.idx,
        &ctx.jtac,
        jtac.side(),
        Some(arg.snd.clone()),
        ActionCmd {
            name: arg.trd,
            action: action.clone(),
            args: ActionArgs::Bomber(WithJtac { jtac: arg.fst, cfg }),
        },
    ) {
        Ok(()) => {
            let msg = format_compact!(
                "BOMBER MISSION STARTED\ntargeting by jtac {} near {}\nstarted by {}",
                arg.fst,
                near,
                name
            );
            ctx.db
                .ephemeral
                .msgs()
                .panel_to_side(10, false, jtac.side(), msg)
        }
        Err(e) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            10,
            &arg.snd,
            format_compact!("bomber mission could not start {e:?}"),
        ),
    }
    Ok(())
}

fn toggle_pin_jtac(lua: MizLua, arg: ArgTuple<SlotId, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let subd = ctx.subscribed_jtac_menus.entry(arg.fst).or_default();
    if subd.pinned.contains(&arg.snd) {
        subd.pinned.remove(&arg.snd);
    } else {
        subd.pinned.insert(arg.snd);
    }
    init_jtac_menu_for_slot(ctx, lua, &arg.fst)
}

pub(super) fn add_menu_for_jtac(
    db: &Db,
    side: Side,
    root: GroupSubMenu,
    lua: MizLua,
    mizgid: GroupId,
    jtac: &Jtac,
    ucid: &Ucid,
    slot: SlotId,
) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let mc = MissionCommands::singleton(lua)?;
    let pinned = ctx
        .subscribed_jtac_menus
        .get(&slot)
        .map(|subd| subd.pinned.contains(&jtac.gid()))
        .unwrap_or(false);
    let name = match jtac.gid() {
        JtId::Group(gid) => match db.group(&gid) {
            Err(_) => format_compact!("{gid}"),
            Ok(group) => match &group.origin {
                DeployKind::Action { name, .. } => format_compact!("{gid}({name})"),
                DeployKind::Deployed { player, spec, .. } => match db.player(player) {
                    Some(player) => {
                        format_compact!("{gid}({} {})", spec.path.last().unwrap(), player.name)
                    }
                    None => format_compact!("{gid}({})", spec.path.last().unwrap()),
                },
                DeployKind::Troop { player, spec, .. } => match db.player(player) {
                    Some(player) => format_compact!("{gid}({} {})", spec.name, player.name),
                    None => format_compact!("{gid}({})", spec.name),
                },
                DeployKind::Objective { .. }
                | DeployKind::ObjectiveDeprecated
                | DeployKind::Crate { .. } => format_compact!("{gid}"),
            },
        },
        JtId::Slot(sl) => {
            let name = match db.ephemeral.player_in_slot(&sl) {
                None => String::from(""),
                Some(ucid) => match db.player(ucid) {
                    None => String::from(""),
                    Some(p) => p.name.clone(),
                },
            };
            let typ = db
                .ephemeral
                .get_slot_info(&sl)
                .map(|ifo| ifo.typ.clone())
                .unwrap_or_else(|| Vehicle::from(""));
            format_compact!("sl{sl}({typ} {name})")
        }
    };
    let root = mc.add_submenu_for_group(mizgid, name.clone().into(), Some(root))?;
    mc.add_command_for_group(
        mizgid,
        if !pinned {
            "Pin".into()
        } else {
            "Unpin".into()
        },
        Some(root.clone()),
        toggle_pin_jtac,
        ArgTuple {
            fst: slot,
            snd: jtac.gid(),
        },
    )?;
    mc.add_command_for_group(
        mizgid,
        "Status".into(),
        Some(root.clone()),
        jtac_status,
        ArgTuple {
            fst: Some(*ucid),
            snd: jtac.gid(),
        },
    )?;
    mc.add_command_for_group(
        mizgid,
        "Toggle Auto Shift".into(),
        Some(root.clone()),
        jtac_toggle_auto_shift,
        ArgTuple {
            fst: *ucid,
            snd: jtac.gid(),
        },
    )?;
    mc.add_command_for_group(
        mizgid,
        "Toggle IR Pointer".into(),
        Some(root.clone()),
        jtac_toggle_ir_pointer,
        ArgTuple {
            fst: *ucid,
            snd: jtac.gid(),
        },
    )?;
    mc.add_command_for_group(
        mizgid,
        "Smoke Current Target".into(),
        Some(root.clone()),
        jtac_smoke_target,
        ArgTuple {
            fst: *ucid,
            snd: jtac.gid(),
        },
    )?;
    mc.add_command_for_group(
        mizgid,
        "Shift".into(),
        Some(root.clone()),
        jtac_shift,
        ArgTuple {
            fst: *ucid,
            snd: jtac.gid(),
        },
    )?;
    let mut filter_root = mc.add_submenu_for_group(mizgid, "Filter".into(), Some(root.clone()))?;
    mc.add_command_for_group(
        mizgid,
        "Clear".into(),
        Some(filter_root.clone()),
        jtac_clear_filter,
        ArgTuple {
            fst: *ucid,
            snd: jtac.gid(),
        },
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
            ArgTriple {
                fst: jtac.gid(),
                snd: BitFlags::from(tag).bits(),
                trd: *ucid,
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
                ArgTriple {
                    fst: jtac.gid(),
                    snd: n * scale,
                    trd: *ucid,
                },
            )?;
        }
    }
    add_artillery_menu_for_jtac(
        lua,
        mizgid,
        *ucid,
        root.clone(),
        jtac.gid(),
        jtac.nearby_artillery(),
    )?;

    add_alcm_menu_for_jtac(
        lua,
        mizgid,
        *ucid,
        root.clone(),
        jtac.gid(),
        jtac.nearby_alcm(),
    )?;

    let bomber_missions = db.ephemeral.cfg.actions.get(&side);
    let bomber_missions = bomber_missions.iter().flat_map(|acts| {
        acts.iter().filter_map(|(n, a)| match a.kind {
            ActionKind::Bomber(_) => Some(n.clone()),
            _ => None,
        })
    });
    for name in bomber_missions {
        let root = mc.add_submenu_for_group(
            mizgid,
            format_compact!("Bomber Mission({name})").into(),
            Some(root.clone()),
        )?;
        mc.add_command_for_group(
            mizgid,
            "Yes, do it!".into(),
            Some(root.clone()),
            call_bomber,
            ArgTriple {
                fst: jtac.gid(),
                snd: ucid.clone(),
                trd: name,
            },
        )?;
    }
    Ok(())
}

fn add_jtacs_by_location(
    lua: MizLua,
    arg: ArgQuad<Ucid, GroupId, ObjectiveId, GroupSubMenu>,
) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let player = ctx
        .db
        .player(&arg.fst)
        .ok_or_else(|| anyhow!("missing player"))?;
    if let Some((slot, _)) = &player.current_slot {
        let slot = *slot;
        ctx.subscribed_jtac_menus
            .entry(slot)
            .or_default()
            .subscribed_objectives
            .insert(arg.trd);
        let mc = MissionCommands::singleton(lua)?;
        let name = ctx.db.objective(&arg.trd)?.name.clone();
        let mut cmd: Vec<String> = arg.fth.clone().into();
        cmd.push(format_compact!("{name}>>").into());
        mc.remove_command_for_group(arg.snd, cmd.into())?;
        let mut root = mc.add_submenu_for_group(arg.snd, name, Some(arg.fth))?;
        let mut n = 0;
        for jtac in ctx.jtac.jtacs() {
            if jtac.side() == player.side && jtac.location().oid == arg.trd {
                if n >= 8 {
                    root =
                        mc.add_submenu_for_group(arg.snd, "NEXT>>".into(), Some(root.clone()))?;
                    n = 0;
                }
                add_menu_for_jtac(
                    &ctx.db,
                    player.side,
                    root.clone(),
                    lua,
                    arg.snd,
                    jtac,
                    &arg.fst,
                    slot,
                )?;
                n += 1
            }
        }
    }
    Ok(())
}

fn jtac_refresh_locations(lua: MizLua, arg: Ucid) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let player = ctx
        .db
        .player(&arg)
        .ok_or_else(|| anyhow!("missing player"))?;
    if let Some((slot, _)) = player.current_slot.as_ref() {
        let slot = *slot;
        init_jtac_menu_for_slot(ctx, lua, &slot)?
    }
    Ok(())
}

fn add_jtac_locations(lua: MizLua, arg: ArgTriple<Ucid, GroupId, SlotId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let slot = arg.trd;
    let mc = MissionCommands::singleton(lua)?;
    let player = ctx
        .db
        .player(&arg.fst)
        .ok_or_else(|| anyhow!("missing player"))?;
    let mut roots: SmallVec<[String; 16]> = smallvec![];
    mc.remove_command_for_group(arg.snd, vec!["JTAC>>".into()].into())?;
    let mut root = mc.add_submenu_for_group(arg.snd, "JTAC".into(), None)?;
    mc.add_command_for_group(
        arg.snd,
        "Refresh Locations".into(),
        Some(root.clone()),
        jtac_refresh_locations,
        arg.fst,
    )?;
    let mut n = 0;
    macro_rules! handle_submenu {
        () => {
            if n >= 8 {
                root = mc.add_submenu_for_group(arg.snd, "NEXT>>".into(), Some(root.clone()))?;
                n = 0;
            }
            n += 1;
        };
    }
    struct JtEntry<'a> {
        jtac: &'a Jtac,
        near: String,
        oid: ObjectiveId,
        pinned: bool,
    }
    let mut jtacs: SmallVec<[JtEntry; 64]> = smallvec![];
    jtacs.extend(ctx.jtac.jtacs().filter_map(|jtac| {
        if jtac.side() != player.side {
            return None;
        }
        let (near, oid) = match ctx
            .db
            .objective(&jtac.location().oid)
            .map(|o| (o.name.clone(), o.id))
            .ok()
        {
            Some((near, oid)) => (near, oid),
            None => {
                error!("jtac near missing objective {:?}", jtac.location());
                return None;
            }
        };
        let pinned = ctx
            .subscribed_jtac_menus
            .get(&slot)
            .map(|subd| subd.pinned.contains(&jtac.gid()))
            .unwrap_or(false);
        Some(JtEntry {
            jtac,
            near,
            oid,
            pinned,
        })
    }));
    jtacs.sort_by(|jte0, jte1| {
        use std::cmp::Ordering;
        match jte1.pinned.cmp(&jte0.pinned) {
            Ordering::Equal => jte0.near.cmp(&jte1.near),
            o => o,
        }
    });
    for jte in jtacs {
        if jte.pinned {
            handle_submenu!();
            add_menu_for_jtac(
                &ctx.db,
                player.side,
                root.clone(),
                lua,
                arg.snd,
                jte.jtac,
                &arg.fst,
                slot,
            )?;
        } else if !roots.contains(&jte.near) {
            roots.push(jte.near.clone());
            handle_submenu!();
            mc.add_command_for_group(
                arg.snd,
                format_compact!("{}>>", jte.near).into(),
                Some(root.clone()),
                add_jtacs_by_location,
                ArgQuad {
                    fst: arg.fst,
                    snd: arg.snd,
                    trd: jte.oid,
                    fth: root.clone(),
                },
            )?;
        }
    }
    Ok(())
}

pub(crate) fn init_jtac_menu_for_slot(ctx: &mut Context, lua: MizLua, slot: &SlotId) -> Result<()> {
    let ucid = match ctx.db.ephemeral.player_in_slot(slot) {
        Some(ucid) => ucid,
        None => return Ok(()),
    };
    let si = ctx
        .db
        .ephemeral
        .get_slot_info(slot)
        .context("getting slot info")?;
    let mc = MissionCommands::singleton(lua)?;
    mc.remove_command_for_group(si.miz_gid, GroupCommandItem::from(vec!["JTAC>>".into()]))?;
    mc.remove_submenu_for_group(si.miz_gid, GroupSubMenu::from(vec!["JTAC".into()]))?;
    mc.add_command_for_group(
        si.miz_gid,
        "JTAC>>".into(),
        None,
        add_jtac_locations,
        ArgTriple {
            fst: *ucid,
            snd: si.miz_gid,
            trd: *slot,
        },
    )?;
    Ok(())
}
