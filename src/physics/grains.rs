//! Grains poured with the Pour tool: sand that piles up, a liquid that
//! flows and levels out, and bouncy beads. Each grain is a tiny rapier ball
//! without an `Object` (no image, trail or menu), so there can be many.

use super::{to_phys, to_screen, PhysWorld};
use crate::config::PPM;
use macroquad::prelude::*;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::{HashSet, VecDeque};

/// The oldest grains are removed past this many.
pub const MAX_GRAINS: usize = 1600;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum GrainKind {
    #[default]
    Sand,
    Liquid,
    Beads,
}

impl GrainKind {
    pub const ALL: &'static [GrainKind] = &[GrainKind::Sand, GrainKind::Liquid, GrainKind::Beads];

    pub fn label(self) -> &'static str {
        match self {
            GrainKind::Sand => "Sand",
            GrainKind::Liquid => "Liquid",
            GrainKind::Beads => "Beads",
        }
    }

    pub fn hint(self) -> [&'static str; 2] {
        match self {
            GrainKind::Sand => ["Piles up and slides in heaps", "Hold to pour  ·  right-drag erases"],
            GrainKind::Liquid => ["Flows and finds its level", "Hold to pour  ·  right-drag erases"],
            GrainKind::Beads => ["Light and bouncy", "Hold to pour  ·  right-drag erases"],
        }
    }

    pub fn accent(self) -> Color {
        match self {
            GrainKind::Sand => Color::from_rgba(232, 196, 120, 255),
            GrainKind::Liquid => Color::from_rgba(70, 150, 245, 255),
            GrainKind::Beads => Color::from_rgba(250, 120, 190, 255),
        }
    }

    /// Radius in world pixels.
    pub fn radius_px(self) -> f32 {
        match self {
            GrainKind::Sand => 4.5,
            GrainKind::Liquid => 4.0,
            GrainKind::Beads => 5.5,
        }
    }

    /// (density, friction, restitution, linear damping, angular damping)
    fn physics(self) -> (f32, f32, f32, f32, f32) {
        match self {
            // Heavy spin damping stands in for rolling resistance, so sand heaps.
            GrainKind::Sand => (1.8, 0.95, 0.02, 0.1, 12.0),
            GrainKind::Liquid => (1.0, 0.0, 0.0, 0.25, 0.0),
            GrainKind::Beads => (0.7, 0.3, 0.75, 0.0, 0.5),
        }
    }
}

struct Grain {
    body: RigidBodyHandle,
    kind: GrainKind,
    /// Colour variation, 0‥1.
    tint: f32,
}

#[derive(Default)]
pub struct Grains {
    list: VecDeque<Grain>,
    /// Pour-rate accumulator (grains owed).
    owed: f32,
}

impl Grains {
    pub fn len(&self) -> usize {
        self.list.len()
    }

    pub fn is_empty(&self) -> bool {
        self.list.is_empty()
    }

