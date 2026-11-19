//! Gadgets attached to objects (or fixed in the world): **lasers** whose
//! beams bounce off mirrors and pass through glass, **thrusters** that push
//! what they are attached to (or blow like a fan when fixed in the world),
//! **cannons** that fire balls, shapes, grains or images, **lamps** that
//! light the night (and cast shadows), and **grappling hooks** that hook
//! onto what they hit and reel in. Each one works all the time, while an
//! arrow key is held, on the music's beats, or on a signal.
//!
//! Signals come from the mechanisms: **buttons** pressed by what rests on
//! them, **sensors** whose invisible beam something crosses, **targets** a
//! laser reaches, and **timers**; each sends on one of four coloured
//! channels, while it is on or toggled at each press. **Pistons** slide the
//! object they are on (a door, a lift), and motors can turn on a signal too.

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
    Lamp,
    Grapple,
    /// Pressed by what rests on it: sends a signal.
    Button,
    /// An invisible beam: sends a signal while something crosses it.
    Sensor,
    /// Sends a signal while a laser reaches it.
    Target,
    /// Sends a signal half of each period.
    Timer,
    /// Slides the object it is on, out and back.
    Piston,
}

/// How a mechanism's signal follows what it senses.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum SendMode {
    /// On while pressed (lit, crossed).
    #[default]
    Hold,
    /// On from the first press, and it stays on.
    Latch,
    /// Each new press switches it on, or off again.
    Toggle,
}

impl SendMode {
    pub const ALL: [SendMode; 3] = [SendMode::Hold, SendMode::Latch, SendMode::Toggle];

    pub fn label(self) -> &'static str {
        match self {
            SendMode::Hold => "While on",
            SendMode::Latch => "Stays on",
            SendMode::Toggle => "Flips",
        }
    }

    /// How it sends, in a sentence's words.
    pub fn about(self) -> &'static str {
        match self {
            SendMode::Hold => "while on",
            SendMode::Latch => "from the first press on",
            SendMode::Toggle => "on and off at each press",
        }
    }
}

/// How many signal channels there are.
pub const CHANNELS: usize = 4;

/// A channel's colour.
pub fn channel_colour(c: u8) -> Color {
    match c {
        0 => Color::from_rgba(255, 92, 92, 255),
        1 => Color::from_rgba(90, 225, 120, 255),
        2 => Color::from_rgba(90, 160, 255, 255),
        _ => Color::from_rgba(255, 215, 70, 255),
    }
}

/// A channel's name.
pub fn channel_name(c: u8) -> &'static str {
    ["red", "green", "blue", "yellow"][(c as usize).min(CHANNELS - 1)]
}

