//! A 3D world being played: the ground a generator made (an endless course
//! or an open zone), the character on it, rings, springs, dash panels,
//! badniks and things that fall and roll (rapier3d); a camera that chases
//! him; and the picture.

use super::figure::{Figure, Pose, Props};
use super::gfx::{checker, draw_at, draw_sky, Gfx, Shape, Sky};
use super::models::Pack;
use super::phys::{Phys, Solid};
use super::player::{Controls, Event as Move, Runner, K, TOP};
use crate::sonic::Character;
use macroquad::prelude::*;
use rapier3d::prelude::{ColliderHandle, RigidBodyHandle};

/// How a stretch of ground looks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Skin {
    Grass,
    Soil,
    Plain,
}

#[derive(Clone, Copy, Debug)]
pub struct Spring {
    pub at: Vec3,
    pub up: Vec3,
    /// Launch speed (guide px a frame).
    pub power: f32,
    pub red: bool,
    pub squash: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Panel {
    pub at: Vec3,
    pub dir: Vec3,
    pub up: Vec3,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadnikKind {
    /// Crawls back and forth on the ground.
    Crawler,
    /// Hovers, and comes at him when he is near.
    Flyer,
}

#[derive(Clone, Copy, Debug)]
pub struct Badnik {
    pub kind: BadnikKind,
    pub at: Vec3,
    pub home: Vec3,
    pub dir: Vec3,
    pub t: f32,
}

/// A loop: he is guided sideways as he goes round so he comes out beside
/// where he went in.
#[derive(Clone, Copy, Debug)]
pub struct Loop {
    /// The bottom, where he goes in.
    pub start: Vec3,
    pub forward: Vec3,
    pub side: Vec3,
    pub radius: f32,
    pub width: f32,
    /// How far sideways the way out is.
    pub shift: f32,
}

impl Loop {
    fn centre(&self) -> Vec3 {
        self.start + Vec3::Y * self.radius
    }

    /// How far round (0 at the bottom, going up the far side) a point is.
    fn angle(&self, p: Vec3) -> f32 {
        let d = p - self.centre();
        d.dot(self.forward).atan2(-d.y)
    }

    fn lateral(&self, p: Vec3) -> f32 {
        (p - self.start).dot(self.side)
    }

    /// Whether someone at `p` going `vel` is starting round it.
    pub fn entering(&self, p: Vec3, vel: Vec3) -> Option<f32> {
        let a = self.angle(p);
        let ok = self.lateral(p).abs() < self.width / 2.0 + 0.5
            && (p - self.start).dot(self.forward).abs() < self.radius
            && (0.02..1.2).contains(&a)
            && vel.dot(self.forward) > 0.0
            && (p - self.centre()).length() < self.radius + 1.5;
        ok.then_some(a)
    }

    /// Guided on round: how far round he is now (unwound, from how far he
    /// was) and where he goes sideways; none when he is out of it.
    pub fn guide(&self, progress: f32, p: Vec3) -> Option<(f32, Vec3)> {
        use std::f32::consts::{PI, TAU};
        let a = self.angle(p);
        let mut d = a - progress.rem_euclid(TAU);
        if d > PI {
            d -= TAU;
        }
        if d < -PI {
            d += TAU;
        }
        let now = progress + d;
        if !(-0.2..TAU - 0.05).contains(&now) {
            return None;
        }
        let want = self.shift * (now / TAU).clamp(0.0, 1.0);
        Some((now, p + self.side * (want - self.lateral(p))))
    }
}

/// What a generator makes for a stretch of world.
#[derive(Default)]
pub struct Stretch {
    /// Solid ground and what it looks like.
    pub ground: Vec<(Shape, Skin)>,
    /// Only drawn.
    pub decor: Vec<(Shape, Skin)>,
    pub rings: Vec<Vec3>,
    pub springs: Vec<Spring>,
    pub panels: Vec<Panel>,
    pub badniks: Vec<Badnik>,
    pub loops: Vec<Loop>,
    pub things: Vec<(Solid, Vec3)>,
    /// Points along the way (the run) with how far along they are.
    pub path: Vec<(Vec3, f32)>,
}

/// One stretch in the world: its meshes and colliders, and an id the
/// rings and the rest made with it carry (to clear them with it).
struct Piece {
    id: u32,
    meshes: Vec<(Vec<Mesh>, Skin)>,
    /// Its textures put on (at the first drawing).
    dressed: bool,
    colliders: Vec<ColliderHandle>,
    path: Vec<(Vec3, f32)>,
}

struct Thing {
    body: RigidBodyHandle,
    solid: Solid,
}

/// Rings knocked out of him: they bounce about, then fade.
struct Spilled {
    at: Vec3,
    vel: Vec3,
    life: u32,
}

/// What happened, for the sounds.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Noise {
    Jump,
    Rev,
    Dash,
    Skid,
    Spring,
    Ring,
    Hurt,
    Scatter,
    Pop,
    Boom,
    Thud,
}

/// The camera that chases him.
struct Chase {
    yaw: f32,
    pitch: f32,
    dist: f32,
    at: Vec3,
    eye: Vec3,
    /// Frames since the player turned it.
    manual: u32,
}

impl Chase {
    fn forward(&self) -> Vec3 {
        vec3(self.yaw.cos(), 0.0, self.yaw.sin())
    }
}

/// The player's hands on the controls this frame.
#[derive(Clone, Copy, Debug, Default)]
pub struct Input {
    /// Stick: x to the right, y forward (−1‥1).
    pub stick: Vec2,
    pub jump: bool,
    pub jump_pressed: bool,
    pub action: bool,
    pub action_pressed: bool,
    /// Turn the camera (−1‥1, to the right).
    pub turn: f32,
    pub spawn: bool,
    pub blast: bool,
    pub again: bool,
}

pub trait Maker {
    /// The next stretch (the run), or everything at once (the zone).
    fn next(&mut self) -> Option<Stretch>;
    /// Where he starts, facing which way.
    fn start(&self) -> (Vec3, Vec3);
    /// Endless: more is made as he goes.
    fn endless(&self) -> bool;
}

pub struct World3 {
    pub phys: Phys,
    pub runner: Runner,
    maker: Box<dyn Maker>,
    pieces: Vec<Piece>,
    next_id: u32,
    rings: Vec<(Vec3, u32)>,
    springs: Vec<(Spring, u32)>,
    panels: Vec<(Panel, u32)>,
    badniks: Vec<(Badnik, u32)>,
    loops: Vec<(Loop, u32)>,
    things: Vec<Thing>,
    spilled: Vec<Spilled>,
    pops: Vec<(Vec3, u32)>,
    blasts: Vec<(Vec3, u32)>,
    guide: Option<(usize, f32)>,
    cam: Chase,
    pub sky: Sky,
    pub ring_count: u32,
    pub score: u32,
    /// The run: how far along (m), the best before this run.
    pub distance: f32,
    pub best: f32,
    pub over: bool,
    invincible: u32,
    frames: u64,
    pose: Pose,
    draw_up: Vec3,
    pub start: (Vec3, Vec3),
    pub noises: Vec<Noise>,
    acc: f32,
    /// The sea far below the run (its height follows the road's).
    sea: f32,
}

/// Looks: the figures and props, and the ground's textures.
pub struct Looks {
    pub gfx: Gfx,
    pub figure: Figure,
    pub props: Props,
    /// The player's own models, where there are some.
    pub pack: Pack,
    grass: Texture2D,
    soil: Texture2D,
}

impl Looks {
    pub fn new(character: Character) -> Option<Looks> {
        Some(Looks {
            gfx: Gfx::new()?,
            figure: Figure::new(character),
            props: Props::new(),
            pack: super::models::load(character),
            grass: checker(Color::new(0.36, 0.72, 0.25, 1.0), Color::new(0.3, 0.62, 0.2, 1.0), 2, 64),
            soil: checker(Color::new(0.78, 0.46, 0.2, 1.0), Color::new(0.55, 0.3, 0.12, 1.0), 2, 64),
        })
    }

