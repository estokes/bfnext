use crate::admin::{AdminCommand, WarehouseKind};
use anyhow::Result;
use arcstr::ArcStr;
use bfprotocols::db::group::GroupId;
use chrono::prelude::*;
use crossbeam::queue::SegQueue;
use dcso3::coalition::Side;
use futures::{channel::mpsc, stream::StreamExt};
use netidx::{
    chars::Chars,
    path::Path,
    publisher::{Publisher, Value},
};
use netidx_protocols::{
    define_rpc,
    rpc::server::{ArgSpec, Proc, RpcCall},
    rpc_err,
};
use regex::Regex;
use std::{str::FromStr, sync::Arc};
use tokio::{sync::oneshot, task};

pub struct Rpcs {
    _reduce_inventory: Proc,
    _transfer_supply: Proc,
    _logistics_tick_now: Proc,
    _logistics_deliver_now: Proc,
    _repair: Proc,
    _tim: Proc,
    _spawn: Proc,
    _side_switch: Proc,
    _ban: Proc,
    _unban: Proc,
    _kick: Proc,
    _connected: Proc,
    _banned: Proc,
    _search: Proc,
    _log_warehouse: Proc,
    _reset_lives: Proc,
    _add_admin: Proc,
    _remove_admin: Proc,
    _balance: Proc,
    _set_points: Proc,
    _delete: Proc,
    _deslot: Proc,
    _remark: Proc,
    _reset: Proc,
    _shutdown: Proc,
}

async fn wait_task(mut ch: mpsc::Receiver<(RpcCall, oneshot::Receiver<Value>)>) {
    while let Some((mut c, ch)) = ch.next().await {
        match ch.await {
            Err(_) => c.reply.send(Value::Error("call failed".into())),
            Ok(v) => c.reply.send(v),
        }
    }
}

