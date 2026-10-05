//! A playable Sonic, moved the way the Mega Drive games move him (after
//! the Sonic Physics Guide): a ground speed along the surface he stands
//! on, floor sensors that follow slopes, walls and ceilings (so he runs
//! round loops), rolling, the spin dash, variable jumps and the slower
//! physics underwater.
//!
//! He is not a rapier body: his sensors are ray casts into the physics
//! world, and what he does to the objects he pushes, lands on or stands
//! on comes back as [`Event`]s for the app to apply. Lengths and speeds
//! are in the guide's pixels (per 60 Hz frame), drawn [`SCALE`] times
//! larger in the world.

pub mod art;

use crate::config::PPM;
use crate::physics::{to_phys, to_screen, PhysWorld};
use macroquad::prelude::{vec2, Vec2};
use rapier2d::prelude::*;

/// World pixels per guide pixel.
pub const SCALE: f32 = 1.5;
/// Sonic's mass for pushing things (rapier units: a 60 px box weighs 1).
pub const MASS: f32 = 1.2;
/// Simulation rate of the guide (frames per second).
pub const FPS: f32 = 60.0;

/// Movement constants (guide pixels and frames).
#[derive(Clone, Copy)]
struct Consts {
    acc: f32,
    dec: f32,
    frc: f32,
    top: f32,
    air: f32,
    grv: f32,
    jmp: f32,
    /// Upward speed kept when the jump button is let go early.
    release: f32,
}

const DRY: Consts =
    Consts { acc: 0.046875, dec: 0.5, frc: 0.046875, top: 6.0, air: 0.09375, grv: 0.21875, jmp: 6.5, release: 4.0 };
const WET: Consts =
    Consts { acc: 0.0234375, dec: 0.25, frc: 0.0234375, top: 3.0, air: 0.046875, grv: 0.0625, jmp: 3.5, release: 2.0 };

const SLOPE: f32 = 0.125;
const SLOPE_ROLL_UP: f32 = 0.078125;
const SLOPE_ROLL_DOWN: f32 = 0.3125;
const ROLL_DEC: f32 = 0.125;
const MAX_SPEED: f32 = 16.0;
const MAX_FALL: f32 = 16.0;
/// Below this ground speed he slips off walls and ceilings.
const SLIP_SPEED: f32 = 2.5;
/// Frames of no steering after slipping.
const SLIP_LOCK: u32 = 30;
/// Speed needed to start a roll, and below which a roll ends.
const ROLL_START: f32 = 1.03125;
const ROLL_END: f32 = 0.5;
/// Push sensors reach this far from his centre.
const PUSH: f32 = 10.0;
/// Size: half widths and half heights standing and curled up.
const STAND: (f32, f32) = (9.0, 19.0);
const BALL: (f32, f32) = (7.0, 14.0);
/// Launch speed of a spring (a surface with bounce 1).
const SPRING: f32 = 10.0;
/// Grounded frames on ordinary ground before loop layers reset.
const LAYER_RESET: u32 = 40;

/// What one of the world's colliders is to Sonic.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Ground {
    /// 0: it does not move when pushed (walls, pinned objects).
    pub mass: f32,
    pub bounce: f32,
    pub friction: f32,
    /// Loops: 0 always solid, 1 only on the way in, 2 only on the way out,
    /// 3 always solid and switches to the way out (the top of a loop).
    pub layer: u8,
}

impl Default for Ground {
    fn default() -> Self {
        Ground { mass: 0.0, bounce: 0.0, friction: 0.5, layer: 0 }
    }
}

/// Buttons held this frame.
#[derive(Clone, Copy, Default, Debug)]
pub struct Input {
    pub left: bool,
    pub right: bool,
    pub up: bool,
    pub down: bool,
    /// Jump held, and pressed since the last frame.
    pub jump: bool,
    pub jump_pressed: bool,
}

/// Things that happened during a frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Jump,
    Roll,
    /// A spin-dash rev.
    Rev,
    /// The spin dash let go.
    Dash,
    Skid,
    Land,
    Spring,
    Splash,
    /// Push a body at `at` (world px) by `impulse` (mass × guide px per frame, y down).
    Push {
        collider: ColliderHandle,
        at: Vec2,
        impulse: Vec2,
    },
    /// Standing on a body at `at`.
    Stand {
        collider: ColliderHandle,
        at: Vec2,
    },
}

