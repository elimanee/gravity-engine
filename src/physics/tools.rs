//! Interaction tools (formerly "drag modes").

use super::to_phys;
use crate::config::PPM;
use macroquad::prelude::Color;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Tool {
    Spring,
    Slingshot,
    Pull,
    Push,
    Vortex,
    Freeze,
    Orbit,
    Bomb,
}

/// Field forces are expressed as accelerations for an object of this mass, so
/// small and large objects respond consistently.
const NOMINAL_MASS: f32 = 2.5;

impl Tool {
    pub const ALL: &'static [Tool] =
        &[Tool::Spring, Tool::Slingshot, Tool::Pull, Tool::Push, Tool::Vortex, Tool::Freeze, Tool::Orbit, Tool::Bomb];

    pub fn label(self) -> &'static str {
        match self {
            Tool::Spring => "Spring",
            Tool::Slingshot => "Slingshot",
            Tool::Pull => "Pull",
            Tool::Push => "Push",
            Tool::Vortex => "Vortex",
            Tool::Freeze => "Freeze",
            Tool::Orbit => "Orbit",
            Tool::Bomb => "Bomb",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Tool::Spring => "Grab, carry and throw",
            Tool::Slingshot => "Pull back, release to launch",
            Tool::Pull => "Attract everything nearby",
            Tool::Push => "Repel everything nearby",
            Tool::Vortex => "Spin objects around the cursor",
            Tool::Freeze => "Slow objects to a crawl",
            Tool::Orbit => "Make objects circle the cursor",
            Tool::Bomb => "Click to detonate",
        }
    }

    pub fn accent(self) -> Color {
        let (r, g, b) = match self {
            Tool::Spring => (255, 212, 70),
            Tool::Slingshot => (236, 96, 220),
            Tool::Pull => (90, 170, 255),
            Tool::Push => (255, 110, 80),
            Tool::Vortex => (96, 230, 180),
            Tool::Freeze => (170, 220, 255),
            Tool::Orbit => (255, 190, 90),
            Tool::Bomb => (255, 96, 48),
        };
        Color::from_rgba(r, g, b, 255)
    }

    /// Continuous area-of-effect tools (active while the button is held).
    pub fn is_field(self) -> bool {
        matches!(self, Tool::Pull | Tool::Push | Tool::Vortex | Tool::Freeze | Tool::Orbit)
    }

    /// Tools that expose radius / strength settings.
    pub fn has_settings(self) -> bool {
        self.is_field() || self == Tool::Bomb
    }
}

/// An object held by the Spring / Slingshot tools (or a pinned object being moved).
pub struct Grab {
    pub body: RigidBodyHandle,
    /// Grab point in the body's local frame.
    pub local: Point<f32>,
    /// World position of the body when grabbed (slingshot anchor).
    pub anchor: (f32, f32),
    pub tool: Tool,
}

impl Grab {
    pub fn new(bodies: &RigidBodySet, body: RigidBodyHandle, cursor_px: (f32, f32), tool: Tool) -> Self {
        let b = &bodies[body];
        let (cx, cy) = to_phys(cursor_px.0, cursor_px.1);
        let local = if tool == Tool::Spring || !b.is_dynamic() {
            b.position().inverse_transform_point(&point![cx, cy])
        } else {
            point![0.0, 0.0]
        };
        Grab { body, local, anchor: (b.translation().x, b.translation().y), tool }
    }

    /// Current grab point in world space.
    pub fn world_point(&self, bodies: &RigidBodySet) -> Option<Point<f32>> {
        bodies.get(self.body).map(|b| b.position() * self.local)
    }

    /// Apply this frame's pull toward the cursor.
    pub fn apply(&self, bodies: &mut RigidBodySet, cursor_px: (f32, f32)) {
        let (cx, cy) = to_phys(cursor_px.0, cursor_px.1);
        let Some(b) = bodies.get_mut(self.body) else { return };

        if b.is_kinematic() {
            // Pinned objects are moved directly.
            let offset = b.rotation() * self.local.coords;
            b.set_next_kinematic_translation(vector![cx, cy] - offset);
            return;
        }

        let m = b.mass();
        let p = b.position() * self.local;
        let (k, c) = match self.tool {
            Tool::Slingshot => (900.0, 54.0),
            _ => (320.0, 28.0),
        };
        let v = b.velocity_at_point(&p);
        let d = vector![cx - p.x, cy - p.y];
        let force = (d * k - v * c) * m;
        b.add_force_at_point(force, p, true);
    }

