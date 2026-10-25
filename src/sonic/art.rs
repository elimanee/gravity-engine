//! Sonic, Tails, Knuckles and Amy drawn with shapes (no sprites from the
//! games): standing, walking, running with blurred legs, braking,
//! crouching, looking up, pushing, springing and curled into a spinning
//! ball; Tails flying on his twin tails, Knuckles gliding, climbing and
//! sliding, Amy swinging her hammer.

use super::{Character, Sonic};
use macroquad::prelude::*;

const SKIN: Color = Color::new(1.0, 0.82, 0.62, 1.0);
const RED: Color = Color::new(0.88, 0.12, 0.14, 1.0);
const SHOE_DARK: Color = Color::new(0.6, 0.06, 0.08, 1.0);
const WHITE_: Color = Color::new(0.98, 0.98, 1.0, 1.0);
const INK: Color = Color::new(0.05, 0.05, 0.1, 1.0);
const GOLD: Color = Color::new(1.0, 0.82, 0.2, 1.0);

/// A character's colours.
struct Look {
    main: Color,
    dark: Color,
    /// Muzzle, belly and arms.
    skin: Color,
    belly: Color,
    shoe: Color,
    shoe_dark: Color,
}

fn look(c: Character) -> Look {
    match c {
        Character::Sonic => Look {
            main: Color::new(0.12, 0.36, 0.9, 1.0),
            dark: Color::new(0.06, 0.2, 0.62, 1.0),
            skin: SKIN,
            belly: SKIN,
            shoe: RED,
            shoe_dark: SHOE_DARK,
        },
        Character::Tails => Look {
            main: Color::new(0.98, 0.6, 0.14, 1.0),
            dark: Color::new(0.76, 0.4, 0.06, 1.0),
            skin: WHITE_,
            belly: WHITE_,
            shoe: RED,
            shoe_dark: SHOE_DARK,
        },
        Character::Knuckles => Look {
            main: Color::new(0.86, 0.1, 0.16, 1.0),
            dark: Color::new(0.56, 0.04, 0.09, 1.0),
            skin: SKIN,
            belly: WHITE_,
            shoe: Color::new(0.16, 0.62, 0.28, 1.0),
            shoe_dark: Color::new(0.95, 0.75, 0.15, 1.0),
        },
        Character::Exe => Look {
            main: Color::new(0.1, 0.2, 0.62, 1.0),
            dark: Color::new(0.04, 0.08, 0.34, 1.0),
            skin: Color::new(0.93, 0.78, 0.66, 1.0),
            belly: Color::new(0.93, 0.78, 0.66, 1.0),
            shoe: RED,
            shoe_dark: SHOE_DARK,
        },
        Character::Amy => Look {
            main: Color::new(1.0, 0.46, 0.66, 1.0),
            dark: Color::new(0.82, 0.26, 0.46, 1.0),
            skin: SKIN,
            belly: SKIN,
            shoe: RED,
            shoe_dark: SHOE_DARK,
        },
    }
}

/// Super: golden fur, shimmering.
fn golden(l: Look, t: f32) -> Look {
    let shine = 0.1 * (t * 10.0).sin();
    Look { main: Color::new(1.0, 0.84 + shine, 0.22 + shine, 1.0), dark: Color::new(0.9, 0.62, 0.08, 1.0), ..l }
}

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

/// Draw the character (world pass).
pub fn draw(s: &Sonic) {
    let upright = !s.grounded || (s.gsp == 0.0 && super::mode_down(s.angle).y > 0.5 && s.angle.sin().abs() < 0.4);
    let mut angle = if upright && s.grounded { 0.0 } else { s.angle };
    let mut at = s.pos;
    // Lying flat: gliding, or sliding on the belly.
    if s.gliding.is_some() || s.sliding {
        angle = -std::f32::consts::FRAC_PI_2 * s.facing;
        if s.sliding {
            at.y += (s.radii().1 - 9.0) * super::SCALE;
        }
    }
    let (sin, cos) = angle.sin_cos();
    let f = Frame { at, sin, cos, facing: s.facing, scale: super::SCALE };
    let l = if s.super_form { golden(look(s.character), get_time() as f32) } else { look(s.character) };
    if s.is_ball() && !(s.hammer_air && s.hammer.is_some()) {
        ball(&f, s, &l);
        if s.character == Character::Tails {
            // His tails trail behind the ball.
            tails(&f, s, &l, (-12.0, 4.0), 2.6, false);
        }
    } else {
        body(&f, s, &l);
    }
}

