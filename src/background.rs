//! Animated scene backgrounds.

use crate::ui::theme::{gradient_v, gradient_v3, radial};
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};
use std::f32::consts::TAU;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BgMode {
    Dark,
    Space,
    Grid,
    Sunset,
    Ocean,
    Aurora,
    Custom,
}

impl BgMode {
    pub const ALL: &'static [BgMode] =
        &[BgMode::Dark, BgMode::Space, BgMode::Grid, BgMode::Sunset, BgMode::Ocean, BgMode::Aurora, BgMode::Custom];

    pub fn label(self) -> &'static str {
        match self {
            BgMode::Dark => "Dark",
            BgMode::Space => "Space",
            BgMode::Grid => "Grid",
            BgMode::Sunset => "Sunset",
            BgMode::Ocean => "Ocean",
            BgMode::Aurora => "Aurora",
            BgMode::Custom => "Custom",
        }
    }

    pub fn cycle(self, dir: i32) -> Self {
        let n = Self::ALL.len() as i32;
        let i = Self::ALL.iter().position(|&m| m == self).unwrap_or(0) as i32;
        Self::ALL[(i + dir).rem_euclid(n) as usize]
    }
}

/// One aurora curtain as a single continuous mesh (no seams between columns).
fn aurora_ribbon(sw: f32, sh: f32, t: f32, c: Color, base: f32, ph: f32) {
    const COLS: usize = 96;
    let clear = Color { a: 0.0, ..c };
    let peak = Color { a: c.a * 3.0, ..c };
    let mut vertices = Vec::with_capacity((COLS + 1) * 3);
    for k in 0..=COLS {
        let u = k as f32 / COLS as f32;
        let x = sw * u;
        let y = sh * (base + 0.08 * (u * 5.0 + t * 0.3 + ph).sin() + 0.04 * (u * 13.0 - t * 0.5 + ph).sin());
        let h = sh * (0.18 + 0.06 * (u * 7.0 + t * 0.4 + ph).sin());
        vertices.push(Vertex::new(x, y, 0.0, 0.0, 0.0, clear));
        vertices.push(Vertex::new(x, y + h, 0.0, 0.0, 0.0, peak));
        vertices.push(Vertex::new(x, y + h * 1.35, 0.0, 0.0, 0.0, clear));
    }
    let mut indices = Vec::with_capacity(COLS * 12);
    for k in 0..COLS as u16 {
        let (a, b) = (k * 3, (k + 1) * 3);
        for r in 0..2 {
            indices.extend_from_slice(&[a + r, b + r, b + r + 1, a + r, b + r + 1, a + r + 1]);
        }
    }
    draw_mesh(&Mesh { vertices, indices, texture: None });
}

pub struct Background {
    pub mode: BgMode,
    pub texture: Option<Texture2D>,
    pub custom_path: Option<String>,
    /// (x, y, radius, twinkle phase) in a 4000×2500 virtual space.
    stars: Vec<(f32, f32, f32, f32)>,
}

impl Background {
    pub fn new(mode: BgMode) -> Self {
        let stars = (0..260)
            .map(|i| {
                let i = i as f32;
                (
                    (i * 137.508).fract() * 4000.0,
                    (i * 97.312).fract() * 2500.0,
                    (i * 53.1).fract() * 1.4 + 0.3,
                    (i * 23.7).fract() * TAU,
                )
            })
            .collect();
        Background { mode, texture: None, custom_path: None, stars }
    }

    /// Load a custom background image from disk.
    pub fn load_custom(&mut self, path: &str) -> Result<(), String> {
        let bytes = std::fs::read(path).map_err(|e| e.to_string())?;
        let img = image::load_from_memory(&bytes).map_err(|e| e.to_string())?.into_rgba8();
        let tex = Texture2D::from_rgba8(img.width() as u16, img.height() as u16, &img);
        tex.set_filter(FilterMode::Linear);
        self.texture = Some(tex);
        self.custom_path = Some(path.to_string());
        self.mode = BgMode::Custom;
        Ok(())
    }

