//! Vector icons drawn with primitives (no image assets needed).

use crate::physics::tools::Tool;
use crate::shapes::Shape;
use macroquad::prelude::*;
use std::f32::consts::{FRAC_PI_2, PI, TAU};

/// Draw the icon of `tool` centred on `c`, fitting in a `size` × `size` box.
pub fn tool(tool: Tool, c: Vec2, size: f32, color: Color, t: f32) {
    let s = size / 48.0; // icons are designed on a 48 px grid
    let th = (2.0 * s).max(1.2);
    let soft = Color { a: color.a * 0.4, ..color };
    match tool {
        Tool::Spring => {
            // Coil from the hand to a small box.
            let n = 5;
            let (x0, x1) = (c.x - 16.0 * s, c.x + 8.0 * s);
            let mut prev = vec2(x0, c.y);
            for i in 1..=n * 2 {
                let x = x0 + (x1 - x0) * i as f32 / (n * 2) as f32;
                let y = c.y
                    + if i % 2 == 0 {
                        0.0
                    } else {
                        if i % 4 == 1 {
                            -6.0
                        } else {
                            6.0
                        }
                    } * s;
                let p = vec2(x, y);
                draw_line(prev.x, prev.y, p.x, p.y, th, color);
                prev = p;
            }
            draw_rectangle(c.x + 9.0 * s, c.y - 8.0 * s, 14.0 * s, 16.0 * s, color);
            draw_circle(x0 - 2.0 * s, c.y, 4.0 * s, color);
        }
        Tool::Slingshot => {
            draw_line(c.x, c.y + 18.0 * s, c.x, c.y + 2.0 * s, th * 1.4, color);
            draw_line(c.x, c.y + 2.0 * s, c.x - 12.0 * s, c.y - 14.0 * s, th * 1.4, color);
            draw_line(c.x, c.y + 2.0 * s, c.x + 12.0 * s, c.y - 14.0 * s, th * 1.4, color);
            let pull = 4.0 * s + (t * 3.0).sin().abs() * 4.0 * s;
            let stone = vec2(c.x, c.y - 4.0 * s + pull);
            draw_line(c.x - 12.0 * s, c.y - 14.0 * s, stone.x, stone.y, th * 0.7, soft);
            draw_line(c.x + 12.0 * s, c.y - 14.0 * s, stone.x, stone.y, th * 0.7, soft);
            draw_circle(stone.x, stone.y, 4.5 * s, color);
        }
        Tool::Pull => {
            for (i, r) in [18.0, 11.0].iter().enumerate() {
                let r = r * s * (1.0 - ((t * 1.2 + i as f32 * 0.5) % 1.0) * 0.3);
                draw_circle_lines(c.x, c.y, r, th * 0.8, if i == 0 { soft } else { color });
            }
            for i in 0..4 {
                let a = i as f32 * FRAC_PI_2 + PI / 4.0;
                let (ca, sa) = (a.cos(), a.sin());
                let tip = c + vec2(ca, sa) * 8.0 * s;
                draw_line(c.x + ca * 20.0 * s, c.y + sa * 20.0 * s, tip.x, tip.y, th, color);
            }
            draw_circle(c.x, c.y, 3.5 * s, color);
        }
        Tool::Push => {
            for i in 0..8 {
                let a = i as f32 * TAU / 8.0;
                let r0 = 7.0 * s;
                let r1 = 16.0 * s + ((t * 3.0 + i as f32).sin() * 2.0 * s);
                draw_line(c.x + a.cos() * r0, c.y + a.sin() * r0, c.x + a.cos() * r1, c.y + a.sin() * r1, th, color);
            }
            draw_circle(c.x, c.y, 4.0 * s, color);
        }
        Tool::Vortex => {
            let turns = 2.2;
            let steps = 40;
            let mut prev = c;
            for i in 1..=steps {
                let u = i as f32 / steps as f32;
                let a = u * turns * TAU + t * 3.0;
                let p = c + vec2(a.cos(), a.sin()) * u * 19.0 * s;
                draw_line(prev.x, prev.y, p.x, p.y, th, Color { a: color.a * (0.3 + 0.7 * u), ..color });
                prev = p;
            }
        }
        Tool::Freeze => {
            for i in 0..6 {
                let a = i as f32 * PI / 3.0 + FRAC_PI_2;
                let (ca, sa) = (a.cos(), a.sin());
                draw_line(c.x, c.y, c.x + ca * 19.0 * s, c.y + sa * 19.0 * s, th, color);
                let b = c + vec2(ca, sa) * 12.0 * s;
                let (pa, pb) = ((a + 0.9).cos(), (a + 0.9).sin());
                let (qa, qb) = ((a - 0.9).cos(), (a - 0.9).sin());
                draw_line(b.x, b.y, b.x + pa * 6.0 * s, b.y + pb * 6.0 * s, th * 0.8, color);
                draw_line(b.x, b.y, b.x + qa * 6.0 * s, b.y + qb * 6.0 * s, th * 0.8, color);
            }
        }
        Tool::Orbit => {
            draw_circle_lines(c.x, c.y, 16.0 * s, th * 0.7, soft);
            draw_circle(c.x, c.y, 5.0 * s, color);
            let a = t * 2.5;
            draw_circle(c.x + a.cos() * 16.0 * s, c.y + a.sin() * 16.0 * s, 4.0 * s, color);
            let b = a + PI;
            draw_circle(c.x + b.cos() * 16.0 * s, c.y + b.sin() * 16.0 * s, 2.5 * s, soft);
        }
        Tool::Bomb => {
            draw_circle(c.x - 2.0 * s, c.y + 4.0 * s, 13.0 * s, color);
            draw_line(c.x + 6.0 * s, c.y - 6.0 * s, c.x + 12.0 * s, c.y - 13.0 * s, th * 1.3, color);
            let flick = 0.6 + (t * 12.0).sin().abs() * 0.4;
            let spark = Color::new(1.0, 0.85, 0.4, color.a * flick);
            for i in 0..5 {
                let a = i as f32 * TAU / 5.0 + t * 4.0;
                let p = vec2(c.x + 13.0 * s, c.y - 15.0 * s);
                draw_line(p.x, p.y, p.x + a.cos() * 5.0 * s, p.y + a.sin() * 5.0 * s, th * 0.7, spark);
            }
            draw_circle(c.x - 7.0 * s, c.y, 3.5 * s, Color::new(1.0, 1.0, 1.0, color.a * 0.35));
        }
        Tool::Swing => {
            // A box hanging by one corner from the cursor, swinging and turning.
            let pivot = vec2(c.x, c.y - 17.0 * s);
            let sway = (t * 2.6).sin() * 0.55;
            let dir = vec2(sway.sin(), sway.cos());
            let corner = pivot + dir * 16.0 * s;
            draw_line(pivot.x, pivot.y, corner.x, corner.y, th, soft);
            draw_circle(pivot.x, pivot.y, 3.0 * s, color);
            // Square hanging from `corner`, rotated with the swing plus a spin.
            let half = 7.0 * s;
            let rot = sway * 1.8 + std::f32::consts::FRAC_PI_4;
            let centre = corner + dir * half * std::f32::consts::SQRT_2;
            draw_poly(centre.x, centre.y, 4, half * std::f32::consts::SQRT_2, rot.to_degrees(), color);
            // Motion arcs showing the spin.
            for i in 0..3 {
                let a0 = rot + 0.6 + i as f32 * 0.35;
                let r = 13.0 * s;
                let p0 = centre + vec2(a0.cos(), a0.sin()) * r;
                let p1 = centre + vec2((a0 + 0.25).cos(), (a0 + 0.25).sin()) * r;
                draw_line(p0.x, p0.y, p1.x, p1.y, th * 0.7, Color { a: color.a * (0.6 - i as f32 * 0.15), ..color });
            }
        }
        Tool::Draw => {
            // A pencil tracing a wavy line that grows over time.
            let grow = 0.35 + 0.65 * ((t * 0.8) % 1.0);
            let wave = |u: f32| vec2(c.x - 19.0 * s + u * 30.0 * s, c.y + 12.0 * s - (u * 7.0).sin() * 5.0 * s);
            let steps = 16;
            let mut prev = wave(0.0);
            for i in 1..=steps {
                let p = wave(i as f32 / steps as f32 * grow);
                draw_line(prev.x, prev.y, p.x, p.y, th * 1.3, soft);
                prev = p;
            }
            let tip = prev;
            let dir = vec2(0.55, -0.83);
            let back = tip + dir * 26.0 * s;
            let n = vec2(-dir.y, dir.x) * 4.5 * s;
            let cone = tip + dir * 8.0 * s;
            draw_triangle(tip, cone + n, cone - n, color);
            draw_triangle(cone + n, cone - n, back + n, color);
            draw_triangle(cone - n, back - n, back + n, color);
            draw_circle(tip.x, tip.y, 1.6 * s, Color::new(0.1, 0.1, 0.12, color.a));
        }
        Tool::Select => {
            // Marching-ants box with an arrow pointer.
            let r = Rect::new(c.x - 17.0 * s, c.y - 14.0 * s, 26.0 * s, 22.0 * s);
            let corners = [vec2(r.x, r.y), vec2(r.x + r.w, r.y), vec2(r.x + r.w, r.y + r.h), vec2(r.x, r.y + r.h)];
            let shift = (t * 2.0) % 1.0;
            for i in 0..4 {
                let (a, b) = (corners[i], corners[(i + 1) % 4]);
                for k in 0..4 {
                    let u0 = (k as f32 + shift) / 4.0;
                    let u1 = (u0 + 0.125).min(1.0);
                    let (p0, p1) = (a.lerp(b, u0.min(1.0)), a.lerp(b, u1));
                    draw_line(p0.x, p0.y, p1.x, p1.y, th * 0.8, soft);
                }
            }
            let tip = vec2(c.x + 2.0 * s, c.y - 2.0 * s);
            draw_triangle(tip, tip + vec2(0.0, 18.0 * s), tip + vec2(12.0 * s, 12.0 * s), color);
            draw_line(tip.x + 5.0 * s, tip.y + 13.0 * s, tip.x + 9.0 * s, tip.y + 20.0 * s, th * 1.4, color);
        }
        Tool::Knife => {
            // A blade with a slash behind it.
            let a = vec2(c.x - 14.0 * s, c.y + 12.0 * s);
            let b = vec2(c.x + 14.0 * s, c.y - 14.0 * s);
            let n = vec2(0.72, 0.69) * 4.0 * s;
            draw_triangle(b, b.lerp(a, 0.62) + n, b.lerp(a, 0.62) - n * 0.3, color);
            let handle = b.lerp(a, 0.62);
            draw_line(handle.x, handle.y, a.x, a.y, th * 2.2, soft);
            let t2 = (t * 1.3) % 1.0;
            let (p, q) = (vec2(c.x - 16.0 * s, c.y - 4.0 * s), vec2(c.x + 10.0 * s, c.y + 14.0 * s));
            let (p0, p1) = (p.lerp(q, (t2 - 0.3).max(0.0)), p.lerp(q, t2));
            draw_line(p0.x, p0.y, p1.x, p1.y, th * 0.8, soft);
        }
        Tool::Gadget => {
            // A laser bouncing off a mirror.
            let em = vec2(c.x - 14.0 * s, c.y + 10.0 * s);
            draw_rectangle(em.x - 6.0 * s, em.y - 4.0 * s, 12.0 * s, 8.0 * s, color);
            let m = vec2(c.x + 8.0 * s, c.y + 10.0 * s);
            let flick = 0.7 + 0.3 * (t * 9.0).sin().abs();
            let beam = Color { a: color.a * flick, ..color };
            draw_line(em.x + 6.0 * s, em.y, m.x, m.y, th, beam);
            draw_line(m.x, m.y, c.x + 16.0 * s, c.y - 16.0 * s, th, beam);
            draw_line(m.x - 7.0 * s, m.y + 7.0 * s, m.x + 7.0 * s, m.y - 7.0 * s, th * 1.6, soft);
            draw_circle(c.x + 16.0 * s, c.y - 16.0 * s, 3.0 * s, color);
        }
        Tool::Fire => {
            // A flickering flame on a match.
            let flick = (t * 9.0).sin() * 1.5 * s;
            let base = vec2(c.x, c.y + 6.0 * s);
            let tip = vec2(c.x + flick, c.y - 18.0 * s);
            let w = 10.0 * s;
            draw_circle(base.x, base.y, w, color);
            draw_triangle(vec2(base.x - w, base.y), vec2(base.x + w, base.y), tip, color);
            let core = Color::new(1.0, 0.95, 0.7, color.a);
            let ctip = vec2(c.x + flick * 0.5, c.y - 6.0 * s);
            draw_circle(base.x, base.y + 2.0 * s, w * 0.5, core);
            draw_triangle(
                vec2(base.x - w * 0.5, base.y + 2.0 * s),
                vec2(base.x + w * 0.5, base.y + 2.0 * s),
                ctip,
                core,
            );
            draw_line(c.x, c.y + 16.0 * s, c.x + 10.0 * s, c.y + 22.0 * s, th * 1.6, soft);
        }
        Tool::Pour => {
            // A tilted cup pouring a stream onto a heap.
            let cup = [
                vec2(c.x - 16.0 * s, c.y - 14.0 * s),
                vec2(c.x - 4.0 * s, c.y - 18.0 * s),
                vec2(c.x - 1.0 * s, c.y - 8.0 * s),
                vec2(c.x - 12.0 * s, c.y - 4.0 * s),
            ];
            draw_triangle(cup[0], cup[1], cup[2], color);
            draw_triangle(cup[0], cup[2], cup[3], color);
            for k in 0..4 {
                let u = ((t * 1.6 + k as f32 * 0.25) % 1.0) * 14.0 * s;
                draw_circle(c.x + 1.0 * s, c.y - 6.0 * s + u, 1.6 * s, color);
            }
            for (dx, dy) in [(-6.0, 0.0), (0.0, 0.0), (6.0, 0.0), (-3.0, -4.5), (3.0, -4.5), (0.0, -9.0)] {
                draw_circle(c.x + (dx + 2.0) * s, c.y + (14.0 + dy) * s, 2.6 * s, soft);
            }
        }
        Tool::Zone => {
            // A dashed frame with wind streaks blowing through it.
            let r = Rect::new(c.x - 17.0 * s, c.y - 13.0 * s, 34.0 * s, 26.0 * s);
            let corners = [vec2(r.x, r.y), vec2(r.x + r.w, r.y), vec2(r.x + r.w, r.y + r.h), vec2(r.x, r.y + r.h)];
            for i in 0..4 {
                let (a, b) = (corners[i], corners[(i + 1) % 4]);
                for k in (0..6).step_by(2) {
                    let (p0, p1) = (a.lerp(b, k as f32 / 6.0), a.lerp(b, (k + 1) as f32 / 6.0));
                    draw_line(p0.x, p0.y, p1.x, p1.y, th * 0.8, soft);
                }
            }
            for k in 0..3 {
                let y = c.y + (k as f32 - 1.0) * 7.0 * s;
                let x = c.x - 12.0 * s + ((t * 1.5 + k as f32 * 0.3) % 1.0) * 10.0 * s;
                draw_line(x, y, x + 12.0 * s, y, th, color);
            }
        }
        Tool::Link => {
            // Two blocks joined by a swaying rope.
            let a = vec2(c.x - 14.0 * s, c.y - 8.0 * s);
            let sway = (t * 2.2).sin() * 4.0 * s;
            let b = vec2(c.x + 12.0 * s + sway * 0.3, c.y + 10.0 * s);
            draw_rectangle(a.x - 7.0 * s, a.y - 7.0 * s, 12.0 * s, 12.0 * s, color);
            draw_rectangle(b.x - 5.0 * s, b.y - 4.0 * s, 12.0 * s, 12.0 * s, color);
            let mid = (a + b) / 2.0 + vec2(-sway, 7.0 * s);
            let mut prev = a;
            for i in 1..=10 {
                let u = i as f32 / 10.0;
                let p = a.lerp(mid, u).lerp(mid.lerp(b, u), u);
                draw_line(prev.x, prev.y, p.x, p.y, th, color);
                prev = p;
            }
            for p in [a, b] {
                draw_circle(p.x, p.y, 2.2 * s, Color::new(0.1, 0.1, 0.14, color.a));
            }
        }
    }
}

