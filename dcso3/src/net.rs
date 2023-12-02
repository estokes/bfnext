use super::{as_tbl, coalition::Side, cvt_err, String};
use crate::{simple_enum, wrapped_prim, wrapped_table, LuaEnv, Sequence};
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

// slots in dcs are always positive numbers except for 4 special cases
// - artillery commanders
// - forward observers
// - observers
// - instructors
//
// Those slots are strings. Since it would be very inefficent to make slotid a string
// just because of a few special cases, we instead use the top 8 bits of slot id as flags
// indicating whether a given slot id is a special case or not.
const ARTYCMDR_MASK: u64 = 0x8000_0000_0000_0000;
const OBSVR_MASK: u64 = 0x4000_0000_0000_0000;
const FWDOBSVR_MASK: u64 = 0x2000_0000_0000_0000;
const INSTR_MASK: u64 = 0x1000_0000_0000_0000;
const RED_MASK: u64 = 0x0100_0000_0000_0000;
const BLUE_MASK: u64 = 0x0200_0000_0000_0000;
const NEUTRAL_MASK: u64 = 0x0400_0000_0000_0000;
const FLAGS_MASK: u64 = 0x00FF_FFFF_FFFF_FFFF;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SlotIdKind {
    Normal,
    ArtilleryCommander,
    ForwardObserver,
    Observer,
    Instructor
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct SlotId(u64);

pub const SPECTATOR: SlotId = SlotId(0);

impl<'lua> FromLua<'lua> for SlotId {
    fn from_lua(value: Value<'lua>, _lua: &'lua Lua) -> LuaResult<Self> {
        fn parse(s: &str) -> LuaResult<u64> {
            s.parse::<u64>()
                .map_err(|_| cvt_err("number expected in slotid"))
        }
        match value {
            Value::Integer(i) => {
                if i < 0 {
                    Err(cvt_err("slots must be positive"))
                } else {
                    Ok(Self(i as u64))
                }
            }
            Value::String(s) => {
                let s = s.to_str()?;
                if s.starts_with("artillery_commander") {
                    if let Some(s) = s.strip_prefix("artillery_commander_red_") {
                        Ok(Self(ARTYCMDR_MASK | RED_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("artillery_commander_blue_") {
                        Ok(Self(ARTYCMDR_MASK | BLUE_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("artillery_commander_neutral_") {
                        Ok(Self(ARTYCMDR_MASK | NEUTRAL_MASK | parse(s)?))
                    } else {
                        Err(cvt_err("malformed artillery commander slot"))
                    }
                } else if s.starts_with("observer") {
                    if let Some(s) = s.strip_prefix("observer_red_") {
                        Ok(Self(OBSVR_MASK | RED_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("observer_blue_") {
                        Ok(Self(OBSVR_MASK | BLUE_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("observer_neutral_") {
                        Ok(Self(OBSVR_MASK | NEUTRAL_MASK | parse(s)?))
                    } else {
                        Err(cvt_err("malformed observer slot"))
                    }
                } else if s.starts_with("forward_observer") {
                    if let Some(s) = s.strip_prefix("forward_observer_red_") {
                        Ok(Self(FWDOBSVR_MASK | RED_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("forward_observer_blue_") {
                        Ok(Self(FWDOBSVR_MASK | BLUE_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("forward_observer_neutral_") {
                        Ok(Self(FWDOBSVR_MASK | BLUE_MASK | parse(s)?))
                    } else {
                        Err(cvt_err("malformed forward observer slot"))
                    }
                } else if s.starts_with("instructor") {
                    if let Some(s) = s.strip_prefix("instructor_red_") {
                        Ok(Self(INSTR_MASK | RED_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("instructor_blue_") {
                        Ok(Self(INSTR_MASK | BLUE_MASK | parse(s)?))
                    } else if let Some(s) = s.strip_prefix("instructor_neutral_") {
                        Ok(Self(FWDOBSVR_MASK | NEUTRAL_MASK | parse(s)?))
                    } else {
                        Err(cvt_err("malformed instructor slot"))
                    }
                } else {
                    Err(cvt_err("invalid string slot id"))
                }
            }
            _ => Err(cvt_err("Invalid type for slotid")),
        }
    }
}

impl<'lua> IntoLua<'lua> for SlotId {
    fn into_lua(self, lua: &'lua Lua) -> LuaResult<Value<'lua>> {
        let index = self.index();
        match self.classify() {
            SlotIdKind::ArtilleryCommander => {
                let side = self.side_str().unwrap();
                String(format_compact!("artillery_commander_{}_{}", side, index)).into_lua(lua)
            }
            SlotIdKind::ForwardObserver => {
                let side = self.side_str().unwrap();
                String(format_compact!("forward_observer_{}_{}", side, index)).into_lua(lua)
            }
            SlotIdKind::Instructor => {
                let side = self.side_str().unwrap();
                String(format_compact!("instructor_{}_{}", side, index)).into_lua(lua)
            }
            SlotIdKind::Observer => {
                let side = self.side_str().unwrap();
                String(format_compact!("observer_{}_{}", side, index)).into_lua(lua)
            }
            SlotIdKind::Normal => {
                index.into_lua(lua)
            }
        }
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
        self.0 & ARTYCMDR_MASK > 0
    }

    pub fn is_observer(&self) -> bool {
        self.0 & OBSVR_MASK > 0
    }

    pub fn is_forward_observer(&self) -> bool {
        self.0 & FWDOBSVR_MASK > 0
    }

    pub fn is_instructor(&self) -> bool {
        self.0 & INSTR_MASK > 0
    }

    pub fn side(&self) -> Option<Side> {
        if self.0 & RED_MASK > 0 {
            Some(Side::Red)
        } else if self.0 & BLUE_MASK > 0 {
            Some(Side::Blue)
        } else if self.0 & NEUTRAL_MASK > 0 {
            Some(Side::Neutral)
        } else {
            None
        }
    }

    pub fn side_str(&self) -> Option<&'static str> {
        match self.side() {
            Some(Side::Red) => Some("red"),
            Some(Side::Blue) => Some("blue"),
            Some(Side::Neutral) => Some("neutral"),
            None => None
        }
    }

    pub fn index(&self) -> u64 {
        self.0 & FLAGS_MASK
    }
}

impl From<i64> for SlotId {
    fn from(value: i64) -> Self {
        Self(value as u64 & FLAGS_MASK)
    }
}

impl From<u64> for SlotId {
    fn from(value: u64) -> Self {
        Self(value & FLAGS_MASK)
    }
}

wrapped_prim!(Ucid, String, Hash);

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
    pub fn singleton<L: LuaEnv<'lua>>(lua: L) -> LuaResult<Self> {
        lua.inner().globals().raw_get("net")
    }

    pub fn send_chat(&self, message: String, all: bool) -> LuaResult<()> {
        self.t.call_function("send_chat", (message, all))
    }

    pub fn send_chat_to(
        &self,
        message: String,
        player: PlayerId,
        from_id: Option<PlayerId>,
    ) -> LuaResult<()> {
        self.t
            .call_function("send_chat_to", (message, player, from_id))
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

    pub fn log(&self, message: String) -> LuaResult<()> {
        self.t.call_function("log", message)
    }
}
