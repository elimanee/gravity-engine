//! Gadgets attached to objects (or fixed in the world): **lasers** whose
//! beams bounce off mirrors and pass through glass, **thrusters** that push
//! what they are attached to (or blow like a fan when fixed in the world),
//! and **cannons** that fire balls, shapes, grains or images. Each one
//! works all the time, while an arrow key is held, or on the music's beats.

use super::{to_screen, PhysWorld};
use macroquad::prelude::*;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// Most mirror bounces and glass crossings a beam makes.
const MAX_BOUNCES: usize = 32;
/// Where beams start, past the end of the emitter (m).
const MUZZLE: f32 = 0.2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GadgetKind {
    #[default]
    Laser,
    Thruster,
    Cannon,
}

impl GadgetKind {
    pub const ALL: &'static [GadgetKind] = &[GadgetKind::Laser, GadgetKind::Thruster, GadgetKind::Cannon];

    pub fn label(self) -> &'static str {
        match self {
            GadgetKind::Laser => "Laser",
            GadgetKind::Thruster => "Thruster",
            GadgetKind::Cannon => "Cannon",
        }
    }

    pub fn hint(self) -> [&'static str; 2] {
        match self {
            GadgetKind::Laser => ["Drag to aim  ·  mirrors reflect it,", "glass lets it through, it burns the rest"],
            GadgetKind::Thruster => ["Drag on an object the way to push it", "(on empty space: a fan)"],
            GadgetKind::Cannon => ["Drag to aim  ·  on an object it", "recoils; right-click a gadget removes it"],
        }
    }

    pub fn accent(self) -> Color {
        match self {
            GadgetKind::Laser => Color::from_rgba(255, 80, 90, 255),
            GadgetKind::Thruster => Color::from_rgba(255, 160, 60, 255),
            GadgetKind::Cannon => Color::from_rgba(170, 180, 200, 255),
        }
    }
}

/// When a gadget works.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Trigger {
    #[default]
    Always,
    Up,
    Down,
    Left,
    Right,
    /// On each beat of the music.
    Beat,
}

impl Trigger {
    pub const ALL: &'static [Trigger] =
        &[Trigger::Always, Trigger::Up, Trigger::Down, Trigger::Left, Trigger::Right, Trigger::Beat];

    pub fn label(self) -> &'static str {
        match self {
            Trigger::Always => "always",
            Trigger::Up => "while ↑ is held",
            Trigger::Down => "while ↓ is held",
            Trigger::Left => "while ← is held",
            Trigger::Right => "while → is held",
            Trigger::Beat => "on the beat",
        }
    }

    /// Screen direction of an arrow trigger.
    pub fn arrow(self) -> Option<Vec2> {
        match self {
            Trigger::Up => Some(vec2(0.0, -1.0)),
            Trigger::Down => Some(vec2(0.0, 1.0)),
            Trigger::Left => Some(vec2(-1.0, 0.0)),
            Trigger::Right => Some(vec2(1.0, 0.0)),
            _ => None,
        }
    }
}

/// What a cannon fires.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Ammo {
    #[default]
    Ball,
    /// The shape spawner's shape, colour and size.
    Shape,
    /// The Pour tool's grains.
    Grains,
    /// Copies of the most recently added image.
    Image,
}

impl Ammo {
    pub const ALL: &'static [Ammo] = &[Ammo::Ball, Ammo::Shape, Ammo::Grains, Ammo::Image];

    pub fn label(self) -> &'static str {
        match self {
            Ammo::Ball => "Balls",
            Ammo::Shape => "Shapes",
            Ammo::Grains => "Grains",
            Ammo::Image => "Image",
        }
    }
}

/// Everything that describes a gadget (saved in scenes and undo).
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(default)]
pub struct GadgetSpec {
    pub kind: GadgetKind,
    pub trigger: Trigger,
    pub ammo: Ammo,
    /// Thruster: thrust as a multiple of its object's weight. Cannon:
    /// muzzle speed (m/s).
    pub power: f32,
    /// Cannon: shots per second.
    pub rate: f32,
    /// Where it sits: a point on its object (m, local), or in the world.
    pub at: [f32; 2],
    /// Which way it fires or pushes (rad, counter-clockwise from +x), on
    /// its object or in the world.
    pub angle: f32,
}