/// A five-pointed star, gold when `filled`.
pub fn star(c: Vec2, size: f32, filled: bool) {
    let (outer, inner) = (size / 2.0, size / 4.6);
    let pts: Vec<Vec2> = (0..10)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let r = if i % 2 == 0 { outer } else { inner };
            c + vec2(a.cos(), a.sin()) * r
        })
        .collect();
    let colour = if filled { Color::from_rgba(255, 200, 60, 255) } else { Color::from_rgba(255, 255, 255, 40) };
    for i in 0..10 {
        draw_triangle(c, pts[i], pts[(i + 1) % 10], colour);
    }
    if filled {
        for i in 0..10 {
            let (a, b) = (pts[i], pts[(i + 1) % 10]);
            draw_line(a.x, a.y, b.x, b.y, 1.0, Color::from_rgba(200, 130, 20, 200));
        }
    }
}

/// Small glyph for a grain kind: a little heap, puddle or scatter.
pub fn grain_kind(kind: crate::physics::grains::GrainKind, c: Vec2, size: f32, color: Color, t: f32) {
    use crate::physics::grains::GrainKind;
    let s = size / 24.0;
    match kind {
        GrainKind::Sand => {
            for (dx, dy) in [(-6.0, 5.0), (0.0, 5.0), (6.0, 5.0), (-3.0, 0.0), (3.0, 0.0), (0.0, -5.0)] {
                draw_circle(c.x + dx * s, c.y + dy * s, 2.8 * s, color);
            }
        }
        GrainKind::Liquid => {
            let top = c.y + 1.0 * s;
            draw_rectangle(c.x - 10.0 * s, top, 20.0 * s, 7.0 * s, color);
            for k in 0..5 {
                let x = c.x - 10.0 * s + k as f32 * 5.0 * s;
                draw_circle(x + 2.5 * s, top + (t * 3.0 + k as f32).sin() * 1.2 * s, 2.8 * s, color);
            }
            draw_circle(c.x + 2.0 * s, c.y - 7.0 * s, 2.5 * s, color);
        }
        GrainKind::Beads => {
            for (i, (dx, dy)) in [(-7.0, 4.0), (1.0, 6.0), (7.0, -1.0), (-2.0, -5.0)].iter().enumerate() {
                let hop = ((t * 4.0 + i as f32 * 1.3).sin() * 2.0).abs() * s;
                draw_circle(c.x + dx * s, c.y + dy * s - hop, 3.2 * s, color);
            }
        }
    }
}

