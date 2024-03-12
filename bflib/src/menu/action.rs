use super::{ArgQuad, ArgTriple, ArgTuple};
use crate::{
    cfg::ActionKind,
    db::{
        group::{DeployKind, GroupId as DbGid},
        objective::ObjectiveId,
    },
    Context,
};
use anyhow::{anyhow, Context as ErrContext, Result};
use compact_str::format_compact;
use dcso3::{
    env::miz::GroupId,
    mission_commands::{GroupCommandItem, GroupSubMenu, MissionCommands},
    net::{SlotId, Ucid},
    object::DcsObject,
    world::World,
    LuaVec3, MizLua, String, Vector3,
};
use fxhash::FxHashMap;

fn run_pos_action(lua: MizLua, arg: ArgTriple<Ucid, String, LuaVec3>) -> Result<()> {
    unimplemented!()
}

fn run_pos_group_action(lua: MizLua, arg: ArgQuad<Ucid, String, LuaVec3, DbGid>) -> Result<()> {
    unimplemented!()
}

fn run_objective_action(lua: MizLua, arg: ArgTriple<Ucid, String, ObjectiveId>) -> Result<()> {
    unimplemented!()
}

fn add_action_menu(lua: MizLua, arg: ArgTriple<Ucid, GroupId, SlotId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let mc = MissionCommands::singleton(lua)?;
    let world = World::singleton(lua)?;
    mc.remove_command_for_group(arg.snd, vec!["Action>>".into()].into())?;
    let root = mc.add_submenu_for_group(arg.snd, "Action".into(), None)?;
    let player = ctx
        .db
        .player(&arg.fst)
        .ok_or_else(|| anyhow!("unknown player"))?;
    let actions = ctx
        .db
        .ephemeral
        .cfg
        .actions
        .get(&player.side)
        .ok_or_else(|| anyhow!("no actions for {}", player.side))?;
    struct Mk {
        pos: Vector3,
        count: usize,
    }
    let mut marks: FxHashMap<String, Mk> = FxHashMap::default();
    for mk in world.get_mark_panels()? {
        let mk = mk?;
        if let Some(unit) = mk.initiator.as_ref() {
            let id = unit.object_id()?;
            if let Some(ucid) = ctx.db.player_in_unit(false, &id) {
                if ucid == arg.fst && mk.text.len() <= 24 {
                    marks
                        .entry(mk.text.clone())
                        .or_insert_with(|| Mk {
                            pos: mk.pos.0,
                            count: 0,
                        })
                        .count += 1;
                }
            }
        }
    }
    marks.retain(|_, mk| mk.count == 1);
    let add_pos = |root: GroupSubMenu, name: String| -> Result<()> {
        for (text, mk) in &marks {
            mc.add_command_for_group(
                arg.snd,
                text.clone(),
                Some(root.clone()),
                run_pos_action,
                ArgTriple {
                    fst: arg.fst,
                    snd: name.clone(),
                    trd: LuaVec3(mk.pos),
                },
            )?;
        }
        Ok(())
    };
    let add_pos_group = |root: GroupSubMenu, name: String| -> Result<()> {
        for gid in &ctx.db.persisted.actions {
            let group = ctx.db.group(gid)?;
            match &group.origin {
                DeployKind::Action { name: aname, .. } if aname == &name => {
                    let root = mc.add_submenu_for_group(
                        arg.snd,
                        format_compact!("{gid}").into(),
                        Some(root.clone()),
                    )?;
                    for (name, mk) in &marks {
                        mc.add_command_for_group(
                            arg.snd,
                            name.clone(),
                            Some(root.clone()),
                            run_pos_group_action,
                            ArgQuad {
                                fst: arg.fst,
                                snd: name.clone(),
                                trd: LuaVec3(mk.pos),
                                fth: *gid,
                            },
                        )?;
                    }
                }
                DeployKind::Action { .. }
                | DeployKind::Crate { .. }
                | DeployKind::Deployed { .. }
                | DeployKind::Objective
                | DeployKind::Troop { .. } => (),
            }
        }
        Ok(())
    };
    let add_objective = |root: GroupSubMenu, name: String| -> Result<()> {
        for (oid, obj) in ctx.db.objectives() {
            if obj.owner == player.side {
                mc.add_command_for_group(
                    arg.snd,
                    obj.name.clone(),
                    Some(root.clone()),
                    run_objective_action,
                    ArgTriple {
                        fst: arg.fst,
                        snd: name.clone(),
                        trd: *oid,
                    },
                )?;
            }
        }
        Ok(())
    };
    for (name, action) in actions {
        let name = if action.cost > 0 {
            String::from(format_compact!("{name}({} pts)", action.cost))
        } else {
            name.clone()
        };
        let root = mc.add_submenu_for_group(arg.snd, name.clone(), Some(root.clone()))?;
        match &action.kind {
            ActionKind::Bomber(_) | ActionKind::LogisticsTransfer(_) | ActionKind::Move(_) => (),
            ActionKind::AttackersWaypoint
            | ActionKind::AwacsWaypoint
            | ActionKind::FighersWaypoint
            | ActionKind::TankerWaypoint
            | ActionKind::DroneWaypoint => add_pos_group(root.clone(), name)?,
            ActionKind::Attackers(_)
            | ActionKind::Awacs(_)
            | ActionKind::Deployable(_)
            | ActionKind::Drone(_)
            | ActionKind::Fighters(_)
            | ActionKind::Tanker(_)
            | ActionKind::Paratrooper(_)
            | ActionKind::Nuke(_) => add_pos(root.clone(), name)?,
            ActionKind::LogisticsRepair(_) => add_objective(root.clone(), name)?,
        }
    }
    ctx.subscribed_action_menus.insert(arg.trd);
    Ok(())
}

pub(super) fn init_action_menu_for_slot(
    ctx: &mut Context,
    lua: MizLua,
    slot: &SlotId,
    ucid: &Ucid,
) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    let si = ctx.db.info_for_slot(slot).context("getting slot info")?;
    ctx.subscribed_jtac_menus.remove(slot);
    mc.remove_command_for_group(si.miz_gid, GroupCommandItem::from(vec!["Actions>>".into()]))?;
    mc.remove_submenu_for_group(si.miz_gid, GroupSubMenu::from(vec!["Actions".into()]))?;
    mc.add_command_for_group(
        si.miz_gid,
        "Actions>>".into(),
        None,
        add_action_menu,
        ArgTriple {
            fst: *ucid,
            snd: si.miz_gid,
            trd: *slot
        },
    )?;
    Ok(())
}
