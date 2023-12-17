use anyhow::{Result, anyhow};
use dcso3::{
    coalition::{Coalition, Side},
    env::miz::{GroupInfo, GroupKind, Miz, MizIndex, TriggerZone},
    MizLua, String, Vector2, DeepClone, LuaEnv, group::GroupCategory,
};
use fxhash::FxHashMap;
use serde_derive::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SpawnLoc {
    AtPos(Vector2),
    AtPosWithComponents(Vector2, FxHashMap<String, Vector2>),
    AtTrigger { name: String, offset: Vector2 },
}

pub struct SpawnCtx<'lua> {
    coalition: Coalition<'lua>,
    miz: Miz<'lua>,
    lua: MizLua<'lua>,
}

pub enum Despawn<'a> {
    Group(&'a str),
    Static(&'a str),
}

impl<'lua> SpawnCtx<'lua> {
    pub fn new(lua: MizLua<'lua>) -> Result<Self> {
        Ok(Self {
            coalition: Coalition::singleton(lua)?,
            miz: Miz::singleton(lua)?,
            lua,
        })
    }

    pub fn get_template(
        &self,
        idx: &MizIndex,
        kind: GroupKind,
        side: Side,
        template_name: &str,
    ) -> Result<GroupInfo> {
        let mut template = self
            .miz
            .get_group_by_name(idx, kind, side, template_name)?
            .ok_or_else(|| anyhow!("no such template {template_name}"))?;
        template.group = template.group.deep_clone(self.lua.inner())?;
        Ok(template)
    }

    pub fn get_trigger_zone(&self, idx: &MizIndex, name: &str) -> Result<TriggerZone> {
        Ok(self
            .miz
            .get_trigger_zone(idx, name)?
            .ok_or_else(|| anyhow!("no such trigger zone {name}"))?)
    }

    pub fn spawn(&self, template: GroupInfo) -> Result<()> {
        match GroupCategory::from_kind(template.category) {
            None => {
                // static objects are not fed to addStaticObject as groups
                let unit = template.group.units()?.first()?;
                self.coalition.add_static_object(template.country, unit)?;
            }
            Some(category) => {
                self.coalition
                    .add_group(template.country, category, template.group)?;
            }
        }
        Ok(())
    }

    pub fn despawn(&self, name: Despawn) -> Result<()> {
        match name {
            Despawn::Group(name) => {
                let group = dcso3::group::Group::get_by_name(self.lua, name)?;
                Ok(group.destroy()?)
            }
            Despawn::Static(name) => {
                let obj = dcso3::static_object::StaticObject::get_by_name(self.lua, name)?;
                Ok(obj.as_object()?.destroy()?)
            }
        }
    }
}
