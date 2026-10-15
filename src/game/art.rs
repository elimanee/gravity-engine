//! Drawings of the game mode (world pass): badniks, shots, spikes,
//! monitors, star posts, the goal signpost, the boss, scattered rings and
//! the shield / invincibility around Sonic. All made of shapes.

use super::*;
use macroquad::prelude::*;

const STEEL: Color = Color::new(0.74, 0.76, 0.84, 1.0);
const STEEL_DARK: Color = Color::new(0.36, 0.38, 0.48, 1.0);
const INK: Color = Color::new(0.06, 0.06, 0.1, 1.0);

/// Everything of the level (world pass), behind and around Sonic.
pub fn draw(g: &Game, t: f32) {
    for s in &g.spikes {
        spikes(s.pos);
    }
    for p in &g.posts {
        post(p, t);
    }
    if let Some(goal) = g.goal {
        signpost(goal, g.goal_passed, t);
    }
    for m in &g.monitors {
        monitor(m, t);
    }
    for e in &g.enemies {
        enemy(e, t);
    }
    if let Some(b) = &g.boss {
        boss(b, t);
    }
    for s in &g.shots {
        draw_circle(s.pos.x, s.pos.y, 7.0, Color::new(1.0, 0.45, 0.1, 0.35));
        draw_circle(s.pos.x, s.pos.y, 4.5, Color::new(1.0, 0.6, 0.15, 1.0));
        draw_circle(s.pos.x, s.pos.y, 2.0, Color::new(1.0, 0.95, 0.8, 1.0));
    }
    for r in &g.lost {
        // Blink in their last second.
        let left = RING_LIFE - r.age;
        if left > 1.0 || (t * 16.0) as i32 % 2 == 0 {
            crate::sonic::art::ring(r.pos, t * 1.6 + r.age, SCALE);
        }
    }
}

/// Shield bubble and invincibility sparkles around Sonic.
pub fn draw_around(g: &Game, at: Vec2, t: f32) {
    if g.shield {
        let pulse = 0.85 + 0.15 * (t * 6.0).sin();
        draw_circle(at.x, at.y, 40.0 * pulse, Color::new(0.35, 0.7, 1.0, 0.16));
        draw_circle_lines(at.x, at.y, 40.0 * pulse, 2.0, Color::new(0.6, 0.85, 1.0, 0.7));
        draw_circle(at.x - 14.0, at.y - 18.0, 5.0, Color::new(1.0, 1.0, 1.0, 0.35));
    }
    if g.invincible > 0.0 {
        for k in 0..8 {
            let a = t * 7.0 + k as f32 * std::f32::consts::TAU / 8.0;
            let r = 34.0 + 8.0 * (t * 9.0 + k as f32).sin();
            let p = at + vec2(a.cos(), a.sin()) * r;
            let c = Color::new(1.0, 1.0, 0.6 + 0.4 * (k % 2) as f32, 0.9);
            star(p, 5.0, c, a);
        }
    }
}

fn star(c: Vec2, r: f32, col: Color, rot: f32) {
    for k in 0..4 {
        let a = rot + k as f32 * std::f32::consts::FRAC_PI_2;
        let tip = c + vec2(a.cos(), a.sin()) * r;
        let side = vec2(-a.sin(), a.cos()) * r * 0.3;
        draw_triangle(c + side, c - side, tip, col);
    }
}

fn spikes(at: Vec2) {
    let (w, h) = (SPIKES.x, SPIKES.y);
    let base = Rect::new(at.x - w / 2.0, at.y + h / 2.0 - 7.0, w, 7.0);
    draw_rectangle(base.x, base.y, base.w, base.h, STEEL_DARK);
    for k in 0..4 {
        let x0 = at.x - w / 2.0 + k as f32 * w / 4.0;
        let (a, b, tip) = (vec2(x0, base.y), vec2(x0 + w / 4.0, base.y), vec2(x0 + w / 8.0, at.y - h / 2.0));
        draw_triangle(a, b, tip, STEEL);
        draw_line(tip.x, tip.y, b.x, b.y, 1.5, STEEL_DARK);
    }
}