/// A sensor's find.
#[derive(Clone, Copy, Debug)]
struct Hit {
    collider: ColliderHandle,
    /// Distance from the sensor's origin (guide px).
    dist: f32,
    /// Where (world px) and the surface normal (y down).
    point: Vec2,
    normal: Vec2,
    ground: Ground,
}

#[derive(Clone, Debug)]
pub struct Sonic {
    /// Centre, in world pixels.
    pub pos: Vec2,
    pub xsp: f32,
    pub ysp: f32,
    /// Speed along the ground.
    pub gsp: f32,
    /// Ground angle (rad, anticlockwise, 0 on flat ground).
    pub angle: f32,
    pub grounded: bool,
    pub rolling: bool,
    /// In the air curled up after a jump.
    pub jumping: bool,
    pub crouching: bool,
    pub looking_up: bool,
    pub skidding: bool,
    pub pushing: bool,
    /// Revs of a spin dash being charged.
    pub spindash: Option<f32>,
    /// 1 facing right, -1 left.
    pub facing: f32,
    pub underwater: bool,
    control_lock: u32,
    /// On the way out of a loop.
    layer_out: bool,
    layer_timer: u32,
    /// What he stands on, and how fast that point moves (guide px per frame).
    pub on: Option<ColliderHandle>,
    carry: Vec2,
    /// Where he comes back after falling out of the world.
    pub spawn: Vec2,
    /// Animation clocks: running legs, and the ball's spin.
    pub stride: f32,
    pub spin: f32,
}

/// Unit vector "down" for his feet in each ground mode.
fn mode_down(angle: f32) -> Vec2 {
    let deg = angle.to_degrees().rem_euclid(360.0);
    if deg <= 45.0 || deg >= 315.0 {
        vec2(0.0, 1.0)
    } else if deg < 135.0 {
        vec2(1.0, 0.0)
    } else if deg <= 225.0 {
        vec2(0.0, -1.0)
    } else {
        vec2(-1.0, 0.0)
    }
}

/// Ground angle of a surface with this normal (world px, y down).
fn angle_of(n: Vec2) -> f32 {
    (-n.x).atan2(-n.y).rem_euclid(std::f32::consts::TAU)
}

fn degrees(angle: f32) -> f32 {
    angle.to_degrees().rem_euclid(360.0)
}

/// Is this collider solid for Sonic right now?
type Env<'a> = &'a dyn Fn(ColliderHandle) -> Option<Ground>;

impl Sonic {
    pub fn new(at: Vec2) -> Self {
        Sonic {
            pos: at,
            xsp: 0.0,
            ysp: 0.0,
            gsp: 0.0,
            angle: 0.0,
            grounded: false,
            rolling: false,
            jumping: false,
            crouching: false,
            looking_up: false,
            skidding: false,
            pushing: false,
            spindash: None,
            facing: 1.0,
            underwater: false,
            control_lock: 0,
            layer_out: false,
            layer_timer: 0,
            on: None,
            carry: Vec2::ZERO,
            spawn: at,
            stride: 0.0,
            spin: 0.0,
        }
    }

    /// Back to where he started, standing still.
    pub fn respawn(&mut self) {
        *self = Sonic { facing: self.facing, ..Sonic::new(self.spawn) };
    }

    /// Curled up (smaller sensors).
    pub fn is_ball(&self) -> bool {
        self.rolling || self.jumping || self.spindash.is_some()
    }

    /// Half width and half height (guide px).
    pub fn radii(&self) -> (f32, f32) {
        if self.is_ball() {
            BALL
        } else {
            STAND
        }
    }

    /// Speed in world pixels per second.
    pub fn velocity_px(&self) -> Vec2 {
        vec2(self.xsp, self.ysp) * SCALE * FPS
    }

