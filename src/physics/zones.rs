//! Zones drawn with the Zone tool: rectangles that push objects (wind), make
//! them float, or teleport them (portal pairs). Challenges add goal zones.

use super::{to_screen, PhysWorld};
use crate::config::PPM;
use macroquad::prelude::*;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ZoneKind {
    Wind,
    Float,
    Portal,
    /// Target area of a challenge.
    Goal,
}

impl ZoneKind {
    /// Kinds offered by the Zone tool.
    pub const TOOL: &'static [ZoneKind] = &[ZoneKind::Wind, ZoneKind::Float, ZoneKind::Portal];

    pub fn label(self) -> &'static str {
        match self {
            ZoneKind::Wind => "Wind",
            ZoneKind::Float => "Float",
            ZoneKind::Portal => "Portal",
            ZoneKind::Goal => "Goal",
        }
    }

    pub fn hint(self) -> [&'static str; 2] {
        match self {
            ZoneKind::Wind => ["Drag a rectangle: objects inside", "are blown in the chosen direction"],
            ZoneKind::Float => ["Drag a rectangle: objects inside", "lose their weight and drift up"],
            ZoneKind::Portal => ["Drag an entrance, then an exit:", "objects jump between the two"],
            ZoneKind::Goal => ["", ""],
        }
    }

    pub fn accent(self) -> Color {
        match self {
            ZoneKind::Wind => Color::from_rgba(140, 210, 255, 255),
            ZoneKind::Float => Color::from_rgba(190, 150, 255, 255),
            ZoneKind::Portal => Color::from_rgba(80, 220, 255, 255),
            ZoneKind::Goal => Color::from_rgba(110, 235, 140, 255),
        }
    }
}

/// Colour of the second portal of a pair.
const EXIT: Color = Color::new(1.0, 0.62, 0.25, 1.0);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Zone {
    pub kind: ZoneKind,
    /// Lower-left and upper-right corners in physics metres.
    pub min: [f32; 2],
    pub max: [f32; 2],
    /// Wind direction (radians, 0 = right, counter-clockwise).
    #[serde(default)]
    pub angle: f32,
    /// Wind or lift strength (m/s²).
    #[serde(default)]
    pub strength: f32,
    /// Portal: index of the other end.
    #[serde(default)]
    pub pair: Option<usize>,
}

impl Zone {
    /// A zone spanning two screen points.
    pub fn from_screen(kind: ZoneKind, a: Vec2, b: Vec2, angle: f32, strength: f32) -> Self {
        let (ax, ay) = super::to_phys(a.x, a.y);
        let (bx, by) = super::to_phys(b.x, b.y);
        Zone { kind, min: [ax.min(bx), ay.min(by)], max: [ax.max(bx), ay.max(by)], angle, strength, pair: None }
    }

    pub fn contains(&self, p: Vector<f32>) -> bool {
        p.x >= self.min[0] && p.x <= self.max[0] && p.y >= self.min[1] && p.y <= self.max[1]
    }

    /// Screen rectangle.
    pub fn rect(&self) -> Rect {
        let tl = to_screen(self.min[0], self.max[1]);
        Rect::new(tl.x, tl.y, (self.max[0] - self.min[0]) * PPM, (self.max[1] - self.min[1]) * PPM)
    }

    fn size(&self) -> (f32, f32) {
        (self.max[0] - self.min[0], self.max[1] - self.min[1])
    }
}

/// Remove zone `i`, keeping portal pairs consistent.
pub fn remove(zones: &mut Vec<Zone>, i: usize) {
    zones.remove(i);
    for z in zones.iter_mut() {
        z.pair = match z.pair {
            Some(p) if p == i => None,
            Some(p) if p > i => Some(p - 1),
            other => other,
        };
    }
}

/// Add a zone; a new portal pairs with the most recent unpaired one.
/// Returns whether a portal pair was completed.
pub fn add(zones: &mut Vec<Zone>, mut zone: Zone) -> bool {
    let i = zones.len();
    let partner = (zone.kind == ZoneKind::Portal)
        .then(|| zones.iter().rposition(|z| z.kind == ZoneKind::Portal && z.pair.is_none()))
        .flatten();
    zone.pair = partner;
    zones.push(zone);
    if let Some(p) = partner {
        zones[p].pair = Some(i);
    }
    partner.is_some()
}

