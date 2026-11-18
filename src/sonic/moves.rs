//! The other characters' own moves, after the Sonic Physics Guide where
//! it describes them (Tails and Knuckles in Sonic 3 & Knuckles):
//!
//! - Tails flies: jump again in the air and he hovers down slowly,
//!   flapping up a little at each press, for 8 seconds before tiring.
//! - Knuckles jumps lower; jump again in the air and he glides, faster
//!   and faster, sinking slowly; let go and he drops. Gliding into a wall
//!   he grabs it and climbs (↑ ↓), pulls himself over its top, or jumps
//!   off it. Landing from a glide he slides on his belly.
//! - Amy swings her hammer (the action button) on the ground, and spins
//!   it when she jumps again in the air; both destroy badniks.
//! - Sonic.exe (the creepypasta's) runs like Sonic; jump again in the air
//!   and he vanishes and reappears a little further on (once a jump),
//!   never through a wall.
//!
//! These are moves in a [`super::kit::Kit`], each on a button, as are the
//! ones any character can be given: a double jump, a dash through the
//! air, a homing attack, a stomp, something to throw.

use super::kit::{Ability, Button};
use super::*;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Default, Hash)]
pub enum Character {
    #[default]
    Sonic,
    Tails,
    Knuckles,
    Amy,
    Exe,
    /// One the player made (its place in [`super::custom`]'s list).
    Custom(u8),
}

impl Character {
    /// The built-in characters.
    pub const ALL: [Character; 5] =
        [Character::Sonic, Character::Tails, Character::Knuckles, Character::Amy, Character::Exe];

    /// Every character: the built-in ones, then the player's.
    pub fn every() -> Vec<Character> {
        let mut out = Self::ALL.to_vec();
        out.extend(super::custom::all());
        out
    }

    pub fn label(self) -> &'static str {
        match self {
            Character::Sonic => "Sonic",
            Character::Tails => "Tails",
            Character::Knuckles => "Knuckles",
            Character::Amy => "Amy",
            Character::Exe => "Sonic.exe",
            Character::Custom(i) => super::custom::label(i),
        }
    }

    /// A shorter name, for small buttons.
    pub fn short(self) -> &'static str {
        match self {
            Character::Exe => "Exe",
            c => c.label(),
        }
    }

    /// The built-in character this one is drawn and sized as (itself, or
    /// what a made-up one starts from).
    pub fn base(self) -> Character {
        match self {
            Character::Custom(i) => super::custom::get(i).map_or(Character::Sonic, |c| c.base.base()),
            c => c,
        }
    }

    pub fn is_custom(self) -> bool {
        matches!(self, Character::Custom(_))
    }

    /// How they move and their moves.
    pub fn kit(self) -> super::kit::Kit {
        match self {
            Character::Custom(i) => super::custom::get(i).map(|c| c.kit).unwrap_or_default(),
            c => super::kit::Kit::of(c),
        }
    }

    /// Made-up characters' own colours (main, belly and muzzle, shoes).
    pub fn colours(self) -> Option<[[u8; 3]; 3]> {
        match self {
            Character::Custom(i) => super::custom::get(i).and_then(|c| c.colours),
            _ => None,
        }
    }

    /// The next one (or the one before), the player's included.
    pub fn cycle(self, dir: i32) -> Self {
        let all = Self::every();
        let i = all.iter().position(|&c| c == self).unwrap_or(0) as i32;
        all[(i + dir).rem_euclid(all.len() as i32) as usize]
    }

    /// What each one does on top of running, rolling and jumping.
    pub fn moves(self) -> String {
        match self {
            Character::Sonic => "↓ + Space spin dash".into(),
            Character::Tails => "Space in the air: fly (tap to flap)".into(),
            Character::Knuckles => "Space in the air: glide  ·  into a wall: climb ↑ ↓".into(),
            Character::Amy => "X: hammer  ·  Space in the air: hammer spin".into(),
            Character::Exe => "Space in the air: teleport forward".into(),
            Character::Custom(_) => self.kit().about(),
        }
    }

    /// How it is kept in files: the built-in ones by name, the player's as
    /// `custom:` and theirs.
    pub fn key(self) -> String {
        match self {
            Character::Sonic => "sonic".into(),
            Character::Tails => "tails".into(),
            Character::Knuckles => "knuckles".into(),
            Character::Amy => "amy".into(),
            Character::Exe => "exe".into(),
            Character::Custom(_) => format!("custom:{}", self.label()),
        }
    }

    /// The character a key names (Sonic if it is not known).
    pub fn from_key(key: &str) -> Character {
        match key {
            "sonic" => Character::Sonic,
            "tails" => Character::Tails,
            "knuckles" => Character::Knuckles,
            "amy" => Character::Amy,
            "exe" => Character::Exe,
            k => k.strip_prefix("custom:").and_then(super::custom::find).unwrap_or(Character::Sonic),
        }
    }

    /// Half width and half height standing (guide px).
    pub(super) fn stand(self) -> (f32, f32) {
        match self.base() {
            Character::Tails => (9.0, 15.0),
            Character::Amy => (9.0, 16.0),
            _ => STAND,
        }
    }
}

impl Serialize for Character {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.serialize_str(&self.key())
    }
}

impl<'de> Deserialize<'de> for Character {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        let key = String::deserialize(d)?;
        Ok(Character::from_key(&key))
    }
}

/// Tails' gravity while flying, the lift of a flap (until he rises at
/// [`FLAP_UNTIL`]), and how long he can fly (frames).
const FLY_GRV: f32 = 0.03125;
const FLAP: f32 = -0.125;
const FLAP_UNTIL: f32 = -1.0;
#[cfg(test)]
const FLY_TIME: u32 = 480;
/// Knuckles: glide start speed, its gain per frame and top; the fall speed
/// a glide settles at and how quickly; belly-slide friction; climbing
/// speed; frames to pull himself over a wall.
const GLIDE_START: f32 = 4.0;
const GLIDE_ACC: f32 = 0.015625;
const GLIDE_TOP: f32 = 24.0;
const GLIDE_FALL: f32 = 0.5;
const GLIDE_GRV: f32 = 0.125;
const SLIDE_FRC: f32 = 0.125;
const CLIMB: f32 = 1.0;
const CLIMB_UP: u32 = 18;
/// Amy's hammer: frames of a swing on the ground (it hits in the middle
/// of it), of a spin in the air, and its reach in front of her (guide px).
const HAMMER: u32 = 22;
const HAMMER_HITS: (u32, u32) = (5, 16);
const HAMMER_AIR: u32 = 30;
const HAMMER_REACH: f32 = 24.0;
/// Sonic.exe's teleport: how long it shows (frames).
const WARP_SHOW: u32 = 14;
/// A dash through the air: how long it lasts (frames); a homing attack's
/// speed and how long it keeps going for its target; how long a second
/// jump and a throw show.
const DASH_TIME: u32 = 14;
const HOMING_SPEED: f32 = 12.0;
const HOMING_TIME: u32 = 45;
const DOUBLE_SHOW: u32 = 16;
const THROW_SHOW: u32 = 12;
/// A drop dash is ready after this many frames held (as in Sonic Mania).
pub(super) const DROP_READY: u32 = 20;
/// A peel out is ready after this many frames revved (as in Sonic CD).
const PEEL_READY: u32 = 30;
/// An insta-shield: how long it lasts (frames) and how far round him it
/// reaches (guide px, as in Sonic 3 & Knuckles).
const SHIELD_TIME: u32 = 16;
const SHIELD_REACH: f32 = 24.0;
/// A bounce: how fast it goes down, and how much higher than a jump it
/// comes back up (Sonic 3's bubble shield: 8, and 7.5 for a 6.5 jump).
const BOUNCE_DOWN: f32 = 8.0;
const BOUNCE_UP: f32 = 1.15;
/// A wall jump: its push off the wall, how close a wall must be (guide
/// px) and how long it shows.
const WALL_KICK: f32 = 4.5;
const WALL_NEAR: f32 = 4.0;
const KICK_SHOW: u32 = 12;
/// Frames after a wall jump before he steers again.
const KICK_LOCK: u32 = 8;
/// A boost: its speed (at least), how fast he may fall boosting in the
/// air, the least of it to start, and how many times slower it fills
/// than it drains.
const BOOST: f32 = 12.0;
const BOOST_FALL: f32 = 1.0;
const BOOST_LEAST: f32 = 15.0;
const BOOST_REFILL: f32 = 3.0;