    /// Thrown by the pointer (or anything else): airborne at this velocity
    /// (world px per second).
    pub fn throw(&mut self, at: Vec2, velocity: Vec2) {
        self.pos = at;
        let v = velocity / (SCALE * FPS);
        (self.xsp, self.ysp) = (v.x.clamp(-MAX_SPEED, MAX_SPEED), v.y.clamp(-MAX_SPEED, MAX_SPEED));
        self.grounded = false;
        self.jumping = false;
        self.rolling = false;
        self.spindash = None;
        self.on = None;
    }

    /// Curl up (or stand up) on the ground, keeping his feet where they are.
    fn curl(&mut self, ball: bool) {
        let before = self.radii().1;
        let after = if ball { BALL.1 } else { STAND.1 };
        if self.grounded {
            self.pos += mode_down(self.angle) * (before - after) * SCALE;
        }
    }

    fn solid(&self, g: &Ground) -> bool {
        match g.layer {
            1 => !self.layer_out,
            2 => self.layer_out,
            _ => true,
        }
    }

    /// Cast a sensor from `from` (world px) along `dir` for `len` guide px.
    fn cast(&self, world: &PhysWorld, env: Env, from: Vec2, dir: Vec2, len: f32) -> Option<Hit> {
        let solid = |c: ColliderHandle| env(c).filter(|g| self.solid(g));
        let pred = |h: ColliderHandle, _: &Collider| solid(h).is_some();
        let filter = QueryFilter::default().exclude_sensors().predicate(&pred);
        let (x, y) = to_phys(from.x, from.y);
        let (c, toi, n) = world.cast_ray(Point::new(x, y), vector![dir.x, -dir.y], len * SCALE / PPM, filter)?;
        let p = Point::new(x, y) + vector![dir.x, -dir.y] * toi;
        Some(Hit {
            collider: c,
            dist: toi * PPM / SCALE,
            point: to_screen(p.x, p.y),
            normal: vec2(n.x, -n.y).normalize_or(-dir),
            ground: solid(c).unwrap_or_default(),
        })
    }

    /// The nearer of the two floor sensors (A and B) along `down`, reaching
    /// `extra` guide px past his feet; its distance is measured from his feet.
    fn floor(&self, world: &PhysWorld, env: Env, down: Vec2, extra: f32) -> Option<Hit> {
        let (wr, hr) = self.radii();
        let side = vec2(-down.y, down.x);
        [-1.0, 1.0]
            .into_iter()
            .filter_map(|k| self.cast(world, env, self.pos + side * k * wr * SCALE, down, hr + extra))
            .map(|h| Hit { dist: h.dist - hr, ..h })
            .filter(|h| h.dist >= -14.0)
            .min_by(|a, b| a.dist.total_cmp(&b.dist))
    }

    /// One 60 Hz frame. `water` is the water surface (world px), `gravity`
    /// the world's gravity relative to Earth's.
    pub fn frame(&mut self, world: &PhysWorld, env: Env, input: Input, water: Option<f32>, gravity: f32) -> Vec<Event> {
        let mut events = vec![];
        let under = water.is_some_and(|w| self.pos.y > w);
        if under != self.underwater {
            self.underwater = under;
            if under {
                self.xsp *= 0.5;
                self.gsp *= 0.5;
                self.ysp *= 0.25;
            } else if !self.grounded && self.ysp < 0.0 {
                self.ysp = (self.ysp * 2.0).max(-MAX_SPEED);
            }
            events.push(Event::Splash);
        }
        let c = if self.underwater { WET } else { DRY };
        let grv = c.grv * gravity.max(0.0);
        if self.grounded {
            self.ground_frame(world, env, input, c, &mut events);
        } else {
            self.air_frame(world, env, input, c, grv, &mut events);
        }
        self.stride += self.gsp.abs().max(0.6) * 0.07;
        self.spin += if self.grounded { self.gsp } else { self.xsp.abs().max(4.0) * self.facing } * 0.12;
        events
    }