impl Default for GadgetSpec {
    fn default() -> Self {
        GadgetSpec {
            kind: GadgetKind::Laser,
            trigger: Trigger::Always,
            ammo: Ammo::Ball,
            power: 2.0,
            rate: 2.0,
            at: [0.0, 0.0],
            angle: 0.0,
        }
    }
}

impl GadgetSpec {
    /// Values read from a file, kept in range.
    pub fn sanitized(mut self) -> Self {
        let ok = |v: f32, lo: f32, hi: f32, default: f32| if v.is_finite() { v.clamp(lo, hi) } else { default };
        self.power = ok(self.power, 0.0, 60.0, 2.0);
        self.rate = ok(self.rate, 0.1, 20.0, 2.0);
        self.at = [ok(self.at[0], -1e4, 1e4, 0.0), ok(self.at[1], -1e4, 1e4, 0.0)];
        self.angle = ok(self.angle, -100.0, 100.0, 0.0);
        self
    }
}

pub struct Gadget {
    pub spec: GadgetSpec,
    /// The object it is attached to (`None`: fixed in the world).
    pub host: Option<RigidBodyHandle>,
    /// Working this frame.
    pub on: bool,
    /// Cannon: seconds until it can fire again.
    pub cooldown: f32,
    /// Cannon: the objects it fired, oldest first.
    pub fired: VecDeque<RigidBodyHandle>,
    /// Beat trigger: seconds left of the current pulse.
    pub pulse: f32,
}

impl Gadget {
    pub fn new(spec: GadgetSpec, host: Option<RigidBodyHandle>) -> Self {
        Gadget { spec, host, on: false, cooldown: 0.0, fired: VecDeque::new(), pulse: 0.0 }
    }

    /// World position (m) and unit direction, or `None` when its object is gone.
    pub fn pose(&self, world: &PhysWorld) -> Option<(Point<f32>, Vector<f32>)> {
        let local = Point::new(self.spec.at[0], self.spec.at[1]);
        let (p, a) = match self.host {
            Some(h) => {
                let b = world.bodies.get(h)?;
                (b.position() * local, b.rotation().angle() + self.spec.angle)
            }
            None => (local, self.spec.angle),
        };
        Some((p, vector![a.cos(), a.sin()]))
    }

    /// Where its beam or shots come out (m).
    pub fn muzzle(&self, world: &PhysWorld) -> Option<(Point<f32>, Vector<f32>)> {
        let (p, d) = self.pose(world)?;
        Some((p + d * MUZZLE, d))
    }
}

/// What a beam does where it meets something.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Surface {
    /// It stops there (and heats and pushes it).
    Stop,
    Mirror,
    /// It goes through, taking some of this colour.
    Glass(Color),
}

pub const LASER_COLOUR: Color = Color::new(1.0, 0.18, 0.22, 1.0);

/// A laser beam: its corners (m), the colour after each corner, and what
/// it ends on.
pub struct Beam {
    pub points: Vec<Point<f32>>,
    pub colours: Vec<Color>,
    /// The collider it stopped on, where, and its direction there.
    pub hit: Option<(ColliderHandle, Point<f32>, Vector<f32>)>,
}

impl Beam {
    /// Whether any part of the beam crosses the rectangle `min`–`max` (m).
    pub fn crosses(&self, min: [f32; 2], max: [f32; 2]) -> bool {
        self.points.windows(2).any(|w| segment_hits_rect(w[0], w[1], min, max))
    }
}

/// Liang–Barsky: whether segment `a`–`b` touches the box.
fn segment_hits_rect(a: Point<f32>, b: Point<f32>, min: [f32; 2], max: [f32; 2]) -> bool {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-d.x, a.x - min[0]), (d.x, max[0] - a.x), (-d.y, a.y - min[1]), (d.y, max[1] - a.y)] {
        if p == 0.0 {
            if q < 0.0 {
                return false;
            }
        } else {
            let r = q / p;
            if p < 0.0 {
                t0 = t0.max(r);
            } else {
                t1 = t1.min(r);
            }
        }
    }
    t0 <= t1
}

