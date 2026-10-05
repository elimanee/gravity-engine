//! The sky: rain streaks and snowflakes falling across the view, storm
//! gusts, and lightning bolts with their flash. Only the look lives here;
//! what the weather does to the world (drops that fill containers, snow
//! that settles, fires put out or started) is in `app::weather`.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum Weather {
    #[default]
    Clear,
    Rain,
    Snow,
    /// Heavy rain, gusts of wind and lightning.
    Storm,
}

impl Weather {
    pub const ALL: &'static [Weather] = &[Weather::Clear, Weather::Rain, Weather::Snow, Weather::Storm];

    pub fn label(self) -> &'static str {
        match self {
            Weather::Clear => "Clear",
            Weather::Rain => "Rain",
            Weather::Snow => "Snow",
            Weather::Storm => "Storm",
        }
    }

    pub fn cycle(self, d: i32) -> Self {
        let n = Self::ALL.len() as i32;
        let i = Self::ALL.iter().position(|w| *w == self).unwrap_or(0) as i32;
        Self::ALL[(i + d).rem_euclid(n) as usize]
    }

    pub fn accent(self) -> Color {
        match self {
            Weather::Clear => Color::from_rgba(255, 210, 110, 255),
            Weather::Rain => Color::from_rgba(120, 180, 255, 255),
            Weather::Snow => Color::from_rgba(225, 238, 255, 255),
            Weather::Storm => Color::from_rgba(190, 160, 255, 255),
        }
    }

    /// Drops or flakes kept falling across the view.
    fn particles(self) -> usize {
        match self {
            Weather::Clear => 0,
            Weather::Rain => 320,
            Weather::Snow => 260,
            Weather::Storm => 520,
        }
    }
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    /// Size and drift phase (snow).
    size: f32,
    phase: f32,
}

/// A lightning bolt: its jagged path (world px) and how long it has shown.
pub struct Bolt {
    pub points: Vec<Vec2>,
    pub age: f32,
}

/// How long a bolt stays visible (s).
const BOLT_LIFE: f32 = 0.35;

#[derive(Default)]
pub struct Sky {
    particles: Vec<Particle>,
    kind: Weather,
    /// Storm wind (m/s², positive to the right) and where it is heading.
    pub gust: f32,
    gust_target: f32,
    gust_timer: f32,
    pub bolt: Option<Bolt>,
    /// Lightning flash, 1 → 0.
    pub flash: f32,
}

impl Sky {
    /// Advance by `dt` over the visible world rectangle `view`.
    pub fn update(&mut self, dt: f32, weather: Weather, view: Rect) {
        if weather != self.kind {
            self.kind = weather;
            self.particles.clear();
        }
        // Gusts wander between ±6 m/s² in a storm.
        if weather == Weather::Storm {
            self.gust_timer -= dt;
            if self.gust_timer <= 0.0 {
                self.gust_timer = gen_range(1.5, 4.0);
                self.gust_target = gen_range(-6.0, 6.0);
            }
        } else {
            self.gust_target = 0.0;
        }
        self.gust += (self.gust_target - self.gust) * (dt * 0.8).min(1.0);
        self.flash = (self.flash - dt * 3.5).max(0.0);
        if let Some(b) = &mut self.bolt {
            b.age += dt;
            if b.age > BOLT_LIFE {
                self.bolt = None;
            }
        }

        let want = weather.particles();
        let wind = self.gust * 40.0;
        while self.particles.len() < want {
            // The first ones fill the whole view, then they come from the top.
            let y = if self.particles.is_empty() || self.particles.len() < want / 2 {
                gen_range(view.y, view.y + view.h)
            } else {
                view.y - gen_range(0.0, 12.0)
            };
            let p = vec2(gen_range(view.x, view.x + view.w), y);
            self.particles.push(self.spawn(weather, p));
        }
        self.particles.truncate(want);
        for i in 0..self.particles.len() {
            let p = &mut self.particles[i];
            match weather {
                Weather::Snow => {
                    p.phase += dt * 1.7;
                    p.vel.x = wind * 0.5 + p.phase.sin() * 22.0;
                }
                _ => p.vel.x += (wind - p.vel.x) * (dt * 2.0).min(1.0),
            }
            p.pos += p.vel * dt;
            let out = p.pos.y > view.y + view.h || p.pos.x < view.x || p.pos.x > view.x + view.w;
            if out {
                let at = vec2(gen_range(view.x, view.x + view.w), view.y - gen_range(0.0, 12.0));
                self.particles[i] = self.spawn(weather, at);
            }
        }
    }

