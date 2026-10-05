//! Sonic drawn with shapes (no sprites from the games): standing, walking,
//! running with blurred legs, braking, crouching, looking up, pushing,
//! springing, and curled into a spinning ball.

use super::Sonic;
use macroquad::prelude::*;

const BLUE: Color = Color::new(0.12, 0.36, 0.9, 1.0);
const DARK: Color = Color::new(0.06, 0.2, 0.62, 1.0);
const SKIN: Color = Color::new(1.0, 0.82, 0.62, 1.0);
const RED: Color = Color::new(0.88, 0.12, 0.14, 1.0);
const SHOE_DARK: Color = Color::new(0.6, 0.06, 0.08, 1.0);
const WHITE_: Color = Color::new(0.98, 0.98, 1.0, 1.0);
const INK: Color = Color::new(0.05, 0.05, 0.1, 1.0);

/// Local guide pixels (x forward, y down) → world pixels.
struct Frame {
    at: Vec2,
    sin: f32,
    cos: f32,
    facing: f32,
    scale: f32,
}

impl Frame {
    fn p(&self, x: f32, y: f32) -> Vec2 {
        let (x, y) = (x * self.facing, y);
        self.at + vec2(x * self.cos + y * self.sin, -x * self.sin + y * self.cos) * self.scale
    }

    fn circle(&self, x: f32, y: f32, r: f32, c: Color) {
        let p = self.p(x, y);
        draw_circle(p.x, p.y, r * self.scale, c);
    }

    fn ellipse(&self, x: f32, y: f32, rx: f32, ry: f32, tilt: f32, c: Color) {
        let p = self.p(x, y);
        let rot = (-self.angle() + tilt * self.facing).to_degrees();
        draw_ellipse(p.x, p.y, rx * self.scale, ry * self.scale, rot, c);
    }

    fn angle(&self) -> f32 {
        self.sin.atan2(self.cos)
    }

    fn tri(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32), col: Color) {
        draw_triangle(self.p(a.0, a.1), self.p(b.0, b.1), self.p(c.0, c.1), col);
    }

    fn line(&self, a: (f32, f32), b: (f32, f32), w: f32, col: Color) {
        let (a, b) = (self.p(a.0, a.1), self.p(b.0, b.1));
        draw_line(a.x, a.y, b.x, b.y, w * self.scale, col);
    }
}

/// Draw Sonic (world pass).
pub fn draw(s: &Sonic) {
    let upright = !s.grounded || (s.gsp == 0.0 && super::mode_down(s.angle).y > 0.5 && s.angle.sin().abs() < 0.4);
    let angle = if upright && !s.grounded {
        s.angle
    } else if upright {
        0.0
    } else {
        s.angle
    };
    let (sin, cos) = angle.sin_cos();
    let f = Frame { at: s.pos, sin, cos, facing: s.facing, scale: super::SCALE };
    // Shadow-free silhouette outline for contrast on dark backgrounds.
    if s.is_ball() {
        ball(&f, s);
    } else {
        body(&f, s);
    }
}

