use crate::admin::AdminCommand;
use anyhow::Result;
use arcstr::ArcStr;
use crossbeam::queue::SegQueue;
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
use std::sync::Arc;
use tokio::{sync::oneshot, task};

pub struct Rpcs {
    reduce_inventory: Proc,
    transfer_supply: Proc,
    logistics_tick_now: Proc,
    logistics_deliver_now: Proc,
    repair: Proc,
    tim: Proc,
    spawn: Proc,
    side_switch: Proc,
    ban: Proc,
    unban: Proc,
    kick: Proc,
    connected: Proc,
    banned: Proc,
    search: Proc,
    log_warehouse: Proc,
    reset_lives: Proc,
    add_admin: Proc,
    remove_admin: Proc,
    balance: Proc,
    set_points: Proc,
    delete: Proc,
    deslot: Proc,
    remark: Proc,
    reset: Proc,
    shutdown: Proc,
    create_mark: Proc,
    delete_mark: Proc,
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
        base: Path,
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
        );
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
        );
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
        );
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
        );
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
        );
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
        );
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
        );
        unimplemented!()
    }
}
