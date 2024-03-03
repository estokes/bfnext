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

use super::{
    objective::{Objective, ObjectiveKind},
    persisted::Persisted,
};
use crate::{cfg::Cfg, msgq::MsgQ};
use compact_str::format_compact;
use dcso3::{
    coalition::Side,
    trigger::{ArrowSpec, CircleSpec, LineType, MarkId, RectSpec, SideFilter, TextSpec},
    Color, LuaVec3, Vector3,
};
use smallvec::SmallVec;

#[derive(Debug, Clone, Default)]
pub(super) struct ObjectiveMarkup {
    side: Side,
    threatened: bool,
    health: u8,
    logi: u8,
    supply: u8,
    fuel: u8,
    owner_ring: MarkId,
    capturable_ring: MarkId,
    threatened_ring: MarkId,
    name: MarkId,
    health_label: MarkId,
    healthbar: [MarkId; 5],
    logi_label: MarkId,
    logibar: [MarkId; 5],
    supply_label: MarkId,
    supplybar: [MarkId; 5],
    supply_connections: SmallVec<[MarkId; 8]>,
    fuel_label: MarkId,
    fuelbar: [MarkId; 5],
}

fn text_color(side: Side, a: f32) -> Color {
    match side {
        Side::Red => Color::red(a),
        Side::Blue => Color::blue(a),
        Side::Neutral => Color::white(a),
    }
}

impl ObjectiveMarkup {
    pub(super) fn remove(self, msgq: &mut MsgQ) {
        let ObjectiveMarkup {
            side: _,
            threatened: _,
            health: _,
            logi: _,
            supply: _,
            fuel: _,
            owner_ring,
            capturable_ring,
            threatened_ring,
            name,
            health_label,
            healthbar,
            logi_label,
            logibar,
            supply_label,
            supplybar,
            supply_connections,
            fuel_label,
            fuelbar,
        } = self;
        msgq.delete_mark(owner_ring);
        msgq.delete_mark(threatened_ring);
        msgq.delete_mark(capturable_ring);
        msgq.delete_mark(name);
        msgq.delete_mark(health_label);
        for id in healthbar {
            msgq.delete_mark(id)
        }
        msgq.delete_mark(logi_label);
        for id in logibar {
            msgq.delete_mark(id)
        }
        msgq.delete_mark(supply_label);
        for id in supplybar {
            msgq.delete_mark(id)
        }
        msgq.delete_mark(fuel_label);
        for id in fuelbar {
            msgq.delete_mark(id)
        }
        for id in supply_connections {
            msgq.delete_mark(id)
        }
    }

    pub(super) fn update(&mut self, msgq: &mut MsgQ, force: bool, obj: &Objective) {
        if obj.owner != self.side || force {
            let new_owner = obj.owner != self.side;
            let text_color = |a| text_color(obj.owner, a);
            self.side = obj.owner;
            msgq.set_markup_color(self.name, text_color(0.75));
            msgq.set_markup_color(self.owner_ring, text_color(1.));
            msgq.set_markup_color(self.health_label, text_color(0.75));
            msgq.set_markup_color(self.logi_label, text_color(0.75));
            msgq.set_markup_color(self.supply_label, text_color(0.75));
            msgq.set_markup_color(self.fuel_label, text_color(0.75));
            if new_owner {
                for id in self.supply_connections.drain(..) {
                    msgq.delete_mark(id);
                }
            }
        }
        if obj.threatened != self.threatened || force {
            self.threatened = obj.threatened;
            msgq.set_markup_color(
                self.threatened_ring,
                Color::yellow(if self.threatened { 0.75 } else { 0. }),
            );
        }
        macro_rules! update_bar {
            ($bar:ident, $field:ident) => {
                for (i, id) in self.$bar.iter().enumerate() {
                    let i = (i + 1) as u8;
                    let (a, ba) = if (i == 1 && obj.$field > 0) || (obj.$field / (i * 20)) > 0 {
                        (0.5, 1.)
                    } else {
                        (0., 0.25)
                    };
                    msgq.set_markup_fill_color(*id, Color::green(a));
                    msgq.set_markup_color(*id, Color::black(ba));
                }
            };
        }
        if self.health != obj.health || force {
            self.health = obj.health;
            update_bar!(healthbar, health);
        }
        if self.logi != obj.logi || force {
            self.logi = obj.logi;
            msgq.set_markup_color(
                self.capturable_ring,
                Color::white(if obj.captureable() { 0.75 } else { 0. }),
            );
            update_bar!(logibar, logi);
        }
        if self.supply != obj.supply || force {
            self.supply = obj.supply;
            update_bar!(supplybar, supply);
        }
        if self.fuel != obj.fuel || force {
            self.fuel = obj.fuel;
            update_bar!(fuelbar, fuel);
        }
        if force {
            for id in &self.supply_connections {
                msgq.set_markup_color(*id, Color::gray(0.5));
                msgq.set_markup_fill_color(*id, Color::gray(0.5));
            }
        }
    }

