//! Drawings of the game mode (world pass): badniks, shots, spikes,
//! monitors, star posts, the goal signpost (and the giant ring), the
//! bosses, scattered rings, bubbles, and the shield / invincibility / air
//! countdown around Sonic. Made of shapes, or taken from the user's sprite
//! sheets when there are some ([`super::sprites`]).

use super::sprites::Sprites;
use super::*;
use macroquad::prelude::*;

const STEEL: Color = Color::new(0.74, 0.76, 0.84, 1.0);
const STEEL_DARK: Color = Color::new(0.36, 0.38, 0.48, 1.0);
const INK: Color = Color::new(0.06, 0.06, 0.1, 1.0);

/// Everything of the level (world pass), behind and around Sonic.
pub fn draw(g: &Game, t: f32, sp: Option<&Sprites>) {
    for s in &g.spikes {
        if !sp.is_some_and(|sp| sp.spikes(s.pos)) {
            spikes(s.pos);
        }
    }
    for p in &g.posts {
        if !sp.is_some_and(|sp| sp.post(p.pos + vec2(0.0, 40.0), p.lit)) {
            post(p, t);
        }
    }
    if let Some(goal) = g.goal {
        let spin = g.goal_passed.map(sign_spin);
        if !sp.is_some_and(|sp| sp.signpost(goal + vec2(0.0, 40.0), spin)) {
            signpost(goal, g.goal_passed, t);
        }
        if g.giant_ring {
            let rise = g.goal_passed.map_or(1.0, |p| (p / 0.6).min(1.0));
            giant_ring(goal - vec2(0.0, 110.0 + 50.0 * rise), 46.0 * rise, t);
        }
    }
    for v in &g.vents {
        vent(v.pos);
    }
    for p in &g.pumpkins {
        pumpkin(*p, t);
    }
    for b in &g.bubbles {
        bubble(b);
    }
    for m in &g.monitors {
        if !sp.is_some_and(|sp| sp.monitor(m.kind, m.pos, m.broken, t)) {
            monitor(m, t);
        }
    }
    for e in &g.enemies {
        enemy(e, t);
    }
    if let Some(b) = &g.boss {
        boss(b, t);
    }
    for s in &g.shots {
        if s.wave {
            wave(s.pos, t);
            continue;
        }
        draw_circle(s.pos.x, s.pos.y, 7.0, Color::new(1.0, 0.45, 0.1, 0.35));
        draw_circle(s.pos.x, s.pos.y, 4.5, Color::new(1.0, 0.6, 0.15, 1.0));
        draw_circle(s.pos.x, s.pos.y, 2.0, Color::new(1.0, 0.95, 0.8, 1.0));
    }
    for r in &g.lost {
        // Blink in their last second.
        let left = RING_LIFE - r.age;
        if (left > 1.0 || (t * 16.0) as i32 % 2 == 0) && !sp.is_some_and(|sp| sp.ring(r.pos, t * 1.6 + r.age)) {
            crate::sonic::art::ring(r.pos, t * 1.6 + r.age, SCALE);
        }
    }
}

/// Shield bubble and invincibility sparkles around Sonic.
pub fn draw_around(g: &Game, at: Vec2, t: f32, sp: Option<&Sprites>) {
    if g.shield && !sp.is_some_and(|sp| sp.shield(at, t)) {
        let pulse = 0.85 + 0.15 * (t * 6.0).sin();
        draw_circle(at.x, at.y, 40.0 * pulse, Color::new(0.35, 0.7, 1.0, 0.16));
        draw_circle_lines(at.x, at.y, 40.0 * pulse, 2.0, Color::new(0.6, 0.85, 1.0, 0.7));
        draw_circle(at.x - 14.0, at.y - 18.0, 5.0, Color::new(1.0, 1.0, 1.0, 0.35));
    }
    if let Some(n) = g.countdown() {
        // The number bubbles up over his head, blinking.
        if (t * 3.0) as i32 % 3 != 0 {
            let c = at - vec2(0.0, 64.0);
            draw_circle(c.x, c.y, 15.0, Color::new(0.75, 0.9, 1.0, 0.35));
            draw_circle_lines(c.x, c.y, 15.0, 2.0, Color::new(0.85, 0.95, 1.0, 0.9));
            let text = n.to_string();
            let w = crate::ui::theme::measure_bold(&text, 22.0);
            crate::ui::theme::text_bold(&text, c.x - w / 2.0, c.y + 8.0, 22.0, WHITE);
        }
    }
    if g.invincible > 0.0 && !sp.is_some_and(|sp| sp.stars(at, t)) {
        for k in 0..8 {
            let a = t * 7.0 + k as f32 * std::f32::consts::TAU / 8.0;
            let r = 34.0 + 8.0 * (t * 9.0 + k as f32).sin();
            let p = at + vec2(a.cos(), a.sin()) * r;
            let c = Color::new(1.0, 1.0, 0.6 + 0.4 * (k % 2) as f32, 0.9);
            star(p, 5.0, c, a);
        }
    }
}