    fn draw_stars(&self, sw: f32, sh: f32, t: f32, alpha: f32) {
        for &(x, y, r, off) in &self.stars {
            let tw = ((t * 1.8 + off).sin() * 0.4 + 0.6).max(0.1);
            draw_circle(x % sw, y % sh, r, Color::new(0.8, 0.84, 1.0, tw * 0.86 * alpha));
        }
    }

    pub fn draw(&self, sw: f32, sh: f32) {
        let t = get_time() as f32;
        let rgb = |r: u8, g: u8, b: u8| Color::from_rgba(r, g, b, 255);
        match self.mode {
            BgMode::Dark => {
                gradient_v3(0.0, 0.0, sw, sh, rgb(11, 11, 22), rgb(18, 15, 34), rgb(32, 20, 54));
            }
            BgMode::Space => {
                gradient_v3(0.0, 0.0, sw, sh, rgb(3, 3, 10), rgb(8, 6, 24), rgb(20, 6, 34));
                // Slowly drifting nebula blobs.
                for (i, (cr, cg, cb)) in [(120u8, 40u8, 160u8), (40, 80, 170), (170, 50, 110)].iter().enumerate() {
                    let fi = i as f32;
                    let cx = sw * (0.25 + 0.3 * fi) + (t * 0.05 + fi).sin() * 40.0;
                    let cy = sh * (0.35 + 0.15 * fi) + (t * 0.04 + fi * 2.0).cos() * 30.0;
                    let c = Color::from_rgba(*cr, *cg, *cb, 34);
                    radial(cx, cy, 280.0, c, Color { a: 0.0, ..c });
                }
                self.draw_stars(sw, sh, t, 1.0);
            }
            BgMode::Grid => {
                gradient_v(0.0, 0.0, sw, sh, rgb(10, 10, 20), rgb(14, 14, 28));
                let step = 48.0_f32;
                let (minor, major) = (Color::from_rgba(40, 40, 68, 255), Color::from_rgba(58, 58, 96, 255));
                let mut i = 0;
                let mut x = 0.0;
                while x <= sw {
                    let (th, c) = if i % 4 == 0 { (1.4, major) } else { (0.7, minor) };
                    draw_line(x, 0.0, x, sh, th, c);
                    x += step;
                    i += 1;
                }
                // Horizontal lines are anchored to the floor so the grid reads as a ruler.
                let mut i = 0;
                let mut y = sh;
                while y >= 0.0 {
                    let (th, c) = if i % 4 == 0 { (1.4, major) } else { (0.7, minor) };
                    draw_line(0.0, y, sw, y, th, c);
                    y -= step;
                    i += 1;
                }
            }
            BgMode::Sunset => {
                gradient_v3(0.0, 0.0, sw, sh * 0.62, rgb(22, 12, 54), rgb(118, 38, 88), rgb(226, 96, 60));
                gradient_v(0.0, sh * 0.62, sw, sh * 0.38, rgb(226, 96, 60), rgb(246, 170, 70));
                let (sx, sy) = (sw * 0.5, sh * 0.70);
                radial(sx, sy, 190.0, Color::from_rgba(255, 214, 130, 120), Color::from_rgba(255, 180, 90, 0));
                draw_circle(sx, sy, 46.0, Color::from_rgba(255, 236, 170, 240));
                draw_circle(sx, sy, 30.0, Color::from_rgba(255, 250, 222, 255));
                // Heat shimmer bands.
                for k in 0..5 {
                    let y = sh * 0.74 + k as f32 * 16.0 + (t * 0.8 + k as f32).sin() * 2.0;
                    draw_line(
                        sx - 80.0 + k as f32 * 6.0,
                        y,
                        sx + 80.0 - k as f32 * 6.0,
                        y,
                        3.0,
                        Color::from_rgba(246, 150, 70, 180),
                    );
                }
            }
            BgMode::Ocean => {
                gradient_v3(0.0, 0.0, sw, sh, rgb(4, 16, 40), rgb(8, 38, 76), rgb(14, 64, 104));
                for i in 0..9 {
                    let fi = i as f32;
                    let cx = sw * ((0.08 + fi * 0.11) % 1.0);
                    let cy = sh * (0.5 + (fi * 0.7).sin() * 0.25);
                    let r = 40.0 + (t * 1.2 + fi * 1.1).sin() * 15.0;
                    let a = 0.12 + (t * 1.5 + fi * 0.9).sin() * 0.08;
                    draw_circle_lines(cx, cy, r, 1.2, Color::new(0.25, 0.6, 0.9, a));
                    draw_circle_lines(cx, cy, r * 0.6, 1.0, Color::new(0.25, 0.6, 0.9, a * 0.6));
                }
                // Light shafts from the surface.
                for i in 0..5 {
                    let fi = i as f32;
                    let x = sw * (0.1 + fi * 0.2) + (t * 0.3 + fi).sin() * 30.0;
                    draw_triangle(
                        vec2(x - 20.0, 0.0),
                        vec2(x + 20.0, 0.0),
                        vec2(x + 90.0, sh),
                        Color::new(0.5, 0.8, 1.0, 0.025),
                    );
                }
            }
            BgMode::Aurora => {
                gradient_v(0.0, 0.0, sw, sh, rgb(4, 8, 20), rgb(10, 22, 34));
                self.draw_stars(sw, sh * 0.7, t, 0.6);
                let bands: [(Color, f32, f32); 3] = [
                    (Color::new(0.2, 1.0, 0.6, 0.05), 0.28, 0.0),
                    (Color::new(0.3, 0.7, 1.0, 0.045), 0.36, 1.7),
                    (Color::new(0.7, 0.4, 1.0, 0.04), 0.22, 3.1),
                ];
                for (c, base, ph) in bands {
                    aurora_ribbon(sw, sh, t, c, base, ph);
                }
            }
            BgMode::Custom => {
                if let Some(tex) = &self.texture {
                    // Cover the window while keeping the aspect ratio.
                    let (tw, th) = (tex.width(), tex.height());
                    let s = (sw / tw).max(sh / th);
                    let (dw, dh) = (tw * s, th * s);
                    draw_texture_ex(
                        tex,
                        (sw - dw) / 2.0,
                        (sh - dh) / 2.0,
                        WHITE,
                        DrawTextureParams { dest_size: Some(vec2(dw, dh)), ..Default::default() },
                    );
                } else {
                    gradient_v3(0.0, 0.0, sw, sh, rgb(11, 11, 22), rgb(18, 15, 34), rgb(32, 20, 54));
                    crate::ui::theme::text_centered(
                        "Shift+G to choose a background image",
                        sw / 2.0,
                        sh / 2.0,
                        18.0,
                        crate::ui::theme::TEXT_MUTED,
                    );
                }
            }
        }
    }
}