    fn ground_frame(&mut self, world: &PhysWorld, env: Env, input: Input, c: Consts, events: &mut Vec<Event>) {
        // Carried by what he stands on.
        self.pos += self.carry * SCALE;
        let ground = self.on.and_then(env).unwrap_or_default();
        let grip = (ground.friction / 0.5).clamp(0.12, 1.0);
        let (sin, cos) = self.angle.sin_cos();

        // Spin dash: rev with jump while crouching, let go of ↓ to dash.
        if let Some(rev) = self.spindash {
            if !input.down {
                self.spindash = None;
                self.rolling = true;
                self.gsp = (8.0 + rev.floor() / 2.0) * self.facing;
                events.push(Event::Dash);
            } else {
                let mut rev = rev;
                if input.jump_pressed {
                    rev = (rev + 2.0).min(8.0);
                    events.push(Event::Rev);
                }
                rev -= (rev / 0.125).floor() / 256.0;
                self.spindash = Some(rev);
                self.gsp = 0.0;
                self.stick(world, env, events);
                return;
            }
        }
        if input.jump_pressed {
            if self.crouching && self.gsp.abs() < 0.5 {
                self.curl(true);
                self.spindash = Some(2.0);
                events.push(Event::Rev);
                return;
            }
            // Jump straight out from the surface.
            self.xsp -= c.jmp * sin;
            self.ysp -= c.jmp * cos;
            self.grounded = false;
            self.jumping = true;
            self.rolling = false;
            self.crouching = false;
            self.on = None;
            self.carry = Vec2::ZERO;
            events.push(Event::Jump);
            return;
        }

        // Slope factor.
        let slope = if !self.rolling {
            SLOPE
        } else if self.gsp.signum() == sin.signum() {
            SLOPE_ROLL_UP
        } else {
            SLOPE_ROLL_DOWN
        };
        if self.gsp != 0.0 || sin.abs() > 0.35 {
            self.gsp -= slope * sin;
        }

        let locked = self.control_lock > 0;
        self.control_lock = self.control_lock.saturating_sub(1);
        let (left, right) = if locked { (false, false) } else { (input.left, input.right) };
        let was_skidding = self.skidding;
        self.skidding = false;
        if self.rolling {
            if left && self.gsp > 0.0 {
                self.gsp -= ROLL_DEC * grip;
            }
            if right && self.gsp < 0.0 {
                self.gsp += ROLL_DEC * grip;
            }
            self.gsp -= self.gsp.abs().min(c.frc * 0.5 * grip) * self.gsp.signum();
            if self.gsp.abs() < ROLL_END {
                self.curl(false);
                self.rolling = false;
            }
        } else {
            let flat = mode_down(self.angle).y > 0.5;
            self.crouching = input.down && self.gsp.abs() < ROLL_START && flat;
            self.looking_up = input.up && self.gsp == 0.0 && !self.crouching && flat;
            if self.crouching {
                self.gsp -= self.gsp.abs().min(c.frc * grip) * self.gsp.signum();
            } else if left || right {
                let d = if right { 1.0 } else { -1.0 };
                if self.gsp * d < 0.0 {
                    // Braking (skidding at speed).
                    self.skidding = self.gsp.abs() >= 4.0 && flat;
                    self.gsp += c.dec * grip * d;
                    if self.gsp * d > 0.0 {
                        self.gsp = 0.5 * d;
                    }
                } else if self.gsp.abs() < c.top {
                    self.gsp = (self.gsp + c.acc * grip.sqrt() * d).clamp(-c.top, c.top);
                }
                if self.gsp * d >= 0.0 {
                    self.facing = d;
                }
            } else {
                self.gsp -= self.gsp.abs().min(c.frc * grip) * self.gsp.signum();
            }
            if input.down && self.gsp.abs() >= ROLL_START && !locked {
                self.curl(true);
                self.rolling = true;
                events.push(Event::Roll);
            }
        }
        if self.skidding && !was_skidding {
            events.push(Event::Skid);
        }
        self.gsp = self.gsp.clamp(-MAX_SPEED, MAX_SPEED);
        self.xsp = self.gsp * cos;
        self.ysp = -self.gsp * sin;

        // Move in steps short enough not to pass through thin things.
        let step = vec2(self.xsp, self.ysp);
        let n = (step.abs().max_element() / 6.0).ceil().max(1.0) as usize;
        self.pushing = false;
        for _ in 0..n {
            self.pos += step * SCALE / n as f32;
            self.push_walls(world, env, input, events);
            if !self.stick(world, env, events) || !self.grounded {
                return;
            }
        }

        // Too slow on a wall or a ceiling: slip off.
        let deg = degrees(self.angle);
        if self.control_lock == 0 && self.gsp.abs() < SLIP_SPEED && (46.0..=315.0).contains(&deg) {
            self.control_lock = SLIP_LOCK;
            if (91.0..=269.0).contains(&deg) {
                self.leave_ground();
            }
        }
    }

