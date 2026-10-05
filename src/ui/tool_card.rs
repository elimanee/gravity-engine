//! Bottom-left card with the current tool's settings: radius and strength
//! for area tools, thickness / colour / pinning for Draw, the kind of link
//! (and motor speed) for Link, the kind of zone for Zone, the selection
//! commands for Select, and the gadget, its trigger and settings for Gadget.

use super::spawner::spawn_color;
use super::theme::*;
use super::widgets::*;
use super::{icons, Action, Input, SelectionCmd};
use crate::config::{PPM, WALL_T};
use crate::physics::gadgets::{Ammo, GadgetKind, Trigger};
use crate::physics::grains::GrainKind;
use crate::physics::links::LinkKind;
use crate::physics::tools::Card;
use crate::physics::zones::ZoneKind;
use crate::settings::*;
use crate::shapes::PALETTE;
use crate::util::approach;
use macroquad::prelude::*;

const W: f32 = 268.0;
/// Wind directions offered, in degrees.
const DIRECTIONS: [f32; 4] = [180.0, 90.0, 270.0, 0.0];

#[derive(Clone, Copy, PartialEq)]
enum SliderId {
    Radius,
    Strength,
    Thickness,
    MotorSpeed,
    ZoneStrength,
    Thrust,
    CannonSpeed,
    CannonRate,
}

pub struct ToolCard {
    shown: f32,
    /// Card kept on screen while sliding out.
    card: Card,
    sliders: [SliderState; 8],
    /// Objects currently selected (set by the app each frame).
    pub selected: usize,
    /// Grains in the world (set by the app each frame).
    pub grains: usize,
}

impl Default for ToolCard {
    fn default() -> Self {
        ToolCard { shown: 0.0, card: Card::None, sliders: Default::default(), selected: 0, grains: 0 }
    }
}

/// Select-card buttons, two rows of three.
const SELECT_CMDS: [(SelectionCmd, &str); 6] = [
    (SelectionCmd::Duplicate, "Duplicate"),
    (SelectionCmd::Delete, "Delete"),
    (SelectionCmd::TogglePin, "Pin"),
    (SelectionCmd::Glue, "Glue"),
    (SelectionCmd::Copy, "Copy"),
    (SelectionCmd::Paste, "Paste"),
];

