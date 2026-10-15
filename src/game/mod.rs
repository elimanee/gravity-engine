//! Sonic's game mode: badniks (a crawler, a flying shooter, a crab),
//! spikes, item monitors, star posts, the goal signpost and a boss; rings
//! scattered when hurt, lives, score and time, and the end-of-act tally.
//!
//! A level's contents are a [`LevelSpec`] saved in scene files. At play
//! time they become a [`Game`], updated after Sonic each frame; what it
//! wants heard or seen comes back as [`Effect`]s.

pub mod art;

use crate::config::PPM;
use crate::physics::{to_phys, to_screen, PhysWorld};
use crate::sonic::{Ground, Sonic, FPS, SCALE};
use macroquad::prelude::{vec2, Vec2};
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

/// Guide pixels per frame → world pixels per second.
const SPEED: f32 = SCALE * FPS;
/// Seconds of safety after a hit.
const HURT_TIME: f32 = 2.0;
/// How long invincibility and speed shoes last (s).
const POWER_TIME: f32 = 20.0;
/// Scattered rings: when they can be caught again and when they vanish (s).
const RING_CATCHABLE: f32 = 64.0 / 60.0;
const RING_LIFE: f32 = 256.0 / 60.0;
/// Pauses before respawning, before the tally, and on Game Over (s).
const DEATH_TIME: f32 = 2.5;
const CLEAR_DELAY: f32 = 1.5;
const GAME_OVER_TIME: f32 = 4.0;
/// Lives at the start.
pub const LIVES: u32 = 3;
/// Boss hits.
const BOSS_HP: u32 = 8;
/// Sizes (world px): monitor and spikes boxes, ring and boss radii.
pub const MONITOR: Vec2 = Vec2::new(45.0, 48.0);
pub const SPIKES: Vec2 = Vec2::new(48.0, 24.0);
const RING_R: f32 = 10.0;
pub const BOSS_R: f32 = 36.0;

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EnemyKind {
    /// Rolls along the ground, turning at edges and walls.
    #[default]
    Crawler,
    /// Flies to and fro and shoots at Sonic.
    Flyer,
    /// Walks slowly and lobs two shots.
    Crab,
}

impl EnemyKind {
    pub const ALL: &'static [EnemyKind] = &[EnemyKind::Crawler, EnemyKind::Flyer, EnemyKind::Crab];

    pub fn label(self) -> &'static str {
        match self {
            EnemyKind::Crawler => "Crawler",
            EnemyKind::Flyer => "Flyer",
            EnemyKind::Crab => "Crab",
        }
    }

    /// Half size of its hit box (world px).
    fn half(self) -> Vec2 {
        match self {
            EnemyKind::Crawler => vec2(20.0, 16.0),
            EnemyKind::Flyer => vec2(22.0, 13.0),
            EnemyKind::Crab => vec2(24.0, 16.0),
        }
    }
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum MonitorKind {
    #[default]
    Rings,
    Shield,
    Invincible,
    Shoes,
    Life,
}

impl MonitorKind {
    pub const ALL: &'static [MonitorKind] =
        &[MonitorKind::Rings, MonitorKind::Shield, MonitorKind::Invincible, MonitorKind::Shoes, MonitorKind::Life];

    pub fn label(self) -> &'static str {
        match self {
            MonitorKind::Rings => "10 rings",
            MonitorKind::Shield => "Shield",
            MonitorKind::Invincible => "Invincible",
            MonitorKind::Shoes => "Speed shoes",
            MonitorKind::Life => "1-up",
        }
    }
}

/// Something placed in a level, at a point in metres.
#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq)]
pub struct Placed<K> {
    pub kind: K,
    pub at: [f32; 2],
}

/// What makes a scene a level (saved in scene files). Points in metres.
#[derive(Serialize, Deserialize, Clone, Debug, Default, PartialEq)]
#[serde(default)]
pub struct LevelSpec {
    pub name: String,
    pub enemies: Vec<Placed<EnemyKind>>,
    pub monitors: Vec<Placed<MonitorKind>>,
    pub spikes: Vec<[f32; 2]>,
    pub posts: Vec<[f32; 2]>,
    pub goal: Option<[f32; 2]>,
    pub boss: Option<[f32; 2]>,
}

impl LevelSpec {
    pub fn is_empty(&self) -> bool {
        self.enemies.is_empty()
            && self.monitors.is_empty()
            && self.spikes.is_empty()
            && self.posts.is_empty()
            && self.goal.is_none()
            && self.boss.is_none()
    }
}

fn px(at: [f32; 2]) -> Vec2 {
    to_screen(at[0], at[1])
}

pub struct Enemy {
    pub kind: EnemyKind,
    pub pos: Vec2,
    vel: Vec2,
    /// Facing / walking direction (±1).
    pub dir: f32,
    /// Its own clock, and where it started (flyers patrol around it).
    pub timer: f32,
    home: Vec2,
    /// Stopped to shoot.
    pub firing: bool,
}

