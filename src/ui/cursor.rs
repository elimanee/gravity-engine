//! World-space feedback: tool cursors, grab lines and explosion effects.

use super::icons;
use super::theme::*;
use crate::physics::tools::{Grab, Tool};
use crate::physics::{to_screen, PhysWorld};
use macroquad::prelude::*;
use std::f32::consts::TAU;

/// A detonation animation.
pub struct Blast {
    pub pos: Vec2,
    pub radius: f32,
    born: f64,
}

impl Blast {
    const DURATION: f32 = 0.6;

    pub fn new(pos: Vec2, radius: f32) -> Self {
        Blast { pos, radius, born: get_time() }
    }

    fn age(&self) -> f32 {
        (get_time() - self.born) as f32
    }

    pub fn alive(&self) -> bool {
        self.age() < Self::DURATION
    }

    pub fn draw(&self) {
        let t = (self.age() / Self::DURATION).min(1.0);
        let ease = 1.0 - (1.0 - t) * (1.0 - t);
        let (x, y) = (self.pos.x, self.pos.y);
        // Flash
        draw_circle(x, y, self.radius * 0.45 * ease, Color::new(1.0, 0.6, 0.25, 0.22 * (1.0 - ease)));
        for off in [0.0f32, 0.18, 0.36] {
            let rt = ((t + off) % 1.0).min(1.0);
            draw_circle_lines(
                x,
                y,
                self.radius * rt * 1.2,
                2.2 * (1.0 - rt) + 0.5,
                Color::new(1.0, 0.47, 0.16, 1.0 - rt),
            );
        }
        let burst = 1.0 - ease;
        if burst > 0.04 {
            let br = self.radius * 0.6 * ease;
            for i in 0..14 {
                let a = i as f32 * TAU / 14.0 + i as f32 * 0.3;
                draw_line(
                    x + a.cos() * br * 0.3,
                    y + a.sin() * br * 0.3,
                    x + a.cos() * br,
                    y + a.sin() * br,
                    1.6,
                    Color::new(1.0, 0.8, 0.32, burst * 0.86),
                );
            }
        }
        let core = 18.0 * (1.0 - ease);
        if core > 0.5 {
            draw_circle(x, y, core, Color::new(1.0, 0.95, 0.7, 0.8 * (1.0 - ease)));
        }
    }
}

/// Draw the rope / slingshot band of an active grab.
pub fn draw_grab(grab: &Grab, world: &PhysWorld, mouse: Vec2) {
    let Some(p) = grab.world_point(&world.bodies) else { return };
    let gp = to_screen(p.x, p.y);
    let c = grab.tool.accent();
    let kinematic = world.bodies.get(grab.body).is_some_and(|b| b.is_kinematic());
    if kinematic {
        draw_circle_lines(gp.x, gp.y, 8.0, 1.5, alpha(TEXT, 0.8));
        return;
    }
    match grab.tool {
        Tool::Slingshot => {
            let anchor = to_screen(grab.anchor.0, grab.anchor.1);
            let d = gp - anchor;
            let n = if d.length() > 1.0 { vec2(-d.y, d.x).normalize() * 5.0 } else { Vec2::ZERO };
            draw_line(anchor.x + n.x, anchor.y + n.y, gp.x + n.x, gp.y + n.y, 2.0, c);
            draw_line(anchor.x - n.x, anchor.y - n.y, gp.x - n.x, gp.y - n.y, 2.0, c);
            draw_circle(anchor.x, anchor.y, 4.0, c);
            // Predicted launch direction.
            let launch = anchor - (mouse - anchor);
            let steps = 10;
            for i in 0..steps {
                let u0 = i as f32 / steps as f32;
                if i % 2 == 0 {
                    let u1 = (i + 1) as f32 / steps as f32;
                    let a = gp.lerp(launch, u0);
                    let b = gp.lerp(launch, u1);
                    draw_line(a.x, a.y, b.x, b.y, 1.5, alpha(c, 0.6));
                }
            }
            draw_circle_lines(launch.x, launch.y, 5.0, 1.2, alpha(c, 0.6));
        }
        _ => {
            // Slightly sagging rope from grab point to cursor.
            let mid = (gp + mouse) / 2.0 + vec2(0.0, (gp.distance(mouse) * 0.08).min(18.0));
            let mut prev = gp;
            for i in 1..=12 {
                let u = i as f32 / 12.0;
                let p = gp.lerp(mid, u).lerp(mid.lerp(mouse, u), u);
                draw_line(prev.x, prev.y, p.x, p.y, 2.0, c);
                prev = p;
            }
            draw_circle(gp.x, gp.y, 3.5, c);
            draw_circle_lines(mouse.x, mouse.y, 5.0, 1.2, c);
        }
    }
}

/// Cursor decoration for area tools.
pub fn draw_tool_cursor(tool: Tool, mouse: Vec2, radius: f32, active: bool) {
    if !tool.has_settings() {
        return;
    }
    let t = get_time() as f32;
    let c = tool.accent();
    // Dashed radius ring.
    let segs = ((radius * TAU / 14.0) as usize).clamp(16, 160);
    let rot = t * if active { 0.8 } else { 0.25 };
    for i in (0..segs).step_by(2) {
        let a0 = rot + i as f32 / segs as f32 * TAU;
        let a1 = rot + (i + 1) as f32 / segs as f32 * TAU;
        draw_line(
            mouse.x + a0.cos() * radius,
            mouse.y + a0.sin() * radius,
            mouse.x + a1.cos() * radius,
            mouse.y + a1.sin() * radius,
            1.3,
            alpha(c, if active { 0.75 } else { 0.35 }),
        );
    }
    if active {
        draw_circle(mouse.x, mouse.y, radius, alpha(c, 0.05));
    }
    let pulse = if tool == Tool::Bomb { 0.75 + (t * 5.0).sin() * 0.25 } else { 1.0 };
    icons::tool(tool, mouse, 34.0, alpha(c, if active { 0.95 } else { 0.7 } * pulse), if active { t } else { t * 0.3 });
}