#[derive(Default)]
struct Layout {
    panel: Rect,
    head: Rect,
    sliders: Vec<(SliderId, Rect)>,
    swatches: Vec<Rect>,
    pin: Option<Rect>,
    /// Kind buttons (link or zone kinds).
    kinds: Vec<Rect>,
    directions: Vec<Rect>,
    /// Select card: selection count line and command buttons.
    info: Option<Rect>,
    commands: Vec<Rect>,
    /// Pour card: "clear grains" button.
    clear: Option<Rect>,
    /// Link card: "drive the motor with the arrows" switch.
    drive: Option<Rect>,
    /// Gadget card: trigger and ammunition buttons.
    triggers: Vec<Rect>,
    ammo: Vec<Rect>,
    /// Two hint lines at the bottom.
    hint: Option<[&'static str; 2]>,
}

fn row_of(r: Rect, n: usize, gap: f32) -> Vec<Rect> {
    let w = (r.w - (n as f32 - 1.0) * gap) / n as f32;
    (0..n).map(|i| Rect::new(r.x + i as f32 * (w + gap), r.y, w, r.h)).collect()
}

impl ToolCard {
    fn layout(&self, s: &Settings) -> Layout {
        let sh = screen_height();
        let mut l = Layout::default();
        // Rows are stacked from y = 48 down; the panel height follows.
        let mut y = 48.0;
        let mut rows: Vec<(f32, f32)> = vec![];
        let mut add = |h: f32, gap: f32| {
            y += gap;
            rows.push((y, h));
            y += h;
        };
        match self.card {
            Card::Area => {
                add(SLIDER_ROW_H, 0.0);
                add(SLIDER_ROW_H, 2.0);
            }
            Card::Draw => {
                add(SLIDER_ROW_H, 0.0);
                add(20.0, 6.0);
                add(30.0, 10.0);
            }
            Card::Link => {
                add(50.0, 6.0);
                if s.link_kind == LinkKind::Motor {
                    add(SLIDER_ROW_H, 8.0);
                    add(28.0, 4.0);
                }
                add(30.0, 6.0);
            }
            Card::Gadget => {
                add(50.0, 6.0);
                add(26.0, 18.0);
                match s.gadget_kind {
                    GadgetKind::Thruster => add(SLIDER_ROW_H, 8.0),
                    GadgetKind::Cannon => {
                        add(SLIDER_ROW_H, 8.0);
                        add(SLIDER_ROW_H, 2.0);
                        add(26.0, 6.0);
                    }
                    GadgetKind::Laser => {}
                }
                add(30.0, 6.0);
            }
            Card::Zone => {
                add(50.0, 6.0);
                match s.zone_kind {
                    ZoneKind::Wind => {
                        add(26.0, 10.0);
                        add(SLIDER_ROW_H, 6.0);
                    }
                    ZoneKind::Float => add(SLIDER_ROW_H, 8.0),
                    _ => {}
                }
                add(30.0, 6.0);
            }
            Card::Pour => {
                add(50.0, 6.0);
                add(28.0, 8.0);
                add(30.0, 6.0);
            }
            Card::Select => {
                add(18.0, 4.0);
                add(28.0, 8.0);
                add(28.0, 6.0);
                add(30.0, 6.0);
            }
            Card::None => {}
        }
        let h = y + 8.0;
        let x = 10.0 - (1.0 - self.shown) * (W + 20.0);
        l.panel = Rect::new(x, sh - WALL_T * PPM - 10.0 - h, W, h);
        l.head = Rect::new(l.panel.x + 12.0, l.panel.y + 10.0, l.panel.w - 24.0, 36.0);
        let inner = |(ry, rh): (f32, f32)| Rect::new(l.panel.x + 14.0, l.panel.y + ry, l.panel.w - 28.0, rh);
        let mut rows = rows.into_iter().map(inner);
        let mut next = || rows.next().unwrap_or_default();
        match self.card {
            Card::Area => {
                l.sliders.push((SliderId::Radius, next()));
                l.sliders.push((SliderId::Strength, next()));
            }
            Card::Draw => {
                l.sliders.push((SliderId::Thickness, next()));
                l.swatches = row_of(next(), PALETTE.len() + 1, 5.0);
                l.pin = Some(next());
            }
            Card::Link => {
                l.kinds = row_of(next(), LinkKind::ALL.len(), 5.0);
                if s.link_kind == LinkKind::Motor {
                    l.sliders.push((SliderId::MotorSpeed, next()));
                    l.drive = Some(next());
                }
                next();
                l.hint = Some(s.link_kind.hint());
            }
            Card::Gadget => {
                l.kinds = row_of(next(), GadgetKind::ALL.len(), 6.0);
                l.triggers = row_of(next(), Trigger::ALL.len(), 4.0);
                match s.gadget_kind {
                    GadgetKind::Thruster => l.sliders.push((SliderId::Thrust, next())),
                    GadgetKind::Cannon => {
                        l.sliders.push((SliderId::CannonSpeed, next()));
                        l.sliders.push((SliderId::CannonRate, next()));
                        l.ammo = row_of(next(), Ammo::ALL.len(), 4.0);
                    }
                    GadgetKind::Laser => {}
                }
                next();
                l.hint = Some(s.gadget_kind.hint());
            }
            Card::Zone => {
                l.kinds = row_of(next(), ZoneKind::TOOL.len(), 6.0);
                match s.zone_kind {
                    ZoneKind::Wind => {
                        l.directions = row_of(next(), DIRECTIONS.len(), 6.0);
                        l.sliders.push((SliderId::ZoneStrength, next()));
                    }
                    ZoneKind::Float => l.sliders.push((SliderId::ZoneStrength, next())),
                    _ => {}
                }
                next();
                l.hint = Some(s.zone_kind.hint());
            }
            Card::Pour => {
                l.kinds = row_of(next(), GrainKind::ALL.len(), 6.0);
                l.clear = Some(next());
                next();
                l.hint = Some(s.grain_kind.hint());
            }
            Card::Select => {
                l.info = Some(next());
                l.commands = row_of(next(), 3, 6.0);
                l.commands.extend(row_of(next(), 3, 6.0));
                next();
                l.hint = Some([
                    "Drag to move  ·  Shift+click adds or removes",
                    "Ctrl+C / Ctrl+V  ·  Ctrl+D duplicates  ·  Del",
                ]);
            }
            Card::None => {}
        }
        l
    }