pub struct Shot {
    pub pos: Vec2,
    vel: Vec2,
    life: f32,
    falls: bool,
}

pub struct Monitor {
    pub kind: MonitorKind,
    pub pos: Vec2,
    body: Option<RigidBodyHandle>,
    pub collider: Option<ColliderHandle>,
    /// Seconds since it was broken (the icon floats up).
    pub broken: Option<f32>,
}

pub struct Spike {
    pub pos: Vec2,
    body: RigidBodyHandle,
    pub collider: ColliderHandle,
}

pub struct Post {
    pub pos: Vec2,
    pub lit: Option<f32>,
}

pub struct LostRing {
    pub pos: Vec2,
    vel: Vec2,
    pub age: f32,
}

pub struct Boss {
    pub pos: Vec2,
    home: Vec2,
    pub hp: u32,
    /// Seconds of flashing after a hit (it cannot be hit meanwhile).
    pub flash: f32,
    pub t: f32,
    dir: f32,
    bomb: f32,
    /// Seconds since it was beaten.
    pub beaten: Option<f32>,
}

/// The end-of-act tally: bonuses counted into the score.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tally {
    pub t: f32,
    pub time_bonus: u32,
    pub ring_bonus: u32,
}

impl Tally {
    pub fn done(&self) -> bool {
        self.time_bonus == 0 && self.ring_bonus == 0 && self.t > 2.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Phase {
    Playing,
    Dying(f32),
    Cleared(Tally),
    Over(f32),
}

/// Things for the app to show or play.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Effect {
    EnemyDown {
        at: Vec2,
        score: u32,
    },
    Hurt(Vec2),
    RingsLost(Vec2),
    RingCaught(Vec2),
    ShieldLost(Vec2),
    Died(Vec2),
    Monitor(MonitorKind, Vec2),
    ExtraLife,
    Post(Vec2),
    Goal(Vec2),
    Cleared,
    Shot(Vec2),
    BombBlast(Vec2),
    BossHit(Vec2),
    BossBeaten(Vec2),
    Explosion(Vec2),
    Respawned(Vec2),
    GameOver,
    /// Start the level again (after Game Over).
    Restart,
}

/// Does this collider count as ground for badniks and scattered rings?
pub type Solid<'a> = &'a dyn Fn(ColliderHandle) -> bool;

/// Distance (world px) to what a ray from `from` along `dir` meets.
fn ray(world: &PhysWorld, solid: Solid, from: Vec2, dir: Vec2, len: f32) -> Option<f32> {
    let pred = |h: ColliderHandle, _: &Collider| solid(h);
    let filter = QueryFilter::default().exclude_sensors().predicate(&pred);
    let (x, y) = to_phys(from.x, from.y);
    world.cast_ray(Point::new(x, y), vector![dir.x, -dir.y], len / PPM, filter).map(|(_, toi, _)| toi * PPM)
}

fn overlap(a: Vec2, ha: Vec2, b: Vec2, hb: Vec2) -> bool {
    (a.x - b.x).abs() < ha.x + hb.x && (a.y - b.y).abs() < ha.y + hb.y
}

/// Time bonus of the Mega Drive games.
pub fn time_bonus(secs: f32) -> u32 {
    match secs {
        s if s < 30.0 => 50_000,
        s if s < 45.0 => 10_000,
        s if s < 60.0 => 5_000,
        s if s < 90.0 => 4_000,
        s if s < 120.0 => 3_000,
        s if s < 180.0 => 2_000,
        s if s < 240.0 => 1_000,
        s if s < 300.0 => 500,
        _ => 0,
    }
}

/// Score for the n-th badnik destroyed in one go (without landing).
fn chain_score(n: u32) -> u32 {
    match n {
        0 => 100,
        1 => 200,
        2 => 500,
        3..=14 => 1_000,
        _ => 10_000,
    }
}

fn fixed_box(world: &mut PhysWorld, centre: Vec2, size: Vec2) -> (RigidBodyHandle, ColliderHandle) {
    let (x, y) = to_phys(centre.x, centre.y);
    let body = world.bodies.insert(RigidBodyBuilder::fixed().translation(vector![x, y]));
    let col = ColliderBuilder::cuboid(size.x / 2.0 / PPM, size.y / 2.0 / PPM).friction(0.6);
    let collider = world.colliders.insert_with_parent(col, body, &mut world.bodies);
    (body, collider)
}

pub struct Game {
    pub spec: LevelSpec,
    pub enemies: Vec<Enemy>,
    pub shots: Vec<Shot>,
    pub monitors: Vec<Monitor>,
    pub spikes: Vec<Spike>,
    pub posts: Vec<Post>,
    pub lost: Vec<LostRing>,
    pub goal: Option<Vec2>,
    /// Seconds since Sonic ran past the goal.
    pub goal_passed: Option<f32>,
    pub boss: Option<Boss>,
    pub score: u32,
    pub lives: u32,
    pub time: f32,
    pub rings: u32,
    /// Rings already turned into extra lives (every 100).
    ring_lives: u32,
    pub invulnerable: f32,
    pub shield: bool,
    pub invincible: f32,
    pub shoes: f32,
    /// Where (and when) he comes back after losing a life.
    checkpoint: Option<(Vec2, f32)>,
    start: Vec2,
    pub phase: Phase,
    chain: u32,
    /// An endless run: no lives, the run ends at the first death.
    pub endless: bool,
}

impl Game {
    /// Set a level up in `world`; Sonic starts at `start` (world px).
    pub fn new(spec: &LevelSpec, world: &mut PhysWorld, start: Vec2) -> Self {
        let mut g = Game {
            spec: spec.clone(),
            enemies: vec![],
            shots: vec![],
            monitors: vec![],
            spikes: vec![],
            posts: spec.posts.iter().map(|&p| Post { pos: px(p), lit: None }).collect(),
            lost: vec![],
            goal: spec.goal.map(px),
            goal_passed: None,
            boss: spec.boss.map(px).map(|p| Boss {
                pos: p,
                home: p,
                hp: BOSS_HP,
                flash: 0.0,
                t: 0.0,
                dir: 1.0,
                bomb: 2.0,
                beaten: None,
            }),
            score: 0,
            lives: LIVES,
            time: 0.0,
            rings: 0,
            ring_lives: 0,
            invulnerable: 0.0,
            shield: false,
            invincible: 0.0,
            shoes: 0.0,
            checkpoint: None,
            start,
            phase: Phase::Playing,
            chain: 0,
            endless: false,
        };
        // With a boss, the goal only shows once it is beaten.
        if g.boss.is_some() {
            g.goal = None;
        }
        for e in &spec.enemies {
            g.add_enemy(e.kind, px(e.at));
        }
        for m in &spec.monitors {
            g.add_monitor(world, m.kind, px(m.at));
        }
        for &s in &spec.spikes {
            g.add_spikes(world, px(s));
        }
        g
    }

