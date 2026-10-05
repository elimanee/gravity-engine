//! Object properties panel (right-click → Properties, or `I`): bounce,
//! friction, mass, gravity, breakability, flammability, mirrors, magnetism and conveyor speed of
//! one object, with a few presets.

use super::theme::*;
use super::widgets::*;
use super::{Fader, Input};
use crate::physics::object::Material;
use crate::util::ellipsize;
use macroquad::prelude::*;
use rapier2d::prelude::RigidBodyHandle;

const W: f32 = 268.0;
const PAD: f32 = 14.0;
const ROW: f32 = SLIDER_ROW_H + 2.0;
/// Tallest the panel gets (used to keep it on screen).
const MAX_H: f32 = 640.0;

pub const BOUNCE_RANGE: (f32, f32) = (0.0, 1.2);
pub const FRICTION_RANGE: (f32, f32) = (0.0, 2.0);
pub const GRAVITY_RANGE: (f32, f32) = (-1.0, 2.0);
pub const STRENGTH_RANGE: (f32, f32) = (2.0, 30.0);
pub const MAGNET_RANGE: (f32, f32) = (-2.0, 2.0);
pub const CONVEYOR_RANGE: (f32, f32) = (-8.0, 8.0);
pub const PLANET_RANGE: (f32, f32) = (0.0, 30.0);
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

const PRESETS: &[&str] = &["Rubber", "Ice", "Heavy", "Balloon", "Glass", "Magnet", "Mirror", "Planet"];

#[derive(Clone, Copy, PartialEq)]
enum SliderId {
    Bounce,
    Friction,
    Mass,
    Gravity,
    Strength,
    Magnet,
    Conveyor,
    Planet,
}

pub struct Inspector {
    pub fader: Fader,
    pub target: Option<RigidBodyHandle>,
    title: String,
    pos: Vec2,
    sliders: [SliderState; 8],
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
    sliders: Vec<(SliderId, Rect)>,
    breakable: Rect,
    flammable: Rect,
    mirror: Rect,
    presets: Vec<Rect>,
}

fn snap(v: &mut f32, targets: &[f32], within: f32) {
    if let Some(t) = targets.iter().find(|t| (*v - **t).abs() < within) {
        *v = *t;
    }
}