/// Small glyph for a link kind.
pub fn link_kind(kind: crate::physics::links::LinkKind, c: Vec2, size: f32, color: Color) {
    use crate::physics::links::LinkKind;
    let s = size / 24.0;
    let (a, b) = (vec2(c.x - 9.0 * s, c.y - 6.0 * s), vec2(c.x + 9.0 * s, c.y + 6.0 * s));
    match kind {
        LinkKind::Rope => {
            let mid = (a + b) / 2.0 + vec2(-3.0 * s, 6.0 * s);
            let mut prev = a;
            for i in 1..=8 {
                let u = i as f32 / 8.0;
                let p = a.lerp(mid, u).lerp(mid.lerp(b, u), u);
                draw_line(prev.x, prev.y, p.x, p.y, 1.8 * s, color);
                prev = p;
            }
        }
        LinkKind::Spring => {
            let d = b - a;
            let n = vec2(-d.y, d.x).normalize() * 3.5 * s;
            let mut prev = a;
            for i in 1..=9 {
                let u = i as f32 / 9.0;
                let side = if i == 9 {
                    0.0
                } else if i % 2 == 1 {
                    1.0
                } else {
                    -1.0
                };
                let p = a + d * u + n * side;
                draw_line(prev.x, prev.y, p.x, p.y, 1.6 * s, color);
                prev = p;
            }
        }
        LinkKind::Glue => {
            // A tube with a drop.
            draw_rectangle(c.x - 9.0 * s, c.y - 3.0 * s, 12.0 * s, 7.0 * s, color);
            draw_triangle(
                vec2(c.x + 3.0 * s, c.y - 3.0 * s),
                vec2(c.x + 3.0 * s, c.y + 4.0 * s),
                vec2(c.x + 8.0 * s, c.y),
                color,
            );
            draw_circle(c.x + 9.0 * s, c.y + 6.0 * s, 2.5 * s, color);
            return;
        }
        LinkKind::Motor => {
            draw_circle_lines(c.x, c.y, 8.0 * s, 2.0 * s, color);
            // Arrow head on the ring shows the turning direction.
            let (p, q) = (c + vec2(8.0 * s, 0.0), c + vec2(8.0 * s, 5.0 * s));
            draw_triangle(p + vec2(-3.5 * s, 0.0), p + vec2(3.5 * s, 0.0), q, color);
            for i in 0..3 {
                let a = i as f32 * std::f32::consts::TAU / 3.0 + 0.4;
                draw_line(c.x, c.y, c.x + a.cos() * 6.0 * s, c.y + a.sin() * 6.0 * s, 1.6 * s, color);
            }
            return;
        }
        LinkKind::Hinge | LinkKind::Limb => {
            draw_rectangle_ex(
                c.x,
                c.y,
                18.0 * s,
                6.0 * s,
                DrawRectangleParams {
                    offset: vec2(0.1, 0.5),
                    rotation: -0.5,
                    color: Color { a: color.a * 0.5, ..color },
                },
            );
            draw_rectangle_ex(
                c.x,
                c.y,
                18.0 * s,
                6.0 * s,
                DrawRectangleParams {
                    offset: vec2(0.1, 0.5),
                    rotation: 0.4,
                    color: Color { a: color.a * 0.8, ..color },
                },
            );
            draw_circle(c.x, c.y, 3.5 * s, color);
            draw_circle(c.x, c.y, 1.4 * s, Color::new(0.1, 0.1, 0.14, color.a));
            return;
        }
    }
    for p in [a, b] {
        draw_circle(p.x, p.y, 2.4 * s, color);
    }
}

