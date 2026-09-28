//! Bottom-left card with the current tool's settings: radius and strength
//! for area tools, thickness / colour / pinning for Draw, and the kind of
//! link for Link.

use super::spawner::spawn_color;
use super::theme::*;
use super::widgets::*;
use super::{icons, Action, Input};
use crate::config::{PPM, WALL_T};
use crate::physics::links::LinkKind;
use crate::physics::tools::Card;
use crate::settings::{Settings, DRAW_THICKNESS_RANGE, RADIUS_RANGE, STRENGTH_RANGE};
use crate::shapes::PALETTE;
use crate::util::approach;
use macroquad::prelude::*;

const W: f32 = 268.0;

pub struct ToolCard {
    shown: f32,
    /// Card kept on screen while sliding out.
    card: Card,
    radius: SliderState,
    strength: SliderState,
    thickness: SliderState,
}

impl Default for ToolCard {
    fn default() -> Self {
        ToolCard {
            shown: 0.0,
            card: Card::None,
            radius: SliderState::default(),
            strength: SliderState::default(),
            thickness: SliderState::default(),
        }
    }
}

struct Layout {
    panel: Rect,
    head: Rect,
    /// Area: radius, strength. Draw: thickness.
    sliders: Vec<Rect>,
    swatches: Vec<Rect>,
    pin: Option<Rect>,
    kinds: Vec<Rect>,
}

fn height(card: Card) -> f32 {
    match card {
        Card::Area => 132.0,
        Card::Draw => 176.0,
        Card::Link => 150.0,
        Card::None => 0.0,
    }
}

impl ToolCard {
    fn layout(&self) -> Layout {
        let sh = screen_height();
        let h = height(self.card);
        let x = 10.0 - (1.0 - self.shown) * (W + 20.0);
        let panel = Rect::new(x, sh - WALL_T * PPM - 10.0 - h, W, h);
        let head = Rect::new(panel.x + 12.0, panel.y + 10.0, panel.w - 24.0, 36.0);
        let inner = |y: f32, h: f32| Rect::new(panel.x + 14.0, panel.y + y, panel.w - 28.0, h);
        let mut l = Layout { panel, head, sliders: vec![], swatches: vec![], pin: None, kinds: vec![] };
        match self.card {
            Card::Area => {
                l.sliders.push(inner(48.0, SLIDER_ROW_H));
                l.sliders.push(inner(48.0 + SLIDER_ROW_H + 2.0, SLIDER_ROW_H));
            }
            Card::Draw => {
                l.sliders.push(inner(48.0, SLIDER_ROW_H));
                let row = inner(92.0, 20.0);
                let n = PALETTE.len() + 1;
                let sw = (row.w - (n as f32 - 1.0) * 5.0) / n as f32;
                l.swatches = (0..n).map(|i| Rect::new(row.x + i as f32 * (sw + 5.0), row.y, sw, row.h)).collect();
                l.pin = Some(inner(122.0, 30.0));
            }
            Card::Link => {
                let row = inner(54.0, 50.0);
                let n = LinkKind::ALL.len();
                let bw = (row.w - (n as f32 - 1.0) * 6.0) / n as f32;
                l.kinds = (0..n).map(|i| Rect::new(row.x + i as f32 * (bw + 6.0), row.y, bw, row.h)).collect();
            }
            Card::None => {}
        }
        l
    }

    pub fn update(&mut self, dt: f32, s: &mut Settings, input: &mut Input, actions: &mut Vec<Action>) {
        let want = s.tool.card();
        if want != Card::None && (want == self.card || self.shown < 0.05) {
            self.card = want;
        }
        let show = want != Card::None && want == self.card;
        self.shown = approach(self.shown, if show { 1.0 } else { 0.0 }, 12.0, dt);
        if self.shown < 0.5 || !show {
            self.radius.dragging = false;
            self.strength.dragging = false;
            self.thickness.dragging = false;
            return;
        }
        let l = self.layout();
        match self.card {
            Card::Area => {
                self.radius.update(l.sliders[0], &mut s.tool_radius, RADIUS_RANGE.0, RADIUS_RANGE.1, input);
                self.strength.update(l.sliders[1], &mut s.tool_strength, STRENGTH_RANGE.0, STRENGTH_RANGE.1, input);
            }
            Card::Draw => {
                let (lo, hi) = DRAW_THICKNESS_RANGE;
                self.thickness.update(l.sliders[0], &mut s.draw_thickness, lo, hi, input);
                for (i, r) in l.swatches.iter().enumerate() {
                    if button(*r, input) {
                        s.spawn_color = i;
                    }
                }
                if l.pin.is_some_and(|r| toggle_row(r, input)) {
                    s.draw_pinned = !s.draw_pinned;
                }
            }
            Card::Link => {
                for (i, r) in l.kinds.iter().enumerate() {
                    if button(*r, input) {
                        s.link_kind = LinkKind::ALL[i];
                    }
                }
            }
            Card::None => {}
        }
        if button(l.head, input) {
            actions.push(Action::OpenToolPicker);
        }
        input.block(l.panel);
    }

