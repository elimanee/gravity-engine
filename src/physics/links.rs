//! Links made with the Link tool: ropes, springs and hinges between two
//! objects, or between an object and the background.

use super::{to_screen, PhysWorld};
use macroquad::prelude::*;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum LinkKind {
    Rope,
    Spring,
    Hinge,
}

impl LinkKind {
    pub const ALL: &'static [LinkKind] = &[LinkKind::Rope, LinkKind::Spring, LinkKind::Hinge];

    pub fn label(self) -> &'static str {
        match self {
            LinkKind::Rope => "Rope",
            LinkKind::Spring => "Spring",
            LinkKind::Hinge => "Hinge",
        }
    }

    /// Two-line usage hint for the tool card.
    pub fn hint(self) -> [&'static str; 2] {
        match self {
            LinkKind::Rope | LinkKind::Spring => ["Drag from one object to another,", "or to empty space to hang it"],
            LinkKind::Hinge => ["Click where two objects overlap,", "or on one to nail it in place"],
        }
    }

    pub fn accent(self) -> Color {
        match self {
            LinkKind::Rope => Color::from_rgba(226, 190, 140, 255),
            LinkKind::Spring => Color::from_rgba(120, 220, 255, 255),
            LinkKind::Hinge => Color::from_rgba(230, 230, 240, 255),
        }
    }
}

/// Spring stiffness and damping, as accelerations (independent of the masses).
const SPRING_K: f32 = 60.0;
const SPRING_C: f32 = 2.4;
/// A spring's slack safety rope, relative to its rest length.
const SPRING_MAX_STRETCH: f32 = 3.0;

pub struct Link {
    pub kind: LinkKind,
    pub a: RigidBodyHandle,
    /// `None`: attached to the background.
    pub b: Option<RigidBodyHandle>,
    /// Anchor in `a`'s local frame.
    pub la: Point<f32>,
    /// Anchor in `b`'s local frame, or a world point for the background.
    pub lb: Point<f32>,
    /// Rope length / spring rest length (m).
    pub length: f32,
    pub joint: ImpulseJointHandle,
}

impl Link {
    /// Join `a` and `b` (or the background) at world points `pa` and `pb`.
    pub fn new(
        world: &mut PhysWorld,
        kind: LinkKind,
        a: RigidBodyHandle,
        b: Option<RigidBodyHandle>,
        pa: Point<f32>,
        pb: Point<f32>,
    ) -> Option<Self> {
        let la = world.bodies.get(a)?.position().inverse_transform_point(&pa);
        let lb = match b {
            Some(b) => world.bodies.get(b)?.position().inverse_transform_point(&pb),
            None => pb,
        };
        let length = (pa - pb).norm().max(0.05);
        Some(Self::restore(world, kind, a, b, la, lb, length))
    }

    /// Recreate a link from its local anchors (scenes, undo).
    pub fn restore(
        world: &mut PhysWorld,
        kind: LinkKind,
        a: RigidBodyHandle,
        b: Option<RigidBodyHandle>,
        la: Point<f32>,
        lb: Point<f32>,
        length: f32,
    ) -> Self {
        let data: GenericJoint = match kind {
            LinkKind::Rope => RopeJointBuilder::new(length).local_anchor1(la).local_anchor2(lb).build().into(),
            // Rapier's spring joint ignores its rest length, so springs are
            // forces (see `apply_springs`) plus a slack rope that stops them
            // from stretching forever.
            LinkKind::Spring => RopeJointBuilder::new(length * SPRING_MAX_STRETCH + 1.0)
                .local_anchor1(la)
                .local_anchor2(lb)
                .build()
                .into(),
            LinkKind::Hinge => {
                RevoluteJointBuilder::new().local_anchor1(la).local_anchor2(lb).contacts_enabled(false).build().into()
            }
        };
        let joint = world.impulse_joints.insert(a, b.unwrap_or(world.ground), data, true);
        Link { kind, a, b, la, lb, length, joint }
    }

