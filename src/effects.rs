//! Particle effects: sparks and dust on hard impacts, droplets when objects
//! hit the water, debris when something shatters, confetti for a win.
//! Purely visual — particles never touch the physics.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use std::f32::consts::TAU;

/// Hard cap, so a huge pile-up cannot bring the frame rate down.
const MAX_PARTICLES: usize = 2500;

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    /// Bright streak that fades from white-hot to orange.
    Spark,
    /// Soft growing puff.
    Dust,
    /// Water droplet.
    Drop,
    /// Spinning triangle of debris.
    Shard,
    /// Spinning paper rectangle.
    Confetti,
}

struct Particle {
    kind: Kind,
    pos: Vec2,
    vel: Vec2,
    age: f32,
    life: f32,
    size: f32,
    color: Color,
    angle: f32,
    spin: f32,
}

#[derive(Default)]
pub struct Effects {
    items: Vec<Particle>,
}

fn dir(a: f32) -> Vec2 {
    vec2(a.cos(), a.sin())
}

impl Effects {
    fn push(&mut self, p: Particle) {
        if self.items.len() < MAX_PARTICLES {
            self.items.push(p);
        }
    }

    pub fn clear(&mut self) {
        self.items.clear();
    }

    #[cfg(test)]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    /// A hard hit at `at` (px): sparks spraying mostly along `normal` (screen
    /// space) and both ways along the surface. `speed` is the impact's speed
    /// change in m/s.
    pub fn sparks(&mut self, at: Vec2, normal: Vec2, speed: f32) {
        let n = ((speed - 5.0) * 1.6).clamp(3.0, 22.0) as usize;
        let base = normal.y.atan2(normal.x);
        for _ in 0..n {
            let side = if gen_range(0, 2) == 0 { 1.0 } else { -1.0 };
            let a = base + side * gen_range(0.6f32, 1.6);
            let v = gen_range(140.0f32, 260.0) * (speed / 10.0).clamp(0.6, 2.2);
            self.push(Particle {
                kind: Kind::Spark,
                pos: at,
                vel: dir(a) * v,
                age: 0.0,
                life: gen_range(0.18, 0.45),
                size: gen_range(1.2, 2.2),
                color: WHITE,
                angle: 0.0,
                spin: 0.0,
            });
        }
    }

    /// A puff of dust where something landed.
    pub fn dust(&mut self, at: Vec2, speed: f32) {
        let n = (speed * 1.2).clamp(3.0, 12.0) as usize;
        for _ in 0..n {
            let a = gen_range(0.0, TAU);
            self.push(Particle {
                kind: Kind::Dust,
                pos: at + dir(a) * gen_range(0.0, 6.0),
                vel: vec2(a.cos() * gen_range(20.0, 70.0), -gen_range(10.0, 50.0)),
                age: 0.0,
                life: gen_range(0.4, 0.8),
                size: gen_range(3.0, 6.0),
                color: Color::new(0.78, 0.76, 0.86, 0.35),
                angle: 0.0,
                spin: 0.0,
            });
        }
    }

    /// Droplets thrown up where an object enters (or leaves) the water.
    pub fn splash(&mut self, at: Vec2, strength: f32) {
        let n = (strength * 7.0).clamp(4.0, 26.0) as usize;
        for _ in 0..n {
            let a = -std::f32::consts::FRAC_PI_2 + gen_range(-0.9f32, 0.9);
            let v = gen_range(90.0f32, 230.0) * strength.clamp(0.5, 2.0);
            self.push(Particle {
                kind: Kind::Drop,
                pos: at + vec2(gen_range(-14.0, 14.0), 0.0),
                vel: dir(a) * v,
                age: 0.0,
                life: gen_range(0.5, 0.9),
                size: gen_range(1.6, 3.2),
                color: Color::new(0.62, 0.84, 1.0, 0.85),
                angle: 0.0,
                spin: 0.0,
            });
        }
    }

    /// Debris of a shattered object of colour `color`.
    pub fn debris(&mut self, at: Vec2, radius: f32, color: Color) {
        let n = (radius / 3.0).clamp(8.0, 30.0) as usize;
        for _ in 0..n {
            let a = gen_range(0.0, TAU);
            let r = gen_range(0.0, radius * 0.7);
            self.push(Particle {
                kind: Kind::Shard,
                pos: at + dir(a) * r,
                vel: dir(a) * gen_range(60.0, 220.0) + vec2(0.0, -60.0),
                age: 0.0,
                life: gen_range(0.6, 1.2),
                size: gen_range(2.5, 5.5),
                color,
                angle: gen_range(0.0, TAU),
                spin: gen_range(-12.0, 12.0),
            });
        }
        self.dust(at, 10.0);
    }