    pub fn add_enemy(&mut self, kind: EnemyKind, at: Vec2) {
        let dir = -1.0;
        self.enemies.push(Enemy { kind, pos: at, vel: Vec2::ZERO, dir, timer: 0.0, home: at, firing: false });
    }

    pub fn add_monitor(&mut self, world: &mut PhysWorld, kind: MonitorKind, at: Vec2) {
        let (body, collider) = fixed_box(world, at, MONITOR);
        self.monitors.push(Monitor { kind, pos: at, body: Some(body), collider: Some(collider), broken: None });
    }

    pub fn add_spikes(&mut self, world: &mut PhysWorld, at: Vec2) {
        let (body, collider) = fixed_box(world, at, SPIKES);
        self.spikes.push(Spike { pos: at, body, collider });
    }

    /// Take the level's bodies out of the world.
    pub fn remove(&self, world: &mut PhysWorld) {
        for m in &self.monitors {
            if let Some(b) = m.body {
                world.remove_body(b);
            }
        }
        for s in &self.spikes {
            world.remove_body(s.body);
        }
    }

    /// Forget what lies far behind `x` (endless runs).
    pub fn forget_before(&mut self, world: &mut PhysWorld, x: f32) {
        self.enemies.retain(|e| e.pos.x > x);
        self.shots.retain(|s| s.pos.x > x);
        self.lost.retain(|r| r.pos.x > x);
        self.posts.retain(|p| p.pos.x > x);
        let (old, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.monitors).into_iter().partition(|m| m.pos.x < x);
        for m in old {
            if let Some(b) = m.body {
                world.remove_body(b);
            }
        }
        self.monitors = keep;
        let (old, keep): (Vec<_>, Vec<_>) = std::mem::take(&mut self.spikes).into_iter().partition(|s| s.pos.x < x);
        for s in old {
            world.remove_body(s.body);
        }
        self.spikes = keep;
    }

    /// Everything moved `dx` px to the left (infinite world).
    pub fn shift(&mut self, dx: f32) {
        let d = vec2(dx, 0.0);
        for e in &mut self.enemies {
            e.pos -= d;
            e.home -= d;
        }
        for s in &mut self.shots {
            s.pos -= d;
        }
        for m in &mut self.monitors {
            m.pos -= d;
        }
        for s in &mut self.spikes {
            s.pos -= d;
        }
        for p in &mut self.posts {
            p.pos -= d;
        }
        for r in &mut self.lost {
            r.pos -= d;
        }
        if let Some(g) = &mut self.goal {
            *g -= d;
        }
        if let Some(b) = &mut self.boss {
            b.pos -= d;
            b.home -= d;
        }
        if let Some((p, _)) = &mut self.checkpoint {
            *p -= d;
        }
        self.start -= d;
    }

    /// The level's own colliders, as ground for Sonic.
    pub fn grounds(&self) -> Vec<(ColliderHandle, Ground)> {
        let monitors = self
            .monitors
            .iter()
            .filter_map(|m| m.collider)
            .map(|c| (c, Ground { breakable: true, friction: 0.6, ..Ground::default() }));
        let spikes = self.spikes.iter().map(|s| (s.collider, Ground { friction: 0.6, ..Ground::default() }));
        monitors.chain(spikes).collect()
    }

