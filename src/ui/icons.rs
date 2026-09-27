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