impl Sonic {
    /// Turn into `c` where he is, feet staying on the ground.
    pub fn become_character(&mut self, c: Character) {
        let before = self.radii().1;
        self.character = c;
        self.kit = c.kit();
        self.end_moves();
        if self.grounded {
            self.pos += mode_down(self.angle) * (before - self.radii().1) * SCALE;
        }
    }

    /// Drop whatever move is going on.
    pub(super) fn end_moves(&mut self) {
        self.flying = None;
        self.flap = false;
        self.gliding = None;
        self.dropping = false;
        self.sliding = false;
        self.climbing = None;
        self.climb_up = None;
        self.hammer = None;
        self.hammer_air = false;
        self.warp = None;
        self.warped = false;
        self.used = 0;
        self.doubled = None;
        self.dashing = None;
        self.throwing = None;
        self.homing = None;
        self.stomping = None;
        self.drop_charge = None;
        self.peeling = None;
        self.shielding = None;
        self.bouncing = None;
        self.bounced = false;
        self.kicked = None;
        self.boosting = false;
    }

    /// The hammer is out and hitting.
    pub(super) fn hammer_hits(&self) -> bool {
        match self.hammer {
            Some(t) if !self.hammer_air => (HAMMER_HITS.0..=HAMMER_HITS.1).contains(&t),
            Some(_) => true,
            None => false,
        }
    }

    /// The box the hammer (or an insta-shield) hits beyond his own
    /// (centre, half size; world px).
    pub fn reach(&self) -> Option<(Vec2, Vec2)> {
        if self.shielding.is_some() {
            return Some((self.pos, Vec2::splat(SHIELD_REACH * SCALE)));
        }
        if !self.hammer_hits() || self.hammer_air {
            return None;
        }
        let half = vec2(HAMMER_REACH / 2.0 + 4.0, 16.0) * SCALE;
        Some((self.pos + vec2(self.facing * (HAMMER_REACH / 2.0 + 6.0), 0.0) * SCALE, half))
    }

    /// Animation speed for a move (frames each picture is held), if one is going on.
    pub(super) fn move_hold(&self) -> Option<f32> {
        if self.flying.is_some() {
            Some(if self.tired() { 6.0 } else { 1.0 })
        } else if self.climbing.is_some() {
            Some(if self.ysp == 0.0 { f32::INFINITY } else { 5.0 })
        } else if self.climb_up.is_some() {
            Some(5.0)
        } else if self.hammer.is_some() {
            Some(if self.hammer_air { 1.0 } else { 3.0 })
        } else if self.gliding.is_some() || self.sliding {
            Some(f32::INFINITY)
        } else if let Some(t) = self.peeling {
            Some((8.0 - t as f32 * 0.25).max(0.0))
        } else if self.warp.is_some() || self.dashing.is_some() || self.homing.is_some() || self.throwing.is_some() {
            Some(3.0)
        } else {
            None
        }
    }

    /// Tired of flying.
    pub fn tired(&self) -> bool {
        self.flying.is_some_and(|f| f >= self.fly_time)
    }

    /// Nothing hurts him for a moment (an insta-shield's flash).
    pub fn shielded(&self) -> bool {
        self.shielding.is_some()
    }

    /// A drop dash charged and ready for the landing.
    pub fn drop_ready(&self) -> bool {
        self.drop_charge.is_some_and(|f| f >= DROP_READY)
    }

    /// A peel out revved enough to go.
    pub fn peel_ready(&self) -> bool {
        self.peeling.is_some_and(|t| t >= PEEL_READY)
    }

    /// The longest boost his moves give (frames), if he has one.
    pub fn boost_most(&self) -> Option<f32> {
        self.kit.blocks.iter().find_map(|b| match b.ability {
            Ability::Boost { seconds } => Some(seconds * FPS),
            _ => None,
        })
    }

    /// How full the boost is (0 to 1), if he has one.
    pub fn boost_gauge(&self) -> Option<f32> {
        self.boost_most().map(|most| (self.boost_left / most).clamp(0.0, 1.0))
    }

    /// Start boosting, if there is enough of it left.
    fn start_boost(&mut self, events: &mut Vec<Event>) -> bool {
        if self.boosting || self.boost_left < BOOST_LEAST {
            return false;
        }
        self.boosting = true;
        events.push(Event::Boost);
        true
    }

    /// Boosting: at least [`BOOST`] (more if he is faster) his way while
    /// the button is held and some is left; off, it fills again slowly.
    /// `top`: his top speed.
    pub(super) fn boost_frame(&mut self, input: Input, top: f32, events: &mut Vec<Event>) {
        let Some(most) = self.boost_most() else {
            self.boosting = false;
            return;
        };
        self.boost_left = self.boost_left.min(most);
        let busy = self.climbing.is_some() || self.climb_up.is_some() || self.spindash.is_some() || self.dead;
        if self.boosting && (!input.action || self.boost_left <= 0.0 || self.hurt || busy) {
            self.boosting = false;
        }
        if !self.boosting {
            self.boost_left = (self.boost_left + 1.0 / BOOST_REFILL).min(most);
            return;
        }
        self.boost_left = (self.boost_left - 1.0).max(0.0);
        let speed = BOOST.max(top * 2.0);
        let f = self.facing;
        if self.grounded {
            self.gsp = f * self.gsp.abs().max(speed);
            self.rolling = false;
            self.crouching = false;
        } else {
            self.xsp = f * self.xsp.abs().max(speed);
            self.ysp = self.ysp.min(BOOST_FALL);
        }
        if self.boost_left <= 0.0 {
            self.boosting = false;
            events.push(Event::Skid);
        }
    }

