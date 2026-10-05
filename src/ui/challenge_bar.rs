//! Bar shown during a challenge: level, goal, ink left and the Go / Retry /
//! Next / Exit buttons.

use super::theme::*;
use super::widgets::*;
use super::{Action, Input};
use macroquad::prelude::*;

const W: f32 = 520.0;
const H: f32 = 76.0;

/// What the bar shows.
pub struct ChallengeView<'a> {
    /// "CHALLENGE 3 / 15", "MY CHALLENGE", "TESTING"…
    pub head: String,
    pub name: &'a str,
    pub goal: &'a str,
    /// Ink left, 0‥1.
    pub ink: f32,
    pub started: bool,
    pub won: bool,
    pub failed: bool,
    pub has_next: bool,
    /// Stars earned, once won.
    pub stars: u8,
    /// Testing a level from the editor: the exit button goes back to it.
    pub testing: bool,
    /// A laser level: no ball to release, the beam is always on.
    pub light: bool,
}

#[derive(Default)]
pub struct ChallengeBar;

struct Layout {
    panel: Rect,
    main: Rect,
    exit: Rect,
    ink: Rect,
}

fn layout(top: f32) -> Layout {
    let panel = Rect::new((screen_width() - W) / 2.0, top.max(10.0) + 6.0, W, H);
    let exit = Rect::new(panel.x + panel.w - 44.0, panel.y + 12.0, 32.0, 26.0);
    let main = Rect::new(exit.x - 118.0, panel.y + 12.0, 110.0, 26.0);
    let ink = Rect::new(panel.x + 14.0, panel.y + H - 18.0, panel.w - 28.0, 6.0);
    Layout { panel, main, exit, ink }
}

impl ChallengeBar {
    pub fn update(&self, v: &ChallengeView, top: f32, input: &mut Input, actions: &mut Vec<Action>) {
        let l = layout(top);
        if button(l.main, input) {
            actions.push(if v.won && v.has_next {
                Action::ChallengeNext
            } else if v.won || v.started || v.light {
                Action::ChallengeRetry
            } else {
                Action::ChallengeGo
            });
        }
        if button(l.exit, input) {
            actions.push(Action::ChallengeExit);
        }
        input.block(l.panel);
    }

    pub fn draw(&self, v: &ChallengeView, top: f32, mouse: Vec2) {
        let l = layout(top);
        let p = l.panel;
        panel(p, 1.0);
        let head = &v.head;
        text_bold(head, p.x + 14.0, p.y + 22.0, 11.0, if v.testing { WARNING } else { TEXT_MUTED });
        let name_x = p.x + 14.0 + measure_bold(head, 11.0) + 10.0;
        let name = crate::util::ellipsize(v.name, 26);
        text_bold(&name, name_x, p.y + 23.0, 15.0, TEXT);
        let status = if v.won && v.testing {
            "Solved!  ·  × goes back to the editor".to_string()
        } else if v.won {
            let more = if v.stars < 3 { "  ·  less ink earns more stars" } else { "" };
            format!("Solved!{more}")
        } else if v.failed {
            "Missed — Retry (R)".to_string()
        } else if v.started {
            "Watch it go…".to_string()
        } else {
            let hint = if v.light { "lines are mirrors" } else { "drawings stay put (Shift: loose)" };
            let full = format!("{}  ·  {hint}", v.goal);
            if measure(&full, 12.0) < l.main.x - p.x - 24.0 {
                full
            } else {
                v.goal.to_string()
            }
        };
        let colour = if v.won {
            SUCCESS
        } else if v.failed {
            WARNING
        } else {
            TEXT_DIM
        };
        text(&status, p.x + 14.0, p.y + 44.0, 12.0, colour);
        if v.won {
            let x = l.main.x - 72.0;
            for k in 0..3 {
                let c = vec2(x + k as f32 * 22.0, l.main.y + l.main.h / 2.0);
                super::icons::star(c, 18.0, k < v.stars as usize);
            }
        }

        let (label, active) = if v.won && v.has_next {
            ("Next  (Enter)", true)
        } else if v.won || v.started || v.light {
            ("Retry  (R)", false)
        } else {
            ("Go  (Space)", true)
        };
        draw_button(l.main, label, l.main.contains(mouse), active, 1.0);
        draw_button(l.exit, "", l.exit.contains(mouse), false, 1.0);
        let (cx, cy) = (l.exit.x + l.exit.w / 2.0, l.exit.y + l.exit.h / 2.0);
        draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.6, TEXT);
        draw_line(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.6, TEXT);

        rrect(l.ink, 3.0, SURFACE_HI);
        let ink_colour = if v.ink < 0.15 { DANGER } else { Color::new(0.45, 0.75, 1.0, 1.0) };
        if v.ink > 0.0 {
            rrect(Rect::new(l.ink.x, l.ink.y, (l.ink.w * v.ink).max(6.0), l.ink.h), 3.0, ink_colour);
        }
        text("Ink", l.ink.x + l.ink.w - measure("Ink", 10.0), l.ink.y - 4.0, 10.0, TEXT_MUTED);
    }
}
