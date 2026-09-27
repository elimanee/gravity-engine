//! Bottom-left card for area tools: icon, description, radius and strength.

use super::theme::*;
use super::widgets::*;
use super::{icons, Action, Input};
use crate::config::{PPM, WALL_T};
use crate::settings::{Settings, RADIUS_RANGE, STRENGTH_RANGE};
use crate::util::approach;
use macroquad::prelude::*;

const W: f32 = 260.0;
const H: f32 = 132.0;

#[derive(Default)]
pub struct ToolCard {
    shown: f32,
    radius: SliderState,
    strength: SliderState,
}

impl ToolCard {
    fn rect(&self) -> Rect {
        let sh = screen_height();
        let x = 10.0 - (1.0 - self.shown) * (W + 20.0);
        Rect::new(x, sh - WALL_T * PPM - 10.0 - H, W, H)
    }

    fn rows(r: Rect) -> (Rect, Rect, Rect) {
        let head = Rect::new(r.x + 12.0, r.y + 10.0, r.w - 24.0, 36.0);
        let a = Rect::new(r.x + 14.0, r.y + 48.0, r.w - 28.0, SLIDER_ROW_H);
        let b = Rect::new(r.x + 14.0, r.y + 48.0 + SLIDER_ROW_H + 2.0, r.w - 28.0, SLIDER_ROW_H);
        (head, a, b)
    }

    pub fn update(&mut self, dt: f32, s: &mut Settings, input: &mut Input, actions: &mut Vec<Action>) {
        let want = s.tool.has_settings();
        self.shown = approach(self.shown, if want { 1.0 } else { 0.0 }, 12.0, dt);
        if self.shown < 0.5 {
            self.radius.dragging = false;
            self.strength.dragging = false;
            return;
        }
        let r = self.rect();
        let (head, a, b) = Self::rows(r);
        self.radius.update(a, &mut s.tool_radius, RADIUS_RANGE.0, RADIUS_RANGE.1, input);
        self.strength.update(b, &mut s.tool_strength, STRENGTH_RANGE.0, STRENGTH_RANGE.1, input);
        if button(head, input) {
            actions.push(Action::OpenToolPicker);
        }
        input.block(r);
    }

    pub fn dragging(&self) -> bool {
        self.radius.dragging || self.strength.dragging
    }

    pub fn draw(&self, s: &Settings, mouse: Vec2) {
        if self.shown < 0.01 {
            return;
        }
        let f = self.shown;
        let r = self.rect();
        panel(r, f);
        let (head, a, b) = Self::rows(r);
        let accent = s.tool.accent();
        let t = get_time() as f32;

        if head.contains(mouse) {
            rrect(Rect::new(head.x - 6.0, head.y - 2.0, head.w + 12.0, head.h + 4.0), 8.0, fade(SURFACE_HI, f * 0.6));
        }
        let ic = Rect::new(head.x, head.y, 36.0, 36.0);
        rrect(ic, 9.0, fade(alpha(accent, 0.16), f));
        icons::tool(s.tool, vec2(ic.x + 18.0, ic.y + 18.0), 26.0, fade(accent, f), t);
        text_bold(s.tool.label(), head.x + 46.0, head.y + 15.0, 15.0, fade(TEXT, f));
        text(s.tool.description(), head.x + 46.0, head.y + 31.0, 12.0, fade(TEXT_MUTED, f));
        let hint = "Tab";
        keycap(head.x + head.w - 30.0, head.y + 11.0, hint, 10.0, f * 0.8);

        let radius = SliderSpec {
            label: "Radius",
            value_text: format!("{} px", s.tool_radius as i32),
            min: RADIUS_RANGE.0,
            max: RADIUS_RANGE.1,
            accent,
        };
        let strength = SliderSpec {
            label: "Strength",
            value_text: format!("{:.0}", s.tool_strength),
            min: STRENGTH_RANGE.0,
            max: STRENGTH_RANGE.1,
            accent,
        };
        draw_slider(a, s.tool_radius, &radius, a.contains(mouse) || self.radius.dragging, f);
        draw_slider(b, s.tool_strength, &strength, b.contains(mouse) || self.strength.dragging, f);
    }
}
