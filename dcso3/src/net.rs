use super::{as_tbl, coalition::Side, cvt_err, String};
use crate::{env::miz::UnitId, simple_enum, wrapped_prim, wrapped_table, LuaEnv, Sequence};
use anyhow::Result;
use compact_str::format_compact;
use mlua::{prelude::*, Value};
use serde_derive::{Deserialize, Serialize};
use std::ops::Deref;

simple_enum!(PlayerStat, u8, [
    Car => 2,
    Crash => 1,
    Eject => 7,
    ExtraAllyAAA => 17,
    ExtraAllyFighters => 14,
    ExtraAllySAM => 16,
    ExtraAllyTransports => 15,
    ExtraAllyTroops => 18,
    ExtraAllyCoalition => 19,
    ExtraEnemyAAA => 12,
    ExtraEnemyFighters => 9,
    ExtraEnemySAM => 11,
    ExtraEnemyTransports => 10,
    ExtraEnemyTroops => 13,
    Land => 6,
    Num => 20,
    OldScore => 8,
    Ping => 0,
    Plane => 3,
    Score => 5,
    Ship => 4
]);

wrapped_prim!(PlayerId, i64, Copy, Hash);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SlotIdKind {
    Normal,
    ArtilleryCommander,
    ForwardObserver,
    Observer,
    Instructor,
}

wrapped_prim!(SlotId, String, Hash);

impl From<UnitId> for SlotId {
    fn from(value: UnitId) -> Self {
        Self::from(String(format_compact!("{}", value.inner())))
    }
}

impl SlotId {
    pub fn classify(&self) -> SlotIdKind {
        if self.is_artillery_commander() {
            SlotIdKind::ArtilleryCommander
        } else if self.is_observer() {
            SlotIdKind::Observer
        } else if self.is_forward_observer() {
            SlotIdKind::ForwardObserver
        } else if self.is_instructor() {
            SlotIdKind::Instructor
        } else {
            SlotIdKind::Normal
        }
    }

    pub fn is_artillery_commander(&self) -> bool {
        self.0.starts_with("artillery_commander_")
    }

    pub fn is_observer(&self) -> bool {
        self.0.starts_with("observer_")
    }

    pub fn is_forward_observer(&self) -> bool {
        self.0.starts_with("forward_observer_")
    }

    pub fn is_instructor(&self) -> bool {
        self.0.starts_with("instructor_")
    }

    pub fn spectator() -> SlotId {
        Self(String::from("0"))
    }

    pub fn as_unit_id(&self) -> Option<UnitId> {
        self.0.parse::<i64>().ok().map(UnitId::from)
    }
}

wrapped_prim!(Ucid, String, Hash);

wrapped_table!(PlayerInfo, None);

impl<'lua> PlayerInfo<'lua> {
    pub fn id(&self) -> Result<PlayerId> {
        Ok(self.t.raw_get("id")?)
    }

    pub fn name(&self) -> Result<String> {
        Ok(self.t.raw_get("name")?)
    }

    pub fn side(&self) -> Result<Side> {
        Ok(self.t.raw_get("side")?)
    }

    pub fn slot(&self) -> Result<SlotId> {
        Ok(self.t.raw_get("slot")?)
    }

    pub fn ping(&self) -> Result<f32> {
        Ok(self.t.raw_get("ping")?)
    }

    pub fn ip(&self) -> Result<Option<String>> {
        Ok(self.t.raw_get("ipaddr")?)
    }

    pub fn ucid(&self) -> Result<Option<Ucid>> {
        Ok(self.t.raw_get("ucid")?)
    }
}

wrapped_table!(Net, None);

impl<'lua> Net<'lua> {
    pub fn singleton<L: LuaEnv<'lua>>(lua: L) -> Result<Self> {
        Ok(lua.inner().globals().raw_get("net")?)
    }

    pub fn send_chat(&self, message: String, all: bool) -> Result<()> {
        Ok(self.t.call_function("send_chat", (message, all))?)
    }

    pub fn send_chat_to(
        &self,
        message: String,
        player: PlayerId,
        from_id: Option<PlayerId>,
    ) -> Result<()> {
        Ok(self.t
            .call_function("send_chat_to", (message, player, from_id))?)
    }

    pub fn get_player_list(&self) -> Result<Sequence<PlayerId>> {
        Ok(self.t.call_function("get_player_list", ())?)
    }

    pub fn get_my_player_id(&self) -> Result<PlayerId> {
        Ok(self.t.call_function("get_my_player_id", ())?)
    }

    pub fn get_server_id(&self) -> Result<PlayerId> {
        Ok(self.t.call_function("get_server_id", ())?)
    }

    pub fn get_player_info(&self, id: PlayerId) -> Result<PlayerInfo> {
        Ok(self.t.call_function("get_player_info", id)?)
    }

    pub fn kick(&self, id: PlayerId, message: String) -> Result<()> {
        Ok(self.t.call_function("kick", (id, message))?)
    }

    pub fn get_stat(&self, id: PlayerId, stat: PlayerStat) -> Result<i64> {
        Ok(self.t.call_function("get_stat", (id, stat))?)
    }

    pub fn get_name(&self, id: PlayerId) -> Result<String> {
        Ok(self.t.call_function("get_name", id)?)
    }

    pub fn get_slot(&self, id: PlayerId) -> Result<(Side, SlotId)> {
        Ok(self.t.call_function("get_slot", id)?)
    }

    pub fn force_player_slot(&self, id: PlayerId, side: Side, slot: SlotId) -> Result<()> {
        Ok(self.t.call_function("force_player_slot", (id, side, slot))?)
    }

    pub fn lua2json<T: IntoLua<'lua>>(&self, v: T) -> Result<String> {
        Ok(self.t.call_function("lua2json", v)?)
    }

    pub fn json2lua<T: FromLua<'lua>>(&self, v: String) -> Result<T> {
        Ok(self.t.call_function("json2lua", v)?)
    }

    pub fn dostring_in(&self, state: String, dostring: String) -> Result<String> {
        Ok(self.t.call_function("dostring_in", (state, dostring))?)
    }

    pub fn log(&self, message: String) -> Result<()> {
        Ok(self.t.call_function("log", message)?)
    }
}