    /// Release: the slingshot launches away from where it was pulled.
    pub fn release(&self, bodies: &mut RigidBodySet, cursor_px: (f32, f32)) {
        let Some(b) = bodies.get_mut(self.body) else { return };
        if self.tool == Tool::Slingshot && b.is_dynamic() {
            let (mx, my) = to_phys(cursor_px.0, cursor_px.1);
            let (ax, ay) = self.anchor;
            b.set_linvel(vector![(ax - mx) * 5.5, (ay - my) * 5.5], true);
        }
        if b.is_kinematic() {
            b.set_linvel(vector![0.0, 0.0], false);
        }
    }
}

/// Force exerted by a field tool on one body.
pub fn field_force(
    tool: Tool,
    body: &RigidBody,
    cursor: (f32, f32),
    radius_px: f32,
    strength: f32,
) -> Option<Vector<f32>> {
    let pos = body.translation();
    let dx = cursor.0 - pos.x;
    let dy = cursor.1 - pos.y;
    let dist = (dx * dx + dy * dy).sqrt().max(0.1);
    if dist > radius_px / PPM {
        return None;
    }
    let (nx, ny) = (dx / dist, dy / dist);
    let fv = strength / NOMINAL_MASS;
    let mag = (fv / (dist * dist)).min(fv * 3.0);
    let acc = match tool {
        Tool::Pull => vector![nx * mag, ny * mag],
        Tool::Push => vector![-nx * mag, -ny * mag],
        Tool::Vortex => vector![-ny * mag, nx * mag],
        Tool::Orbit => {
            let inward = (fv * 0.6 / (dist * dist)).min(fv * 2.0);
            let tangent = (fv * 0.4 / dist).min(fv * 1.5);
            vector![nx * inward - ny * tangent, ny * inward + nx * tangent]
        }
        Tool::Freeze => {
            // Explicit damping; clamp so a single step can never reverse velocity.
            let k = (fv * 0.15).min(45.0);
            -*body.linvel() * k
        }
        _ => return None,
    };
    Some(acc * body.mass())
}

/// Apply a field tool to every dynamic body.
pub fn apply_field(
    bodies: &mut RigidBodySet,
    handles: &[RigidBodyHandle],
    tool: Tool,
    cursor_px: (f32, f32),
    radius_px: f32,
    strength: f32,
) {
    let cursor = to_phys(cursor_px.0, cursor_px.1);
    for &h in handles {
        let Some(b) = bodies.get_mut(h) else { continue };
        if !b.is_dynamic() {
            continue;
        }
        if let Some(f) = field_force(tool, b, cursor, radius_px, strength) {
            b.add_force(f, true);
            if tool == Tool::Freeze {
                let w = b.angvel();
                b.set_angvel(w * 0.9, true);
            }
        }
    }
}

/// Instant radial explosion (impulse-based, so light objects fly further).
pub fn detonate(
    bodies: &mut RigidBodySet,
    handles: &[RigidBodyHandle],
    cursor_px: (f32, f32),
    radius_px: f32,
    strength: f32,
) {
    let (bx, by) = to_phys(cursor_px.0, cursor_px.1);
    for &h in handles {
        let Some(b) = bodies.get_mut(h) else { continue };
        if !b.is_dynamic() {
            continue;
        }
        let pos = *b.translation();
        let (dx, dy) = (pos.x - bx, pos.y - by);
        let dist = (dx * dx + dy * dy).sqrt().max(0.1);
        if dist <= radius_px / PPM {
            let mag = (strength * 4.0 / (dist * dist)).min(strength * 15.0);
            b.apply_impulse(vector![dx / dist * mag, dy / dist * mag], true);
            b.apply_torque_impulse((macroquad::rand::gen_range(-1.0f32, 1.0)) * mag * 0.05, true);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn body_at(x: f32, y: f32, mass: f32) -> RigidBody {
        let mut b = RigidBodyBuilder::dynamic().translation(vector![x, y]).build();
        b.set_additional_mass(mass, false);
        b
    }

    #[test]
    fn pull_attracts_push_repels() {
        let mut set = RigidBodySet::new();
        let h = set.insert(body_at(0.0, 0.0, 1.0));
        let mut cols = ColliderSet::new();
        cols.insert_with_parent(ColliderBuilder::ball(0.5), h, &mut set);
        let b = &set[h];
        let pull = field_force(Tool::Pull, b, (1.0, 0.0), 600.0, 300.0).unwrap();
        let push = field_force(Tool::Push, b, (1.0, 0.0), 600.0, 300.0).unwrap();
        assert!(pull.x > 0.0 && push.x < 0.0);
        assert!(field_force(Tool::Pull, b, (100.0, 0.0), 60.0, 300.0).is_none());
    }
}
