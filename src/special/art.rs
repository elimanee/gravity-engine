//! The special stage on screen: the checkered globe seen from behind and
//! above him, the spheres, rings, bumpers and springs on it, the runner
//! from behind, the counters, and the emerald won (or the stage lost).

use super::*;
use crate::sonic::Character;
use macroquad::prelude::*;

/// The emeralds' colours, in the order they are found.
pub const EMERALD_COLOURS: [Color; 7] = [
    Color::new(0.2, 0.85, 0.35, 1.0),
    Color::new(1.0, 0.85, 0.2, 1.0),
    Color::new(0.3, 0.55, 1.0, 1.0),
    Color::new(0.95, 0.25, 0.3, 1.0),
    Color::new(0.75, 0.4, 0.95, 1.0),
    Color::new(0.45, 0.95, 0.95, 1.0),
    Color::new(0.88, 0.88, 0.94, 1.0),
];

/// Each stage's checkers (two colours) and sky (top, horizon).
const PALETTES: [[Color; 4]; 7] = [
    [rgb(0.18, 0.32, 0.9), rgb(0.95, 0.7, 0.25), rgb(0.05, 0.08, 0.3), rgb(0.35, 0.55, 0.95)],
    [rgb(0.15, 0.6, 0.3), rgb(0.7, 0.92, 0.45), rgb(0.02, 0.15, 0.12), rgb(0.3, 0.7, 0.6)],
    [rgb(0.55, 0.2, 0.75), rgb(0.98, 0.6, 0.8), rgb(0.12, 0.03, 0.22), rgb(0.75, 0.4, 0.8)],
    [rgb(0.1, 0.55, 0.6), rgb(0.92, 0.95, 0.98), rgb(0.02, 0.12, 0.2), rgb(0.4, 0.75, 0.9)],
    [rgb(0.8, 0.2, 0.15), rgb(1.0, 0.85, 0.3), rgb(0.25, 0.04, 0.05), rgb(0.95, 0.55, 0.3)],
    [rgb(0.1, 0.15, 0.45), rgb(0.35, 0.85, 0.95), rgb(0.0, 0.02, 0.1), rgb(0.2, 0.35, 0.7)],
    [rgb(0.4, 0.4, 0.48), rgb(0.95, 0.82, 0.35), rgb(0.08, 0.08, 0.1), rgb(0.55, 0.55, 0.62)],
];

const fn rgb(r: f32, g: f32, b: f32) -> Color {
    Color::new(r, g, b, 1.0)
}

/// The globe's radius, the camera's height above him and distance behind
/// him (cells), and how far it looks down (rad).
const RADIUS: f32 = 16.0;
const CAM_UP: f32 = 2.7;
const CAM_BACK: f32 = 3.4;
const PITCH: f32 = 0.42;

struct View {
    centre: Vec2,
    focal: f32,
    /// Ahead and to the right, on the grid.
    ahead: Vec2,
    right: Vec2,
}

impl View {
    fn new(s: &Stage, sw: f32, sh: f32) -> Self {
        let a = s.view * std::f32::consts::FRAC_PI_2;
        View {
            centre: vec2(sw / 2.0, sh * 0.48),
            focal: sh.min(sw * 0.75),
            ahead: vec2(a.sin(), a.cos()),
            right: vec2(a.cos(), -a.sin()),
        }
    }

    /// A grid offset from him → (ahead, right) cells.
    fn local(&self, r: Vec2) -> (f32, f32) {
        (r.dot(self.ahead), r.dot(self.right))
    }

    /// A point `ahead` and `right` of him on the globe, `up` above it: where
    /// it shows and how far it is (None when hidden behind the globe).
    fn project(&self, ahead: f32, right: f32, up: f32) -> Option<(Vec2, f32)> {
        let (a, b) = (ahead / RADIUS, right / RADIUS);
        let n = vec3(b.sin(), a.cos() * b.cos(), a.sin() * b.cos());
        let p = vec3(0.0, -RADIUS, 0.0) + n * (RADIUS + up);
        let rel = p - vec3(0.0, CAM_UP, -CAM_BACK);
        // The ground there faces away from the camera: over the horizon.
        let ground = vec3(0.0, -RADIUS, 0.0) + n * RADIUS - vec3(0.0, CAM_UP, -CAM_BACK);
        if n.dot(ground) > 0.0 {
            return None;
        }
        let fwd = vec3(0.0, -PITCH.sin(), PITCH.cos());
        let upv = vec3(0.0, PITCH.cos(), PITCH.sin());
        let d = rel.dot(fwd);
        if d < 0.4 {
            return None;
        }
        let u = rel.dot(upv);
        Some((vec2(self.centre.x + rel.x / d * self.focal, self.centre.y - u / d * self.focal), d))
    }
}