    /// Follow the floor; false (and airborne) when there is none.
    fn stick(&mut self, world: &PhysWorld, env: Env, events: &mut Vec<Event>) -> bool {
        let down = mode_down(self.angle);
        let Some(hit) = self.floor(world, env, down, 16.0) else {
            self.leave_ground();
            return false;
        };
        let tolerance = (self.xsp.abs().max(self.ysp.abs()) + 4.0).min(14.0);
        if hit.dist > tolerance {
            self.leave_ground();
            return false;
        }
        self.pos += down * hit.dist * SCALE;
        self.angle = angle_of(hit.normal);
        // Springs launch him off whatever way they face.
        if hit.ground.bounce >= 0.6 {
            self.spring(hit, events);
            return false;
        }
        self.land_on(world, hit, events);
        true
    }

    fn leave_ground(&mut self) {
        self.grounded = false;
        self.on = None;
        self.crouching = false;
        self.looking_up = false;
        self.skidding = false;
        // Momentum carried from the platform.
        self.xsp += self.carry.x;
        self.ysp += self.carry.y;
        self.carry = Vec2::ZERO;
    }

    /// Note what he stands on (layers, riding, weight).
    fn land_on(&mut self, world: &PhysWorld, hit: Hit, events: &mut Vec<Event>) {
        self.on = Some(hit.collider);
        match hit.ground.layer {
            3 => {
                self.layer_out = true;
                self.layer_timer = 0;
            }
            0 => {
                self.layer_timer += 1;
                if self.layer_timer > LAYER_RESET {
                    self.layer_out = false;
                }
            }
            _ => self.layer_timer = 0,
        }
        self.carry = Vec2::ZERO;
        if hit.ground.mass > 0.0 {
            events.push(Event::Stand { collider: hit.collider, at: hit.point });
        }
        if let Some(b) = world.colliders.get(hit.collider).and_then(|c| c.parent()).and_then(|b| world.bodies.get(b)) {
            if !b.is_fixed() {
                let (x, y) = to_phys(hit.point.x, hit.point.y);
                let v = b.velocity_at_point(&Point::new(x, y));
                self.carry = vec2(v.x, -v.y) * PPM / (SCALE * FPS);
            }
        }
    }

    fn spring(&mut self, hit: Hit, events: &mut Vec<Event>) {
        // A spring lying flat (or its rounded end) launches straight up.
        let hit = if hit.normal.y < -0.5 { Hit { normal: vec2(0.0, -1.0), ..hit } } else { hit };
        let v = vec2(self.xsp, self.ysp);
        let into = (-v.dot(hit.normal)).max(0.0);
        let out = (into * hit.ground.bounce).max(SPRING * hit.ground.bounce.min(1.6));
        let v = v + hit.normal * (into + out);
        (self.xsp, self.ysp) = (v.x, v.y);
        self.grounded = false;
        self.jumping = false;
        self.rolling = false;
        self.on = None;
        self.carry = Vec2::ZERO;
        events.push(Event::Spring);
    }

    /// Momentum shared with a body he runs into: returns his speed along
    /// `dir` afterwards.
    fn shove(&self, world: &PhysWorld, hit: &Hit, dir: Vec2, speed: f32, events: &mut Vec<Event>) -> f32 {
        let m = hit.ground.mass;
        if m <= 0.0 || speed <= 0.0 {
            return 0.0;
        }
        let (x, y) = to_phys(hit.point.x, hit.point.y);
        let vb = world
            .colliders
            .get(hit.collider)
            .and_then(|c| c.parent())
            .and_then(|b| world.bodies.get(b))
            .map(|b| b.velocity_at_point(&Point::new(x, y)))
            .map_or(0.0, |v| vec2(v.x, -v.y).dot(dir) * PPM / (SCALE * FPS));
        if vb >= speed {
            return speed;
        }
        let shared = (MASS * speed + m * vb) / (MASS + m);
        // Curled up he bowls things over.
        let target = if self.is_ball() { 2.0 * shared - vb } else { shared };
        events.push(Event::Push { collider: hit.collider, at: hit.point, impulse: dir * m * (target - vb) });
        shared
    }