    /// Bomb flash: a ring of sparks.
    pub fn explosion(&mut self, at: Vec2, radius: f32) {
        for i in 0..36 {
            let a = i as f32 / 36.0 * TAU + gen_range(-0.08, 0.08);
            self.push(Particle {
                kind: Kind::Spark,
                pos: at,
                vel: dir(a) * gen_range(0.9, 1.6) * radius * 1.8,
                age: 0.0,
                life: gen_range(0.25, 0.5),
                size: gen_range(1.5, 2.6),
                color: WHITE,
                angle: 0.0,
                spin: 0.0,
            });
        }
    }

    /// Celebration burst.
    pub fn confetti(&mut self, at: Vec2) {
        for _ in 0..120 {
            let a = -std::f32::consts::FRAC_PI_2 + gen_range(-1.2f32, 1.2);
            let (r, g, b) = crate::util::hsv_to_rgb(gen_range(0.0, 1.0), 0.65, 1.0);
            self.push(Particle {
                kind: Kind::Confetti,
                pos: at,
                vel: dir(a) * gen_range(250.0, 620.0),
                age: 0.0,
                life: gen_range(1.8, 3.2),
                size: gen_range(4.0, 7.0),
                color: Color::from_rgba(r, g, b, 255),
                angle: gen_range(0.0, TAU),
                spin: gen_range(-9.0, 9.0),
            });
        }
    }

    /// Advance by `dt` seconds; `gravity` is in px/s², screen y down.
    pub fn update(&mut self, dt: f32, gravity: f32, floor_y: f32) {
        for p in &mut self.items {
            p.age += dt;
            let (g, drag) = match p.kind {
                Kind::Spark => (gravity * 0.5, 2.5),
                Kind::Dust => (-8.0, 3.5),
                Kind::Drop => (gravity, 0.3),
                Kind::Shard => (gravity, 0.6),
                Kind::Confetti => (gravity * 0.18, 2.2),
            };
            p.vel.y += g * dt;
            p.vel *= (-drag * dt).exp();
            if p.kind == Kind::Confetti {
                // Flutter sideways as it falls.
                p.vel.x += (p.age * 7.0 + p.spin).sin() * 60.0 * dt;
            }
            p.pos += p.vel * dt;
            p.angle += p.spin * dt;
            if p.kind == Kind::Dust {
                p.size += dt * 9.0;
            }
            // Debris and confetti settle on the floor instead of falling through.
            if matches!(p.kind, Kind::Shard | Kind::Confetti) && p.pos.y > floor_y {
                p.pos.y = floor_y;
                p.vel = vec2(p.vel.x * 0.4, -p.vel.y.abs() * 0.2);
                p.spin *= 0.5;
            }
        }
        self.items.retain(|p| p.age < p.life);
    }

    pub fn draw(&self) {
        for p in &self.items {
            let t = p.age / p.life;
            let fade = 1.0 - t;
            match p.kind {
                Kind::Spark => {
                    // White-hot → orange, drawn as a short streak along the velocity.
                    let c = Color::new(1.0, 1.0 - 0.5 * t, 0.75 - 0.6 * t, fade);
                    let tail = p.pos - p.vel * 0.025;
                    draw_line(tail.x, tail.y, p.pos.x, p.pos.y, p.size, c);
                }
                Kind::Dust => {
                    draw_circle(p.pos.x, p.pos.y, p.size, Color { a: p.color.a * fade, ..p.color });
                }
                Kind::Drop => {
                    draw_circle(p.pos.x, p.pos.y, p.size, Color { a: p.color.a * fade.sqrt(), ..p.color });
                }
                Kind::Shard => {
                    let c = Color { a: fade.min(1.0), ..p.color };
                    let s = p.size;
                    let pts = [0.0f32, 2.2, 4.1].map(|o| p.pos + dir(p.angle + o) * s);
                    draw_triangle(pts[0], pts[1], pts[2], c);
                }
                Kind::Confetti => {
                    let c = Color { a: (fade * 3.0).min(1.0), ..p.color };
                    // Rotating card: its apparent width follows the spin.
                    let w = p.size * (p.angle * 2.0).cos().abs().max(0.15);
                    draw_rectangle_ex(
                        p.pos.x,
                        p.pos.y,
                        w,
                        p.size * 0.6,
                        DrawRectangleParams { offset: vec2(0.5, 0.5), rotation: p.angle * 0.3, color: c },
                    );
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn particles_expire_and_are_capped() {
        let mut fx = Effects::default();
        for _ in 0..200 {
            fx.confetti(vec2(100.0, 100.0));
        }
        assert_eq!(fx.items.len(), MAX_PARTICLES);
        for _ in 0..400 {
            fx.update(1.0 / 60.0, 900.0, 700.0);
        }
        assert!(fx.is_empty());
    }

    #[test]
    fn debris_rests_on_the_floor() {
        let mut fx = Effects::default();
        fx.debris(vec2(100.0, 600.0), 40.0, RED);
        for _ in 0..30 {
            fx.update(1.0 / 60.0, 900.0, 650.0);
        }
        assert!(fx.items.iter().filter(|p| p.kind == Kind::Shard).all(|p| p.pos.y <= 650.0));
    }
}
