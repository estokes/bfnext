use super::ArgTuple;
use crate::Context;
use anyhow::{anyhow, Context as ErrContext, Result};
use dcso3::{
    env::miz::GroupId,
    mission_commands::{GroupCommandItem, GroupSubMenu, MissionCommands},
    net::{SlotId, Ucid},
    object::DcsObject,
    world::World,
    MizLua, String, Vector3,
};
use fxhash::FxHashMap;

fn add_action_menu(lua: MizLua, arg: ArgTuple<Ucid, GroupId>) -> Result<()> {
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
                if ucid == arg.fst {
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
    for (name, action) in actions {}
    unimplemented!()
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
        ArgTuple {
            fst: *ucid,
            snd: si.miz_gid,
        },
    )?;
    Ok(())
}