    fn texture(&self, s: Skin) -> Texture2D {
        match s {
            Skin::Grass => self.grass.clone(),
            Skin::Soil => self.soil.clone(),
            Skin::Plain => self.gfx.white.clone(),
        }
    }
}

const RING_REACH: f32 = 1.1;
const BADNIK_REACH: f32 = 1.2;
/// How far ahead the run is made (m along).
const AHEAD: f32 = 260.0;
/// How far behind it is kept.
const BEHIND: f32 = 60.0;

impl World3 {
    pub fn new(maker: Box<dyn Maker>, character: Character, best: f32) -> World3 {
        let start = maker.start();
        let runner = Runner::new(character, start.0, start.1);
        let yaw = start.1.z.atan2(start.1.x);
        let mut w = World3 {
            phys: Phys::new(),
            runner,
            maker,
            pieces: vec![],
            next_id: 0,
            rings: vec![],
            springs: vec![],
            panels: vec![],
            badniks: vec![],
            loops: vec![],
            things: vec![],
            spilled: vec![],
            pops: vec![],
            blasts: vec![],
            guide: None,
            cam: Chase {
                yaw,
                pitch: 0.32,
                dist: 7.0,
                at: start.0,
                eye: start.0 - start.1 * 7.0 + Vec3::Y * 2.5,
                manual: 0,
            },
            sky: Sky::DAY,
            ring_count: 0,
            score: 0,
            distance: 0.0,
            best,
            over: false,
            invincible: 0,
            frames: 0,
            pose: Pose::default(),
            draw_up: Vec3::Y,
            start,
            noises: vec![],
            acc: 0.0,
            sea: start.0.y - 35.0,
        };
        w.fill();
        w.phys.refresh();
        w
    }