/// The whole screen.
pub fn draw(s: &Stage, who: Character, golden: bool, emeralds: u8, t: f32) {
    let (sw, sh) = (screen_width(), screen_height());
    let pal = PALETTES[s.index % PALETTES.len()];
    sky(pal[2], pal[3], sw, sh, t);
    let v = View::new(s, sw, sh);
    let here = vec2(s.pos.x.round(), s.pos.y.round());
    const REACH: i32 = 16;

    // Tiles, far to near.
    let mut tiles = vec![];
    for j in -REACH..=REACH {
        for i in -REACH..=REACH {
            let c = here + vec2(i as f32, j as f32);
            let (ahead, right) = v.local(c - s.pos);
            if !(-3.5..=20.0).contains(&ahead) || right.abs() > 16.0 {
                continue;
            }
            tiles.push((ahead, c));
        }
    }
    tiles.sort_by(|a, b| b.0.total_cmp(&a.0));
    for &(_, c) in &tiles {
        let corners = [vec2(-0.5, -0.5), vec2(0.5, -0.5), vec2(0.5, 0.5), vec2(-0.5, 0.5)].map(|k| {
            let (a, r) = v.local(c + k - s.pos);
            v.project(a, r, 0.0)
        });
        let Some(q) = corners.iter().copied().collect::<Option<Vec<_>>>() else { continue };
        let odd = (c.x as i32 + c.y as i32).rem_euclid(2) == 1;
        // Further tiles fade into the sky's haze.
        let haze = ((q[0].1 - 8.0) / 12.0).clamp(0.0, 0.7);
        let col = mix(if odd { pal[0] } else { pal[1] }, pal[3], haze);
        draw_triangle(q[0].0, q[1].0, q[2].0, col);
        draw_triangle(q[0].0, q[2].0, q[3].0, col);
    }

    // Things on the globe, far to near.
    let mut things = vec![];
    for &(_, c) in &tiles {
        let cell = s.cell(c.x as i32, c.y as i32);
        if cell == Cell::Empty {
            continue;
        }
        let (a, r) = v.local(c - s.pos);
        if let Some((p, d)) = v.project(a, r, 0.38) {
            things.push((d, p, cell));
        }
    }
    things.sort_by(|a, b| b.0.total_cmp(&a.0));
    for (d, p, cell) in things {
        let size = 0.36 / d * v.focal;
        thing(cell, p, size, t);
    }

    // Him, running away from us.
    if let Some((p, d)) = v.project(0.0, 0.0, 0.5 + s.height()) {
        let shadow = v.project(0.0, 0.0, 0.0).map_or(p, |(q, _)| q);
        let w = 0.4 / d * v.focal;
        draw_ellipse(shadow.x, shadow.y, w, w * 0.3, 0.0, Color::new(0.0, 0.0, 0.0, 0.25));
        runner(p, 0.42 / d * v.focal, who, golden, s, t);
    }
    hud(s, sw, t);
    overlay(s, emeralds, sw, sh, t);
}

fn mix(a: Color, b: Color, k: f32) -> Color {
    Color::new(a.r + (b.r - a.r) * k, a.g + (b.g - a.g) * k, a.b + (b.b - a.b) * k, 1.0)
}

fn sky(top: Color, horizon: Color, sw: f32, sh: f32, t: f32) {
    let bands = 24;
    for k in 0..bands {
        let y0 = sh * k as f32 / bands as f32;
        let col = mix(top, horizon, (k as f32 / (bands - 1) as f32).powf(1.4));
        draw_rectangle(0.0, y0, sw, sh / bands as f32 + 1.0, col);
    }
    // Twinkling stars in the upper sky.
    for k in 0..48u32 {
        let h = k.wrapping_mul(2_654_435_761);
        let x = (h % 10_000) as f32 / 10_000.0 * sw;
        let y = ((h / 10_000) % 1000) as f32 / 1000.0 * sh * 0.4;
        let a = 0.35 + 0.35 * (t * 2.0 + k as f32).sin();
        draw_circle(x, y, 1.2, Color::new(1.0, 1.0, 1.0, a));
    }
}

