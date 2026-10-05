//! What the weather does to the world: rain and snow fall as grains, rain
//! puts out fires under the open sky, fire melts snow, storm gusts push
//! things around and lightning starts fires where it strikes.

use super::*;
use crate::audio::sfx::Sound;
use crate::physics::grains::GrainKind;
use crate::weather::Weather;
use rapier2d::prelude::{nalgebra, vector, QueryFilter};

/// Drops or flakes added per second (per window width).
const RAIN_RATE: f32 = 4.0;
const STORM_RATE: f32 = 7.0;
const SNOW_RATE: f32 = 12.0;
/// How often rain checks for fires under the open sky (s), and the chance
/// each check puts one out.
const DOUSE_EVERY: f32 = 0.3;
const DOUSE_CHANCE: f32 = 0.3;
/// Seconds between lightning strikes in a storm.
const LIGHTNING_GAP: (f32, f32) = (4.0, 9.0);

#[derive(Default)]
pub(super) struct Climate {
    /// Grains owed to the sky.
    owed: f32,
    douse: f32,
    lightning: f32,
}

impl App {
    /// The weather in effect: none in challenges and the editor.
    pub(super) fn weather(&self) -> Weather {
        if self.challenge.is_some() || self.editor.is_some() {
            Weather::Clear
        } else {
            self.s.weather
        }
    }

    /// The sky's look (every frame) and, while the simulation runs, what it
    /// does to the world (`step` in simulated seconds).
    pub(super) fn update_weather(&mut self, dt: f32, running: bool, step: f32, mouse: Vec2) {
        let weather = self.weather();
        let view = self.view();
        let (sw, sh) = (screen_width(), screen_height());
        // The visible part of the arena.
        let (aw, ah) = self.arena();
        let (a, b) = (view.to_world(vec2(0.0, 0.0)).max(Vec2::ZERO), view.to_world(vec2(sw, sh)).min(vec2(aw, ah)));
        let area = Rect::new(a.x, a.y, (b.x - a.x).max(1.0), (b.y - a.y).max(1.0));
        self.sky.update(if running { dt } else { 0.0 }, weather, area);
        if !running || step <= 0.0 {
            return;
        }
        self.fall(weather, step);
        if matches!(weather, Weather::Rain | Weather::Storm) {
            self.rain_on_fires(step);
        }
        self.melt_snow(mouse);
        if weather == Weather::Storm {
            self.gusts();
            self.climate.lightning -= step;
            if self.climate.lightning <= 0.0 {
                self.climate.lightning = rand::gen_range(LIGHTNING_GAP.0, LIGHTNING_GAP.1);
                self.lightning();
            }
        } else {
            self.climate.lightning = rand::gen_range(1.0, 3.0);
        }
    }

    /// Height (world px) just under the ceiling, where drops appear.
    fn sky_top(&self) -> f32 {
        if self.world.border.walls().ceiling {
            WALL_T * PPM + 6.0
        } else {
            4.0
        }
    }

    /// Rain drops and snowflakes join the world as grains.
    fn fall(&mut self, weather: Weather, step: f32) {
        let (kind, rate, speed) = match weather {
            Weather::Clear => return,
            Weather::Rain => (GrainKind::Liquid, RAIN_RATE, 6.0),
            Weather::Storm => (GrainKind::Liquid, STORM_RATE, 7.0),
            Weather::Snow => (GrainKind::Snow, SNOW_RATE, 0.6),
        };
        // Without gravity pulling them down, drops would hang in the air.
        if self.s.gravity > -0.5 {
            return;
        }
        let (aw, _) = self.arena();
        self.climate.owed += rate * step * (aw / screen_width().max(1.0));
        let top = self.sky_top();
        while self.climate.owed >= 1.0 {
            self.climate.owed -= 1.0;
            let x = rand::gen_range(10.0, aw - 10.0);
            let wind = self.sky.gust * 0.3;
            self.grains.add(&mut self.world, kind, vec2(x, top), vector![wind, -speed]);
        }
    }