/// Colours of a Sonic level's sky: the gradient, the far and near hills,
/// the clouds and how much the sun shows.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LevelSky {
    pub top: Color,
    pub mid: Color,
    pub low: Color,
    pub far: Color,
    pub near: Color,
    pub cloud: Color,
    pub sun: f32,
}

const fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::new(r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0)
}

impl LevelSky {
    /// A bright day over green hills.
    pub const DAY: LevelSky = LevelSky {
        top: rgb(46, 112, 220),
        mid: rgb(110, 172, 245),
        low: rgb(196, 228, 255),
        far: rgb(120, 180, 210),
        near: rgb(84, 168, 120),
        cloud: Color::new(1.0, 1.0, 1.0, 0.85),
        sun: 1.0,
    };
    /// Dusk: violet over a red glow.
    pub const DUSK: LevelSky = LevelSky {
        top: rgb(40, 30, 92),
        mid: rgb(122, 70, 150),
        low: rgb(240, 150, 120),
        far: rgb(112, 88, 150),
        near: rgb(70, 58, 112),
        cloud: Color::new(0.95, 0.75, 0.8, 0.6),
        sun: 0.0,
    };
    /// Pale sky over snowy hills.
    pub const SNOW: LevelSky = LevelSky {
        top: rgb(92, 140, 208),
        mid: rgb(170, 200, 235),
        low: rgb(232, 240, 250),
        far: rgb(196, 212, 234),
        near: rgb(236, 242, 250),
        cloud: Color::new(1.0, 1.0, 1.0, 0.9),
        sun: 0.6,
    };
    /// Tropical: deep blue sky over teal islands.
    pub const TROPIC: LevelSky = LevelSky {
        top: rgb(18, 128, 212),
        mid: rgb(88, 200, 236),
        low: rgb(212, 246, 240),
        far: rgb(88, 190, 192),
        near: rgb(56, 168, 136),
        cloud: Color::new(1.0, 1.0, 1.0, 0.9),
        sun: 1.0,
    };
    /// Storm: low grey clouds over dark peaks.
    pub const STORM: LevelSky = LevelSky {
        top: rgb(48, 54, 70),
        mid: rgb(88, 94, 110),
        low: rgb(140, 145, 156),
        far: rgb(84, 94, 110),
        near: rgb(62, 72, 84),
        cloud: Color::new(0.55, 0.58, 0.64, 0.9),
        sun: 0.0,
    };