/// A monitor: a TV on a stand, its item on the screen.
fn monitor(m: &Monitor, t: f32) {
    let (w, h) = (MONITOR.x, MONITOR.y);
    let r = Rect::new(m.pos.x - w / 2.0, m.pos.y - h / 2.0, w, h);
    if let Some(age) = m.broken {
        // A broken stand, the item floating up and fading.
        draw_rectangle(r.x + 4.0, r.y + h - 10.0, w - 8.0, 10.0, STEEL_DARK);
        if age < 1.4 {
            let a = (1.0 - age / 1.4).clamp(0.0, 1.0);
            let c = m.pos - vec2(0.0, 18.0 + age * 40.0);
            draw_rectangle(c.x - 15.0, c.y - 13.0, 30.0, 26.0, Color::new(0.1, 0.12, 0.2, 0.8 * a));
            item(m.kind, c, a, t);
        }
        return;
    }
    draw_rectangle(r.x, r.y, w, h - 6.0, STEEL);
    draw_rectangle(r.x + 6.0, r.y + h - 8.0, w - 12.0, 8.0, STEEL_DARK);
    draw_rectangle_lines(r.x, r.y, w, h - 6.0, 2.0, STEEL_DARK);
    let screen = Rect::new(r.x + 5.0, r.y + 5.0, w - 10.0, h - 18.0);
    // Static flicker now and then.
    let fuzz = ((t * 23.0 + m.pos.x).sin() * 43758.5).fract().abs() > 0.9;
    let bg = if fuzz { Color::new(0.75, 0.78, 0.85, 1.0) } else { Color::new(0.08, 0.1, 0.18, 1.0) };
    draw_rectangle(screen.x, screen.y, screen.w, screen.h, bg);
    if !fuzz {
        item(m.kind, vec2(screen.x + screen.w / 2.0, screen.y + screen.h / 2.0), 1.0, t);
    }
}

/// The item icon of a monitor.
pub fn item(kind: MonitorKind, c: Vec2, a: f32, t: f32) {
    let fade = |col: Color| Color { a: col.a * a, ..col };
    match kind {
        MonitorKind::Rings => {
            let gold = fade(Color::new(1.0, 0.82, 0.15, 1.0));
            draw_circle_lines(c.x, c.y, 8.0, 3.0, gold);
        }
        MonitorKind::Shield => {
            draw_circle(c.x, c.y, 10.0, fade(Color::new(0.35, 0.7, 1.0, 0.5)));
            draw_circle_lines(c.x, c.y, 10.0, 1.5, fade(Color::new(0.7, 0.9, 1.0, 1.0)));
        }
        MonitorKind::Invincible => {
            for k in 0..3 {
                let p = c + vec2((k as f32 - 1.0) * 7.0, ((k % 2) as f32 - 0.5) * 6.0);
                star(p, 5.0, fade(Color::new(1.0, 1.0, 0.5, 1.0)), t * 3.0 + k as f32);
            }
        }
        MonitorKind::Shoes => {
            let red = fade(Color::new(0.9, 0.15, 0.15, 1.0));
            draw_ellipse(c.x, c.y + 3.0, 10.0, 5.0, 0.0, red);
            draw_rectangle(c.x - 9.0, c.y - 6.0, 7.0, 7.0, red);
            draw_line(c.x - 4.0, c.y - 1.0, c.x - 4.0, c.y + 7.0, 2.0, fade(WHITE));
        }
        MonitorKind::Life => {
            // A little blue head (the character's) and "1".
            draw_circle(c.x - 3.0, c.y, 8.0, fade(Color::new(0.12, 0.36, 0.9, 1.0)));
            draw_circle(c.x - 1.0, c.y - 2.0, 2.5, fade(WHITE));
            draw_text("1", c.x + 6.0, c.y + 7.0, 18.0, fade(WHITE));
        }
    }
}

/// A star post: blue ball until passed, then red and spinning.
fn post(p: &Post, t: f32) {
    let base = p.pos + vec2(0.0, 40.0);
    let top = p.pos - vec2(0.0, 26.0);
    draw_rectangle(base.x - 3.0, top.y, 6.0, base.y - top.y, STEEL_DARK);
    draw_rectangle(base.x - 9.0, base.y - 4.0, 18.0, 6.0, STEEL);
    let (ball, col) = match p.lit {
        Some(age) if age < 0.8 => {
            let a = age * 14.0;
            (top + vec2(a.sin() * 10.0, -a.cos() * 10.0), Color::new(1.0, 0.25, 0.2, 1.0))
        }
        Some(_) => (top - vec2(0.0, 6.0), if (t * 4.0) as i32 % 2 == 0 { RED } else { Color::new(1.0, 0.6, 0.3, 1.0) }),
        None => (top - vec2(0.0, 6.0), Color::new(0.25, 0.5, 1.0, 1.0)),
    };
    draw_circle(ball.x, ball.y, 7.0, col);
    draw_circle(ball.x - 2.0, ball.y - 2.0, 2.0, Color::new(1.0, 1.0, 1.0, 0.7));
}