    /// Make stretches until there is enough ahead.
    fn fill(&mut self) {
        loop {
            let ahead = self.pieces.last().and_then(|p| p.path.last()).map_or(0.0, |p| p.1);
            if self.maker.endless() && ahead > self.distance + AHEAD {
                break;
            }
            let Some(s) = self.maker.next() else { break };
            self.add(s);
        }
    }

    fn add(&mut self, s: Stretch) {
        let id = self.next_id;
        self.next_id += 1;
        let mut colliders = vec![];
        let mut meshes = vec![];
        for (shape, skin) in &s.ground {
            if let Some(c) = self.phys.add_mesh(&shape.pos, &shape.tris) {
                colliders.push(c);
            }
            meshes.push((shape.meshes(None), *skin));
        }
        for (shape, skin) in &s.decor {
            meshes.push((shape.meshes(None), *skin));
        }
        self.rings.extend(s.rings.iter().map(|r| (*r, id)));
        self.springs.extend(s.springs.iter().map(|r| (*r, id)));
        self.panels.extend(s.panels.iter().map(|r| (*r, id)));
        self.badniks.extend(s.badniks.iter().map(|r| (*r, id)));
        self.loops.extend(s.loops.iter().map(|r| (*r, id)));
        for (solid, at) in s.things {
            let body = self.phys.add_body(solid, at, Vec3::ZERO, 0.6);
            self.things.push(Thing { body, solid });
        }
        self.pieces.push(Piece { id, meshes, dressed: false, colliders, path: s.path });
    }

    /// Clear stretches far behind.
    fn clear_behind(&mut self) {
        while self.pieces.len() > 2 && self.pieces[0].path.last().is_some_and(|p| p.1 < self.distance - BEHIND) {
            let p = self.pieces.remove(0);
            for c in p.colliders {
                self.phys.remove_collider(c);
            }
            let id = p.id;
            self.rings.retain(|r| r.1 != id);
            self.springs.retain(|r| r.1 != id);
            self.panels.retain(|r| r.1 != id);
            self.badniks.retain(|r| r.1 != id);
            if self.guide.is_some_and(|(i, _)| self.loops[i].1 == id) {
                self.guide = None;
            }
            self.loops.retain(|r| r.1 != id);
        }
        // Things that fell out of the world.
        let low = self.runner.pos.y - 120.0;
        let gone: Vec<RigidBodyHandle> = self
            .things
            .iter()
            .filter(|t| self.phys.pose(t.body).is_none_or(|(p, _)| p.y < low))
            .map(|t| t.body)
            .collect();
        for b in gone {
            self.phys.remove_body(b);
            self.things.retain(|t| t.body != b);
        }
    }

    /// The nearest point of the way and how far along it is.
    fn along(&self, p: Vec3) -> Option<(Vec3, f32)> {
        self.pieces
            .iter()
            .flat_map(|piece| piece.path.iter())
            .min_by(|a, b| (a.0 - p).length_squared().total_cmp(&(b.0 - p).length_squared()))
            .copied()
    }

    /// The way the course goes near `p` (for the camera).
    fn way(&self, p: Vec3) -> Option<Vec3> {
        let (_, d) = self.along(p)?;
        let pts: Vec<&(Vec3, f32)> = self.pieces.iter().flat_map(|piece| piece.path.iter()).collect();
        let a = pts.iter().min_by(|a, b| (a.1 - d).abs().total_cmp(&(b.1 - d).abs()))?;
        let b = pts.iter().min_by(|x, y| (x.1 - d - 12.0).abs().total_cmp(&(y.1 - d - 12.0).abs()))?;
        let w = b.0 - a.0;
        (w.length() > 1.0).then(|| vec3(w.x, 0.0, w.z).normalize_or_zero())
    }

    /// Time passes: steps at 60 Hz.
    pub fn update(&mut self, dt: f32, input: Input) {
        self.acc = (self.acc + dt).min(4.0 / 60.0);
        let mut first = true;
        while self.acc >= 1.0 / 60.0 - 1e-4 {
            self.acc -= 1.0 / 60.0;
            let mut i = input;
            if !first {
                i.jump_pressed = false;
                i.action_pressed = false;
                i.spawn = false;
                i.blast = false;
                i.again = false;
            }
            first = false;
            self.step(i);
        }
    }

