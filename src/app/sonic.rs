//! Sonic in the sandbox: dropped in with Shift+O, played with the arrows
//! and Space, pushing and riding the objects, collecting rings, followed
//! by the camera, and picked up and thrown with the pointer.

use super::*;
use crate::audio::sfx::Sound;
use crate::sonic::{art, Event, Ground, Input as Pad, Sonic, FPS, MASS, SCALE};
use rapier2d::prelude::{nalgebra, vector, ColliderHandle};

/// Most frames run to catch up after a slow one.
const MAX_FRAMES: f32 = 4.0;
/// How quickly the camera catches up with him (per frame).
const FOLLOW: f32 = 0.12;

pub(super) struct SonicRun {
    pub sonic: Sonic,
    /// Simulated time not yet run (s).
    acc: f32,
    /// Jump pressed since the last frame.
    pub jump: bool,
    pub rings: u32,
    /// Held by the pointer: where it was last frame.
    held: Option<Vec2>,
}

impl App {
    /// Shift+O: Sonic joins at `at`, or leaves.
    pub(super) fn toggle_sonic(&mut self, at: Vec2) {
        if self.challenge.is_some() {
            return;
        }
        if self.sonic.take().is_some() {
            self.toasts.status("sonic", "Sonic left  ·  Shift+O brings him back");
            return;
        }
        self.spawn_sonic(at);
        self.toasts.status(
            "sonic",
            "Sonic  ·  ← → run  ·  ↓ roll  ·  Space jump  ·  ↓ + Space spin dash  ·  Shift+Space pause",
        );
    }

    pub(super) fn spawn_sonic(&mut self, at: Vec2) {
        let sonic = Sonic::new(at);
        self.sonic = Some(SonicRun { sonic, acc: 0.0, jump: false, rings: 0, held: None });
    }

    /// What every collider is to Sonic.
    fn sonic_grounds(&self) -> HashMap<ColliderHandle, Ground> {
        let mut map = HashMap::new();
        for o in &self.objects {
            let mass = self.world.bodies.get(o.body).filter(|b| b.is_dynamic()).map_or(0.0, |b| b.mass());
            let m = &o.material;
            map.insert(o.collider, Ground { mass, bounce: m.bounce, friction: m.friction, layer: m.layer });
        }
        map
    }

    /// Run Sonic for `step` simulated seconds (after the physics step).
    pub(super) fn update_sonic(&mut self, step: f32) {
        if self.sonic.is_none() {
            return;
        }
        let grounds = self.sonic_grounds();
        let grains: HashSet<RigidBodyHandle> = self.grains.bodies().collect();
        let world = &self.world;
        let env = |c: ColliderHandle| {
            if let Some(g) = grounds.get(&c) {
                return Some(*g);
            }
            let body = world.colliders.get(c)?.parent();
            match body.and_then(|b| world.bodies.get(b).map(|r| (b, r))) {
                Some((b, _)) if grains.contains(&b) => None,
                Some((_, r)) if r.is_dynamic() => Some(Ground { mass: r.mass(), ..Ground::default() }),
                _ => Some(Ground::default()),
            }
        };
        let typing = self.editor_bar.typing();
        let key = |k| !typing && is_key_down(k);
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let water = self.s.water.then(|| {
            let rest = water::rest_level(world, self.s.water_level);
            crate::physics::to_screen(0.0, rest).y
        });
        let gravity = -self.s.gravity / 9.8;
        let Some(run) = &mut self.sonic else { return };
        let mut pad = Pad {
            left: key(KeyCode::Left),
            right: key(KeyCode::Right),
            up: key(KeyCode::Up),
            down: key(KeyCode::Down),
            jump: key(KeyCode::Space) && !shift,
            jump_pressed: run.jump,
        };
        let mut events = vec![];
        if run.held.is_none() {
            run.acc = (run.acc + step).min(MAX_FRAMES / FPS);
            while run.acc >= 1.0 / FPS {
                run.acc -= 1.0 / FPS;
                events.extend(run.sonic.frame(world, &env, pad, water, gravity));
                pad.jump_pressed = false;
                run.jump = false;
            }
        }
        let s = run.sonic.clone();
        // Fell out of the world: back to the start.
        let (aw, ah) = self.arena();
        if s.pos.x < -300.0 || s.pos.x > aw + 300.0 || s.pos.y > ah + 300.0 || s.pos.y < -1500.0 {
            if let Some(run) = &mut self.sonic {
                run.sonic.respawn();
            }
        }
        self.sonic_events(&s, &events, step);
        self.collect_rings(&s);
        if self.pan_last.is_none() {
            let screen = vec2(screen_width(), screen_height());
            self.camera.follow(s.pos, screen, FOLLOW);
        }
    }

