//! The game mode in the app: a level's badniks, hazards and items come to
//! life around Sonic, with sounds, particles and rumble, the score / time
//! / rings / lives display, the end-of-act tally and Game Over.

use super::*;
use crate::audio::music::Jingle;
use crate::audio::sfx::Sound;
use crate::game::{art as game_art, Effect, Game, LevelSpec, MonitorKind, Phase};
use rapier2d::prelude::ColliderHandle;

/// A score popping up where a badnik was destroyed.
pub(super) struct Popup {
    pub at: Vec2,
    text: String,
    age: f32,
}

/// An explosion or a ring's sparkle from the sprite sheets.
pub(super) struct Boom {
    pub at: Vec2,
    age: f32,
    kind: BoomKind,
}

#[derive(Clone, Copy, PartialEq)]
enum BoomKind {
    Puff,
    Fire,
    Sparkle,
}

impl Boom {
    pub fn sparkle(at: Vec2) -> Self {
        Boom { at, age: 0.0, kind: BoomKind::Sparkle }
    }
}

impl App {
    /// Start the level `spec` with Sonic at `start` (after the scene is built).
    pub(super) fn start_game(&mut self, spec: &LevelSpec, start: Vec2) {
        self.stop_game();
        self.spawn_sonic(start);
        let mut game = Game::new(spec, &mut self.world, start);
        let emeralds = crate::special::count(self.s.emeralds);
        game.emeralds_missing = emeralds < crate::special::EMERALDS && !self.level_testing();
        game.emeralds_all = emeralds == crate::special::EMERALDS;
        self.game = Some(game);
        self.popups.clear();
        self.paused = false;
        let name = if spec.name.is_empty() { "Act" } else { spec.name.as_str() };
        let aim = if spec.goal.is_some() || spec.boss.is_some() { "reach the goal!" } else { "run as far as you can!" };
        if spec.hunter {
            self.toasts.warn(format!("{name}  ·  Sonic.exe is coming: run for the goal!"));
        } else {
            self.toasts.status("game", format!("{name}  ·  {aim}  ·  curl up to destroy badniks"));
        }
    }

    /// Leave the game mode (its bodies go too).
    pub(super) fn stop_game(&mut self) {
        self.booms.clear();
        if let Some(g) = self.game.take() {
            g.remove(&mut self.world);
        }
    }

    /// After Sonic's frames: the level moves, and what happens is shown.
    pub(super) fn update_game(&mut self, step: f32) {
        for p in &mut self.popups {
            p.age += step;
            p.at.y -= 40.0 * step;
        }
        self.popups.retain(|p| p.age < 1.2);
        for b in &mut self.booms {
            b.age += step;
        }
        self.booms.retain(|b| b.age < 0.6);
        self.scare = (self.scare - step).max(0.0);
        let (Some(game), Some(run)) = (&mut self.game, &mut self.sonic) else { return };
        let grains: HashSet<RigidBodyHandle> = self.grains.bodies().collect();
        let world = &self.world;
        let solid =
            |c: ColliderHandle| world.colliders.get(c).and_then(|c| c.parent()).is_none_or(|b| !grains.contains(&b));
        let fall_y = self.world.arena.1 * PPM + 150.0;
        game.water =
            self.s.water.then(|| crate::physics::to_screen(0.0, water::rest_level(world, self.s.water_level)).y);
        let fx = game.update(step, &self.world, &solid, &mut run.sonic, fall_y);
        self.game_effects(fx);
    }

    /// A monitor broken by Sonic's `Break` event; false if it was not one.
    pub(super) fn break_game_monitor(&mut self, collider: ColliderHandle) -> bool {
        let (Some(game), Some(run)) = (&mut self.game, &mut self.sonic) else { return false };
        if !game.is_monitor(collider) {
            return false;
        }
        let fx = game.break_monitor(&mut self.world, collider, &mut run.sonic);
        self.game_effects(fx);
        true
    }