/// A sphere, ring, bumper or spring of `size` px radius.
fn thing(cell: Cell, p: Vec2, size: f32, t: f32) {
    match cell {
        Cell::Blue => ball(p, size, Color::new(0.15, 0.35, 1.0, 1.0)),
        Cell::Red => ball(p, size, Color::new(0.95, 0.15, 0.15, 1.0)),
        Cell::Ring => {
            let w = size * (0.35 + 0.65 * (t * 4.0 + p.x * 0.01).cos().abs());
            draw_ellipse_lines(p.x, p.y, w, size, 0.0, size * 0.28, Color::new(0.75, 0.5, 0.05, 1.0));
            draw_ellipse_lines(p.x, p.y, w, size, 0.0, size * 0.16, Color::new(1.0, 0.85, 0.2, 1.0));
        }
        Cell::Bumper => {
            ball(p, size, Color::new(0.92, 0.92, 0.96, 1.0));
            star(p, size * 0.7, Color::new(0.95, 0.2, 0.25, 1.0), t * 2.0);
        }
        Cell::Spring => {
            ball(p, size, Color::new(1.0, 0.8, 0.15, 1.0));
            star(p, size * 0.6, Color::new(0.95, 0.4, 0.1, 1.0), 0.0);
        }
        Cell::Empty => {}
    }
}

/// A shiny ball: a dark rim, the colour, and a highlight.
fn ball(p: Vec2, r: f32, col: Color) {
    let dark = Color::new(col.r * 0.45, col.g * 0.45, col.b * 0.45, 1.0);
    draw_circle(p.x, p.y + r * 0.08, r, dark);
    draw_circle(p.x - r * 0.06, p.y, r * 0.9, col);
    draw_circle(p.x - r * 0.35, p.y - r * 0.38, r * 0.28, Color::new(1.0, 1.0, 1.0, 0.7));
}

fn star(c: Vec2, r: f32, col: Color, rot: f32) {
    for k in 0..5 {
        let a = rot + k as f32 * std::f32::consts::TAU / 5.0 - std::f32::consts::FRAC_PI_2;
        let tip = c + vec2(a.cos(), a.sin()) * r;
        let side = vec2(-a.sin(), a.cos()) * r * 0.32;
        draw_triangle(c + side, c - side, tip, col);
    }
}

/// Body colour of each character, and when super.
fn colour(who: Character, golden: bool) -> Color {
    if golden {
        return Color::new(1.0, 0.85, 0.25, 1.0);
    }
    match who {
        Character::Sonic => Color::new(0.12, 0.36, 0.9, 1.0),
        Character::Tails => Color::new(0.98, 0.62, 0.15, 1.0),
        Character::Knuckles => Color::new(0.88, 0.16, 0.18, 1.0),
        Character::Amy => Color::new(0.98, 0.5, 0.7, 1.0),
        Character::Exe => Color::new(0.08, 0.16, 0.5, 1.0),
    }
}