    pub fn step(&mut self, input: Input) {
        self.frames += 1;
        if self.over {
            self.camera(input.turn);
            return;
        }
        // The stick, turned by the camera.
        let f = self.cam.forward();
        let right = vec3(-f.z, 0.0, f.x);
        let mut dir = f * input.stick.y + right * input.stick.x;
        if dir.length() > 1.0 {
            dir = dir.normalize();
        }
        let c = Controls {
            dir,
            jump: input.jump,
            jump_pressed: input.jump_pressed,
            action: input.action,
            action_pressed: input.action_pressed,
        };
        let moves = self.runner.frame(&self.phys, c);
        for m in moves {
            match m {
                Move::Jump => self.noises.push(Noise::Jump),
                Move::Rev => self.noises.push(Noise::Rev),
                Move::Dash => self.noises.push(Noise::Dash),
                Move::Skid => self.noises.push(Noise::Skid),
                Move::Hammer | Move::Flap | Move::Glide | Move::Roll => self.noises.push(Noise::Rev),
                Move::Teleport => self.noises.push(Noise::Dash),
                Move::Land => {}
                Move::Push { body, impulse, at } => self.phys.push(body, impulse, at),
            }
        }
        self.guide_loops();
        self.phys.step();
        self.touch();
        self.badniks_move();
        self.spill_move();
        if input.spawn {
            self.spawn_thing();
        }
        if input.blast {
            let at = self.runner.pos;
            self.phys.blast(at, 8.0, 14.0);
            self.blasts.push((at, 0));
            self.noises.push(Noise::Boom);
        }
        self.invincible = self.invincible.saturating_sub(1);
        if let Some((_, d)) = self.along(self.runner.pos) {
            if self.maker.endless() {
                self.distance = self.distance.max(d);
            }
        }
        if self.maker.endless() {
            self.fill();
            self.clear_behind();
        }
        self.phys.refresh();
        // Fell: below everything near.
        let floor = self.along(self.runner.pos).map_or(-60.0, |p| p.0.y - 40.0);
        if self.runner.pos.y < floor.min(-60.0).max(floor) {
            self.die();
        }
        if let Some((p, _)) = self.along(self.runner.pos) {
            self.sea += (p.y - 35.0 - self.sea) * 0.01;
        }
        self.animate();
        self.camera(input.turn);
        self.pops.retain_mut(|p| {
            p.1 += 1;
            p.1 < 24
        });
        self.blasts.retain_mut(|p| {
            p.1 += 1;
            p.1 < 30
        });
    }

    fn die(&mut self) {
        self.noises.push(Noise::Hurt);
        if self.maker.endless() {
            self.over = true;
        } else {
            let (at, facing) = self.start;
            let c = self.runner.character;
            self.runner = Runner::new(c, at, facing);
            self.ring_count = 0;
            self.guide = None;
        }
    }

    /// Round a loop: kept to it as it goes sideways.
    fn guide_loops(&mut self) {
        let r = &self.runner;
        if !r.grounded {
            self.guide = None;
            return;
        }
        if let Some((i, progress)) = self.guide {
            self.guide = self.loops[i].0.guide(progress, r.pos).map(|(now, at)| {
                self.runner.pos = at;
                (i, now)
            });
            return;
        }
        self.guide = self.loops.iter().enumerate().find_map(|(i, (l, _))| l.entering(r.pos, r.vel).map(|a| (i, a)));
    }

    /// Rings, springs, panels and badniks he touches.
    fn touch(&mut self) {
        let p = self.runner.pos;
        let before = self.rings.len();
        self.rings.retain(|r| (r.0 - p).length() > RING_REACH);
        let got = (before - self.rings.len()) as u32;
        if got > 0 {
            self.ring_count += got;
            self.score += got * 10;
            self.noises.push(Noise::Ring);
        }
        let mut sprung = None;
        for (s, _) in &mut self.springs {
            s.squash = (s.squash - 0.08).max(0.0);
            let top = s.at + s.up * 0.7;
            let d = p - top;
            if d.dot(s.up).abs() < 1.1 && (d - s.up * d.dot(s.up)).length() < 0.9 {
                s.squash = 1.0;
                sprung = Some(*s);
            }
        }
        if let Some(s) = sprung {
            let keep = self.runner.vel - s.up * self.runner.vel.dot(s.up);
            let along = if s.up.y > 0.9 { keep } else { Vec3::ZERO };
            self.runner.launch(s.up * s.power + along);
            self.runner.pos = s.at + s.up * (0.8 + self.runner.half());
            self.noises.push(Noise::Spring);
        }
        if self.runner.grounded {
            for (pn, _) in &self.panels {
                if (p - pn.at).length() < 1.6 {
                    let v = self.runner.vel;
                    let speed = v.length().max(12.0);
                    self.runner.vel = pn.dir * speed;
                    self.runner.facing = pn.dir;
                    if v.length() < 11.0 {
                        self.noises.push(Noise::Dash);
                    }
                }
            }
        }
        let mut hit_by = None;
        let attacking = self.runner.attacking();
        let mut popped = vec![];
        for (k, (b, _)) in self.badniks.iter().enumerate() {
            let centre = b.at + Vec3::Y * 0.5;
            if (centre - p).length() < BADNIK_REACH {
                if attacking {
                    popped.push(k);
                } else if self.invincible == 0 {
                    hit_by = Some(centre);
                }
            }
        }
        for k in popped.into_iter().rev() {
            let (b, _) = self.badniks.remove(k);
            self.pops.push((b.at + Vec3::Y * 0.5, 0));
            self.score += 100;
            self.noises.push(Noise::Pop);
            if !self.runner.grounded {
                let v = &mut self.runner.vel;
                v.y = v.y.max(4.0).max(-v.y);
            }
        }
        if let Some(from) = hit_by {
            self.hurt(from);
        }
    }

