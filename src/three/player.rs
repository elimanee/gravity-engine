//! The character in 3D, moving the way the 2D game does (after the Sonic
//! Physics Guide: acceleration, braking, friction, slopes, a jump straight
//! out from the ground, rolling, the spin dash) with its momentum rules
//! (holding the way you go keeps the speed over jumps and rolls), carried
//! over to a world of surfaces: the stick sets a direction relative to the
//! camera, the character runs along whatever is under its feet (round
//! loops and up walls when fast enough) and turns smoothly, so speed is
//! kept through bends.
//!
//! Speeds are the guide's (pixels per 60 Hz frame); one guide pixel is
//! [`K`] metres.

use super::phys::{Hit, Phys};
use crate::sonic::kit::{Ability, Button, Kit};
use crate::sonic::Character;
use macroquad::prelude::{vec3, Quat, Vec3};
use rapier3d::prelude::RigidBodyHandle;

/// Metres per guide pixel.
pub const K: f32 = 0.04;
pub const ACC: f32 = 0.046875;
pub const DEC: f32 = 0.5;
pub const FRC: f32 = 0.046875;
pub const TOP: f32 = 6.0;
pub const AIR: f32 = 0.09375;
pub const GRV: f32 = 0.21875;
pub const JMP: f32 = 6.5;
const RELEASE: f32 = 4.0;
const SLOPE: f32 = 0.125;
const SLOPE_ROLL_UP: f32 = 0.078125;
const SLOPE_ROLL_DOWN: f32 = 0.3125;
const ROLL_DEC: f32 = 0.125;
pub const MAX_SPEED: f32 = 16.0;
const MAX_FALL: f32 = 16.0;
const SLIP_SPEED: f32 = 2.5;
const SLIP_LOCK: u32 = 30;
const ROLL_START: f32 = 1.03125;
const ROLL_END: f32 = 0.5;
/// The centre's height over the feet, standing and curled up (metres).
pub const HALF: f32 = 19.0 * K;
pub const BALL_HALF: f32 = 14.0 * K;
/// How far ahead walls stop him (metres from the centre).
const REACH: f32 = 0.45;
/// Ground sensors start this far behind the centre (metres).
const LIFT: f32 = 0.3;
/// Turning: radians a frame, slow and fast.
const TURN_SLOW: f32 = 0.2;
const TURN_FAST: f32 = 0.07;
/// Tails' flight (its time is the kit's), Knuckles' glide.
const FLY_GRV: f32 = 0.03125;
const FLAP: f32 = 1.6;
const GLIDE_START: f32 = 4.0;
const GLIDE_TOP: f32 = 9.0;
const GLIDE_FALL: f32 = 0.5;
/// Amy's hammer swing (frames), Sonic.exe's teleport (metres).
const HAMMER: u32 = 22;
#[cfg(test)]
const WARP: f32 = 5.0;
/// A dash through the air (frames), a homing attack's speed (guide px a
/// frame) and how long it keeps after its target, how long a second jump
/// and a throw show.
const DASH_TIME: u32 = 14;
const HOMING_SPEED: f32 = 12.0;
const HOMING_TIME: u32 = 50;
const DOUBLE_SHOW: u32 = 16;
const THROW_SHOW: u32 = 12;

/// What the player asks for this frame. `dir` is where to go, in the
/// world (flat, at most 1 long: a stick pushed half way is half).
#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    pub dir: Vec3,
    pub jump: bool,
    pub jump_pressed: bool,
    pub action: bool,
    pub action_pressed: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Event {
    Jump,
    Land,
    Roll,
    Rev,
    Dash,
    Skid,
    Flap,
    Glide,
    Hammer,
    Teleport,
    DoubleJump,
    AirDash,
    Stomp,
    Stomped,
    /// Something thrown from `from` along `dir` (flat, normalised) at
    /// `speed` (guide px a frame).
    Shoot {
        from: Vec3,
        dir: Vec3,
        speed: f32,
    },
    /// He ran into something that moves: push it (kg·m/s, at a point).
    Push {
        body: RigidBodyHandle,
        impulse: Vec3,
        at: Vec3,
    },
}

/// Anything rays can be cast in (the world, or a test's).
pub trait Sense {
    fn ray(&self, from: Vec3, dir: Vec3, max: f32) -> Option<Hit>;
}

impl Sense for Phys {
    fn ray(&self, from: Vec3, dir: Vec3, max: f32) -> Option<Hit> {
        Phys::ray(self, from, dir, max)
    }
}

#[derive(Clone, Debug)]
pub struct Runner {
    pub character: Character,
    /// The centre (metres).
    pub pos: Vec3,
    /// Up from the ground under him (in the air, turning back to the sky's).
    pub up: Vec3,
    /// Velocity (guide px a frame): along the ground when on it.
    pub vel: Vec3,
    /// Which way he faces (along the ground).
    pub facing: Vec3,
    pub grounded: bool,
    pub jumping: bool,
    pub rolling: bool,
    pub skidding: bool,
    pub spindash: Option<f32>,
    pub hurt: bool,
    pub control_lock: u32,
    pub flying: Option<u32>,
    pub gliding: Option<f32>,
    pub hammer: Option<u32>,
    pub warped: bool,
    /// Frames since he stood still (for idling).
    pub idle: u32,
    /// The axis the ground has tilted him about (kept when upside down).
    tilt: Vec3,
    /// How he moves and his moves.
    pub kit: Kit,
    /// The moves used this jump (one bit each, by their place in the kit).
    used: u8,
    fly_time: u32,
    /// Frames into a second jump, a dash through the air, a throw.
    pub doubled: Option<u32>,
    pub dashing: Option<u32>,
    pub throwing: Option<u32>,
    /// Homing in on this (metres), stomping down at this speed.
    pub homing: Option<Vec3>,
    pub stomping: Option<f32>,
    /// What a homing attack can go for (badniks), told him by the world.
    pub targets: Vec<Vec3>,
}

