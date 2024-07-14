/*
Copyright 2024 Eric Stokes.

This file is part of bflib.

bflib is free software: you can redistribute it and/or modify it under
the terms of the GNU Affero Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your
option) any later version.

bflib is distributed in the hope that it will be useful, but WITHOUT
ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE. See the GNU Affero Public License
for more details.
*/

use dcso3::{
    coalition::Side,
    env::miz::{GroupId, UnitId},
    net::{Net, PlayerId},
    trigger::{Action, ArrowSpec, CircleSpec, MarkId, RectSpec, SideFilter, TextSpec},
    Color, LuaVec3, String, Vector2, Vector3,
};
use log::error;
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy)]
pub enum PanelDest {
    All,
    Side(Side),
    Group(GroupId),
    Unit(UnitId),
}

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub enum MarkDest {
    All,
    Side(Side),
    Group(GroupId),
}

#[derive(Debug, Clone)]
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
#[allow(dead_code)]
pub enum Msg {
    Message {
        typ: MsgTyp,
        text: String,
    },
    Circle {
        id: MarkId,
        to: SideFilter,
        spec: CircleSpec,
        message: Option<String>,
    },
    Rect {
        id: MarkId,
        to: SideFilter,
        spec: RectSpec,
        message: Option<String>,
    },
    Text {
        id: MarkId,
        to: SideFilter,
        spec: TextSpec,
    },
    Arrow {
        id: MarkId,
        to: SideFilter,
        spec: ArrowSpec,
        message: Option<String>,
    },
    SetMarkupColor {
        id: MarkId,
        color: Color,
    },
    SetMarkupFillColor {
        id: MarkId,
        color: Color,
    },
    SetMarkupText {
        id: MarkId,
        text: String,
    },
}

#[derive(Debug, Clone)]
pub enum Cmd {
    Send(Msg),
    DeleteMark(MarkId),
}

#[derive(Debug, Clone)]
pub struct MsgQ(Vec<VecDeque<Cmd>>);

impl Default for MsgQ {
    fn default() -> Self {
        MsgQ(vec![
            VecDeque::default(),
            VecDeque::default(),
            VecDeque::default(),
        ])
    }
}

impl MsgQ {
    fn send_with_priority<S: Into<String>>(&mut self, p: usize, typ: MsgTyp, text: S) {
        self.0[p].push_back(Cmd::Send(Msg::Message {
            typ,
            text: text.into(),
        }))
    }

    pub fn send<S: Into<String>>(&mut self, typ: MsgTyp, text: S) {
        self.send_with_priority(0, typ, text)
    }

    pub fn delete_mark(&mut self, did: MarkId) {
        let mut push = true;
        let mut remove = |pri: usize| {
            self.0[pri].retain(|cmd| match cmd {
                Cmd::DeleteMark(_) => true,
                Cmd::Send(msg) => match msg {
                    Msg::Message { .. } => true,
                    Msg::Circle { id, .. }
                    | Msg::Rect { id, .. }
                    | Msg::Text { id, .. }
                    | Msg::Arrow { id, .. } => {
                        if *id == did {
                            push = false;
                            false
                        } else {
                            true
                        }
                    }
                    Msg::SetMarkupColor { id, .. }
                    | Msg::SetMarkupFillColor { id, .. }
                    | Msg::SetMarkupText { id, .. } => *id != did,
                },
            })
        };
        remove(0);
        remove(1);
        remove(2);
        if push {
            self.0[1].push_back(Cmd::DeleteMark(did))
        }
    }