/// Filled silhouette of a spawn shape.
pub fn shape(shape: Shape, c: Vec2, size: f32, color: Color) {
    let r = size / 2.0;
    match shape {
        Shape::Circle => draw_circle(c.x, c.y, r, color),
        Shape::Box => super::theme::rounded_rect(c.x - r, c.y - r, size, size, size * 0.12, color),
        Shape::Capsule => super::theme::rounded_rect(c.x - r, c.y - r * 0.5, size, r, r * 0.5, color),
        s => {
            let pts: Vec<Vec2> =
                s.polygon().unwrap_or_default().iter().map(|[u, v]| c + vec2(*u, -*v) * size).collect();
            // Fan from the centre works for these star-shaped polygons.
            for i in 0..pts.len() {
                draw_triangle(c, pts[i], pts[(i + 1) % pts.len()], color);
            }
        }
    }
}

/// Small "pin" glyph.
pub fn pin(c: Vec2, size: f32, color: Color) {
    let s = size / 16.0;
    draw_circle(c.x, c.y - 3.0 * s, 4.5 * s, color);
    draw_line(c.x, c.y - 1.0 * s, c.x, c.y + 7.0 * s, 1.6 * s, color);
    draw_circle(c.x - 1.3 * s, c.y - 4.2 * s, 1.4 * s, Color::new(1.0, 1.0, 1.0, color.a * 0.6));
}

