//! Object properties panel (right-click → Properties, or `I`): bounce,
//! friction, mass and gravity of one object, with a few presets.

use super::theme::*;
use super::widgets::*;
use super::{Fader, Input};
use crate::physics::object::Material;
use crate::util::ellipsize;
use macroquad::prelude::*;
use rapier2d::prelude::RigidBodyHandle;

const W: f32 = 264.0;
const H: f32 = 292.0;
const PAD: f32 = 14.0;

pub const BOUNCE_RANGE: (f32, f32) = (0.0, 1.2);
pub const FRICTION_RANGE: (f32, f32) = (0.0, 2.0);
pub const GRAVITY_RANGE: (f32, f32) = (-1.0, 2.0);
/// Mass slider range, as log10(kg).
const LOG_MASS_RANGE: (f32, f32) = (-1.3, 2.7);

/// Values edited by the panel.
#[derive(Clone, Copy, PartialEq)]
pub struct Props {
    pub material: Material,
    pub mass: f32,
    /// Mass the object would have at the default density (for Reset / Heavy).
    pub default_mass: f32,
}

const PRESETS: &[&str] = &["Rubber", "Ice", "Heavy", "Balloon"];

pub struct Inspector {
    pub fader: Fader,
    pub target: Option<RigidBodyHandle>,
    title: String,
    pos: Vec2,
    sliders: [SliderState; 4],
    log_mass: f32,
}

impl Default for Inspector {
    fn default() -> Self {
        Inspector {
            fader: Fader::default(),
            target: None,
            title: String::new(),
            pos: Vec2::ZERO,
            sliders: Default::default(),
            log_mass: 0.0,
        }
    }
}

struct Layout {
    panel: Rect,
    close: Rect,
    reset: Rect,
    sliders: [Rect; 4],
    presets: Vec<Rect>,
}

impl Inspector {
    pub fn open(&mut self, target: RigidBodyHandle, title: String, near: Vec2) {
        let (sw, sh) = (screen_width(), screen_height());
        let x = if near.x + 40.0 + W < sw { near.x + 40.0 } else { near.x - 40.0 - W };
        self.pos = vec2(x.clamp(8.0, (sw - W - 8.0).max(8.0)), (near.y - H / 2.0).clamp(8.0, (sh - H - 8.0).max(8.0)));
        self.target = Some(target);
        self.title = ellipsize(&title, 22);
        self.fader.open = true;
    }

    pub fn close(&mut self) {
        self.fader.open = false;
        self.target = None;
        for s in &mut self.sliders {
            s.dragging = false;
        }
    }

    fn layout(&self) -> Layout {
        let f = self.fader.value();
        let panel = Rect::new(self.pos.x, self.pos.y + (1.0 - f) * 10.0, W, H);
        let close = Rect::new(panel.x + panel.w - 34.0, panel.y + 10.0, 24.0, 24.0);
        let reset = Rect::new(close.x - 62.0, panel.y + 10.0, 56.0, 24.0);
        let row = |i: usize| {
            Rect::new(panel.x + PAD, panel.y + 46.0 + i as f32 * (SLIDER_ROW_H + 4.0), W - PAD * 2.0, SLIDER_ROW_H)
        };
        let py = panel.y + 46.0 + 4.0 * (SLIDER_ROW_H + 4.0) + 20.0;
        let n = PRESETS.len();
        let bw = (W - PAD * 2.0 - (n as f32 - 1.0) * 6.0) / n as f32;
        let presets = (0..n).map(|i| Rect::new(panel.x + PAD + i as f32 * (bw + 6.0), py, bw, 28.0)).collect();
        Layout { panel, close, reset, sliders: [row(0), row(1), row(2), row(3)], presets }
    }

    pub fn dragging(&self) -> bool {
        self.sliders.iter().any(|s| s.dragging)
    }

