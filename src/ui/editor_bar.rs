//! Bar shown in the challenge editor: the level's name, placing the ball
//! and the goal, the ink budget, and Test / Save.

use super::theme::*;
use super::widgets::*;
use super::{icons, Action, EditorCmd, Input};
use crate::physics::zones::ZoneKind;
use macroquad::prelude::*;

const W: f32 = 600.0;
const H: f32 = 98.0;
const MAX_NAME: usize = 40;

/// What the bar shows.
pub struct EditorView {
    pub ink: f32,
    pub placing_ball: bool,
    pub placing_goal: bool,
    pub has_ball: bool,
    pub has_goal: bool,
    /// The level was solved in a test since the last change.
    pub verified: bool,
}

#[derive(Default)]
pub struct EditorBar {
    /// The name field has the keyboard.
    focus: bool,
}

struct Layout {
    panel: Rect,
    name: Rect,
    test: Rect,
    save: Rect,
    exit: Rect,
    ball: Rect,
    goal: Rect,
    ink: Rect,
}

fn layout(top: f32) -> Layout {
    let panel = Rect::new((screen_width() - W) / 2.0, top.max(10.0) + 6.0, W, H);
    let (x, y) = (panel.x + 14.0, panel.y + 12.0);
    let exit = Rect::new(panel.x + panel.w - 44.0, y, 32.0, 26.0);
    let save = Rect::new(exit.x - 78.0, y, 70.0, 26.0);
    let test = Rect::new(save.x - 78.0, y, 70.0, 26.0);
    let name = Rect::new(x + 130.0, y, test.x - 10.0 - (x + 130.0), 26.0);
    let y2 = y + 34.0;
    let ball = Rect::new(x, y2, 96.0, 26.0);
    let goal = Rect::new(x + 102.0, y2, 96.0, 26.0);
    let ink = Rect::new(x + 214.0, y2, 200.0, 26.0);
    Layout { panel, name, test, save, exit, ball, goal, ink }
}

impl EditorBar {
    /// The name field is being typed in: keyboard shortcuts are off.
    pub fn typing(&self) -> bool {
        self.focus
    }

    pub fn update(&mut self, name: &mut String, top: f32, input: &mut Input, actions: &mut Vec<Action>) {
        let l = layout(top);
        if input.left_pressed {
            let was = self.focus;
            self.focus = l.name.contains(input.mouse);
            if self.focus && !was {
                // Forget keys typed before the field had the keyboard.
                while get_char_pressed().is_some() {}
            }
        }
        if self.focus {
            while let Some(c) = get_char_pressed() {
                if !c.is_control() && name.chars().count() < MAX_NAME {
                    name.push(c);
                }
            }
            if is_key_pressed(KeyCode::Backspace) {
                name.pop();
            }
            if is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Escape) {
                self.focus = false;
            }
        }
        let cmds = [
            (l.ball, EditorCmd::PlaceBall),
            (l.goal, EditorCmd::PlaceGoal),
            (l.test, EditorCmd::Test),
            (l.save, EditorCmd::Save),
            (l.exit, EditorCmd::Exit),
        ];
        for (r, cmd) in cmds {
            if button(r, input) {
                actions.push(Action::Editor(cmd));
            }
        }
        let d = stepper(l.ink, input);
        if d != 0 {
            actions.push(Action::Editor(EditorCmd::Ink(d)));
        }
        input.block(l.panel);
    }

    pub fn draw(&self, v: &EditorView, name: &str, top: f32, mouse: Vec2) {
        let l = layout(top);
        let p = l.panel;
        panel(p, 1.0);
        text_bold("CHALLENGE EDITOR", p.x + 14.0, p.y + 29.0, 11.0, ACCENT_HI);

        // Name field.
        let r = l.name;
        rrect(r, 7.0, if self.focus { SURFACE_HI } else { SURFACE_2 });
        rrect_lines(r, 7.0, 1.0, if self.focus { ACCENT } else { BORDER });
        let shown = if name.is_empty() && !self.focus { "Name your challenge…" } else { name };
        let col = if name.is_empty() { TEXT_MUTED } else { TEXT };
        text(shown, r.x + 10.0, r.y + 17.0, 13.0, col);
        if self.focus && (get_time() * 2.0) as i64 % 2 == 0 {
            let x = r.x + 11.0 + measure(name, 13.0);
            draw_line(x, r.y + 6.0, x, r.y + r.h - 6.0, 1.2, TEXT);
        }

        draw_button(l.test, "Test", l.test.contains(mouse), v.has_ball && v.has_goal && !v.verified, 1.0);
        draw_button(l.save, "Save", l.save.contains(mouse), v.verified, 1.0);
        draw_button(l.exit, "", l.exit.contains(mouse), false, 1.0);
        let (cx, cy) = (l.exit.x + l.exit.w / 2.0, l.exit.y + l.exit.h / 2.0);
        draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.6, TEXT);
        draw_line(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.6, TEXT);

        // Ball and goal buttons, lit while placing.
        let t = get_time() as f32;
        for (r, label, placing, done) in
            [(l.ball, "Ball", v.placing_ball, v.has_ball), (l.goal, "Goal", v.placing_goal, v.has_goal)]
        {
            let hov = r.contains(mouse);
            let bg = if placing {
                mix(SURFACE_2, ACCENT, 0.35)
            } else if hov {
                SURFACE_HI
            } else {
                SURFACE_2
            };
            rrect(r, 7.0, bg);
            rrect_lines(r, 7.0, 1.0, if placing { ACCENT_HI } else { BORDER });
            let c = vec2(r.x + 16.0, r.y + r.h / 2.0);
            if label == "Ball" {
                draw_circle(c.x, c.y, 7.0, Color::from_rgba(255, 196, 36, 255));
            } else {
                icons::zone_kind(ZoneKind::Goal, c, 16.0, SUCCESS, t);
            }
            text(label, r.x + 30.0, r.y + 17.0, 12.0, TEXT);
            if done {
                let (x, y) = (r.x + r.w - 18.0, r.y + r.h / 2.0);
                draw_line(x, y, x + 3.5, y + 3.5, 1.8, SUCCESS);
                draw_line(x + 3.5, y + 3.5, x + 10.0, y - 4.0, 1.8, SUCCESS);
            }
        }
        draw_stepper(l.ink, "Ink", &format!("{:.0} px", v.ink), Color::new(0.45, 0.75, 1.0, 1.0), mouse, 1.0);

        let (status, colour) = if v.placing_ball {
            ("Click where the ball starts", ACCENT_HI)
        } else if v.placing_goal {
            ("Drag a rectangle for the goal", ACCENT_HI)
        } else if !v.has_ball || !v.has_goal {
            ("Place the ball and the goal, build the level with any tool", TEXT_DIM)
        } else if v.verified {
            ("Solved in a test  ·  ready to Save", SUCCESS)
        } else {
            ("Test it: you must solve the level before saving it", TEXT_DIM)
        };
        text(status, p.x + 14.0, p.y + H - 14.0, 12.0, colour);
    }
}