/// The app logo: a glowing core with orbiting blocks.
pub fn logo(c: Vec2, scale: f32, t: f32, alpha: f32) {
    let a = |v: f32| v * alpha;
    for i in 0..8 {
        let ang = i as f32 * TAU / 8.0;
        for seg in 0..3 {
            let f = seg as f32 / 3.0;
            let (r0, r1) = ((18.0 + f * 38.0) * scale, (28.0 + f * 38.0) * scale);
            draw_line(
                c.x + r0 * ang.cos(),
                c.y + r0 * ang.sin(),
                c.x + r1 * ang.cos(),
                c.y + r1 * ang.sin(),
                1.0,
                Color::new(0.47, 0.39, 0.82, a((1.0 - f) * 0.15)),
            );
        }
    }
    for orbit in [33.0f32, 47.0, 61.0] {
        let n = (orbit * 1.1) as usize;
        for j in 0..n {
            let ang = j as f32 / n as f32 * TAU;
            draw_circle(
                c.x + orbit * scale * ang.cos(),
                c.y + orbit * scale * ang.sin(),
                0.9 * scale.max(1.0),
                Color::new(0.35, 0.31, 0.61, a(0.18)),
            );
        }
    }
    // (orbit radius, speed, phase, w, h, r, g, b)
    type Orb = (f32, f32, f32, f32, f32, u8, u8, u8);
    const ORBS: &[Orb] = &[
        (33.0, 0.70, 0.00, 12.0, 9.0, 100, 185, 255),
        (47.0, -0.50, 1.30, 13.0, 11.0, 255, 150, 65),
        (61.0, 0.38, 3.10, 11.0, 13.0, 170, 95, 255),
        (40.0, 1.35, 4.70, 9.0, 9.0, 75, 215, 140),
        (54.0, -0.85, 2.20, 12.0, 10.0, 255, 95, 140),
    ];
    for &(r, spd, ph, w, h, cr, cg, cb) in ORBS {
        let (r, w, h) = (r * scale, w * scale, h * scale);
        let ang = t * spd + ph;
        let o = c + vec2(ang.cos(), ang.sin()) * r;
        let ta = ang - spd.signum() * 0.28;
        let tr = c + vec2(ta.cos(), ta.sin()) * r;
        draw_rectangle(
            tr.x - w * 0.35,
            tr.y - h * 0.35,
            w * 0.7,
            h * 0.7,
            Color::from_rgba(cr, cg, cb, (55.0 * alpha) as u8),
        );
        super::theme::rounded_rect(
            o.x - w / 2.0,
            o.y - h / 2.0,
            w,
            h,
            2.0 * scale,
            Color::from_rgba(cr, cg, cb, (225.0 * alpha) as u8),
        );
    }
    for i in 0..5 {
        let r = (22.0 - i as f32 * 3.2) * scale;
        draw_circle(c.x, c.y, r, Color::new(0.76, 0.69, 1.0, a((5 - i) as f32 / 5.0 * 0.14)));
    }
    draw_circle(c.x, c.y, 11.0 * scale, Color::new(0.75, 0.67, 1.0, a(0.95)));
    draw_circle(c.x, c.y, 7.0 * scale, Color::new(0.88, 0.84, 1.0, a(1.0)));
    draw_circle(c.x, c.y, 3.5 * scale, Color::new(1.0, 0.99, 1.0, a(1.0)));
}