impl Rpcs {
    pub async fn new(
        publisher: &Publisher,
        q: &Arc<SegQueue<(AdminCommand, oneshot::Sender<Value>)>>,
        base: &Path,
    ) -> Result<Self> {
        let base = base.append("api");
        let (wait, rx) = mpsc::channel(10);
        task::spawn(wait_task(rx));
        let _q = Arc::clone(&q);
        let reduce_inventory = define_rpc!(
            publisher,
            base.append("reduce-inventory"),
            "Reduce inventory at an airfield",
            |c: RpcCall, airbase: Chars, amount: u8| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::ReduceInventory { airbase: airbase.as_ref().into(), amount };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            airbase: Chars = Value::Null; "The airbase to reduce",
            amount: u8 = Value::Null; "The amount, as a whole number percentage, to reduce"
        )?;
        let _q = Arc::clone(&q);
        let transfer_supply = define_rpc!(
            publisher,
            base.append("transfer-supply"),
            "Transfer supply from one objective to another",
            |c: RpcCall, from: Chars, to: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::TransferSupply { from: from.as_ref().into(), to: to.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            from: Chars = Value::Null; "The airbase to transfer supply from",
            to: Chars = Value::Null; "The airbase to transfer supply to"
        )?;
        let _q = Arc::clone(&q);
        let logistics_tick_now = define_rpc!(
            publisher,
            base.append("logistics-tick-now"),
            "Force a logistics tick to happen on the next timed events",
            |c: RpcCall, _: Value| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::LogisticsTickNow;
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            arg: Value = Value::Null; ""
        )?;
        let _q = Arc::clone(&q);
        let logistics_deliver_now = define_rpc!(
            publisher,
            base.append("logistics-deliver-now"),
            "Force a logistics delivery to happen on the next timed events",
            |c: RpcCall, _: Value| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::LogisticsDeliverNow;
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            arg: Value = Value::Null; ""
        )?;
        let _q = Arc::clone(&q);
        let repair = define_rpc!(
            publisher,
            base.append("repair"),
            "Repair one logistics group",
            |c: RpcCall, airbase: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Repair { airbase: airbase.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            airbase: Chars = Value::Null; "The airbase to repair"
        )?;
        let _q = Arc::clone(&q);
        let tim = define_rpc!(
            publisher,
            base.append("tim"),
            "Cause an explosion on the specified mark",
            |c: RpcCall, key: Chars, size: usize| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Tim { key: key.as_ref().into(), size };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            key: Chars = Value::Null; "The text in the mark you want to blow up",
            size: usize = 3000; "The size of the explosion in kg of TNT"
        )?;
        let _q = Arc::clone(&q);
        let spawn = define_rpc!(
            publisher,
            base.append("spawn"),
            "Spawn a group at the specified mark",
            |c: RpcCall, key: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Spawn { key: key.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            key: Chars = Value::Null; "The key of the mark you want to spawn"
        )?;
        let _q = Arc::clone(&q);
        let side_switch = define_rpc!(
            publisher,
            base.append("side-switch"),
            "Side switch a player",
            |mut c: RpcCall, player: Chars, side: Chars| {
                let (tx, rx) = oneshot::channel();
                let side = match Side::from_str(&side) {
                    Ok(side) => side,
                    Err(e) => {
                        c.reply.send(Value::Error(format!("{e:?}").into()));
                        return None
                    },
                };
                let cmd = AdminCommand::SideSwitch { side, player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The name of the player to switch",
            side: Chars = Value::Null; "The side to switch the player to"
        )?;
        let _q = Arc::clone(&q);
        let ban = define_rpc!(
            publisher,
            base.append("ban"),
            "Ban a player",
            |c: RpcCall, player: Chars, until: Option<DateTime<Utc>>| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Ban { player: player.as_ref().into(), until };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The name of the player to ban",
            until: Option<DateTime<Utc>> = Value::Null; "Optional end time of the ban"
        )?;
        let _q = Arc::clone(&q);
        let unban = define_rpc!(
            publisher,
            base.append("unban"),
            "Unban a player",
            |c: RpcCall, player: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Unban { player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The name of the player to unban"
        )?;
        let _q = Arc::clone(&q);
        let kick = define_rpc!(
            publisher,
            base.append("kick"),
            "Kick a player",
            |c: RpcCall, player: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Kick { player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The name of the player to kick"
        )?;
        let _q = Arc::clone(&q);
        let connected = define_rpc!(
            publisher,
            base.append("connected"),
            "List connected players",
            |c: RpcCall, _: Value| {
                let (tx, rx) = oneshot::channel();
                _q.push((AdminCommand::Connected, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            arg: Value = Value::Null; ""
        )?;
        let _q = Arc::clone(&q);
        let banned = define_rpc!(
            publisher,
            base.append("banned"),
            "List banned players",
            |c: RpcCall, _: Value| {
                let (tx, rx) = oneshot::channel();
                _q.push((AdminCommand::Banned, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            arg: Value = Value::Null; ""
        )?;
        let _q = Arc::clone(&q);
        let search = define_rpc!(
            publisher,
            base.append("search"),
            "Search players",
            |mut c: RpcCall, expr: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = match Regex::new(&expr) {
                    Ok(expr) => AdminCommand::Search { expr },
                    Err(e) => {
                        c.reply.send(Value::Error(format!("{e:?}").into()));
                        return None
                    }
                };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            expr: Chars = Value::Null; "The regular expression to search for"
        )?;
        let _q = Arc::clone(&q);
        let log_warehouse = define_rpc!(
            publisher,
            base.append("log-warehouse"),
            "Log the contents of the specified warehouse",
            |mut c: RpcCall, airbase: Chars, kind: Chars| {
                let (tx, rx) = oneshot::channel();
                let kind = match kind.as_ref() {
                    "Objective" => WarehouseKind::Objective,
                    "DCS" => WarehouseKind::DCS,
                    s => {
                        c.reply.send(Value::Error(format!("invalid objective kind {s}").into()));
                        return None
                    }
                };
                let cmd = AdminCommand::LogWarehouse { kind, airbase: airbase.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            airbase: Chars = Value::Null; "The airbase to log",
            kind: Chars = Value::Null; "The kind of warehouse to log (Objective or DCS)"
        )?;
        let _q = Arc::clone(&q);
        let reset_lives = define_rpc!(
            publisher,
            base.append("reset-lives"),
            "Reset the specified player's lives",
            |c: RpcCall, player: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::ResetLives { player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The player to reset"
        )?;
        let _q = Arc::clone(&q);
        let add_admin = define_rpc!(
            publisher,
            base.append("add-admin"),
            "Add player as an admin",
            |c: RpcCall, player: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::AddAdmin { player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The player to add"
        )?;
        let _q = Arc::clone(&q);
        let remove_admin = define_rpc!(
            publisher,
            base.append("remove-admin"),
            "Remove player as an admin",
            |c: RpcCall, player: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::RemoveAdmin { player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The player to remove"
        )?;
        let _q = Arc::clone(&q);
        let balance = define_rpc!(
            publisher,
            base.append("balance"),
            "Return a player's points balance",
            |c: RpcCall, player: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Balance { player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The player"
        )?;
        let _q = Arc::clone(&q);
        let set_points = define_rpc!(
            publisher,
            base.append("set-points"),
            "Set a player's points balance",
            |c: RpcCall, player: Chars, amount: i32| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::SetPoints { player: player.as_ref().into(), amount };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The player",
            amount: i32 = Value::Null; "The balance"
        )?;
        let _q = Arc::clone(&q);
        let delete = define_rpc!(
            publisher,
            base.append("delete-group"),
            "Delete a group",
            |c: RpcCall, group: i64| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Delete { group: GroupId::from(group) };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            group: i64 = Value::Null; "The id of the group to delete"
        )?;
        let _q = Arc::clone(&q);
        let deslot = define_rpc!(
            publisher,
            base.append("deslot"),
            "Deslot a player",
            |c: RpcCall, player: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Deslot { player: player.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            player: Chars = Value::Null; "The player to deslot"
        )?;
        let _q = Arc::clone(&q);
        let remark = define_rpc!(
            publisher,
            base.append("remark"),
            "Remark an objective",
            |c: RpcCall, objective: Chars| {
                let (tx, rx) = oneshot::channel();
                let cmd = AdminCommand::Remark { objective: objective.as_ref().into() };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            objective: Chars = Value::Null; "The objective to remark"
        )?;
        let _q = Arc::clone(&q);
        let reset = define_rpc!(
            publisher,
            base.append("reset"),
            "Reset the campaign",
            |mut c: RpcCall, winner: Option<Chars>| {
                let (tx, rx) = oneshot::channel();
                let cmd = match winner.map(|s| Side::from_str(&s)).transpose() {
                    Ok(winner) => AdminCommand::Reset { winner },
                    Err(e) => {
                        c.reply.send(Value::Error(format!("{e:?}").into()));
                        return None
                    }
                };
                _q.push((cmd, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            winner: Option<Chars> = Value::Null; "The winner, if any"
        )?;
        let _q = Arc::clone(&q);
        let shutdown = define_rpc!(
            publisher,
            base.append("shutdown"),
            "Shutdown the server",
            |c: RpcCall, _: Value| {
                let (tx, rx) = oneshot::channel();
                _q.push((AdminCommand::Shutdown, tx));
                Some((c, rx))
            },
            Some(wait.clone()),
            arg: Value = Value::Null; ""
        )?;
        Ok(Self {
            _reduce_inventory: reduce_inventory,
            _transfer_supply: transfer_supply,
            _logistics_tick_now: logistics_tick_now,
            _logistics_deliver_now: logistics_deliver_now,
            _repair: repair,
            _tim: tim,
            _spawn: spawn,
            _side_switch: side_switch,
            _ban: ban,
            _unban: unban,
            _kick: kick,
            _connected: connected,
            _banned: banned,
            _search: search,
            _log_warehouse: log_warehouse,
            _reset_lives: reset_lives,
            _add_admin: add_admin,
            _remove_admin: remove_admin,
            _balance: balance,
            _set_points: set_points,
            _delete: delete,
            _deslot: deslot,
            _remark: remark,
            _reset: reset,
            _shutdown: shutdown,
        })
    }
}