/// Index of the top-most zone under a screen point.
pub fn zone_at(zones: &[Zone], p: Vec2) -> Option<usize> {
    zones.iter().rposition(|z| z.rect().contains(p))
}

/// Tracks which bodies are inside a portal, so an object that just came out
/// of one is not sent straight back.
#[derive(Default)]
pub struct PortalState {
    inside: HashSet<RigidBodyHandle>,
}

/// A teleport, for the particle effects (physics metres).
pub struct Teleport {
    pub from: Vector<f32>,
    pub to: Vector<f32>,
}

/// Apply every zone to the dynamic objects. Returns the teleports.
/// Push the bodies (objects and grains) that are inside zones.
pub fn apply(
    zones: &[Zone],
    state: &mut PortalState,
    world: &mut PhysWorld,
    bodies: &[RigidBodyHandle],
) -> Vec<Teleport> {
    let gravity = world.gravity;
    let mut teleports = vec![];
    let mut inside_now = HashSet::new();
    for &body in bodies {
        let Some(b) = world.bodies.get_mut(body) else { continue };
        if !b.is_dynamic() {
            continue;
        }
        let p = *b.translation();
        let m = b.mass();
        for z in zones {
            if !z.contains(p) {
                continue;
            }
            match z.kind {
                ZoneKind::Wind => {
                    let dir = vector![z.angle.cos(), z.angle.sin()];
                    b.add_force(dir * z.strength * m, true);
                }
                ZoneKind::Float => {
                    // Cancel this body's weight, add a gentle lift and damp the motion.
                    let lift = -gravity * b.gravity_scale() + vector![0.0, z.strength * 0.15];
                    let damp = -*b.linvel() * 1.2;
                    b.add_force((lift + damp) * m, true);
                    let w = b.angvel();
                    b.set_angvel(w * 0.98, true);
                }
                ZoneKind::Portal => {
                    inside_now.insert(body);
                    let Some(exit) = z.pair.and_then(|j| zones.get(j)) else { continue };
                    if state.inside.contains(&body) {
                        continue;
                    }
                    // Same relative position in the exit, same velocity.
                    let (w0, h0) = z.size();
                    let (w1, h1) = exit.size();
                    let u = ((p.x - z.min[0]) / w0.max(0.01)).clamp(0.0, 1.0);
                    let v = ((p.y - z.min[1]) / h0.max(0.01)).clamp(0.0, 1.0);
                    let to = vector![exit.min[0] + u * w1, exit.min[1] + v * h1];
                    b.set_translation(to, true);
                    teleports.push(Teleport { from: p, to });
                    break;
                }
                ZoneKind::Goal => {}
            }
        }
    }
    state.inside = inside_now;
    teleports
}