/// Small glyph for a zone kind.
pub fn zone_kind(kind: crate::physics::zones::ZoneKind, c: Vec2, size: f32, color: Color, t: f32) {
    use crate::physics::zones::ZoneKind;
    let s = size / 24.0;
    match kind {
        ZoneKind::Wind => {
            for k in 0..3 {
                let y = c.y + (k as f32 - 1.0) * 5.0 * s;
                let len = if k == 1 { 16.0 } else { 11.0 } * s;
                draw_line(c.x - 8.0 * s, y, c.x - 8.0 * s + len, y, 1.8 * s, color);
                draw_circle_lines(c.x - 8.0 * s + len, y - 2.0 * s, 2.0 * s, 1.2 * s, color);
            }
        }
        ZoneKind::Float => {
            for k in 0..3 {
                let phase = (t * 0.6 + k as f32 / 3.0) % 1.0;
                let x = c.x + (k as f32 - 1.0) * 6.0 * s;
                let y = c.y + 8.0 * s - phase * 16.0 * s;
                draw_circle_lines(
                    x,
                    y,
                    (1.5 + k as f32) * s,
                    1.2 * s,
                    Color { a: color.a * (1.0 - phase * 0.6), ..color },
                );
            }
        }
        ZoneKind::Portal => {
            let exit = Color { a: color.a, ..Color::new(1.0, 0.62, 0.25, 1.0) };
            draw_circle_lines(c.x - 6.0 * s, c.y, 5.5 * s, 2.0 * s, color);
            draw_circle_lines(c.x + 6.0 * s, c.y, 5.5 * s, 2.0 * s, exit);
        }
        ZoneKind::Goal => {
            draw_line(c.x - 4.0 * s, c.y - 9.0 * s, c.x - 4.0 * s, c.y + 9.0 * s, 1.8 * s, color);
            draw_triangle(
                vec2(c.x - 4.0 * s, c.y - 9.0 * s),
                vec2(c.x - 4.0 * s, c.y - 1.0 * s),
                vec2(c.x + 8.0 * s, c.y - 5.0 * s),
                color,
            );
        }
    }
}

