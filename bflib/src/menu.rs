use crate::{cfg::Cfg, Context};
use anyhow::Result;
use dcso3::{
    as_tbl,
    coalition::Side,
    env::miz::{GroupId, Miz},
    lua_err,
    mission_commands::MissionCommands,
    MizLua, String,
};
use mlua::{prelude::*, Value};

struct SpawnCrateArg {
    group: GroupId,
    crate_name: String,
}

impl<'lua> IntoLua<'lua> for SpawnCrateArg {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<LuaValue<'lua>> {
        let tbl = lua.create_table()?;
        tbl.raw_set("group", self.group)?;
        tbl.raw_set("crate_name", self.crate_name)?;
        Ok(Value::Table(tbl))
    }
}

impl<'lua> FromLua<'lua> for SpawnCrateArg {
    fn from_lua(value: LuaValue<'lua>, lua: &'lua Lua) -> LuaResult<Self> {
        let tbl = as_tbl("SpawnCrateArg", None, value).map_err(lua_err)?;
        Ok(Self {
            group: tbl.raw_get("group")?,
            crate_name: tbl.raw_get("crate_name")?,
        })
    }
}

fn unpakistan(lua: MizLua, gid: GroupId) -> Result<()> {
    unimplemented!()
}

fn load_crate(lua: MizLua, gid: GroupId) -> Result<()> {
    unimplemented!()
}

fn unload_crate(lua: MizLua, gid: GroupId) -> Result<()> {
    unimplemented!()
}

fn list_nearby_crates(lua: MizLua, gid: GroupId) -> Result<()> {
    unimplemented!()
}

fn destroy_nearby_crate(lua: MizLua, gid: GroupId) -> Result<()> {
    unimplemented!()
}

fn spawn_crate(lua: MizLua, arg: SpawnCrateArg) -> Result<()> {
    unimplemented!()
}

fn add_cargo_menu_for_group(
    lua: MizLua,
    cfg: &Cfg,
    mc: &MissionCommands,
    side: &Side,
    group: GroupId,
) -> Result<()> {
    let root = mc.add_submenu_for_group(group, "Cargo".into(), None)?;
    mc.add_command_for_group(
        group,
        "Unpakistan!".into(),
        Some(root.clone()),
        unpakistan,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Load Nearby Crate".into(),
        Some(root.clone()),
        load_crate,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Unload Crate".into(),
        Some(root.clone()),
        unload_crate,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "List Nearby Crates".into(),
        Some(root.clone()),
        list_nearby_crates,
        group,
    )?;
    mc.add_command_for_group(
        group,
        "Destroy Nearby Crate".into(),
        Some(root.clone()),
        destroy_nearby_crate,
        group,
    )?;
    for dep in cfg.deployables.get(side).unwrap_or(&vec![]) {
        let root = dep.path.iter().fold(Ok(root.clone()), |root: Result<_>, p| {
            let root = root?;
            Ok(mc.add_submenu_for_group(group, p.clone(), Some(root))?)
        })?;
        for cr in dep.crates.iter() {
            mc.add_command_for_group(
                group,
                cr.name.clone(),
                Some(root.clone()),
                spawn_crate,
                SpawnCrateArg {
                    group,
                    crate_name: cr.name.clone(),
                },
            )?;
        }
    }
    Ok(())
}

pub(super) fn init(ctx: &Context, lua: MizLua) -> Result<()> {
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