impl GadgetKind {
    /// The things that do something, then the mechanisms (as the Gadget
    /// tool shows them, in two rows).
    pub const ALL: &'static [GadgetKind] = &[
        GadgetKind::Laser,
        GadgetKind::Thruster,
        GadgetKind::Cannon,
        GadgetKind::Lamp,
        GadgetKind::Grapple,
        GadgetKind::Button,
        GadgetKind::Sensor,
        GadgetKind::Target,
        GadgetKind::Timer,
        GadgetKind::Piston,
    ];
    /// How many of [`GadgetKind::ALL`] are in the first row.
    pub const FIRST_ROW: usize = 5;

    pub fn label(self) -> &'static str {
        match self {
            GadgetKind::Laser => "Laser",
            GadgetKind::Thruster => "Thruster",
            GadgetKind::Cannon => "Cannon",
            GadgetKind::Lamp => "Lamp",
            GadgetKind::Grapple => "Hook",
            GadgetKind::Button => "Button",
            GadgetKind::Sensor => "Sensor",
            GadgetKind::Target => "Target",
            GadgetKind::Timer => "Timer",
            GadgetKind::Piston => "Piston",
        }
    }

    /// It sends a signal (instead of working on one).
    pub fn sends(self) -> bool {
        matches!(self, GadgetKind::Button | GadgetKind::Sensor | GadgetKind::Target | GadgetKind::Timer)
    }

    pub fn hint(self) -> [&'static str; 2] {
        match self {
            GadgetKind::Laser => ["Drag to aim  ·  mirrors reflect it,", "glass lets it through, it burns the rest"],
            GadgetKind::Thruster => ["Drag on an object the way to push it", "(on empty space: a fan)"],
            GadgetKind::Cannon => ["Drag to aim  ·  on an object it", "recoils; right-click a gadget removes it"],
            GadgetKind::Lamp => ["Lights the night (Shift+T) and casts", "shadows  ·  drag to aim a spotlight"],
            GadgetKind::Grapple => ["Put it on an object: an arrow key shoots", "the hook at the pointer and reels in"],
            GadgetKind::Button => ["On a floor or a wall, facing the way you", "drag: what rests on it presses it"],
            GadgetKind::Sensor => ["An invisible beam the way you drag:", "on while something crosses it"],
            GadgetKind::Target => ["On while a laser's beam reaches it", "(mirrors can bring one round)"],
            GadgetKind::Timer => ["On, then off, again and again:", "half of each period each"],
            GadgetKind::Piston => ["Put it on an object (a plank: a door, a", "lift): it slides it the way you drag"],
        }
    }

    pub fn accent(self) -> Color {
        match self {
            GadgetKind::Laser => Color::from_rgba(255, 80, 90, 255),
            GadgetKind::Thruster => Color::from_rgba(255, 160, 60, 255),
            GadgetKind::Cannon => Color::from_rgba(170, 180, 200, 255),
            GadgetKind::Lamp => Color::from_rgba(255, 220, 130, 255),
            GadgetKind::Grapple => Color::from_rgba(120, 220, 170, 255),
            GadgetKind::Button => Color::from_rgba(110, 230, 140, 255),
            GadgetKind::Sensor => Color::from_rgba(255, 120, 200, 255),
            GadgetKind::Target => Color::from_rgba(255, 150, 90, 255),
            GadgetKind::Timer => Color::from_rgba(190, 200, 255, 255),
            GadgetKind::Piston => Color::from_rgba(230, 195, 120, 255),
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
    /// While this channel's signal is on.
    Signal(u8),
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
            Trigger::Signal(0) => "on the red signal",
            Trigger::Signal(1) => "on the green signal",
            Trigger::Signal(2) => "on the blue signal",
            Trigger::Signal(_) => "on the yellow signal",
        }
    }

    /// The channel it listens to, if any.
    pub fn channel(self) -> Option<u8> {
        match self {
            Trigger::Signal(c) => Some(c),
            _ => None,
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
    /// Lamp: half-angle of its light (rad), π all round.
    pub spread: f32,
    /// Lamp: light colour.
    pub colour: [u8; 3],
    /// Button, sensor, target, timer: the channel it sends on.
    pub channel: u8,
    /// Button, sensor, target: whether the signal is on while pressed,
    /// stays on, or is switched at each press.
    pub mode: SendMode,
    /// Piston: how far it has slid its object out (m).
    pub ext: f32,
}

/// Lamp colours offered by the Gadget tool.
pub const LAMP_COLOURS: [[u8; 3]; 6] =
    [[255, 214, 150], [235, 242, 255], [255, 96, 80], [110, 255, 150], [100, 160, 255], [214, 120, 255]];
/// Half-angle of a spotlight (rad).
pub const SPOT_SPREAD: f32 = 0.5;

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
            spread: std::f32::consts::PI,
            colour: LAMP_COLOURS[0],
            channel: 0,
            mode: SendMode::Hold,
            ext: 0.0,
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
        self.spread = ok(self.spread, 0.1, std::f32::consts::PI, std::f32::consts::PI);
        self.channel = self.channel.min(CHANNELS as u8 - 1);
        if let Trigger::Signal(c) = self.trigger {
            self.trigger = Trigger::Signal(c.min(CHANNELS as u8 - 1));
        }
        self.ext = ok(self.ext, 0.0, self.power, 0.0);
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
    /// Grapple: the rope to what it hooked, or where a shot that missed
    /// got to (world px) and how long ago (s).
    pub hook: Option<super::links::Link>,
    pub miss: Option<(Vec2, f32)>,
    /// Mechanisms: what it sensed last frame (to tell a new press), and
    /// a toggled signal.
    pub pressed: bool,
    pub latched: bool,
    /// Target: a beam reached it this frame. Sensor: how far its beam goes
    /// (m). Timer: seconds into its period.
    pub lit: bool,
    pub seen: f32,
    pub clock: f32,
}

