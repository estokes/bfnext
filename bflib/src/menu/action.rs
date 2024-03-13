use super::{ArgPent, ArgQuad, ArgTriple};
use crate::{
    cfg::{Action, ActionKind},
    db::{
        actions::{ActionArgs, ActionCmd, WithObj, WithPos, WithPosAndGroup},
        group::{DeployKind, GroupId as DbGid},
        objective::ObjectiveId,
    },
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
    trigger::MarkId,
    world::World,
    LuaVec3, MizLua, String, Vector2, Vector3,
};
use fxhash::FxHashMap;

fn run_action(
    ctx: &mut Context,
    lua: MizLua,
    side: Side,
    slot: SlotId,
    ucid: Ucid,
    mark: Option<MarkId>,
    cmd: ActionCmd,
) -> Result<()> {
    let spctx = SpawnCtx::new(lua)?;
    ctx.db
        .start_action(&spctx, &ctx.idx, &ctx.jtac, side, Some(ucid), cmd)?;
    if let Some(mark) = mark {
        ctx.db.ephemeral.msgs().delete_mark(mark);
    }
    init_action_menu_for_slot(ctx, lua, &slot, &ucid)
}

fn do_pos_action(
    ctx: &mut Context,
    lua: MizLua,
    side: Side,
    slot: SlotId,
    ucid: Ucid,
    name: String,
    pos: LuaVec3,
    mark: MarkId,
    action: Action,
) -> Result<()> {
    let pos = Vector2::new(pos.0.x, pos.0.z);
    let args = match &action.kind {
        ActionKind::Attackers(cfg) => ActionArgs::Attackers(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Awacs(cfg) => ActionArgs::Awacs(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Deployable(cfg) => ActionArgs::Deployable(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Drone(cfg) => ActionArgs::Drone(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Fighters(cfg) => ActionArgs::Fighters(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Tanker(cfg) => ActionArgs::Tanker(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Paratrooper(cfg) => ActionArgs::Paratrooper(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Nuke(cfg) => ActionArgs::Nuke(WithPos {
            cfg: cfg.clone(),
            pos,
        }),
        ActionKind::Bomber(_)
        | ActionKind::LogisticsTransfer(_)
        | ActionKind::LogisticsRepair(_)
        | ActionKind::Move(_)
        | ActionKind::TankerWaypoint
        | ActionKind::AwacsWaypoint
        | ActionKind::FighersWaypoint
        | ActionKind::DroneWaypoint
        | ActionKind::AttackersWaypoint => bail!("invalid action type for this menu item"),
    };
    let cmd = ActionCmd { name, action, args };
    run_action(ctx, lua, side, slot, ucid, Some(mark), cmd)
}

fn side_slot_action(ctx: &mut Context, ucid: &Ucid, name: &str) -> Result<(Side, SlotId, Action)> {
    let player = ctx
        .db
        .player(ucid)
        .ok_or_else(|| anyhow!("no such player"))?;
    let side = player.side;
    let slot = player
        .current_slot
        .as_ref()
        .map(|(slot, _)| *slot)
        .ok_or_else(|| anyhow!("missing slot"))?;
    let action = ctx
        .db
        .ephemeral
        .cfg
        .actions
        .get(&side)
        .ok_or_else(|| anyhow!("missions actions for {side}"))?
        .get(name)
        .ok_or_else(|| anyhow!("missing action {}", name))?
        .clone();
    Ok((side, slot, action))
}

fn run_pos_action(lua: MizLua, arg: ArgQuad<Ucid, String, LuaVec3, MarkId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot, action) = side_slot_action(ctx, &arg.fst, &arg.snd)?;
    match do_pos_action(
        ctx,
        lua,
        side,
        slot,
        arg.fst,
        arg.snd.clone(),
        arg.trd,
        arg.fth,
        action,
    ) {
        Ok(()) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            10,
            &arg.fst,
            format_compact!("action {} started", arg.snd),
        ),
        Err(e) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            10,
            &arg.fst,
            format_compact!("could not start {}, {e:?}", arg.snd),
        ),
    }
    Ok(())
}

fn do_pos_group_action(
    ctx: &mut Context,
    lua: MizLua,
    side: Side,
    slot: SlotId,
    ucid: Ucid,
    name: String,
    pos: LuaVec3,
    group: DbGid,
    mark: MarkId,
    action: Action,
) -> Result<()> {
    let pos = Vector2::new(pos.0.x, pos.0.z);
    let args = match &action.kind {
        ActionKind::TankerWaypoint => ActionArgs::TankerWaypoint(WithPosAndGroup {
            cfg: (),
            pos,
            group,
        }),
        ActionKind::AwacsWaypoint => ActionArgs::AwacsWaypoint(WithPosAndGroup {
            cfg: (),
            pos,
            group,
        }),
        ActionKind::FighersWaypoint => ActionArgs::FightersWaypoint(WithPosAndGroup {
            cfg: (),
            pos,
            group,
        }),
        ActionKind::DroneWaypoint => ActionArgs::DroneWaypoint(WithPosAndGroup {
            cfg: (),
            pos,
            group,
        }),
        ActionKind::AttackersWaypoint => ActionArgs::AttackersWaypoint(WithPosAndGroup {
            cfg: (),
            pos,
            group,
        }),
        ActionKind::Move(cfg) => ActionArgs::Move(WithPosAndGroup {
            cfg: cfg.clone(),
            pos,
            group,
        }),
        ActionKind::Attackers(_)
        | ActionKind::Awacs(_)
        | ActionKind::Deployable(_)
        | ActionKind::Drone(_)
        | ActionKind::Fighters(_)
        | ActionKind::Tanker(_)
        | ActionKind::Paratrooper(_)
        | ActionKind::Nuke(_)
        | ActionKind::Bomber(_)
        | ActionKind::LogisticsTransfer(_)
        | ActionKind::LogisticsRepair(_) => bail!("invalid action type for this menu item"),
    };
    let cmd = ActionCmd { name, action, args };
    run_action(ctx, lua, side, slot, ucid, Some(mark), cmd)
}

fn run_pos_group_action(
    lua: MizLua,
    arg: ArgPent<Ucid, String, LuaVec3, DbGid, MarkId>,
) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot, action) = side_slot_action(ctx, &arg.fst, &arg.snd)?;
    match do_pos_group_action(
        ctx,
        lua,
        side,
        slot,
        arg.fst,
        arg.snd.clone(),
        arg.trd,
        arg.fth,
        arg.pnt,
        action,
    ) {
        Ok(()) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            10,
            &arg.fst,
            format_compact!("action {} started for {}", arg.snd, arg.fth),
        ),
        Err(e) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            10,
            &arg.fst,
            format_compact!("could not start {} for {}, {e:?}", arg.snd, arg.fth),
        ),
    }
    Ok(())
}