    /// Push sensors on the ground: stop at walls, shove what moves.
    fn push_walls(&mut self, world: &PhysWorld, env: Env, input: Input, events: &mut Vec<Event>) {
        if self.gsp == 0.0 {
            return;
        }
        let (sin, cos) = self.angle.sin_cos();
        let dir = vec2(cos, -sin) * self.gsp.signum();
        let down = vec2(sin, cos);
        // Low on flat ground, so small steps count as walls.
        let low = if mode_down(self.angle).y > 0.5 && !self.is_ball() { 8.0 } else { 0.0 };
        let from = self.pos + down * low * SCALE;
        let Some(hit) = self.cast(world, env, from, dir, PUSH) else { return };
        // Only things in the way, not slopes he can run up.
        if hit.normal.dot(dir) > -0.6 {
            return;
        }
        self.pos -= dir * (PUSH - hit.dist) * SCALE;
        let speed = self.shove(world, &hit, dir, self.gsp.abs(), events);
        self.gsp = speed * self.gsp.signum();
        self.pushing = (input.right && dir.x > 0.0) || (input.left && dir.x < 0.0);
    }

    fn air_frame(&mut self, world: &PhysWorld, env: Env, input: Input, c: Consts, grv: f32, events: &mut Vec<Event>) {
        self.crouching = false;
        self.looking_up = false;
        self.skidding = false;
        self.pushing = false;
        if self.jumping && !input.jump && self.ysp < -c.release {
            self.ysp = -c.release;
        }
        if input.right {
            if self.xsp < c.top {
                self.xsp = (self.xsp + c.air).min(c.top);
            }
            self.facing = 1.0;
        } else if input.left {
            if self.xsp > -c.top {
                self.xsp = (self.xsp - c.air).max(-c.top);
            }
            self.facing = -1.0;
        }
        // Air drag near the top of a jump.
        if self.ysp < 0.0 && self.ysp > -4.0 {
            self.xsp -= (self.xsp / 0.125).trunc() / 256.0;
        }
        let step = vec2(self.xsp, self.ysp);
        let n = (step.abs().max_element() / 6.0).ceil().max(1.0) as usize;
        for _ in 0..n {
            self.pos += step * SCALE / n as f32;
            if self.collide_air(world, env, events) {
                return;
            }
        }
        self.ysp = (self.ysp + grv).min(MAX_FALL);
        // Turn back upright.
        let a = (self.angle + std::f32::consts::PI).rem_euclid(std::f32::consts::TAU) - std::f32::consts::PI;
        self.angle = (a * 0.85).rem_euclid(std::f32::consts::TAU);
    }

