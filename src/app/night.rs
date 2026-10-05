//! Night: lamps (with shadows), fire, lasers, thrusters and lightning
//! light the darkened scene.

use super::*;
use crate::lighting;
use crate::physics::gadgets::{GadgetKind, LAMP_COLOURS};
use crate::physics::to_screen;
use rapier2d::prelude::{nalgebra, vector, Collider, QueryFilter};

/// Rays per full turn of a lamp's light.
const LAMP_RAYS: f32 = 220.0;
/// How far light reaches into what it falls on (px), so lit faces show.
const SKIN: f32 = 10.0;

/// A lamp's light: its centre, the outline of what it lights, its reach
/// and colour.
struct Fan {
    at: Vec2,
    rim: Vec<Vec2>,
    reach: f32,
    colour: Color,
}

pub(super) fn lamp_colour(c: [u8; 3]) -> Color {
    Color::from_rgba(c[0], c[1], c[2], 255)
}

impl App {
    /// The light of every lamp that is on, cut by what is in its way.
    fn lamp_fans(&mut self) -> Vec<Fan> {
        if !self.gadgets.iter().any(|g| g.on && g.spec.kind == GadgetKind::Lamp) {
            return vec![];
        }
        if self.paused {
            self.world.refresh_queries();
        }
        let grains: HashSet<RigidBodyHandle> = self.grains.bodies().collect();
        let not_grain = |_, c: &Collider| c.parent().is_none_or(|b| !grains.contains(&b));
        let mut fans = vec![];
        for g in self.gadgets.iter().filter(|g| g.on && g.spec.kind == GadgetKind::Lamp) {
            let Some((p, d)) = g.pose(&self.world) else { continue };
            let reach = g.spec.power * PPM;
            let spread = g.spec.spread;
            let base = d.y.atan2(d.x);
            let n = ((LAMP_RAYS * spread / std::f32::consts::PI) as usize).max(16);
            let mut filter = QueryFilter::default().exclude_sensors().predicate(&not_grain);
            filter.exclude_rigid_body = g.host;
            let at = to_screen(p.x, p.y);
            let rim: Vec<Vec2> = (0..=n)
                .map(|k| {
                    let a = base - spread + 2.0 * spread * k as f32 / n as f32;
                    let dir = vector![a.cos(), a.sin()];
                    let dist = match self.world.cast_ray(p, dir, reach / PPM, filter) {
                        Some((_, toi, _)) => (toi * PPM + SKIN).min(reach),
                        None => reach,
                    };
                    at + vec2(dir.x, -dir.y) * dist
                })
                .collect();
            // A spotlight's fan starts at the lamp.
            let rim = if spread < std::f32::consts::PI { [vec![at], rim, vec![at]].concat() } else { rim };
            fans.push(Fan { at, rim, reach, colour: lamp_colour(g.spec.colour) });
        }
        fans
    }

    /// Darken the world pass and light it (screen pass, right after it).
    pub(super) fn draw_night(&mut self, mouse: Vec2) {
        if !self.s.night {
            return;
        }
        let fans = self.lamp_fans();
        let visible = self.view().visible();
        if !self.lights.begin(visible, lighting::ambient(self.s.darkness, self.sky.flash)) {
            return;
        }
        let l = &self.lights;
        let t = get_time() as f32;
        for f in &fans {
            l.fan(f.at, &f.rim, f.reach, Color { a: 0.95, ..f.colour });
            l.glow(f.at, 28.0, Color { a: 0.9, ..f.colour });
        }
        // Burning and hot objects.
        for o in &self.objects {
            let k = self.fire.glow(o.body);
            if k <= 0.0 {
                continue;
            }
            let (p, _) = o.screen_pos(&self.world);
            let flicker = 0.85 + 0.15 * (t * 13.0 + p.x * 0.1).sin() * (t * 7.3).cos();
            let r = (o.size.length() * 1.3 + 60.0) * flicker * (0.4 + 0.6 * k);
            l.glow(p, r, Color::new(1.0, 0.55, 0.22, 0.75 * k));
        }
        self.effects.lights(|p, r, c| l.glow(p, r, c));
        // Laser beams.
        for beam in &self.beams {
            for (w, c) in beam.points.windows(2).zip(&beam.colours) {
                let (a, b) = (to_screen(w[0].x, w[0].y), to_screen(w[1].x, w[1].y));
                l.line(a, b, 46.0, Color { a: 0.32, ..*c });
            }
            if let Some((_, q, _)) = beam.hit {
                let c = *beam.colours.last().unwrap_or(&WHITE);
                l.glow(to_screen(q.x, q.y), 70.0, Color { a: 0.7, ..c });
            }
        }
        // Thrusters' flames.
        for g in self.gadgets.iter().filter(|g| g.on && g.spec.kind == GadgetKind::Thruster) {
            let Some((p, d)) = g.pose(&self.world) else { continue };
            let back = to_screen(p.x, p.y) - vec2(d.x, -d.y) * 24.0;
            l.glow(back, 90.0, Color::new(1.0, 0.65, 0.3, 0.6));
        }
        // The Fire tool's flame.
        if self.fire.lit && self.s.tool == Tool::Fire {
            l.glow(mouse, 110.0, Color::new(1.0, 0.6, 0.25, 0.8));
        }
        // Lightning.
        if let Some(b) = &self.sky.bolt {
            for w in b.points.windows(2) {
                l.line(w[0], w[1], 110.0, Color::new(0.7, 0.75, 1.0, 0.35));
            }
        }
        l.end();
    }

    /// Lamps themselves (world pass).
    pub(super) fn draw_lamps(&self) {
        for g in self.gadgets.iter().filter(|g| g.spec.kind == GadgetKind::Lamp) {
            let Some((p, d)) = g.pose(&self.world) else { continue };
            let spot = g.spec.spread < std::f32::consts::PI;
            crate::physics::gadgets::draw_lamp(
                to_screen(p.x, p.y),
                vec2(d.x, -d.y),
                spot,
                lamp_colour(g.spec.colour),
                g.on,
            );
        }
    }
}

/// The lamp the Gadget tool would place (cursor).
pub(super) fn cursor_lamp(s: &Settings, at: Vec2, dir: Vec2) {
    let c = lamp_colour(LAMP_COLOURS[s.lamp_colour.min(LAMP_COLOURS.len() - 1)]);
    crate::physics::gadgets::draw_lamp(at, dir, s.lamp_spot, c, true);
}