    fn slider(id: SliderId, s: &mut Settings) -> (&mut f32, (f32, f32)) {
        match id {
            SliderId::Radius => (&mut s.tool_radius, RADIUS_RANGE),
            SliderId::Strength => (&mut s.tool_strength, STRENGTH_RANGE),
            SliderId::Thickness => (&mut s.draw_thickness, DRAW_THICKNESS_RANGE),
            SliderId::MotorSpeed => (&mut s.motor_speed, MOTOR_SPEED_RANGE),
            SliderId::ZoneStrength => (&mut s.zone_strength, ZONE_STRENGTH_RANGE),
            SliderId::Thrust => (&mut s.thrust, THRUST_RANGE),
            SliderId::CannonSpeed => (&mut s.cannon_speed, CANNON_SPEED_RANGE),
            SliderId::CannonRate => (&mut s.cannon_rate, CANNON_RATE_RANGE),
        }
    }

    fn spec(id: SliderId, s: &Settings, accent: Color) -> (SliderSpec<'static>, f32) {
        let (label, value, text, range) = match id {
            SliderId::Radius => ("Radius", s.tool_radius, format!("{} px", s.tool_radius as i32), RADIUS_RANGE),
            SliderId::Strength => ("Strength", s.tool_strength, format!("{:.0}", s.tool_strength), STRENGTH_RANGE),
            SliderId::Thickness => {
                ("Thickness", s.draw_thickness, format!("{} px", s.draw_thickness as i32), DRAW_THICKNESS_RANGE)
            }
            SliderId::MotorSpeed => {
                let dir = match s.motor_speed {
                    v if v.abs() < 0.05 => "stopped",
                    v if v > 0.0 => "clockwise",
                    _ => "anticlockwise",
                };
                ("Speed", s.motor_speed, format!("{:.1} rad/s  {dir}", s.motor_speed.abs()), MOTOR_SPEED_RANGE)
            }
            SliderId::ZoneStrength => {
                let label = if s.zone_kind == ZoneKind::Float { "Lift" } else { "Strength" };
                (label, s.zone_strength, format!("{:.0}", s.zone_strength), ZONE_STRENGTH_RANGE)
            }
            SliderId::Thrust => ("Thrust", s.thrust, format!("×{:.1} its weight", s.thrust), THRUST_RANGE),
            SliderId::CannonSpeed => {
                ("Speed", s.cannon_speed, format!("{:.0} m/s", s.cannon_speed), CANNON_SPEED_RANGE)
            }
            SliderId::CannonRate => {
                ("Rate", s.cannon_rate, format!("{:.1} shots / s", s.cannon_rate), CANNON_RATE_RANGE)
            }
        };
        (SliderSpec { label, value_text: text, min: range.0, max: range.1, accent }, value)
    }

