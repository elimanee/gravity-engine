//! Modal grid for choosing the interaction tool (Tab, or keys 1–8).

use super::theme::*;
use super::{icons, Action, Fader, Input};
use crate::physics::tools::Tool;
use macroquad::prelude::*;

const COLS: usize = 4;
const CELL_W: f32 = 172.0;
const CELL_H: f32 = 118.0;
const GAP: f32 = 10.0;
const PAD: f32 = 18.0;
const HEAD: f32 = 34.0;

#[derive(Default)]
pub struct ToolPicker {
    pub fader: Fader,
}

impl ToolPicker {
    fn panel() -> Rect {
        let rows = Tool::ALL.len().div_ceil(COLS);
        let w = COLS as f32 * CELL_W + (COLS - 1) as f32 * GAP + PAD * 2.0;
        let h = rows as f32 * CELL_H + (rows - 1) as f32 * GAP + PAD * 2.0 + HEAD;
        Rect::new((screen_width() - w) / 2.0, (screen_height() - h) / 2.0, w, h)
    }

    fn cell(i: usize, lift: f32) -> Rect {
        let p = Self::panel();
        Rect::new(
            p.x + PAD + (i % COLS) as f32 * (CELL_W + GAP),
            p.y + PAD + HEAD + (i / COLS) as f32 * (CELL_H + GAP) + lift,
            CELL_W,
            CELL_H,
        )
    }

    pub fn update(&mut self, dt: f32, input: &mut Input, actions: &mut Vec<Action>) {
        self.fader.update(dt, 7.0);
        if !self.fader.open {
            return;
        }
        if input.left_pressed {
            for (i, &tool) in Tool::ALL.iter().enumerate() {
                if Self::cell(i, 0.0).contains(input.mouse) {
                    actions.push(Action::SetTool(tool));
                }
            }
            if !Self::panel().contains(input.mouse) || !actions.is_empty() {
                self.fader.open = false;
            }
        }
        // Modal: swallow all pointer input.
        input.over_ui = true;
        input.consumed |= input.left_pressed || input.right_pressed;
        input.wheel = 0.0;
    }

    pub fn draw(&self, current: Tool, mouse: Vec2) {
        if !self.fader.visible() {
            return;
        }
        let f = self.fader.value();
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.02, 0.45 * f));
        let lift = (1.0 - f) * 16.0;
        let p = Self::panel();
        let p = Rect::new(p.x, p.y + lift, p.w, p.h);
        panel(p, f);
        text_bold("Choose a tool", p.x + PAD, p.y + PAD + 12.0, 17.0, fade(TEXT, f));
        let hint = "1–8 to pick · Tab to close";
        text(hint, p.x + p.w - PAD - measure(hint, 12.0), p.y + PAD + 12.0, 12.0, fade(TEXT_MUTED, f));

        let t = get_time() as f32;
        for (i, &tool) in Tool::ALL.iter().enumerate() {
            let r = Self::cell(i, lift);
            let cur = tool == current;
            let hov = r.contains(mouse);
            let accent = tool.accent();
            let bg = if cur {
                mix(SURFACE_2, accent, 0.18)
            } else if hov {
                SURFACE_HI
            } else {
                SURFACE_2
            };
            rrect(r, 10.0, fade(bg, f));
            rrect_lines(
                r,
                10.0,
                if cur { 1.5 } else { 1.0 },
                fade(
                    if cur {
                        accent
                    } else if hov {
                        BORDER_HI
                    } else {
                        BORDER
                    },
                    f,
                ),
            );

            let icon_t = if hov || cur { t } else { 0.0 };
            icons::tool(tool, vec2(r.x + r.w / 2.0, r.y + 36.0), 40.0, fade(accent, f), icon_t);
            text_bold_centered(tool.label(), r.x + r.w / 2.0, r.y + 76.0, 15.0, fade(TEXT, f));
            text_centered(tool.description(), r.x + r.w / 2.0, r.y + 97.0, 11.0, fade(TEXT_MUTED, f));
            keycap(r.x + 8.0, r.y + 16.0, &(i + 1).to_string(), 10.0, f * 0.9);
        }
    }
}