    /// Sonic is out of control (dying, past the goal, tallying).
    pub fn locks_input(&self) -> bool {
        self.phase != Phase::Playing || self.goal_passed.is_some()
    }

    /// Invulnerable after a hit: he blinks.
    pub fn blinking(&self, t: f32) -> bool {
        self.invulnerable > 0.0 && (t * 20.0) as i32 % 2 == 0
    }

    fn attacking(&self, sonic: &Sonic) -> bool {
        sonic.attacking() || self.invincible > 0.0
    }

    /// Rings caught (outside the game's own scattered ones).
    pub fn add_rings(&mut self, n: u32) -> Vec<Effect> {
        self.rings += n;
        let mut out = vec![];
        while self.rings / 100 > self.ring_lives {
            self.ring_lives += 1;
            self.lives += 1;
            out.push(Effect::ExtraLife);
        }
        out
    }

    /// A broken monitor (Sonic's `Break` event on `collider`); its item.
    pub fn break_monitor(&mut self, world: &mut PhysWorld, collider: ColliderHandle, sonic: &mut Sonic) -> Vec<Effect> {
        let Some(i) = self.monitors.iter().position(|m| m.collider == Some(collider)) else { return vec![] };
        let m = &mut self.monitors[i];
        if let Some(b) = m.body.take() {
            world.remove_body(b);
        }
        m.collider = None;
        m.broken = Some(0.0);
        let (kind, at) = (m.kind, m.pos);
        self.score += 10;
        sonic.rebound();
        let mut out = vec![Effect::Monitor(kind, at)];
        match kind {
            MonitorKind::Rings => out.extend(self.add_rings(10)),
            MonitorKind::Shield => self.shield = true,
            MonitorKind::Invincible => self.invincible = POWER_TIME,
            MonitorKind::Shoes => self.shoes = POWER_TIME,
            MonitorKind::Life => {
                self.lives += 1;
                out.push(Effect::ExtraLife);
            }
        }
        out
    }

    /// Is `collider` one of this level's monitors?
    pub fn is_monitor(&self, collider: ColliderHandle) -> bool {
        self.monitors.iter().any(|m| m.collider == Some(collider))
    }

    /// Sonic got hit by something at `from` (world px).
    pub fn hurt(&mut self, sonic: &mut Sonic, from: Vec2, out: &mut Vec<Effect>) {
        if self.invulnerable > 0.0 || self.invincible > 0.0 || sonic.dead || self.phase != Phase::Playing {
            return;
        }
        if self.shield {
            self.shield = false;
            out.push(Effect::ShieldLost(sonic.pos));
        } else if self.rings > 0 {
            self.scatter(sonic.pos);
            out.push(Effect::RingsLost(sonic.pos));
        } else {
            self.die(sonic, out);
            return;
        }
        sonic.knock(from.x);
        self.invulnerable = HURT_TIME;
        out.push(Effect::Hurt(sonic.pos));
    }

    /// The rings fly out in two circles, as in the games.
    fn scatter(&mut self, at: Vec2) {
        let n = self.rings.min(32);
        self.rings = 0;
        let mut angle = 101.25f32.to_radians();
        let mut speed = 4.0;
        let mut flip = false;
        for i in 0..n {
            if i == 16 {
                speed = 2.0;
                angle = 101.25f32.to_radians();
            }
            let mut v = vec2(angle.cos() * speed, -angle.sin() * speed);
            if flip {
                v.x = -v.x;
                angle += 22.5f32.to_radians();
            }
            flip = !flip;
            self.lost.push(LostRing { pos: at, vel: v * SPEED, age: 0.0 });
        }
    }

    fn die(&mut self, sonic: &mut Sonic, out: &mut Vec<Effect>) {
        if sonic.dead {
            return;
        }
        sonic.die();
        self.shield = false;
        self.invincible = 0.0;
        self.shoes = 0.0;
        sonic.shoes = false;
        self.phase = Phase::Dying(0.0);
        out.push(Effect::Died(sonic.pos));
    }