    pub(super) fn game_effects(&mut self, fx: Vec<Effect>) {
        for e in fx {
            match e {
                Effect::EnemyDown { at, score } => {
                    self.boom(at, BoomKind::Puff);
                    self.sound(Sound::Boom, at, 0.35);
                    if self.s.effects {
                        self.effects.debris(at, 18.0, Color::new(0.7, 0.72, 0.8, 1.0));
                        self.effects.dust(at, 6.0);
                    }
                    self.popups.push(Popup { at, text: score.to_string(), age: 0.0 });
                    self.pad.rumble(0.3, 0.1);
                }
                Effect::Hurt(at) => {
                    self.sound(Sound::Hurt, at, 0.6);
                    self.pad.rumble(0.7, 0.3);
                }
                Effect::RingsLost(at) => self.sound(Sound::Scatter, at, 0.6),
                Effect::RingCaught(at) => {
                    self.boom(at, BoomKind::Sparkle);
                    self.sound(Sound::Ring, at, 0.4);
                }
                Effect::ShieldLost(at) => self.sound(Sound::Pop, at, 0.6),
                Effect::Died(at) => {
                    self.sound(Sound::Hurt, at, 0.8);
                    self.pad.rumble(1.0, 0.5);
                    self.cancel_jingle(Jingle::Drowning);
                }
                Effect::Monitor(kind, at) => {
                    self.boom(at, BoomKind::Puff);
                    self.sound(Sound::Pop, at, 0.7);
                    if self.s.effects {
                        self.effects.debris(at, 16.0, Color::new(0.7, 0.75, 0.85, 1.0));
                    }
                    let msg = match kind {
                        MonitorKind::Rings => "+10 rings",
                        MonitorKind::Shield => "Shield",
                        MonitorKind::Invincible => "Invincible!",
                        MonitorKind::Shoes => "Speed shoes!",
                        MonitorKind::Life => "1-up!",
                    };
                    // The sprites show the item popping out instead.
                    if self.game_sprites.is_none() {
                        self.popups.push(Popup { at, text: msg.to_string(), age: 0.0 });
                    }
                    if kind == MonitorKind::Rings {
                        self.sound(Sound::Ring, at, 0.5);
                    }
                }
                Effect::ExtraLife => {
                    if self.s.sonic_music {
                        self.play_jingle(Jingle::OneUp);
                    } else {
                        self.sound(Sound::Win, vec2(screen_width() / 2.0, 0.0), 0.4);
                    }
                    self.toasts.success("1-up!");
                }
                Effect::Post(at) => self.sound(Sound::Post, at, 0.6),
                Effect::Goal(at) => self.sound(Sound::Spring, at, 0.4),
                Effect::Cleared => {
                    if self.s.sonic_music {
                        self.play_jingle(Jingle::ActClear);
                    } else {
                        self.sound(Sound::Win, vec2(screen_width() / 2.0, 0.0), 0.7);
                    }
                }
                Effect::Shot(at) => self.sound(Sound::Shot, at, 0.25),
                Effect::BombBlast(at) | Effect::Explosion(at) => {
                    self.boom(at, BoomKind::Fire);
                    self.sound(Sound::Boom, at, 0.4);
                    if self.s.effects {
                        self.effects.debris(at, 22.0, Color::new(1.0, 0.6, 0.2, 1.0));
                        self.effects.smoke(at, 18.0, 0.2);
                    }
                }
                Effect::BossHit(at) => {
                    self.sound(Sound::Clang, at, 0.8);
                    self.pad.rumble(0.8, 0.2);
                    if self.s.effects {
                        self.effects.sparks(at, vec2(0.0, -1.0), 14.0);
                    }
                }
                Effect::BossBeaten(at) => {
                    self.sound(Sound::Boom, at, 0.9);
                    self.toasts.success("The boss is beaten!  ·  on to the goal");
                }
                Effect::Respawned(at) => {
                    self.camera.center = at;
                }
                Effect::Swoop(at) => self.sound(Sound::Skid, at, 0.3),
                Effect::AirWarning(at) => self.sound(Sound::Chime, at, 0.6),
                Effect::Drowning => self.play_jingle(Jingle::Drowning),
                Effect::Breathe(at) => {
                    self.sound(Sound::Gulp, at, 0.6);
                    self.cancel_jingle(Jingle::Drowning);
                    if self.s.effects {
                        self.effects.splash(at, 0.3);
                    }
                }
                Effect::SuperEnd(at) => self.sound(Sound::Pop, at, 0.6),
                Effect::HunterAppears(at) => {
                    self.sound(Sound::Static, at, 0.7);
                    self.scare = 0.35;
                }
                Effect::HunterCaught(at) => {
                    self.sound(Sound::Laugh, at, 0.8);
                    self.sound(Sound::Static, at, 0.5);
                    self.scare = 0.6;
                    self.pad.rumble(0.9, 0.4);
                }
                Effect::GiantRing(at) => {
                    self.sound(Sound::Warp, at, 0.7);
                    self.toasts.success("A giant ring!  ·  a special stage after the tally");
                }
                Effect::GameOver => {
                    let endless = self.game.as_ref().is_some_and(|g| g.endless);
                    if !endless {
                        self.toasts.warn("Game Over");
                        self.play_jingle(Jingle::GameOver);
                    }
                }
                Effect::Restart => self.restart_level(),
            }
        }
    }

