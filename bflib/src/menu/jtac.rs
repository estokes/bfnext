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
    cfg::{Action, ActionKind, UnitTag},
    db::{
        self,
        actions::{self, ActionArgs, ActionCmd, WithJtac, WithPosAndGroup},
        group::GroupId as DbGid,
        objective::ObjectiveId,
        Db,
    },
    jtac::{AdjustmentDir, JtId, Jtac, Jtacs},
    spawnctx::SpawnCtx,
    Context,
};
use anyhow::{anyhow, bail, Context as ErrContext, Result};
use compact_str::format_compact;
use dcso3::{
    coalition::Side,
    env::miz::GroupId,
    mission_commands::{GroupCommandItem, GroupSubMenu, MissionCommands},
    net::{SlotId, Ucid},
    object::DcsObject,
    unit::Unit,
    LuaEnv, MizLua, String,
};
use enumflags2::{BitFlag, BitFlags};
use log::{error, info};
use mlua::LuaSerdeExt;
use smallvec::{smallvec, SmallVec};

fn jtac_status(_: MizLua, arg: ArgTuple<Option<Ucid>, JtId>) -> Result<()> {
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

fn jtac_toggle_auto_shift(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    jtac.toggle_auto_shift(&ctx.db, lua)
        .with_context(|| format_compact!("toggle auto shift {}", arg.snd))?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.fst);
    let msg = format_compact!(
        "AUTO SHIFT NOW {}\nfor jtac {} near {}\nchanged by {}",
        jtac.autoshift(),
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

fn jtac_toggle_ir_pointer(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
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

fn jtac_smoke_target(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    jtac.smoke_target(lua).context("smoking jtac target")?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.fst);
    let msg = format_compact!(
        "SMOKE DEPLOYED ON TARGET\njtac {} near {}\nrequested by {}",
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

fn jtac_shift(lua: MizLua, arg: ArgTuple<Ucid, JtId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.snd)?;
    jtac.shift(&ctx.db, lua).context("shifting jtac target")?;
    let (near, name) = change_info(jtac, &ctx.db, &arg.fst);
    let target = jtac
        .target()
        .as_ref()
        .and_then(|t| ctx.db.unit(&t.uid).ok())
        .map(|u| u.typ.clone())
        .unwrap_or("no target".into());
    let msg = format_compact!(
        "JTAC SHIFTED NOW TARGETING {}\nauto shift is now disabled\njtac {} near {}\nrequested by {}",
        target,
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

fn jtac_cruise_missile_get_ammo(lua: MizLua, arg: ArgTriple<Ucid, DbGid, bool>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };

    let puid = ctx
        .db
        .player(&arg.fst)
        .unwrap()
        .current_slot
        .clone()
        .unwrap()
        .0
        .as_unit_id()
        .unwrap();

    let aircraft = ctx.db.unit(
        ctx.db
            .group(&arg.snd)?
            .units
            .into_iter()
            .next()
            .ok_or(anyhow!("no unit!"))?,
    )?;

    let oid = ctx
        .db
        .ephemeral
        .get_object_id_by_uid(&aircraft.id)
        .ok_or(anyhow!("no object with id"))?;

    let unit = dcso3::unit::Unit::get_instance(lua, oid).context("getting unit")?;

    let ar = unit.get_ammo()?;

    if ar.len() <= 0 {
        unit.as_object()?.destroy()?;
        bail!("no more weapons!")
    };

    let ammo_state = ar.into_iter().next().unwrap()?.count()? as i64;

    let msg = format_compact!(
        "{} {} | Missiles Remaining: {}/12",
        aircraft.typ,
        aircraft.id,
        ammo_state
    );
    ctx.db.ephemeral.msgs().panel_to_unit(10, false, puid, msg);
    Ok(())
}

fn jtac_cruise_missile_get_ammo_return_i64(
    lua: MizLua,
    arg: ArgTriple<Ucid, DbGid, bool>,
) -> Result<i64> {
    let ctx = unsafe { Context::get_mut() };

    let puid = ctx
        .db
        .player(&arg.fst)
        .unwrap()
        .current_slot
        .clone()
        .unwrap()
        .0
        .as_unit_id()
        .unwrap();

    let aircraft = ctx.db.unit(
        ctx.db
            .group(&arg.snd)?
            .units
            .into_iter()
            .next()
            .ok_or(anyhow!("no unit!"))?,
    )?;

    let oid = ctx
        .db
        .ephemeral
        .get_object_id_by_uid(&aircraft.id)
        .ok_or(anyhow!("no object with id"))?;

    let unit = dcso3::unit::Unit::get_instance(lua, oid).context("getting unit")?;

    let ar = unit.get_ammo()?;

    if ar.len() <= 0 {
        unit.as_object()?.destroy()?;
        bail!("no more weapons!")
    };

    let ammo_state = ar.into_iter().next().unwrap()?.count()? as i64;

    if arg.trd {
        let msg = format_compact!(
            "{} {} | Missiles Remaining: {}/12",
            aircraft.typ,
            aircraft.id,
            ammo_state
        );
        ctx.db.ephemeral.msgs().panel_to_unit(10, false, puid, msg);
    }
    Ok(ammo_state)
}

fn jtac_cruise_missile_mission(lua: MizLua, arg: ArgQuad<JtId, DbGid, u8, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let db = &ctx.db;
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.fst)?;
    let mut min_dist: Option<f64> = None;
    let quant = arg.trd as i64;
    let mut final_gid: Option<db::group::GroupId> = None;

    for gid in &db.persisted.actions {
        let aircraft = db.unit(
            db.group(gid)?
                .units
                .into_iter()
                .next()
                .ok_or(anyhow!("no unit!"))?,
        )?;
        let dist = na::distance(&aircraft.pos.into(), &jtac.location().pos.into());

        min_dist = match min_dist {
            Some(c) => {
                if dist < c {
                    final_gid = Some(*gid);
                    Some(dist)
                } else {
                    final_gid = Some(*gid);
                    Some(c)
                }
            }
            None => {
                final_gid = Some(*gid);
                Some(dist)
            }
        };
    }

    match min_dist {
        Some(_) => {
            call_cruise_missile_strike(
                lua,
                "cruise-missile".to_owned().into(),
                ArgQuad {
                    fst: arg.fst,
                    snd: arg.fth,
                    trd: quant,
                    fth: final_gid.ok_or(anyhow!("no bomber to use for mission!"))?,
                },
            )?;
            Ok(())
        }
        None => bail!("no bombers available!"),
    }
}

fn jtac_artillery_mission(lua: MizLua, arg: ArgQuad<JtId, DbGid, u8, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let adjustment = ctx.jtac.get_artillery_adjustment(&arg.snd);
    let jtac = get_jtac_mut(&mut ctx.jtac, &arg.fst)?;
    match jtac.artillery_mission(&ctx.db, lua, adjustment, &arg.snd, arg.trd) {
        Ok(()) => {
            let (near, name) = change_info(jtac, &ctx.db, &arg.fth);
            let msg =
                format_compact!(
                "ARTILLERY MISSION STARTED for {}\ndirected by jtac {} near {}\nrequested by {}",
                arg.snd, arg.fst, near, name
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

fn jtac_adjust_solution(_lua: MizLua, arg: ArgQuad<DbGid, AdjustmentDir, u16, Ucid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let side = ctx.db.group(&arg.fst)?.side;
    ctx.jtac
        .adjust_artillery_solution(&arg.fst, arg.snd, arg.trd);
    let a = ctx.jtac.get_artillery_adjustment(&arg.fst);
    let name = ctx
        .db
        .player(&arg.fth)
        .map(|p| p.name.clone())
        .unwrap_or("unknown".into());
    ctx.db.ephemeral.msgs().panel_to_side(
        10,
        false,
        side,
        format_compact!(
            "artillery solution for {} adjusted now {:?}\nrequested by {}",
            arg.fst,
            a,
            name
        ),
    );
    Ok(())
}

fn jtac_show_adjustment(_lua: MizLua, arg: ArgTuple<Ucid, DbGid>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let a = ctx.jtac.get_artillery_adjustment(&arg.snd);
    let msg = format_compact!("adjustment for {} is {:?}", arg.snd, a);
    ctx.db
        .ephemeral
        .panel_to_player(&ctx.db.persisted, 10, &arg.fst, msg);
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

fn jtac_set_code(lua: MizLua, arg: ArgTriple<JtId, u16, Ucid>) -> Result<()> {
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
        let add_adjust = |root: &GroupSubMenu, dir: AdjustmentDir| -> Result<()> {
            mc.add_command_for_group(
                mizgid,
                "10m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgQuad {
                    fst: *gid,
                    snd: dir,
                    trd: 10,
                    fth: ucid,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "25m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgQuad {
                    fst: *gid,
                    snd: dir,
                    trd: 25,
                    fth: ucid,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "50m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgQuad {
                    fst: *gid,
                    snd: dir,
                    trd: 50,
                    fth: ucid,
                },
            )?;
            mc.add_command_for_group(
                mizgid,
                "100m".into(),
                Some(root.clone()),
                jtac_adjust_solution,
                ArgQuad {
                    fst: *gid,
                    snd: dir,
                    trd: 100,
                    fth: ucid,
                },
            )?;
            Ok(())
        };
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
        mc.add_command_for_group(
            mizgid,
            "Show Adjustment".into(),
            Some(root.clone()),
            jtac_show_adjustment,
            ArgTuple {
                fst: ucid,
                snd: *gid,
            },
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

fn add_cruise_missile_menu_for_jtac(
    lua: MizLua,
    mizgid: GroupId,
    ucid: Ucid,
    root: GroupSubMenu,
    jtac: JtId,
    aircraft: &[DbGid],
) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    let root = mc.add_submenu_for_group(mizgid, "Cruise Missiles".into(), Some(root.clone()))?;
    for gid in aircraft {
        let root =
            mc.add_submenu_for_group(mizgid, format_compact!("{gid}").into(), Some(root.clone()))?;
        mc.add_command_for_group(
            mizgid,
            "Get Ammo State".into(),
            Some(root.clone()),
            jtac_cruise_missile_get_ammo,
            ArgTriple {
                fst: ucid,
                snd: *gid,
                trd: true,
            },
        )?;
        let for_effect =
            mc.add_submenu_for_group(mizgid, "Launch Salvo".into(), Some(root.clone()))?;
        mc.add_command_for_group(
            mizgid,
            "1".into(),
            Some(for_effect.clone()),
            jtac_cruise_missile_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 1,
                fth: ucid,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "2".into(),
            Some(for_effect.clone()),
            jtac_cruise_missile_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 2,
                fth: ucid,
            },
        )?;
        mc.add_command_for_group(
            mizgid,
            "4".into(),
            Some(for_effect.clone()),
            jtac_cruise_missile_mission,
            ArgQuad {
                fst: jtac,
                snd: *gid,
                trd: 4,
                fth: ucid,
            },
        )?;
    }
    Ok(())
}

fn call_bomber(lua: MizLua, arg: ArgTriple<JtId, Ucid, String>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
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

fn call_cruise_missile_strike(
    lua: MizLua,
    s: String,
    arg: ArgQuad<JtId, Ucid, i64, db::group::GroupId>,
) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let spctx = SpawnCtx::new(lua)?;
    let jtac = get_jtac(&ctx.jtac, &arg.fst)?;
    let q = arg.trd;
    let gid = arg.fth;
    let tgt = jtac.target().clone().ok_or(anyhow!("no jtac target!"))?;

    let ammo_state: i64 = match jtac_cruise_missile_get_ammo_return_i64(
        lua,
        ArgTriple {
            fst: arg.snd,
            snd: gid,
            trd: false,
        },
    ) {
        Ok(a) => a,
        Err(_) => 0,
    };

    let (near, name) = change_info(jtac, &ctx.db, &arg.snd);
    let action = ctx
        .db
        .ephemeral
        .cfg
        .actions
        .get(&jtac.side())
        .and_then(|acts| acts.get(&s))
        .ok_or_else(|| anyhow!("no such action {}", arg.trd))?;
    // let cfg = match &action.kind {
    //     ActionKind::CruiseMissile(cfg, q) => cfg.clone(),
    //     _ => bail!("not a cruise missile bomber action"),
    // };
    match ctx.db.start_action(
        &spctx,
        &ctx.idx,
        &ctx.jtac,
        jtac.side(),
        Some(arg.snd.clone()),
        ActionCmd {
            name: s,
            action: action.clone(),
            args: ActionArgs::CruiseMissile(
                WithPosAndGroup {
                    cfg: (),
                    pos: na::base::Vector2::new(tgt.pos.x, tgt.pos.z),
                    group: gid,
                },
                q,
            ),
        },
    ) {
        Ok(()) => {
            let msg = format_compact!(
                "CRUISE MISSILE STRIKE REQUESTED\ntargeting by jtac {} near {}\nstarted by {}\n{} retasking available at {}/12",
                arg.fst,
                near,
                name,
                gid,
                ammo_state
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
            format_compact!("cruise missile strike could not start {e:?}"),
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
        mc.add_submenu_for_group(mizgid, format_compact!("{}", jtac.gid()).into(), Some(root))?;
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
    add_cruise_missile_menu_for_jtac(
        lua,
        mizgid,
        *ucid,
        root.clone(),
        jtac.gid(),
        jtac.nearby_cruise_missile_bombers(),
    )?;

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
            .insert(arg.trd);
        let mc = MissionCommands::singleton(lua)?;
        let name = ctx.db.objective(&arg.trd)?.name.clone();
        let mut cmd: Vec<String> = arg.fth.clone().into();
        cmd.push(format_compact!("{name}>>").into());
        mc.remove_command_for_group(arg.snd, cmd.into())?;
        let root = mc.add_submenu_for_group(arg.snd, name, Some(arg.fth))?;
        for jtac in ctx.jtac.jtacs() {
            if jtac.side() == player.side && jtac.location().oid == arg.trd {
                add_menu_for_jtac(
                    &ctx.db,
                    player.side,
                    root.clone(),
                    lua,
                    arg.snd,
                    jtac,
                    &arg.fst,
                )?
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

fn add_jtac_locations(lua: MizLua, arg: ArgTuple<Ucid, GroupId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
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
    for jtac in ctx.jtac.jtacs() {
        if jtac.side() == player.side {
            let (near, oid) = match ctx
                .db
                .objective(&jtac.location().oid)
                .map(|o| (o.name.clone(), o.id))
                .ok()
            {
                Some((near, oid)) => (near, oid),
                None => {
                    error!("jtac near missing objective {:?}", jtac.location());
                    continue;
                }
            };
            if !roots.contains(&near) {
                roots.push(near.clone());
                if n >= 8 {
                    root =
                        mc.add_submenu_for_group(arg.snd, "NEXT>>".into(), Some(root.clone()))?;
                    n = 0;
                }
                n += 1;
                mc.add_command_for_group(
                    arg.snd,
                    format_compact!("{near}>>").into(),
                    Some(root.clone()),
                    add_jtacs_by_location,
                    ArgQuad {
                        fst: arg.fst,
                        snd: arg.snd,
                        trd: oid,
                        fth: root.clone(),
                    },
                )?;
            }
        }
    }
    Ok(())
}

pub(crate) fn init_jtac_menu_for_slot(ctx: &mut Context, lua: MizLua, slot: &SlotId) -> Result<()> {
    let ucid = match ctx.db.ephemeral.player_in_slot(slot) {
        Some(ucid) => ucid,
        None => return Ok(()),
    };
    let mc = MissionCommands::singleton(lua)?;
    let si = ctx.db.info_for_slot(slot).context("getting slot info")?;
    ctx.subscribed_jtac_menus.remove(slot);
    mc.remove_command_for_group(si.miz_gid, GroupCommandItem::from(vec!["JTAC>>".into()]))?;
    mc.remove_submenu_for_group(si.miz_gid, GroupSubMenu::from(vec!["JTAC".into()]))?;
    mc.add_command_for_group(
        si.miz_gid,
        "JTAC>>".into(),
        None,
        add_jtac_locations,
        ArgTuple {
            fst: *ucid,
            snd: si.miz_gid,
        },
    )?;
    Ok(())
}