/// The runner seen from behind, `r` px tall from the centre: curled up
/// in a jump, otherwise running.
fn runner(p: Vec2, r: f32, who: Character, golden: bool, s: &Stage, t: f32) {
    let body = colour(who, golden);
    let dark = Color::new(body.r * 0.6, body.g * 0.6, body.b * 0.6, 1.0);
    if s.air.is_some() {
        draw_circle(p.x, p.y, r * 0.7, dark);
        draw_circle(p.x, p.y, r * 0.62, body);
        let spin = t * 18.0;
        for k in 0..3 {
            let a = spin + k as f32 * std::f32::consts::TAU / 3.0;
            let q = p + vec2(a.cos(), a.sin()) * r * 0.45;
            draw_line(p.x, p.y, q.x, q.y, r * 0.08, Color::new(1.0, 1.0, 1.0, 0.5));
        }
        return;
    }
    let pace = if s.running() { s.speed * 2.2 } else { 0.0 };
    let stride = (t * pace).sin();
    let shoes = Color::new(0.88, 0.12, 0.12, 1.0);
    let glove = WHITE;
    // Legs and shoes, one up as the other is down.
    for (k, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        let lift = if k == 0 { stride } else { -stride }.max(0.0);
        let hip = p + vec2(side * r * 0.18, r * 0.35);
        let foot = p + vec2(side * r * 0.22, r * (0.85 - 0.25 * lift));
        draw_line(hip.x, hip.y, foot.x, foot.y, r * 0.12, if golden { body } else { dark });
        draw_ellipse(foot.x, foot.y, r * 0.17, r * 0.1 + r * 0.04 * (1.0 - lift), 0.0, shoes);
        draw_line(foot.x - r * 0.12, foot.y, foot.x + r * 0.12, foot.y, r * 0.04, WHITE);
    }
    if who == Character::Tails {
        for side in [-1.0f32, 1.0] {
            let wag = (t * 10.0 + side).sin() * r * 0.1;
            let tip = p + vec2(side * r * 0.55 + wag, r * 0.55);
            draw_line(p.x, p.y + r * 0.35, tip.x, tip.y, r * 0.2, body);
            draw_circle(tip.x, tip.y, r * 0.14, WHITE);
        }
    }
    // Body, arms swinging, head.
    draw_ellipse(p.x, p.y + r * 0.12, r * 0.32, r * 0.36, 0.0, body);
    for (k, side) in [-1.0f32, 1.0].into_iter().enumerate() {
        let swing = if k == 0 { -stride } else { stride };
        let hand = p + vec2(side * r * 0.5, r * (0.15 + 0.15 * swing));
        draw_line(p.x + side * r * 0.25, p.y - r * 0.02, hand.x, hand.y, r * 0.1, body);
        draw_circle(hand.x, hand.y, r * 0.11, glove);
    }
    let head = p - vec2(0.0, r * 0.38);
    match who {
        Character::Sonic | Character::Exe => {
            // Quills sweeping back and down.
            for (dx, dy) in [(-0.42f32, 0.25f32), (0.42, 0.25), (-0.25, 0.45), (0.25, 0.45), (0.0, 0.55)] {
                let tip = head + vec2(dx, dy) * r;
                let base = vec2(-dy, dx).normalize_or_zero() * r * 0.16;
                draw_triangle(head + base, head - base, tip, dark);
            }
        }
        Character::Knuckles => {
            for dx in [-0.3f32, -0.1, 0.1, 0.3] {
                let tip = head + vec2(dx * 1.4, 0.55) * r;
                draw_line(head.x + dx * r, head.y, tip.x, tip.y, r * 0.14, body);
            }
        }
        Character::Amy => {
            draw_triangle(
                p + vec2(-r * 0.38, r * 0.4),
                p + vec2(r * 0.38, r * 0.4),
                p,
                Color::new(0.85, 0.1, 0.2, 1.0),
            );
        }
        Character::Tails => {}
    }
    draw_circle(head.x, head.y, r * 0.34, body);
    for side in [-1.0f32, 1.0] {
        let ear = head + vec2(side * r * 0.24, -r * 0.24);
        draw_triangle(ear + vec2(-r * 0.1, r * 0.06), ear + vec2(r * 0.1, r * 0.06), ear - vec2(0.0, r * 0.16), body);
    }
    if golden {
        let a = 0.25 + 0.15 * (t * 8.0).sin();
        draw_circle(p.x, p.y, r * 0.9, Color::new(1.0, 0.95, 0.5, a));
    }
}

fn shadowed(text: &str, x: f32, y: f32, size: f32, col: Color) {
    crate::ui::theme::text_bold(text, x + 2.0, y + 2.0, size, Color::new(0.0, 0.0, 0.0, 0.6));
    crate::ui::theme::text_bold(text, x, y, size, col);
}

fn centred(text: &str, cx: f32, y: f32, size: f32, col: Color) {
    let w = crate::ui::theme::measure_bold(text, size);
    shadowed(text, cx - w / 2.0, y, size, col);
}