/// Golden sparkles round a super character.
pub fn super_sparkles(at: Vec2, t: f32) {
    for k in 0..5 {
        let phase = (t * 1.7 + k as f32 * 0.37).fract();
        let a = k as f32 * 2.4 + (t * 0.9).floor() * 1.3;
        let p = at + vec2(a.cos(), a.sin()) * (24.0 + phase * 14.0);
        let alpha = 1.0 - phase;
        star(p, 3.0 + 4.0 * alpha, Color::new(1.0, 0.95, 0.6, alpha), t * 4.0 + k as f32);
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

/// How far the signpost has turned (rad), `passed` s after Sonic ran by:
/// fast at first, slowing down to about one and a half turns.
fn sign_spin(passed: f32) -> f32 {
    passed * 14.0 / (1.0 + passed * 1.5)
}

/// Past this turn (rad) the signpost shows Sonic instead of Robotnik.
pub const SIGN_FLIP: f32 = 1.5 * std::f32::consts::PI;

/// The goal signpost; it spins when Sonic runs past.
fn signpost(at: Vec2, passed: Option<f32>, t: f32) {
    let foot = at + vec2(0.0, 40.0);
    draw_rectangle(foot.x - 3.0, at.y - 10.0, 6.0, 50.0, STEEL_DARK);
    let spin = passed.map_or(0.0, sign_spin);
    let w = 30.0 * spin.cos().abs().max(0.08);
    let panel = at - vec2(0.0, 34.0);
    draw_rectangle(panel.x - w, panel.y - 28.0, w * 2.0, 56.0, STEEL);
    draw_rectangle_lines(panel.x - w, panel.y - 28.0, w * 2.0, 56.0, 2.0, STEEL_DARK);
    if w > 10.0 {
        if passed.is_some() && spin > SIGN_FLIP {
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
        EnemyKind::Fish => {
            // A red piranha, nose up on the way up and down on the way down.
            let going_up = (e.timer % 2.8) < 0.75;
            let nose = if going_up { -1.0 } else { 1.0 };
            let fin = (t * 18.0).sin() * 3.0;
            let body = Color::new(0.92, 0.25, 0.3, 1.0);
            draw_triangle(
                p + vec2(0.0, -nose * 12.0),
                p + vec2(-10.0 + fin, -nose * 26.0),
                p + vec2(10.0 + fin, -nose * 26.0),
                Color::new(0.8, 0.15, 0.2, 1.0),
            );
            draw_ellipse(p.x, p.y, 13.0, 18.0, 0.0, body);
            draw_ellipse(p.x + 3.0, p.y + nose * 2.0, 6.0, 10.0, 0.0, Color::new(1.0, 0.85, 0.75, 1.0));
            for k in [-1.0f32, 1.0] {
                draw_circle(p.x + k * 6.0, p.y + nose * 8.0, 3.5, WHITE);
                draw_circle(p.x + k * 6.0, p.y + nose * 9.0, 1.6, INK);
            }
            // Teeth.
            let jaw = p.y + nose * 16.0;
            for k in -2..=2 {
                let x = p.x + k as f32 * 3.0;
                draw_triangle(vec2(x - 1.5, jaw), vec2(x + 1.5, jaw), vec2(x, jaw - nose * 3.0), WHITE);
            }
        }
        EnemyKind::Gull => {
            // A grey gull with a metal beak, wings beating (folded when it dives).
            let diving = e.firing && e.vel.y > 0.0;
            let flap = if diving { 0.8 } else { (t * 9.0).sin() };
            for k in [-1.0f32, 1.0] {
                let root = p + vec2(k * 6.0, -2.0);
                let tip = p + vec2(k * 30.0, -6.0 - flap * 16.0);
                let mid = p + vec2(k * 18.0, -10.0 - flap * 6.0);
                draw_triangle(root, mid, tip, Color::new(0.92, 0.94, 0.98, 1.0));
                draw_triangle(mid, tip, tip + vec2(-k * 6.0, 6.0), INK);
            }
            draw_ellipse(p.x, p.y, 18.0, 9.0, 0.0, Color::new(0.95, 0.96, 1.0, 1.0));
            let head = p + vec2(d * 15.0, -4.0);
            draw_circle(head.x, head.y, 7.0, Color::new(0.95, 0.96, 1.0, 1.0));
            draw_circle(head.x + d * 2.0, head.y - 2.0, 2.2, Color::new(1.0, 0.25, 0.2, 1.0));
            draw_triangle(head + vec2(d * 5.0, -2.0), head + vec2(d * 5.0, 3.0), head + vec2(d * 15.0, 2.0), STEEL);
            draw_triangle(p + vec2(-d * 14.0, -3.0), p + vec2(-d * 14.0, 4.0), p + vec2(-d * 24.0, 0.0), STEEL_DARK);
        }
        EnemyKind::Puffer => {
            // A round puffer: spikes out now and then.
            let out = e.spiky();
            let r = if out { 19.0 } else { 15.0 };
            if out {
                for k in 0..12 {
                    let a = k as f32 * std::f32::consts::TAU / 12.0 + t * 0.5;
                    let dir = vec2(a.cos(), a.sin());
                    let side = vec2(-dir.y, dir.x) * 3.5;
                    draw_triangle(p + dir * (r - 2.0) + side, p + dir * (r - 2.0) - side, p + dir * (r + 10.0), STEEL);
                }
            } else {
                for k in 0..8 {
                    let a = k as f32 * std::f32::consts::TAU / 8.0;
                    let q = p + vec2(a.cos(), a.sin()) * r;
                    draw_circle(q.x, q.y, 2.0, STEEL_DARK);
                }
            }
            draw_circle(p.x, p.y, r, Color::new(0.35, 0.75, 0.6, 1.0));
            draw_ellipse(p.x, p.y + r * 0.35, r * 0.7, r * 0.45, 0.0, Color::new(0.85, 0.95, 0.8, 1.0));
            draw_circle(p.x + d * 6.0, p.y - 5.0, 4.5, WHITE);
            draw_circle(p.x + d * 7.0, p.y - 5.0, 2.0, INK);
            draw_circle(p.x + d * (r - 2.0), p.y + 3.0, 2.5, Color::new(0.9, 0.35, 0.4, 1.0));
        }
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

/// The giant ring to a special stage, turning slowly.
pub fn giant_ring(c: Vec2, r: f32, t: f32) {
    if r < 1.0 {
        return;
    }
    let w = r * (0.25 + 0.75 * (t * 1.5).cos().abs());
    draw_circle(c.x, c.y, r * 1.25, Color::new(1.0, 0.9, 0.4, 0.12));
    draw_ellipse_lines(c.x, c.y, w, r, 0.0, r * 0.3, Color::new(0.72, 0.45, 0.05, 1.0));
    draw_ellipse_lines(c.x, c.y, w, r, 0.0, r * 0.18, Color::new(1.0, 0.85, 0.2, 1.0));
    draw_ellipse_lines(c.x - w * 0.1, c.y - r * 0.1, w * 0.8, r * 0.85, 0.0, r * 0.05, Color::new(1.0, 1.0, 0.8, 0.8));
    // Sparkles round it.
    for k in 0..4 {
        let a = t * 2.0 + k as f32 * std::f32::consts::FRAC_PI_2;
        let p = c + vec2(a.cos() * r * 1.3, a.sin() * r * 1.1);
        star(p, 4.0 + 2.0 * (t * 6.0 + k as f32).sin().abs(), Color::new(1.0, 1.0, 0.8, 0.9), a);
    }
}

/// A jack-o'-lantern sitting on the ground at `at` (its centre).
fn pumpkin(at: Vec2, t: f32) {
    let orange = Color::new(0.95, 0.5, 0.08, 1.0);
    let dark = Color::new(0.65, 0.28, 0.04, 1.0);
    for (dx, r) in [(-9.0, 12.0), (9.0, 12.0), (0.0, 14.0)] {
        draw_ellipse(at.x + dx, at.y, r, 14.0, 0.0, if dx == 0.0 { orange } else { dark });
    }
    draw_ellipse(at.x, at.y, 9.0, 13.5, 0.0, orange);
    draw_rectangle(at.x - 2.0, at.y - 19.0, 4.0, 7.0, Color::new(0.3, 0.45, 0.15, 1.0));
    // The candle inside flickers through the face.
    let glow = Color::new(1.0, 0.85 + 0.15 * (t * 11.0 + at.x).sin(), 0.3, 1.0);
    for s in [-1.0f32, 1.0] {
        let e = at + vec2(s * 6.0, -4.0);
        draw_triangle(e + vec2(-3.5, 2.5), e + vec2(3.5, 2.5), e + vec2(0.0, -3.5), glow);
    }
    let m = at + vec2(0.0, 5.0);
    draw_triangle(m + vec2(-9.0, -1.0), m + vec2(9.0, -1.0), m + vec2(0.0, 5.0), glow);
    for k in [-5.0f32, 0.0, 5.0] {
        draw_triangle(m + vec2(k - 2.0, -1.0), m + vec2(k + 2.0, -1.0), m + vec2(k, -4.0), orange);
    }
}

/// A vent on the sea floor.
fn vent(at: Vec2) {
    draw_rectangle(at.x - 14.0, at.y - 6.0, 28.0, 10.0, STEEL_DARK);
    draw_rectangle(at.x - 9.0, at.y - 12.0, 18.0, 8.0, STEEL);
    draw_ellipse(at.x, at.y - 12.0, 9.0, 3.0, 0.0, INK);
}

fn bubble(b: &Bubble) {
    let r = b.radius();
    let wobble = 1.0 + 0.08 * (b.age * 9.0 + b.seed).sin();
    draw_ellipse(b.pos.x, b.pos.y, r * wobble, r / wobble, 0.0, Color::new(0.75, 0.9, 1.0, 0.18));
    draw_ellipse_lines(b.pos.x, b.pos.y, r * wobble, r / wobble, 0.0, 1.5, Color::new(0.85, 0.95, 1.0, 0.8));
    draw_circle(b.pos.x - r * 0.35, b.pos.y - r * 0.35, r * 0.25, Color::new(1.0, 1.0, 1.0, 0.7));
}

/// A shock wave running along the ground.
fn wave(at: Vec2, t: f32) {
    for k in 0..3 {
        let h = 18.0 - k as f32 * 5.0 + (t * 30.0 + k as f32).sin() * 2.0;
        let a = 0.75 - k as f32 * 0.2;
        draw_ellipse(at.x, at.y + 8.0 - h / 2.0, 14.0 - k as f32 * 3.0, h, 0.0, Color::new(0.7, 0.9, 1.0, a));
    }
    draw_ellipse(at.x, at.y + 10.0, 20.0, 4.0, 0.0, Color::new(1.0, 1.0, 1.0, 0.6));
}

/// The bosses.
fn boss(b: &Boss, t: f32) {
    match b.kind {
        BossKind::Bomber => bomber(b, t),
        BossKind::Anchor => crane(b, t),
    }
}

/// The anchor crane: a striped hovering cab, a chain and the anchor.
fn crane(b: &Boss, t: f32) {
    let p = b.pos;
    let hit = b.flash > 0.0 && (t * 20.0) as i32 % 2 == 0;
    let yellow = if hit { WHITE } else { Color::new(0.98, 0.78, 0.15, 1.0) };
    let r = BOSS_R;
    // The chain: links from the hook to the anchor.
    let top = p + vec2(0.0, r * 0.6);
    let n = ((b.anchor - top).length() / 12.0).max(1.0) as usize;
    for k in 0..n {
        let q = top + (b.anchor - top) * (k as f32 + 0.5) / n as f32;
        let flat = k % 2 == 0;
        draw_ellipse_lines(q.x, q.y, if flat { 5.0 } else { 2.0 }, 6.0, 0.0, 2.0, STEEL_DARK);
    }
    anchor(b.anchor, (b.anchor - top).normalize_or(vec2(0.0, 1.0)));
    // Twin rotors.
    for k in [-1.0f32, 1.0] {
        let hub = p + vec2(k * r * 0.9, -r * 0.7);
        let w = r * 0.55 * (t * 28.0 + k).sin().abs();
        draw_line(hub.x, hub.y + 10.0, hub.x, hub.y, 3.0, STEEL_DARK);
        draw_line(hub.x - w, hub.y, hub.x + w, hub.y, 3.0, STEEL);
    }
    // The cab: hazard stripes, a window with the robot eye.
    draw_rectangle(p.x - r * 1.2, p.y - r * 0.6, r * 2.4, r * 1.2, yellow);
    for k in 0..6 {
        let x = p.x - r * 1.2 + k as f32 * r * 0.42;
        draw_triangle(
            vec2(x, p.y + r * 0.6),
            vec2(x + r * 0.2, p.y + r * 0.6),
            vec2(x + r * 0.42, p.y + r * 0.25),
            INK,
        );
    }
    draw_rectangle_lines(p.x - r * 1.2, p.y - r * 0.6, r * 2.4, r * 1.2, 2.0, STEEL_DARK);
    draw_rectangle(p.x - r * 0.55, p.y - r * 0.45, r * 1.1, r * 0.55, Color::new(0.55, 0.8, 1.0, 0.8));
    let look = vec2((t * 1.1).sin() * 8.0, 0.0);
    draw_circle(p.x + look.x, p.y - r * 0.18, 7.0, Color::new(0.15, 0.15, 0.2, 1.0));
    draw_circle(p.x + look.x, p.y - r * 0.18, 3.5, Color::new(1.0, 0.2, 0.2, 1.0));
    draw_rectangle(p.x - 6.0, p.y + r * 0.6, 12.0, 8.0, STEEL_DARK);
    for k in 0..(BOSS_HP - b.hp).min(6) {
        let x = p.x - r + k as f32 * r * 0.35;
        draw_line(x, p.y - r * 0.5, x + 6.0, p.y - r * 0.2, 2.0, Color::new(0.1, 0.1, 0.1, 0.6));
    }
}

/// A ship's anchor hanging along `down`.
fn anchor(c: Vec2, down: Vec2) {
    let side = vec2(-down.y, down.x);
    let r = ANCHOR_R;
    let ring = c - down * r * 1.1;
    let crown = c + down * r * 0.75;
    draw_circle_lines(ring.x, ring.y, 5.0, 2.5, STEEL_DARK);
    // Shank and stock.
    draw_line(ring.x, ring.y, crown.x, crown.y, 6.0, STEEL_DARK);
    let (s0, s1) = (c - down * r * 0.6 - side * r * 0.6, c - down * r * 0.6 + side * r * 0.6);
    draw_line(s0.x, s0.y, s1.x, s1.y, 5.0, STEEL_DARK);
    // Arms curving up to the flukes.
    for k in [-1.0f32, 1.0] {
        let mut last = crown;
        for i in 1..=6 {
            let a = i as f32 / 6.0 * 1.3;
            let q = crown - down * r * 0.75 * (1.0 - a.cos()) + side * k * r * a.sin();
            draw_line(last.x, last.y, q.x, q.y, 6.0, STEEL_DARK);
            last = q;
        }
        draw_triangle(last, last - down * 12.0 + side * k * 2.0, last + side * k * 9.0 - down * 2.0, STEEL);
    }
    draw_circle(crown.x, crown.y, 6.0, STEEL);
}

/// The first boss: a hovering pod with a glass dome, arms and a propeller.
fn bomber(b: &Boss, t: f32) {
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