/// Horseshoe magnet glyph (red and blue poles).
pub fn magnet(c: Vec2, size: f32, alpha: f32, repel: bool) {
    let s = size / 16.0;
    let body = if repel { Color::new(0.55, 0.6, 0.7, alpha) } else { Color::new(0.85, 0.25, 0.3, alpha) };
    let steps = 10;
    let r = 5.0 * s;
    let mut prev = c + vec2(-r, 0.0);
    for i in 1..=steps {
        let a = std::f32::consts::PI + i as f32 / steps as f32 * std::f32::consts::PI;
        let p = c + vec2(a.cos() * r, -a.sin() * r);
        draw_line(prev.x, prev.y, p.x, p.y, 3.0 * s, body);
        prev = p;
    }
    for (x, col) in [(-r, Color::new(0.9, 0.9, 0.95, alpha)), (r, Color::new(0.35, 0.55, 1.0, alpha))] {
        draw_line(c.x + x, c.y, c.x + x, c.y - 5.0 * s, 3.0 * s, col);
    }
}

/// Chevrons showing which way a conveyor surface moves.
pub fn conveyor(c: Vec2, size: f32, speed: f32, t: f32, alpha: f32) {
    let s = size / 16.0;
    let dir = if speed >= 0.0 { 1.0 } else { -1.0 };
    let col = Color::new(1.0, 0.85, 0.3, alpha);
    for k in 0..3 {
        let x = c.x + (k as f32 - 1.0) * 5.0 * s + ((t * speed.abs() * 2.0) % 1.0) * 5.0 * s * dir;
        draw_line(x - 2.0 * s * dir, c.y - 3.5 * s, x + 2.0 * s * dir, c.y, 1.8 * s, col);
        draw_line(x + 2.0 * s * dir, c.y, x - 2.0 * s * dir, c.y + 3.5 * s, 1.8 * s, col);
    }
}