    /// An explosion or sparkle, when the sprite sheets have them.
    fn boom(&mut self, at: Vec2, kind: BoomKind) {
        if self.game_sprites.is_some() {
            self.booms.push(Boom { at, age: 0.0, kind });
        }
    }

    /// A spring pad drawn from the sprite sheet over its plank, in a level:
    /// true when it was.
    pub(super) fn draw_spring_sprite(&self, o: &crate::physics::object::Object) -> bool {
        let Some(sp) = self.game_sprites.as_ref().filter(|_| self.game.is_some()) else { return false };
        if !o.pinned || o.material.bounce < 0.6 || o.size.x < o.size.y * 2.0 {
            return false;
        }
        let (p, angle) = o.screen_pos(&self.world);
        if angle.sin().abs() > 0.1 {
            return false;
        }
        let foot = p + vec2(0.0, o.size.y / 2.0);
        let out = self
            .spring_fired
            .is_some_and(|(at, when)| get_time() - when < 0.25 && (at.x - p.x).abs() < o.size.x && at.y < p.y);
        sp.spring(foot, o.size.x, out)
    }

    /// Space / A / Enter after the tally: the next act, or back to the library.
    pub(super) fn level_continue(&mut self) {
        let done = self.game.as_ref().is_some_and(|g| matches!(g.phase, Phase::Cleared(t) if t.done()));
        if !done {
            return;
        }
        if self.level_testing() {
            // Tried from the editor: back to editing.
            self.level_cmd(crate::ui::LevelCmd::Stop);
            return;
        }
        if let Some(g) = &self.game {
            let best = self.s.act_scores.entry(g.spec.name.clone()).or_insert(0);
            *best = (*best).max(g.score);
            let _ = self.s.save();
        }
        // Through the giant ring first.
        let next = self.next_act();
        if self.game.as_ref().is_some_and(|g| g.giant_ring) && crate::special::next_missing(self.s.emeralds).is_some() {
            self.start_special(next);
        } else if let Some(next) = next {
            self.open_act(next);
        } else {
            self.stop_game();
            self.library.fader.open = true;
            self.toasts.success("Well done!");
        }
    }

    /// Who is playing.
    fn player(&self) -> crate::sonic::Character {
        self.sonic.as_ref().map_or(self.s.character, |r| r.sonic.character)
    }

    /// Whether the tally is finished (waiting for the player).
    pub(super) fn tally_done(&self) -> bool {
        self.game.as_ref().is_some_and(|g| matches!(g.phase, Phase::Cleared(t) if t.done()))
    }

    /// The level (world pass), before Sonic is drawn.
    pub(super) fn draw_game(&self) {
        let Some(g) = &self.game else { return };
        let t = get_time() as f32;
        game_art::draw(g, t, self.game_sprites.as_ref());
        self.draw_hunter(g, t);
    }

