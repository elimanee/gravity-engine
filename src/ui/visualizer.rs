//! Audio visualizer: a full-screen layer behind the objects, and a live
//! "screen" drawn on visualizer objects (rotated with their body).

use super::theme::*;
use crate::audio::analyzer::{Analyzer, BANDS, WAVE_POINTS};
use crate::util::hsv_to_rgb;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::{PI, TAU};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum VisStyle {
    Off,
    Bars,
    Wave,
    Radial,
}

impl VisStyle {
    pub const ALL: &'static [VisStyle] = &[VisStyle::Off, VisStyle::Bars, VisStyle::Wave, VisStyle::Radial];

    pub fn label(self) -> &'static str {
        match self {
            VisStyle::Off => "Off",
            VisStyle::Bars => "Bars",
            VisStyle::Wave => "Wave",
            VisStyle::Radial => "Radial",
        }
    }

    /// Next style; `with_off` controls whether `Off` is part of the cycle.
    pub fn cycle(self, dir: i32, with_off: bool) -> Self {
        let list: Vec<VisStyle> = Self::ALL.iter().copied().filter(|s| with_off || *s != VisStyle::Off).collect();
        let i = list.iter().position(|&s| s == self).unwrap_or(0) as i32;
        list[(i + dir).rem_euclid(list.len() as i32) as usize]
    }
}

/// Colour of band `i` (violet lows → cyan mids → pink highs).
fn band_color(i: usize, alpha: f32) -> Color {
    let u = i as f32 / (BANDS - 1) as f32;
    let (r, g, b) = hsv_to_rgb(0.74 - u * 0.62, 0.62, 1.0);
    Color::from_rgba(r, g, b, (alpha * 255.0) as u8)
}

/// Gentle placeholder motion while nothing is playing.
fn idle_level(i: usize, t: f32) -> f32 {
    0.04 + 0.03 * ((t * 1.3 + i as f32 * 0.35).sin() * 0.5 + 0.5)
}

fn level(a: &Analyzer, i: usize, t: f32) -> f32 {
    if a.active {
        a.bands[i]
    } else {
        a.bands[i].max(idle_level(i, t))
    }
}

// ── Full-screen layer ───────────────────────────────────────────
pub fn draw_background(style: VisStyle, a: &Analyzer, sw: f32, sh: f32, floor: f32) {
    let t = get_time() as f32;
    // A soft flash on each beat.
    if a.beat > 0.0 && style != VisStyle::Off {
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.55, 0.45, 1.0, a.beat * 0.05));
    }
    match style {
        VisStyle::Off => {}
        VisStyle::Bars => {
            let base = sh - floor;
            let max_h = sh * 0.45;
            let slot = sw / BANDS as f32;
            let w = slot * 0.72;
            for i in 0..BANDS {
                let h = level(a, i, t) * max_h;
                let x = i as f32 * slot + (slot - w) / 2.0;
                let top = band_color(i, 0.42);
                let bottom = band_color(i, 0.10);
                gradient_v(x, base - h, w, h, top, bottom);
                // Reflection on the floor side.
                gradient_v(x, base, w, (h * 0.25).min(floor), band_color(i, 0.10), band_color(i, 0.0));
                let py = base - a.peaks[i] * max_h - 3.0;
                if a.active {
                    draw_rectangle(x, py, w, 2.0, band_color(i, 0.6));
                }
            }
        }
        VisStyle::Wave => {
            let cy = sh * 0.5;
            let amp = sh * 0.22 * (0.7 + a.level * 0.6);
            let pts: Vec<Vec2> = (0..WAVE_POINTS)
                .map(|i| {
                    let x = sw * i as f32 / (WAVE_POINTS - 1) as f32;
                    let v = if a.active { a.wave[i] } else { (t * 2.0 + i as f32 * 0.08).sin() * 0.03 };
                    vec2(x, cy - v * amp)
                })
                .collect();
            for (thick, alpha) in [(9.0, 0.06), (4.0, 0.14), (1.8, 0.55)] {
                for i in 0..pts.len() - 1 {
                    let c = band_color(i * BANDS / WAVE_POINTS, alpha);
                    draw_line(pts[i].x, pts[i].y, pts[i + 1].x, pts[i + 1].y, thick, c);
                }
            }
        }
        VisStyle::Radial => {
            let c = vec2(sw / 2.0, (sh - floor) / 2.0);
            let r0 = sw.min(sh) * 0.14 * (1.0 + a.bass * 0.25 + a.beat * 0.15);
            let len = sw.min(sh) * 0.26;
            radial(c.x, c.y, r0 * 1.2, Color::new(0.5, 0.4, 1.0, 0.10 + a.beat * 0.12), Color::new(0.5, 0.4, 1.0, 0.0));
            let spokes = BANDS * 2;
            let rot = t * 0.1;
            for k in 0..spokes {
                // Mirror the spectrum so it is symmetric left/right.
                let i = if k < BANDS { k } else { spokes - 1 - k };
                let ang = rot + k as f32 / spokes as f32 * TAU - PI / 2.0;
                let l = level(a, i, t) * len;
                let dir = vec2(ang.cos(), ang.sin());
                let p0 = c + dir * r0;
                let p1 = c + dir * (r0 + 2.0 + l);
                draw_line(p0.x, p0.y, p1.x, p1.y, (TAU * r0 / spokes as f32 * 0.55).max(1.5), band_color(i, 0.45));
            }
            draw_circle_lines(c.x, c.y, r0, 1.5, Color::new(0.8, 0.75, 1.0, 0.35 + a.beat * 0.4));
        }
    }
}

