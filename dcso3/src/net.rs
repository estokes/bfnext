use crate::{simple_enum, wrapped_table, wrapped_prim, Sequence};
use super::{as_tbl, coalition::Side, cvt_err, String};
use mlua::{prelude::*, Value};
use serde_derive::{Serialize, Deserialize};
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

wrapped_prim!(PlayerId, i64, Copy);
wrapped_prim!(SlotId, i64, Copy);
wrapped_prim!(Ucid, String);

wrapped_table!(PlayerInfo, None);

impl<'lua> PlayerInfo<'lua> {
    pub fn id(&self) -> LuaResult<PlayerId> {
        self.t.raw_get("id")
    }

    pub fn name(&self) -> LuaResult<String> {
        self.t.raw_get("name")
    }

    pub fn side(&self) -> LuaResult<Side> {
        self.t.raw_get("side")
    }

    pub fn slot(&self) -> LuaResult<SlotId> {
        self.t.raw_get("slot")
    }

    pub fn ping(&self) -> LuaResult<f32> {
        self.t.raw_get("ping")
    }

    pub fn ip(&self) -> LuaResult<Option<String>> {
        self.t.raw_get("ipaddr")
    }

    pub fn ucid(&self) -> LuaResult<Option<Ucid>> {
        self.t.raw_get("ucid")
    }
}

wrapped_table!(Net, None);

impl<'lua> Net<'lua> {
    pub fn singleton(lua: &'lua Lua) -> LuaResult<Self> {
        lua.globals().raw_get("net")
    }

    pub fn send_chat(&self, message: String, all: bool) -> LuaResult<()> {
        self.t.call_function("send_chat", (message, all))
    }

    pub fn send_chat_to(&self, message: String, player: PlayerId, from_id: Option<PlayerId>) -> LuaResult<()> {
        self.t.call_function("send_chat_to", (message, player, from_id))
    }

    pub fn get_player_list(&self) -> LuaResult<Sequence<PlayerId>> {
        self.t.call_function("get_player_list", ())
    }

    pub fn get_my_player_id(&self) -> LuaResult<PlayerId> {
        self.t.call_function("get_my_player_id", ())
    }

    pub fn get_server_id(&self) -> LuaResult<PlayerId> {
        self.t.call_function("get_server_id", ())
    } 

    pub fn get_player_info(&self, id: PlayerId) -> LuaResult<PlayerInfo> {
        self.t.call_function("get_player_info", id)
    }

    pub fn kick(&self, id: PlayerId, message: String) -> LuaResult<()> {
        self.t.call_function("kick", (id, message))
    }

    pub fn get_stat(&self, id: PlayerId, stat: PlayerStat) -> LuaResult<i64> {
        self.t.call_function("get_stat", (id, stat))
    }

    pub fn get_name(&self, id: PlayerId) -> LuaResult<String> {
        self.t.call_function("get_name", id)
    }

    pub fn get_slot(&self, id: PlayerId) -> LuaResult<(Side, SlotId)> {
        self.t.call_function("get_slot", id)
    }

    pub fn force_player_slot(&self, id: PlayerId, side: Side, slot: SlotId) -> LuaResult<()> {
        self.t.call_function("force_player_slot", (id, side, slot))
    }

    pub fn lua2json<T: IntoLua<'lua>>(&self, v: T) -> LuaResult<String> {
        self.t.call_function("lua2json", v)
    }

    pub fn json2lua<T: FromLua<'lua>>(&self, v: String) -> LuaResult<T> {
        self.t.call_function("json2lua", v)
    }

    pub fn dostring_in(&self, state: String, dostring: String) -> LuaResult<String> {
        self.t.call_function("dostring_in", (state, dostring))
    }
}
