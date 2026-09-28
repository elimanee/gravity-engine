//! A pool of water filling the bottom of the arena: buoyancy and drag on
//! submerged objects, and a surface of spring-coupled columns that ripples
//! when objects splash in (and with the music's bass).

use super::object::Object;
use super::{to_screen, PhysWorld};
use crate::config::{PPM, WALL_T};
use macroquad::prelude::*;
use rapier2d::prelude::*;
use std::collections::HashMap;

const COLS: usize = 120;
/// Wave speed² in columns²/s².
const WAVE_C2: f32 = 260.0;
const RESTORE: f32 = 3.0;
const DAMPING: f32 = 1.4;
/// Linear drag in the water, as an acceleration per m/s.
const DRAG: f32 = 2.0;
const ANGULAR_DRAG: f32 = 2.5;

pub struct Water {
    /// Surface displacement of each column (m).
    h: Vec<f32>,
    v: Vec<f32>,
    time: f32,
    /// Ambient wave amplitude (m), follows the music.
    swell: f32,
    /// Submerged fraction of each body last frame (to detect splashes).
    submerged: HashMap<RigidBodyHandle, f32>,
}

impl Default for Water {
    fn default() -> Self {
        Water { h: vec![0.0; COLS], v: vec![0.0; COLS], time: 0.0, swell: 0.0, submerged: HashMap::new() }
    }
}

/// Height of the still water surface (m) for a level in 0‥1.
pub fn rest_level(world: &PhysWorld, level: f32) -> f32 {
    let floor = if world.border.walls().floor { WALL_T } else { 0.0 };
    floor + level * (world.arena.1 - floor)
}

impl Water {
    fn column_of(&self, x: f32, width: f32) -> f32 {
        (x / width.max(0.1)).clamp(0.0, 1.0) * (COLS - 1) as f32
    }

    /// Surface height (m) at `x` (m).
    pub fn surface(&self, x: f32, width: f32, rest: f32) -> f32 {
        let c = self.column_of(x, width);
        let (i, t) = (c.floor() as usize, c.fract());
        let h = self.h[i] + (self.h[(i + 1).min(COLS - 1)] - self.h[i]) * t;
        let t = self.time;
        let ambient = (x * 0.9 + t * 1.7).sin() * 0.6 + (x * 2.3 - t * 2.6).sin() * 0.4;
        rest + h + ambient * (0.025 + self.swell)
    }

    fn splash(&mut self, x: f32, width: f32, impulse: f32) {
        let c = self.column_of(x, width).round() as i32;
        for d in -2i32..=2 {
            let i = (c + d).clamp(0, COLS as i32 - 1) as usize;
            self.v[i] += impulse * (1.0 - d.abs() as f32 * 0.3);
        }
    }

    /// Advance the surface. `bass` (0‥1) and `beat` come from the audio analyzer.
    pub fn step(&mut self, dt: f32, bass: f32, beat: bool) {
        self.time += dt;
        self.swell += ((bass * 0.16) - self.swell) * (1.0 - (-dt * 3.0).exp());
        if beat {
            let i = macroquad::rand::gen_range(0, COLS);
            self.v[i] -= 0.6 + bass * 1.5;
        }
        let n = ((dt / (1.0 / 240.0)).ceil() as usize).clamp(1, 16);
        let h = dt / n as f32;
        for _ in 0..n {
            for i in 0..COLS {
                let l = self.h[i.saturating_sub(1)];
                let r = self.h[(i + 1).min(COLS - 1)];
                let lap = l + r - 2.0 * self.h[i];
                self.v[i] += (WAVE_C2 * lap - RESTORE * self.h[i] - DAMPING * self.v[i]) * h;
            }
            for i in 0..COLS {
                self.h[i] = (self.h[i] + self.v[i] * h).clamp(-2.0, 2.0);
            }
        }
    }

    /// Buoyancy and drag on every dynamic object; splashes on entry.
    pub fn apply(&mut self, world: &mut PhysWorld, objects: &[Object], rest: f32, density: f32, dt: f32) {
        let width = world.arena.0;
        let gravity = world.gravity;
        let mut submerged = HashMap::with_capacity(objects.len());
        let mut splashes = vec![];
        for o in objects {
            let area = o.area(world);
            let Some(col) = world.colliders.get(o.collider) else { continue };
            let aabb = col.compute_aabb();
            let Some(b) = world.bodies.get_mut(o.body) else { continue };
            if !b.is_dynamic() {
                continue;
            }
            let x = b.translation().x;
            let surface = self.surface(x, width, rest);
            let height = (aabb.maxs.y - aabb.mins.y).max(0.01);
            let frac = ((surface - aabb.mins.y) / height).clamp(0.0, 1.0);
            let before = self.submerged.get(&o.body).copied().unwrap_or(frac);
            submerged.insert(o.body, frac);
            if frac <= 0.0 {
                continue;
            }
            let v = *b.linvel();
            if before <= 0.0 || (before >= 1.0 && frac < 1.0) {
                // Entering pushes the surface down, leaving pulls it up.
                let size = (aabb.maxs.x - aabb.mins.x).clamp(0.2, 4.0);
                splashes.push((x, (v.y * 0.12 * size.sqrt()).clamp(-2.5, 2.5)));
            }
            let m = b.mass();
            let buoyancy = -gravity * density * area * frac;
            let drag = -v * DRAG * m * frac;
            b.add_force(buoyancy + drag, true);
            let w = b.angvel();
            b.set_angvel(w * (-ANGULAR_DRAG * frac * dt).exp(), true);
        }
        self.submerged = submerged;
        for (x, imp) in splashes {
            self.splash(x, width, imp);
        }
    }

