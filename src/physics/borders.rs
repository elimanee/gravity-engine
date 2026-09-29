//! Arena border modes: which walls exist, what happens at the edges, and how
//! each mode is drawn.

use super::PhysWorld;
use crate::config::{CULL_MARGIN, PPM, WALL_T};
use crate::util::hsv_to_rgb;
use macroquad::prelude::*;
use rapier2d::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BorderMode {
    #[serde(alias = "Default")]
    Walls,
    Loop,
    Kill,
    Warp,
    Bounce,
    Repulse,
    Portal,
}

pub struct WallSpec {
    pub floor: bool,
    pub ceiling: bool,
    pub sides: bool,
}

impl BorderMode {
    pub const ALL: &'static [BorderMode] = &[
        BorderMode::Walls,
        BorderMode::Loop,
        BorderMode::Kill,
        BorderMode::Warp,
        BorderMode::Bounce,
        BorderMode::Repulse,
        BorderMode::Portal,
    ];

    pub fn label(self) -> &'static str {
        match self {
            BorderMode::Walls => "Walls",
            BorderMode::Loop => "Loop",
            BorderMode::Kill => "Kill",
            BorderMode::Warp => "Warp",
            BorderMode::Bounce => "Bounce",
            BorderMode::Repulse => "Repulse",
            BorderMode::Portal => "Portal",
        }
    }

    pub fn description(self) -> &'static str {
        match self {
            BorderMode::Walls => "Solid walls all around",
            BorderMode::Loop => "Objects wrap left ↔ right",
            BorderMode::Kill => "Objects leaving the sides are deleted",
            BorderMode::Warp => "Objects wrap on all four sides",
            BorderMode::Bounce => "Springy trampolines on the sides",
            BorderMode::Repulse => "Soft force field on every edge",
            BorderMode::Portal => "No floor — every edge is a portal",
        }
    }

    pub fn accent(self) -> Color {
        match self {
            BorderMode::Walls => Color::from_rgba(140, 132, 200, 255),
            BorderMode::Loop => Color::from_rgba(90, 170, 255, 255),
            BorderMode::Kill => Color::from_rgba(255, 80, 70, 255),
            BorderMode::Warp => Color::from_rgba(0, 220, 200, 255),
            BorderMode::Bounce => Color::from_rgba(80, 240, 120, 255),
            BorderMode::Repulse => Color::from_rgba(190, 100, 255, 255),
            BorderMode::Portal => {
                let (r, g, b) = hsv_to_rgb(get_time() as f32 * 0.4, 0.7, 1.0);
                Color::from_rgba(r, g, b, 255)
            }
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let n = Self::ALL.len() as i32;
        let i = Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as i32;
        Self::ALL[(i + dir).rem_euclid(n) as usize]
    }

    pub fn walls(self) -> WallSpec {
        WallSpec {
            floor: self != BorderMode::Portal,
            ceiling: !matches!(self, BorderMode::Warp | BorderMode::Portal),
            sides: matches!(self, BorderMode::Walls | BorderMode::Bounce | BorderMode::Repulse),
        }
    }

    /// Edge forces / impulses, applied before the physics step.
    pub fn apply_forces(self, world: &mut PhysWorld, bodies: &[RigidBodyHandle]) {
        let (sw, _) = world.arena;
        match self {
            BorderMode::Bounce => {
                let (margin, strength) = (1.5_f32, 120.0_f32);
                for &body in bodies {
                    let Some(b) = world.bodies.get_mut(body) else { continue };
                    if !b.is_dynamic() {
                        continue;
                    }
                    let (pos, vel) = (*b.translation(), *b.linvel());
                    let mut imp = 0.0;
                    if pos.x < margin && vel.x < 0.0 {
                        imp += strength * (margin - pos.x) / margin;
                    }
                    if pos.x > sw - margin && vel.x > 0.0 {
                        imp -= strength * (pos.x - (sw - margin)) / margin;
                    }
                    if imp != 0.0 {
                        // Scaled by mass so every object gets the same kick.
                        let m = b.mass() / 2.5;
                        b.apply_impulse(vector![imp * m, 0.0], true);
                    }
                }
            }
            BorderMode::Repulse => {
                let (zone, strength) = (3.0_f32, 80.0_f32);
                for &body in bodies {
                    let Some(b) = world.bodies.get_mut(body) else { continue };
                    if !b.is_dynamic() {
                        continue;
                    }
                    let pos = *b.translation();
                    let push = |d: f32| if d < zone { strength * (1.0 - d.max(0.0) / zone) } else { 0.0 };
                    let fx = push(pos.x) - push(sw - pos.x);
                    let fy = push(pos.y);
                    if fx != 0.0 || fy != 0.0 {
                        let m = b.mass() / 2.5;
                        b.add_force(vector![fx * m, fy * m], true);
                    }
                }
            }
            _ => {}
        }
    }

    /// Wrapping and culling, applied after the physics step. Returns the
    /// bodies that must be deleted.
    pub fn apply_positions(self, world: &mut PhysWorld, bodies: &[RigidBodyHandle]) -> Vec<RigidBodyHandle> {
        let (sw, sh) = world.arena;
        let wrap_x = matches!(self, BorderMode::Loop | BorderMode::Warp | BorderMode::Portal);
        let wrap_y = matches!(self, BorderMode::Warp | BorderMode::Portal);
        let mut dead = vec![];
        for &body in bodies {
            let Some(b) = world.bodies.get_mut(body) else { continue };
            let pos = *b.translation();
            let (mut nx, mut ny) = (pos.x, pos.y);
            if wrap_x {
                if pos.x < -1.0 {
                    nx = pos.x + sw + 1.0;
                } else if pos.x > sw + 1.0 {
                    nx = pos.x - sw - 1.0;
                }
            }
            if wrap_y {
                if pos.y < -1.0 {
                    ny = pos.y + sh + 1.0;
                } else if pos.y > sh + 1.0 {
                    ny = pos.y - sh - 1.0;
                }
            }
            if nx != pos.x || ny != pos.y {
                b.set_translation(vector![nx, ny], true);
            }
            let out_x = pos.x < -1.0 || pos.x > sw + 1.0;
            let lost =
                pos.x < -CULL_MARGIN || pos.x > sw + CULL_MARGIN || pos.y < -CULL_MARGIN || pos.y > sh + CULL_MARGIN;
            if (self == BorderMode::Kill && out_x) || lost {
                dead.push(body);
            }
        }
        dead
    }

    /// Draw the floor and the edge decoration of this mode.
    pub fn draw(self, sw: f32, sh: f32) {
        let ft = WALL_T * PPM;
        let t = get_time() as f32;
        let wall = Color::from_rgba(44, 42, 72, 255);
        let wall_hi = Color::from_rgba(96, 90, 150, 255);

        if self.walls().floor {
            draw_rectangle(0.0, sh - ft, sw, ft, wall);
            draw_line(0.0, sh - ft, sw, sh - ft, 1.5, wall_hi);
        }

        let spacing = 52.0_f32;
        let scroll = (t * 42.0) % spacing;
        let (cx_l, cx_r, cy_t, cy_b) = (ft * 0.5, sw - ft * 0.5, ft * 0.5, sh - ft * 0.5);
        let side_arrows = |col: Color| {
            let mut y = -scroll;
            while y < sh {
                draw_triangle(vec2(cx_l - 5.0, y - 4.0), vec2(cx_l - 5.0, y + 4.0), vec2(cx_l + 6.0, y), col);
                draw_triangle(vec2(cx_r + 5.0, y - 4.0), vec2(cx_r + 5.0, y + 4.0), vec2(cx_r - 6.0, y), col);
                y += spacing;
            }
        };
        let top_arrows = |col: Color| {
            let mut x = -scroll;
            while x < sw {
                draw_triangle(vec2(x - 4.0, cy_t + 5.0), vec2(x + 4.0, cy_t + 5.0), vec2(x, cy_t - 6.0), col);
                x += spacing;
            }
        };
        let with_a = |c: Color, a: f32| Color { a, ..c };

        match self {
            BorderMode::Walls => {
                draw_rectangle(0.0, 0.0, ft, sh, wall);
                draw_rectangle(sw - ft, 0.0, ft, sh, wall);
                draw_line(ft, 0.0, ft, sh - ft, 1.5, wall_hi);
                draw_line(sw - ft, 0.0, sw - ft, sh - ft, 1.5, wall_hi);
            }
            BorderMode::Loop => {
                let c = self.accent();
                draw_rectangle(0.0, 0.0, ft, sh, with_a(c, 0.18));
                draw_rectangle(sw - ft, 0.0, ft, sh, with_a(c, 0.18));
                side_arrows(with_a(c, 0.8));
            }
            BorderMode::Kill => {
                let pulse = (t * 3.0).sin() * 0.5 + 0.5;
                let c = self.accent();
                draw_rectangle(0.0, 0.0, ft, sh, with_a(c, 0.15 + pulse * 0.15));
                draw_rectangle(sw - ft, 0.0, ft, sh, with_a(c, 0.15 + pulse * 0.15));
                let mark = with_a(c, 0.5 + pulse * 0.35);
                let s = 5.0;
                let mut y = spacing * 0.5;
                while y < sh {
                    for cx in [cx_l, cx_r] {
                        draw_line(cx - s, y - s, cx + s, y + s, 1.5, mark);
                        draw_line(cx + s, y - s, cx - s, y + s, 1.5, mark);
                    }
                    y += spacing;
                }
            }
            BorderMode::Warp => {
                let c = self.accent();
                draw_rectangle(0.0, 0.0, ft, sh, with_a(c, 0.2));
                draw_rectangle(sw - ft, 0.0, ft, sh, with_a(c, 0.2));
                draw_rectangle(0.0, 0.0, sw, ft, with_a(c, 0.2));
                side_arrows(with_a(c, 0.8));
                top_arrows(with_a(c, 0.8));
            }
            BorderMode::Bounce => {
                let pulse = (t * 4.0).sin() * 0.4 + 0.6;
                let c = self.accent();
                draw_rectangle(0.0, 0.0, ft, sh, with_a(c, 0.24 * pulse));
                draw_rectangle(sw - ft, 0.0, ft, sh, with_a(c, 0.24 * pulse));
                draw_line(ft, 0.0, ft, sh, 1.5, with_a(c, 0.7 * pulse));
                draw_line(sw - ft, 0.0, sw - ft, sh, 1.5, with_a(c, 0.7 * pulse));
            }
            BorderMode::Repulse => {
                let pulse = (t * 2.5).sin() * 0.3 + 0.7;
                let c = self.accent();
                let z = ft * 3.0;
                crate::ui::theme::gradient_h(0.0, 0.0, z, sh, with_a(c, 0.25 * pulse), with_a(c, 0.0));
                crate::ui::theme::gradient_h(sw - z, 0.0, z, sh, with_a(c, 0.0), with_a(c, 0.25 * pulse));
                crate::ui::theme::gradient_v(0.0, sh - z, sw, z, with_a(c, 0.0), with_a(c, 0.25 * pulse));
            }
            BorderMode::Portal => {
                let hue = (t * 0.4) % 1.0;
                let (r, g, b) = hsv_to_rgb(hue, 0.8, 1.0);
                let (r2, g2, b2) = hsv_to_rgb(hue + 0.5, 0.8, 1.0);
                let c1 = Color::from_rgba(r, g, b, 255);
                let c2 = Color::from_rgba(r2, g2, b2, 255);
                draw_rectangle(0.0, 0.0, ft, sh, with_a(c1, 0.22));
                draw_rectangle(sw - ft, 0.0, ft, sh, with_a(c2, 0.22));
                draw_rectangle(0.0, 0.0, sw, ft, with_a(c1, 0.22));
                draw_rectangle(0.0, sh - ft, sw, ft, with_a(c2, 0.22));
                draw_line(ft, 0.0, ft, sh, 1.5, with_a(c1, 0.86));
                draw_line(sw - ft, 0.0, sw - ft, sh, 1.5, with_a(c2, 0.86));
                draw_line(0.0, ft, sw, ft, 1.5, with_a(c1, 0.86));
                draw_line(0.0, sh - ft, sw, sh - ft, 1.5, with_a(c2, 0.86));
                side_arrows(with_a(c1, 0.8));
                top_arrows(with_a(c1, 0.8));
                let mut x = -scroll;
                while x < sw {
                    draw_triangle(
                        vec2(x - 4.0, cy_b - 5.0),
                        vec2(x + 4.0, cy_b - 5.0),
                        vec2(x, cy_b + 6.0),
                        with_a(c2, 0.8),
                    );
                    x += spacing;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_wraps_both_ways() {
        assert_eq!(BorderMode::Walls.cycle(-1), BorderMode::Portal);
        assert_eq!(BorderMode::Portal.cycle(1), BorderMode::Walls);
    }

    #[test]
    fn legacy_name_deserialises() {
        let m: BorderMode = serde_json::from_str("\"Default\"").unwrap();
        assert_eq!(m, BorderMode::Walls);
    }
}
