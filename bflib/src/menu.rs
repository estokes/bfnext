use crate::{cfg::Cfg, Context};
use dcso3::{
    coalition::Side,
    env::miz::{GroupId, Miz},
    mission_commands::MissionCommands,
    MizLua,
};
use mlua::prelude::*;

fn add_cargo_menu_for_group(
    lua: MizLua,
    cfg: &Cfg,
    mc: &MissionCommands,
    side: &Side,
    group: GroupId,
) -> LuaResult<()> {
    let root = mc.add_submenu_for_group(group, "Cargo".into(), None)?;

    for dep in cfg.deployables.get(side).unwrap_or(&vec![]) {

    }
    unimplemented!()
}

pub(super) fn init(ctx: &Context, lua: MizLua) -> LuaResult<()> {
    let cfg = ctx.db.cfg();
    let miz = Miz::singleton(lua)?;
    let mc = MissionCommands::singleton(lua)?;
    for side in [Side::Red, Side::Blue, Side::Neutral] {
        let coa = miz.coalition(side)?;
        for country in coa.countries()? {
            let country = country?;
            for heli in country.helicopters()? {
                let heli = heli?;
                unimplemented!()
            }
        }
    }
    unimplemented!()
}