    pub fn update(&mut self, dt: f32, s: &mut Settings, input: &mut Input, actions: &mut Vec<Action>) {
        let want = s.tool.card();
        if want != Card::None && (want == self.card || self.shown < 0.05) {
            self.card = want;
        }
        let show = want != Card::None && want == self.card;
        self.shown = approach(self.shown, if show { 1.0 } else { 0.0 }, 12.0, dt);
        if self.shown < 0.5 || !show {
            for st in &mut self.sliders {
                st.dragging = false;
            }
            return;
        }
        let l = self.layout(s);
        for &(id, r) in &l.sliders {
            let (value, (lo, hi)) = Self::slider(id, s);
            self.sliders[id as usize].update(r, value, lo, hi, input);
        }
        if self.sliders[SliderId::MotorSpeed as usize].dragging && s.motor_speed.abs() < 0.3 {
            s.motor_speed = 0.0;
        }
        for (i, r) in l.swatches.iter().enumerate() {
            if button(*r, input) {
                s.spawn_color = i;
            }
        }
        if l.pin.is_some_and(|r| toggle_row(r, input)) {
            s.draw_pinned = !s.draw_pinned;
        }
        if l.drive.is_some_and(|r| toggle_row(r, input)) {
            s.motor_drive = !s.motor_drive;
        }
        for (i, r) in l.triggers.iter().enumerate() {
            if button(*r, input) {
                s.gadget_trigger = Trigger::ALL[i];
            }
        }
        for (i, r) in l.ammo.iter().enumerate() {
            if button(*r, input) {
                s.cannon_ammo = Ammo::ALL[i];
            }
        }
        for (i, r) in l.kinds.iter().enumerate() {
            if button(*r, input) {
                match self.card {
                    Card::Link => s.link_kind = LinkKind::ALL[i],
                    Card::Pour => s.grain_kind = GrainKind::ALL[i],
                    Card::Gadget => s.gadget_kind = GadgetKind::ALL[i],
                    _ => s.zone_kind = ZoneKind::TOOL[i],
                }
            }
        }
        for (i, r) in l.directions.iter().enumerate() {
            if button(*r, input) {
                s.zone_angle = DIRECTIONS[i];
            }
        }
        if l.clear.is_some_and(|r| button(r, input)) {
            actions.push(Action::ClearGrains);
        }
        for (i, r) in l.commands.iter().enumerate() {
            if button(*r, input) {
                actions.push(Action::Selection(SELECT_CMDS[i].0));
            }
        }
        if button(l.head, input) {
            actions.push(Action::OpenToolPicker);
        }
        input.block(l.panel);
    }

    pub fn dragging(&self) -> bool {
        self.sliders.iter().any(|s| s.dragging)
    }