/// `v` along the plane whose normal is `n`.
fn flat(v: Vec3, n: Vec3) -> Vec3 {
    v - n * v.dot(n)
}

/// Turn the direction `from` towards `to` by at most `rate` radians.
fn turn(from: Vec3, to: Vec3, rate: f32) -> Vec3 {
    let a = from.dot(to).clamp(-1.0, 1.0).acos();
    if a <= rate || a < 1e-4 {
        return to;
    }
    let t = rate / a;
    (from * (1.0 - t) + to * t).normalize_or(from)
}

impl Runner {
    pub fn new(character: Character, feet: Vec3, facing: Vec3) -> Runner {
        Runner {
            character,
            pos: feet + Vec3::Y * HALF,
            up: Vec3::Y,
            vel: Vec3::ZERO,
            facing: facing.normalize_or(Vec3::X),
            grounded: true,
            jumping: false,
            rolling: false,
            skidding: false,
            spindash: None,
            hurt: false,
            control_lock: 0,
            flying: None,
            gliding: None,
            hammer: None,
            warped: false,
            idle: 0,
            tilt: Vec3::Z,
            kit: character.kit(),
            used: 0,
            fly_time: 480,
            doubled: None,
            dashing: None,
            throwing: None,
            homing: None,
            stomping: None,
            targets: vec![],
        }
    }

    pub fn is_ball(&self) -> bool {
        self.rolling || self.spindash.is_some() || (self.jumping && self.flying.is_none() && self.gliding.is_none())
    }

    /// Curled up, gliding or swinging the hammer: badniks break.
    pub fn attacking(&self) -> bool {
        self.is_ball() || self.gliding.is_some() || self.hammer.is_some_and(|t| (4..16).contains(&t))
    }

    pub fn half(&self) -> f32 {
        if self.is_ball() {
            BALL_HALF
        } else {
            HALF
        }
    }

    pub fn speed(&self) -> f32 {
        self.vel.length()
    }

    /// Where his feet are.
    #[cfg(test)]
    pub fn feet(&self) -> Vec3 {
        self.pos - self.up * self.half()
    }

    /// Launched (by a spring, a hit): `vel` in guide px a frame.
    pub fn launch(&mut self, vel: Vec3) {
        self.vel = vel;
        self.grounded = false;
        self.jumping = false;
        self.rolling = false;
        self.spindash = None;
        self.flying = None;
        self.gliding = None;
        self.dashing = None;
        self.homing = None;
        self.stomping = None;
    }

    /// Bounced off a badnik he broke in the air: out of a homing attack,
    /// up off it with every move ready again.
    pub fn rebound(&mut self) {
        if self.homing.take().is_some() || self.dashing.take().is_some() {
            self.vel = flat(self.vel, Vec3::Y) * 0.25 + Vec3::Y * 5.5;
            self.jumping = true;
            self.used = 0;
        } else {
            self.vel.y = self.vel.y.abs().max(4.0);
        }
    }

    /// Hurt: thrown up and back from `from`, no control until he lands.
    pub fn knock_back(&mut self, from: Vec3) {
        let away = flat(self.pos - from, Vec3::Y).normalize_or(-self.facing);
        self.launch(away * 2.0 + Vec3::Y * 4.0);
        self.hurt = true;
    }

    pub fn frame(&mut self, world: &impl Sense, c: Controls) -> Vec<Event> {
        let mut events = vec![];
        let c = if self.hurt { Controls::default() } else { c };
        if let Some(t) = self.hammer {
            self.hammer = (t + 1 < HAMMER).then_some(t + 1);
        }
        if self.grounded {
            self.ground_frame(world, c, &mut events);
        } else {
            self.air_frame(world, c, &mut events);
        }
        self.idle = if self.grounded && self.speed() < 0.01 { self.idle + 1 } else { 0 };
        events
    }

    /// The stick's direction turned with the ground under him (as the sky's
    /// up turns into the ground's), so "onward" stays onward up a wall and
    /// round a loop instead of turning into braking.
    fn on_ground(&mut self, dir: Vec3) -> Vec3 {
        let n = self.up;
        let axis = Vec3::Y.cross(n);
        if axis.length() > 0.05 {
            self.tilt = axis.normalize();
        }
        let q = Quat::from_axis_angle(self.tilt, Vec3::Y.dot(n).clamp(-1.0, 1.0).acos());
        flat(q * dir, n)
    }