// ── Visualizer objects ──────────────────────────────────────────
/// Maps local panel coordinates (origin at the centre, y down) to the screen.
struct Frame {
    c: Vec2,
    cos: f32,
    sin: f32,
}

impl Frame {
    fn p(&self, x: f32, y: f32) -> Vec2 {
        self.c + vec2(x * self.cos - y * self.sin, x * self.sin + y * self.cos)
    }
    fn quad(&self, x: f32, y: f32, w: f32, h: f32, color: Color) {
        let (a, b, c, d) = (self.p(x, y), self.p(x + w, y), self.p(x + w, y + h), self.p(x, y + h));
        draw_triangle(a, b, c, color);
        draw_triangle(a, c, d, color);
    }
    fn line(&self, x0: f32, y0: f32, x1: f32, y1: f32, thick: f32, color: Color) {
        let (a, b) = (self.p(x0, y0), self.p(x1, y1));
        draw_line(a.x, a.y, b.x, b.y, thick, color);
    }
    /// Rounded rectangle centred on the origin, as a triangle fan.
    fn rounded(&self, w: f32, h: f32, r: f32, color: Color) {
        let pts = rounded_points(w, h, r);
        let c = self.p(0.0, 0.0);
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            draw_triangle(c, self.p(a.x, a.y), self.p(b.x, b.y), color);
        }
    }
    fn rounded_lines(&self, w: f32, h: f32, r: f32, thick: f32, color: Color) {
        let pts = rounded_points(w, h, r);
        for i in 0..pts.len() {
            let (a, b) = (pts[i], pts[(i + 1) % pts.len()]);
            self.line(a.x, a.y, b.x, b.y, thick, color);
        }
    }
}

fn rounded_points(w: f32, h: f32, r: f32) -> Vec<Vec2> {
    let r = r.min(w / 2.0).min(h / 2.0);
    let (hw, hh) = (w / 2.0, h / 2.0);
    let corners =
        [(hw - r, -hh + r, -PI / 2.0), (hw - r, hh - r, 0.0), (-hw + r, hh - r, PI / 2.0), (-hw + r, -hh + r, PI)];
    let mut pts = Vec::with_capacity(28);
    for (cx, cy, a0) in corners {
        for i in 0..=6 {
            let a = a0 + PI / 2.0 * i as f32 / 6.0;
            pts.push(vec2(cx + a.cos() * r, cy + a.sin() * r));
        }
    }
    pts
}