/// Follow a beam from `from` along `dir` (m), not hitting `skip` on its way
/// out. `surface` says what each collider does to it.
pub fn trace(
    world: &PhysWorld,
    from: Point<f32>,
    dir: Vector<f32>,
    skip: Option<RigidBodyHandle>,
    surface: impl Fn(ColliderHandle) -> Surface,
) -> Beam {
    let reach = (world.arena.0.powi(2) + world.arena.1.powi(2)).sqrt() * 2.0 + 10.0;
    let mut beam = Beam { points: vec![from], colours: vec![], hit: None };
    let (mut p, mut d) = (from, dir.try_normalize(1e-6).unwrap_or(vector![1.0, 0.0]));
    let mut colour = LASER_COLOUR;
    let mut last: Option<ColliderHandle> = None;
    for bounce in 0..MAX_BOUNCES {
        let mut filter = QueryFilter::default().exclude_sensors();
        filter.exclude_collider = last;
        if bounce == 0 {
            filter.exclude_rigid_body = skip;
        }
        let Some((c, toi, n)) = world.cast_ray(p, d, reach, filter) else {
            beam.points.push(p + d * reach);
            beam.colours.push(colour);
            return beam;
        };
        let q = p + d * toi;
        beam.points.push(q);
        beam.colours.push(colour);
        match surface(c) {
            Surface::Mirror => {
                d -= n * (2.0 * d.dot(&n));
                p = q + d * 1e-3;
                last = Some(c);
            }
            Surface::Glass(tint) => {
                // Out through the far side, tinted on the way.
                let inside = Ray::new(q + d * 1e-3, d);
                let out = world
                    .colliders
                    .get(c)
                    .and_then(|col| col.shape().cast_ray(col.position(), &inside, reach, false))
                    .unwrap_or(0.0);
                colour = Color::new(
                    colour.r * 0.4 + tint.r * 0.6,
                    colour.g * 0.4 + tint.g * 0.6,
                    colour.b * 0.4 + tint.b * 0.6,
                    1.0,
                );
                let exit = inside.origin + d * out;
                beam.points.push(exit);
                beam.colours.push(colour);
                p = exit + d * 1e-3;
                last = Some(c);
            }
            Surface::Stop => {
                beam.hit = Some((c, q, d));
                return beam;
            }
        }
    }
    beam
}

/// Draw a beam (world pass): a soft glow, the beam and a bright core.
pub fn draw_beam(beam: &Beam, t: f32) {
    let flicker = 0.85 + 0.15 * (t * 37.0).sin();
    for (w, c) in beam.points.windows(2).zip(&beam.colours) {
        let (a, b) = (to_screen(w[0].x, w[0].y), to_screen(w[1].x, w[1].y));
        draw_line(a.x, a.y, b.x, b.y, 12.0, Color { a: 0.08 * flicker, ..*c });
        draw_line(a.x, a.y, b.x, b.y, 5.0, Color { a: 0.4 * flicker, ..*c });
        let core = Color::new(1.0, 0.75 + c.g * 0.25, 0.75 + c.b * 0.25, 0.95);
        draw_line(a.x, a.y, b.x, b.y, 1.8, core);
    }
    if let Some((_, q, _)) = beam.hit {
        let p = to_screen(q.x, q.y);
        let c = *beam.colours.last().unwrap_or(&LASER_COLOUR);
        draw_circle(p.x, p.y, 7.0 * flicker, Color { a: 0.25, ..c });
        draw_circle(p.x, p.y, 3.0, Color::new(1.0, 0.95, 0.9, 0.9));
    }
}