    fn ground_frame(&mut self, world: &impl Sense, c: Controls, events: &mut Vec<Event>) {
        let n = self.up;
        let d = self.on_ground(c.dir);
        let push = c.dir.length().min(1.0);
        let d = if push > 0.05 { d.normalize_or_zero() } else { Vec3::ZERO };
        let mut speed = self.vel.length();
        let mut dir = if speed > 1e-4 { self.vel / speed } else { flat(self.facing, n).normalize_or(self.facing) };

        let p = self.kit.physics;
        let (acc, top, jmp) = (ACC * p.accel, TOP * p.speed, JMP * p.jump);
        // The action button: a move of theirs on the ground if they have
        // one (Amy's hammer…), else rolling and the spin dash.
        let ground_move =
            self.kit.blocks.iter().map(|b| (b.ability, b.on)).find(|(a, on)| *on == Button::Action && a.on_ground());
        // The spin dash: hold the action button standing, jump to rev, let go.
        let can_dash = self.kit.spin_dash && ground_move.is_none();
        if let Some(rev) = self.spindash {
            if !c.action {
                self.spindash = None;
                self.rolling = true;
                speed = 8.0 + rev.floor() / 2.0;
                dir = flat(self.facing, n).normalize_or(dir);
                events.push(Event::Dash);
            } else {
                let mut rev = rev;
                if c.jump_pressed {
                    rev = (rev + 2.0).min(8.0);
                    events.push(Event::Rev);
                }
                rev -= (rev / 0.125).floor() / 256.0;
                self.spindash = Some(rev);
                if d != Vec3::ZERO {
                    self.facing = d;
                }
                self.vel = Vec3::ZERO;
                self.stick(world, events);
                return;
            }
        } else if can_dash && c.action_pressed && speed < 0.5 {
            self.spindash = Some(2.0);
            events.push(Event::Rev);
            self.vel = Vec3::ZERO;
            return;
        }
        if c.jump_pressed {
            self.vel = dir * speed + n * jmp;
            self.used = 0;
            self.leave_ground();
            self.jumping = true;
            self.rolling = false;
            events.push(Event::Jump);
            return;
        }
        // The action button's moves that work on the ground: the first one.
        if c.action_pressed && self.hammer.is_none() {
            match ground_move {
                Some((Ability::Hammer, _)) => {
                    self.hammer = Some(0);
                    events.push(Event::Hammer);
                }
                Some((Ability::Shoot { speed }, _)) => self.throw(speed, events),
                Some((Ability::Teleport { distance }, _)) => self.teleport(world, distance, events),
                _ => {}
            }
        }
        if let Some(t) = self.throwing {
            self.throwing = (t + 1 < THROW_SHOW).then_some(t + 1);
        }

        // Slopes: gravity along the ground.
        let down = flat(-Vec3::Y, n);
        let slope = if !self.rolling {
            SLOPE
        } else if (dir * speed).dot(down) < 0.0 {
            SLOPE_ROLL_UP
        } else {
            SLOPE_ROLL_DOWN
        };
        let mut v = dir * speed;
        if speed > 0.0 || down.length() > 0.35 {
            v += down * slope;
        }
        speed = v.length();
        dir = if speed > 1e-4 { v / speed } else { dir };

        let locked = self.control_lock > 0;
        self.control_lock = self.control_lock.saturating_sub(1);
        let d = if locked { Vec3::ZERO } else { d };
        let onward = d != Vec3::ZERO && d.dot(dir) > 0.5;
        self.skidding = false;
        if self.rolling {
            if d != Vec3::ZERO && d.dot(dir) < -0.3 {
                speed -= ROLL_DEC;
            } else if d != Vec3::ZERO {
                dir = turn(dir, d, TURN_FAST * 0.5);
            }
            // Holding the way he rolls keeps the speed.
            if !onward {
                speed -= speed.min(FRC * 0.5);
            }
            if speed < ROLL_END {
                self.rolling = false;
            }
        } else if d != Vec3::ZERO {
            if speed < 0.01 {
                dir = d;
                speed = acc;
            } else if d.dot(dir) < -0.5 {
                // Braking (skidding at speed).
                self.skidding = speed >= 4.0;
                speed -= DEC;
                if speed <= 0.0 {
                    dir = d;
                    speed = 0.5;
                }
            } else {
                let k = (speed / top).min(1.0);
                dir = turn(dir, d, TURN_SLOW + (TURN_FAST - TURN_SLOW) * k);
                let top = top * push;
                if speed < top {
                    speed = (speed + acc).min(top);
                }
            }
            if self.skidding && !events.contains(&Event::Skid) {
                events.push(Event::Skid);
            }
        } else {
            speed -= speed.min(FRC);
        }
        if !self.rolling && c.action_pressed && speed >= ROLL_START && self.kit.roll && ground_move.is_none() {
            self.rolling = true;
            events.push(Event::Roll);
        }
        speed = speed.clamp(0.0, MAX_SPEED);
        self.vel = dir * speed;
        if speed > 0.1 {
            self.facing = dir;
        }

        // Move in steps short enough not to go through thin things.
        let steps = (speed / 6.0).ceil().max(1.0) as usize;
        for _ in 0..steps {
            self.pos += self.vel * K / steps as f32;
            self.walls(world, events);
            if !self.stick(world, events) {
                return;
            }
        }
        // Too slow on a wall or a ceiling: slip off.
        if self.control_lock == 0 && self.speed() < SLIP_SPEED && self.up.y < 0.5 {
            self.control_lock = SLIP_LOCK;
            if self.up.y < 0.0 {
                self.leave_ground();
            }
        }
    }

    fn leave_ground(&mut self) {
        self.grounded = false;
        self.skidding = false;
    }