    pub fn draw(&self, s: &Settings, mouse: Vec2) {
        if self.shown < 0.01 || self.card == Card::None {
            return;
        }
        let f = self.shown;
        let l = self.layout(s);
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

        for &(id, r) in &l.sliders {
            let (spec, value) = Self::spec(id, s, accent);
            draw_slider(r, value, &spec, r.contains(mouse) || self.sliders[id as usize].dragging, f);
        }
        for (i, r) in l.swatches.iter().enumerate() {
            rrect(*r, 5.0, fade(spawn_color(i, t), f));
            if i == PALETTE.len() {
                text_centered("?", r.x + r.w / 2.0, r.y + r.h / 2.0, 11.0, fade(Color::new(0.1, 0.1, 0.1, 0.8), f));
            }
            if i == s.spawn_color {
                rrect_lines(Rect::new(r.x - 2.5, r.y - 2.5, r.w + 5.0, r.h + 5.0), 7.0, 1.8, fade(TEXT, f));
            }
        }
        if let Some(r) = l.pin {
            draw_toggle_row(r, "Pin drawings  (Shift inverts)", s.draw_pinned, r.contains(mouse), f);
        }
        if let Some(r) = l.drive {
            draw_toggle_row(r, "Drive it with ← →", s.motor_drive, r.contains(mouse), f);
        }
        let small_button = |r: Rect, active: bool| {
            let hov = r.contains(mouse);
            let bg = if active {
                alpha(accent, 0.3)
            } else if hov {
                SURFACE_HI
            } else {
                SURFACE_2
            };
            rrect(r, 6.0, fade(bg, f));
            rrect_lines(r, 6.0, 1.0, fade(if active { accent } else { BORDER }, f));
            fade(if active { TEXT } else { TEXT_DIM }, f)
        };
        for (i, r) in l.triggers.iter().enumerate() {
            let col = small_button(*r, Trigger::ALL[i] == s.gadget_trigger);
            icons::trigger(Trigger::ALL[i], vec2(r.x + r.w / 2.0, r.y + r.h / 2.0), 16.0, col);
        }
        if let Some(r) = l.triggers.first() {
            let label = format!("Works {}", s.gadget_trigger.label());
            text(&label, r.x, r.y - 3.0, 10.0, fade(TEXT_MUTED, f));
        }
        for (i, r) in l.ammo.iter().enumerate() {
            let col = small_button(*r, Ammo::ALL[i] == s.cannon_ammo);
            text_centered(Ammo::ALL[i].label(), r.x + r.w / 2.0, r.y + r.h / 2.0, 11.0, col);
        }
        for (i, r) in l.kinds.iter().enumerate() {
            let (label, kind_accent, active) = match self.card {
                Card::Link => (LinkKind::ALL[i].label(), LinkKind::ALL[i].accent(), LinkKind::ALL[i] == s.link_kind),
                Card::Pour => {
                    let k = GrainKind::ALL[i];
                    (k.label(), k.accent(), k == s.grain_kind)
                }
                Card::Gadget => {
                    let k = GadgetKind::ALL[i];
                    (k.label(), k.accent(), k == s.gadget_kind)
                }
                _ => (ZoneKind::TOOL[i].label(), ZoneKind::TOOL[i].accent(), ZoneKind::TOOL[i] == s.zone_kind),
            };
            let hov = r.contains(mouse);
            let bg = if active {
                mix(SURFACE_2, kind_accent, 0.2)
            } else if hov {
                SURFACE_HI
            } else {
                SURFACE_2
            };
            rrect(*r, 8.0, fade(bg, f));
            rrect_lines(*r, 8.0, 1.0, fade(if active { kind_accent } else { BORDER }, f));
            let col = fade(if active { kind_accent } else { TEXT_DIM }, f);
            let c = vec2(r.x + r.w / 2.0, r.y + 18.0);
            match self.card {
                Card::Link => icons::link_kind(LinkKind::ALL[i], c, 24.0, col),
                Card::Pour => icons::grain_kind(GrainKind::ALL[i], c, 24.0, col, t),
                Card::Gadget => icons::gadget_kind(GadgetKind::ALL[i], c, 24.0, col, t),
                _ => icons::zone_kind(ZoneKind::TOOL[i], c, 24.0, col, t),
            }
            text_centered(label, r.x + r.w / 2.0, r.y + 39.0, 12.0, fade(TEXT, f));
        }
        for (i, r) in l.directions.iter().enumerate() {
            let active = (s.zone_angle - DIRECTIONS[i]).abs() < 1.0;
            let hov = r.contains(mouse);
            rrect(
                *r,
                6.0,
                fade(
                    if active {
                        alpha(accent, 0.3)
                    } else if hov {
                        SURFACE_HI
                    } else {
                        SURFACE_2
                    },
                    f,
                ),
            );
            let a = DIRECTIONS[i].to_radians();
            let d = vec2(a.cos(), -a.sin());
            let n = vec2(-d.y, d.x);
            let c = vec2(r.x + r.w / 2.0, r.y + r.h / 2.0);
            let col = fade(if active { TEXT } else { TEXT_DIM }, f);
            draw_line(c.x - d.x * 8.0, c.y - d.y * 8.0, c.x + d.x * 6.0, c.y + d.y * 6.0, 2.0, col);
            draw_triangle(c + d * 9.0, c + d * 3.0 + n * 5.0, c + d * 3.0 - n * 5.0, col);
        }
        if let Some(r) = l.clear {
            let label = match self.grains {
                0 => "No grains yet".to_string(),
                n => format!("Clear {n} grains"),
            };
            draw_button(
                r,
                &label,
                r.contains(mouse) && self.grains > 0,
                false,
                if self.grains > 0 { f } else { f * 0.5 },
            );
        }
        if let Some(r) = l.info {
            let line = match self.selected {
                0 => "Nothing selected  ·  click or drag a box".to_string(),
                1 => "1 object selected".to_string(),
                n => format!("{n} objects selected"),
            };
            text(&line, r.x, r.y + 13.0, 13.0, fade(if self.selected > 0 { TEXT } else { TEXT_MUTED }, f));
        }
        for (i, r) in l.commands.iter().enumerate() {
            let (cmd, label) = SELECT_CMDS[i];
            let enabled = self.selected > 0 || cmd == SelectionCmd::Paste;
            let hov = enabled && r.contains(mouse);
            rrect(*r, 7.0, fade(if hov { SURFACE_HI } else { SURFACE_2 }, f));
            rrect_lines(*r, 7.0, 1.0, fade(BORDER, f));
            let col = if enabled { TEXT } else { TEXT_MUTED };
            text_centered(label, r.x + r.w / 2.0, r.y + r.h / 2.0, 12.0, fade(col, f));
        }
        if let Some(hint) = l.hint {
            let r = l.panel;
            for (i, line) in hint.iter().enumerate() {
                let y = r.y + r.h - 30.0 + i as f32 * 15.0;
                text_centered(line, r.x + r.w / 2.0, y, 11.0, fade(TEXT_MUTED, f));
            }
        }
    }
}
