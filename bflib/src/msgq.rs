use dcso3::{
    coalition::Side,
    env::miz::{GroupId, UnitId},
    net::{Net, PlayerId},
    trigger::{Action, MarkId},
    LuaVec3, String, Vector2, Vector3,
};
use log::{debug, error};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy)]
pub enum PanelDest {
    All,
    Side(Side),
    Group(GroupId),
    Unit(UnitId),
}

#[derive(Debug, Clone, Copy)]
pub enum MarkDest {
    All,
    Side(Side),
    Group(GroupId),
}

#[derive(Debug, Clone, Copy)]
pub enum MsgTyp {
    Chat(Option<PlayerId>),
    Panel {
        to: PanelDest,
        display_time: i64,
        clear_view: bool,
    },
    Mark {
        id: MarkId,
        to: MarkDest,
        position: LuaVec3,
        read_only: bool,
    },
}

#[derive(Debug, Clone)]
pub struct Msg {
    pub typ: MsgTyp,
    pub text: String,
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Send(Msg),
    DeleteMark(MarkId),
}

#[derive(Debug, Clone, Default)]
pub struct MsgQ(VecDeque<Cmd>);

impl MsgQ {
    pub fn send<S: Into<String>>(&mut self, typ: MsgTyp, text: S) {
        self.0.push_back(Cmd::Send(Msg {
            typ,
            text: text.into(),
        }))
    }

    pub fn delete_mark(&mut self, id: MarkId) {
        self.0.push_back(Cmd::DeleteMark(id))
    }

    pub fn mark_to_all<S: Into<String>>(
        &mut self,
        position: Vector2,
        read_only: bool,
        text: S,
    ) -> MarkId {
        let id = MarkId::new();
        self.send(
            MsgTyp::Mark {
                id,
                to: MarkDest::All,
                position: LuaVec3(Vector3::new(position.x, 0., position.y)),
                read_only,
            },
            text,
        );
        id
    }

    pub fn mark_to_side<S: Into<String>>(
        &mut self,
        side: Side,
        position: Vector2,
        read_only: bool,
        text: S,
    ) -> MarkId {
        let id = MarkId::new();
        self.send(
            MsgTyp::Mark {
                id,
                to: MarkDest::Side(side),
                position: LuaVec3(Vector3::new(position.x, 0., position.y)),
                read_only,
            },
            text,
        );
        id
    }

    pub fn mark_to_group<S: Into<String>>(
        &mut self,
        group: GroupId,
        position: Vector2,
        read_only: bool,
        text: S,
    ) -> MarkId {
        let id = MarkId::new();
        self.send(
            MsgTyp::Mark {
                id,
                to: MarkDest::Group(group),
                position: LuaVec3(Vector3::new(position.x, 0., position.y)),
                read_only,
            },
            text,
        );
        id
    }

    #[allow(dead_code)]
    pub fn panel_to_all<S: Into<String>>(&mut self, display_time: i64, clear_view: bool, text: S) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::All,
                display_time,
                clear_view,
            },
            text,
        )
    }

    pub fn panel_to_side<S: Into<String>>(
        &mut self,
        display_time: i64,
        clear_view: bool,
        side: Side,
        text: S,
    ) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::Side(side),
                display_time,
                clear_view,
            },
            text,
        )
    }

    pub fn panel_to_group<S: Into<String>>(
        &mut self,
        display_time: i64,
        clear_view: bool,
        group: GroupId,
        text: S,
    ) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::Group(group),
                display_time,
                clear_view,
            },
            text,
        )
    }

    pub fn panel_to_unit<S: Into<String>>(
        &mut self,
        display_time: i64,
        clear_view: bool,
        unit: UnitId,
        text: S,
    ) {
        self.send(
            MsgTyp::Panel {
                to: PanelDest::Unit(unit),
                display_time,
                clear_view,
            },
            text,
        )
    }

    pub fn process(&mut self, net: &Net, act: &Action) {
        for _ in 0..5 {
            let cmd = match self.0.pop_front() {
                Some(cmd) => cmd,
                None => return,
            };
            debug!("server sending {:?}", cmd);
            let res = match cmd {
                Cmd::DeleteMark(id) => act.remove_mark(id),
                Cmd::Send(msg) => match msg.typ {
                    MsgTyp::Mark {
                        id,
                        to,
                        position,
                        read_only,
                    } => match to {
                        MarkDest::All => act.mark_to_all(id, msg.text, position, read_only, None),
                        MarkDest::Side(side) => {
                            act.mark_to_coalition(id, msg.text, position, side, read_only, None)
                        }
                        MarkDest::Group(group) => {
                            act.mark_to_group(id, msg.text, position, group, read_only, None)
                        }
                    },
                    MsgTyp::Chat(to) => match to {
                        None => net.send_chat(msg.text, true),
                        Some(id) => net.send_chat_to(msg.text, id, Some(PlayerId::from(1))),
                    },
                    MsgTyp::Panel {
                        to,
                        display_time,
                        clear_view,
                    } => match to {
                        PanelDest::All => act.out_text(msg.text, display_time, clear_view),
                        PanelDest::Group(gid) => {
                            act.out_text_for_group(gid, msg.text, display_time, clear_view)
                        }
                        PanelDest::Side(side) => {
                            act.out_text_for_coalition(side, msg.text, display_time, clear_view)
                        }
                        PanelDest::Unit(uid) => {
                            act.out_text_for_unit(uid, msg.text, display_time, clear_view)
                        }
                    },
                },
            };
            if let Err(e) = res {
                error!("could not send message {:?}", e)
            }
        }
    }
}