/// Draw a visualizer screen of `size` at `pos`, rotated by the body `angle`.
pub fn draw_object(style: VisStyle, a: &Analyzer, pos: Vec2, angle: f32, size: Vec2, alpha: f32) {
    let t = get_time() as f32;
    let (sin, cos) = (-angle).sin_cos();
    let f = Frame { c: pos, cos, sin };
    let (w, h) = (size.x, size.y);
    let radius = (w.min(h) * 0.12).min(14.0);

    // Shadow, bezel, screen.
    let shadow = Frame { c: pos + vec2(3.0, 4.0), cos, sin };
    shadow.rounded(w, h, radius, Color::new(0.0, 0.0, 0.0, 0.25 * alpha));
    f.rounded(w, h, radius, fade(Color::new(0.10, 0.09, 0.17, 1.0), alpha));
    let pad = (w.min(h) * 0.07).clamp(3.0, 10.0);
    let (iw, ih) = (w - pad * 2.0, h - pad * 2.0);
    f.rounded(iw, ih, radius * 0.6, fade(Color::new(0.03, 0.03, 0.07, 1.0), alpha));
    let glow = 0.35 + a.beat * 0.6;
    f.rounded_lines(w, h, radius, 1.5, fade(Color::new(0.55, 0.47, 1.0, glow), alpha));

    let (x0, y0) = (-iw / 2.0, -ih / 2.0);
    match style {
        VisStyle::Off | VisStyle::Bars => {
            let n = ((iw / 6.0) as usize).clamp(8, BANDS);
            let slot = iw / n as f32;
            let bw = slot * 0.7;
            for k in 0..n {
                let i = k * BANDS / n;
                let lv = level(a, i, t);
                let bh = lv * (ih - 4.0);
                let x = x0 + k as f32 * slot + (slot - bw) / 2.0;
                f.quad(x, y0 + ih - 2.0 - bh, bw, bh, fade(band_color(i, 0.95), alpha));
                if a.active {
                    let py = y0 + ih - 2.0 - a.peaks[i] * (ih - 4.0) - 2.0;
                    f.quad(x, py, bw, 1.5, fade(Color::new(1.0, 1.0, 1.0, 0.7), alpha));
                }
            }
        }
        VisStyle::Wave => {
            let cy = 0.0;
            f.line(x0, cy, x0 + iw, cy, 1.0, fade(Color::new(1.0, 1.0, 1.0, 0.08), alpha));
            let step = 4;
            let mut prev: Option<(f32, f32)> = None;
            for i in (0..WAVE_POINTS).step_by(step) {
                let x = x0 + iw * i as f32 / (WAVE_POINTS - 1) as f32;
                let v = if a.active { a.wave[i] } else { (t * 3.0 + i as f32 * 0.1).sin() * 0.04 };
                let y = cy - v * ih * 0.45;
                if let Some((px, py)) = prev {
                    f.line(px, py, x, y, 2.0, fade(band_color(i * BANDS / WAVE_POINTS, 0.95), alpha));
                }
                prev = Some((x, y));
            }
        }
        VisStyle::Radial => {
            let r0 = iw.min(ih) * 0.18 * (1.0 + a.bass * 0.3);
            let len = iw.min(ih) * 0.28;
            let spokes = 48;
            for k in 0..spokes {
                let i = (if k < spokes / 2 { k } else { spokes - 1 - k }) * BANDS * 2 / spokes;
                let i = i.min(BANDS - 1);
                let ang = k as f32 / spokes as f32 * TAU + t * 0.2;
                let l = level(a, i, t) * len;
                let (dx, dy) = (ang.cos(), ang.sin());
                f.line(
                    dx * r0,
                    dy * r0,
                    dx * (r0 + 1.5 + l),
                    dy * (r0 + 1.5 + l),
                    2.0,
                    fade(band_color(i, 0.95), alpha),
                );
            }
            let c = f.p(0.0, 0.0);
            draw_circle_lines(c.x, c.y, r0, 1.2, fade(Color::new(0.85, 0.8, 1.0, 0.6 + a.beat * 0.4), alpha));
        }
    }

    // A little status LED: lit while audio is flowing.
    let led = f.p(w / 2.0 - pad - 3.0, -h / 2.0 + pad * 0.5 + 1.0);
    let on = if a.active { SUCCESS } else { TEXT_MUTED };
    draw_circle(led.x, led.y, 1.8, fade(on, alpha));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_skips_off_for_objects() {
        assert_eq!(VisStyle::Radial.cycle(1, true), VisStyle::Off);
        assert_eq!(VisStyle::Radial.cycle(1, false), VisStyle::Bars);
        assert_eq!(VisStyle::Bars.cycle(-1, false), VisStyle::Radial);
    }
}