    /// Stop at what is in the way (and shove what moves).
    fn walls(&mut self, world: &impl Sense, events: &mut Vec<Event>) {
        let speed = self.vel.length();
        if speed < 1e-4 {
            return;
        }
        let dir = self.vel / speed;
        let Some(hit) = world.ray(self.pos, dir, REACH) else { return };
        // Slopes he can run up are not in the way.
        if hit.normal.dot(self.up) > 0.55 {
            return;
        }
        if let Some(body) = hit.moving {
            let impulse = dir * speed * K * 60.0 * if self.is_ball() { 6.0 } else { 3.0 };
            events.push(Event::Push { body, impulse, at: hit.point });
            if self.is_ball() {
                return;
            }
        }
        self.pos -= dir * (REACH - hit.dist);
        let into = self.vel.dot(hit.normal);
        if into < 0.0 {
            self.vel -= hit.normal * into;
        }
    }

    /// Keep to the ground; false (and in the air) when it has gone.
    fn stick(&mut self, world: &impl Sense, events: &mut Vec<Event>) -> bool {
        let _ = events;
        let half = self.half();
        let n = self.up;
        let tolerance = ((self.speed() + 4.0).min(14.0)) * K;
        let from = self.pos + n * LIFT;
        // As far as his standing height reaches, so curling up keeps him down.
        let Some(hit) = world.ray(from, -n, LIFT + HALF.max(half) + tolerance) else {
            self.leave_ground();
            return false;
        };
        // Too sharp a change of surface (a cliff edge): off he goes.
        if hit.normal.dot(n) < 0.5 {
            self.leave_ground();
            return false;
        }
        self.pos = hit.point + hit.normal * half;
        let speed = self.vel.length();
        self.up = hit.normal;
        self.vel = flat(self.vel, self.up).normalize_or_zero() * speed;
        true
    }

    fn air_frame(&mut self, world: &impl Sense, c: Controls, events: &mut Vec<Event>) {
        let d = flat(c.dir, Vec3::Y);
        let d = if d.length() > 0.05 { d.normalize() } else { Vec3::ZERO };
        let again = c.jump_pressed && self.jumping;
        let busy = self.flying.is_some() || self.gliding.is_some() || self.stomping.is_some() || self.homing.is_some();
        if (again || c.action_pressed) && !busy && !self.hurt {
            let blocks = self.kit.blocks.clone();
            for (k, b) in blocks.iter().enumerate().take(8) {
                let pressed = match b.on {
                    Button::JumpAgain => again,
                    Button::Action => c.action_pressed,
                };
                if pressed && self.used & (1 << k) == 0 && self.start_move(b.ability, world, events) {
                    self.used |= 1 << k;
                    break;
                }
            }
        }
        for t in [&mut self.doubled, &mut self.throwing] {
            *t = t.map(|f| f + 1).filter(|&f| f < DOUBLE_SHOW.max(THROW_SHOW));
        }
        if let Some(f) = self.dashing {
            self.dashing = (f + 1 < if self.homing.is_some() { HOMING_TIME } else { DASH_TIME }).then_some(f + 1);
        }
        if let Some(target) = self.homing {
            let to = target - self.pos;
            if to.length() < 0.3 || self.dashing.is_none() {
                self.homing = None;
            } else {
                self.vel = to.normalize() * HOMING_SPEED;
                self.facing = flat(self.vel, Vec3::Y).normalize_or(self.facing);
            }
        }
        let p = self.kit.physics;
        let (air, top, grv, release) = (AIR * p.air, TOP * p.speed, GRV * p.gravity, RELEASE * p.jump);
        if self.jumping && !c.jump && self.vel.y > release && self.doubled.is_none() {
            self.vel.y = release;
        }
        // Dashing, homing in or stomping, the move is the motion.
        if self.dashing.is_some() || self.stomping.is_some() {
            if let Some(speed) = self.stomping {
                self.vel = Vec3::NEG_Y * speed;
            }
            let steps = (self.vel.length() / 6.0).ceil().max(1.0) as usize;
            for _ in 0..steps {
                self.pos += self.vel * K / steps as f32;
                if self.collide_air(world, events) {
                    return;
                }
            }
            return;
        }
        let mut h = vec3(self.vel.x, 0.0, self.vel.z);
        let onward = d != Vec3::ZERO && h.length() > 0.1 && d.dot(h.normalize()) > 0.5;
        if let Some(speed) = self.gliding {
            if !c.jump {
                // Let go: he drops, slowed.
                self.gliding = None;
                h *= 0.25;
            } else {
                let dir = if d != Vec3::ZERO {
                    turn(h.normalize_or(self.facing), d, 0.04)
                } else {
                    h.normalize_or(self.facing)
                };
                let speed = (speed + 0.015).min(GLIDE_TOP);
                self.gliding = Some(speed);
                h = dir * speed;
                self.facing = dir;
            }
        } else if d != Vec3::ZERO {
            let along = h.dot(d);
            if along < top {
                h += d * air.min(top - along);
            }
            if h.length() > 0.5 {
                h = turn(h.normalize(), d, 0.03) * h.length();
                self.facing = h.normalize();
            }
        }
        // Air drag near the top of a jump, unless he pushes on the way he goes.
        if !onward && self.gliding.is_none() && self.vel.y > 0.0 && self.vel.y < 4.0 {
            h -= h / 32.0;
        }
        // Gravity (gentler flying and gliding); Tails flaps up with jump.
        let vy = if let Some(f) = self.flying {
            let f = f + 1;
            self.flying = Some(f);
            let mut vy = self.vel.y - FLY_GRV;
            if c.jump_pressed && f < self.fly_time {
                vy = vy.max(0.0) + FLAP;
                events.push(Event::Flap);
            }
            vy.clamp(-MAX_FALL, 3.0)
        } else if self.gliding.is_some() {
            (self.vel.y - 0.05).max(-GLIDE_FALL)
        } else {
            (self.vel.y - grv).max(-MAX_FALL)
        };
        self.vel = vec3(h.x, vy, h.z);

        let steps = (self.vel.length() / 6.0).ceil().max(1.0) as usize;
        for _ in 0..steps {
            self.pos += self.vel * K / steps as f32;
            if self.collide_air(world, events) {
                break;
            }
        }
        self.up = (self.up + (Vec3::Y - self.up) * 0.15).normalize_or(Vec3::Y);
    }