    /// Both anchors in world space.
    pub fn ends(&self, world: &PhysWorld) -> Option<(Point<f32>, Point<f32>)> {
        let pa = world.bodies.get(self.a)?.position() * self.la;
        let pb = match self.b {
            Some(b) => world.bodies.get(b)?.position() * self.lb,
            None => self.lb,
        };
        Some((pa, pb))
    }

    pub fn involves(&self, body: RigidBodyHandle) -> bool {
        self.a == body || self.b == Some(body)
    }

    pub fn remove(&self, world: &mut PhysWorld) {
        world.impulse_joints.remove(self.joint, true);
    }

    pub fn draw(&self, world: &PhysWorld) {
        let Some((pa, pb)) = self.ends(world) else { return };
        let (a, b) = (to_screen(pa.x, pa.y), to_screen(pb.x, pb.y));
        let c = self.kind.accent();
        let shade = Color::new(0.0, 0.0, 0.0, 0.3);
        match self.kind {
            LinkKind::Rope => {
                // Sag when slack: the curve keeps roughly the rope's length.
                let d = a.distance(b);
                let len = self.length * crate::config::PPM;
                let sag = if d < len { ((len * len - d * d).sqrt() * 0.5).min(len) } else { 0.0 };
                let mid = (a + b) / 2.0 + vec2(0.0, sag);
                let mut prev = a;
                for i in 1..=16 {
                    let u = i as f32 / 16.0;
                    let p = a.lerp(mid, u).lerp(mid.lerp(b, u), u);
                    draw_line(prev.x + 1.0, prev.y + 2.0, p.x + 1.0, p.y + 2.0, 3.0, shade);
                    draw_line(prev.x, prev.y, p.x, p.y, 2.5, c);
                    prev = p;
                }
            }
            LinkKind::Spring => {
                let d = b - a;
                let len = d.length();
                if len > 1.0 {
                    let dir = d / len;
                    let n = vec2(-dir.y, dir.x);
                    let coils = 12;
                    let lead = (len * 0.12).min(12.0);
                    let mut prev = a + dir * lead;
                    draw_line(a.x, a.y, prev.x, prev.y, 2.0, c);
                    for i in 1..=coils * 2 {
                        let u = i as f32 / (coils * 2) as f32;
                        let side = if i == coils * 2 {
                            0.0
                        } else if i % 2 == 1 {
                            1.0
                        } else {
                            -1.0
                        };
                        let p = a + dir * (lead + (len - 2.0 * lead) * u) + n * side * 6.0;
                        draw_line(prev.x, prev.y, p.x, p.y, 2.0, c);
                        prev = p;
                    }
                    draw_line(prev.x, prev.y, b.x, b.y, 2.0, c);
                }
            }
            LinkKind::Hinge => {}
        }
        for (p, attached_to_bg) in [(a, false), (b, self.b.is_none())] {
            if self.kind == LinkKind::Hinge && p != a {
                continue;
            }
            draw_circle(p.x, p.y + 1.0, 5.5, shade);
            draw_circle(p.x, p.y, 5.0, if attached_to_bg { Color::from_rgba(120, 118, 140, 255) } else { c });
            draw_circle(p.x, p.y, 2.0, Color::new(0.15, 0.14, 0.2, 1.0));
        }
    }

    /// Distance (px) from the screen point `m` to this link's drawn line.
    pub fn distance_px(&self, world: &PhysWorld, m: Vec2) -> f32 {
        let Some((pa, pb)) = self.ends(world) else { return f32::MAX };
        let (a, b) = (to_screen(pa.x, pa.y), to_screen(pb.x, pb.y));
        let ab = b - a;
        let t = if ab.length_squared() > 0.0 { ((m - a).dot(ab) / ab.length_squared()).clamp(0.0, 1.0) } else { 0.0 };
        m.distance(a + ab * t)
    }
}

