//! Object spawner panel (N). While it is open, clicking (or holding) in the
//! scene spawns the selected shape.

use super::theme::*;
use super::widgets::*;
use super::{icons, Fader, Input};
use crate::settings::{Settings, SPAWN_SIZE_RANGE};
use crate::shapes::{Shape, PALETTE};
use crate::util::hsv_to_rgb;
use macroquad::prelude::*;

const W: f32 = 286.0;
const H: f32 = 262.0;

#[derive(Default)]
pub struct Spawner {
    pub fader: Fader,
    size: SliderState,
}

struct Layout {
    panel: Rect,
    close: Rect,
    shapes: Vec<Rect>,
    swatches: Vec<Rect>,
    size: Rect,
}

impl Spawner {
    fn layout(&self) -> Layout {
        let f = self.fader.value();
        let sh = screen_height();
        let panel = Rect::new(10.0 - (1.0 - f) * (W + 20.0), ((sh - H) / 2.0).max(64.0), W, H);
        let pad = 14.0;
        let close = Rect::new(panel.x + panel.w - 34.0, panel.y + 10.0, 24.0, 24.0);
        let n = Shape::ALL.len();
        let bw = (panel.w - pad * 2.0 - (n as f32 - 1.0) * 4.0) / n as f32;
        let shapes = (0..n).map(|i| Rect::new(panel.x + pad + i as f32 * (bw + 4.0), panel.y + 44.0, bw, bw)).collect();
        let ns = PALETTE.len() + 1;
        let sw = (panel.w - pad * 2.0 - (ns as f32 - 1.0) * 5.0) / ns as f32;
        let swatches =
            (0..ns).map(|i| Rect::new(panel.x + pad + i as f32 * (sw + 5.0), panel.y + 132.0, sw, sw)).collect();
        let size = Rect::new(panel.x + pad, panel.y + 172.0, panel.w - pad * 2.0, SLIDER_ROW_H);
        Layout { panel, close, shapes, swatches, size }
    }

    pub fn is_open(&self) -> bool {
        self.fader.open
    }

    pub fn update(&mut self, dt: f32, s: &mut Settings, input: &mut Input) {
        self.fader.update(dt, 6.0);
        if !self.fader.visible() {
            self.size.dragging = false;
            return;
        }
        let l = self.layout();
        self.size.update(l.size, &mut s.spawn_size, SPAWN_SIZE_RANGE.0, SPAWN_SIZE_RANGE.1, input);
        if button(l.close, input) {
            self.fader.open = false;
        }
        for (i, r) in l.shapes.iter().enumerate() {
            if button(*r, input) {
                s.spawn_shape = Shape::ALL[i];
            }
        }
        for (i, r) in l.swatches.iter().enumerate() {
            if button(*r, input) {
                s.spawn_color = i;
            }
        }
        input.block(l.panel);
    }

    pub fn draw(&self, s: &Settings, mouse: Vec2) {
        if !self.fader.visible() {
            return;
        }
        let f = self.fader.value();
        let l = self.layout();
        panel(l.panel, f);
        let t = get_time() as f32;
        let preview = spawn_color(s.spawn_color, t);

        text_bold("Spawn objects", l.panel.x + 14.0, l.panel.y + 27.0, 16.0, fade(TEXT, f));
        let hov = l.close.contains(mouse);
        if hov {
            rrect(l.close, 6.0, fade(SURFACE_HI, f));
        }
        let (cx, cy) = (l.close.x + 12.0, l.close.y + 12.0);
        let c = fade(if hov { TEXT } else { TEXT_MUTED }, f);
        draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.6, c);
        draw_line(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.6, c);

        for (i, r) in l.shapes.iter().enumerate() {
            let shape = Shape::ALL[i];
            let active = shape == s.spawn_shape;
            let hov = r.contains(mouse);
            rrect(
                *r,
                8.0,
                fade(
                    if active {
                        mix(SURFACE_2, ACCENT, 0.25)
                    } else if hov {
                        SURFACE_HI
                    } else {
                        SURFACE_2
                    },
                    f,
                ),
            );
            if active {
                rrect_lines(*r, 8.0, 1.2, fade(ACCENT_HI, f));
            }
            let col = if active { preview } else { TEXT_DIM };
            icons::shape(shape, vec2(r.x + r.w / 2.0, r.y + r.h / 2.0), r.w * 0.6, fade(col, f));
        }
        let named = l.shapes.iter().position(|r| r.contains(mouse)).map_or(s.spawn_shape, |i| Shape::ALL[i]);
        let row_bottom = l.shapes.first().map_or(l.panel.y + 80.0, |r| r.y + r.h);
        text_centered(named.label(), l.panel.x + l.panel.w / 2.0, row_bottom + 14.0, 12.0, fade(TEXT_DIM, f));

        text("Colour", l.panel.x + 14.0, l.panel.y + 124.0, 12.0, fade(TEXT_DIM, f));
        for (i, r) in l.swatches.iter().enumerate() {
            let c = if i < PALETTE.len() {
                let (r, g, b) = PALETTE[i];
                Color::from_rgba(r, g, b, 255)
            } else {
                spawn_color(i, t)
            };
            rrect(*r, 6.0, fade(c, f));
            if i == PALETTE.len() {
                text_centered("?", r.x + r.w / 2.0, r.y + r.h / 2.0, 12.0, fade(Color::new(0.1, 0.1, 0.1, 0.8), f));
            }
            if i == s.spawn_color {
                rrect_lines(Rect::new(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0), 8.0, 2.0, fade(TEXT, f));
            }
        }

        let spec = SliderSpec {
            label: "Size",
            value_text: format!("{} px", s.spawn_size as i32),
            min: SPAWN_SIZE_RANGE.0,
            max: SPAWN_SIZE_RANGE.1,
            accent: ACCENT,
        };
        draw_slider(l.size, s.spawn_size, &spec, l.size.contains(mouse) || self.size.dragging, f);

        let hint = "Click or hold in the scene to spawn";
        text_centered(hint, l.panel.x + l.panel.w / 2.0, l.panel.y + l.panel.h - 20.0, 12.0, fade(TEXT_MUTED, f));
    }

    /// Ghost preview of the next spawn under the cursor.
    pub fn draw_ghost(&self, s: &Settings, mouse: Vec2) {
        if !self.fader.open {
            return;
        }
        let c = spawn_color(s.spawn_color, get_time() as f32);
        let size = s.spawn_size;
        icons::shape(s.spawn_shape, mouse, size, Color { a: 0.18, ..c });
    }
}

/// Colour for palette index `i` (the extra last index cycles through hues).
pub fn spawn_color(i: usize, t: f32) -> Color {
    if i < PALETTE.len() {
        let (r, g, b) = PALETTE[i];
        Color::from_rgba(r, g, b, 255)
    } else {
        let (r, g, b) = hsv_to_rgb(t * 0.25, 0.6, 0.98);
        Color::from_rgba(r, g, b, 255)
    }
}
