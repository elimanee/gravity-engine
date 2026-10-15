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

use super::*;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum Character {
    #[default]
    Sonic,
    Tails,
    Knuckles,
    Amy,
}

impl Character {
    pub const ALL: [Character; 4] = [Character::Sonic, Character::Tails, Character::Knuckles, Character::Amy];

    pub fn label(self) -> &'static str {
        match self {
            Character::Sonic => "Sonic",
            Character::Tails => "Tails",
            Character::Knuckles => "Knuckles",
            Character::Amy => "Amy",
        }
    }

    /// The next one (or the one before).
    pub fn cycle(self, dir: i32) -> Self {
        let i = Self::ALL.iter().position(|&c| c == self).unwrap_or(0) as i32;
        Self::ALL[(i + dir).rem_euclid(Self::ALL.len() as i32) as usize]
    }

    /// What each one does on top of running, rolling and jumping.
    pub fn moves(self) -> &'static str {
        match self {
            Character::Sonic => "↓ + Space spin dash",
            Character::Tails => "Space in the air: fly (tap to flap)",
            Character::Knuckles => "Space in the air: glide  ·  into a wall: climb ↑ ↓",
            Character::Amy => "X: hammer  ·  Space in the air: hammer spin",
        }
    }

    /// Half width and half height standing (guide px).
    pub(super) fn stand(self) -> (f32, f32) {
        match self {
            Character::Tails => (9.0, 15.0),
            Character::Amy => (9.0, 16.0),
            _ => STAND,
        }
    }
}

/// Tails' gravity while flying, the lift of a flap (until he rises at
/// [`FLAP_UNTIL`]), and how long he can fly (frames).
const FLY_GRV: f32 = 0.03125;
const FLAP: f32 = -0.125;
const FLAP_UNTIL: f32 = -1.0;
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

impl Sonic {
    /// Turn into `c` where he is, feet staying on the ground.
    pub fn become_character(&mut self, c: Character) {
        let before = self.radii().1;
        self.character = c;
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
    }

    /// Jump speed for this character.
    pub(super) fn jump_speed(&self, c: Consts) -> f32 {
        if self.character == Character::Knuckles {
            c.jmp - 0.5
        } else {
            c.jmp
        }
    }

    /// The hammer is out and hitting.
    pub(super) fn hammer_hits(&self) -> bool {
        match self.hammer {
            Some(t) if !self.hammer_air => (HAMMER_HITS.0..=HAMMER_HITS.1).contains(&t),
            Some(_) => true,
            None => false,
        }
    }

    /// The box the hammer hits beyond his own (centre, half size; world px).
    pub fn reach(&self) -> Option<(Vec2, Vec2)> {
        if !self.hammer_hits() || self.hammer_air {
            return None;
        }
        let half = vec2(HAMMER_REACH / 2.0 + 4.0, 16.0) * SCALE;
        Some((self.pos + vec2(self.facing * (HAMMER_REACH / 2.0 + 6.0), 0.0) * SCALE, half))
    }

    /// Animation speed for a move (frames each picture is held), if one is going on.
    pub(super) fn move_hold(&self) -> Option<f32> {
        if self.flying.is_some() {
            Some(if self.flying.is_some_and(|f| f >= FLY_TIME) { 6.0 } else { 1.0 })
        } else if self.climbing.is_some() {
            Some(if self.ysp == 0.0 { f32::INFINITY } else { 5.0 })
        } else if self.climb_up.is_some() {
            Some(5.0)
        } else if self.hammer.is_some() {
            Some(if self.hammer_air { 1.0 } else { 3.0 })
        } else if self.gliding.is_some() || self.sliding {
            Some(f32::INFINITY)
        } else {
            None
        }
    }

    /// Tired of flying.
    pub fn tired(&self) -> bool {
        self.flying.is_some_and(|f| f >= FLY_TIME)
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
        if self.character == Character::Amy
            && input.action_pressed
            && self.hammer.is_none()
            && !self.rolling
            && self.spindash.is_none()
        {
            self.hammer = Some(0);
            self.hammer_air = false;
            self.crouching = false;
            self.looking_up = false;
            events.push(Event::Hammer);
        }
        if let Some(t) = self.hammer.filter(|_| !self.hammer_air) {
            self.hammer = (t + 1 < HAMMER).then_some(t + 1);
            if self.hammer_hits() {
                self.hammer_smash(world, env, events);
            }
            // She stops to swing.
            self.gsp -= self.gsp.abs().min(0.25) * self.gsp.signum();
            return Input { left: false, right: false, up: false, down: false, jump_pressed: false, ..input };
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

    /// In the air: jumping again starts a character's move; moves going on
    /// carry on. Returns whether he steers as usual.
    pub(super) fn air_moves(&mut self, input: Input, events: &mut Vec<Event>) -> bool {
        let again = input.jump_pressed && self.jumping && !self.hurt;
        match self.character {
            Character::Tails if again && self.flying.is_none() => {
                self.jumping = false;
                self.flying = Some(0);
                self.flap = false;
                events.push(Event::Flap);
            }
            Character::Knuckles if again && self.gliding.is_none() => {
                self.jumping = false;
                self.gliding = Some(GLIDE_START);
                self.xsp = GLIDE_START * self.facing;
                self.ysp = self.ysp.max(0.0);
                events.push(Event::Glide);
            }
            Character::Amy if (again || input.action_pressed) && self.hammer.is_none() => {
                self.hammer = Some(0);
                self.hammer_air = true;
                self.jumping = true;
                events.push(Event::Hammer);
            }
            _ => {}
        }
        if let Some(f) = self.flying {
            self.flying = Some(f + 1);
            if input.jump_pressed && f < FLY_TIME && self.ysp >= FLAP_UNTIL {
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
        !self.dropping
    }

    /// Gravity for this frame in the air (`k`: the world's gravity relative to Earth's).
    pub(super) fn fall(&self, grv: f32, k: f32) -> f32 {
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
    pub(super) fn land_moves(&mut self) {
        let slide = self.gliding.is_some() && self.gsp.abs() > 0.5;
        self.end_moves();
        self.sliding = slide;
    }

    /// Gliding into a wall (`hit`, on side `d`), Knuckles grabs it.
    pub(super) fn grab(&mut self, hit: &Hit, d: f32, events: &mut Vec<Event>) -> bool {
        if self.gliding.is_none()
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
    }
}