/// The goal signpost; it spins when Sonic runs past.
fn signpost(at: Vec2, passed: Option<f32>, t: f32) {
    let foot = at + vec2(0.0, 40.0);
    draw_rectangle(foot.x - 3.0, at.y - 10.0, 6.0, 50.0, STEEL_DARK);
    let spin = passed.map_or(0.0, |p| (p * 14.0 / (1.0 + p * 1.5)).min(40.0));
    let w = 30.0 * spin.cos().abs().max(0.08);
    let panel = at - vec2(0.0, 34.0);
    draw_rectangle(panel.x - w, panel.y - 28.0, w * 2.0, 56.0, STEEL);
    draw_rectangle_lines(panel.x - w, panel.y - 28.0, w * 2.0, 56.0, 2.0, STEEL_DARK);
    let front = spin.cos() > 0.0;
    if w > 10.0 {
        if passed.is_some() && (front || spin > 30.0) {
            // His blue head when done.
            draw_circle(panel.x, panel.y, w * 0.6, Color::new(0.12, 0.36, 0.9, 1.0));
            draw_circle(panel.x + w * 0.15, panel.y - 3.0, w * 0.2, WHITE);
        } else {
            star(panel, w * 0.7, Color::new(1.0, 0.85, 0.2, 1.0), t);
        }
    }
}

fn enemy(e: &Enemy, t: f32) {
    let d = e.dir;
    let p = e.pos;
    match e.kind {
        EnemyKind::Crawler => {
            // A red shelled bug on a wheel.
            let wheel = p + vec2(-d * 2.0, 10.0);
            draw_circle(wheel.x, wheel.y, 7.0, INK);
            draw_circle(wheel.x, wheel.y, 3.0, STEEL);
            let spoke = t * 12.0 * -d;
            draw_line(wheel.x, wheel.y, wheel.x + spoke.cos() * 6.0, wheel.y + spoke.sin() * 6.0, 1.5, STEEL);
            draw_ellipse(p.x, p.y - 2.0, 20.0, 14.0, 0.0, Color::new(0.85, 0.12, 0.15, 1.0));
            draw_ellipse(p.x - d * 3.0, p.y - 6.0, 10.0, 5.0, 0.0, Color::new(1.0, 0.45, 0.4, 0.8));
            for k in [-1.0, 1.0] {
                draw_circle(p.x + k * 7.0 - d * 2.0, p.y - 4.0, 3.0, INK);
            }
            let face = p + vec2(d * 18.0, 2.0);
            draw_circle(face.x, face.y, 7.0, STEEL_DARK);
            draw_circle(face.x + d * 2.0, face.y - 2.0, 2.5, WHITE);
            draw_line(face.x + d * 4.0, face.y + 3.0, face.x + d * 12.0, face.y + 1.0, 2.0, STEEL);
        }
        EnemyKind::Flyer => {
            // A striped wasp with buzzing wings and a cannon tail.
            let flap = (t * 40.0).sin();
            draw_ellipse(p.x - d * 4.0, p.y - 12.0 - flap * 3.0, 12.0, 5.0, 0.0, Color::new(0.8, 0.9, 1.0, 0.45));
            draw_ellipse(p.x, p.y, 20.0, 9.0, 0.0, Color::new(1.0, 0.8, 0.1, 1.0));
            for k in [-6.0, 2.0] {
                draw_rectangle(p.x + k * d - 2.0, p.y - 8.0, 4.0, 16.0, INK);
            }
            let head = p + vec2(d * 18.0, -2.0);
            draw_circle(head.x, head.y, 7.0, STEEL_DARK);
            draw_circle(head.x + d * 2.0, head.y - 1.0, 3.0, Color::new(1.0, 0.3, 0.2, 1.0));
            let aim = if e.firing { vec2(d * 0.5, 1.0).normalize() } else { vec2(-d, 0.3).normalize() };
            let tail = p + vec2(-d * 16.0, 4.0);
            let tip = tail + aim * 12.0;
            draw_line(tail.x, tail.y, tip.x, tip.y, 4.0, STEEL);
        }
        EnemyKind::Crab => {
            // A red crab, claws up while it fires.
            let up = if e.firing { -14.0 } else { -4.0 };
            for k in [-1.0f32, 1.0] {
                for leg in [6.0, 14.0] {
                    let a = p + vec2(k * leg * 0.6, 6.0);
                    let b = p + vec2(k * (leg + 6.0), 16.0 + (t * 10.0 + leg).sin() * 1.5);
                    draw_line(a.x, a.y, b.x, b.y, 2.5, Color::new(0.7, 0.1, 0.1, 1.0));
                }
                let claw = p + vec2(k * 26.0, up);
                draw_line(p.x + k * 14.0, p.y - 2.0, claw.x, claw.y, 3.0, Color::new(0.75, 0.12, 0.12, 1.0));
                draw_circle(claw.x, claw.y, 7.0, Color::new(0.95, 0.25, 0.2, 1.0));
                draw_triangle(claw + vec2(-2.0 * k, -7.0), claw + vec2(5.0 * k, -9.0), claw + vec2(3.0 * k, -1.0), INK);
            }
            draw_ellipse(p.x, p.y, 22.0, 12.0, 0.0, Color::new(0.9, 0.2, 0.18, 1.0));
            for k in [-1.0, 1.0] {
                draw_line(p.x + k * 6.0, p.y - 10.0, p.x + k * 8.0, p.y - 18.0, 2.0, STEEL_DARK);
                draw_circle(p.x + k * 8.0, p.y - 19.0, 3.5, WHITE);
                draw_circle(p.x + k * 8.0, p.y - 19.0, 1.5, INK);
            }
        }
    }
}

