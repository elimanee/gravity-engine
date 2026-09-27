//! Reusable controls. Each control is drawn and hit-tested from the same
//! rectangle, so layout code only has to compute rectangles once.

use super::theme::*;
use super::Input;
use macroquad::prelude::*;

/// Persistent drag state of a slider.
#[derive(Default)]
pub struct SliderState {
    pub dragging: bool,
}

pub struct SliderSpec<'a> {
    pub label: &'a str,
    pub value_text: String,
    pub min: f32,
    pub max: f32,
    pub accent: Color,
}

/// Height of a labelled slider row.
pub const SLIDER_ROW_H: f32 = 38.0;

fn track_of(row: Rect) -> Rect {
    Rect::new(row.x, row.y + row.h - 12.0, row.w, 6.0)
}

impl SliderState {
    /// Handle input for a slider row. Returns true when the pointer interaction
    /// belongs to this slider (so nothing underneath should react).
    pub fn update(&mut self, row: Rect, value: &mut f32, spec_min: f32, spec_max: f32, input: &mut Input) -> bool {
        let track = track_of(row);
        let hit = Rect::new(track.x - 8.0, row.y + row.h - 24.0, track.w + 16.0, 24.0);
        if input.left_pressed && hit.contains(input.mouse) && !input.consumed {
            self.dragging = true;
        }
        if !self.dragging {
            return false;
        }
        // Applied before the release check so a quick click still sets the value.
        let t = ((input.mouse.x - track.x) / track.w).clamp(0.0, 1.0);
        *value = spec_min + t * (spec_max - spec_min);
        input.consumed = true;
        if !input.left_down {
            self.dragging = false;
        }
        true
    }
}

pub fn draw_slider(row: Rect, value: f32, spec: &SliderSpec, hovered: bool, f: f32) {
    let track = track_of(row);
    let t = ((value - spec.min) / (spec.max - spec.min)).clamp(0.0, 1.0);
    text(spec.label, row.x, row.y + 14.0, 13.0, fade(TEXT_DIM, f));
    let vw = measure(&spec.value_text, 13.0);
    text(&spec.value_text, row.x + row.w - vw, row.y + 14.0, 13.0, fade(TEXT, f));

    rrect(track, 3.0, fade(SURFACE_HI, f));
    if t > 0.0 {
        rrect(Rect::new(track.x, track.y, (track.w * t).max(6.0), track.h), 3.0, fade(spec.accent, f));
    }
    let (kx, ky) = (track.x + track.w * t, track.y + track.h / 2.0);
    let kr = if hovered { 8.0 } else { 7.0 };
    draw_circle(kx, ky + 1.5, kr, fade(Color::new(0.0, 0.0, 0.0, 0.35), f));
    draw_circle(kx, ky, kr, fade(TEXT, f));
    draw_circle(kx, ky, kr - 3.0, fade(spec.accent, f));
}

/// Row with a label and an on/off switch. Returns true when clicked.
pub fn toggle_row(row: Rect, input: &mut Input) -> bool {
    button(row, input)
}

pub fn draw_toggle_row(row: Rect, label: &str, on: bool, hovered: bool, f: f32) {
    text_left(row, 0.0, label, 14.0, fade(if hovered { TEXT } else { TEXT_DIM }, f));
    let sw = Rect::new(row.x + row.w - 38.0, row.y + row.h / 2.0 - 10.0, 38.0, 20.0);
    rrect(sw, 10.0, fade(if on { ACCENT } else { SURFACE_HI }, f));
    let kx = if on { sw.x + sw.w - 10.0 } else { sw.x + 10.0 };
    draw_circle(kx, sw.y + 10.0, 7.5, fade(TEXT, f));
}

/// Clickable button. Returns true when clicked this frame.
pub fn button(r: Rect, input: &mut Input) -> bool {
    if input.left_pressed && !input.consumed && r.contains(input.mouse) {
        input.consumed = true;
        return true;
    }
    false
}

pub fn draw_button(r: Rect, label: &str, hovered: bool, active: bool, f: f32) {
    let bg = if active {
        fade(ACCENT, 0.9 * f)
    } else if hovered {
        fade(SURFACE_HI, f)
    } else {
        fade(SURFACE_2, f)
    };
    rrect(r, 7.0, bg);
    rrect_lines(r, 7.0, 1.0, fade(if hovered || active { BORDER_HI } else { BORDER }, f));
    let size = 13.0;
    text_centered(label, r.x + r.w / 2.0, r.y + r.h / 2.0, size, fade(TEXT, f));
}

/// `< value >` selector. Returns -1 / +1 when an arrow is clicked.
pub fn stepper(r: Rect, input: &mut Input) -> i32 {
    let left = Rect::new(r.x, r.y, 30.0, r.h);
    let right = Rect::new(r.x + r.w - 30.0, r.y, 30.0, r.h);
    if button(left, input) {
        return -1;
    }
    if button(right, input) || button(r, input) {
        return 1;
    }
    0
}

pub fn draw_stepper(r: Rect, label: &str, value: &str, accent: Color, mouse: Vec2, f: f32) {
    let hovered = r.contains(mouse);
    rrect(r, 7.0, fade(if hovered { SURFACE_HI } else { SURFACE_2 }, f));
    rrect_lines(r, 7.0, 1.0, fade(BORDER, f));
    let cy = r.y + r.h / 2.0;
    for (x, dir) in [(r.x + 15.0, -1.0f32), (r.x + r.w - 15.0, 1.0)] {
        let c = fade(TEXT_MUTED, f);
        draw_triangle(vec2(x + 4.0 * dir, cy), vec2(x - 3.0 * dir, cy - 5.0), vec2(x - 3.0 * dir, cy + 5.0), c);
    }
    let lw = measure(label, 12.0);
    let vw = measure(value, 14.0);
    let total = lw + 8.0 + vw;
    let x0 = r.x + (r.w - total) / 2.0;
    text(label, x0, baseline(cy, 12.0), 12.0, fade(TEXT_MUTED, f));
    text(value, x0 + lw + 8.0, baseline(cy, 14.0), 14.0, fade(accent, f));
}

/// Restrict drawing to `r` (screen coordinates); `None` removes the clip.
pub fn clip(r: Option<Rect>) {
    let dpi = screen_dpi_scale();
    let mut gl = unsafe { get_internal_gl() };
    gl.flush();
    gl.quad_gl.scissor(
        r.map(|r| ((r.x * dpi) as i32, (r.y * dpi) as i32, (r.w * dpi).ceil() as i32, (r.h * dpi).ceil() as i32)),
    );
}

/// Section heading inside a panel; returns the height used.
pub fn section(x: f32, y: f32, w: f32, title: &str, f: f32) -> f32 {
    text_bold(title, x, y + 14.0, 11.0, fade(TEXT_MUTED, f));
    let tw = measure_bold(title, 11.0);
    draw_line(x + tw + 8.0, y + 10.0, x + w, y + 10.0, 1.0, fade(BORDER, f * 0.8));
    22.0
}
