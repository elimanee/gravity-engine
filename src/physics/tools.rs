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
    Swing,
    Draw,
    Link,
    Zone,
    Select,
    Pour,
    Knife,
    Fire,
    Gadget,
}

/// What the bottom-left tool card shows for a tool.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Card {
    None,
    /// Radius and strength.
    Area,
    /// Thickness, colour and pinning.
    Draw,
    /// Rope / spring / hinge / motor.
    Link,
    /// Wind / float / portal.
    Zone,
    /// Selection count and actions.
    Select,
    /// Sand / liquid / beads.
    Pour,
    /// Laser / thruster / cannon, trigger and settings.
    Gadget,
}

/// Field forces are expressed as accelerations for an object of this mass, so
/// small and large objects respond consistently.
const NOMINAL_MASS: f32 = 2.5;

impl Tool {
    pub const ALL: &'static [Tool] = &[
        Tool::Spring,
        Tool::Slingshot,
        Tool::Pull,
        Tool::Push,
        Tool::Vortex,
        Tool::Freeze,
        Tool::Orbit,
        Tool::Bomb,
        Tool::Swing,
        Tool::Draw,
        Tool::Link,
        Tool::Zone,
        Tool::Select,
        Tool::Pour,
        Tool::Knife,
        Tool::Fire,
        Tool::Gadget,
    ];

    /// Keyboard shortcut.
    pub fn key(self) -> &'static str {
        match self {
            Tool::Spring => "1",
            Tool::Slingshot => "2",
            Tool::Pull => "3",
            Tool::Push => "4",
            Tool::Vortex => "5",
            Tool::Freeze => "6",
            Tool::Orbit => "7",
            Tool::Bomb => "8",
            Tool::Swing => "9",
            Tool::Draw => "0",
            Tool::Link => "J",
            Tool::Zone => "Z",
            Tool::Select => "S",
            Tool::Pour => "K",
            Tool::Knife => "C",
            Tool::Fire => "Y",
            Tool::Gadget => "L",
        }
    }

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
            Tool::Swing => "Swing",
            Tool::Draw => "Draw",
            Tool::Link => "Link",
            Tool::Zone => "Zone",
            Tool::Select => "Select",
            Tool::Pour => "Pour",
            Tool::Knife => "Knife",
            Tool::Fire => "Fire",
            Tool::Gadget => "Gadgets",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            Tool::Spring => "Grab, carry and throw (no spin)",
            Tool::Slingshot => "Pull back, release to launch",
            Tool::Pull => "Attract everything nearby",
            Tool::Push => "Repel everything nearby",
            Tool::Vortex => "Spin objects around the cursor",
            Tool::Freeze => "Slow objects to a crawl",
            Tool::Orbit => "Make objects circle the cursor",
            Tool::Bomb => "Click to detonate",
            Tool::Swing => "Hold by a point, throw it spinning",
            Tool::Draw => "Draw shapes and planks",
            Tool::Link => "Ropes, springs, motors, glue",
            Tool::Zone => "Wind, float and portal areas",
            Tool::Select => "Select, move, copy and glue",
            Tool::Pour => "Pour sand, liquid and beads",
            Tool::Knife => "Cut ropes, joints and objects",
            Tool::Fire => "Set things on fire, melt ice",
            Tool::Gadget => "Lasers, thrusters, lamps, hooks…",
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
            Tool::Swing => (170, 236, 90),
            Tool::Draw => (255, 140, 190),
            Tool::Link => (226, 190, 140),
            Tool::Zone => (120, 210, 255),
            Tool::Select => (235, 235, 245),
            Tool::Pour => (232, 196, 120),
            Tool::Knife => (220, 235, 255),
            Tool::Fire => (255, 128, 40),
            Tool::Gadget => (255, 96, 120),
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

    pub fn card(self) -> Card {
        match self {
            _ if self.has_settings() => Card::Area,
            Tool::Draw => Card::Draw,
            Tool::Link => Card::Link,
            Tool::Zone => Card::Zone,
            Tool::Select => Card::Select,
            Tool::Pour => Card::Pour,
            Tool::Gadget => Card::Gadget,
            _ => Card::None,
        }
    }
}