    /// Advance by `dt` s, after Sonic has moved. `fall_y`: below this
    /// (world px) he has fallen out of the level.
    pub fn update(&mut self, dt: f32, world: &PhysWorld, solid: Solid, sonic: &mut Sonic, fall_y: f32) -> Vec<Effect> {
        let mut out = vec![];
        match self.phase {
            Phase::Dying(t) => {
                let t = t + dt;
                self.phase = Phase::Dying(t);
                if t > DEATH_TIME {
                    if self.endless {
                        self.phase = Phase::Over(0.0);
                        out.push(Effect::GameOver);
                    } else {
                        self.lives = self.lives.saturating_sub(1);
                        if self.lives == 0 {
                            self.phase = Phase::Over(0.0);
                            out.push(Effect::GameOver);
                        } else {
                            let (at, time) = self.checkpoint.unwrap_or((self.start, 0.0));
                            *sonic = Sonic::new(at);
                            self.rings = 0;
                            self.ring_lives = 0;
                            self.time = time;
                            self.invulnerable = 0.0;
                            self.lost.clear();
                            self.shots.clear();
                            self.phase = Phase::Playing;
                            out.push(Effect::Respawned(at));
                        }
                    }
                }
                self.update_world(dt, world, solid, sonic, &mut out, false);
                return out;
            }
            Phase::Over(t) => {
                let t = t + dt;
                self.phase = Phase::Over(t);
                if t > GAME_OVER_TIME && !self.endless {
                    out.push(Effect::Restart);
                }
                return out;
            }
            Phase::Cleared(mut tally) => {
                tally.t += dt;
                if tally.t > 1.0 {
                    // Count the bonuses into the score.
                    let step = ((dt * 30_000.0) as u32).max(100);
                    let a = tally.time_bonus.min(step);
                    let b = tally.ring_bonus.min(step);
                    tally.time_bonus -= a;
                    tally.ring_bonus -= b;
                    self.score += a + b;
                }
                self.phase = Phase::Cleared(tally);
                self.update_world(dt, world, solid, sonic, &mut out, false);
                return out;
            }
            Phase::Playing => {}
        }
        self.time += dt;
        self.invulnerable = (self.invulnerable - dt).max(0.0);
        self.invincible = (self.invincible - dt).max(0.0);
        self.shoes = (self.shoes - dt).max(0.0);
        sonic.shoes = self.shoes > 0.0;
        if sonic.grounded {
            self.chain = 0;
        }
        if sonic.pos.y > fall_y && !sonic.dead {
            self.die(sonic, &mut out);
        }
        // Spikes hurt when stood on.
        if let Some(s) = self.spikes.iter().find(|s| sonic.grounded && sonic.on == Some(s.collider)) {
            let from = vec2(s.pos.x - sonic.facing, s.pos.y);
            self.hurt(sonic, from, &mut out);
        }
        // Star posts and the goal.
        for p in &mut self.posts {
            if p.lit.is_none() && (p.pos.x - sonic.pos.x).abs() < 24.0 && (p.pos.y - sonic.pos.y).abs() < 90.0 {
                p.lit = Some(0.0);
                self.checkpoint = Some((vec2(p.pos.x, p.pos.y), self.time));
                out.push(Effect::Post(p.pos));
            }
        }
        if let (Some(g), None) = (self.goal, self.goal_passed) {
            if sonic.pos.x > g.x - 12.0 && (g.y - sonic.pos.y).abs() < 160.0 {
                self.goal_passed = Some(0.0);
                out.push(Effect::Goal(g));
            }
        }
        if let Some(t) = &mut self.goal_passed {
            *t += dt;
            if *t > CLEAR_DELAY {
                self.phase =
                    Phase::Cleared(Tally { t: 0.0, time_bonus: time_bonus(self.time), ring_bonus: self.rings * 100 });
                out.push(Effect::Cleared);
            }
        }
        self.update_world(dt, world, solid, sonic, &mut out, true);
        out
    }