    #[allow(dead_code)]
    pub fn mark_to_all<S: Into<String>>(
        &mut self,
        position: Vector2,
        read_only: bool,
        text: S,
    ) -> MarkId {
        let id = MarkId::new();
        self.send_with_priority(
            1,
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
        self.send_with_priority(
            1,
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

    #[allow(dead_code)]
    pub fn mark_to_group<S: Into<String>>(
        &mut self,
        group: GroupId,
        position: Vector2,
        read_only: bool,
        text: S,
    ) -> MarkId {
        let id = MarkId::new();
        self.send_with_priority(
            1,
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
        self.send_with_priority(
            0,
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
        self.send_with_priority(
            0,
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
        self.send_with_priority(
            0,
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
        self.send_with_priority(
            0,
            MsgTyp::Panel {
                to: PanelDest::Unit(unit),
                display_time,
                clear_view,
            },
            text,
        )
    }

    pub fn circle_to_all(
        &mut self,
        to: SideFilter,
        id: MarkId,
        spec: CircleSpec,
        message: Option<String>,
    ) {
        self.0[2].push_back(Cmd::Send(Msg::Circle {
            id,
            to,
            spec,
            message,
        }))
    }

    #[allow(dead_code)]
    pub fn rect_to_all(
        &mut self,
        to: SideFilter,
        id: MarkId,
        spec: RectSpec,
        message: Option<String>,
    ) {
        self.0[2].push_back(Cmd::Send(Msg::Rect {
            id,
            to,
            spec,
            message,
        }))
    }

    pub fn text_to_all(&mut self, to: SideFilter, id: MarkId, spec: TextSpec) {
        self.0[1].push_back(Cmd::Send(Msg::Text { id, to, spec }))
    }

    pub fn arrow_to(
        &mut self,
        to: SideFilter,
        id: MarkId,
        spec: ArrowSpec,
        message: Option<String>,
    ) {
        self.0[2].push_back(Cmd::Send(Msg::Arrow {
            id,
            to,
            spec,
            message,
        }))
    }

    pub fn set_markup_color(&mut self, id: MarkId, color: Color) {
        self.0[2].push_back(Cmd::Send(Msg::SetMarkupColor { id, color }))
    }

    #[allow(dead_code)]
    pub fn set_markup_fill_color(&mut self, id: MarkId, color: Color) {
        self.0[2].push_back(Cmd::Send(Msg::SetMarkupFillColor { id, color }))
    }

    pub fn set_markup_text(&mut self, id: MarkId, text: String) {
        self.0[2].push_back(Cmd::Send(Msg::SetMarkupText { id, text }))
    }

    pub fn process(&mut self, max_rate: usize, net: &Net, act: &Action) {
        for _ in 0..max_rate {
            let cmd = match self.0[0].pop_front() {
                Some(cmd) => cmd,
                None => match self.0[1].pop_front() {
                    Some(cmd) => cmd,
                    None => match self.0[2].pop_front() {
                        Some(cmd) => cmd,
                        None => return,
                    },
                },
            };
            let res = match cmd {
                Cmd::DeleteMark(id) => act.remove_mark(id),
                Cmd::Send(Msg::Message { typ, text }) => match typ {
                    MsgTyp::Mark {
                        id,
                        to,
                        position,
                        read_only,
                    } => match to {
                        MarkDest::All => act.mark_to_all(id, text, position, read_only, None),
                        MarkDest::Side(side) => {
                            act.mark_to_coalition(id, text, position, side, read_only, None)
                        }
                        MarkDest::Group(group) => {
                            act.mark_to_group(id, text, position, group, read_only, None)
                        }
                    },
                    MsgTyp::Chat(to) => match to {
                        None => net.send_chat(text, true),
                        Some(id) => net.send_chat_to(text, id, Some(PlayerId::from(1))),
                    },
                    MsgTyp::Panel {
                        to,
                        display_time,
                        clear_view,
                    } => match to {
                        PanelDest::All => act.out_text(text, display_time, clear_view),
                        PanelDest::Group(gid) => {
                            act.out_text_for_group(gid, text, display_time, clear_view)
                        }
                        PanelDest::Side(side) => {
                            act.out_text_for_coalition(side, text, display_time, clear_view)
                        }
                        PanelDest::Unit(uid) => {
                            act.out_text_for_unit(uid, text, display_time, clear_view)
                        }
                    },
                },
                Cmd::Send(Msg::Circle {
                    id,
                    to,
                    spec,
                    message,
                }) => act.circle_to_all(to, id, spec, message),
                Cmd::Send(Msg::Rect {
                    id,
                    to,
                    spec,
                    message,
                }) => act.rect_to_all(to, id, spec, message),
                Cmd::Send(Msg::Text { id, to, spec }) => act.text_to_all(to, id, spec),
                Cmd::Send(Msg::Arrow {
                    id,
                    to,
                    spec,
                    message,
                }) => act.arrow_to_all(to, id, spec, message),
                Cmd::Send(Msg::SetMarkupColor { id, color }) => act.set_markup_color(id, color),
                Cmd::Send(Msg::SetMarkupFillColor { id, color }) => {
                    act.set_markup_fill_color(id, color)
                }
                Cmd::Send(Msg::SetMarkupText { id, text }) => act.set_markup_text(id, text),
            };
            if let Err(e) = res {
                error!("could not send message {:?}", e)
            }
        }
    }
}