    fn hurt(&mut self, from: Vec3) {
        if self.ring_count == 0 {
            self.die();
            return;
        }
        let n = self.ring_count.min(24);
        for k in 0..n {
            let a = k as f32 / n as f32 * std::f32::consts::TAU;
            let speed = if k % 2 == 0 { 6.0 } else { 4.0 };
            self.spilled.push(Spilled {
                at: self.runner.pos + Vec3::Y * 0.5,
                vel: vec3(a.cos() * speed, 7.0 + (k % 3) as f32, a.sin() * speed),
                life: 0,
            });
        }
        self.ring_count = 0;
        self.runner.knock_back(from);
        self.invincible = 120;
        self.noises.push(Noise::Scatter);
    }

    fn badniks_move(&mut self) {
        let target = self.runner.pos;
        for (b, _) in &mut self.badniks {
            b.t += 1.0;
            match b.kind {
                BadnikKind::Crawler => {
                    let along = (b.t / 120.0).sin() * 4.0;
                    let want = b.home + b.dir * along;
                    let from = vec3(want.x, want.y + 3.0, want.z);
                    let y = self.phys.ray(from, -Vec3::Y, 8.0).map_or(b.home.y, |h| h.point.y);
                    b.at = vec3(want.x, y, want.z);
                }
                BadnikKind::Flyer => {
                    let bob = (b.t / 30.0).sin() * 0.3;
                    let to = target - b.at;
                    if to.length() < 16.0 && (target - b.home).length() < 22.0 {
                        b.at += to.normalize_or_zero() * 0.05;
                    } else {
                        b.at += (b.home - b.at) * 0.02;
                    }
                    b.at.y += bob * 0.02;
                }
            }
        }
    }

    fn spill_move(&mut self) {
        let p = self.runner.pos;
        let dt = 1.0 / 60.0;
        let mut got = 0;
        for s in &mut self.spilled {
            s.life += 1;
            s.vel.y -= 9.81 * 2.2 * dt;
            let step = s.vel * dt;
            if let Some(h) = self.phys.ray(s.at, step.normalize_or_zero(), step.length() + 0.3) {
                s.at = h.point + h.normal * 0.3;
                s.vel = (s.vel - h.normal * 2.0 * s.vel.dot(h.normal)) * 0.7;
            } else {
                s.at += step;
            }
            if s.life > 40 && (s.at - p).length() < RING_REACH {
                s.life = u32::MAX;
                got += 1;
            }
        }
        self.spilled.retain(|s| s.life < 260);
        if got > 0 {
            self.ring_count += got;
            self.noises.push(Noise::Ring);
        }
    }

    fn spawn_thing(&mut self) {
        let f = self.cam.forward();
        let at = self.runner.pos + f * 3.0 + Vec3::Y * 3.0;
        let solid = match self.frames % 3 {
            0 => Solid::Box(Vec3::splat(0.5)),
            1 => Solid::Ball(0.5),
            _ => Solid::Cylinder(0.5, 0.4),
        };
        let body = self.phys.add_body(solid, at, f * 2.0, 0.6);
        self.things.push(Thing { body, solid });
        self.noises.push(Noise::Thud);
        if self.things.len() > 80 {
            let t = self.things.remove(0);
            self.phys.remove_body(t.body);
        }
    }

    fn animate(&mut self) {
        let r = &self.runner;
        let speed = r.speed();
        self.pose.phase += 0.1 + speed * 0.045;
        self.pose.spin += 0.35 + speed * 0.12;
        self.pose.speed = speed;
        self.pose.ball = r.is_ball();
        self.pose.air = !r.grounded;
        self.pose.flying = r.flying.is_some();
        self.pose.gliding = r.gliding.is_some();
        self.pose.hammer = r.hammer;
        self.pose.skidding = r.skidding;
        let up = if r.grounded { r.up } else { Vec3::Y };
        self.draw_up = (self.draw_up + (up - self.draw_up) * 0.35).normalize_or(Vec3::Y);
        let (anim, rate) = anim_for(r);
        if anim != self.pose.anim {
            self.pose.anim = anim;
            self.pose.time = 0.0;
        }
        self.pose.time += rate;
    }