    /// Walls, ceiling and landing. True when he landed.
    fn collide_air(&mut self, world: &PhysWorld, env: Env, events: &mut Vec<Event>) -> bool {
        for d in [-1.0f32, 1.0] {
            let dir = vec2(d, 0.0);
            if let Some(hit) = self.cast(world, env, self.pos, dir, PUSH) {
                self.pos.x -= d * (PUSH - hit.dist) * SCALE;
                if self.xsp * d > 0.0 {
                    let speed = self.shove(world, &hit, dir, self.xsp.abs(), events);
                    self.xsp = speed * d;
                }
            }
        }
        if self.ysp < 0.0 {
            let up = vec2(0.0, -1.0);
            if let Some(hit) = self.floor(world, env, up, 0.0).filter(|h| h.dist < 0.0) {
                self.pos.y -= hit.dist * SCALE;
                let deg = degrees(angle_of(hit.normal));
                // Steep ceilings catch him running.
                if (91.0..=135.0).contains(&deg) || (225.0..=269.0).contains(&deg) {
                    self.angle = angle_of(hit.normal);
                    self.gsp = self.ysp * -self.angle.sin().signum();
                    self.grounded = true;
                    self.jumping = false;
                    self.land_on(world, hit, events);
                    return true;
                }
                self.ysp = 0.0;
            }
            return false;
        }
        let down = vec2(0.0, 1.0);
        let Some(hit) = self.floor(world, env, down, self.ysp + 2.0) else { return false };
        if hit.dist > 0.0 || hit.dist < -(self.ysp + 8.0) {
            return false;
        }
        self.pos.y += hit.dist * SCALE;
        self.angle = angle_of(hit.normal);
        if hit.ground.bounce >= 0.6 {
            self.spring(hit, events);
            return true;
        }
        // Ground speed from the landing, as in the guide.
        let deg = degrees(self.angle);
        let s = -self.angle.sin().signum();
        self.gsp = if deg <= 23.0 || deg >= 339.0 || self.xsp.abs() > self.ysp {
            self.xsp
        } else if deg <= 45.0 || deg >= 316.0 {
            self.ysp * 0.5 * s
        } else {
            self.ysp * s
        };
        if hit.ground.mass > 0.0 {
            events.push(Event::Push { collider: hit.collider, at: hit.point, impulse: down * MASS * self.ysp });
        }
        self.grounded = true;
        self.jumping = false;
        self.rolling = false;
        self.control_lock = 0;
        self.land_on(world, hit, events);
        events.push(Event::Land);
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WALL_T;
    use crate::physics::borders::BorderMode;

    const W: f32 = 3000.0;
    const H: f32 = 900.0;

    fn floor_y() -> f32 {
        H - WALL_T * PPM
    }

    fn world() -> PhysWorld {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (W, H));
        w.refresh_queries();
        w
    }

    fn walls(_: ColliderHandle) -> Option<Ground> {
        Some(Ground::default())
    }

    fn standing(x: f32) -> Sonic {
        let mut s = Sonic::new(vec2(x, floor_y() - 19.0 * SCALE));
        s.grounded = true;
        s
    }

    fn run(s: &mut Sonic, w: &PhysWorld, env: Env, input: Input, frames: usize) -> Vec<Event> {
        (0..frames).flat_map(|_| s.frame(w, env, input, None, 1.0)).collect()
    }

    #[test]
    fn runs_up_to_top_speed_and_stops() {
        let w = world();
        let mut s = standing(300.0);
        run(&mut s, &w, &walls, Input { right: true, ..Input::default() }, 140);
        assert!(s.grounded);
        assert!((s.gsp - 6.0).abs() < 1e-3, "top speed 6, got {}", s.gsp);
        assert!((s.pos.y - (floor_y() - 19.0 * SCALE)).abs() < 1.0, "feet on the floor");
        // 6 / 0.046875 = 128 frames to top speed, then it coasts to a stop.
        run(&mut s, &w, &walls, Input::default(), 140);
        assert_eq!(s.gsp, 0.0);
    }

    #[test]
    fn jumps_as_high_as_the_guide_says() {
        let w = world();
        let mut s = standing(300.0);
        let start = s.pos.y;
        let mut top = start;
        let mut input = Input { jump: true, jump_pressed: true, ..Input::default() };
        for _ in 0..120 {
            let ev = s.frame(&w, &walls, input, None, 1.0);
            input.jump_pressed = false;
            top = top.min(s.pos.y);
            if ev.contains(&Event::Land) {
                break;
            }
        }
        // 6.5² / (2 × 0.21875) ≈ 96.6 guide px.
        let rise = (start - top) / SCALE;
        assert!((rise - 96.6).abs() < 8.0, "rose {rise} px");
        assert!(s.grounded, "landed again");
        // A short hop when jump is let go at once.
        let mut s = standing(300.0);
        let mut top = s.pos.y;
        s.frame(&w, &walls, Input { jump: true, jump_pressed: true, ..Input::default() }, None, 1.0);
        for _ in 0..80 {
            s.frame(&w, &walls, Input::default(), None, 1.0);
            top = top.min(s.pos.y);
        }
        assert!((start - top) / SCALE < 45.0, "short hop");
    }