fn ball(f: &Frame, s: &Sonic) {
    // The ball sits on the ground: centre on his (smaller) centre.
    let r = 13.0;
    f.circle(0.0, 0.0, r + 1.0, DARK);
    f.circle(0.0, 0.0, r, BLUE);
    let spin = s.spin;
    // Quills sweeping round.
    for k in 0..5 {
        let a = spin + k as f32 * std::f32::consts::TAU / 5.0;
        let tip = (a.cos() * (r + 2.5), a.sin() * (r + 2.5));
        let b1 = ((a - 0.5).cos() * (r - 5.0), (a - 0.5).sin() * (r - 5.0));
        let b2 = ((a + 0.4).cos() * (r - 2.0), (a + 0.4).sin() * (r - 2.0));
        f.tri(b1, b2, tip, DARK);
    }
    // A flash of skin and shoe going round.
    let a = spin * 1.0 + 1.3;
    f.circle(a.cos() * 6.0, a.sin() * 6.0, 4.0, SKIN);
    let b = a + 2.4;
    f.circle(b.cos() * 8.0, b.sin() * 8.0, 3.2, RED);
    // Motion arcs.
    let speed = s.gsp.abs().max(s.xsp.abs());
    if speed > 3.0 {
        let k = ((speed - 3.0) / 8.0).min(1.0);
        for i in 0..3 {
            let a0 = spin * 0.5 + i as f32 * 2.1;
            let p1 = (a0.cos() * (r + 3.0), a0.sin() * (r + 3.0));
            let p2 = ((a0 + 0.9).cos() * (r + 3.0), (a0 + 0.9).sin() * (r + 3.0));
            f.line(p1, p2, 1.2, Color::new(1.0, 1.0, 1.0, 0.5 * k));
        }
    }
    if let Some(rev) = s.spindash {
        // Revving: dust kicked out behind.
        let k = 0.4 + rev / 8.0 * 0.6;
        for i in 0..4 {
            let t = (s.spin * 2.0 + i as f32 * 1.7).sin().abs();
            f.circle(-r - 4.0 - t * 8.0, r - 3.0 - t * 4.0, 2.5 + t * 2.0, Color::new(0.85, 0.85, 0.9, 0.5 * k));
        }
    }
}

fn body(f: &Frame, s: &Sonic) {
    let speed = s.gsp.abs();
    let running = s.grounded && speed >= 6.0;
    let walking = s.grounded && speed > 0.05;
    let crouch = if s.crouching { 6.0 } else { 0.0 };
    let look = if s.looking_up { -2.0 } else { 0.0 };
    // Lean into the run, back when braking.
    let lean: f32 = if s.skidding {
        -0.35
    } else if s.pushing {
        0.3
    } else if running {
        0.18
    } else {
        0.0
    };
    let (ls, lc) = lean.sin_cos();
    let tilt = |x: f32, y: f32| -> (f32, f32) {
        // Lean around the feet (y = 19).
        let (dx, dy) = (x, y - 19.0);
        (dx * lc - dy * ls, dx * ls + dy * lc + 19.0)
    };
    let t = |x: f32, y: f32| tilt(x, y + crouch);
    let phase = s.stride;
    let air = !s.grounded;

    // Back arm (behind the body).
    let swing = if walking { phase.sin() } else { 0.0 };
    let shoulder = t(-1.0, -1.0);
    let back_hand = if air {
        t(-4.0, -14.0)
    } else if running {
        t(-9.0, 4.0)
    } else {
        t(-3.0 - swing * 6.0, 7.0)
    };
    f.line(shoulder, back_hand, 2.4, SKIN);
    f.circle(back_hand.0, back_hand.1, 2.6, WHITE_);

    // Legs and shoes.
    let hip = t(0.0, 8.0);
    if running {
        // Blurred figure-of-eight.
        let c = t(3.0, 15.0);
        f.ellipse(c.0, c.1, 8.0, 4.0, 0.3, Color::new(RED.r, RED.g, RED.b, 0.55));
        f.ellipse(c.0 - 2.0, c.1, 6.0, 3.0, -0.4, Color::new(BLUE.r, BLUE.g, BLUE.b, 0.5));
        for k in 0..2 {
            let a = phase * 2.0 + k as f32 * std::f32::consts::PI;
            let foot = t(3.0 + a.cos() * 7.0, 16.0 + a.sin() * 3.0);
            f.line(hip, foot, 2.6, BLUE);
            shoe(f, foot, 0.0, 0.8);
        }
    } else {
        let feet = if air {
            [t(-3.0, 18.0), t(4.0, 19.0)]
        } else if s.crouching {
            [t(-5.0, 19.0 - crouch), t(5.0, 19.0 - crouch)]
        } else if walking {
            let (a, b) = (phase.sin(), (phase + std::f32::consts::PI).sin());
            [t(a * 6.0, 18.5 - (phase.cos()).max(0.0) * 3.0), t(b * 6.0, 18.5 - (-phase.cos()).max(0.0) * 3.0)]
        } else {
            [t(-3.5, 18.5), t(3.5, 18.5)]
        };
        for foot in feet {
            f.line(hip, foot, 2.6, BLUE);
            shoe(f, foot, 0.0, 1.0);
        }
    }

    // Quills, then head and body over them.
    let h = t(1.0, -8.0 + look * 0.5);
    let q = |x: f32, y: f32| t(x, y + look * 0.5);
    f.tri(q(-2.0, -16.0), q(-4.0, -9.0), q(-20.0, -17.0), DARK);
    f.tri(q(-4.0, -11.0), q(-4.0, -3.0), q(-22.0, -7.0), DARK);
    f.tri(q(-4.0, -6.0), q(-2.0, 1.0), q(-17.0, 3.0), DARK);
    f.tri(q(-2.0, -15.0), q(-4.0, -9.0), q(-18.0, -15.5), BLUE);
    f.tri(q(-4.0, -10.0), q(-4.0, -4.0), q(-20.0, -7.0), BLUE);
    // Body and belly.
    let b = t(0.0, 3.0);
    f.ellipse(b.0, b.1, 6.5, 7.5, lean, BLUE);
    let belly = t(2.5, 4.0);
    f.ellipse(belly.0, belly.1, 3.6, 5.0, lean, SKIN);
    // Head.
    f.circle(h.0, h.1, 10.0, BLUE);
    // Ear.
    f.tri(q(-3.0, -15.0), q(2.0, -17.0), q(-3.0, -23.0), BLUE);
    f.tri(q(-2.0, -16.0), q(1.0, -17.0), q(-2.5, -20.5), SKIN);
    // Muzzle, nose and mouth.
    let m = q(8.0, -4.5);
    f.ellipse(m.0, m.1, 5.5, 3.8, lean, SKIN);
    let nose = q(12.5, -7.0);
    f.circle(nose.0, nose.1, 1.7, INK);
    f.line(q(7.0, -2.5), q(10.5, -3.0), 0.8, INK);
    // Eye: one shared white shape with a pupil looking ahead (or up).
    let e = q(5.0, -11.0);
    f.ellipse(e.0, e.1, 4.0, 5.2, lean, WHITE_);
    let pupil = q(7.2, -11.0 + look);
    f.ellipse(pupil.0, pupil.1, 1.4, 2.4, lean, INK);
    // Front arm.
    let front_hand = if air {
        t(4.0, -15.0)
    } else if s.pushing {
        t(10.0, -1.0)
    } else if running {
        t(8.0, 2.0)
    } else if s.skidding {
        t(-2.0, 3.0)
    } else {
        t(3.0 + swing * 6.0, 7.0)
    };
    let fs = t(2.0, 0.0);
    f.line(fs, front_hand, 2.4, SKIN);
    f.circle(front_hand.0, front_hand.1, 2.8, WHITE_);
}