    /// Sonic.exe after the player: from his sheet when there is one, else
    /// drawn with shapes; he flickers in through the static.
    fn draw_hunter(&self, g: &Game, t: f32) {
        use crate::sonic::{Character, Sonic};
        let Some(h) = g.hunter.as_ref().filter(|h| h.present()) else { return };
        if h.shown < 0.6 && (t * 30.0) as i32 % 3 == 0 {
            return;
        }
        let at = h.pos + vec2(0.0, (t * 3.0).sin() * 4.0);
        let laughing = h.laughed.is_some();
        draw_circle(at.x, at.y, 44.0, Color::new(0.0, 0.0, 0.0, 0.22));
        // Timed from the laugh, or from his coming (where an animation played
        // once, or opening before its loop, starts).
        let (name, since, fps) = match h.laughed {
            Some(since) => ("laugh", since, 8.0),
            None => ("chase", h.shown, 10.0),
        };
        let sheet = self.sonic_sheet.get(Character::Exe);
        if !sheet.is_some_and(|sh| sh.draw_anim(name, since, fps, at, h.facing, WHITE)) {
            let mut s = Sonic::as_character(at, Character::Exe);
            s.grounded = true;
            s.facing = h.facing;
            s.gsp = if laughing { 0.0 } else { 7.0 * h.facing };
            s.stride = t * 14.0;
            s.anim = t * 10.0;
            crate::sonic::art::draw(&s);
        }
    }