    /// A wall close beside him in the air: the side it is on (the way he
    /// faces first).
    fn wall_beside(&self, world: &PhysWorld, env: Env) -> Option<f32> {
        [self.facing, -self.facing].into_iter().find(|&d| {
            self.cast(world, env, self.pos, vec2(d, 0.0), PUSH + WALL_NEAR)
                .is_some_and(|h| h.normal.x.abs() > 0.7 && h.ground.mass <= 0.0 && h.ground.bounce < 0.6)
        })
    }

    /// On the ground, before moving: Amy's hammer and Knuckles' slide take
    /// over the controls. Returns the input left to the usual movement.
    pub(super) fn ground_moves(&mut self, world: &PhysWorld, env: Env, input: Input, events: &mut Vec<Event>) -> Input {
        if self.sliding {
            self.gsp -= self.gsp.abs().min(SLIDE_FRC) * self.gsp.signum();
            if self.gsp.abs() < 0.1 {
                self.gsp = 0.0;
                self.sliding = false;
            }
            return Input::default();
        }
        // Revving a peel out he stands still (turning round if asked);
        // let go of the button, he is off if it was revved enough. A jump
        // drops it.
        if let Some(t) = self.peeling {
            if input.jump_pressed {
                self.peeling = None;
                return input;
            }
            if input.action {
                self.peeling = Some(t + 1);
                if t + 1 == PEEL_READY {
                    events.push(Event::Rev);
                }
                if input.left != input.right {
                    self.facing = if input.left { -1.0 } else { 1.0 };
                }
                self.gsp = 0.0;
                self.crouching = false;
                self.looking_up = false;
                return Input::default();
            }
            self.peeling = None;
            if t >= PEEL_READY {
                self.gsp = self.facing * self.peel_speed;
                events.push(Event::Dash);
            }
            return input;
        }
        // The action button's moves that work on the ground: the first one.
        let busy = self.hammer.is_some() || self.rolling || self.spindash.is_some() || self.boosting;
        if input.action_pressed && !busy {
            let ground = self
                .kit
                .blocks
                .iter()
                .map(|b| (b.ability, b.on))
                .find(|(a, on)| *on == Button::Action && a.on_ground());
            match ground {
                Some((Ability::Hammer, _)) => {
                    self.hammer = Some(0);
                    self.hammer_air = false;
                    self.crouching = false;
                    self.looking_up = false;
                    events.push(Event::Hammer);
                }
                Some((Ability::Shoot { speed }, _)) => self.throw_one(speed, events),
                Some((Ability::Teleport { distance }, _)) => self.teleport(world, env, distance, events),
                Some((Ability::PeelOut { speed }, _)) if self.gsp.abs() < 0.5 => {
                    self.peeling = Some(0);
                    self.peel_speed = speed;
                    events.push(Event::Rev);
                }
                Some((Ability::Boost { .. }, _)) => {
                    self.start_boost(events);
                }
                _ => {}
            }
        }
        // Boosting he goes on his way: no braking, no turning round.
        if self.boosting {
            let onward = |d: f32| self.facing * d > 0.0;
            return Input { left: input.left && onward(-1.0), right: input.right && onward(1.0), ..input };
        }
        if let Some(t) = self.throwing {
            self.throwing = (t + 1 < THROW_SHOW).then_some(t + 1);
        }
        if let Some(t) = self.hammer.filter(|_| !self.hammer_air) {
            self.hammer = (t + 1 < HAMMER).then_some(t + 1);
            if self.hammer_hits() {
                self.hammer_smash(world, env, events);
            }
            // She swings on the run, keeping her speed (still, she swings
            // where she stands): she can only keep going or brake.
            let onward = |d: f32| self.gsp * d > 0.0;
            return Input {
                left: input.left && onward(-1.0),
                right: input.right && onward(1.0),
                up: false,
                down: false,
                jump_pressed: false,
                ..input
            };
        }
        input
    }

    /// Breakable things within the hammer's reach go to pieces.
    fn hammer_smash(&self, world: &PhysWorld, env: Env, events: &mut Vec<Event>) {
        let dir = vec2(self.facing, 0.0);
        let hr = self.radii().1;
        for dy in [-hr * 0.5, 0.0, hr * 0.6] {
            let from = self.pos + vec2(0.0, dy) * SCALE;
            if let Some(hit) = self.cast(world, env, from, dir, PUSH + HAMMER_REACH) {
                if hit.ground.breakable && !events.contains(&Event::Break { collider: hit.collider, at: hit.point }) {
                    events.push(Event::Break { collider: hit.collider, at: hit.point });
                }
            }
        }
    }

    /// In the air: jumping again (or the action button) starts one of his
    /// moves, the first on that button not used yet this jump; moves going
    /// on carry on. `jmp`: his jump's speed. Returns whether he steers as
    /// usual.
    pub(super) fn air_moves(
        &mut self,
        world: &PhysWorld,
        env: Env,
        input: Input,
        jmp: f32,
        events: &mut Vec<Event>,
    ) -> bool {
        let again = input.jump_pressed && self.jumping && !self.hurt;
        // With every emerald and enough rings, jumping again turns him super first.
        if again && self.can_super && !self.super_form {
            self.super_form = true;
            self.can_super = false;
            self.transforming = Some(0);
            events.push(Event::Transform);
            return true;
        }
        let action = input.action_pressed && !self.hurt && !self.dead;
        // Flying, gliding or stomping, the buttons are theirs.
        let busy = self.flying.is_some() || self.gliding.is_some() || self.stomping.is_some() || self.homing.is_some();
        if (again || action) && !busy {
            let blocks = self.kit.blocks.clone();
            for (k, b) in blocks.iter().enumerate().take(8) {
                let pressed = match b.on {
                    Button::JumpAgain => again,
                    Button::Action => action,
                };
                if pressed && self.used & (1 << k) == 0 && self.start_move(b.ability, world, env, jmp, events) {
                    // A wall jump and a boost can go again (a wall jump
                    // even makes the others ready again).
                    let again = matches!(b.ability, Ability::WallJump { .. } | Ability::Boost { .. });
                    if !again {
                        self.used |= 1 << k;
                    }
                    break;
                }
            }
        }
        if let Some(f) = self.doubled {
            self.doubled = (f + 1 < DOUBLE_SHOW).then_some(f + 1);
        }
        if let Some(f) = self.drop_charge {
            // Charging while jump is held; let go, it is gone.
            self.drop_charge = input.jump.then_some(f + 1);
            if f + 1 == DROP_READY && input.jump {
                events.push(Event::Rev);
            }
        }
        if let Some(f) = self.shielding {
            self.shielding = (f + 1 < SHIELD_TIME).then_some(f + 1);
        }
        if let Some(f) = self.kicked {
            self.kicked = (f + 1 < KICK_SHOW).then_some(f + 1);
        }
        if let Some(t) = self.throwing {
            self.throwing = (t + 1 < THROW_SHOW).then_some(t + 1);
        }
        if let Some(f) = self.dashing {
            self.dashing = (f + 1 < DASH_TIME).then_some(f + 1);
        }
        if let Some(target) = self.homing {
            // Straight at it; if it takes too long (it went), off it goes.
            let to = target - self.pos;
            let dist = to.length();
            let f = self.dashing.unwrap_or(0);
            if dist < 6.0 || f >= HOMING_TIME {
                self.homing = None;
            } else {
                let v = to / dist * HOMING_SPEED;
                (self.xsp, self.ysp) = (v.x, v.y);
                self.facing = if v.x < 0.0 { -1.0 } else { 1.0 };
                self.dashing = Some(f + 1);
            }
        }
        if let Some(f) = self.warp {
            self.warp = (f + 1 < WARP_SHOW).then_some(f + 1);
        }
        if let Some(f) = self.flying {
            self.flying = Some(f + 1);
            if input.jump_pressed && f < self.fly_time && self.ysp >= FLAP_UNTIL {
                self.flap = true;
                events.push(Event::Flap);
            }
            if self.flap && self.ysp < FLAP_UNTIL {
                self.flap = false;
            }
        }
        if let Some(t) = self.hammer.filter(|_| self.hammer_air) {
            self.hammer = (t + 1 < HAMMER_AIR).then_some(t + 1);
        }
        if let Some(speed) = self.gliding {
            if !input.jump {
                // Let go: he drops, slowed down.
                self.gliding = None;
                self.dropping = true;
                self.xsp *= 0.25;
                return false;
            }
            // Turning round keeps some of the speed.
            let back = (input.left && self.facing > 0.0) || (input.right && self.facing < 0.0);
            let speed = if back {
                self.facing = -self.facing;
                (speed * 0.5).max(GLIDE_START)
            } else {
                (speed + GLIDE_ACC).min(GLIDE_TOP)
            };
            self.gliding = Some(speed);
            self.xsp = speed * self.facing;
            return false;
        }
        !self.dropping && !self.kicked.is_some_and(|f| f < KICK_LOCK)
    }