fn shoe(f: &Frame, at: (f32, f32), tilt: f32, alpha: f32) {
    let c = |c: Color| Color { a: c.a * alpha, ..c };
    f.ellipse(at.0 + 2.5, at.1, 5.5, 2.8, tilt, c(RED));
    f.ellipse(at.0 + 2.5, at.1 + 1.6, 5.0, 1.0, tilt, c(SHOE_DARK));
    f.line((at.0 + 0.5, at.1 - 2.5), (at.0 + 0.5, at.1 + 2.0), 1.4, c(WHITE_));
}

/// A ring (world px), turning.
pub fn ring(at: Vec2, t: f32, scale: f32) {
    let w = (t * 4.0).cos().abs().max(0.12);
    let (rx, ry) = (6.0 * w * scale, 6.0 * scale);
    let gold = Color::new(1.0, 0.82, 0.15, 1.0);
    let dark = Color::new(0.7, 0.45, 0.05, 1.0);
    draw_ellipse_lines(at.x, at.y, rx + 0.6, ry + 0.6, 0.0, 3.4 * scale, dark);
    draw_ellipse_lines(at.x, at.y, rx, ry, 0.0, 2.4 * scale, gold);
    draw_circle(at.x - rx * 0.5, at.y - ry * 0.6, 1.1 * scale, Color::new(1.0, 1.0, 0.9, 0.9));
}
