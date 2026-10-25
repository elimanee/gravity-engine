//! Sonic in the sandbox: dropped in with Shift+O, played with the arrows
//! and Space, pushing and riding the objects, collecting rings, followed
//! by the camera, and picked up and thrown with the pointer.

use super::*;
use crate::audio::sfx::Sound;
use crate::sonic::sheet::{Sheet, Sheets};
use crate::sonic::{art, Character, Event, Ground, Input as Pad, Sonic, FPS, MASS, SCALE};
use rapier2d::prelude::{nalgebra, vector, ColliderHandle};

/// Most frames run to catch up after a slow one.
const MAX_FRAMES: f32 = 4.0;
/// How quickly the camera catches up with him (per frame).
const FOLLOW: f32 = 0.12;

pub(super) struct SonicRun {
    pub sonic: Sonic,
    /// Simulated time not yet run (s).
    acc: f32,
    /// Jump and the action button pressed since the last frame.
    pub jump: bool,
    pub action: bool,
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
        self.character_toast();
    }

    fn character_toast(&mut self) {
        let c = self.s.character;
        let msg =
            format!("{}  ·  ← → run  ·  ↓ roll  ·  Space jump  ·  {}  ·  Shift+N next character", c.label(), c.moves());
        self.toasts.status("sonic", msg);
    }

    /// Play as `c` (the one in play changes on the spot).
    pub(super) fn set_character(&mut self, c: Character) {
        self.s.character = c;
        match &mut self.sonic {
            Some(run) => {
                run.sonic.become_character(c);
                self.character_toast();
            }
            None => self.toasts.status("sonic", format!("{} will play  ·  Shift+O to start", c.label())),
        }
    }

    pub(super) fn spawn_sonic(&mut self, at: Vec2) {
        if !self.sonic_sheet_tried {
            self.sonic_sheet_tried = true;
            self.load_sonic_sheet();
        }
        let sonic = Sonic::as_character(at, self.s.character);
        self.sonic = Some(SonicRun { sonic, acc: 0.0, jump: false, action: false, rings: 0, held: None });
    }

    /// (Re)load the sprite sheets from the Sonic folder: the characters', and the objects'.
    pub(super) fn load_sonic_sheet(&mut self) {
        let (sheets, errors) = Sheets::load();
        self.sonic_sheet = sheets;
        for e in errors {
            self.toasts.warn(format!("Sprite sheet not used: {e}"));
        }
        match crate::game::sprites::Sprites::load() {
            Ok(sprites) => self.game_sprites = sprites,
            Err(e) => {
                self.game_sprites = None;
                self.toasts.warn(format!("Object sprites not used: {e}"));
            }
        }
    }

    /// Pick a sprite sheet (PNG, with its sheet.json if it has one).
    pub(super) fn pick_sonic_sheet(&mut self) {
        let filter = FileDialog::new().add_filter("Sprite sheet", &["png", "PNG", "webp", "WEBP"]);
        let Some(png) = filter.pick_file() else { return };
        let installed = match Sheet::install(&png) {
            Ok(name) => name,
            Err(e) => {
                self.toasts.error(format!("Could not copy the sheet: {e}"));
                return;
            }
        };
        self.sonic_sheet_tried = true;
        self.load_sonic_sheet();
        if let Some(c) = Character::ALL
            .into_iter()
            .find(|&c| self.sonic_sheet.get(c).is_some_and(|s| s.name == installed) && c != Character::Sonic)
        {
            self.toasts.success(format!("{} now uses {installed}", c.label()));
            return;
        }
        if installed != "sheet.png" {
            if let Some(sp) = &self.game_sprites {
                let msg = format!("The game's objects now use the {}", sp.describe());
                self.toasts.success(msg);
            }
            return;
        }
        if let Some(sheet) = self.sonic_sheet.get(Character::Sonic) {
            let msg = format!("Sonic now uses {}", sheet.name);
            self.toasts.success(msg);
            if self.sonic.is_none() && self.challenge.is_none() {
                let at = self.spawn_point(screen_width() / 2.0);
                self.spawn_sonic(at);
            }
        }
    }

    /// What every collider is to Sonic.
    fn sonic_grounds(&self) -> HashMap<ColliderHandle, Ground> {
        let mut map = HashMap::new();
        for o in &self.objects {
            let mass = self.world.bodies.get(o.body).filter(|b| b.is_dynamic()).map_or(0.0, |b| b.mass());
            let m = &o.material;
            let conveyor = m.conveyor * PPM / (SCALE * FPS);
            let g = Ground {
                mass,
                bounce: m.bounce,
                friction: m.friction,
                layer: m.layer,
                breakable: m.breakable,
                conveyor,
            };
            map.insert(o.collider, g);
        }
        if let Some(game) = &self.game {
            map.extend(game.grounds());
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
        let typing = self.editor_bar.typing() || self.level_bar.typing();
        let key = |k| !typing && is_key_down(k);
        let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
        let water = self.s.water.then(|| {
            let rest = water::rest_level(world, self.s.water_level);
            crate::physics::to_screen(0.0, rest).y
        });
        let gravity = -self.s.gravity / 9.8;
        let gp = self.pad.state();
        let gp_jump = self.pad.jump_pressed();
        let gp_action = self.pad.pressed(crate::gamepad::Button::West);
        let locked = self.game.as_ref().is_some_and(|g| g.locks_input());
        let bench = self.bench_frame();
        let (aw, ah) = self.arena();
        let floor = world.walls().floor.then_some(ah - WALL_T * PPM);
        let Some(run) = &mut self.sonic else { return };
        let mut pad = Pad {
            left: key(KeyCode::Left) || gp.left(),
            right: key(KeyCode::Right) || gp.right(),
            up: key(KeyCode::Up) || gp.up(),
            down: key(KeyCode::Down) || gp.down(),
            jump: (key(KeyCode::Space) && !shift) || gp.jump(),
            jump_pressed: run.jump || gp_jump,
            action_pressed: run.action || gp_action,
        };
        if locked {
            pad = Pad::default();
            run.jump = false;
        }
        // The benchmark runs him to the right, hopping now and then.
        if let Some(frame) = bench {
            pad.right = true;
            pad.jump = frame % 50 < 20;
            pad.jump_pressed = frame % 50 == 0;
        }
        let mut events = vec![];
        if run.held.is_none() {
            run.acc = (run.acc + step).min(MAX_FRAMES / FPS);
            while run.acc >= 1.0 / FPS {
                run.acc -= 1.0 / FPS;
                events.extend(run.sonic.frame(world, &env, pad, water, gravity));
                pad.jump_pressed = false;
                pad.action_pressed = false;
                run.jump = false;
                run.action = false;
            }
        }
        // Never lost under the world's floor (something heavy pushed him in).
        if let Some(top) = floor.filter(|&top| !run.sonic.dead && run.sonic.pos.y > top) {
            run.sonic.pos.y = top - run.sonic.radii().1 * SCALE;
            run.sonic.ysp = run.sonic.ysp.min(0.0);
        }
        let s = run.sonic.clone();
        // Fell out of the world: back to the start (a level takes a life instead).
        if self.game.is_none()
            && (s.pos.x < -300.0 || s.pos.x > aw + 300.0 || s.pos.y > ah + 300.0 || s.pos.y < -1500.0)
        {
            let endless = self.endless.is_some();
            if let Some(run) = &mut self.sonic {
                // In an endless run he comes back where he fell.
                if endless {
                    run.sonic.spawn = vec2(s.pos.x.clamp(0.0, aw), ah - WALL_T * PPM - 200.0);
                }
                run.sonic.respawn();
            }
        }
        self.sonic_events(&s, &events, step);
        self.collect_rings(&s);
        if self.pan_last.is_none() && !s.dead {
            let screen = vec2(screen_width(), screen_height());
            // In a boss's arena the view holds still on it.
            let arena = self.game.as_ref().and_then(|g| g.arena());
            let target = arena.map_or(s.pos, |(l, r)| vec2((l + r) / 2.0, s.pos.y));
            self.camera.follow(target, screen, FOLLOW);
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
                Event::Break { collider, at } => {
                    if self.break_game_monitor(collider) {
                        continue;
                    }
                    let Some(h) = self.world.colliders.get(collider).and_then(|c| c.parent()) else { continue };
                    let (x, y) = to_phys(at.x, at.y);
                    if self.shatter(h, Point::new(x, y)) {
                        self.sound(Sound::Shatter, at, 0.6);
                        self.pad.rumble(0.6, 0.15);
                    }
                }
                Event::Jump => self.sound(Sound::Jump, s.pos, 0.5),
                Event::Roll => self.sound(Sound::Dash, s.pos, 0.25),
                Event::Rev => {
                    let rev = s.spindash.unwrap_or(0.0);
                    self.sound(Sound::Rev, s.pos, 0.35 + rev * 0.04);
                }
                Event::Dash => {
                    self.sound(Sound::Dash, s.pos, 0.6);
                    self.pad.rumble(0.5, 0.2);
                }
                Event::Skid => self.sound(Sound::Skid, feet, 0.5),
                Event::Spring => {
                    self.spring_fired = Some((s.pos, get_time()));
                    self.sound(Sound::Spring, s.pos, 0.6);
                    self.pad.rumble(0.35, 0.12);
                }
                Event::Splash => {
                    self.effects.splash(s.pos, 0.8);
                }
                Event::Flap => self.sound(Sound::Dash, s.pos, 0.15),
                Event::Glide => self.sound(Sound::Dash, s.pos, 0.3),
                Event::Grab => self.sound(Sound::Hit { size: 40.0, hard: 0.2 }, s.pos, 0.5),
                Event::Hammer => {
                    self.sound(Sound::Dash, s.pos, 0.45);
                    self.pad.rumble(0.3, 0.1);
                }
                Event::Teleport { from } => {
                    self.sound(Sound::Static, s.pos, 0.6);
                    self.warp_echo = Some((from, get_time()));
                    if self.s.effects {
                        self.effects.smoke(from, 10.0, 0.15);
                        self.effects.sparks(s.pos, vec2(-s.facing, 0.0), 10.0);
                    }
                }
                Event::Transform => {
                    self.sound(Sound::Transform, s.pos, 0.7);
                    self.pad.rumble(0.6, 0.3);
                    if self.s.effects {
                        self.effects.sparks(s.pos, vec2(0.0, -1.0), 24.0);
                    }
                    self.toasts.success(format!("Super {}!", s.character.label()));
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
            if self.game_sprites.is_some() {
                self.booms.push(super::game::Boom::sparkle(*r));
            } else if self.s.effects {
                self.effects.sparks(*r, vec2(0.0, -1.0), 6.0);
            }
        }
        if let Some(at) = got.first() {
            self.sound(Sound::Ring, *at, 0.5);
            if let Some(run) = &mut self.sonic {
                run.rings += got.len() as u32;
            }
            if let Some(game) = &mut self.game {
                let fx = game.add_rings(got.len() as u32);
                self.game_effects(fx);
            } else if self.rings.is_empty() {
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
        let sp = self.game_sprites.as_ref();
        for (k, r) in self.rings.iter().enumerate() {
            if !sp.is_some_and(|sp| sp.ring(*r, t)) {
                art::ring(*r, t + k as f32 * 0.07, SCALE);
            }
        }
        // The static Sonic.exe leaves where he vanished.
        if let Some((at, when)) = self.warp_echo {
            let age = (get_time() - when) as f32;
            if age < 0.4 {
                let a = 1.0 - age / 0.4;
                let seed = (get_time() * 60.0) as u32;
                for k in 0..14u32 {
                    let h = k.wrapping_add(seed).wrapping_mul(2_654_435_761) >> 8;
                    let y = at.y - 30.0 + (h % 60) as f32;
                    let w = 20.0 + ((h >> 6) % 40) as f32;
                    let x = at.x - w / 2.0 + ((h >> 12) % 16) as f32 - 8.0;
                    let grey = 0.3 + ((h >> 3) % 60) as f32 / 100.0;
                    draw_rectangle(x, y, w, 2.0, Color::new(grey, grey * 0.6, grey * 0.6, 0.7 * a));
                }
            }
        }
        if let Some(run) = self.sonic.as_ref().filter(|_| !self.sonic_blinks()) {
            match self.sonic_sheet.get(run.sonic.character) {
                Some(sheet) => sheet.draw(&run.sonic),
                None => art::draw(&run.sonic),
            }
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
        if !self.rings.is_empty() && self.endless.is_none() {
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

impl App {
    /// Read the game controller, and say when one comes or goes.
    pub(super) fn poll_pad(&mut self) {
        let was = self.pad.connected();
        self.pad.poll();
        match (was, self.pad.name.clone()) {
            (false, Some(name)) => self.toasts.status("pad", format!("Controller connected: {name}")),
            (true, None) => self.toasts.status("pad", "Controller disconnected"),
            _ => {}
        }
    }
}
