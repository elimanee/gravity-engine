//! Floating top bar: brand, gravity presets and status chips. It slides away
//! while you play and comes back when paused or when the pointer touches the
//! top edge.

use super::theme::*;
use super::{icons, Action, Input};
use crate::background::BgMode;
use crate::config::GRAVITY_PRESETS;
use crate::physics::borders::BorderMode;
use crate::physics::tools::Tool;
use crate::util::approach;
use macroquad::prelude::*;

pub const HUD_H: f32 = 44.0;
const MARGIN: f32 = 10.0;
const SEG_FONT: f32 = 12.0;
const CHIP_FONT: f32 = 12.0;

pub struct HudState<'a> {
    pub gravity: f32,
    pub tool: Tool,
    pub border: BorderMode,
    pub background: BgMode,
    pub objects: usize,
    pub paused: bool,
    pub time_scale: f32,
    pub busy: bool,
    pub title: &'a str,
}

#[derive(Default)]
pub struct Hud {
    shown: f32,
}

struct Layout {
    bar: Rect,
    brand: Option<Rect>,
    segments: Vec<Rect>,
    tool: Rect,
    border: Option<Rect>,
    background: Option<Rect>,
    count: Rect,
    paused: Option<Rect>,
}

fn chip_w(label: &str, icon: bool) -> f32 {
    chip_width(label, CHIP_FONT) + if icon { 18.0 } else { 0.0 }
}

impl Hud {
    /// Bottom edge of the bar in screen space (0 when hidden).
    pub fn bottom(&self) -> f32 {
        (MARGIN + HUD_H + 6.0) * self.shown
    }

    fn layout(&self, s: &HudState, sw: f32) -> Layout {
        let y = MARGIN - (HUD_H + MARGIN + 6.0) * (1.0 - self.shown);
        let bar = Rect::new(MARGIN, y, sw - MARGIN * 2.0, HUD_H);
        let cy = bar.y + bar.h / 2.0;
        let chip_h = CHIP_FONT * 1.9;

        let seg_ws: Vec<f32> = GRAVITY_PRESETS.iter().map(|(l, _)| measure(l, SEG_FONT) + 18.0).collect();
        let seg_total: f32 = seg_ws.iter().sum::<f32>() + 8.0;

        let tool_label = s.tool.label();
        let border_label = s.border.label();
        let bg_label = s.background.label();
        let count_label = format!("{} obj", s.objects);
        let brand_w = 34.0 + measure_bold(s.title, 14.0) + 12.0;

        // Chips are dropped (brand first, then world chips) as the window narrows.
        const GAP: f32 = 6.0;
        let core_w = chip_w(tool_label, true) + chip_w(&count_label, false) + GAP * 2.0;
        let world_w = chip_w(border_label, false) + chip_w(bg_label, false) + GAP * 2.0;
        let paused_w = if s.paused { chip_w("Paused", false) + GAP } else { 0.0 };
        let avail = bar.w - 20.0;
        let show_world = seg_total + core_w + world_w + paused_w + 50.0 < avail;
        let show_brand = show_world && brand_w + seg_total + core_w + world_w + paused_w + 30.0 < avail;
        let show_paused = s.paused && seg_total + core_w + paused_w + 50.0 < avail;

        // Right cluster, laid out right-to-left.
        let mut x = bar.x + bar.w - 10.0;
        let mut place = |w: f32| {
            x -= w;
            let r = Rect::new(x, cy - chip_h / 2.0, w, chip_h);
            x -= GAP;
            r
        };
        let paused = show_paused.then(|| place(chip_w("Paused", false)));
        let count = place(chip_w(&count_label, false));
        let background = show_world.then(|| place(chip_w(bg_label, false)));
        let border = show_world.then(|| place(chip_w(border_label, false)));
        let tool = place(chip_w(tool_label, true));
        let right_edge = x;

        let brand = show_brand.then(|| Rect::new(bar.x + 8.0, bar.y + 4.0, brand_w, bar.h - 8.0));
        let left_edge = brand.map_or(bar.x + 8.0, |b| b.x + b.w + 8.0);

        // Gravity presets centred in the remaining space.
        let mut segments = vec![];
        let seg_h = 28.0;
        let mut sx = ((left_edge + right_edge) / 2.0 - seg_total / 2.0).max(left_edge) + 4.0;
        for w in seg_ws {
            if sx + w > right_edge {
                break;
            }
            segments.push(Rect::new(sx, cy - seg_h / 2.0, w, seg_h));
            sx += w;
        }

        Layout { bar, brand, segments, tool, border, background, count, paused }
    }

    pub fn update(&mut self, dt: f32, s: &HudState, input: &mut Input, grabbing: bool, actions: &mut Vec<Action>) {
        let sw = screen_width();
        let over = input.mouse.y < MARGIN + HUD_H + 8.0 && self.shown > 0.5;
        let want = s.paused || (!grabbing && (input.mouse.y < 6.0 || over));
        self.shown = approach(self.shown, if want { 1.0 } else { 0.0 }, 14.0, dt);
        if self.shown < 0.3 {
            return;
        }

        let l = self.layout(s, sw);
        input.block(l.bar);
        let clicked = |r: Rect, input: &Input| input.left_pressed && r.contains(input.mouse);
        for (i, r) in l.segments.iter().enumerate() {
            if clicked(*r, input) {
                actions.push(Action::SetGravity(GRAVITY_PRESETS[i].1));
            }
        }
        if clicked(l.tool, input) {
            actions.push(Action::OpenToolPicker);
        }
        if let Some(r) = l.border {
            if clicked(r, input) {
                actions.push(Action::CycleBorder(if input.shift { -1 } else { 1 }));
            }
        }
        if let Some(r) = l.background {
            if clicked(r, input) {
                actions.push(Action::CycleBackground(if input.shift { -1 } else { 1 }));
            }
        }
        if let Some(r) = l.paused {
            if clicked(r, input) {
                actions.push(Action::TogglePause);
            }
        }
        if let Some(r) = l.brand {
            if clicked(r, input) {
                actions.push(Action::ToggleHelp);
            }
        }
    }