fn do_objective_action(
    ctx: &mut Context,
    lua: MizLua,
    side: Side,
    slot: SlotId,
    ucid: Ucid,
    name: String,
    oid: ObjectiveId,
    action: Action,
) -> Result<()> {
    let args = match &action.kind {
        ActionKind::LogisticsRepair(cfg) => ActionArgs::LogisticsRepair(WithObj {
            cfg: cfg.clone(),
            oid,
        }),
        ActionKind::TankerWaypoint
        | ActionKind::AwacsWaypoint
        | ActionKind::FighersWaypoint
        | ActionKind::DroneWaypoint
        | ActionKind::AttackersWaypoint
        | ActionKind::Attackers(_)
        | ActionKind::Awacs(_)
        | ActionKind::Deployable(_)
        | ActionKind::Drone(_)
        | ActionKind::Fighters(_)
        | ActionKind::Tanker(_)
        | ActionKind::Paratrooper(_)
        | ActionKind::Nuke(_)
        | ActionKind::Bomber(_)
        | ActionKind::LogisticsTransfer(_)
        | ActionKind::Move(_) => bail!("invalid action type for this menu item"),
    };
    let cmd = ActionCmd { name, action, args };
    run_action(ctx, lua, side, slot, ucid, None, cmd)
}

fn run_objective_action(lua: MizLua, arg: ArgTriple<Ucid, String, ObjectiveId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let (side, slot, action) = side_slot_action(ctx, &arg.fst, &arg.snd)?;
    match do_objective_action(
        ctx,
        lua,
        side,
        slot,
        arg.fst,
        arg.snd.clone(),
        arg.trd,
        action,
    ) {
        Ok(()) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            10,
            &arg.fst,
            format_compact!("action {} started", arg.snd),
        ),
        Err(e) => ctx.db.ephemeral.panel_to_player(
            &ctx.db.persisted,
            10,
            &arg.fst,
            format_compact!("could not start {}, {e:?}", arg.snd),
        ),
    }
    Ok(())
}