fn ball(f: &Frame, s: &Sonic, l: &Look) {
    // The ball sits on the ground: centre on his (smaller) centre.
    let r = 13.0;
    f.circle(0.0, 0.0, r + 1.0, l.dark);
    f.circle(0.0, 0.0, r, l.main);
    let spin = s.spin;
    // Quills sweeping round.
    for k in 0..5 {
        let a = spin + k as f32 * std::f32::consts::TAU / 5.0;
        let tip = (a.cos() * (r + 2.5), a.sin() * (r + 2.5));
        let b1 = ((a - 0.5).cos() * (r - 5.0), (a - 0.5).sin() * (r - 5.0));
        let b2 = ((a + 0.4).cos() * (r - 2.0), (a + 0.4).sin() * (r - 2.0));
        f.tri(b1, b2, tip, l.dark);
    }
    // A flash of skin and shoe going round.
    let a = spin * 1.0 + 1.3;
    f.circle(a.cos() * 6.0, a.sin() * 6.0, 4.0, l.skin);
    let b = a + 2.4;
    f.circle(b.cos() * 8.0, b.sin() * 8.0, 3.2, l.shoe);
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

/// Tails' two tails, from `root`; `rotor`: spun above him (flying).
fn tails(f: &Frame, s: &Sonic, l: &Look, root: (f32, f32), wag: f32, rotor: bool) {
    let t = get_time() as f32;
    for k in 0..2 {
        let (a, len) = if rotor {
            // Whirling round above his back.
            let speed = if s.tired() { 8.0 } else { 26.0 };
            (t * speed + k as f32 * std::f32::consts::PI, 13.0)
        } else {
            let base = if k == 0 { 2.7 } else { 2.35 };
            (base + (t * wag + k as f32 * 1.3).sin() * 0.18, 14.0)
        };
        let (dx, dy) = (a.cos(), a.sin() * if rotor { 0.35 } else { 1.0 });
        let (x, y) = if rotor { (root.0 - 2.0, root.1 - 12.0) } else { root };
        let mid = (x + dx * len * 0.55, y + dy * len * 0.55);
        let tip = (x + dx * len, y + dy * len);
        let tilt = dy.atan2(dx);
        f.ellipse(mid.0, mid.1, len * 0.55, 4.2, -tilt, l.dark);
        f.ellipse(mid.0, mid.1, len * 0.5, 3.5, -tilt, l.main);
        f.ellipse(tip.0, tip.1, 4.0, 3.0, -tilt, WHITE_);
    }
}

/// Amy's hammer, swung `k` (0 raised behind ‥ 1 struck down in front) from `hand`.
fn hammer(f: &Frame, hand: (f32, f32), a: f32) {
    let (dx, dy) = (a.cos(), a.sin());
    let head = (hand.0 + dx * 16.0, hand.1 + dy * 16.0);
    f.line(hand, head, 2.0, Color::new(0.55, 0.32, 0.16, 1.0));
    let tilt = dy.atan2(dx) + std::f32::consts::FRAC_PI_2;
    f.ellipse(head.0, head.1, 8.0, 5.0, -tilt, Color::new(0.85, 0.12, 0.18, 1.0));
    f.ellipse(head.0, head.1, 8.0, 1.6, -tilt, GOLD);
}

fn body(f: &Frame, s: &Sonic, l: &Look) {
    let ch = s.character;
    let speed = s.gsp.abs();
    let running = s.grounded && speed >= 6.0 && !s.sliding;
    let walking = s.grounded && speed > 0.05 && !s.sliding;
    let crouch = if s.crouching { 6.0 } else { 0.0 };
    let look_up = if s.looking_up { -2.0 } else { 0.0 };
    // Shorter characters: everything above the feet comes down.
    let short = (19.0 - s.radii().1).max(0.0);
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
        (dx * lc - dy * ls, dx * ls + dy * lc + 19.0 - short)
    };
    let t = |x: f32, y: f32| tilt(x, y + crouch + if y < 12.0 { short * 0.6 } else { 0.0 });
    let phase = s.stride;
    let air = !s.grounded || s.climbing.is_some() || s.climb_up.is_some();
    let swinging = s.hammer.filter(|_| !s.hammer_air).map(|h| (h as f32 / 22.0).min(1.0));

    // Behind the body: Tails' tails, Knuckles' dreadlocks.
    if ch == Character::Tails {
        let flying = s.flying.is_some();
        tails(f, s, l, t(-5.0, 9.0), if s.gsp == 0.0 && s.grounded { 3.0 } else { 9.0 }, flying);
    }

    // Back arm (behind the body).
    let swing = if walking { phase.sin() } else { 0.0 };
    let shoulder = t(-1.0, -1.0);
    let back_hand = if swinging.is_some() || s.gliding.is_some() {
        t(5.0, -14.0)
    } else if air {
        t(-4.0, -14.0)
    } else if running {
        t(-9.0, 4.0)
    } else {
        t(-3.0 - swing * 6.0, 7.0)
    };
    f.line(shoulder, back_hand, 2.4, l.skin);
    f.circle(back_hand.0, back_hand.1, if ch == Character::Knuckles { 3.6 } else { 2.6 }, WHITE_);

    // Legs and shoes.
    let hip = t(0.0, 8.0);
    if running {
        // Blurred figure-of-eight.
        let c = t(3.0, 15.0);
        f.ellipse(c.0, c.1, 8.0, 4.0, 0.3, Color { a: 0.55, ..l.shoe });
        f.ellipse(c.0 - 2.0, c.1, 6.0, 3.0, -0.4, Color { a: 0.5, ..l.main });
        for k in 0..2 {
            let a = phase * 2.0 + k as f32 * std::f32::consts::PI;
            let foot = t(3.0 + a.cos() * 7.0, 16.0 + a.sin() * 3.0);
            f.line(hip, foot, 2.6, l.main);
            shoe(f, l, foot, 0.0, 0.8);
        }
    } else {
        let feet = if s.gliding.is_some() || s.sliding {
            [t(-3.0, 19.0), t(2.0, 20.0)]
        } else if air {
            [t(-3.0, 18.0), t(4.0, 19.0)]
        } else if s.crouching {
            [t(-5.0, 19.0 - crouch), t(5.0, 19.0 - crouch)]
        } else if walking {
            let (a, b) = (phase.sin(), (phase + std::f32::consts::PI).sin());
            [t(a * 6.0, 18.5 - (phase.cos()).max(0.0) * 3.0), t(b * 6.0, 18.5 - (-phase.cos()).max(0.0) * 3.0)]
        } else {
            [t(-3.5, 18.5), t(3.5, 18.5)]
        };
        let leg = if ch == Character::Amy { SKIN } else { l.main };
        for foot in feet {
            f.line(hip, foot, 2.6, leg);
            shoe(f, l, foot, 0.0, 1.0);
        }
    }

    // Quills (or the like), then head and body over them.
    let h = t(1.0, -8.0 + look_up * 0.5);
    let q = |x: f32, y: f32| t(x, y + look_up * 0.5);
    match ch {
        Character::Sonic | Character::Exe => {
            f.tri(q(-2.0, -16.0), q(-4.0, -9.0), q(-20.0, -17.0), l.dark);
            f.tri(q(-4.0, -11.0), q(-4.0, -3.0), q(-22.0, -7.0), l.dark);
            f.tri(q(-4.0, -6.0), q(-2.0, 1.0), q(-17.0, 3.0), l.dark);
            f.tri(q(-2.0, -15.0), q(-4.0, -9.0), q(-18.0, -15.5), l.main);
            f.tri(q(-4.0, -10.0), q(-4.0, -4.0), q(-20.0, -7.0), l.main);
        }
        Character::Knuckles => {
            // Dreadlocks hanging down his back.
            for (k, (y0, len)) in [(-14.0, 18.0), (-9.0, 20.0), (-4.0, 16.0)].into_iter().enumerate() {
                let x = -6.0 - k as f32 * 1.5;
                f.tri(q(x + 3.0, y0 - 3.0), q(x + 2.0, y0 + 3.0), q(x - 7.0, y0 + len), l.dark);
                f.tri(q(x + 2.0, y0 - 2.0), q(x + 1.0, y0 + 2.0), q(x - 5.0, y0 + len - 3.0), l.main);
            }
        }
        Character::Amy => {
            // Short quills falling down her back.
            for (y0, len) in [(-14.0, 11.0), (-8.0, 12.0)] {
                f.tri(q(-3.0, y0 - 3.0), q(-4.0, y0 + 3.0), q(-12.0, y0 + len), l.dark);
                f.tri(q(-3.0, y0 - 2.0), q(-4.0, y0 + 2.0), q(-10.5, y0 + len - 2.0), l.main);
            }
        }
        Character::Tails => {}
    }
    // Body and belly (Amy's dress).
    let b = t(0.0, 3.0);
    if ch == Character::Amy {
        f.tri(t(-3.5, -1.0), t(3.5, -1.0), t(-8.0, 12.0), Color::new(0.85, 0.12, 0.2, 1.0));
        f.tri(t(3.5, -1.0), t(8.0, 12.0), t(-8.0, 12.0), Color::new(0.85, 0.12, 0.2, 1.0));
        f.ellipse(b.0, b.1 - 2.0, 4.5, 4.0, lean, Color::new(0.85, 0.12, 0.2, 1.0));
    } else {
        f.ellipse(b.0, b.1, 6.5, 7.5, lean, l.main);
        let belly = t(2.5, 4.0);
        f.ellipse(belly.0, belly.1, 3.6, 5.0, lean, l.belly);
    }
    if ch == Character::Knuckles {
        // The white crescent on his chest.
        let c = t(2.5, -0.5);
        f.ellipse(c.0, c.1, 4.2, 2.2, lean, WHITE_);
        let c2 = t(2.5, -1.8);
        f.ellipse(c2.0, c2.1, 3.6, 1.6, lean, l.main);
    }
    // Head.
    f.circle(h.0, h.1, 10.0, l.main);
    // Ears.
    if ch == Character::Tails {
        for dx in [-5.0, 1.0] {
            f.tri(q(dx - 2.0, -15.0), q(dx + 3.0, -16.0), q(dx - 1.0, -24.0), l.main);
            f.tri(q(dx - 1.0, -16.0), q(dx + 2.0, -16.5), q(dx - 0.5, -21.0), INK);
        }
        // Bangs.
        for k in 0..3 {
            let x = 1.0 + k as f32 * 2.5;
            f.tri(q(x - 2.0, -16.0), q(x + 2.0, -16.0), q(x + 3.5, -20.5 + k as f32), l.main);
        }
    } else if ch != Character::Knuckles {
        f.tri(q(-3.0, -15.0), q(2.0, -17.0), q(-3.0, -23.0), l.main);
        f.tri(q(-2.0, -16.0), q(1.0, -17.0), q(-2.5, -20.5), l.skin);
    }
    if ch == Character::Amy {
        // Her red headband and fringe.
        f.line(q(-6.0, -15.0), q(4.0, -17.5), 2.4, Color::new(0.85, 0.12, 0.2, 1.0));
        f.tri(q(3.0, -17.0), q(9.0, -14.0), q(6.0, -9.0), l.main);
    }
    // Muzzle, nose and mouth.
    let m = q(8.0, -4.5);
    f.ellipse(m.0, m.1, 5.5, 3.8, lean, l.skin);
    let nose = q(12.5, -7.0);
    f.circle(nose.0, nose.1, 1.7, INK);
    f.line(q(7.0, -2.5), q(10.5, -3.0), 0.8, INK);
    // Eye: one shared white shape with a pupil looking ahead (or up);
    // Sonic.exe's is black with a glowing red pupil, and it bleeds.
    let e = q(5.0, -11.0);
    let exe = ch == Character::Exe;
    f.ellipse(e.0, e.1, 4.0, 5.2, lean, if exe { INK } else { WHITE_ });
    let pupil = q(7.2, -11.0 + look_up);
    if exe {
        f.circle(pupil.0, pupil.1, 1.8, Color::new(1.0, 0.08, 0.08, 1.0));
        f.circle(pupil.0, pupil.1, 0.8, Color::new(1.0, 0.75, 0.7, 1.0));
        f.line(q(6.0, -6.5), q(6.5, -1.0), 1.1, Color::new(0.75, 0.0, 0.05, 1.0));
        f.line(q(9.0, -6.0), q(9.0, -2.0), 0.9, Color::new(0.75, 0.0, 0.05, 1.0));
    }
    let iris = if ch == Character::Amy || ch == Character::Tails {
        Color::new(0.1, 0.45, 0.2, 1.0)
    } else if ch == Character::Knuckles {
        Color::new(0.45, 0.1, 0.5, 1.0)
    } else {
        INK
    };
    if !exe {
        f.ellipse(pupil.0, pupil.1, 1.4, 2.4, lean, iris);
    }
    // Front arm.
    let front_hand = if let Some(k) = swinging {
        // Up behind her head, then down in front.
        let a = -2.4 + k * 2.6;
        t(2.0 + a.cos() * 10.0, -2.0 + a.sin() * 10.0)
    } else if s.gliding.is_some() {
        t(5.0, -15.0)
    } else if air {
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
    f.line(fs, front_hand, 2.4, l.skin);
    if let Some(k) = swinging {
        hammer(f, front_hand, -2.4 + k * 2.6 + 0.3);
    }
    if ch == Character::Amy && s.hammer_air && s.hammer.is_some() {
        hammer(f, front_hand, s.spin * 2.0);
    }
    if ch == Character::Knuckles {
        // Big spiked fists.
        f.circle(front_hand.0, front_hand.1, 3.8, WHITE_);
        f.tri(
            (front_hand.0 + 2.0, front_hand.1 - 2.5),
            (front_hand.0 + 2.0, front_hand.1 - 0.5),
            (front_hand.0 + 6.0, front_hand.1 - 1.5),
            WHITE_,
        );
    } else {
        f.circle(front_hand.0, front_hand.1, 2.8, WHITE_);
    }
}

fn shoe(f: &Frame, l: &Look, at: (f32, f32), tilt: f32, alpha: f32) {
    let c = |c: Color| Color { a: c.a * alpha, ..c };
    f.ellipse(at.0 + 2.5, at.1, 5.5, 2.8, tilt, c(l.shoe));
    f.ellipse(at.0 + 2.5, at.1 + 1.6, 5.0, 1.0, tilt, c(l.shoe_dark));
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