    pub fn dragging(&self) -> bool {
        self.radius.dragging || self.strength.dragging || self.thickness.dragging
    }

    pub fn draw(&self, s: &Settings, mouse: Vec2) {
        if self.shown < 0.01 || self.card == Card::None {
            return;
        }
        let f = self.shown;
        let l = self.layout();
        panel(l.panel, f);
        let head = l.head;
        let accent = s.tool.accent();
        let t = get_time() as f32;

        if head.contains(mouse) {
            rrect(Rect::new(head.x - 6.0, head.y - 2.0, head.w + 12.0, head.h + 4.0), 8.0, fade(SURFACE_HI, f * 0.6));
        }
        let ic = Rect::new(head.x, head.y, 36.0, 36.0);
        rrect(ic, 9.0, fade(alpha(accent, 0.16), f));
        icons::tool(s.tool, vec2(ic.x + 18.0, ic.y + 18.0), 26.0, fade(accent, f), t);
        text_bold(s.tool.label(), head.x + 46.0, head.y + 15.0, 15.0, fade(TEXT, f));
        text(s.tool.description(), head.x + 46.0, head.y + 31.0, 12.0, fade(TEXT_MUTED, f));
        keycap(head.x + head.w - 30.0, head.y + 11.0, "Tab", 10.0, f * 0.8);

        match self.card {
            Card::Area => {
                let radius = SliderSpec {
                    label: "Radius",
                    value_text: format!("{} px", s.tool_radius as i32),
                    min: RADIUS_RANGE.0,
                    max: RADIUS_RANGE.1,
                    accent,
                };
                let strength = SliderSpec {
                    label: "Strength",
                    value_text: format!("{:.0}", s.tool_strength),
                    min: STRENGTH_RANGE.0,
                    max: STRENGTH_RANGE.1,
                    accent,
                };
                let (a, b) = (l.sliders[0], l.sliders[1]);
                draw_slider(a, s.tool_radius, &radius, a.contains(mouse) || self.radius.dragging, f);
                draw_slider(b, s.tool_strength, &strength, b.contains(mouse) || self.strength.dragging, f);
            }
            Card::Draw => {
                let thick = SliderSpec {
                    label: "Thickness",
                    value_text: format!("{} px", s.draw_thickness as i32),
                    min: DRAW_THICKNESS_RANGE.0,
                    max: DRAW_THICKNESS_RANGE.1,
                    accent,
                };
                let a = l.sliders[0];
                draw_slider(a, s.draw_thickness, &thick, a.contains(mouse) || self.thickness.dragging, f);
                for (i, r) in l.swatches.iter().enumerate() {
                    rrect(*r, 5.0, fade(spawn_color(i, t), f));
                    if i == PALETTE.len() {
                        text_centered(
                            "?",
                            r.x + r.w / 2.0,
                            r.y + r.h / 2.0,
                            11.0,
                            fade(Color::new(0.1, 0.1, 0.1, 0.8), f),
                        );
                    }
                    if i == s.spawn_color {
                        rrect_lines(Rect::new(r.x - 2.5, r.y - 2.5, r.w + 5.0, r.h + 5.0), 7.0, 1.8, fade(TEXT, f));
                    }
                }
                if let Some(r) = l.pin {
                    draw_toggle_row(r, "Pin drawings  (Shift inverts)", s.draw_pinned, r.contains(mouse), f);
                }
            }
            Card::Link => {
                for (i, r) in l.kinds.iter().enumerate() {
                    let kind = LinkKind::ALL[i];
                    let active = kind == s.link_kind;
                    let hov = r.contains(mouse);
                    let bg = if active {
                        mix(SURFACE_2, kind.accent(), 0.2)
                    } else if hov {
                        SURFACE_HI
                    } else {
                        SURFACE_2
                    };
                    rrect(*r, 8.0, fade(bg, f));
                    rrect_lines(*r, 8.0, 1.0, fade(if active { kind.accent() } else { BORDER }, f));
                    let col = if active { kind.accent() } else { TEXT_DIM };
                    icons::link_kind(kind, vec2(r.x + r.w / 2.0, r.y + 18.0), 24.0, fade(col, f));
                    text_centered(kind.label(), r.x + r.w / 2.0, r.y + 39.0, 12.0, fade(TEXT, f));
                }
                let r = l.panel;
                for (i, line) in s.link_kind.hint().iter().enumerate() {
                    let y = r.y + r.h - 30.0 + i as f32 * 15.0;
                    text_centered(line, r.x + r.w / 2.0, y, 11.0, fade(TEXT_MUTED, f));
                }
            }
            Card::None => {}
        }
    }
}