    /// Broken static over the screen while Sonic.exe appears (screen pass).
    pub(super) fn draw_scare(&self) {
        if self.scare <= 0.0 {
            return;
        }
        let (sw, sh) = (screen_width(), screen_height());
        let a = (self.scare / 0.35).min(1.0);
        let seed = (get_time() * 60.0) as u32;
        for k in 0..60u32 {
            let h = (k.wrapping_add(seed).wrapping_mul(2_654_435_761)) >> 8;
            let y = (h % 10_000) as f32 / 10_000.0 * sh;
            let grey = ((h >> 4) % 100) as f32 / 100.0;
            let thick = 1.0 + ((h >> 11) % 4) as f32;
            draw_rectangle(0.0, y, sw, thick, Color::new(grey, grey * 0.8, grey * 0.8, 0.35 * a));
        }
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.4, 0.0, 0.0, 0.12 * a));
    }

    /// Shield, sparkles and score popups (world pass), after Sonic.
    pub(super) fn draw_game_over_sonic(&self) {
        let t = get_time() as f32;
        let sp = self.game_sprites.as_ref();
        if let (Some(g), Some(run)) = (&self.game, &self.sonic) {
            if !run.sonic.dead {
                game_art::draw_around(g, run.sonic.pos, t, sp);
            }
        }
        if let Some(run) = self.sonic.as_ref().filter(|r| r.sonic.super_form && !r.sonic.dead) {
            game_art::super_sparkles(run.sonic.pos, t);
        }
        if let Some(sp) = sp {
            for b in &self.booms {
                match b.kind {
                    BoomKind::Sparkle => sp.sparkle(b.at, b.age),
                    kind => sp.explosion(b.at, b.age, kind == BoomKind::Fire),
                };
            }
        }
        for p in &self.popups {
            let a = (1.0 - p.age / 1.2).clamp(0.0, 1.0);
            if sp.is_some_and(|sp| sp.score(&p.text, p.at, a)) {
                continue;
            }
            let w = theme::measure_bold(&p.text, 16.0);
            theme::text_bold(&p.text, p.at.x - w / 2.0 + 1.0, p.at.y + 1.0, 16.0, Color::new(0.0, 0.0, 0.0, 0.6 * a));
            theme::text_bold(&p.text, p.at.x - w / 2.0, p.at.y, 16.0, Color::new(1.0, 1.0, 1.0, a));
        }
    }

    /// Whether Sonic is hidden this frame (blinking after a hit).
    pub(super) fn sonic_blinks(&self) -> bool {
        self.game.as_ref().is_some_and(|g| g.blinking(get_time() as f32))
    }

    /// Score, time, rings and lives; the tally and Game Over (screen pass).
    pub(super) fn draw_game_hud(&self, top: f32) {
        let Some(g) = &self.game else { return };
        let t = get_time() as f32;
        let yellow = Color::new(1.0, 0.88, 0.2, 1.0);
        let shade = Color::new(0.0, 0.0, 0.0, 0.6);
        let line = |label: &str, value: String, y: f32, warn: bool| {
            let c = if warn && (t * 4.0) as i32 % 2 == 0 { Color::new(1.0, 0.25, 0.2, 1.0) } else { yellow };
            theme::text_bold(label, 18.0, y + 2.0, 20.0, shade);
            theme::text_bold(label, 16.0, y, 20.0, c);
            theme::text_bold(&value, 118.0, y + 2.0, 20.0, shade);
            theme::text_bold(&value, 116.0, y, 20.0, WHITE);
        };
        let secs = g.time as u32;
        line("SCORE", g.score.to_string(), top + 24.0, false);
        line("TIME", format!("{}:{:02}", secs / 60, secs % 60), top + 48.0, secs >= 540);
        line("RINGS", g.rings.to_string(), top + 72.0, g.rings == 0);
        if !g.endless {
            let y = screen_height() - 24.0;
            if !self.game_sprites.as_ref().is_some_and(|sp| sp.face(self.player(), vec2(30.0, y - 6.0), 28.0)) {
                draw_circle(30.0, y - 6.0, 13.0, Color::new(0.12, 0.36, 0.9, 1.0));
                draw_circle(34.0, y - 9.0, 4.0, WHITE);
            }
            theme::text_bold(&format!("×  {}", g.lives), 50.0, y, 20.0, WHITE);
        }
        let (sw, sh) = (screen_width(), screen_height());
        match g.phase {
            Phase::Cleared(tally) => {
                draw_rectangle(0.0, sh * 0.25, sw, 210.0, Color::new(0.0, 0.0, 0.0, 0.55));
                let name = if g.spec.name.is_empty() { "THE ACT" } else { g.spec.name.as_str() };
                theme::text_bold_centered(
                    &format!("{} GOT THROUGH {}", self.player().label().to_uppercase(), name.to_uppercase()),
                    sw / 2.0,
                    sh * 0.25 + 34.0,
                    26.0,
                    yellow,
                );
                let row = |label: &str, value: u32, y: f32| {
                    theme::text_bold(label, sw / 2.0 - 180.0, y, 20.0, yellow);
                    let v = value.to_string();
                    let w = theme::measure_bold(&v, 20.0);
                    theme::text_bold(&v, sw / 2.0 + 180.0 - w, y, 20.0, WHITE);
                };
                row("TIME BONUS", tally.time_bonus, sh * 0.25 + 84.0);
                row("RING BONUS", tally.ring_bonus, sh * 0.25 + 114.0);
                row("SCORE", g.score, sh * 0.25 + 150.0);
                if g.giant_ring {
                    let c = vec2(sw / 2.0 + 250.0, sh * 0.25 + 110.0);
                    game_art::giant_ring(c, 34.0, t);
                    theme::text_bold_centered("SPECIAL STAGE", c.x, c.y + 56.0, 14.0, yellow);
                }
                if tally.done() && (t * 2.0) as i32 % 2 == 0 {
                    theme::text_centered("Space / A to continue", sw / 2.0, sh * 0.25 + 190.0, 15.0, WHITE);
                }
            }
            Phase::Over(_) if !g.endless => {
                draw_rectangle(0.0, sh * 0.4, sw, 80.0, Color::new(0.0, 0.0, 0.0, 0.6));
                let centre = vec2(sw / 2.0, sh * 0.4 + 40.0);
                if !self.game_sprites.as_ref().is_some_and(|sp| sp.game_over(centre, 3.0)) {
                    theme::text_bold_centered("GAME  OVER", sw / 2.0, sh * 0.4 + 46.0, 40.0, yellow);
                }
            }
            _ => {}
        }
    }
}