    /// Badniks, shots, scattered rings and the boss.
    fn update_world(
        &mut self,
        dt: f32,
        world: &PhysWorld,
        solid: Solid,
        sonic: &mut Sonic,
        out: &mut Vec<Effect>,
        live: bool,
    ) {
        let me = sonic.pos;
        let (wr, hr) = sonic.radii();
        let my_half = vec2(wr * SCALE, hr * SCALE) * 0.9;
        let live = live && !sonic.dead;

        // Badniks.
        let attacking = self.attacking(sonic);
        let mut killed = vec![];
        let mut hurt_from = None;
        for (i, e) in self.enemies.iter_mut().enumerate() {
            e.timer += dt;
            match e.kind {
                EnemyKind::Crawler | EnemyKind::Crab => {
                    let crab = e.kind == EnemyKind::Crab;
                    // Crabs stop now and then to lob two shots.
                    let cycle = e.timer % 4.0;
                    let was = e.firing;
                    e.firing = crab && cycle > 2.8;
                    if crab && e.firing && !was && (e.pos - me).length() < 700.0 {
                        for d in [-1.0, 1.0] {
                            self.shots.push(Shot {
                                pos: e.pos + vec2(d * 20.0, -12.0),
                                vel: vec2(d * 130.0, -380.0),
                                life: 3.0,
                                falls: true,
                            });
                        }
                        out.push(Effect::Shot(e.pos));
                    }
                    let speed = if crab { 45.0 } else { 90.0 };
                    let half = e.kind.half();
                    // On the ground?
                    match ray(world, solid, e.pos, vec2(0.0, 1.0), half.y + 12.0) {
                        Some(d) => {
                            e.pos.y += d - half.y;
                            e.vel.y = 0.0;
                            if !e.firing {
                                let ahead = e.pos + vec2(e.dir * (half.x + 4.0), 0.0);
                                let edge = ray(world, solid, ahead, vec2(0.0, 1.0), half.y + 20.0).is_none();
                                let wall =
                                    ray(world, solid, e.pos, vec2(e.dir, 0.0), half.x + speed * dt + 2.0).is_some();
                                if edge || wall {
                                    e.dir = -e.dir;
                                } else {
                                    e.pos.x += e.dir * speed * dt;
                                }
                            }
                        }
                        None => {
                            e.vel.y = (e.vel.y + 900.0 * dt).min(900.0);
                            e.pos.y += e.vel.y * dt;
                        }
                    }
                    if e.pos.y > fall_limit(world) {
                        killed.push((i, false));
                        continue;
                    }
                }
                EnemyKind::Flyer => {
                    let cycle = e.timer % 3.0;
                    let was = e.firing;
                    e.firing = cycle > 2.2;
                    if e.firing {
                        // Turn to face Sonic and fire once.
                        e.dir = if me.x < e.pos.x { -1.0 } else { 1.0 };
                        if !was && (me - e.pos).length() < 520.0 && me.y > e.pos.y - 40.0 {
                            let v = (me - e.pos).normalize_or(vec2(e.dir, 1.0)) * 280.0;
                            self.shots.push(Shot {
                                pos: e.pos + vec2(e.dir * 18.0, 12.0),
                                vel: v,
                                life: 3.0,
                                falls: false,
                            });
                            out.push(Effect::Shot(e.pos));
                        }
                    } else {
                        e.pos.x += e.dir * 110.0 * dt;
                        if (e.pos.x - e.home.x).abs() > 240.0 {
                            e.dir = (e.home.x - e.pos.x).signum();
                        }
                        e.pos.y = e.home.y + (e.timer * 3.0).sin() * 6.0;
                    }
                }
            }
            if live && overlap(me, my_half, e.pos, e.kind.half()) {
                if attacking {
                    killed.push((i, true));
                } else {
                    hurt_from.get_or_insert(e.pos);
                }
            }
        }
        // Highest index first, so the others stay valid.
        for &(i, by_player) in killed.iter().rev() {
            let e = self.enemies.remove(i);
            if by_player {
                let score = chain_score(self.chain);
                self.chain += 1;
                self.score += score;
                sonic.rebound();
                out.push(Effect::EnemyDown { at: e.pos, score });
            }
        }
        if let Some(from) = hurt_from {
            self.hurt(sonic, from, out);
        }

        // Shots.
        let mut hit_by = None;
        let mut blasts = vec![];
        self.shots.retain_mut(|s| {
            s.life -= dt;
            if s.falls {
                s.vel.y += 700.0 * dt;
            }
            let step = s.vel * dt;
            // Bombs go off where they land.
            if s.falls && s.vel.y > 0.0 && ray(world, solid, s.pos, vec2(0.0, 1.0), step.y + 4.0).is_some() {
                blasts.push(s.pos);
                return false;
            }
            s.pos += step;
            if live && overlap(me, my_half, s.pos, vec2(6.0, 6.0)) {
                hit_by = Some(s.pos);
                return false;
            }
            s.life > 0.0
        });
        for at in blasts {
            out.push(Effect::BombBlast(at));
            if live && (me - at).length() < 52.0 {
                hit_by = Some(at);
            }
        }
        if let Some(from) = hit_by {
            self.hurt(sonic, from, out);
        }

        // Scattered rings bounce about, blink, and can be caught again.
        let mut caught = 0;
        let gravity = 0.09375 * SCALE * FPS * FPS;
        self.lost.retain_mut(|r| {
            r.age += dt;
            r.vel.y += gravity * dt;
            let step = r.vel * dt;
            if r.vel.y > 0.0 && ray(world, solid, r.pos, vec2(0.0, 1.0), step.y + RING_R).is_some() {
                r.vel.y = -r.vel.y * 0.75;
            }
            if r.vel.x != 0.0 && ray(world, solid, r.pos, vec2(r.vel.x.signum(), 0.0), step.x.abs() + RING_R).is_some()
            {
                r.vel.x = -r.vel.x * 0.75;
            }
            r.pos += r.vel * dt;
            if live && r.age > RING_CATCHABLE && overlap(me, my_half, r.pos, vec2(RING_R, RING_R)) {
                caught += 1;
                return false;
            }
            r.age < RING_LIFE
        });
        if caught > 0 {
            out.extend(self.add_rings(caught));
            out.push(Effect::RingCaught(me));
        }

        // Monitors' icons float up after breaking.
        for m in &mut self.monitors {
            if let Some(t) = &mut m.broken {
                *t += dt;
            }
        }
        for p in &mut self.posts {
            if let Some(t) = &mut p.lit {
                *t += dt;
            }
        }

        // The boss: to and fro above its arena, dropping bombs.
        let mut boss_hurts = None;
        if let Some(b) = &mut self.boss {
            b.t += dt;
            b.flash = (b.flash - dt).max(0.0);
            match &mut b.beaten {
                Some(t) => {
                    *t += dt;
                    b.pos.y += 60.0 * dt * (*t).min(2.0);
                    if (*t * 6.0) as i32 != ((*t - dt) * 6.0) as i32 {
                        let jitter =
                            vec2(macroquad::rand::gen_range(-30.0, 30.0), macroquad::rand::gen_range(-30.0, 30.0));
                        out.push(Effect::Explosion(b.pos + jitter));
                    }
                }
                None => {
                    let speed = 110.0 + (BOSS_HP - b.hp) as f32 * 12.0;
                    b.pos.x += b.dir * speed * dt;
                    if (b.pos.x - b.home.x).abs() > 280.0 {
                        b.dir = (b.home.x - b.pos.x).signum();
                    }
                    b.pos.y = b.home.y + (b.t * 2.0).sin() * 14.0;
                    b.bomb -= dt;
                    if b.bomb <= 0.0 {
                        b.bomb = (2.4 - (BOSS_HP - b.hp) as f32 * 0.15).max(1.0);
                        self.shots.push(Shot {
                            pos: b.pos + vec2(0.0, BOSS_R),
                            vel: vec2(b.dir * 40.0, 60.0),
                            life: 6.0,
                            falls: true,
                        });
                        out.push(Effect::Shot(b.pos));
                    }
                    if live && (b.pos - me).length() < BOSS_R + my_half.x.max(my_half.y) {
                        if attacking {
                            if b.flash <= 0.0 {
                                b.hp -= 1;
                                b.flash = 1.2;
                                self.score += 100;
                                out.push(Effect::BossHit(b.pos));
                                // Knocked back off it.
                                let away = (me - b.pos).normalize_or(vec2(0.0, -1.0));
                                sonic.xsp = away.x * 4.0;
                                sonic.ysp = -4.0f32.min(away.y * 4.0);
                                if b.hp == 0 {
                                    b.beaten = Some(0.0);
                                    self.score += 1_000;
                                    out.push(Effect::BossBeaten(b.pos));
                                }
                            }
                        } else if b.flash <= 0.0 {
                            boss_hurts = Some(b.pos);
                        }
                    }
                }
            }
        }
        if let Some(from) = boss_hurts {
            self.hurt(sonic, from, out);
        }
        // Once the boss has fallen, the goal appears where it fought.
        if let Some(b) = &self.boss {
            if b.beaten.is_some_and(|t| t > 2.5) && self.goal.is_none() {
                let floor_y = ray(world, solid, b.home, vec2(0.0, 1.0), 2000.0).map_or(b.home.y, |d| b.home.y + d);
                self.goal = Some(vec2(b.home.x + 320.0, floor_y - 40.0));
            }
        }
        if self.boss.as_ref().is_some_and(|b| b.beaten.is_some_and(|t| t > 3.0)) {
            self.boss = None;
        }
    }
}

/// Things falling below this (world px) are gone.
fn fall_limit(world: &PhysWorld) -> f32 {
    world.arena.1 * PPM + 400.0
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WALL_T;
    use crate::physics::borders::BorderMode;

    const H: f32 = 900.0;

    fn floor_y() -> f32 {
        H - WALL_T * PPM
    }

    fn world() -> PhysWorld {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (3000.0, H));
        w.refresh_queries();
        w
    }