    pub fn draw(&self, world: &PhysWorld, rest: f32, sw: f32, sh: f32) {
        let width = world.arena.0;
        // Stay inside the walls and above the floor.
        let walls = world.border.walls();
        let side = if walls.sides { WALL_T * PPM } else { 0.0 };
        let bottom = if walls.floor { sh - WALL_T * PPM } else { sh };
        let (x0, x1) = (side, sw - side);
        let top_c = Color::new(0.24, 0.55, 1.0, 0.30);
        let deep_c = Color::new(0.06, 0.16, 0.45, 0.62);
        let deepest = to_screen(0.0, rest - 4.0).y.min(bottom);
        let n = COLS * 2;
        let mut vertices = Vec::with_capacity((n + 1) * 2);
        let mut surface_px = Vec::with_capacity(n + 1);
        for i in 0..=n {
            let x_px = x0 + (x1 - x0) * i as f32 / n as f32;
            let y = to_screen(0.0, self.surface(x_px / PPM, width, rest)).y;
            surface_px.push(vec2(x_px, y));
            let depth_t = ((bottom - y) / (bottom - deepest).max(1.0)).clamp(0.0, 1.0);
            let bottom_c = Color::new(
                top_c.r + (deep_c.r - top_c.r) * depth_t,
                top_c.g + (deep_c.g - top_c.g) * depth_t,
                top_c.b + (deep_c.b - top_c.b) * depth_t,
                top_c.a + (deep_c.a - top_c.a) * depth_t,
            );
            vertices.push(macroquad::models::Vertex::new(x_px, y, 0.0, 0.0, 0.0, top_c));
            vertices.push(macroquad::models::Vertex::new(x_px, bottom, 0.0, 0.0, 0.0, bottom_c));
        }
        let mut indices = Vec::with_capacity(n * 6);
        for i in 0..n as u16 {
            let (a, b, c, d) = (i * 2, i * 2 + 1, i * 2 + 2, i * 2 + 3);
            indices.extend_from_slice(&[a, b, c, b, d, c]);
        }
        draw_mesh(&Mesh { vertices, indices, texture: None });

        // Surface highlight and a faint second line just below.
        let t = self.time;
        for w in surface_px.windows(2) {
            let (a, b) = (w[0], w[1]);
            draw_line(a.x, a.y, b.x, b.y, 2.0, Color::new(0.70, 0.88, 1.0, 0.75));
            draw_line(a.x, a.y + 5.0, b.x, b.y + 5.0, 1.0, Color::new(0.55, 0.78, 1.0, 0.18));
        }
        // Glints drifting along the surface.
        for k in 0..14 {
            let u = (k as f32 * 0.137 + t * 0.013 * (1.0 + (k % 3) as f32)) % 1.0;
            let i = (u * n as f32) as usize;
            let p = surface_px[i.min(n)];
            let a = 0.25 + 0.25 * (t * 2.0 + k as f32).sin();
            draw_line(
                p.x - 6.0,
                p.y + 9.0 + (k % 4) as f32 * 7.0,
                p.x + 6.0,
                p.y + 9.0 + (k % 4) as f32 * 7.0,
                1.0,
                Color::new(0.8, 0.92, 1.0, a * 0.5),
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;

    fn float_height(density: f32) -> f32 {
        float_height_of(ColliderBuilder::cuboid(0.5, 0.5).density(density), 900)
    }

    fn float_height_of(collider: ColliderBuilder, steps: usize) -> f32 {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (1200.0, 720.0));
        let h = w.bodies.insert(RigidBodyBuilder::dynamic().translation(vector![10.0, 8.0]));
        let c = w.colliders.insert_with_parent(collider, h, &mut w.bodies);
        let o = Object::test_stub(h, c);
        let mut water = Water::default();
        let rest = 5.0;
        for _ in 0..steps {
            w.reset_forces();
            water.apply(&mut w, std::slice::from_ref(&o), rest, 1.6, 1.0 / 60.0);
            water.step(1.0 / 60.0, 0.0, false);
            w.step_fixed();
        }
        w.bodies[h].translation().y - rest
    }

    #[test]
    fn light_objects_float_heavy_ones_sink() {
        let light = float_height(1.0);
        assert!(light > -0.2 && light < 0.3, "a light box floats at the surface, offset {light}");
        let heavy = float_height(4.0);
        assert!(heavy < -3.0, "a heavy box sinks to the floor, offset {heavy}");
    }

    #[test]
    fn drawn_concave_shapes_float_too() {
        // A lumpy hand-drawn blob, decomposed into convex parts.
        let pts: Vec<_> = (0..20)
            .map(|i| {
                let a = i as f32 / 20.0 * std::f32::consts::TAU;
                let r = 0.75 * (1.0 + 0.25 * (a * 3.0).sin());
                point![a.cos() * r, a.sin() * r]
            })
            .collect();
        let idx: Vec<[u32; 2]> = (0..20).map(|i| [i, (i + 1) % 20]).collect();
        let offset = float_height_of(ColliderBuilder::convex_decomposition(&pts, &idx), 600);
        assert!(offset > -0.5, "the blob floats back up to the surface, offset {offset}");
    }
}
