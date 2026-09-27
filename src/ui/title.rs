//! Title screen and the pixel-out transition into the sandbox.

use super::icons;
use super::theme::*;
use crate::config::APP_VERSION;
use macroquad::prelude::*;

/// Mosaic of blocks that vanish in random order, revealing the scene below.
pub struct PixelOut {
    order: Vec<usize>,
    colors: Vec<Color>,
    cleared: f32,
    cols: usize,
    block: f32,
}

const PALETTE: &[(u8, u8, u8)] = &[
    (10, 10, 20),
    (22, 14, 38),
    (28, 24, 48),
    (44, 36, 80),
    (70, 60, 130),
    (120, 100, 220),
    (160, 150, 240),
    (210, 205, 255),
];

impl PixelOut {
    pub fn new(sw: f32, sh: f32) -> Self {
        let block = 16.0;
        let cols = (sw / block).ceil() as usize + 1;
        let rows = (sh / block).ceil() as usize + 1;
        let n = cols * rows;
        let mut order: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            order.swap(i, rand::gen_range(0usize, i + 1));
        }
        // Mostly dark blocks with a sprinkle of accent colours.
        let colors = (0..n)
            .map(|_| {
                let k = rand::gen_range(0.0f32, 1.0);
                let idx = if k < 0.7 { rand::gen_range(0, 3) } else { rand::gen_range(3, PALETTE.len()) };
                let (r, g, b) = PALETTE[idx];
                Color::from_rgba(r, g, b, 255)
            })
            .collect();
        PixelOut { order, colors, cleared: 0.0, cols, block }
    }

    /// Advance; returns true when finished.
    pub fn update(&mut self, dt: f32) -> bool {
        let total = self.order.len() as f32;
        // Ease-in: starts slowly then accelerates.
        let progress = self.cleared / total;
        let speed = total / 0.5 * (0.4 + progress * 1.6);
        self.cleared = (self.cleared + dt * speed).min(total);
        self.cleared as usize >= self.order.len()
    }

    pub fn draw(&self) {
        for &idx in &self.order[self.cleared as usize..] {
            let (c, r) = (idx % self.cols, idx / self.cols);
            draw_rectangle(c as f32 * self.block, r as f32 * self.block, self.block, self.block, self.colors[idx]);
        }
    }
}

pub fn draw_title(sw: f32, sh: f32) {
    let t = get_time() as f32;
    gradient_v3(
        0.0,
        0.0,
        sw,
        sh,
        Color::from_rgba(6, 6, 14, 255),
        Color::from_rgba(14, 10, 30, 255),
        Color::from_rgba(28, 16, 50, 255),
    );
    // Parallax star field.
    for i in 0..160 {
        let fi = i as f32;
        let depth = (fi * 0.618).fract() * 0.8 + 0.2;
        let x = ((fi * 137.508).fract() * sw + t * 6.0 * depth) % sw;
        let y = (fi * 91.731).fract() * sh;
        let tw = 0.5 + 0.5 * (t * 1.5 + fi).sin();
        draw_circle(x, y, 0.5 + depth * 1.1, Color::new(0.8, 0.82, 1.0, (0.25 + 0.5 * tw) * depth));
    }

    let cx = sw / 2.0;
    let scale = (sh / 680.0).clamp(0.7, 1.6);
    icons::logo(vec2(cx, sh * 0.27), 1.5 * scale, t, 1.0);

    let title = "GRAVITY ENGINE";
    let size = 54.0 * scale;
    let ty = sh * 0.27 + 128.0 * scale;
    let tw = measure_bold(title, size);
    // Soft glow behind the wordmark.
    for i in 0..3 {
        let g = (i + 1) as f32 * 2.0;
        text_bold(title, cx - tw / 2.0 + g * 0.3, ty + g * 0.5, size, Color::new(0.5, 0.35, 1.0, 0.06));
    }
    text_bold(title, cx - tw / 2.0, ty, size, TEXT);
    gradient_h(cx - tw / 2.0, ty + 12.0 * scale, tw / 2.0, 2.0, alpha(ACCENT, 0.0), ACCENT);
    gradient_h(cx, ty + 12.0 * scale, tw / 2.0, 2.0, ACCENT, alpha(ACCENT, 0.0));

    let sub = "Drop anything. Watch it fall.";
    text_centered(sub, cx, ty + 38.0 * scale, 17.0 * scale, TEXT_DIM);
    let ver = format!("v{APP_VERSION} remaster");
    let vw = chip_width(&ver, 11.0);
    chip(cx - vw / 2.0, ty + 66.0 * scale, &ver, 11.0, ACCENT_HI, alpha(ACCENT, 0.18));

    // Quick-start keys.
    let tips: &[(&str, &str)] = &[("A", "add images"), ("N", "spawn shapes"), ("Tab", "tools"), ("F1", "all controls")];
    let ks = 12.0 * scale;
    let widths: Vec<f32> =
        tips.iter().map(|(k, d)| (measure(k, ks) + ks).max(ks * 1.7) + 8.0 + measure(d, 13.0 * scale)).collect();
    let gap = 26.0 * scale;
    let total: f32 = widths.iter().sum::<f32>() + gap * (tips.len() - 1) as f32;
    let mut x = cx - total / 2.0;
    let ky = sh * 0.8;
    for (i, (k, d)) in tips.iter().enumerate() {
        let kw = keycap(x, ky, k, ks, 1.0);
        text(d, x + kw + 8.0, baseline(ky, 13.0 * scale), 13.0 * scale, TEXT_MUTED);
        x += widths[i] + gap;
    }

    let blink = 0.55 + 0.45 * (t * 3.0).sin();
    text_centered("Press any key or click to start", cx, sh * 0.9, 15.0 * scale, alpha(ACCENT_HI, blink));
}
