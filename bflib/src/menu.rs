use crate::{cfg::Cfg, Context};
use anyhow::{anyhow, bail, Result};
use compact_str::format_compact;
use dcso3::{
    as_tbl,
    coalition::Side,
    env::miz::{Group, GroupId, Miz},
    lua_err,
    mission_commands::{GroupSubMenu, MissionCommands},
    net::SlotId,
    LuaEnv, MizLua, String,
};
use fxhash::FxHashMap;
use log::debug;
use mlua::{prelude::*, Value};
use std::collections::hash_map::Entry;

#[derive(Debug)]
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

fn slot_for_group(lua: MizLua, ctx: &Context, gid: &GroupId) -> Result<SlotId> {
    let hooks = dcso3::is_hooks_env(lua.inner());
    debug!("getting mission. is hooks: {hooks}");
    let miz = Miz::singleton(lua)?;
    debug!("getting slot for group {:?}", gid);
    let group = miz
        .get_group(&ctx.idx, gid)?
        .ok_or_else(|| anyhow!("no such group {:?}", gid))?;
    debug!("got group {:?}", group);
    let units = group.group.units()?;
    if units.len() > 1 {
        bail!(
            "groups with more than one member can't spawn crates {:?}",
            gid
        )
    }
    debug!("getting unit");
    let unit = units.first()?;
    debug!("got unit {:?}", unit);
    let slot = SlotId::from(unit.id()?);
    Ok(slot)
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
    debug!("spawning crate {:?}", arg);
    let ctx = unsafe { Context::get_mut() };
    debug!("getting slot for group");
    let slot = slot_for_group(lua, ctx, &arg.group)?;
    debug!("group slot is {:?}", slot);
    ctx.db.spawn_crate(lua, &ctx.idx, &slot, &arg.crate_name)
}

fn add_cargo_menu_for_group(
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
    let mut created_menus: FxHashMap<String, GroupSubMenu> = FxHashMap::default();
    for dep in cfg.deployables.get(side).unwrap_or(&vec![]) {
        let root = dep
            .path
            .iter()
            .fold(Ok(root.clone()), |root: Result<_>, p| {
                let root = root?;
                match created_menus.entry(p.clone()) {
                    Entry::Occupied(e) => Ok(e.get().clone()),
                    Entry::Vacant(e) => Ok(e
                        .insert(mc.add_submenu_for_group(group, p.clone(), Some(root))?)
                        .clone()),
                }
            })?;
        for cr in dep.crates.iter() {
            let title = if cr.required > 1 {
                String::from(format_compact!("{}({})", cr.name, cr.required))
            } else {
                cr.name.clone()
            };
            mc.add_command_for_group(
                group,
                title,
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

fn can_carry_cargo(cfg: &Cfg, group: &Group) -> Result<bool> {
    Ok(group
        .units()?
        .into_iter()
        .map(|unit| {
            let unit = unit?;
            let typ = unit.typ()?;
            match cfg.cargo.get(&**typ) {
                None => Ok(false),
                Some(c) if c.crate_slots > 0 && c.total_slots > 0 => Ok(true),
                Some(_) => Ok(false),
            }
        })
        .any(|r: Result<bool>| match r {
            Ok(true) => true,
            Ok(false) => false,
            Err(_) => false,
        }))
}

pub(super) fn init(ctx: &Context, lua: MizLua) -> Result<()> {
    debug!("initializing menus");
    let cfg = ctx.db.cfg();
    let miz = Miz::singleton(lua)?;
    let mc = MissionCommands::singleton(lua)?;
    for side in [Side::Red, Side::Blue, Side::Neutral] {
        debug!("init menus for {:?}", side);
        let coa = miz.coalition(side)?;
        for country in coa.countries()? {
            let country = country?;
            debug!("init menus for country {:?}", country);
            for heli in country.helicopters()? {
                let heli = heli?;
                debug!("checking if heli {:?} can carry cargo", heli);
                if can_carry_cargo(cfg, &heli)? {
                    debug!("adding cargo menus for heli");
                    add_cargo_menu_for_group(cfg, &mc, &side, heli.id()?)?
                }
            }
            for plane in country.planes()? {
                let plane = plane?;
                debug!("checking if plane {:?} can carry cargo", plane);
                if can_carry_cargo(cfg, &plane)? {
                    debug!("adding cargo menus for plane");
                    add_cargo_menu_for_group(cfg, &mc, &side, plane.id()?)?
                }
            }
        }
    }
    Ok(())
}