    fn all(_: ColliderHandle) -> bool {
        true
    }

    fn standing(x: f32) -> Sonic {
        let mut s = Sonic::new(vec2(x, floor_y() - 19.0 * SCALE));
        s.grounded = true;
        s
    }

    #[test]
    fn hits_scatter_rings_then_kill() {
        let mut w = world();
        let mut g = Game::new(&LevelSpec::default(), &mut w, vec2(100.0, 100.0));
        let mut s = standing(500.0);
        g.rings = 5;
        let mut out = vec![];
        let from = vec2(600.0, s.pos.y);
        g.hurt(&mut s, from, &mut out);
        assert_eq!((g.rings, g.lost.len()), (0, 5), "rings fly out");
        assert!(s.hurt && !s.grounded && s.xsp < 0.0, "knocked back away from the hit");
        assert_eq!(g.phase, Phase::Playing);
        // Safe for a moment, then a hit with no rings is a lost life.
        let from = vec2(600.0, s.pos.y);
        g.hurt(&mut s, from, &mut out);
        assert!(!s.dead, "still invulnerable");
        g.invulnerable = 0.0;
        let from = vec2(600.0, s.pos.y);
        g.hurt(&mut s, from, &mut out);
        assert!(s.dead);
        assert!(matches!(g.phase, Phase::Dying(_)));
        // After a while he is back where he started, one life down.
        for _ in 0..200 {
            g.update(1.0 / 60.0, &w, &all, &mut s, 1e9);
        }
        assert_eq!(g.lives, LIVES - 1);
        assert!(!s.dead && g.phase == Phase::Playing);
        assert_eq!(s.pos, vec2(100.0, 100.0));
    }