impl Inspector {
    pub fn open(&mut self, target: RigidBodyHandle, title: String, near: Vec2) {
        let (sw, sh) = (screen_width(), screen_height());
        let x = if near.x + 40.0 + W < sw { near.x + 40.0 } else { near.x - 40.0 - W };
        self.pos =
            vec2(x.clamp(8.0, (sw - W - 8.0).max(8.0)), (near.y - MAX_H / 2.0).clamp(8.0, (sh - MAX_H - 8.0).max(8.0)));
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

    fn layout(&self, m: &Material) -> Layout {
        let f = self.fader.value();
        let (x, mut y) = (self.pos.x + PAD, self.pos.y + (1.0 - f) * 10.0 + 46.0);
        let mut sliders = vec![];
        let mut slider = |id, y: &mut f32| {
            sliders.push((id, Rect::new(x, *y, W - PAD * 2.0, SLIDER_ROW_H)));
            *y += ROW;
        };
        for id in [SliderId::Bounce, SliderId::Friction, SliderId::Mass, SliderId::Gravity] {
            slider(id, &mut y);
        }
        let breakable = Rect::new(x, y + 2.0, W - PAD * 2.0, 28.0);
        y += 34.0;
        if m.breakable {
            slider(SliderId::Strength, &mut y);
        }
        let flammable = Rect::new(x, y + 2.0, W - PAD * 2.0, 28.0);
        y += 34.0;
        let mirror = Rect::new(x, y + 2.0, W - PAD * 2.0, 28.0);
        y += 34.0;
        slider(SliderId::Magnet, &mut y);
        slider(SliderId::Conveyor, &mut y);
        slider(SliderId::Planet, &mut y);
        y += 20.0;
        let bw = (W - PAD * 2.0 - 12.0) / 3.0;
        let presets = (0..PRESETS.len())
            .map(|i| Rect::new(x + (i % 3) as f32 * (bw + 6.0), y + (i / 3) as f32 * 34.0, bw, 28.0))
            .collect();
        y += PRESETS.len().div_ceil(3) as f32 * 34.0 + 8.0;
        let top = self.pos.y + (1.0 - f) * 10.0;
        let panel = Rect::new(self.pos.x, top, W, y - top);
        let close = Rect::new(panel.x + panel.w - 34.0, panel.y + 10.0, 24.0, 24.0);
        let reset = Rect::new(close.x - 62.0, panel.y + 10.0, 56.0, 24.0);
        Layout { panel, close, reset, sliders, breakable, flammable, mirror, presets }
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
        let l = self.layout(&p.material);
        if !self.sliders[SliderId::Mass as usize].dragging {
            self.log_mass = p.mass.max(1e-3).log10();
        }
        for &(id, r) in &l.sliders {
            let st = &mut self.sliders[id as usize];
            let m = &mut p.material;
            let (value, range) = match id {
                SliderId::Bounce => (&mut m.bounce, BOUNCE_RANGE),
                SliderId::Friction => (&mut m.friction, FRICTION_RANGE),
                SliderId::Mass => (&mut self.log_mass, LOG_MASS_RANGE),
                SliderId::Gravity => (&mut m.gravity, GRAVITY_RANGE),
                SliderId::Strength => (&mut m.strength, STRENGTH_RANGE),
                SliderId::Magnet => (&mut m.magnet, MAGNET_RANGE),
                SliderId::Conveyor => (&mut m.conveyor, CONVEYOR_RANGE),
                SliderId::Planet => (&mut m.planet, PLANET_RANGE),
            };
            if st.update(r, value, range.0, range.1, input) {
                match id {
                    SliderId::Mass => p.mass = 10f32.powf(self.log_mass),
                    SliderId::Gravity => snap(&mut p.material.gravity, &[0.0, 1.0], 0.06),
                    SliderId::Magnet => snap(&mut p.material.magnet, &[0.0], 0.12),
                    SliderId::Conveyor => snap(&mut p.material.conveyor, &[0.0], 0.4),
                    SliderId::Planet => snap(&mut p.material.planet, &[0.0], 0.8),
                    _ => {}
                }
            }
        }
        if toggle_row(l.breakable, input) {
            p.material.breakable = !p.material.breakable;
        }
        if toggle_row(l.flammable, input) {
            p.material.flammable = !p.material.flammable;
        }
        if toggle_row(l.mirror, input) {
            p.material.mirror = !p.material.mirror;
        }
        for (i, r) in l.presets.iter().enumerate() {
            if button(*r, input) {
                match i {
                    0 => p.material = Material::RUBBER,
                    1 => p.material = Material::ICE,
                    2 => {
                        p.material = Material { bounce: 0.1, friction: 0.8, ..Material::DEFAULT };
                        p.mass = p.default_mass * 6.0;
                    }
                    3 => {
                        p.material = Material::BALLOON;
                        p.mass = p.default_mass * 0.2;
                    }
                    4 => p.material = Material::GLASS,
                    5 => p.material = Material::MAGNET,
                    6 => p.material = Material::MIRROR,
                    _ => {
                        p.material = Material::PLANET;
                        p.mass = p.default_mass * 4.0;
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
        let l = self.layout(&p.material);
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
        for &(id, r) in &l.sliders {
            let (label, value_text, (min, max), v, accent) = match id {
                SliderId::Bounce => ("Bounce", format!("{:.0}%", m.bounce * 100.0), BOUNCE_RANGE, m.bounce, SUCCESS),
                SliderId::Friction => ("Friction", format!("{:.2}", m.friction), FRICTION_RANGE, m.friction, WARNING),
                SliderId::Mass => {
                    let text = if p.mass >= 100.0 { format!("{:.0} kg", p.mass) } else { format!("{:.2} kg", p.mass) };
                    ("Mass", text, LOG_MASS_RANGE, p.mass.max(1e-3).log10(), ACCENT)
                }
                SliderId::Gravity => {
                    let text = match m.gravity {
                        0.0 => "weightless".to_string(),
                        g if g < 0.0 => format!("×{g:.2} · floats up"),
                        g => format!("×{g:.2}"),
                    };
                    ("Gravity", text, GRAVITY_RANGE, m.gravity, Color::new(0.4, 0.8, 1.0, 1.0))
                }
                SliderId::Strength => {
                    let text = format!("breaks above {:.0} m/s", m.strength);
                    ("Toughness", text, STRENGTH_RANGE, m.strength, Color::new(0.75, 0.9, 1.0, 1.0))
                }
                SliderId::Magnet => {
                    let text = match m.magnet {
                        0.0 => "off".to_string(),
                        q if q > 0.0 => format!("{q:.1} · attracts"),
                        q => format!("{:.1} · repels", -q),
                    };
                    ("Magnet", text, MAGNET_RANGE, m.magnet, Color::new(1.0, 0.4, 0.45, 1.0))
                }
                SliderId::Conveyor => {
                    let text = match m.conveyor {
                        0.0 => "off".to_string(),
                        v if v > 0.0 => format!("{v:.1} m/s clockwise"),
                        v => format!("{:.1} m/s anticlockwise", -v),
                    };
                    ("Conveyor", text, CONVEYOR_RANGE, m.conveyor, Color::new(1.0, 0.85, 0.3, 1.0))
                }
                SliderId::Planet => {
                    let text = match m.planet {
                        0.0 => "off".to_string(),
                        g => format!("{g:.1} m/s² at its surface"),
                    };
                    ("Planet gravity", text, PLANET_RANGE, m.planet, Color::new(0.55, 0.7, 1.0, 1.0))
                }
            };
            let spec = SliderSpec { label, value_text, min, max, accent };
            draw_slider(r, v, &spec, r.contains(mouse) || self.sliders[id as usize].dragging, f);
        }
        draw_toggle_row(l.breakable, "Breakable (shatters)", m.breakable, l.breakable.contains(mouse), f);
        draw_toggle_row(l.flammable, "Flammable (burns)", m.flammable, l.flammable.contains(mouse), f);
        draw_toggle_row(l.mirror, "Mirror (reflects lasers)", m.mirror, l.mirror.contains(mouse), f);
        let py = l.presets[0].y;
        text("Presets", l.panel.x + PAD, py - 7.0, 11.0, fade(TEXT_MUTED, f));
        for (i, r) in l.presets.iter().enumerate() {
            draw_button(*r, PRESETS[i], r.contains(mouse), false, f);
        }
    }
}