fn hud(s: &Stage, sw: f32, t: f32) {
    ball(vec2(34.0, 34.0), 14.0, Color::new(0.15, 0.35, 1.0, 1.0));
    shadowed(&s.blue_left.to_string(), 58.0, 42.0, 26.0, WHITE);
    let rings = s.rings.to_string();
    let w = crate::ui::theme::measure_bold(&rings, 26.0);
    draw_circle_lines(sw - 34.0, 34.0, 12.0, 4.0, Color::new(1.0, 0.85, 0.2, 1.0));
    shadowed(&rings, sw - 58.0 - w, 42.0, 26.0, WHITE);
    if s.intro() {
        let a = if (t * 4.0) as i32 % 2 == 0 { 1.0 } else { 0.7 };
        centred("GET BLUE SPHERES!", sw / 2.0, screen_height() * 0.3, 34.0, Color::new(1.0, 1.0, 1.0, a));
    }
}

/// A cut gem: the table on top, the crown's facets, the point below.
pub fn emerald(c: Vec2, r: f32, col: Color, alpha: f32) {
    let col = Color { a: alpha, ..col };
    let light = Color::new((col.r + 0.45).min(1.0), (col.g + 0.45).min(1.0), (col.b + 0.45).min(1.0), alpha);
    let dark = Color::new(col.r * 0.55, col.g * 0.55, col.b * 0.55, alpha);
    let (tl, tr) = (c + vec2(-r * 0.5, -r * 0.55), c + vec2(r * 0.5, -r * 0.55));
    let (ml, mr) = (c + vec2(-r, -r * 0.1), c + vec2(r, -r * 0.1));
    let bottom = c + vec2(0.0, r);
    draw_triangle(tl, tr, c, light);
    draw_triangle(tl, ml, c, col);
    draw_triangle(tr, mr, c, col);
    draw_triangle(ml, c, bottom, dark);
    draw_triangle(mr, c, bottom, col);
}

/// The seven emeralds in a row: those found shine, the others are outlines.
pub fn emerald_row(centre: Vec2, size: f32, mask: u8) {
    for (i, &col) in EMERALD_COLOURS.iter().enumerate() {
        let c = centre + vec2((i as f32 - 3.0) * size * 2.6, 0.0);
        if mask & (1 << i) != 0 {
            emerald(c, size, col, 1.0);
        } else {
            emerald(c, size, Color::new(1.0, 1.0, 1.0, 1.0), 0.18);
        }
    }
}

/// The end: the emerald won, or the stage lost, then the way on.
fn overlay(s: &Stage, emeralds: u8, sw: f32, sh: f32, t: f32) {
    let Some(o) = s.outcome else { return };
    let (since, won) = match o {
        Outcome::Won(x) => (x, true),
        Outcome::Lost(x) => (x, false),
    };
    // A white flash, then a dim screen.
    let flash = (1.0 - since / 0.6).clamp(0.0, 1.0);
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.05, since.min(0.82)));
    draw_rectangle(0.0, 0.0, sw, sh, Color::new(1.0, 1.0, 1.0, flash * 0.9));
    if since < 0.5 {
        return;
    }
    let yellow = Color::new(1.0, 0.88, 0.2, 1.0);
    if won {
        let bob = (t * 2.0).sin() * 6.0;
        emerald(vec2(sw / 2.0, sh * 0.32 + bob), 46.0, EMERALD_COLOURS[s.index % 7], 1.0);
        centred("YOU GOT A CHAOS EMERALD", sw / 2.0, sh * 0.5, 30.0, yellow);
    } else {
        centred("SPECIAL STAGE", sw / 2.0, sh * 0.42, 30.0, yellow);
        centred("Touched a red sphere", sw / 2.0, sh * 0.5, 18.0, WHITE);
    }
    let mut lines = vec![format!("RINGS  {}", s.rings)];
    if s.perfect() {
        lines.push("PERFECT!".to_string());
    }
    for (k, line) in lines.iter().enumerate() {
        centred(line, sw / 2.0, sh * 0.58 + k as f32 * 28.0, 20.0, WHITE);
    }
    let shown = if won { emeralds | 1 << s.index } else { emeralds };
    emerald_row(vec2(sw / 2.0, sh * 0.74), 13.0, shown);
    if count(shown) == EMERALDS && won {
        centred("All seven!  ·  50 rings, then jump twice: Super!", sw / 2.0, sh * 0.82, 16.0, yellow);
    }
    if since > 2.0 && (t * 2.0) as i32 % 2 == 0 {
        crate::ui::theme::text_centered("Space / A to continue", sw / 2.0, sh * 0.9, 15.0, WHITE);
    }
}