    #[test]
    fn curled_up_he_destroys_badniks_and_scores_in_a_chain() {
        let mut w2 = world();
        let spec = LevelSpec {
            enemies: vec![
                Placed { kind: EnemyKind::Crawler, at: [8.0, 2.0] },
                Placed { kind: EnemyKind::Crawler, at: [9.0, 2.0] },
            ],
            ..LevelSpec::default()
        };
        let mut g = Game::new(&spec, &mut w2, vec2(0.0, 0.0));
        let mut s = Sonic::new(g.enemies[0].pos);
        s.jumping = true;
        s.ysp = 3.0;
        let out = g.update(1.0 / 60.0, &w2, &all, &mut s, 1e9);
        assert!(out.iter().any(|e| matches!(e, Effect::EnemyDown { score: 100, .. })));
        assert_eq!(g.enemies.len(), 1);
        assert!(s.ysp < 0.0, "bounced off it");
        // Another one before landing is worth more.
        s.pos = g.enemies[0].pos;
        s.ysp = 2.0;
        g.update(1.0 / 60.0, &w2, &all, &mut s, 1e9);
        assert_eq!(g.score, 100 + 200);
        // Walking into one hurts.
        let mut g = Game::new(&spec, &mut w2, vec2(0.0, 0.0));
        g.rings = 3;
        let mut s = standing(0.0);
        s.pos = g.enemies[0].pos;
        let out = g.update(1.0 / 60.0, &w2, &all, &mut s, 1e9);
        assert!(out.iter().any(|e| matches!(e, Effect::RingsLost(_))));
        assert_eq!(g.enemies.len(), 2);
    }

    #[test]
    fn monitors_give_their_item() {
        let mut w = world();
        let spec = LevelSpec {
            monitors: vec![
                Placed { kind: MonitorKind::Rings, at: [5.0, 1.0] },
                Placed { kind: MonitorKind::Life, at: [7.0, 1.0] },
            ],
            ..LevelSpec::default()
        };
        let mut g = Game::new(&spec, &mut w, vec2(0.0, 0.0));
        assert_eq!(g.grounds().len(), 2, "monitors are solid");
        let mut s = standing(300.0);
        let c = g.monitors[0].collider.unwrap();
        assert!(g.is_monitor(c));
        g.break_monitor(&mut w, c, &mut s);
        assert_eq!(g.rings, 10);
        assert!(!g.is_monitor(c), "broken");
        let c = g.monitors[1].collider.unwrap();
        g.break_monitor(&mut w, c, &mut s);
        assert_eq!(g.lives, LIVES + 1);
        // Rings past 100 are a life too.
        let out = g.add_rings(95);
        assert!(out.contains(&Effect::ExtraLife));
        assert_eq!(g.lives, LIVES + 2);
    }

    #[test]
    fn the_goal_leads_to_the_tally() {
        let mut w = world();
        let spec = LevelSpec { goal: Some([10.0, 1.0]), ..LevelSpec::default() };
        let mut g = Game::new(&spec, &mut w, vec2(0.0, 0.0));
        g.rings = 12;
        let mut s = standing(0.0);
        s.pos = g.goal.unwrap() + vec2(20.0, 0.0);
        let out = g.update(1.0 / 60.0, &w, &all, &mut s, 1e9);
        assert!(out.iter().any(|e| matches!(e, Effect::Goal(_))));
        assert!(g.locks_input());
        for _ in 0..120 {
            g.update(1.0 / 60.0, &w, &all, &mut s, 1e9);
        }
        let Phase::Cleared(_) = g.phase else { panic!("cleared: {:?}", g.phase) };
        for _ in 0..600 {
            g.update(1.0 / 60.0, &w, &all, &mut s, 1e9);
        }
        let Phase::Cleared(t) = g.phase else { panic!() };
        assert!(t.done());
        assert_eq!(g.score, 50_000 + 1_200, "fast time bonus and 100 per ring");
        assert_eq!(time_bonus(100.0), 3_000);
    }

    #[test]
    fn falling_out_costs_a_life_and_the_boss_takes_eight_hits() {
        let mut w = world();
        let spec = LevelSpec { boss: Some([20.0, 6.0]), ..LevelSpec::default() };
        let mut g = Game::new(&spec, &mut w, vec2(0.0, 0.0));
        assert!(g.goal.is_none(), "no goal until the boss is beaten");
        let mut s = standing(0.0);
        for _ in 0..BOSS_HP {
            let b = g.boss.as_mut().unwrap();
            b.flash = 0.0;
            s.pos = b.pos;
            s.jumping = true;
            g.update(1.0 / 60.0, &w, &all, &mut s, 1e9);
        }
        assert!(g.boss.as_ref().unwrap().beaten.is_some());
        s.jumping = false;
        s.pos = vec2(-500.0, 0.0);
        for _ in 0..300 {
            g.update(1.0 / 60.0, &w, &all, &mut s, 1e9);
        }
        assert!(g.boss.is_none() && g.goal.is_some(), "the goal appears");
        let out = g.update(1.0 / 60.0, &w, &all, &mut s, -1.0);
        assert!(out.iter().any(|e| matches!(e, Effect::Died(_))), "fell out");
    }
}