    /// Walls, the ceiling and landing. True when he landed.
    fn collide_air(&mut self, world: &impl Sense, events: &mut Vec<Event>) -> bool {
        let h = vec3(self.vel.x, 0.0, self.vel.z);
        if h.length() > 1e-3 {
            let dir = h.normalize();
            if let Some(hit) = world.ray(self.pos, dir, REACH).filter(|h| h.normal.y.abs() < 0.6) {
                if let Some(body) = hit.moving {
                    events.push(Event::Push { body, impulse: dir * h.length() * K * 60.0 * 3.0, at: hit.point });
                }
                self.pos -= dir * (REACH - hit.dist);
                let into = self.vel.dot(hit.normal);
                if into < 0.0 {
                    self.vel -= hit.normal * into;
                }
                self.gliding = None;
            }
        }
        let half = self.half();
        if self.vel.y > 0.0 {
            if let Some(hit) = world.ray(self.pos, Vec3::Y, half) {
                self.pos.y = hit.point.y - half;
                self.vel.y = 0.0;
            }
            return false;
        }
        // Landing: the feet reach the ground. Coming down on an edge with
        // the middle above it, he lands on it (never slips into it).
        let fall = -self.vel.y * K;
        let from = self.pos + Vec3::Y * LIFT;
        let Some(hit) = world.ray(from, -Vec3::Y, LIFT + half + fall) else { return false };
        if hit.normal.y < 0.3 {
            return false;
        }
        self.pos = hit.point + hit.normal * half;
        self.up = hit.normal;
        // The momentum carries into the ground.
        let speed_along = flat(self.vel, self.up);
        self.vel = speed_along;
        if self.hurt {
            self.hurt = false;
            self.vel = Vec3::ZERO;
        }
        self.grounded = true;
        self.jumping = false;
        self.rolling = false;
        self.flying = None;
        self.gliding = None;
        self.warped = false;
        if self.stomping.take().is_some() {
            events.push(Event::Stomped);
        }
        self.used = 0;
        self.dashing = None;
        self.homing = None;
        self.doubled = None;
        events.push(Event::Land);
        true
    }

    /// Start one of his moves in the air; false if it cannot go now.
    fn start_move(&mut self, a: Ability, world: &impl Sense, events: &mut Vec<Event>) -> bool {
        match a {
            Ability::Fly { seconds } if self.flying.is_none() => {
                self.flying = Some(0);
                self.fly_time = (seconds * 60.0) as u32;
                self.jumping = false;
                events.push(Event::Flap);
            }
            Ability::Glide { .. } if self.gliding.is_none() => {
                self.gliding = Some(GLIDE_START);
                self.jumping = false;
                self.vel.y = self.vel.y.min(0.0);
                events.push(Event::Glide);
            }
            Ability::Hammer if self.hammer.is_none() => {
                self.hammer = Some(0);
                events.push(Event::Hammer);
            }
            Ability::Teleport { distance } if !self.warped => {
                self.teleport(world, distance, events);
                self.vel.y = self.vel.y.max(2.0);
                self.warped = true;
            }
            Ability::DoubleJump { power } => {
                self.vel.y = self.vel.y.max(JMP * self.kit.physics.jump * power);
                self.jumping = true;
                self.doubled = Some(0);
                events.push(Event::DoubleJump);
            }
            Ability::AirDash { speed } => self.air_dash(speed, events),
            Ability::Homing { range } => {
                let reach = range * K;
                let ahead = flat(self.facing, Vec3::Y).normalize_or(Vec3::X);
                let target = self
                    .targets
                    .iter()
                    .copied()
                    .filter(|t| {
                        let d = *t - self.pos;
                        d.length() < reach && d.dot(ahead) > -0.3 && d.y > -reach * 0.6
                    })
                    .min_by(|a, b| (*a - self.pos).length().total_cmp(&(*b - self.pos).length()));
                match target {
                    Some(t) => {
                        self.homing = Some(t);
                        self.dashing = Some(0);
                        self.jumping = true;
                        events.push(Event::AirDash);
                    }
                    None => self.air_dash(9.0, events),
                }
            }
            Ability::Stomp { speed } => {
                self.stomping = Some(speed);
                self.jumping = true;
                events.push(Event::Stomp);
            }
            Ability::Shoot { speed } => self.throw(speed, events),
            _ => return false,
        }
        true
    }