fn add_action_menu(lua: MizLua, arg: ArgTriple<Ucid, GroupId, SlotId>) -> Result<()> {
    let ctx = unsafe { Context::get_mut() };
    let mc = MissionCommands::singleton(lua)?;
    let world = World::singleton(lua)?;
    mc.remove_command_for_group(arg.snd, vec!["Actions>>".into()].into())?;
    let mut root = mc.add_submenu_for_group(arg.snd, "Actions".into(), None)?;
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
        id: MarkId,
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
                            id: mk.id,
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
                ArgQuad {
                    fst: arg.fst,
                    snd: name.clone(),
                    trd: LuaVec3(mk.pos),
                    fth: mk.id,
                },
            )?;
        }
        Ok(())
    };
    let add_pos_group = |mut root: GroupSubMenu, name: String, action: bool| -> Result<()> {
        let iter = if action {
            ctx.db.persisted.actions.into_iter()
        } else {
            ctx.db.persisted.deployed.into_iter()
        };
        let mut n = 0;
        for gid in iter {
            if n >= 9 {
                root = mc.add_submenu_for_group(arg.snd, "Next>>".into(), Some(root))?;
                n = 0;
            }
            let group = ctx.db.group(gid)?;
            let key = match &group.origin {
                DeployKind::Action { name, .. } => {
                    if action {
                        Some(name.clone())
                    } else {
                        None
                    }
                }
                DeployKind::Deployed { spec, .. } => {
                    if !action {
                        Some(spec.path.last().unwrap().clone())
                    } else {
                        None
                    }
                }
                DeployKind::Troop { spec, .. } => {
                    if !action {
                        Some(format_compact!("{} Troop", spec.name).into())
                    } else {
                        None
                    }
                }
                DeployKind::Crate { .. } | DeployKind::Objective => None,
            };
            if let Some(key) = key {
                let root = mc.add_submenu_for_group(
                    arg.snd,
                    format_compact!("{gid}({key})").into(),
                    Some(root.clone()),
                )?;
                for (text, mk) in &marks {
                    mc.add_command_for_group(
                        arg.snd,
                        text.clone(),
                        Some(root.clone()),
                        run_pos_group_action,
                        ArgPent {
                            fst: arg.fst,
                            snd: name.clone(),
                            trd: LuaVec3(mk.pos),
                            fth: *gid,
                            pnt: mk.id,
                        },
                    )?;
                }
            }
            n += 1;
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
    let mut n = 0;
    for (name, action) in actions {
        if n > 9 {
            root = mc.add_submenu_for_group(arg.snd, "Next>>".into(), Some(root))?;
            n = 0;
        }
        let title = if action.cost > 0 {
            String::from(format_compact!("{name}({} pts)", action.cost))
        } else {
            name.clone()
        };
        match &action.kind {
            ActionKind::Bomber(_) | ActionKind::LogisticsTransfer(_) => (),
            ActionKind::AttackersWaypoint
            | ActionKind::AwacsWaypoint
            | ActionKind::FighersWaypoint
            | ActionKind::TankerWaypoint
            | ActionKind::DroneWaypoint => {
                let root = mc.add_submenu_for_group(arg.snd, title, Some(root.clone()))?;
                add_pos_group(root.clone(), name.clone(), true)?
            }
            ActionKind::Move(_) => {
                let root = mc.add_submenu_for_group(arg.snd, title, Some(root.clone()))?;
                add_pos_group(root.clone(), name.clone(), false)?
            }
            ActionKind::Attackers(_)
            | ActionKind::Awacs(_)
            | ActionKind::Deployable(_)
            | ActionKind::Drone(_)
            | ActionKind::Fighters(_)
            | ActionKind::Tanker(_)
            | ActionKind::Paratrooper(_)
            | ActionKind::Nuke(_) => {
                let root = mc.add_submenu_for_group(arg.snd, title, Some(root.clone()))?;
                add_pos(root.clone(), name.clone())?
            }
            ActionKind::LogisticsRepair(_) => {
                let root = mc.add_submenu_for_group(arg.snd, title, Some(root.clone()))?;
                add_objective(root.clone(), name.clone())?
            }
        }
        n += 1;
    }
    ctx.subscribed_action_menus.insert(arg.trd);
    Ok(())
}

pub(crate) fn init_action_menu_for_slot(
    ctx: &mut Context,
    lua: MizLua,
    slot: &SlotId,
    ucid: &Ucid,
) -> Result<()> {
    let mc = MissionCommands::singleton(lua)?;
    let si = ctx.db.info_for_slot(slot).context("getting slot info")?;
    ctx.subscribed_action_menus.remove(slot);
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
            trd: *slot,
        },
    )?;
    Ok(())
}