/// Icon of a gadget kind (tool card).
pub fn gadget_kind(kind: crate::physics::gadgets::GadgetKind, c: Vec2, size: f32, color: Color, t: f32) {
    use crate::physics::gadgets::GadgetKind;
    let s = size / 24.0;
    let soft = Color { a: color.a * 0.45, ..color };
    match kind {
        GadgetKind::Laser => {
            draw_rectangle(c.x - 10.0 * s, c.y - 3.5 * s, 9.0 * s, 7.0 * s, color);
            let reach = 4.0 + 7.0 * ((t * 2.0) % 1.0);
            draw_line(c.x - 1.0 * s, c.y, c.x + reach * s, c.y, 1.6 * s, color);
            draw_circle(c.x + reach * s, c.y, 1.8 * s, soft);
        }
        GadgetKind::Thruster => {
            draw_triangle(
                vec2(c.x - 2.0 * s, c.y - 6.0 * s),
                vec2(c.x + 4.0 * s, c.y - 3.0 * s),
                vec2(c.x + 4.0 * s, c.y + 3.0 * s),
                color,
            );
            draw_triangle(
                vec2(c.x - 2.0 * s, c.y - 6.0 * s),
                vec2(c.x - 2.0 * s, c.y + 6.0 * s),
                vec2(c.x + 4.0 * s, c.y + 3.0 * s),
                color,
            );
            let k = 0.7 + 0.3 * (t * 20.0).sin();
            draw_triangle(
                vec2(c.x - 2.0 * s, c.y - 4.0 * s),
                vec2(c.x - 2.0 * s, c.y + 4.0 * s),
                vec2(c.x - (2.0 + 9.0 * k) * s, c.y),
                soft,
            );
        }
        GadgetKind::Cannon => {
            draw_circle(c.x - 4.0 * s, c.y + 3.0 * s, 5.0 * s, color);
            let d = vec2(0.8, -0.6);
            let n = vec2(-d.y, d.x) * 3.0 * s;
            let (a, b) = (vec2(c.x - 4.0 * s, c.y + 3.0 * s), vec2(c.x - 4.0 * s, c.y + 3.0 * s) + d * 13.0 * s);
            draw_triangle(a + n, a - n, b + n, color);
            draw_triangle(a - n, b - n, b + n, color);
            let u = (t * 1.5) % 1.0;
            let ball = b + d * u * 8.0 * s;
            draw_circle(ball.x, ball.y, 2.0 * s, Color { a: color.a * (1.0 - u), ..color });
        }
    }
}

/// Glyph of a gadget trigger: an arrow, a dot (always) or a note (beat).
pub fn trigger(trigger: crate::physics::gadgets::Trigger, c: Vec2, size: f32, color: Color) {
    use crate::physics::gadgets::Trigger;
    let s = size / 16.0;
    match trigger.arrow() {
        Some(d) => {
            let n = vec2(-d.y, d.x);
            draw_line(
                c.x - d.x * 6.0 * s,
                c.y - d.y * 6.0 * s,
                c.x + d.x * 2.0 * s,
                c.y + d.y * 2.0 * s,
                2.0 * s,
                color,
            );
            draw_triangle(c + d * 7.0 * s, c + d * 1.0 * s + n * 5.0 * s, c + d * 1.0 * s - n * 5.0 * s, color);
        }
        None if trigger == Trigger::Beat => {
            draw_circle(c.x - 2.5 * s, c.y + 4.0 * s, 3.0 * s, color);
            draw_line(c.x + 0.3 * s, c.y + 4.0 * s, c.x + 0.3 * s, c.y - 6.0 * s, 1.6 * s, color);
            draw_line(c.x + 0.3 * s, c.y - 6.0 * s, c.x + 5.0 * s, c.y - 3.5 * s, 1.6 * s, color);
        }
        None => {
            draw_circle_lines(c.x, c.y, 5.5 * s, 1.6 * s, color);
            draw_circle(c.x, c.y, 2.2 * s, color);
        }
    }
}