/// Draw a gadget at `at` (world px) pointing along `dir` (screen space).
pub fn draw_gadget(kind: GadgetKind, at: Vec2, dir: Vec2, on: bool, t: f32) {
    let n = vec2(-dir.y, dir.x);
    let dark = Color::from_rgba(44, 46, 60, 255);
    let rim = Color::from_rgba(150, 156, 180, 255);
    let quad = |c: Vec2, len: f32, half: f32, colour: Color| {
        let (a, b) = (c - dir * len / 2.0, c + dir * len / 2.0);
        draw_triangle(a + n * half, a - n * half, b + n * half, colour);
        draw_triangle(a - n * half, b - n * half, b + n * half, colour);
    };
    match kind {
        GadgetKind::Laser => {
            quad(at, 22.0, 6.5, rim);
            quad(at, 19.0, 5.0, dark);
            let lens = at + dir * 10.0;
            let glow = if on { 1.0 } else { 0.35 };
            draw_circle(lens.x, lens.y, 3.6, Color { a: glow, ..LASER_COLOUR });
            draw_circle(lens.x, lens.y, 1.4, Color::new(1.0, 0.9, 0.9, glow));
        }
        GadgetKind::Thruster => {
            // A bell that opens backwards, away from where it pushes.
            let (front, back) = (at + dir * 6.0, at - dir * 10.0);
            draw_triangle(front + n * 4.0, front - n * 4.0, back + n * 8.0, rim);
            draw_triangle(front - n * 4.0, back - n * 8.0, back + n * 8.0, rim);
            draw_line(back.x + n.x * 8.0, back.y + n.y * 8.0, back.x - n.x * 8.0, back.y - n.y * 8.0, 2.0, dark);
            if on {
                let k = 0.7 + 0.3 * (t * 50.0).sin();
                let tip = back - dir * 14.0 * k;
                draw_triangle(back + n * 5.0, back - n * 5.0, tip, Color::new(1.0, 0.85, 0.4, 0.9));
            }
        }
        GadgetKind::Cannon => {
            quad(at + dir * 9.0, 22.0, 5.5, dark);
            quad(at + dir * 18.0, 4.0, 6.5, rim);
            draw_circle(at.x, at.y, 8.5, rim);
            draw_circle(at.x, at.y, 6.5, dark);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;

    fn wall(w: &mut PhysWorld, x: f32, y: f32, hw: f32, hh: f32, angle: f32) -> ColliderHandle {
        let b = w.bodies.insert(RigidBodyBuilder::fixed().translation(vector![x, y]).rotation(angle));
        w.colliders.insert_with_parent(ColliderBuilder::cuboid(hw, hh), b, &mut w.bodies)
    }

    #[test]
    fn beams_reflect_off_mirrors_and_cross_glass() {
        let mut w = PhysWorld::new(0.0, BorderMode::Portal, (1200.0, 1200.0));
        // A 45° mirror turns a beam going right upwards.
        let mirror = wall(&mut w, 10.0, 5.0, 1.0, 0.05, std::f32::consts::FRAC_PI_4);
        // A glass pane above it, then a wall.
        let glass = wall(&mut w, 10.0, 9.0, 1.0, 0.2, 0.0);
        let stop = wall(&mut w, 10.0, 14.0, 1.0, 0.2, 0.0);
        w.refresh_queries();
        let surface = |c: ColliderHandle| {
            if c == mirror {
                Surface::Mirror
            } else if c == glass {
                Surface::Glass(Color::new(0.2, 0.9, 1.0, 1.0))
            } else {
                Surface::Stop
            }
        };
        let beam = trace(&w, point![2.0, 5.0], vector![1.0, 0.0], None, surface);
        let (c, q, d) = beam.hit.expect("ends on the wall");
        assert_eq!(c, stop);
        assert!((q.x - 10.0).abs() < 0.1 && (q.y - 13.8).abs() < 0.05, "hits the wall above: {q:?}");
        assert!(d.y > 0.99, "going up");
        assert_eq!(beam.points.len(), 5, "start, mirror, into and out of the glass, wall");
        let last = beam.colours.last().unwrap();
        assert!(last.g > 0.5 && last.r < 0.8, "tinted by the glass: {last:?}");
        assert!(beam.crosses([9.5, 11.0], [10.5, 12.0]));
        assert!(!beam.crosses([12.0, 11.0], [13.0, 12.0]));
    }

    #[test]
    fn gadgets_follow_their_object() {
        let mut w = PhysWorld::new(0.0, BorderMode::Portal, (1200.0, 1200.0));
        let b = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![5.0, 5.0]).rotation(1.0));
        let spec = GadgetSpec { at: [1.0, 0.0], angle: 0.5, ..GadgetSpec::default() };
        let g = Gadget::new(spec, Some(b));
        let (p, d) = g.pose(&w).unwrap();
        assert!((p - point![5.0 + 1f32.cos(), 5.0 + 1f32.sin()]).norm() < 1e-4);
        assert!((d - vector![1.5f32.cos(), 1.5f32.sin()]).norm() < 1e-4);
        w.remove_body(b);
        assert!(g.pose(&w).is_none(), "gone with its object");
        let wild = GadgetSpec { power: f32::NAN, rate: -3.0, ..GadgetSpec::default() }.sanitized();
        assert_eq!((wild.power, wild.rate), (2.0, 0.1));
    }
}