    /// Edit `props`; returns whether anything changed.
    pub fn update(&mut self, dt: f32, input: &mut Input, props: Option<&mut Props>) -> bool {
        self.fader.update(dt, 9.0);
        let Some(p) = props.filter(|_| self.fader.open) else {
            if self.fader.open {
                self.close(); // the object is gone
            }
            return false;
        };
        let before = *p;
        let l = self.layout();
        if !self.sliders[2].dragging {
            self.log_mass = p.mass.max(1e-3).log10();
        }
        let m = &mut p.material;
        self.sliders[0].update(l.sliders[0], &mut m.bounce, BOUNCE_RANGE.0, BOUNCE_RANGE.1, input);
        self.sliders[1].update(l.sliders[1], &mut m.friction, FRICTION_RANGE.0, FRICTION_RANGE.1, input);
        if self.sliders[2].update(l.sliders[2], &mut self.log_mass, LOG_MASS_RANGE.0, LOG_MASS_RANGE.1, input) {
            p.mass = 10f32.powf(self.log_mass);
        }
        self.sliders[3].update(l.sliders[3], &mut p.material.gravity, GRAVITY_RANGE.0, GRAVITY_RANGE.1, input);
        if self.sliders[3].dragging && p.material.gravity.abs() < 0.06 {
            p.material.gravity = 0.0;
        }
        if self.sliders[3].dragging && (p.material.gravity - 1.0).abs() < 0.06 {
            p.material.gravity = 1.0;
        }
        for (i, r) in l.presets.iter().enumerate() {
            if button(*r, input) {
                match i {
                    0 => p.material = Material::RUBBER,
                    1 => p.material = Material::ICE,
                    2 => {
                        p.material = Material { bounce: 0.1, friction: 0.8, gravity: 1.0 };
                        p.mass = p.default_mass * 6.0;
                    }
                    _ => {
                        p.material = Material::BALLOON;
                        p.mass = p.default_mass * 0.2;
                    }
                }
            }
        }
        if button(l.reset, input) {
            p.material = Material::default();
            p.mass = p.default_mass;
        }
        if button(l.close, input) {
            self.close();
        }
        input.block(l.panel);
        *p != before
    }

    pub fn draw(&self, props: Option<&Props>, mouse: Vec2) {
        let Some(p) = props.filter(|_| self.fader.visible()) else { return };
        let f = self.fader.value();
        let l = self.layout();
        panel(l.panel, f);
        text_bold("Properties", l.panel.x + PAD, l.panel.y + 22.0, 15.0, fade(TEXT, f));
        text(&self.title, l.panel.x + PAD, l.panel.y + 37.0, 11.0, fade(TEXT_MUTED, f));
        draw_button(l.reset, "Reset", l.reset.contains(mouse), false, f);
        let hov = l.close.contains(mouse);
        if hov {
            rrect(l.close, 6.0, fade(SURFACE_HI, f));
        }
        let (cx, cy) = (l.close.x + 12.0, l.close.y + 12.0);
        let c = fade(if hov { TEXT } else { TEXT_MUTED }, f);
        draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.6, c);
        draw_line(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.6, c);

        let m = p.material;
        let mass_text = if p.mass >= 100.0 { format!("{:.0} kg", p.mass) } else { format!("{:.2} kg", p.mass) };
        let gravity_text = match m.gravity {
            0.0 => "weightless".to_string(),
            g if g < 0.0 => format!("×{g:.2} · floats up"),
            g => format!("×{g:.2}"),
        };
        let specs = [
            ("Bounce", format!("{:.0}%", m.bounce * 100.0), BOUNCE_RANGE, m.bounce, SUCCESS),
            ("Friction", format!("{:.2}", m.friction), FRICTION_RANGE, m.friction, WARNING),
            ("Mass", mass_text, LOG_MASS_RANGE, p.mass.max(1e-3).log10(), ACCENT),
            ("Gravity", gravity_text, GRAVITY_RANGE, m.gravity, Color::new(0.4, 0.8, 1.0, 1.0)),
        ];
        for (i, (label, value_text, (min, max), v, accent)) in specs.into_iter().enumerate() {
            let r = l.sliders[i];
            let spec = SliderSpec { label, value_text, min, max, accent };
            draw_slider(r, v, &spec, r.contains(mouse) || self.sliders[i].dragging, f);
        }
        let py = l.presets[0].y;
        text("Presets", l.panel.x + PAD, py - 7.0, 11.0, fade(TEXT_MUTED, f));
        for (i, r) in l.presets.iter().enumerate() {
            draw_button(*r, PRESETS[i], r.contains(mouse), false, f);
        }
    }
}