    /// Start one of his moves in the air; false if it cannot go now.
    fn start_move(&mut self, a: Ability, world: &PhysWorld, env: Env, jmp: f32, events: &mut Vec<Event>) -> bool {
        match a {
            Ability::Fly { seconds } if self.flying.is_none() => {
                self.jumping = false;
                self.flying = Some(0);
                self.fly_time = (seconds * FPS) as u32;
                self.flap = false;
                events.push(Event::Flap);
            }
            Ability::Glide { climb } if self.gliding.is_none() => {
                self.jumping = false;
                self.gliding = Some(GLIDE_START);
                self.glide_climb = climb;
                self.xsp = GLIDE_START * self.facing;
                self.ysp = self.ysp.max(0.0);
                events.push(Event::Glide);
            }
            Ability::Hammer if self.hammer.is_none() => {
                self.hammer = Some(0);
                self.hammer_air = true;
                self.jumping = true;
                events.push(Event::Hammer);
            }
            Ability::Teleport { distance } if !self.warped => self.teleport(world, env, distance, events),
            Ability::DoubleJump { power } => {
                self.ysp = self.ysp.min(-jmp * power);
                self.jumping = true;
                self.doubled = Some(0);
                events.push(Event::DoubleJump);
            }
            Ability::AirDash { speed } => self.air_dash(speed, events),
            Ability::Homing { range } => {
                let reach = range * SCALE;
                let target = self
                    .targets
                    .iter()
                    .copied()
                    .filter(|t| {
                        let d = *t - self.pos;
                        d.length() < reach && d.x * self.facing > -8.0 && d.y > -reach * 0.5
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
                self.xsp *= 0.2;
                self.ysp = speed;
                events.push(Event::Stomp);
            }
            Ability::Shoot { speed } => self.throw_one(speed, events),
            Ability::DropDash { speed } if self.drop_charge.is_none() => {
                self.drop_charge = Some(0);
                self.drop_speed = speed;
            }
            Ability::InstaShield if self.shielding.is_none() => {
                self.shielding = Some(0);
                self.jumping = true;
                events.push(Event::InstaShield);
            }
            Ability::Bounce { height } if self.bouncing.is_none() => {
                self.bouncing = Some(jmp * height * BOUNCE_UP);
                self.jumping = true;
                self.xsp *= 0.25;
                self.ysp = BOUNCE_DOWN;
                self.drop_charge = None;
                events.push(Event::Stomp);
            }
            Ability::WallJump { power } => {
                let Some(side) = self.wall_beside(world, env) else { return false };
                self.facing = -side;
                self.xsp = -side * WALL_KICK * power;
                self.ysp = -jmp * 0.9 * power;
                self.jumping = true;
                self.used = 0;
                self.kicked = Some(0);
                (self.dashing, self.homing, self.bouncing, self.drop_charge) = (None, None, None, None);
                events.push(Event::WallJump { side });
            }
            Ability::Boost { .. } => return self.start_boost(events),
            _ => return false,
        }
        true
    }

    /// Forward through the air, level, for a moment.
    fn air_dash(&mut self, speed: f32, events: &mut Vec<Event>) {
        self.xsp = self.facing * self.xsp.abs().max(speed);
        self.ysp = 0.0;
        self.jumping = true;
        self.dashing = Some(0);
        events.push(Event::AirDash);
    }

    /// Something thrown forward (the game makes it).
    fn throw_one(&mut self, speed: f32, events: &mut Vec<Event>) {
        let from = self.pos + vec2(self.facing * 10.0, -4.0) * SCALE;
        self.throwing = Some(0);
        events.push(Event::Shoot { from, dir: self.facing, speed: speed + self.xsp.abs().min(8.0) * 0.5 });
    }

    /// Sonic.exe vanishes and reappears up to `distance` ahead, short of a
    /// wall, keeping his speed (at least a run), with a little lift.
    fn teleport(&mut self, world: &PhysWorld, env: Env, distance: f32, events: &mut Vec<Event>) {
        let from = self.pos;
        let dir = vec2(self.facing, 0.0);
        let wr = self.radii().0;
        let room =
            self.cast(world, env, from, dir, distance + wr + 2.0).map_or(distance, |h| (h.dist - wr - 2.0).max(0.0));
        self.pos.x += self.facing * room * SCALE;
        self.xsp = self.facing * self.xsp.abs().max(4.0);
        if !self.grounded {
            self.ysp = self.ysp.min(-2.0);
        } else {
            self.gsp = self.xsp;
        }
        self.warped = true;
        self.warp = Some(0);
        events.push(Event::Teleport { from });
    }

    /// Gravity for this frame in the air (`k`: the world's gravity relative to Earth's).
    pub(super) fn fall(&self, grv: f32, k: f32) -> f32 {
        if self.homing.is_some() || self.dashing.is_some() {
            return self.ysp;
        }
        if let Some(speed) = self.stomping {
            return self.ysp.max(speed);
        }
        if self.bouncing.is_some() {
            return (self.ysp + grv).clamp(BOUNCE_DOWN, MAX_FALL);
        }
        if self.gliding.is_some() {
            let toward = if self.ysp < GLIDE_FALL { GLIDE_GRV } else { -GLIDE_GRV };
            return (self.ysp + toward * k.max(0.1)).clamp(-MAX_FALL, MAX_FALL);
        }
        if self.flying.is_some() {
            let g = if self.flap { FLAP } else { FLY_GRV };
            return (self.ysp + g * k).min(MAX_FALL);
        }
        (self.ysp + grv).min(MAX_FALL)
    }

    /// Landing ends the air moves; a glide turns into a slide.
    pub(super) fn land_moves(&mut self, events: &mut Vec<Event>) {
        let slide = self.gliding.is_some() && self.gsp.abs() > 0.5;
        if self.stomping.is_some() {
            events.push(Event::Stomped);
        }
        // A boost goes on along the ground.
        let boosting = self.boosting;
        self.end_moves();
        self.sliding = slide;
        self.boosting = boosting;
    }

    /// Gliding into a wall (`hit`, on side `d`), Knuckles grabs it.
    pub(super) fn grab(&mut self, hit: &Hit, d: f32, events: &mut Vec<Event>) -> bool {
        if self.gliding.is_none()
            || !self.glide_climb
            || self.xsp * d <= 0.0
            || hit.ground.mass > 0.0
            || hit.ground.bounce >= 0.6
            || hit.normal.x.abs() < 0.7
        {
            return false;
        }
        self.gliding = None;
        self.climbing = Some(d);
        self.facing = d;
        self.xsp = 0.0;
        self.ysp = 0.0;
        events.push(Event::Grab);
        true
    }

    /// Holding on to a wall: climb, pull up over the top, jump off or let go.
    pub(super) fn climb_frame(&mut self, world: &PhysWorld, env: Env, input: Input, events: &mut Vec<Event>) {
        let (wr, hr) = self.radii();
        if let Some((t, from, to)) = self.climb_up {
            let t = t + 1;
            let k = (t as f32 / CLIMB_UP as f32).min(1.0);
            let k = k * k * (3.0 - 2.0 * k);
            self.pos = from + (to - from) * k;
            if t < CLIMB_UP {
                self.climb_up = Some((t, from, to));
                return;
            }
            // On top: stand there (or fall if there is nothing after all).
            self.climb_up = None;
            self.grounded = true;
            self.angle = 0.0;
            self.gsp = 0.0;
            self.stick(world, env, events);
            return;
        }
        let Some(side) = self.climbing else { return };
        let dir = vec2(side, 0.0);
        if input.jump_pressed {
            self.climbing = None;
            self.jumping = true;
            self.facing = -side;
            self.xsp = -side * 4.0;
            self.ysp = -4.0;
            events.push(Event::Jump);
            return;
        }
        let dy = if input.up {
            -CLIMB
        } else if input.down {
            CLIMB
        } else {
            0.0
        };
        // `ysp` only tells the animation whether he is moving.
        self.ysp = dy;
        self.xsp = 0.0;
        let head = self.pos - vec2(0.0, hr * SCALE);
        if dy < 0.0 && self.cast(world, env, head, dir, PUSH + 3.0).is_none() {
            // Over the top: find where to stand.
            let probe = self.pos + vec2(side * (PUSH + wr) * SCALE, -(hr * 2.0 + 4.0) * SCALE);
            match self.cast(world, env, probe, vec2(0.0, 1.0), hr * 2.0 + 8.0) {
                Some(top) => {
                    let to = vec2(probe.x, top.point.y - hr * SCALE);
                    self.climb_up = Some((0, self.pos, to));
                }
                None => {
                    self.climbing = None;
                    self.dropping = true;
                }
            }
            return;
        }
        self.pos.y += dy * SCALE;
        if dy > 0.0 {
            if let Some(hit) = self.floor(world, env, vec2(0.0, 1.0), 1.0, 8.0).filter(|h| h.dist <= 0.5) {
                // Down at the floor: stand.
                self.pos.y += hit.dist * SCALE;
                self.climbing = None;
                self.grounded = true;
                self.gsp = 0.0;
                self.angle = angle_of(hit.normal);
                self.land_on(world, hit, events);
                return;
            }
        }
        match self.cast(world, env, self.pos, dir, PUSH + 3.0) {
            Some(hit) => self.pos.x += side * (hit.dist - PUSH) * SCALE,
            None => {
                // The wall ended under him: he lets go.
                self.climbing = None;
                self.dropping = true;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::WALL_T;
    use crate::physics::borders::BorderMode;

    const W: f32 = 3000.0;
    const H: f32 = 900.0;

    fn world() -> PhysWorld {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (W, H));
        w.refresh_queries();
        w
    }

    fn walls(_: ColliderHandle) -> Option<Ground> {
        Some(Ground::default())
    }

    fn floor_y() -> f32 {
        H - WALL_T * PPM
    }

    fn standing(c: Character, x: f32) -> Sonic {
        let hr = c.stand().1;
        let mut s = Sonic::as_character(vec2(x, floor_y() - hr * SCALE), c);
        s.grounded = true;
        s
    }

    fn run(s: &mut Sonic, w: &PhysWorld, input: Input, frames: usize) -> Vec<Event> {
        (0..frames).flat_map(|_| s.frame(w, &walls, input, None, 1.0)).collect()
    }

    fn jump_then(s: &mut Sonic, w: &PhysWorld, hold: Input, wait: usize) {
        let press = Input { jump: true, jump_pressed: true, ..hold };
        s.frame(w, &walls, press, None, 1.0);
        run(s, w, Input { jump: true, ..hold }, wait);
        s.frame(w, &walls, press, None, 1.0);
    }

    #[test]
    fn jumping_again_turns_him_super_when_he_can() {
        let w = world();
        // Not yet: a plain second press does nothing for Sonic.
        let mut s = standing(Character::Sonic, 500.0);
        jump_then(&mut s, &w, Input::default(), 10);
        assert!(!s.super_form);
        let mut s = standing(Character::Sonic, 500.0);
        s.can_super = true;
        jump_then(&mut s, &w, Input::default(), 10);
        assert!(s.super_form && !s.can_super);
        // Faster on the ground.
        let mut s = standing(Character::Sonic, 500.0);
        s.super_form = true;
        let right = Input { right: true, ..Input::default() };
        run(&mut s, &w, right, 120);
        assert!(s.gsp > 9.0, "super top speed, got {}", s.gsp);
        // Tails turns super before he flies.
        let mut s = standing(Character::Tails, 500.0);
        s.can_super = true;
        jump_then(&mut s, &w, Input::default(), 10);
        assert!(s.super_form && s.flying.is_none());
    }

    #[test]
    fn sonic_exe_teleports_once_a_jump_and_not_through_walls() {
        let w = world();
        let mut s = standing(Character::Exe, 500.0);
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        s.frame(&w, &walls, press, None, 1.0);
        run(&mut s, &w, Input { jump: true, ..Input::default() }, 8);
        let before = s.pos.x;
        let out = s.frame(&w, &walls, press, None, 1.0);
        assert!(out.iter().any(|e| matches!(e, Event::Teleport { .. })));
        assert!(s.pos.x - before > 70.0, "a blink forward: {}", s.pos.x - before);
        assert!(s.warped && s.warp.is_some());
        // Once a jump.
        let x = s.pos.x;
        let out = s.frame(&w, &walls, press, None, 1.0);
        assert!(!out.iter().any(|e| matches!(e, Event::Teleport { .. })));
        assert!((s.pos.x - x).abs() < 15.0);
        // Never into the wall at the world's right edge.
        let right = W - WALL_T * PPM;
        let mut s = standing(Character::Exe, right - 60.0);
        s.frame(&w, &walls, press, None, 1.0);
        run(&mut s, &w, Input { jump: true, ..Input::default() }, 8);
        s.frame(&w, &walls, press, None, 1.0);
        assert!(s.pos.x + s.radii().0 * SCALE <= right + 1.0, "{} past the wall at {right}", s.pos.x);
        assert_eq!(Character::ALL.len(), 5);
        assert_eq!(Character::Exe.cycle(1), Character::Sonic);
    }

    #[test]
    fn tails_flies_slowly_down_and_tires() {
        let w = world();
        let mut s = standing(Character::Tails, 500.0);
        jump_then(&mut s, &w, Input::default(), 20);
        assert!(s.flying.is_some() && !s.jumping, "flying after jumping again");
        // Hovering: past the top he falls far slower than he would.
        run(&mut s, &w, Input { jump: true, ..Input::default() }, 70);
        assert!(s.ysp > 0.0 && s.ysp < 1.5, "slow fall while flying ({})", s.ysp);
        // Flapping lifts him.
        let y = s.pos.y;
        let flap = Input { jump: true, jump_pressed: true, ..Input::default() };
        for _ in 0..12 {
            s.frame(&w, &walls, flap, None, 1.0);
            run(&mut s, &w, Input::default(), 3);
        }
        assert!(s.pos.y < y, "flapping lifts him ({y} → {})", s.pos.y);
        s.flying = Some(FLY_TIME);
        assert!(s.tired());
        run(&mut s, &w, Input::default(), 400);
        assert!(s.grounded && s.flying.is_none(), "landing ends the flight");
    }

    #[test]
    fn knuckles_jumps_lower_glides_and_slides() {
        let w = world();
        let mut sonic = standing(Character::Sonic, 500.0);
        let mut knux = standing(Character::Knuckles, 900.0);
        let jump = Input { jump: true, jump_pressed: true, ..Input::default() };
        let hold = Input { jump: true, ..Input::default() };
        sonic.frame(&w, &walls, jump, None, 1.0);
        knux.frame(&w, &walls, jump, None, 1.0);
        let (mut top_s, mut top_k) = (f32::MAX, f32::MAX);
        for _ in 0..60 {
            sonic.frame(&w, &walls, hold, None, 1.0);
            knux.frame(&w, &walls, hold, None, 1.0);
            top_s = top_s.min(sonic.pos.y);
            top_k = top_k.min(knux.pos.y);
        }
        assert!(top_k > top_s + 5.0, "Knuckles jumps lower ({top_k} vs {top_s})");

        let mut k = standing(Character::Knuckles, 500.0);
        jump_then(&mut k, &w, Input::default(), 15);
        assert!(k.gliding.is_some(), "gliding");
        run(&mut k, &w, hold, 30);
        assert!(k.xsp > GLIDE_START && k.ysp <= GLIDE_FALL + 0.2, "speeding up, sinking slowly");
        run(&mut k, &w, hold, 400);
        assert!(k.grounded, "glided down");
        assert!(k.sliding || k.gsp == 0.0, "slides on landing");
        run(&mut k, &w, Input::default(), 200);
        assert!(!k.sliding && k.gsp == 0.0, "the slide stops");
    }

    #[test]
    fn knuckles_climbs_a_wall_and_over_it() {
        let mut w = world();
        // A block 160 px high to the right.
        let (x0, y0) = to_phys(700.0, floor_y() - 160.0);
        let (x1, y1) = to_phys(900.0, floor_y());
        let body = w.bodies.insert(RigidBodyBuilder::fixed().translation(vector![(x0 + x1) / 2.0, (y0 + y1) / 2.0]));
        let col = ColliderBuilder::cuboid((x1 - x0) / 2.0, (y0 - y1).abs() / 2.0);
        w.colliders.insert_with_parent(col, body, &mut w.bodies);
        w.refresh_queries();
        let mut k = standing(Character::Knuckles, 560.0);
        let right = Input { right: true, ..Input::default() };
        jump_then(&mut k, &w, right, 6);
        run(&mut k, &w, Input { jump: true, ..right }, 60);
        assert_eq!(k.climbing, Some(1.0), "grabbed the wall");
        let y = k.pos.y;
        run(&mut k, &w, Input { up: true, ..Input::default() }, 10);
        assert!(k.pos.y < y, "climbs up");
        run(&mut k, &w, Input { up: true, ..Input::default() }, 200);
        assert!(k.grounded && k.climbing.is_none(), "pulled himself up");
        assert!(k.pos.x > 700.0 && k.pos.y < floor_y() - 160.0, "on top ({:?})", k.pos);
    }

    #[test]
    fn amy_swings_her_hammer() {
        let w = world();
        let mut a = standing(Character::Amy, 500.0);
        let events = run(&mut a, &w, Input { action_pressed: true, ..Input::default() }, 1);
        assert!(events.contains(&Event::Hammer) && a.hammer.is_some());
        run(&mut a, &w, Input::default(), HAMMER_HITS.0 as usize);
        assert!(a.attacking() && a.reach().is_some(), "the hammer hits in front of her");
        run(&mut a, &w, Input::default(), HAMMER as usize);
        assert!(a.hammer.is_none() && !a.attacking());
        jump_then(&mut a, &w, Input::default(), 10);
        assert!(a.hammer_air && a.attacking(), "spins it in the air");
        // Running, she swings without stopping.
        let mut a = standing(Character::Amy, 300.0);
        let right = Input { right: true, ..Input::default() };
        a.gsp = 6.0;
        run(&mut a, &w, Input { action_pressed: true, ..right }, 1);
        run(&mut a, &w, right, HAMMER as usize);
        assert!(a.gsp > 5.9, "kept her speed: {}", a.gsp);
    }

    /// Sonic with these moves.
    fn with(blocks: Vec<super::super::kit::Block>, x: f32) -> Sonic {
        let mut s = standing(Character::Sonic, x);
        s.kit.blocks = blocks;
        s
    }

    fn block(a: Ability, on: Button) -> super::super::kit::Block {
        super::super::kit::Block { ability: a, on }
    }

    #[test]
    fn a_second_jump_goes_higher_once_a_jump() {
        let w = world();
        let top = |s: &mut Sonic| {
            let mut top = s.pos.y;
            for _ in 0..200 {
                s.frame(&w, &walls, Input { jump: true, ..Input::default() }, None, 1.0);
                top = top.min(s.pos.y);
                if s.grounded {
                    break;
                }
            }
            floor_y() - top
        };
        let mut plain = standing(Character::Sonic, 500.0);
        plain.frame(&w, &walls, Input { jump: true, jump_pressed: true, ..Input::default() }, None, 1.0);
        let one = top(&mut plain);
        let mut d = with(vec![block(Ability::DoubleJump { power: 1.0 }, Button::JumpAgain)], 500.0);
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        d.frame(&w, &walls, press, None, 1.0);
        run(&mut d, &w, Input { jump: true, ..Input::default() }, 20);
        let events = d.frame(&w, &walls, press, None, 1.0);
        assert!(events.contains(&Event::DoubleJump) && d.doubled.is_some());
        // Again in the same jump: nothing more.
        run(&mut d, &w, Input { jump: true, ..Input::default() }, 2);
        assert!(!d.frame(&w, &walls, press, None, 1.0).contains(&Event::DoubleJump));
        let two = top(&mut d);
        assert!(two > one + 40.0, "higher: {two} against {one}");
    }

    #[test]
    fn dashes_stomps_and_throws() {
        let w = world();
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        // A dash through the air: fast and level for a moment.
        let mut d = with(vec![block(Ability::AirDash { speed: 10.0 }, Button::JumpAgain)], 500.0);
        d.frame(&w, &walls, press, None, 1.0);
        run(&mut d, &w, Input { jump: true, ..Input::default() }, 10);
        let y = d.pos.y;
        assert!(d.frame(&w, &walls, press, None, 1.0).contains(&Event::AirDash));
        run(&mut d, &w, Input { jump: true, ..Input::default() }, 6);
        assert!(d.xsp >= 10.0 && (d.pos.y - y).abs() < 2.0, "level at speed: {} {}", d.xsp, d.pos.y - y);
        // A stomp: straight down, and a thud on landing.
        let mut s = with(vec![block(Ability::Stomp { speed: 12.0 }, Button::Action)], 500.0);
        s.frame(&w, &walls, press, None, 1.0);
        run(&mut s, &w, Input { jump: true, ..Input::default() }, 12);
        assert!(s
            .frame(&w, &walls, Input { action_pressed: true, ..Input::default() }, None, 1.0)
            .contains(&Event::Stomp));
        assert!(s.ysp >= 12.0 && s.attacking());
        let events = run(&mut s, &w, Input::default(), 60);
        assert!(s.grounded && events.contains(&Event::Stomped));
        // A throw, standing or in the air.
        let mut t = with(vec![block(Ability::Shoot { speed: 9.0 }, Button::Action)], 500.0);
        let events = t.frame(&w, &walls, Input { action_pressed: true, ..Input::default() }, None, 1.0);
        assert!(events.iter().any(|e| matches!(e, Event::Shoot { dir, speed, .. } if *dir == 1.0 && *speed >= 9.0)));
        assert!(t.throwing.is_some());
    }

    #[test]
    fn a_homing_attack_goes_for_the_nearest_thing_ahead() {
        let w = world();
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        let mut h = with(vec![block(Ability::Homing { range: 200.0 }, Button::JumpAgain)], 500.0);
        let target = vec2(500.0 + 150.0, floor_y() - 150.0);
        h.targets = vec![vec2(500.0 - 100.0, floor_y() - 120.0), target];
        h.frame(&w, &walls, press, None, 1.0);
        run(&mut h, &w, Input { jump: true, ..Input::default() }, 8);
        assert!(h.frame(&w, &walls, press, None, 1.0).contains(&Event::AirDash));
        assert_eq!(h.homing, Some(target), "the one ahead, not the one behind");
        for _ in 0..40 {
            h.frame(&w, &walls, Input::default(), None, 1.0);
            if (h.pos - target).length() < 20.0 {
                break;
            }
        }
        assert!((h.pos - target).length() < 20.0, "got there: {:?}", h.pos);
        // Hitting it: up off it, ready to go again.
        h.rebound();
        assert!(h.homing.is_none() && h.ysp < 0.0 && h.used == 0);
        // Nothing in reach: a dash forward instead.
        let mut h = with(vec![block(Ability::Homing { range: 200.0 }, Button::JumpAgain)], 500.0);
        h.frame(&w, &walls, press, None, 1.0);
        run(&mut h, &w, Input { jump: true, ..Input::default() }, 8);
        h.frame(&w, &walls, press, None, 1.0);
        assert!(h.homing.is_none() && h.dashing.is_some() && h.xsp > 8.0);
    }

    #[test]
    fn moves_mix_in_order_and_physics_scale() {
        let w = world();
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        // Jump again: first the double jump, then the glide.
        let mut m = with(
            vec![
                block(Ability::DoubleJump { power: 0.8 }, Button::JumpAgain),
                block(Ability::Glide { climb: false }, Button::JumpAgain),
            ],
            500.0,
        );
        m.frame(&w, &walls, press, None, 1.0);
        run(&mut m, &w, Input { jump: true, ..Input::default() }, 10);
        assert!(m.frame(&w, &walls, press, None, 1.0).contains(&Event::DoubleJump));
        run(&mut m, &w, Input { jump: true, ..Input::default() }, 10);
        assert!(m.frame(&w, &walls, press, None, 1.0).contains(&Event::Glide));
        // A glide that does not climb lets go of walls.
        assert!(!m.glide_climb);
        // Faster, by the kit.
        let mut f = standing(Character::Sonic, 300.0);
        f.kit.physics.speed = 1.5;
        f.kit.physics.accel = 2.0;
        run(&mut f, &w, Input { right: true, ..Input::default() }, 110);
        assert!((f.gsp - 9.0).abs() < 0.05, "top speed ×1.5: {}", f.gsp);
        // No spin dash in this kit.
        let mut n = standing(Character::Sonic, 500.0);
        n.kit.spin_dash = false;
        run(&mut n, &w, Input { down: true, ..Input::default() }, 2);
        n.frame(&w, &walls, Input { down: true, jump: true, jump_pressed: true, ..Input::default() }, None, 1.0);
        assert!(n.spindash.is_none());
    }

    #[test]
    fn a_drop_dash_charged_in_the_air_rolls_off_on_landing() {
        let w = world();
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        let hold = Input { jump: true, ..Input::default() };
        let mut d = with(vec![block(Ability::DropDash { speed: 8.0 }, Button::JumpAgain)], 500.0);
        d.frame(&w, &walls, press, None, 1.0);
        run(&mut d, &w, hold, 6);
        d.frame(&w, &walls, press, None, 1.0);
        assert!(d.drop_charge.is_some() && !d.drop_ready());
        let mut events = vec![];
        for _ in 0..200 {
            events.extend(d.frame(&w, &walls, hold, None, 1.0));
            if d.grounded {
                break;
            }
        }
        assert!(d.grounded && d.rolling && d.gsp >= 8.0, "off rolling: {} {}", d.rolling, d.gsp);
        assert!(events.contains(&Event::Dash));
        // Let go of jump before landing: no dash.
        let mut d = with(vec![block(Ability::DropDash { speed: 8.0 }, Button::JumpAgain)], 500.0);
        d.frame(&w, &walls, press, None, 1.0);
        run(&mut d, &w, hold, 6);
        d.frame(&w, &walls, press, None, 1.0);
        run(&mut d, &w, hold, DROP_READY as usize + 2);
        assert!(d.drop_ready());
        run(&mut d, &w, Input::default(), 1);
        assert!(d.drop_charge.is_none());
        run(&mut d, &w, Input::default(), 200);
        assert!(d.grounded && !d.rolling && d.gsp.abs() < 1.0);
    }

    #[test]
    fn a_peel_out_revs_standing_and_goes_off_running() {
        let w = world();
        let peel = || with(vec![block(Ability::PeelOut { speed: 12.0 }, Button::Action)], 500.0);
        let held = Input { action: true, ..Input::default() };
        let mut p = peel();
        p.frame(&w, &walls, Input { action_pressed: true, ..held }, None, 1.0);
        let x = p.pos.x;
        run(&mut p, &w, held, PEEL_READY as usize + 5);
        assert!(p.peel_ready() && (p.pos.x - x).abs() < 0.5, "revs where he stands");
        assert_eq!(p.poses()[0], "peel_out");
        let events = run(&mut p, &w, Input::default(), 1);
        assert!(events.contains(&Event::Dash) && p.gsp >= 11.5 && !p.rolling, "off running: {}", p.gsp);
        // Too short a rev: nothing.
        let mut p = peel();
        p.frame(&w, &walls, Input { action_pressed: true, ..held }, None, 1.0);
        run(&mut p, &w, held, 8);
        run(&mut p, &w, Input::default(), 1);
        assert!(p.gsp.abs() < 0.5 && p.peeling.is_none());
        // Turning round while revving goes the other way.
        let mut p = peel();
        p.frame(&w, &walls, Input { action_pressed: true, ..held }, None, 1.0);
        run(&mut p, &w, Input { left: true, ..held }, PEEL_READY as usize + 1);
        run(&mut p, &w, Input::default(), 1);
        assert!(p.gsp <= -11.5);
    }

    #[test]
    fn an_insta_shield_reaches_further_for_a_moment() {
        let w = world();
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        let mut s = with(vec![block(Ability::InstaShield, Button::JumpAgain)], 500.0);
        s.frame(&w, &walls, press, None, 1.0);
        run(&mut s, &w, Input { jump: true, ..Input::default() }, 6);
        assert!(s.reach().is_none());
        assert!(s.frame(&w, &walls, press, None, 1.0).contains(&Event::InstaShield));
        let (c, half) = s.reach().expect("a reach");
        assert!(c == s.pos && half.x >= SHIELD_REACH * SCALE && s.shielded());
        run(&mut s, &w, Input { jump: true, ..Input::default() }, SHIELD_TIME as usize);
        assert!(!s.shielded() && s.reach().is_none(), "over");
        // Once a jump.
        assert!(!s.frame(&w, &walls, press, None, 1.0).contains(&Event::InstaShield));
    }

    #[test]
    fn a_bounce_goes_down_and_back_up_again_and_again() {
        let w = world();
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        let action = Input { action: true, action_pressed: true, ..Input::default() };
        let mut b = with(vec![block(Ability::Bounce { height: 1.0 }, Button::Action)], 500.0);
        b.frame(&w, &walls, press, None, 1.0);
        run(&mut b, &w, Input { jump: true, ..Input::default() }, 10);
        assert!(b.frame(&w, &walls, action, None, 1.0).contains(&Event::Stomp));
        assert!(b.ysp >= BOUNCE_DOWN && b.bouncing.is_some());
        let mut events = vec![];
        for _ in 0..80 {
            events.extend(b.frame(&w, &walls, Input::default(), None, 1.0));
            if events.contains(&Event::Bounce) {
                break;
            }
        }
        assert!(events.contains(&Event::Bounce) && !b.grounded && b.ysp < -6.0, "up again: {}", b.ysp);
        // Up higher than a jump, and ready to bounce again.
        let mut top = b.pos.y;
        for _ in 0..30 {
            b.frame(&w, &walls, Input::default(), None, 1.0);
            top = top.min(b.pos.y);
        }
        assert!(floor_y() - top > 100.0, "{}", floor_y() - top);
        assert!(b.frame(&w, &walls, action, None, 1.0).contains(&Event::Stomp));
    }

    #[test]
    fn a_wall_jump_kicks_off_walls_and_else_lets_the_next_move_go() {
        let w = world();
        let right_wall = W - WALL_T * PPM;
        let press = Input { jump: true, jump_pressed: true, ..Input::default() };
        let right = Input { right: true, ..Input::default() };
        let moves = vec![
            block(Ability::WallJump { power: 1.0 }, Button::JumpAgain),
            block(Ability::DoubleJump { power: 0.8 }, Button::JumpAgain),
        ];
        let mut k = with(moves.clone(), right_wall - 20.0);
        k.frame(&w, &walls, Input { right: true, ..press }, None, 1.0);
        run(&mut k, &w, Input { jump: true, ..right }, 12);
        let events = k.frame(&w, &walls, Input { right: true, ..press }, None, 1.0);
        assert!(events.iter().any(|e| matches!(e, Event::WallJump { side } if *side == 1.0)));
        assert!(k.xsp < -3.0 && k.ysp < -4.0 && k.facing == -1.0, "off it: {} {}", k.xsp, k.ysp);
        // Kicking off again, the other moves are ready again.
        assert_eq!(k.used, 0);
        // No wall: the double jump instead.
        let mut k = with(moves, 500.0);
        k.frame(&w, &walls, press, None, 1.0);
        run(&mut k, &w, Input { jump: true, ..Input::default() }, 12);
        let events = k.frame(&w, &walls, press, None, 1.0);
        assert!(events.contains(&Event::DoubleJump) && !events.iter().any(|e| matches!(e, Event::WallJump { .. })));
    }

    #[test]
    fn a_boost_runs_fast_while_held_and_fills_again_slowly() {
        let w = world();
        let mut b = with(vec![block(Ability::Boost { seconds: 1.0 }, Button::Action)], 500.0);
        let held = Input { action: true, ..Input::default() };
        assert_eq!(b.boost_gauge(), Some(1.0));
        let events = run(&mut b, &w, Input { action_pressed: true, ..held }, 1);
        assert!(events.contains(&Event::Boost) && b.boosting && b.gsp >= BOOST);
        assert!(b.attacking(), "breaks what it meets");
        run(&mut b, &w, held, 20);
        assert!(b.gsp >= BOOST && b.boost_gauge().unwrap() < 0.7);
        // Let go: it stops, and fills again (slowly).
        run(&mut b, &w, Input::default(), 1);
        assert!(!b.boosting);
        let g = b.boost_gauge().unwrap();
        run(&mut b, &w, Input::default(), 30);
        let g2 = b.boost_gauge().unwrap();
        assert!(g2 > g && g2 < g + 0.25, "{g} → {g2}");
        // Held until it is gone: it stops, and cannot start again at once.
        run(&mut b, &w, Input { action_pressed: true, ..held }, 1);
        run(&mut b, &w, held, 80);
        assert!(!b.boosting && b.boost_left < BOOST_LEAST);
        run(&mut b, &w, Input::default(), 1);
        run(&mut b, &w, Input { action_pressed: true, ..held }, 1);
        assert!(!b.boosting);
        // In the air too, falling slowly.
        let mut b = with(vec![block(Ability::Boost { seconds: 3.0 }, Button::Action)], 500.0);
        b.frame(&w, &walls, Input { jump: true, jump_pressed: true, ..Input::default() }, None, 1.0);
        run(&mut b, &w, Input::default(), 30);
        run(&mut b, &w, Input { action_pressed: true, ..held }, 1);
        run(&mut b, &w, held, 10);
        assert!(b.boosting && b.xsp >= BOOST && b.ysp <= BOOST_FALL + 0.3);
    }
}