    /// From `self` to `other` as `t` goes from 0 to 1.
    pub fn mix(&self, other: &LevelSky, t: f32) -> LevelSky {
        let t = t.clamp(0.0, 1.0);
        let m = |a: Color, b: Color| {
            Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, a.a + (b.a - a.a) * t)
        };
        LevelSky {
            top: m(self.top, other.top),
            mid: m(self.mid, other.mid),
            low: m(self.low, other.low),
            far: m(self.far, other.far),
            near: m(self.near, other.near),
            cloud: m(self.cloud, other.cloud),
            sun: self.sun + (other.sun - self.sun) * t,
        }
    }
}

/// The sky behind a Sonic level: gradient, sun, drifting clouds and two
/// rows of hills scrolling with the camera (`scroll`, world px).
pub fn level_sky(sw: f32, sh: f32, scroll: f32, sky: &LevelSky) {
    let t = get_time() as f32;
    gradient_v3(0.0, 0.0, sw, sh, sky.top, sky.mid, sky.low);
    if sky.sun > 0.0 {
        let a = sky.sun;
        radial(sw * 0.82, sh * 0.16, 160.0, Color::new(1.0, 0.98, 0.86, 0.35 * a), Color::new(1.0, 0.98, 0.86, 0.0));
        draw_circle(sw * 0.82, sh * 0.16, 34.0, Color::new(1.0, 0.99, 0.91, a));
    }
    // Clouds: puffs of circles drifting by.
    let span = sw + 400.0;
    for k in 0..7 {
        let fk = k as f32;
        let x = ((fk * 331.0 - scroll * 0.05 + t * 8.0) % span + span) % span - 200.0;
        let y = sh * (0.1 + (fk * 0.37).fract() * 0.3);
        let s = 0.7 + (fk * 0.61).fract() * 0.6;
        let c = sky.cloud;
        for (dx, dy, r) in [(-34.0, 6.0, 20.0), (-12.0, -6.0, 26.0), (14.0, -2.0, 24.0), (36.0, 8.0, 17.0)] {
            draw_circle(x + dx * s, y + dy * s, r * s, c);
        }
        draw_rectangle(x - 34.0 * s, y + 6.0 * s, 70.0 * s, 16.0 * s, c);
    }
    // Far hills (two rows), as triangle strips down to the bottom.
    for (k, (speed, base, height, col)) in
        [(0.08, 0.62, 90.0, sky.far), (0.18, 0.72, 70.0, sky.near)].into_iter().enumerate()
    {
        let off = scroll * speed;
        let step = 24.0;
        let n = (sw / step).ceil() as usize + 1;
        let mut vertices = Vec::with_capacity(n * 2 + 2);
        let mut indices = Vec::with_capacity(n * 6);
        for i in 0..=n {
            let x = i as f32 * step;
            let u = (x + off) / 260.0 + k as f32 * 1.7;
            let y = sh * base - height * (0.55 + 0.3 * u.sin() + 0.15 * (u * 2.3 + 1.0).sin());
            vertices.push(Vertex::new(x, y, 0.0, 0.0, 0.0, col));
            vertices.push(Vertex::new(x, sh, 0.0, 0.0, 0.0, col));
            if i > 0 {
                let (a, b) = ((i as u16 - 1) * 2, i as u16 * 2);
                indices.extend_from_slice(&[a, b, b + 1, a, b + 1, a + 1]);
            }
        }
        draw_mesh(&Mesh { vertices, indices, texture: None });
    }
}