    fn sonic_events(&mut self, s: &Sonic, events: &[Event], step: f32) {
        let feet = s.pos + vec2(0.0, s.radii().1 * SCALE);
        let g = -self.s.gravity;
        for e in events {
            match *e {
                Event::Push { collider, at, impulse } => {
                    let Some(h) = self.world.colliders.get(collider).and_then(|c| c.parent()) else { continue };
                    let Some(b) = self.world.bodies.get_mut(h).filter(|b| b.is_dynamic()) else { continue };
                    let i = vector![impulse.x, -impulse.y] * SCALE * FPS / PPM;
                    let (x, y) = to_phys(at.x, at.y);
                    b.apply_impulse_at_point(i, Point::new(x, y), true);
                }
                Event::Stand { collider, at } => {
                    let Some(h) = self.world.colliders.get(collider).and_then(|c| c.parent()) else { continue };
                    let Some(b) = self.world.bodies.get_mut(h).filter(|b| b.is_dynamic()) else { continue };
                    let (x, y) = to_phys(at.x, at.y);
                    b.apply_impulse_at_point(vector![0.0, -MASS * g.max(0.0) / FPS], Point::new(x, y), true);
                }
                Event::Jump => self.sound(Sound::Jump, s.pos, 0.5),
                Event::Roll => self.sound(Sound::Dash, s.pos, 0.25),
                Event::Rev => {
                    let rev = s.spindash.unwrap_or(0.0);
                    self.sound(Sound::Rev, s.pos, 0.35 + rev * 0.04);
                }
                Event::Dash => self.sound(Sound::Dash, s.pos, 0.6),
                Event::Skid => self.sound(Sound::Skid, feet, 0.5),
                Event::Spring => self.sound(Sound::Spring, s.pos, 0.6),
                Event::Splash => {
                    self.effects.splash(s.pos, 0.8);
                }
                Event::Land => {
                    if self.s.effects && s.gsp.abs() > 4.0 {
                        self.effects.dust(feet, s.gsp.abs());
                    }
                }
            }
        }
        if s.skidding && self.s.effects && rand::gen_range(0.0, 1.0) < step * 30.0 {
            self.effects.dust(feet - vec2(s.facing * 6.0, 0.0), 6.0);
        }
        if s.spindash.is_some() && self.s.effects && rand::gen_range(0.0, 1.0) < step * 20.0 {
            self.effects.dust(feet - vec2(s.facing * 16.0, 0.0), 5.0);
        }
    }

    fn collect_rings(&mut self, s: &Sonic) {
        let (wr, hr) = s.radii();
        let reach = vec2(wr * SCALE + 10.0, hr * SCALE + 10.0);
        let (got, left): (Vec<Vec2>, Vec<Vec2>) = std::mem::take(&mut self.rings)
            .into_iter()
            .partition(|r| (r.x - s.pos.x).abs() < reach.x && (r.y - s.pos.y).abs() < reach.y);
        self.rings = left;
        for r in &got {
            if self.s.effects {
                self.effects.sparks(*r, vec2(0.0, -1.0), 6.0);
            }
        }
        if let Some(at) = got.first() {
            self.sound(Sound::Ring, *at, 0.5);
            if let Some(run) = &mut self.sonic {
                run.rings += got.len() as u32;
            }
            if self.rings.is_empty() {
                self.toasts.success("Every ring collected!");
            }
        }
    }

    /// The pointer picks him up (Spring, Slingshot or Swing tool): true when it did.
    pub(super) fn grab_sonic(&mut self, m: Vec2) -> bool {
        let Some(run) = &mut self.sonic else { return false };
        let (wr, hr) = run.sonic.radii();
        let d = m - run.sonic.pos;
        if d.x.abs() > wr * SCALE + 8.0 || d.y.abs() > hr * SCALE + 8.0 {
            return false;
        }
        run.held = Some(m);
        true
    }

    /// While held he follows the pointer; let go, he flies on.
    pub(super) fn drag_sonic(&mut self, m: Vec2, down: bool, dt: f32) {
        let Some(run) = &mut self.sonic else { return };
        let Some(last) = run.held else { return };
        let v = if dt > 0.0 { (m - last) / dt } else { Vec2::ZERO };
        run.sonic.throw(m, v.clamp_length_max(1400.0));
        run.held = down.then_some(m);
    }

    /// Rings and Sonic (world pass).
    pub(super) fn draw_sonic(&self) {
        let t = get_time() as f32;
        for (k, r) in self.rings.iter().enumerate() {
            art::ring(*r, t + k as f32 * 0.07, SCALE);
        }
        if let Some(run) = &self.sonic {
            art::draw(&run.sonic);
        }
    }

    /// Ring counter (screen pass).
    pub(super) fn draw_sonic_hud(&self, top: f32) {
        let Some(run) = &self.sonic else { return };
        let gold = Color::new(1.0, 0.85, 0.2, 1.0);
        let shade = Color::new(0.0, 0.0, 0.0, 0.6);
        let line = format!("RINGS  {}", run.rings);
        theme::text_bold(&line, 18.0, top + 26.0, 20.0, shade);
        theme::text_bold(&line, 16.0, top + 24.0, 20.0, gold);
        if !self.rings.is_empty() {
            let left = format!("{} left", self.rings.len());
            theme::text(&left, 16.0, top + 44.0, 13.0, Color::new(1.0, 1.0, 1.0, 0.7));
        }
    }
}

impl App {
    /// A bomb going off near Sonic blows him away.
    pub(super) fn blast_sonic(&mut self, at: Vec2, radius: f32, strength: f32) {
        let Some(run) = &mut self.sonic else { return };
        let d = run.sonic.pos - at;
        let dist = d.length();
        if dist > radius {
            return;
        }
        let dir = if dist > 1.0 { d / dist } else { vec2(0.0, -1.0) };
        let kick = (300.0 + strength * 2.0) * (1.0 - dist / radius).max(0.3);
        let v = run.sonic.velocity_px() + (dir + vec2(0.0, -0.4)) * kick;
        let pos = run.sonic.pos;
        run.sonic.throw(pos, v);
        run.sonic.jumping = true;
    }
}