    /// On through the air the way he faces, level, for a moment.
    fn air_dash(&mut self, speed: f32, events: &mut Vec<Event>) {
        let ahead = flat(self.facing, Vec3::Y).normalize_or(Vec3::X);
        self.vel = ahead * flat(self.vel, Vec3::Y).length().max(speed);
        self.jumping = true;
        self.dashing = Some(0);
        events.push(Event::AirDash);
    }

    fn throw(&mut self, speed: f32, events: &mut Vec<Event>) {
        let dir = flat(self.facing, Vec3::Y).normalize_or(Vec3::X);
        self.throwing = Some(0);
        let speed = speed + flat(self.vel, Vec3::Y).length().min(8.0) * 0.5;
        events.push(Event::Shoot { from: self.pos + dir * 0.4, dir, speed });
    }

    /// Vanish and turn up `distance` guide px ahead, short of walls.
    fn teleport(&mut self, world: &impl Sense, distance: f32, events: &mut Vec<Event>) {
        let dir = flat(self.facing, Vec3::Y).normalize_or(Vec3::X);
        let far = distance * K * 2.2;
        let room = world.ray(self.pos, dir, far + 0.6).map_or(far, |h| (h.dist - 0.6).max(0.0));
        self.pos += dir * room;
        events.push(Event::Teleport);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A floor at y = 0, 400 m square.
    fn floor() -> Phys {
        let mut p = Phys::new();
        let s = 200.0;
        let pts = [vec3(-s, 0.0, -s), vec3(s, 0.0, -s), vec3(s, 0.0, s), vec3(-s, 0.0, s)];
        p.add_mesh(&pts, &[[0, 2, 1], [0, 3, 2]]);
        p.refresh();
        p
    }

    fn go(dir: Vec3) -> Controls {
        Controls { dir, ..Controls::default() }
    }

    fn run(r: &mut Runner, w: &Phys, c: Controls, frames: usize) -> Vec<Event> {
        (0..frames).flat_map(|_| r.frame(w, c)).collect()
    }

    #[test]
    fn he_speeds_up_to_the_top_turns_keeping_his_speed_and_stops() {
        let w = floor();
        let mut r = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
        run(&mut r, &w, go(Vec3::X), 200);
        assert!((r.speed() - TOP).abs() < 1e-3, "top speed: {}", r.speed());
        assert!(r.grounded && (r.feet().y).abs() < 1e-3, "on the floor: {:?}", r.feet());
        assert!(r.pos.x > 10.0);
        // Turning to the side keeps the speed.
        run(&mut r, &w, go(Vec3::Z), 60);
        assert!(r.vel.normalize().dot(Vec3::Z) > 0.99, "turned: {:?}", r.vel);
        assert!(r.speed() > TOP - 0.01, "kept his speed through the bend: {}", r.speed());
        // Letting go, friction stops him; the other way, he brakes.
        let mut s = r.clone();
        run(&mut s, &w, Controls::default(), 200);
        assert!(s.speed() < 1e-3);
        let events = run(&mut r, &w, go(-Vec3::Z), 4);
        assert!(events.contains(&Event::Skid) && r.speed() < TOP - 1.5);
    }

    #[test]
    fn a_jump_rises_lands_and_keeps_the_momentum_held_onward() {
        let w = floor();
        let mut r = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
        r.vel = Vec3::X * 9.0;
        let start = r.pos.y;
        let mut top = start;
        let ev = r.frame(&w, Controls { dir: Vec3::X, jump: true, jump_pressed: true, ..Controls::default() });
        assert!(ev.contains(&Event::Jump) && !r.grounded && r.is_ball());
        let mut frames = 0;
        while !r.grounded && frames < 200 {
            r.frame(&w, Controls { dir: Vec3::X, jump: true, ..Controls::default() });
            top = top.max(r.pos.y);
            frames += 1;
        }
        let rise = (top - start) / K;
        assert!((rise - 96.6).abs() < 8.0, "rose {rise} guide px");
        assert!(r.grounded && (r.speed() - 9.0).abs() < 0.01, "landed as fast: {}", r.speed());
        // Let go of the way he goes and the air slows him.
        let mut r = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
        r.vel = Vec3::X * 9.0;
        r.frame(&w, Controls { jump: true, jump_pressed: true, ..Controls::default() });
        run(&mut r, &w, Controls { jump: true, ..Controls::default() }, 40);
        assert!(r.vel.x < 8.0);
    }

    #[test]
    fn rolling_and_the_spin_dash() {
        let w = floor();
        let mut r = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
        r.vel = Vec3::X * 8.0;
        let ev = r.frame(&w, Controls { dir: Vec3::X, action_pressed: true, action: true, ..Controls::default() });
        assert!(ev.contains(&Event::Roll) && r.rolling);
        run(&mut r, &w, go(Vec3::X), 120);
        assert!(r.rolling && r.speed() > 7.9, "rolled on held onward: {}", r.speed());
        run(&mut r, &w, Controls::default(), 400);
        assert!(!r.rolling && r.speed() < 3.0, "slowed and stood up: {}", r.speed());
        // The spin dash from standing.
        let mut r = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
        let hold = Controls { action: true, action_pressed: true, ..Controls::default() };
        assert!(r.frame(&w, hold).contains(&Event::Rev));
        r.frame(&w, Controls { action: true, jump_pressed: true, ..Controls::default() });
        r.frame(&w, Controls { action: true, ..Controls::default() });
        let ev = r.frame(&w, Controls::default());
        assert!(ev.contains(&Event::Dash) && r.rolling && r.speed() > 8.0, "dashed off at {}", r.speed());
    }

    #[test]
    fn slopes_speed_him_up_and_slow_him_down_and_ledges_drop_him() {
        // A ramp down along x: 30 m long, 9 m drop, then flat.
        let mut w = Phys::new();
        let pts = [
            vec3(0.0, 9.0, -5.0),
            vec3(30.0, 0.0, -5.0),
            vec3(30.0, 0.0, 5.0),
            vec3(0.0, 9.0, 5.0),
            vec3(80.0, 0.0, -5.0),
            vec3(80.0, 0.0, 5.0),
        ];
        w.add_mesh(&pts, &[[0, 2, 1], [0, 3, 2], [1, 5, 4], [1, 2, 5]]);
        w.refresh();
        let mut r = Runner::new(Character::Sonic, vec3(2.0, 8.4, 0.0), Vec3::X);
        r.frame(&w, go(Vec3::X));
        assert!(r.grounded && r.up.x > 0.2, "on the slope: {:?}", r.up);
        let mut most: f32 = 0.0;
        for _ in 0..600 {
            r.frame(&w, go(Vec3::X));
            most = most.max(r.speed());
            if r.pos.x > 75.0 {
                break;
            }
        }
        assert!(most > TOP + 0.5, "downhill takes him past the top: {most}");
        // Off the end of the floor he falls.
        run(&mut r, &w, go(Vec3::X), 60);
        assert!(!r.grounded && r.pos.y < -0.5, "off the edge: {:?}", r.pos);
    }

    #[test]
    fn walls_stop_him_and_things_that_move_are_pushed() {
        let mut w = floor();
        // A wall across x = 10.
        let pts = [vec3(10.0, 0.0, -5.0), vec3(10.0, 0.0, 5.0), vec3(10.0, 4.0, 5.0), vec3(10.0, 4.0, -5.0)];
        w.add_mesh(&pts, &[[0, 1, 2], [0, 2, 3]]);
        let crate_ = w.add_body(super::super::phys::Solid::Box(Vec3::splat(0.5)), vec3(0.0, 0.5, 6.0), Vec3::ZERO, 1.0);
        w.refresh();
        let mut r = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
        run(&mut r, &w, go(Vec3::X), 240);
        assert!(r.pos.x < 10.0 && r.pos.x > 9.0, "stopped at the wall: {:?}", r.pos);
        assert!(r.speed() < 0.2, "{}", r.speed());
        // Running into the crate pushes it.
        let mut r = Runner::new(Character::Sonic, vec3(0.0, 0.0, 0.0), Vec3::Z);
        let events = run(&mut r, &w, go(Vec3::Z), 120);
        assert!(events.iter().any(|e| matches!(e, Event::Push { body, .. } if *body == crate_)));
    }

    #[test]
    fn each_character_has_their_move_in_the_air() {
        let w = floor();
        let jump = |c: Character| {
            let mut r = Runner::new(c, Vec3::ZERO, Vec3::X);
            r.frame(&w, Controls { jump: true, jump_pressed: true, ..Controls::default() });
            run(&mut r, &w, Controls { jump: true, ..Controls::default() }, 10);
            r
        };
        let again = Controls { jump: true, jump_pressed: true, ..Controls::default() };
        // Tails flies: flapping keeps him up.
        let mut t = jump(Character::Tails);
        assert!(t.frame(&w, again).contains(&Event::Flap) && t.flying.is_some());
        for _ in 0..60 {
            t.frame(&w, if t.vel.y < 0.0 { again } else { Controls { jump: true, ..Controls::default() } });
        }
        assert!(!t.grounded && t.pos.y > 2.0, "flying: {:?}", t.pos);
        // Knuckles glides: slow fall, speeding up along.
        let mut k = jump(Character::Knuckles);
        assert!(k.frame(&w, again).contains(&Event::Glide));
        run(&mut k, &w, Controls { jump: true, dir: Vec3::X, ..Controls::default() }, 30);
        assert!(k.gliding.is_some() && k.vel.y >= -GLIDE_FALL - 1e-4 && k.vel.x > GLIDE_START);
        // Amy swings her hammer, and it breaks things for a while.
        let mut a = jump(Character::Amy);
        assert!(a.frame(&w, again).contains(&Event::Hammer));
        run(&mut a, &w, Controls { jump: true, ..Controls::default() }, 5);
        assert!(a.attacking());
        // Sonic.exe vanishes and turns up further on, once a jump.
        let mut e = jump(Character::Exe);
        let x = e.pos.x;
        assert!(e.frame(&w, again).contains(&Event::Teleport));
        assert!((e.pos.x - x - WARP).abs() < 0.2, "warped {} m", e.pos.x - x);
        assert!(!e.frame(&w, again).contains(&Event::Teleport));
    }

    #[test]
    fn made_up_moves_work_in_3d_too() {
        use crate::sonic::kit::Block;
        let w = floor();
        let with = |blocks: Vec<Block>| {
            let mut r = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
            r.kit.blocks = blocks;
            r
        };
        let press = Controls { jump: true, jump_pressed: true, ..Controls::default() };
        let hold = Controls { jump: true, ..Controls::default() };
        // A second jump.
        let mut d = with(vec![Block::new(Ability::DoubleJump { power: 1.0 })]);
        d.frame(&w, press);
        run(&mut d, &w, hold, 20);
        assert!(d.frame(&w, press).contains(&Event::DoubleJump) && d.vel.y > 5.0);
        // A dash: level and fast.
        let mut a = with(vec![Block::new(Ability::AirDash { speed: 10.0 })]);
        a.frame(&w, press);
        run(&mut a, &w, hold, 10);
        let y = a.pos.y;
        assert!(a.frame(&w, press).contains(&Event::AirDash));
        run(&mut a, &w, hold, 5);
        assert!(a.vel.x >= 10.0 && (a.pos.y - y).abs() < 0.05);
        // Homing in on a target ahead, then up off it.
        let mut h = with(vec![Block::new(Ability::Homing { range: 200.0 })]);
        let target = vec3(4.0, 3.0, 0.5);
        h.targets = vec![vec3(-4.0, 2.0, 0.0), target];
        h.frame(&w, press);
        run(&mut h, &w, hold, 6);
        h.frame(&w, press);
        assert_eq!(h.homing, Some(target));
        for _ in 0..30 {
            h.frame(&w, Controls::default());
            if (h.pos - target).length() < 0.6 {
                break;
            }
        }
        assert!((h.pos - target).length() < 0.6, "got there: {:?}", h.pos);
        h.rebound();
        assert!(h.homing.is_none() && h.vel.y > 5.0);
        // A stomp lands with a thud; a throw comes out ahead.
        let mut s = with(vec![Block { ability: Ability::Stomp { speed: 12.0 }, on: Button::Action }]);
        s.frame(&w, press);
        run(&mut s, &w, hold, 15);
        assert!(s.frame(&w, Controls { action_pressed: true, ..Controls::default() }).contains(&Event::Stomp));
        let events = run(&mut s, &w, Controls::default(), 60);
        assert!(s.grounded && events.contains(&Event::Stomped));
        let mut t = with(vec![Block { ability: Ability::Shoot { speed: 9.0 }, on: Button::Action }]);
        let events = t.frame(&w, Controls { action_pressed: true, ..Controls::default() });
        assert!(events.iter().any(|e| matches!(e, Event::Shoot { dir, .. } if dir.x > 0.99)));
        // Faster by the kit.
        let mut f = Runner::new(Character::Sonic, Vec3::ZERO, Vec3::X);
        f.kit.physics.speed = 1.5;
        run(&mut f, &w, go(Vec3::X), 300);
        assert!((f.speed() - 9.0).abs() < 0.01, "{}", f.speed());
    }

    /// A loop of radius 6 m in the x-y plane, its floor at y = 0, inside
    /// facing in, on a long floor. (The courses shift a loop sideways as he
    /// goes round so he comes out beside where he went in; here he just goes
    /// round and round.)
    fn loop_world() -> Phys {
        let mut w = Phys::new();
        let mut pts = vec![];
        let mut tris = vec![];
        let (r, width, n) = (6.0f32, 3.0f32, 48);
        for k in 0..=n {
            let t = k as f32 / n as f32 * std::f32::consts::TAU;
            let p = vec3(t.sin() * r, r - t.cos() * r, 0.0);
            pts.push(p - vec3(0.0, 0.0, width));
            pts.push(p + vec3(0.0, 0.0, width));
        }
        for k in 0..n as u32 {
            let (a, b) = (k * 2, k * 2 + 2);
            tris.push([a, b, b + 1]);
            tris.push([a, b + 1, a + 1]);
        }
        let base = pts.len() as u32;
        pts.extend([vec3(-60.0, 0.0, -3.0), vec3(60.0, 0.0, -3.0), vec3(60.0, 0.0, 3.0), vec3(-60.0, 0.0, 3.0)]);
        tris.extend([[base, base + 2, base + 1], [base, base + 3, base + 2]]);
        w.add_mesh(&pts, &tris);
        w.refresh();
        w
    }

    #[test]
    fn he_goes_round_a_loop_when_fast_and_falls_off_when_slow() {
        let w = loop_world();
        let mut fast = Runner::new(Character::Sonic, vec3(-20.0, 0.0, 0.0), Vec3::X);
        fast.vel = Vec3::X * 12.0;
        let mut lowest_up: f32 = 1.0;
        let mut round = false;
        for _ in 0..400 {
            fast.frame(&w, go(Vec3::X));
            lowest_up = lowest_up.min(fast.up.y);
            assert!(fast.grounded, "kept to the loop at {:?}", fast.pos);
            if lowest_up < -0.9 && fast.up.y > 0.99 {
                round = true;
                break;
            }
        }
        assert!(lowest_up < -0.9, "upside down at the top: {lowest_up}");
        assert!(round && fast.vel.x > 6.0, "came round at the bottom going on: {:?} {:?}", fast.pos, fast.vel);
        let mut slow = Runner::new(Character::Sonic, vec3(-6.0, 0.0, 0.0), Vec3::X);
        slow.vel = Vec3::X * 4.0;
        let mut fell = false;
        for _ in 0..400 {
            slow.frame(&w, go(Vec3::X));
            fell |= !slow.grounded && slow.up.y < 0.9;
        }
        assert!(fell, "too slow for the loop");
    }
}