/// Tools that hold a single object.
impl Tool {
    pub fn grabs(self) -> bool {
        matches!(self, Tool::Spring | Tool::Slingshot | Tool::Swing)
    }
}

/// An object held by the Spring / Slingshot / Swing tools (or a pinned object
/// being moved). Positions are physics metres.
pub struct Grab {
    pub body: RigidBodyHandle,
    /// Grab point in the body's local frame.
    pub local: Point<f32>,
    /// World position of the body when grabbed (slingshot anchor).
    pub anchor: (f32, f32),
    pub tool: Tool,
}

impl Grab {
    pub fn new(bodies: &RigidBodySet, body: RigidBodyHandle, cursor: (f32, f32), tool: Tool) -> Self {
        let b = &bodies[body];
        let (cx, cy) = cursor;
        // Swing holds the object by the clicked point, so pulling it off-centre
        // makes it rotate; Spring and Slingshot pull the centre of mass (no
        // torque). Pinned objects keep their offset so they don't jump.
        let local = if tool == Tool::Swing || !b.is_dynamic() {
            b.position().inverse_transform_point(&point![cx, cy])
        } else {
            b.position().inverse_transform_point(b.center_of_mass())
        };
        Grab { body, local, anchor: (b.translation().x, b.translation().y), tool }
    }

    /// Current grab point in world space.
    pub fn world_point(&self, bodies: &RigidBodySet) -> Option<Point<f32>> {
        bodies.get(self.body).map(|b| b.position() * self.local)
    }

    /// Apply this frame's pull toward the cursor.
    pub fn apply(&self, bodies: &mut RigidBodySet, cursor: (f32, f32)) {
        let (cx, cy) = cursor;
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
    pub fn release(&self, bodies: &mut RigidBodySet, cursor: (f32, f32)) {
        let Some(b) = bodies.get_mut(self.body) else { return };
        if self.tool == Tool::Slingshot && b.is_dynamic() {
            let (mx, my) = cursor;
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
/// Returns each body hit with the speed it was kicked to (m/s).
pub fn detonate(
    bodies: &mut RigidBodySet,
    handles: &[RigidBodyHandle],
    cursor_px: (f32, f32),
    radius_px: f32,
    strength: f32,
) -> Vec<(RigidBodyHandle, f32)> {
    let mut hits = vec![];
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
            hits.push((h, mag / b.mass().max(1e-3)));
        }
    }
    hits
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

    /// Grab a 2 m × 1 m box near its corner, drag it sideways for a second
    /// and return how far it rotated.
    fn rotation_after_drag(tool: Tool) -> f32 {
        use crate::physics::borders::BorderMode;
        use crate::physics::PhysWorld;
        // Portal mode has no walls, and gravity is off: only the grab acts.
        let mut w = PhysWorld::new(0.0, BorderMode::Portal, (1200.0, 1200.0));
        let h = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![10.0, 10.0]));
        w.colliders.insert_with_parent(ColliderBuilder::cuboid(1.0, 0.5), h, &mut w.bodies);

        let grab = Grab::new(&w.bodies, h, (10.9, 10.4), tool);
        for _ in 0..60 {
            w.reset_forces();
            grab.apply(&mut w.bodies, (14.0, 10.4));
            w.step_fixed();
        }
        grab.release(&mut w.bodies, (14.0, 10.4));
        w.bodies[h].rotation().angle().abs()
    }

    #[test]
    fn spring_does_not_spin_but_swing_does() {
        assert!(rotation_after_drag(Tool::Spring) < 1e-3, "Spring must pull the centre of mass");
        assert!(rotation_after_drag(Tool::Swing) > 0.2, "Swing must rotate the object");
    }
}