    #[test]
    fn spin_dash_launches_him_rolling() {
        let w = world();
        let mut s = standing(300.0);
        let down = Input { down: true, ..Input::default() };
        run(&mut s, &w, &walls, down, 2);
        assert!(s.crouching);
        for _ in 0..3 {
            s.frame(&w, &walls, Input { jump: true, jump_pressed: true, ..down }, None, 1.0);
            s.frame(&w, &walls, down, None, 1.0);
        }
        assert!(s.spindash.is_some());
        let ev = s.frame(&w, &walls, Input::default(), None, 1.0);
        assert!(ev.contains(&Event::Dash));
        assert!(s.rolling && s.gsp >= 10.0, "dashing at {}", s.gsp);
    }

    #[test]
    fn slower_underwater() {
        let w = world();
        let mut s = standing(300.0);
        let right = Input { right: true, ..Input::default() };
        for _ in 0..200 {
            s.frame(&w, &walls, right, Some(0.0), 1.0);
        }
        assert!(s.underwater);
        assert!((s.gsp - 3.0).abs() < 1e-3, "top speed 3 underwater, got {}", s.gsp);
    }

    /// A loop of capsules: the way in, the top and the way out on their
    /// own colliders (layers 1, 3 and 2).
    #[test]
    fn runs_round_a_loop() {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (W, H));
        let ground = w.bodies.insert(RigidBodyBuilder::fixed());
        let r = 150.0;
        let centre = vec2(1200.0, floor_y() - r + 8.0);
        let mut layer = std::collections::HashMap::new();
        // Screen angles: 90° is the bottom, 0° the right, -90° the top.
        for (from, to, l) in [(90.0f32, -60.0f32, 1u8), (-60.0, -120.0, 3), (-120.0, -270.0, 2)] {
            let n = 24;
            let pts: Vec<Vec2> = (0..=n)
                .map(|k| {
                    let a = (from + (to - from) * k as f32 / n as f32).to_radians();
                    centre + vec2(a.cos(), a.sin()) * (r + 8.0)
                })
                .collect();
            for p in pts.windows(2) {
                let (a, b) = (to_phys(p[0].x, p[0].y), to_phys(p[1].x, p[1].y));
                let col = ColliderBuilder::capsule_from_endpoints(point![a.0, a.1], point![b.0, b.1], 8.0 / PPM);
                let h = w.colliders.insert_with_parent(col, ground, &mut w.bodies);
                layer.insert(h, l);
            }
        }
        w.refresh_queries();
        let env = |c: ColliderHandle| Some(Ground { layer: layer.get(&c).copied().unwrap_or(0), ..Ground::default() });
        let mut s = standing(600.0);
        s.gsp = 12.0;
        let right = Input { right: true, ..Input::default() };
        let mut top = s.pos.y;
        let mut upside_down = false;
        for _ in 0..400 {
            s.frame(&w, &env, right, None, 1.0);
            top = top.min(s.pos.y);
            upside_down |= s.grounded && (135.0..=225.0).contains(&degrees(s.angle));
            if s.pos.x > centre.x + r * 2.0 {
                break;
            }
        }
        assert!(upside_down, "ran along the ceiling of the loop");
        assert!(top < centre.y - r * 0.5, "went over the top: {top}");
        assert!(s.pos.x > centre.x + r * 2.0 && s.gsp > 0.0, "came out the other side: {:?}", s.pos);
        assert!(s.grounded);
    }

    #[test]
    fn modes_and_angles() {
        assert_eq!(mode_down(0.0), vec2(0.0, 1.0));
        assert_eq!(mode_down(90f32.to_radians()), vec2(1.0, 0.0));
        assert_eq!(mode_down(180f32.to_radians()), vec2(0.0, -1.0));
        assert_eq!(mode_down(270f32.to_radians()), vec2(-1.0, 0.0));
        assert!(angle_of(vec2(0.0, -1.0)).abs() < 1e-6, "flat");
        let up_right = angle_of(vec2(-1.0, -1.0).normalize());
        assert!((up_right.to_degrees() - 45.0).abs() < 1e-3, "a slope rising to the right is 45°");
    }
}