    pub(super) fn new(cfg: &Cfg, msgq: &mut MsgQ, obj: &Objective, persisted: &Persisted) -> Self {
        let text_color = |a| text_color(obj.owner, a);
        let all_spec = match obj.kind {
            ObjectiveKind::Airbase | ObjectiveKind::Fob | ObjectiveKind::Logistics => {
                SideFilter::All
            }
            ObjectiveKind::Farp { .. } => obj.owner.into(),
        };
        let bar_with_label = |msgq: &mut MsgQ,
                              pos3: Vector3,
                              label: MarkId,
                              text: &str,
                              marks: &[MarkId; 5],
                              val: u8| {
            msgq.text_to_all(
                all_spec,
                label,
                TextSpec {
                    pos: LuaVec3(Vector3::new(pos3.x + 200., 0., pos3.z)),
                    color: text_color(0.75),
                    fill_color: Color::black(0.),
                    font_size: 12,
                    read_only: true,
                    text: text.into(),
                },
            );
            for (i, id) in marks.iter().enumerate() {
                let j = (i + 1) as u8;
                let i = i as f64;
                let (a, ba) = if (i == 0. && val > 0) || (val / (j * 20)) > 0 {
                    (0.5, 1.)
                } else {
                    (0., 0.25)
                };
                msgq.rect_to_all(
                    all_spec,
                    *id,
                    RectSpec {
                        start: LuaVec3(Vector3::new(pos3.x, 0., pos3.z + i * 500.)),
                        end: LuaVec3(Vector3::new(pos3.x - 400., 0., pos3.z + i * 500. + 400.)),
                        color: Color::black(ba),
                        fill_color: Color::green(a),
                        line_type: LineType::Solid,
                        read_only: true,
                    },
                    None,
                );
            }
        };
        let mut t = ObjectiveMarkup::default();
        t.side = obj.owner;
        t.threatened = obj.threatened;
        t.health = obj.health;
        t.logi = obj.logi;
        t.supply = obj.supply;
        let mut pos3 = Vector3::new(obj.pos.x, 0., obj.pos.y);
        msgq.circle_to_all(
            all_spec,
            t.owner_ring,
            CircleSpec {
                center: LuaVec3(pos3),
                radius: obj.radius,
                color: text_color(1.),
                fill_color: Color::white(0.),
                line_type: LineType::Dashed,
                read_only: true,
            },
            None,
        );
        msgq.circle_to_all(
            all_spec,
            t.threatened_ring,
            CircleSpec {
                center: LuaVec3(pos3),
                radius: cfg.logistics_exclusion as f64,
                color: Color::yellow(if obj.threatened { 0.75 } else { 0. }),
                fill_color: Color::white(0.),
                line_type: LineType::Solid,
                read_only: true,
            },
            None,
        );
        msgq.circle_to_all(
            all_spec,
            t.capturable_ring,
            CircleSpec {
                center: LuaVec3(pos3),
                radius: obj.radius as f64 * 0.9,
                color: Color::white(if obj.captureable() { 0.75 } else { 0. }),
                fill_color: Color::white(0.),
                line_type: LineType::Solid,
                read_only: true,
            },
            None,
        );
        msgq.text_to_all(
            all_spec,
            t.name,
            TextSpec {
                pos: LuaVec3(Vector3::new(pos3.x + 1500., 1., pos3.z + 1500.)),
                color: text_color(1.),
                fill_color: Color::black(0.),
                font_size: 14,
                read_only: true,
                text: format_compact!("{} {}", obj.name, obj.kind.name()).into(),
            },
        );
        pos3.x += 5000.;
        pos3.z -= 5000.;
        bar_with_label(
            msgq,
            pos3,
            t.health_label,
            "Health",
            &t.healthbar,
            obj.health,
        );
        pos3.x -= 1500.;
        bar_with_label(msgq, pos3, t.logi_label, "Logi", &t.logibar, obj.logi);
        pos3.x -= 1500.;
        bar_with_label(
            msgq,
            pos3,
            t.supply_label,
            "Supply",
            &t.supplybar,
            obj.supply,
        );
        pos3.x -= 1500.;
        bar_with_label(msgq, pos3, t.fuel_label, "Fuel", &t.fuelbar, obj.fuel);
        match obj.kind {
            ObjectiveKind::Airbase | ObjectiveKind::Farp { .. } | ObjectiveKind::Fob => (),
            ObjectiveKind::Logistics => {
                let pos = obj.pos;
                for oid in &obj.warehouse.destination {
                    let id = MarkId::new();
                    let dobj = &persisted.objectives[oid];
                    let dir = (dobj.pos - pos).normalize();
                    let spos = pos + dir * obj.radius * 1.1;
                    let rdir = (pos - dobj.pos).normalize();
                    let dpos = dobj.pos + rdir * dobj.radius * 1.1;
                    msgq.arrow_to(
                        if dobj.is_farp() {
                            dobj.owner.into()
                        } else {
                            all_spec
                        },
                        id,
                        ArrowSpec {
                            start: LuaVec3(Vector3::new(dpos.x, 0., dpos.y)),
                            end: LuaVec3(Vector3::new(spos.x, 0., spos.y)),
                            color: Color::gray(0.5),
                            fill_color: Color::gray(0.5),
                            line_type: LineType::NoLine,
                            read_only: true,
                        },
                        None,
                    );
                    t.supply_connections.push(id);
                }
            }
        }
        t
    }
}