impl Gadget {
    pub fn new(spec: GadgetSpec, host: Option<RigidBodyHandle>) -> Self {
        Gadget {
            spec,
            host,
            on: false,
            cooldown: 0.0,
            fired: VecDeque::new(),
            pulse: 0.0,
            hook: None,
            miss: None,
            pressed: false,
            latched: false,
            lit: false,
            seen: 0.0,
            clock: 0.0,
        }
    }

    /// Its signal this frame, from what it senses now (`raw`): on while
    /// pressed, on from the first press, or switched at each new press (a
    /// timer is always on while on).
    pub fn signal(&mut self, raw: bool) -> bool {
        let fresh = raw && !self.pressed;
        self.pressed = raw;
        let mode = if self.spec.kind == GadgetKind::Timer { SendMode::Hold } else { self.spec.mode };
        self.on = match mode {
            SendMode::Hold => raw,
            SendMode::Latch => {
                self.latched |= raw;
                self.latched
            }
            SendMode::Toggle => {
                if fresh {
                    self.latched = !self.latched;
                }
                self.latched
            }
        };
        self.on
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

    /// A button's pad, where what presses it is: its centre (m) and the
    /// way it faces.
    pub fn pad(&self, world: &PhysWorld) -> Option<(Point<f32>, Vector<f32>)> {
        let (p, d) = self.pose(world)?;
        Some((p + d * (PAD_OUT + PAD_HALF.0), d))
    }

    /// A button: whether something that is not fixed in place (pinned and
    /// driven things count) is on its pad.
    pub fn pad_touched(&self, world: &PhysWorld) -> bool {
        let Some((centre, d)) = self.pad(world) else { return false };
        let pad = Cuboid::new(vector![PAD_HALF.0, PAD_HALF.1]);
        let at = Isometry::new(centre.coords, d.y.atan2(d.x));
        let mut filter = QueryFilter::default().exclude_sensors();
        filter.exclude_rigid_body = self.host;
        world.overlapping(&at, &pad, filter).into_iter().any(|c| moving(world, c))
    }

    /// A sensor: how far its beam goes (m), and whether what stops it is
    /// not fixed in place.
    pub fn sense(&self, world: &PhysWorld) -> (f32, bool) {
        let Some((p, d)) = self.muzzle(world) else { return (0.0, false) };
        let reach = self.spec.power.max(0.1);
        let mut filter = QueryFilter::default().exclude_sensors();
        filter.exclude_rigid_body = self.host;
        match world.cast_ray(p, d, reach, filter) {
            Some((c, toi, _)) => (toi, moving(world, c)),
            None => (reach, false),
        }
    }
}

/// What presses a button: a pad this far out from it, this deep and this
/// wide (half sizes, m).
const PAD_OUT: f32 = 0.16;
pub const PAD_HALF: (f32, f32) = (0.12, 0.3);

/// Whether collider `c` is on something that is not fixed in place.
fn moving(world: &PhysWorld, c: ColliderHandle) -> bool {
    let body = world.colliders.get(c).and_then(|c| c.parent()).and_then(|b| world.bodies.get(b));
    body.is_some_and(|b| b.body_type() != RigidBodyType::Fixed)
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
pub fn segment_hits_rect(a: Point<f32>, b: Point<f32>, min: [f32; 2], max: [f32; 2]) -> bool {
    segment_enters_rect(a, b, min, max).is_some()
}

/// How far along segment `a`–`b` (0 to 1) it gets into the box, if it does.
fn segment_enters_rect(a: Point<f32>, b: Point<f32>, min: [f32; 2], max: [f32; 2]) -> Option<f32> {
    let d = b - a;
    let (mut t0, mut t1) = (0.0f32, 1.0f32);
    for (p, q) in [(-d.x, a.x - min[0]), (d.x, max[0] - a.x), (-d.y, a.y - min[1]), (d.y, max[1] - a.y)] {
        if p == 0.0 {
            if q < 0.0 {
                return None;
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
    (t0 <= t1).then_some(t0)
}

/// Half the size of a target, as beams see it (m).
pub const TARGET_HALF: f32 = 0.22;

/// Stop `beam` at the first of `targets` (their centres, m) it reaches:
/// returns which one (its place in `targets`).
pub fn clip_at_target(beam: &mut Beam, targets: &[Point<f32>]) -> Option<usize> {
    for k in 0..beam.points.len().saturating_sub(1) {
        let (a, b) = (beam.points[k], beam.points[k + 1]);
        let first = targets
            .iter()
            .enumerate()
            .filter_map(|(i, c)| {
                let (min, max) = ([c.x - TARGET_HALF, c.y - TARGET_HALF], [c.x + TARGET_HALF, c.y + TARGET_HALF]);
                segment_enters_rect(a, b, min, max).map(|t| (i, t))
            })
            .min_by(|x, y| x.1.total_cmp(&y.1));
        if let Some((i, t)) = first {
            beam.points.truncate(k + 1);
            beam.points.push(a + (b - a) * t);
            beam.colours.truncate(k + 1);
            beam.hit = None;
            return Some(i);
        }
    }
    None
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
        GadgetKind::Lamp => {}
        GadgetKind::Grapple => {
            // A launcher with a three-pronged hook at its mouth.
            quad(at + dir * 4.0, 16.0, 5.0, rim);
            quad(at + dir * 4.0, 13.0, 3.5, dark);
            draw_circle(at.x, at.y, 6.0, rim);
            draw_circle(at.x, at.y, 4.0, dark);
            if !on {
                draw_hook(at + dir * 14.0, dir, Color::from_rgba(200, 210, 220, 255));
            }
        }
        GadgetKind::Button => {
            // A plate on the surface, its cap sticking out (pushed in when pressed).
            let lit = if on { GadgetKind::Button.accent() } else { Color::from_rgba(70, 120, 84, 255) };
            quad(at + dir * 1.5, 4.0, 15.0, rim);
            let out = if on { 2.0 } else { 6.0 };
            quad(at + dir * (3.0 + out / 2.0), out, 9.0, lit);
            if on {
                let glow = at + dir * 6.0;
                draw_circle(glow.x, glow.y, 13.0, Color { a: 0.15, ..lit });
            }
        }
        GadgetKind::Sensor => {
            quad(at, 14.0, 6.0, rim);
            quad(at, 11.0, 4.5, dark);
            let lens = at + dir * 6.0;
            let c = GadgetKind::Sensor.accent();
            draw_circle(lens.x, lens.y, 3.2, if on { c } else { Color { a: 0.5, ..c } });
        }
        GadgetKind::Target => {
            let (red, white) = (Color::from_rgba(220, 60, 60, 255), Color::from_rgba(240, 236, 228, 255));
            if on {
                draw_circle(at.x, at.y, 16.0 + 2.0 * (t * 12.0).sin(), Color::new(1.0, 0.6, 0.3, 0.3));
            }
            for (r, c) in [(11.0, rim), (10.0, white), (7.5, red), (5.0, white), (2.5, red)] {
                draw_circle(at.x, at.y, r, c);
            }
            if on {
                draw_circle(at.x, at.y, 2.5, Color::new(1.0, 0.95, 0.7, 1.0));
            }
        }
        GadgetKind::Timer => {
            let face = if on { GadgetKind::Timer.accent() } else { Color::from_rgba(70, 74, 96, 255) };
            draw_circle(at.x, at.y, 10.5, rim);
            draw_circle(at.x, at.y, 8.5, face);
            for k in 0..4 {
                let a = k as f32 * std::f32::consts::FRAC_PI_2;
                let (p, q) = (at + vec2(a.cos(), a.sin()) * 6.5, at + vec2(a.cos(), a.sin()) * 8.0);
                draw_line(p.x, p.y, q.x, q.y, 1.2, dark);
            }
            let a = t * 3.0;
            let hand = at + vec2(a.sin(), -a.cos()) * 6.0;
            draw_line(at.x, at.y, hand.x, hand.y, 1.6, dark);
            draw_circle(at.x, at.y, 1.5, dark);
        }
        GadgetKind::Piston => {
            // A cylinder, its rod and an arrow the way it pushes.
            quad(at - dir * 3.0, 16.0, 6.5, rim);
            quad(at - dir * 3.0, 13.0, 5.0, dark);
            quad(at + dir * 8.0, 8.0, 2.0, rim);
            let c = if on { GadgetKind::Piston.accent() } else { Color::from_rgba(120, 110, 90, 255) };
            let tip = at + dir * 3.0;
            draw_triangle(tip, tip - dir * 5.0 + n * 3.5, tip - dir * 5.0 - n * 3.5, c);
        }
    }
}

/// A mechanism's channel: a dot of its colour at `at` (screen), ringed
/// while the signal is on.
pub fn draw_channel_dot(at: Vec2, c: u8, on: bool) {
    let colour = channel_colour(c);
    draw_circle(at.x, at.y, 4.0, Color::from_rgba(20, 22, 30, 230));
    draw_circle(at.x, at.y, 3.0, colour);
    if on {
        draw_circle_lines(at.x, at.y, 6.0, 1.2, Color { a: 0.8, ..colour });
    }
}

/// The hook itself, its point at `at`, facing `dir`.
pub fn draw_hook(at: Vec2, dir: Vec2, colour: Color) {
    let n = vec2(-dir.y, dir.x);
    let back = at - dir * 6.0;
    draw_line(back.x, back.y, at.x, at.y, 2.0, colour);
    for side in [-1.0, 1.0] {
        let tip = at - dir * 3.0 + n * side * 5.0;
        draw_line(at.x, at.y, tip.x, tip.y, 1.8, colour);
    }
}

/// Draw a lamp at `at` (world px): a bulb, with a hood when it is a
/// spotlight facing `dir`.
pub fn draw_lamp(at: Vec2, dir: Vec2, spot: bool, colour: Color, on: bool) {
    let n = vec2(-dir.y, dir.x);
    let rim = Color::from_rgba(150, 156, 180, 255);
    let dark = Color::from_rgba(44, 46, 60, 255);
    if spot {
        let (back, front) = (at - dir * 6.0, at + dir * 6.0);
        draw_triangle(back + n * 4.0, back - n * 4.0, front + n * 8.0, rim);
        draw_triangle(back - n * 4.0, front - n * 8.0, front + n * 8.0, rim);
        draw_line(front.x + n.x * 7.0, front.y + n.y * 7.0, front.x - n.x * 7.0, front.y - n.y * 7.0, 3.0, dark);
    } else {
        draw_circle(at.x, at.y, 7.5, rim);
    }
    let glass = if on { colour } else { Color::new(colour.r * 0.35, colour.g * 0.35, colour.b * 0.35, 1.0) };
    let bulb = if spot { at + dir * 5.0 } else { at };
    draw_circle(bulb.x, bulb.y, if spot { 4.0 } else { 5.5 }, glass);
    if on {
        draw_circle(bulb.x, bulb.y, 2.0, Color::new(1.0, 1.0, 1.0, 0.9));
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

    #[test]
    fn mechanisms_send_while_on_or_toggle_at_each_press() {
        let mut held = Gadget::new(GadgetSpec { kind: GadgetKind::Button, ..GadgetSpec::default() }, None);
        assert!(held.signal(true) && held.signal(true) && !held.signal(false));
        let spec = GadgetSpec { kind: GadgetKind::Button, mode: SendMode::Toggle, ..GadgetSpec::default() };
        let mut toggle = Gadget::new(spec, None);
        // Pressed: on, and it stays on let go; pressed again: off.
        assert!(toggle.signal(true) && toggle.signal(true) && toggle.signal(false));
        assert!(!toggle.signal(true) && !toggle.signal(false));
        // Stays on: once pressed, for good.
        let spec = GadgetSpec { kind: GadgetKind::Sensor, mode: SendMode::Latch, ..GadgetSpec::default() };
        let mut latch = Gadget::new(spec, None);
        assert!(!latch.signal(false) && latch.signal(true) && latch.signal(false) && latch.signal(true));
        assert!(latch.signal(false));
        // A timer does not toggle.
        let spec = GadgetSpec { kind: GadgetKind::Timer, mode: SendMode::Toggle, ..GadgetSpec::default() };
        let mut timer = Gadget::new(spec, None);
        assert!(timer.signal(true) && !timer.signal(false));
        assert!(GadgetKind::Button.sends() && !GadgetKind::Piston.sends() && !GadgetKind::Laser.sends());
        assert_eq!(Trigger::Signal(2).channel(), Some(2));
        let wild =
            GadgetSpec { channel: 9, trigger: Trigger::Signal(7), ext: 50.0, power: 2.0, ..GadgetSpec::default() };
        let wild = wild.sanitized();
        assert_eq!((wild.channel, wild.trigger, wild.ext), (3, Trigger::Signal(3), 2.0));
    }

    #[test]
    fn buttons_feel_what_rests_on_them_and_sensors_what_crosses() {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (1200.0, 1200.0));
        // A floor, a button on it facing up, and a box falling on it.
        let floor = w.bodies.insert(RigidBodyBuilder::fixed().translation(vector![10.0, 1.0]));
        w.colliders.insert_with_parent(ColliderBuilder::cuboid(8.0, 0.5), floor, &mut w.bodies);
        let up = std::f32::consts::FRAC_PI_2;
        let button = Gadget::new(
            GadgetSpec { kind: GadgetKind::Button, at: [10.0, 1.5], angle: up, ..GadgetSpec::default() },
            None,
        );
        w.refresh_queries();
        assert!(!button.pad_touched(&w), "nothing on it (the floor is fixed)");
        let crate_ = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![10.0, 4.0]));
        w.colliders.insert_with_parent(ColliderBuilder::cuboid(0.3, 0.3), crate_, &mut w.bodies);
        for _ in 0..120 {
            w.step_fixed();
        }
        assert!(button.pad_touched(&w), "the box landed on it");
        // A sensor's beam along the floor: a wall stops it, a ball in it is seen.
        let spec = GadgetSpec { kind: GadgetKind::Sensor, at: [3.0, 1.8], power: 5.0, ..GadgetSpec::default() };
        let sensor = Gadget::new(spec, None);
        let (seen, hit) = sensor.sense(&w);
        assert!(!hit && (seen - 5.0).abs() < 1e-3, "nothing in reach: {seen}");
        let ball = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![5.0, 1.8]).gravity_scale(0.0));
        w.colliders.insert_with_parent(ColliderBuilder::ball(0.2), ball, &mut w.bodies);
        w.refresh_queries();
        let (seen, hit) = sensor.sense(&w);
        assert!(hit && (seen - 1.6).abs() < 0.05, "sees the ball: {seen}");
    }

    #[test]
    fn targets_stop_the_beams_that_reach_them() {
        let mut beam = Beam {
            points: vec![point![0.0, 0.0], point![4.0, 0.0], point![4.0, 4.0]],
            colours: vec![LASER_COLOUR, LASER_COLOUR],
            hit: None,
        };
        // Not the one off the way; the one after the corner.
        let targets = [point![2.0, 3.0], point![4.0, 2.0]];
        assert_eq!(clip_at_target(&mut beam, &targets), Some(1));
        assert_eq!(beam.points.len(), 3);
        let end = beam.points[2];
        assert!((end.x - 4.0).abs() < 1e-4 && (end.y - (2.0 - TARGET_HALF)).abs() < 1e-4, "{end:?}");
        assert_eq!(beam.colours.len(), 2);
        let mut miss =
            Beam { points: vec![point![0.0, 0.0], point![1.0, 0.0]], colours: vec![LASER_COLOUR], hit: None };
        assert_eq!(clip_at_target(&mut miss, &targets), None);
    }
}