    pub fn draw(&self, s: &HudState, mouse: Vec2) {
        let sw = screen_width();
        // Peek strip hinting that the bar can be revealed.
        if self.shown < 0.99 {
            let a = (1.0 - self.shown) * 0.55;
            gradient_h(sw * 0.3, 0.0, sw * 0.2, 2.0, alpha(ACCENT, 0.0), alpha(ACCENT, a));
            gradient_h(sw * 0.5, 0.0, sw * 0.2, 2.0, alpha(ACCENT, a), alpha(ACCENT, 0.0));
        }
        if self.shown < 0.01 {
            return;
        }
        let f = self.shown;
        let l = self.layout(s, sw);
        panel(l.bar, f);
        let t = get_time() as f32;
        let cy = l.bar.y + l.bar.h / 2.0;

        if let Some(b) = l.brand {
            if b.contains(mouse) {
                rrect(b, 8.0, fade(SURFACE_HI, f * 0.6));
            }
            icons::logo(vec2(b.x + 16.0, cy), 0.23, t, f);
            text_bold(s.title, b.x + 34.0, baseline(cy, 14.0), 14.0, fade(TEXT, f));
        }

        // Gravity segmented control.
        if let (Some(first), Some(last)) = (l.segments.first(), l.segments.last()) {
            let track = Rect::new(first.x - 4.0, first.y - 3.0, last.x + last.w - first.x + 8.0, first.h + 6.0);
            rrect(track, 10.0, fade(SURFACE_2, f));
            let active = GRAVITY_PRESETS.iter().position(|(_, g)| (s.gravity - g).abs() < 0.05);
            for (i, r) in l.segments.iter().enumerate() {
                let is_active = active == Some(i);
                let hov = r.contains(mouse);
                if is_active {
                    rrect(*r, 8.0, fade(ACCENT, f));
                } else if hov {
                    rrect(*r, 8.0, fade(SURFACE_HI, f));
                }
                let c = if is_active || hov { TEXT } else { TEXT_DIM };
                text_centered(GRAVITY_PRESETS[i].0, r.x + r.w / 2.0, r.y + r.h / 2.0, SEG_FONT, fade(c, f));
            }
            if active.is_none() {
                let label = format!("g {:+.1}", s.gravity);
                let x = track.x + track.w + 8.0;
                if x + measure(&label, 12.0) < l.tool.x - 8.0 {
                    text(&label, x, baseline(cy, 12.0), 12.0, fade(WARNING, f));
                }
            }
        }

        let chip_bg = |r: Rect, accent: Color| {
            let hov = r.contains(mouse);
            rrect(r, r.h / 2.0, fade(if hov { SURFACE_HI } else { SURFACE_2 }, f));
            if hov {
                rrect_lines(r, r.h / 2.0, 1.0, fade(accent, f * 0.8));
            }
        };

        // Tool chip with icon.
        chip_bg(l.tool, s.tool.accent());
        icons::tool(s.tool, vec2(l.tool.x + 14.0, cy), 16.0, fade(s.tool.accent(), f), t);
        text(s.tool.label(), l.tool.x + 26.0, baseline(cy, CHIP_FONT), CHIP_FONT, fade(TEXT, f));

        if let Some(r) = l.border {
            chip_bg(r, s.border.accent());
            text_centered(s.border.label(), r.x + r.w / 2.0, cy, CHIP_FONT, fade(s.border.accent(), f));
        }
        if let Some(r) = l.background {
            chip_bg(r, ACCENT);
            text_centered(s.background.label(), r.x + r.w / 2.0, cy, CHIP_FONT, fade(TEXT_DIM, f));
        }
        let count = format!("{} obj", s.objects);
        rrect(l.count, l.count.h / 2.0, fade(SURFACE_2, f));
        text_centered(
            &count,
            l.count.x + l.count.w / 2.0,
            cy,
            CHIP_FONT,
            fade(if s.busy { WARNING } else { TEXT_DIM }, f),
        );
        if let Some(r) = l.paused {
            let pulse = 0.75 + (t * 3.0).sin() * 0.25;
            rrect(r, r.h / 2.0, fade(WARNING, f * 0.2));
            text_centered("Paused", r.x + r.w / 2.0, cy, CHIP_FONT, fade(WARNING, f * pulse));
        }
        if (s.time_scale - 1.0).abs() > 0.01 {
            let label = format!("×{:.2}", s.time_scale);
            let w = measure(&label, 11.0);
            text(&label, l.tool.x - w - 8.0, baseline(cy, 11.0), 11.0, fade(ACCENT_HI, f));
        }
    }
}