    fn camera(&mut self, turn: f32) {
        let road = if self.maker.endless() { self.way(self.runner.pos) } else { None };
        let r = &self.runner;
        let cam = &mut self.cam;
        if turn.abs() > 0.05 {
            cam.yaw += turn * 0.045;
            cam.manual = 90;
        } else if cam.manual > 0 {
            cam.manual -= 1;
        } else {
            let guided = self.guide.map(|(i, _)| self.loops[i].0.forward);
            let want = guided.or(road).or_else(|| {
                let v = vec3(r.vel.x, 0.0, r.vel.z);
                let upright = !r.grounded || r.up.y > 0.7;
                (upright && v.length() > 1.5).then_some(v.normalize())
            });
            if let Some(w) = want {
                let target = w.z.atan2(w.x);
                let mut d = target - cam.yaw;
                d = (d + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
                let rate = if guided.is_some() { 0.12 } else { 0.035 * (r.speed() / TOP).clamp(0.3, 1.6) };
                cam.yaw += d * rate;
            }
        }
        let target = r.pos + Vec3::Y * 0.7;
        cam.at += (target - cam.at) * 0.35;
        let f = vec3(cam.yaw.cos(), 0.0, cam.yaw.sin());
        let back = -f * cam.dist * cam.pitch.cos() + Vec3::Y * (cam.dist * cam.pitch.sin() + 0.8);
        let mut eye = cam.at + back;
        // Not through the ground (but out of a loop he is going round).
        if self.guide.is_none() {
            if let Some(h) = self.phys.ray(cam.at, back.normalize(), back.length()) {
                if h.moving.is_none() && h.dist > 1.5 {
                    eye = h.point + h.normal * 0.4;
                }
            }
        }
        cam.eye += (eye - cam.eye) * 0.3;
    }

    /// Restart the run (a new course) or the zone.
    pub fn restart(&mut self, maker: Box<dyn Maker>) {
        let best = self.best.max(self.distance);
        *self = World3::new(maker, self.runner.character, best);
    }

    /// The picture: sky, ground, things, him, then the heads-up display.
    pub fn draw(&mut self, looks: &Looks) {
        for p in self.pieces.iter_mut().filter(|p| !p.dressed) {
            p.dressed = true;
            for (meshes, skin) in &mut p.meshes {
                let t = (*skin != Skin::Plain).then(|| looks.texture(*skin));
                for m in meshes {
                    m.texture = t.clone();
                }
            }
        }
        let cam = Camera3D {
            position: self.cam.eye,
            target: self.cam.at,
            up: Vec3::Y,
            fovy: 58f32.to_radians(),
            z_near: 0.1,
            z_far: 900.0,
            ..Default::default()
        };
        // Where the horizon is on the screen.
        let f = self.cam.forward();
        let far = vec3(self.cam.eye.x + f.x * 5000.0, self.cam.eye.y, self.cam.eye.z + f.z * 5000.0);
        let clip = cam.matrix() * far.extend(1.0);
        let ndc = if clip.w.abs() > 1e-6 { clip.y / clip.w } else { 0.0 };
        let horizon = (1.0 - ndc) / 2.0 * screen_height();
        set_default_camera();
        draw_sky(&self.sky, horizon);
        set_camera(&cam);
        looks.gfx.begin(&self.sky, self.cam.eye);
        for p in &self.pieces {
            for (meshes, _) in &p.meshes {
                for m in meshes {
                    draw_mesh(m);
                }
            }
        }
        if self.maker.endless() {
            let (e, s) = (self.cam.eye, 1500.0);
            let blue = Color::new(0.16, 0.42, 0.72, 1.0);
            let v = |x: f32, z: f32| macroquad::models::Vertex {
                position: vec3(e.x + x, self.sea, e.z + z),
                uv: Vec2::ZERO,
                color: super::gfx::rgba(blue),
                normal: vec4(0.0, 1.0, 0.0, 0.0),
            };
            draw_mesh(&Mesh {
                vertices: vec![v(-s, -s), v(s, -s), v(s, s), v(-s, s)],
                indices: vec![0, 1, 2, 0, 2, 3],
                texture: None,
            });
        }
        let time = self.frames as f32 / 60.0;
        let props = &looks.props;
        let pack = &looks.pack;
        for (r, _) in &self.rings {
            if (*r - self.cam.eye).length() < 160.0 {
                let m = Mat4::from_translation(*r) * Mat4::from_rotation_y(time * 3.0);
                match &pack.ring {
                    Some(ring) => ring.draw_still(m * Mat4::from_translation(vec3(0.0, -ring.height / 2.0, 0.0))),
                    None => draw_at(&props.ring, m),
                }
            }
        }
        for s in &self.spilled {
            if s.life < 180 || (s.life / 4) % 2 == 0 {
                draw_at(
                    &props.ring,
                    Mat4::from_translation(s.at)
                        * Mat4::from_rotation_y(time * 6.0)
                        * Mat4::from_scale(Vec3::splat(0.8)),
                );
            }
        }
        for (s, _) in &self.springs {
            let q = Quat::from_rotation_arc(Vec3::Y, s.up);
            let squash = 1.0 - s.squash * 0.4;
            let m = Mat4::from_rotation_translation(q, s.at) * Mat4::from_scale(vec3(1.0, squash, 1.0));
            match if s.red { &pack.spring_red } else { &pack.spring } {
                Some(model) => model.draw_still(m * Mat4::from_scale(Vec3::splat(1.5))),
                None => draw_at(if s.red { &props.spring_red } else { &props.spring }, m),
            }
        }
        for (pn, _) in &self.panels {
            let right = pn.dir.cross(pn.up).normalize_or_zero();
            let m = Mat4::from_cols(
                pn.dir.extend(0.0),
                pn.up.extend(0.0),
                right.extend(0.0),
                (pn.at + pn.up * 0.02).extend(1.0),
            );
            draw_at(&props.panel, m);
        }
        for (b, _) in &self.badniks {
            let d = b.dir.normalize_or(Vec3::X);
            let face = match b.kind {
                BadnikKind::Crawler => {
                    let going = (b.t / 120.0).cos();
                    if going >= 0.0 {
                        d
                    } else {
                        -d
                    }
                }
                BadnikKind::Flyer => {
                    let to = self.runner.pos - b.at;
                    vec3(to.x, 0.0, to.z).normalize_or(d)
                }
            };
            let right = face.cross(Vec3::Y).normalize_or_zero();
            let m = Mat4::from_cols(face.extend(0.0), Vec3::Y.extend(0.0), right.extend(0.0), b.at.extend(1.0));
            let model = if b.kind == BadnikKind::Crawler { &pack.crawler } else { &pack.flyer };
            match model {
                Some(model) => {
                    let anim = model.anims.keys().next().cloned().unwrap_or_default();
                    model.draw_anim(m, &anim, b.t * 0.15);
                }
                None => draw_at(if b.kind == BadnikKind::Crawler { &props.crawler } else { &props.flyer }, m),
            }
        }
        for t in &self.things {
            let Some((at, rot)) = self.phys.pose(t.body) else { continue };
            let q = Quat::from_xyzw(rot.i, rot.j, rot.k, rot.w);
            let place = Mat4::from_rotation_translation(q, at);
            match t.solid {
                Solid::Box(h) => draw_at(&props.crate_, place * Mat4::from_scale(h * 2.0)),
                Solid::Ball(r) => draw_at(&props.ball, place * Mat4::from_scale(Vec3::splat(r * 2.0))),
                Solid::Cylinder(h, r) => {
                    draw_at(&props.barrel, place * Mat4::from_scale(vec3(r / 0.4, h * 2.0, r / 0.4)))
                }
            }
        }
        // Badniks bursting, and blasts: bits flying out.
        let bursts = self.pops.iter().map(|(p, t)| (*p, *t as f32 / 24.0, 6, 1.5, 0.3));
        let bursts = bursts.chain(self.blasts.iter().map(|(p, t)| (*p, *t as f32 / 30.0, 14, 7.0, 0.6)));
        for (p, k, n, reach, size) in bursts {
            for j in 0..n {
                let a = j as f32 / n as f32 * std::f32::consts::TAU;
                let at = p + vec3(a.cos(), 0.6 + k, a.sin()) * k * reach;
                draw_at(&props.ball, Mat4::from_translation(at) * Mat4::from_scale(Vec3::splat(size * (1.0 - k))));
            }
        }
        // His shadow, and him (blinking while he cannot be hurt).
        let r = &self.runner;
        if let Some(h) = self.phys.ray(r.pos, -Vec3::Y, 30.0) {
            let far = (h.dist / 10.0).clamp(0.0, 0.8);
            let q = Quat::from_rotation_arc(Vec3::Y, h.normal);
            let m = Mat4::from_rotation_translation(q, h.point + h.normal * 0.04)
                * Mat4::from_scale(Vec3::splat(1.6 * (1.0 - far)));
            draw_at(&props.shadow, m);
        }
        if self.invincible == 0 || (self.invincible / 3).is_multiple_of(2) {
            let up = self.draw_up;
            let fwd = {
                let f = r.facing - up * r.facing.dot(up);
                if f.length() > 1e-3 {
                    f.normalize()
                } else {
                    vec3(r.vel.x, 0.0, r.vel.z).normalize_or(Vec3::X)
                }
            };
            let right = fwd.cross(up).normalize_or_zero();
            let feet = r.pos - up * r.half();
            let place = Mat4::from_cols(fwd.extend(0.0), up.extend(0.0), right.extend(0.0), feet.extend(1.0));
            match &pack.character {
                Some(model) => {
                    // A ball of one still pose is spun round.
                    let ball = matches!(self.pose.anim, "ROLL" | "SPIN") && model.still_ball();
                    let place = if ball {
                        let mid = Mat4::from_translation(vec3(0.0, r.half(), 0.0));
                        place * mid * Mat4::from_rotation_z(-self.pose.spin) * mid.inverse()
                    } else {
                        place
                    };
                    model.draw_anim(place, self.pose.anim, self.pose.time);
                    // Tails' tails are frames of their own.
                    if model.has("TAL0") {
                        let tails = if r.flying.is_some() {
                            "TAL7"
                        } else if !r.grounded || r.speed() > 0.5 {
                            "TAL1"
                        } else {
                            "TAL0"
                        };
                        let tails = if model.has(tails) { tails } else { "TAL0" };
                        model.draw_anim(place, tails, self.frames as f32 * 0.3);
                    }
                }
                None => looks.figure.draw(place, &self.pose),
            }
        }
        looks.gfx.end();
        set_default_camera();
        self.hud();
    }

    fn hud(&self) {
        use crate::ui::theme::{text_bold, text_bold_centered};
        let shadowed = |s: &str, x: f32, y: f32, size: f32, c: Color| {
            text_bold(s, x + 2.0, y + 2.0, size, Color::new(0.0, 0.0, 0.0, 0.55));
            text_bold(s, x, y, size, c);
        };
        let yellow = Color::new(1.0, 0.86, 0.2, 1.0);
        shadowed("SCORE", 24.0, 40.0, 22.0, yellow);
        shadowed(&self.score.to_string(), 120.0, 40.0, 22.0, WHITE);
        let secs = self.frames / 60;
        shadowed("TIME", 24.0, 68.0, 22.0, yellow);
        shadowed(&format!("{}:{:02}", secs / 60, secs % 60), 120.0, 68.0, 22.0, WHITE);
        let flash = self.ring_count == 0 && (self.frames / 20).is_multiple_of(2);
        shadowed("RINGS", 24.0, 96.0, 22.0, if flash { RED } else { yellow });
        shadowed(&self.ring_count.to_string(), 120.0, 96.0, 22.0, WHITE);
        if self.maker.endless() {
            shadowed(&format!("{} m", self.distance as u32), screen_width() - 160.0, 40.0, 26.0, WHITE);
            if self.best >= 20.0 {
                let c = if self.distance > self.best { yellow } else { Color::new(1.0, 1.0, 1.0, 0.75) };
                shadowed(&format!("record {} m", self.best as u32), screen_width() - 160.0, 66.0, 16.0, c);
            }
        }
        let speed = self.runner.speed() * K * 60.0 * 3.6;
        shadowed(&format!("{:.0} km/h", speed), 24.0, screen_height() - 24.0, 18.0, Color::new(1.0, 1.0, 1.0, 0.8));
        if self.over {
            let (w, h) = (screen_width(), screen_height());
            draw_rectangle(0.0, 0.0, w, h, Color::new(0.0, 0.0, 0.0, 0.35));
            text_bold_centered(&format!("{} m", self.distance as u32), w / 2.0, h / 2.0 - 30.0, 48.0, WHITE);
            let line = if self.distance > self.best && self.best >= 20.0 {
                "New record!".to_string()
            } else if self.best >= 20.0 {
                format!("Record: {} m", self.best as u32)
            } else {
                String::new()
            };
            text_bold_centered(&line, w / 2.0, h / 2.0 + 14.0, 22.0, yellow);
            text_bold_centered("Space: run again  ·  Esc: back", w / 2.0, h / 2.0 + 52.0, 18.0, WHITE);
        }
    }
}

/// The model pack's animation for what he is doing (SRB2's names), and
/// how many frames of it go by in a frame of the game.
fn anim_for(r: &Runner) -> (&'static str, f32) {
    let speed = r.speed();
    if r.hurt {
        ("PAIN", 0.1)
    } else if r.spindash.is_some() {
        ("SPIN", 0.6)
    } else if r.is_ball() {
        ("ROLL", 0.35 + speed * 0.04)
    } else if r.gliding.is_some() {
        ("GLID", 0.1)
    } else if r.flying.is_some() {
        ("FLY_", 0.4)
    } else if r.hammer.is_some() {
        ("MLEE", 0.25)
    } else if !r.grounded {
        if r.vel.y > 0.0 {
            ("SPNG", 0.1)
        } else {
            ("FALL", 0.15)
        }
    } else if r.skidding {
        ("SKID", 0.2)
    } else if speed < 0.05 {
        if r.idle > 360 {
            ("WAIT", 0.04)
        } else {
            ("STND", 0.0)
        }
    } else if speed < 4.0 {
        ("WALK", 0.06 + speed * 0.05)
    } else if speed < 9.5 {
        ("RUN_", 0.12 + speed * 0.025)
    } else {
        ("DASH", 0.4)
    }
}