/// Draw every zone (under the objects).
pub fn draw(zones: &[Zone]) {
    let t = get_time() as f32;
    for (i, z) in zones.iter().enumerate() {
        let r = z.rect();
        let exit = z.kind == ZoneKind::Portal && z.pair.is_some_and(|p| p < i);
        let c = if exit { EXIT } else { z.kind.accent() };
        draw_rectangle(r.x, r.y, r.w, r.h, Color { a: 0.07, ..c });
        let unpaired = z.kind == ZoneKind::Portal && z.pair.is_none();
        dashed_rect(r, Color { a: if unpaired { 0.45 } else { 0.7 }, ..c }, t * if unpaired { 0.0 } else { 18.0 });

        // Animated content, clipped to the rectangle.
        crate::ui::widgets::clip(Some(crate::camera::current().rect_to_screen(r)));
        match z.kind {
            ZoneKind::Wind => {
                let dir = vec2(z.angle.cos(), -z.angle.sin());
                let span = r.w.abs() + r.h.abs();
                for k in 0..((r.w * r.h / 1400.0) as usize).clamp(6, 60) {
                    let seed = k as f32 * 12.9898;
                    let across = (seed.sin() * 43758.547).fract().abs();
                    let along = ((seed * 0.37).cos() * 1753.1).fract().abs();
                    let phase = (along + t * (0.25 + z.strength * 0.02)) % 1.0;
                    let n = vec2(-dir.y, dir.x);
                    let centre = vec2(r.x + r.w / 2.0, r.y + r.h / 2.0);
                    let p = centre + dir * (phase - 0.5) * span + n * (across - 0.5) * span;
                    let tail = p - dir * 22.0;
                    draw_line(
                        tail.x,
                        tail.y,
                        p.x,
                        p.y,
                        1.5,
                        Color { a: 0.35 * (1.0 - (phase - 0.5).abs() * 2.0), ..c },
                    );
                }
            }
            ZoneKind::Float => {
                for k in 0..((r.w * r.h / 2500.0) as usize).clamp(5, 40) {
                    let seed = k as f32 * 7.31;
                    let x = r.x + ((seed.sin() * 9131.7).fract().abs()) * r.w;
                    let phase = ((seed.cos() * 311.3).fract().abs() + t * 0.18) % 1.0;
                    let y = r.y + r.h * (1.0 - phase);
                    let wobble = (t * 3.0 + seed).sin() * 3.0;
                    draw_circle_lines(x + wobble, y, 2.0 + (k % 3) as f32, 1.0, Color { a: 0.4, ..c });
                }
            }
            ZoneKind::Portal => {
                let centre = vec2(r.x + r.w / 2.0, r.y + r.h / 2.0);
                let radius = r.w.min(r.h) * 0.38;
                for ring in 0..3 {
                    let rr = radius * (1.0 - ring as f32 * 0.25);
                    let spin = t * (1.5 + ring as f32) * if exit { -1.0 } else { 1.0 };
                    for s in 0..6 {
                        let a0 = spin + s as f32 * TAU / 6.0;
                        let (p0, p1) = (
                            centre + vec2(a0.cos(), a0.sin()) * rr,
                            centre + vec2((a0 + 0.6).cos(), (a0 + 0.6).sin()) * rr,
                        );
                        draw_line(p0.x, p0.y, p1.x, p1.y, 2.0, Color { a: 0.55 - ring as f32 * 0.12, ..c });
                    }
                }
            }
            ZoneKind::Goal => {
                // Checkered band along the bottom and a waving flag.
                let sq = 8.0;
                let n = (r.w / sq).ceil() as usize;
                for k in 0..n {
                    for row in 0..2 {
                        if (k + row) % 2 == 0 {
                            draw_rectangle(
                                r.x + k as f32 * sq,
                                r.y + r.h - sq * (row + 1) as f32,
                                sq,
                                sq,
                                Color { a: 0.35, ..c },
                            );
                        }
                    }
                }
                let (fx, fy) = (r.x + r.w / 2.0, r.y + r.h * 0.2);
                draw_line(fx, fy, fx, fy + 30.0, 2.0, c);
                let wave = (t * 4.0).sin() * 3.0;
                draw_triangle(vec2(fx, fy), vec2(fx, fy + 14.0), vec2(fx + 20.0, fy + 7.0 + wave), c);
            }
        }
        crate::ui::widgets::clip(None);

        let label = match z.kind {
            ZoneKind::Portal if unpaired => "Portal · draw the exit",
            ZoneKind::Portal if exit => "Portal exit",
            k => k.label(),
        };
        crate::ui::theme::text(label, r.x + 6.0, r.y + 14.0, 11.0, Color { a: 0.85, ..c });
    }
    // Faint link between the two ends of each portal.
    for (i, z) in zones.iter().enumerate() {
        let Some(j) = z.pair.filter(|&j| j > i) else { continue };
        let (a, b) = (z.rect(), zones[j].rect());
        let (pa, pb) = (vec2(a.x + a.w / 2.0, a.y + a.h / 2.0), vec2(b.x + b.w / 2.0, b.y + b.h / 2.0));
        let steps = (pa.distance(pb) / 12.0) as usize;
        for s in (0..steps).step_by(2) {
            let (u0, u1) = (s as f32 / steps as f32, (s + 1) as f32 / steps as f32);
            let (p0, p1) = (pa.lerp(pb, u0), pa.lerp(pb, u1));
            draw_line(p0.x, p0.y, p1.x, p1.y, 1.0, Color::new(0.6, 0.8, 1.0, 0.18));
        }
    }
}