/// The boss: a hovering pod with a glass dome, arms and a propeller.
fn boss(b: &Boss, t: f32) {
    let p = b.pos;
    let hit = b.flash > 0.0 && (t * 20.0) as i32 % 2 == 0;
    let body = if hit { WHITE } else { Color::new(0.55, 0.55, 0.65, 1.0) };
    let tilt = if b.beaten.is_some() { 0.4 } else { (t * 2.0).sin() * 0.05 };
    let r = BOSS_R;
    // Propeller.
    let w = r * (t * 30.0).sin().abs();
    draw_line(p.x, p.y - r, p.x, p.y - r - 12.0, 3.0, STEEL_DARK);
    draw_line(p.x - w, p.y - r - 12.0, p.x + w, p.y - r - 12.0, 3.0, STEEL);
    // Arms with claws.
    for k in [-1.0f32, 1.0] {
        let sway = (t * 3.0 + k).sin() * 6.0;
        let hand = p + vec2(k * (r + 14.0), r * 0.6 + sway);
        draw_line(p.x + k * r * 0.7, p.y + 4.0, hand.x, hand.y, 4.0, STEEL_DARK);
        draw_circle(hand.x, hand.y, 7.0, STEEL);
    }
    draw_ellipse(p.x, p.y + 6.0, r * 1.2, r * 0.75, tilt.to_degrees(), body);
    draw_ellipse(p.x, p.y + 10.0, r * 1.2, r * 0.25, tilt.to_degrees(), Color::new(0.35, 0.2, 0.55, 1.0));
    // Dome with a robot eye looking around.
    draw_circle(p.x, p.y - r * 0.2, r * 0.62, Color::new(0.6, 0.85, 1.0, 0.55));
    let look = vec2((t * 1.3).sin(), 0.2) * 8.0;
    draw_circle(p.x + look.x, p.y - r * 0.2 + look.y, 8.0, Color::new(0.15, 0.15, 0.2, 1.0));
    draw_circle(p.x + look.x, p.y - r * 0.2 + look.y, 4.0, Color::new(1.0, 0.2, 0.2, 1.0));
    // Damage: smoke-black streaks.
    for k in 0..(BOSS_HP - b.hp).min(6) {
        let x = p.x - r * 0.8 + k as f32 * r * 0.3;
        draw_line(x, p.y + 2.0, x + 6.0, p.y + 12.0, 2.0, Color::new(0.1, 0.1, 0.1, 0.6));
    }
}
