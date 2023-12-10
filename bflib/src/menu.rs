use crate::Context;
use dcso3::{
    coalition::Side,
    env::miz::{GroupId, Miz},
    MizLua,
};
use mlua::prelude::*;

fn add_cargo_menu_for_group(lua: MizLua, group: GroupId) -> LuaResult<()> {
    unimplemented!()
}

pub(super) fn init(ctx: &Context, lua: MizLua) -> LuaResult<()> {
    let cfg = ctx.db.cfg();
    let miz = Miz::singleton(lua)?;
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