/// Spring forces, applied every frame before the physics step. Stiffness is
/// scaled by the reduced mass, so light and heavy objects bounce alike.
pub fn apply_springs(links: &[Link], world: &mut PhysWorld) {
    for l in links.iter().filter(|l| l.kind == LinkKind::Spring) {
        let Some((pa, pb)) = l.ends(world) else { continue };
        let d = pb - pa;
        let len = d.norm();
        if len < 1e-4 {
            continue;
        }
        let dir = d / len;
        let body = |h: Option<RigidBodyHandle>| h.and_then(|h| world.bodies.get(h)).filter(|b| b.is_dynamic());
        let (ba, bb) = (body(Some(l.a)), body(l.b));
        let va = ba.map_or(vector![0.0, 0.0], |b| b.velocity_at_point(&pa));
        let vb = bb.map_or(vector![0.0, 0.0], |b| b.velocity_at_point(&pb));
        let m = match (ba.map(|b| b.mass()), bb.map(|b| b.mass())) {
            (Some(ma), Some(mb)) => ma * mb / (ma + mb).max(1e-6),
            (Some(m), None) | (None, Some(m)) => m,
            (None, None) => continue,
        };
        let stretch = len - l.length;
        let closing = (vb - va).dot(&dir);
        let f = dir * (SPRING_K * stretch + SPRING_C * closing) * m;
        if let Some(b) = world.bodies.get_mut(l.a).filter(|b| b.is_dynamic()) {
            b.add_force_at_point(f, pa, true);
        }
        if let Some(b) = l.b.and_then(|h| world.bodies.get_mut(h)).filter(|b| b.is_dynamic()) {
            b.add_force_at_point(-f, pb, true);
        }
    }
}

/// Drop links whose joint no longer exists (an attached object was removed).
pub fn prune(links: &mut Vec<Link>, world: &PhysWorld) {
    links.retain(|l| world.impulse_joints.contains(l.joint));
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;

    fn world_with_ball(y: f32) -> (PhysWorld, RigidBodyHandle) {
        let mut w = PhysWorld::new(-9.8, BorderMode::Portal, (1200.0, 1200.0));
        let h = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![10.0, y]));
        w.colliders.insert_with_parent(ColliderBuilder::ball(0.3), h, &mut w.bodies);
        (w, h)
    }

    #[test]
    fn rope_holds_an_object_below_its_anchor() {
        let (mut w, h) = world_with_ball(10.0);
        let link = Link::new(&mut w, LinkKind::Rope, h, None, point![10.0, 10.0], point![10.0, 12.0]).unwrap();
        for _ in 0..240 {
            w.step_fixed();
        }
        let y = w.bodies[h].translation().y;
        assert!((y - 10.0).abs() < 0.1, "hangs at the rope's length, got y = {y}");
        assert!(link.ends(&w).is_some());
    }

    #[test]
    fn spring_settles_near_its_rest_length() {
        let (mut w, h) = world_with_ball(10.0);
        let links = vec![Link::new(&mut w, LinkKind::Spring, h, None, point![10.0, 10.0], point![10.0, 12.0]).unwrap()];
        for _ in 0..1200 {
            w.reset_forces();
            apply_springs(&links, &mut w);
            w.step_fixed();
        }
        let y = w.bodies[h].translation().y;
        // Rest length 2 m, stretched by g / k.
        let expected = 12.0 - 2.0 - 9.8 / SPRING_K;
        assert!((y - expected).abs() < 0.15, "expected ≈{expected}, got {y}");
    }

    #[test]
    fn removing_a_body_prunes_its_links() {
        let (mut w, h) = world_with_ball(10.0);
        let mut links =
            vec![Link::new(&mut w, LinkKind::Hinge, h, None, point![10.0, 10.0], point![10.0, 10.0]).unwrap()];
        w.remove_body(h);
        prune(&mut links, &w);
        assert!(links.is_empty());
    }
}