    fn spawn(&self, weather: Weather, pos: Vec2) -> Particle {
        let (vel, size) = match weather {
            Weather::Snow => (vec2(0.0, gen_range(45.0, 95.0)), gen_range(1.4, 2.8)),
            _ => (vec2(self.gust * 40.0, gen_range(780.0, 1050.0)), gen_range(0.9, 1.5)),
        };
        Particle { pos, vel, size, phase: gen_range(0.0, std::f32::consts::TAU) }
    }

    /// Start a lightning bolt from the top of the view down to `to`.
    pub fn strike(&mut self, top: f32, to: Vec2) {
        let mut points = vec![vec2(to.x + gen_range(-80.0, 80.0), top)];
        let steps = (((to.y - top) / 28.0).ceil() as usize).max(2);
        for k in 1..steps {
            let u = k as f32 / steps as f32;
            let base = points[0].lerp(to, u);
            points.push(base + vec2(gen_range(-22.0, 22.0), 0.0));
        }
        points.push(to);
        self.bolt = Some(Bolt { points, age: 0.0 });
        self.flash = 1.0;
    }

    /// Rain, snow and lightning (world pass).
    pub fn draw(&self) {
        for p in &self.particles {
            match self.kind {
                Weather::Snow => {
                    draw_circle(p.pos.x, p.pos.y, p.size, Color::new(0.95, 0.97, 1.0, 0.8));
                }
                _ => {
                    let tail = p.pos - p.vel * 0.018;
                    draw_line(tail.x, tail.y, p.pos.x, p.pos.y, p.size, Color::new(0.72, 0.82, 1.0, 0.32));
                }
            }
        }
        if let Some(b) = &self.bolt {
            let k = 1.0 - b.age / BOLT_LIFE;
            // Flickers: off for a moment in the middle of its life.
            let on = if (b.age * 30.0) as i32 % 3 == 1 { 0.4 } else { 1.0 };
            for w in b.points.windows(2) {
                draw_line(w[0].x, w[0].y, w[1].x, w[1].y, 10.0, Color::new(0.6, 0.6, 1.0, 0.18 * k * on));
                draw_line(w[0].x, w[0].y, w[1].x, w[1].y, 3.0, Color::new(0.9, 0.9, 1.0, 0.9 * k * on));
                draw_line(w[0].x, w[0].y, w[1].x, w[1].y, 1.2, Color::new(1.0, 1.0, 1.0, k * on));
            }
        }
    }

    /// The lightning flash over the whole screen (screen pass).
    pub fn draw_flash(&self) {
        if self.flash > 0.0 {
            draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.85, 0.88, 1.0, self.flash * 0.35));
        }
    }

    #[cfg(test)]
    fn count(&self) -> usize {
        self.particles.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_cycles_and_fills_the_view() {
        assert_eq!(Weather::Clear.cycle(1), Weather::Rain);
        assert_eq!(Weather::Clear.cycle(-1), Weather::Storm);
        let mut sky = Sky::default();
        let view = Rect::new(0.0, 0.0, 800.0, 600.0);
        sky.update(0.016, Weather::Snow, view);
        assert_eq!(sky.count(), Weather::Snow.particles());
        for _ in 0..200 {
            sky.update(0.016, Weather::Rain, view);
        }
        assert_eq!(sky.count(), Weather::Rain.particles());
        sky.update(0.016, Weather::Clear, view);
        assert_eq!(sky.count(), 0);
        sky.strike(0.0, vec2(400.0, 500.0));
        assert!(sky.bolt.as_ref().is_some_and(|b| b.points.last() == Some(&vec2(400.0, 500.0))));
        assert_eq!(sky.flash, 1.0);
    }
}