fn dashed_rect(r: Rect, c: Color, offset: f32) {
    let corners = [vec2(r.x, r.y), vec2(r.x + r.w, r.y), vec2(r.x + r.w, r.y + r.h), vec2(r.x, r.y + r.h)];
    for i in 0..4 {
        let (a, b) = (corners[i], corners[(i + 1) % 4]);
        let len = a.distance(b);
        let dir = (b - a) / len.max(1.0);
        let mut d = -(offset % 12.0);
        while d < len {
            let (s, e) = (d.max(0.0), (d + 6.0).min(len));
            if e > s {
                let (p0, p1) = (a + dir * s, a + dir * e);
                draw_line(p0.x, p0.y, p1.x, p1.y, 1.5, c);
            }
            d += 12.0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;
    use crate::physics::object::Object;

    fn zone(kind: ZoneKind, min: [f32; 2], max: [f32; 2]) -> Zone {
        Zone { kind, min, max, angle: 0.0, strength: 20.0, pair: None }
    }

    #[test]
    fn portals_pair_up_and_removal_keeps_pairs_valid() {
        let mut zones = vec![zone(ZoneKind::Wind, [0.0, 0.0], [1.0, 1.0])];
        assert!(!add(&mut zones, zone(ZoneKind::Portal, [2.0, 0.0], [3.0, 1.0])));
        assert!(add(&mut zones, zone(ZoneKind::Portal, [5.0, 0.0], [6.0, 1.0])));
        assert_eq!((zones[1].pair, zones[2].pair), (Some(2), Some(1)));
        remove(&mut zones, 0);
        assert_eq!((zones[0].pair, zones[1].pair), (Some(1), Some(0)));
        remove(&mut zones, 1);
        assert_eq!(zones[0].pair, None);
    }

    fn world_with_box(x: f32, y: f32) -> (PhysWorld, Object) {
        let mut w = PhysWorld::new(-9.8, BorderMode::Portal, (1200.0, 1200.0));
        let h = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![x, y]));
        let c = w.colliders.insert_with_parent(ColliderBuilder::cuboid(0.2, 0.2), h, &mut w.bodies);
        (w, Object::test_stub(h, c))
    }

    #[test]
    fn portals_teleport_once_per_entry() {
        let (mut w, o) = world_with_box(2.5, 0.5);
        let mut zones = vec![];
        add(&mut zones, zone(ZoneKind::Portal, [2.0, 0.0], [3.0, 1.0]));
        add(&mut zones, zone(ZoneKind::Portal, [8.0, 4.0], [10.0, 6.0]));
        let mut state = PortalState::default();
        let t = apply(&zones, &mut state, &mut w, &[o.body]);
        assert_eq!(t.len(), 1);
        let p = *w.bodies[o.body].translation();
        assert!(zones[1].contains(p) && (p.x - 9.0).abs() < 0.01 && (p.y - 5.0).abs() < 0.01);
        // Still inside the exit: no bounce back.
        assert!(apply(&zones, &mut state, &mut w, &[o.body]).is_empty());
    }

    #[test]
    fn wind_pushes_and_float_cancels_gravity() {
        let (mut w, o) = world_with_box(5.0, 5.0);
        let zones = vec![zone(ZoneKind::Wind, [0.0, 0.0], [10.0, 10.0])];
        let mut state = PortalState::default();
        apply(&zones, &mut state, &mut w, &[o.body]);
        w.step_fixed();
        assert!(w.bodies[o.body].linvel().x > 0.2);

        let (mut w, o) = world_with_box(5.0, 5.0);
        let zones = vec![zone(ZoneKind::Float, [0.0, 0.0], [10.0, 10.0])];
        for _ in 0..60 {
            w.reset_forces();
            apply(&zones, &mut state, &mut w, &[o.body]);
            w.step_fixed();
        }
        assert!(w.bodies[o.body].translation().y >= 5.0, "floats instead of falling");
    }
}