    /// Fires with nothing over them are put out, one now and then.
    fn rain_on_fires(&mut self, step: f32) {
        self.climate.douse -= step;
        if self.climate.douse > 0.0 {
            return;
        }
        self.climate.douse = DOUSE_EVERY;
        let grains: HashSet<RigidBodyHandle> = self.grains.bodies().collect();
        let mut doused = vec![];
        for o in self.objects.iter().filter(|o| self.fire.burning(o.body)) {
            let Some(c) = self.world.colliders.get(o.collider) else { continue };
            let aabb = c.compute_aabb();
            let x = rand::gen_range(aabb.mins.x, aabb.maxs.x);
            let from = Point::new(x, aabb.maxs.y + 0.02);
            let filter = QueryFilter::default().exclude_rigid_body(o.body).exclude_sensors();
            let hit = self.world.cast_ray(from, vector![0.0, 1.0], 1000.0, filter);
            let open = hit.is_none_or(|(h, _, _)| {
                self.world.colliders.get(h).and_then(|c| c.parent()).is_some_and(|b| grains.contains(&b))
            });
            if open && rand::gen_range(0.0, 1.0) < DOUSE_CHANCE {
                doused.push(o.body);
            }
        }
        for body in doused {
            if !self.fire.douse(body) {
                continue;
            }
            let Some(i) = self.objects.iter().position(|o| o.body == body) else { continue };
            let (at, _) = self.objects[i].screen_pos(&self.world);
            let r = self.objects[i].size.length() / 4.0;
            for _ in 0..8 {
                let p = at + vec2(rand::gen_range(-r, r), rand::gen_range(-r, r));
                self.effects.smoke(p, 12.0, 1.0);
            }
            self.sound(Sound::Hiss, at, 0.5);
        }
    }

    /// Snow near burning objects or the Fire tool melts.
    fn melt_snow(&mut self, mouse: Vec2) {
        let mut heat: Vec<(Vec2, f32)> = self
            .objects
            .iter()
            .filter(|o| self.fire.burning(o.body))
            .map(|o| (o.screen_pos(&self.world).0, o.size.length() / 2.0 + 18.0))
            .collect();
        if self.fire.lit && self.s.tool == Tool::Fire {
            heat.push((mouse, 26.0));
        }
        let n = self.grains.melt_snow(&mut self.world, &heat);
        if n > 0 && rand::gen_range(0, 8) == 0 {
            let at = heat[0].0;
            self.sound(Sound::Hiss, at, 0.15);
        }
    }

    /// Storm wind pushing everything that moves sideways.
    fn gusts(&mut self) {
        let gust = self.sky.gust;
        if gust.abs() < 0.05 {
            return;
        }
        let mut bodies = self.rigid_bodies();
        bodies.extend(self.soft_bodies());
        bodies.extend(self.player_body.as_ref().map(|p| p.body));
        for h in bodies {
            let Some(b) = self.world.bodies.get_mut(h).filter(|b| b.is_dynamic()) else { continue };
            let f = gust * b.mass();
            b.add_force(vector![f, 0.0], true);
        }
    }

    /// A lightning bolt strikes the highest of a few random spots: what it
    /// hits gets hot enough to burn (or crack, or melt).
    fn lightning(&mut self) {
        let (aw, ah) = self.arena();
        let top = self.sky_top();
        let grains: HashSet<RigidBodyHandle> = self.grains.bodies().collect();
        let mut best: Option<(Vec2, Option<RigidBodyHandle>)> = None;
        for _ in 0..4 {
            let x = rand::gen_range(aw * 0.05, aw * 0.95);
            let (px, py) = to_phys(x, top);
            let not_grain = |_, c: &rapier2d::prelude::Collider| c.parent().is_none_or(|b| !grains.contains(&b));
            let filter = QueryFilter::default().exclude_sensors().predicate(&not_grain);
            let (to, body) = match self.world.cast_ray(Point::new(px, py), vector![0.0, -1.0], ah / PPM, filter) {
                Some((c, toi, _)) => (vec2(x, top + toi * PPM), self.world.colliders.get(c).and_then(|c| c.parent())),
                None => (vec2(x, ah), None),
            };
            if best.is_none_or(|(b, _)| to.y < b.y) {
                best = Some((to, body));
            }
        }
        let Some((to, body)) = best else { return };
        self.sky.strike(top - 40.0, to);
        if let Some(i) = body.and_then(|b| self.objects.iter().position(|o| o.body == b)) {
            let body = self.objects[i].body;
            self.fire.warm(body, 1.5);
            if let Some(b) = self.world.bodies.get_mut(body).filter(|b| b.is_dynamic()) {
                let m = b.mass();
                b.apply_impulse(vector![rand::gen_range(-0.5, 0.5) * m, -1.5 * m], true);
            }
        }
        self.effects.sparks(to, vec2(0.0, -1.0), 22.0);
        for _ in 0..3 {
            self.effects.flame(to + vec2(rand::gen_range(-6.0, 6.0), 0.0), 14.0);
        }
        self.sound(Sound::Thunder, to, 0.9);
    }
}