    pub fn bodies(&self) -> impl Iterator<Item = RigidBodyHandle> + '_ {
        self.list.iter().map(|g| g.body)
    }

    /// Add one grain at `at` (world px), moving at `vel` (m/s).
    pub fn add(&mut self, world: &mut PhysWorld, kind: GrainKind, at: Vec2, vel: Vector<f32>) {
        let (x, y) = to_phys(at.x, at.y);
        let (density, friction, bounce, lin_damp, ang_damp) = kind.physics();
        let body = RigidBodyBuilder::dynamic()
            .translation(vector![x, y])
            .linvel(vel)
            .linear_damping(lin_damp)
            .angular_damping(ang_damp)
            .build();
        let body = world.bodies.insert(body);
        let col = ColliderBuilder::ball(kind.radius_px() / PPM)
            .density(density)
            .friction(friction)
            .restitution(bounce)
            .build();
        world.colliders.insert_with_parent(col, body, &mut world.bodies);
        let tint = rand::gen_range(0.0, 1.0);
        self.list.push_back(Grain { body, kind, tint });
        while self.list.len() > MAX_GRAINS {
            if let Some(g) = self.list.pop_front() {
                world.remove_body(g.body);
            }
        }
    }

    /// Pour for `dt` seconds at `rate` grains per second around `at`.
    /// Returns how many were added.
    pub fn pour(&mut self, world: &mut PhysWorld, kind: GrainKind, at: Vec2, rate: f32, dt: f32) -> usize {
        self.owed += rate * dt;
        let n = self.owed.floor() as usize;
        self.owed -= n as f32;
        let spread = kind.radius_px() * 3.0;
        for _ in 0..n {
            let a = rand::gen_range(0.0, std::f32::consts::TAU);
            let r = spread * rand::gen_range(0.0f32, 1.0).sqrt();
            let p = at + vec2(a.cos(), a.sin()) * r;
            let vel = vector![rand::gen_range(-0.15, 0.15), -rand::gen_range(0.5, 1.5)];
            self.add(world, kind, p, vel);
        }
        n
    }

    /// Remove grains within `radius` px of `at`. Returns how many.
    pub fn erase(&mut self, world: &mut PhysWorld, at: Vec2, radius: f32) -> usize {
        let before = self.list.len();
        self.list.retain(|g| {
            let hit = world.bodies.get(g.body).is_none_or(|b| {
                let t = b.translation();
                to_screen(t.x, t.y).distance(at) < radius
            });
            if hit {
                world.remove_body(g.body);
            }
            !hit
        });
        before - self.list.len()
    }

    /// Forget grains whose bodies were removed (lost off-screen…).
    pub fn remove(&mut self, world: &mut PhysWorld, dead: &HashSet<RigidBodyHandle>) {
        self.list.retain(|g| {
            let gone = dead.contains(&g.body);
            if gone {
                world.remove_body(g.body);
            }
            !gone
        });
    }

    pub fn clear(&mut self, world: &mut PhysWorld) {
        for g in self.list.drain(..) {
            world.remove_body(g.body);
        }
    }

    /// Draw in world space. The liquid is drawn as overlapping discs, dark
    /// and wide first, so neighbouring drops merge into one body of liquid.
    pub fn draw(&self, world: &PhysWorld) {
        let pos = |g: &Grain| world.bodies.get(g.body).map(|b| to_screen(b.translation().x, b.translation().y));
        let liquid = |g: &&Grain| g.kind == GrainKind::Liquid;
        for g in self.list.iter().filter(liquid) {
            if let Some(p) = pos(g) {
                draw_circle(p.x, p.y, 7.5, Color::from_rgba(28, 86, 196, 255));
            }
        }
        for g in self.list.iter().filter(liquid) {
            if let Some(p) = pos(g) {
                let speed = world.bodies[g.body].linvel().norm();
                // Fast drops look foamier.
                let c = Color::from_rgba(56, 136, 238, 255);
                let foam = (speed / 8.0).min(1.0) * 0.5;
                draw_circle(p.x, p.y, 5.2, Color::new(c.r + foam * 0.5, c.g + foam * 0.4, c.b, 1.0));
            }
        }
        for g in self.list.iter().filter(|g| g.kind != GrainKind::Liquid) {
            let Some(p) = pos(g) else { continue };
            let r = g.kind.radius_px();
            match g.kind {
                GrainKind::Sand => {
                    let k = 0.85 + g.tint * 0.25;
                    let c = g.kind.accent();
                    draw_circle(p.x, p.y, r + 0.6, Color::new(c.r * k, c.g * k, c.b * k * 0.95, 1.0));
                }
                _ => {
                    let (cr, cg, cb) = crate::util::hsv_to_rgb(g.tint * 360.0, 0.55, 1.0);
                    draw_circle(p.x, p.y, r, Color::from_rgba(cr, cg, cb, 255));
                    draw_circle(p.x - r * 0.3, p.y - r * 0.3, r * 0.35, Color::new(1.0, 1.0, 1.0, 0.6));
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::physics::borders::BorderMode;

    #[test]
    fn grains_are_capped_and_erasable() {
        let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (1200.0, 800.0));
        let mut g = Grains::default();
        let walls = w.bodies.len();
        for i in 0..(MAX_GRAINS + 50) {
            let at = vec2(100.0 + (i % 100) as f32 * 10.0, 100.0 + (i / 100) as f32 * 10.0);
            g.add(&mut w, GrainKind::Sand, at, vector![0.0, 0.0]);
        }
        assert_eq!(g.len(), MAX_GRAINS);
        assert_eq!(w.bodies.len(), walls + MAX_GRAINS, "the oldest bodies are removed too");
        let erased = g.erase(&mut w, vec2(600.0, 600.0), 5000.0);
        assert_eq!((erased, g.len()), (MAX_GRAINS, 0));
    }

    #[test]
    fn liquid_spreads_wider_than_sand() {
        let spread = |kind| {
            let mut w = PhysWorld::new(-9.8, BorderMode::Walls, (2400.0, 800.0));
            let mut g = Grains::default();
            for i in 0..240 {
                // Staggered rows: a perfect grid of balls would stand as a column.
                let x = 1200.0 + (i % 6) as f32 * 10.0 + if (i / 6) % 2 == 0 { 0.0 } else { 4.0 };
                let at = vec2(x, 300.0 + (i / 6) as f32 * 10.0);
                g.add(&mut w, kind, at, vector![0.0, 0.0]);
            }
            for _ in 0..600 {
                w.step_fixed();
            }
            let xs: Vec<f32> = g.bodies().map(|b| w.bodies[b].translation().x).collect();
            let lo = xs.iter().cloned().fold(f32::MAX, f32::min);
            let hi = xs.iter().cloned().fold(f32::MIN, f32::max);
            hi - lo
        };
        let (sand, liquid) = (spread(GrainKind::Sand), spread(GrainKind::Liquid));
        assert!(liquid > sand * 1.5, "liquid {liquid} m vs sand {sand} m");
    }
}
