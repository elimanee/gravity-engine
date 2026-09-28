//! Library panel (E): example scenes and challenges.

use super::theme::*;
use super::widgets::*;
use super::{icons, Action, Fader, Input};
use crate::library::{challenges, examples};
use crate::physics::zones::ZoneKind;
use macroquad::prelude::*;

const COLS: usize = 4;
const CELL_W: f32 = 200.0;
const CELL_H: f32 = 86.0;
const GAP: f32 = 10.0;
const PAD: f32 = 18.0;
const HEAD: f32 = 76.0;

#[derive(Clone, Copy, PartialEq, Default)]
enum Tab {
    #[default]
    Examples,
    Challenges,
}

#[derive(Default)]
pub struct Library {
    pub fader: Fader,
    tab: Tab,
}

impl Library {
    fn count(&self) -> usize {
        match self.tab {
            Tab::Examples => examples::ALL.len(),
            Tab::Challenges => challenges::ALL.len(),
        }
    }

    fn panel(&self) -> Rect {
        let rows = examples::ALL.len().max(challenges::ALL.len()).div_ceil(COLS);
        let w = COLS as f32 * CELL_W + (COLS - 1) as f32 * GAP + PAD * 2.0;
        let h = rows as f32 * CELL_H + (rows - 1) as f32 * GAP + PAD * 2.0 + HEAD;
        Rect::new((screen_width() - w) / 2.0, (screen_height() - h) / 2.0, w, h)
    }

    fn tabs(p: Rect) -> [Rect; 2] {
        let y = p.y + 44.0;
        [Rect::new(p.x + PAD, y, 120.0, 26.0), Rect::new(p.x + PAD + 126.0, y, 120.0, 26.0)]
    }

    fn cell(p: Rect, i: usize) -> Rect {
        Rect::new(
            p.x + PAD + (i % COLS) as f32 * (CELL_W + GAP),
            p.y + PAD + HEAD + (i / COLS) as f32 * (CELL_H + GAP),
            CELL_W,
            CELL_H,
        )
    }

    pub fn open(&mut self, challenges: bool) {
        self.fader.open = true;
        self.tab = if challenges { Tab::Challenges } else { Tab::Examples };
    }

    pub fn update(&mut self, dt: f32, input: &mut Input, actions: &mut Vec<Action>) {
        self.fader.update(dt, 7.0);
        if !self.fader.open {
            return;
        }
        let p = self.panel();
        if input.left_pressed {
            let tabs = Self::tabs(p);
            if tabs[0].contains(input.mouse) {
                self.tab = Tab::Examples;
            } else if tabs[1].contains(input.mouse) {
                self.tab = Tab::Challenges;
            } else if let Some(i) = (0..self.count()).find(|&i| Self::cell(p, i).contains(input.mouse)) {
                actions.push(match self.tab {
                    Tab::Examples => Action::LoadExample(i),
                    Tab::Challenges => Action::StartChallenge(i),
                });
                self.fader.open = false;
            } else if !p.contains(input.mouse) {
                self.fader.open = false;
            }
        }
        // Modal: swallow all pointer input.
        input.over_ui = true;
        input.consumed |= input.left_pressed || input.right_pressed;
        input.wheel = 0.0;
    }

    pub fn draw(&self, done: &[String], mouse: Vec2) {
        if !self.fader.visible() {
            return;
        }
        let f = self.fader.value();
        draw_rectangle(0.0, 0.0, screen_width(), screen_height(), Color::new(0.0, 0.0, 0.02, 0.5 * f));
        let lift = (1.0 - f) * 16.0;
        let base = self.panel();
        let p = Rect::new(base.x, base.y + lift, base.w, base.h);
        panel(p, f);
        text_bold("Library", p.x + PAD, p.y + PAD + 12.0, 17.0, fade(TEXT, f));
        let solved = challenges::ALL.iter().filter(|c| done.iter().any(|d| d == c.id)).count();
        let hint = format!("{solved} / {} challenges solved · E or Esc to close", challenges::ALL.len());
        text(&hint, p.x + p.w - PAD - measure(&hint, 12.0), p.y + PAD + 12.0, 12.0, fade(TEXT_MUTED, f));

        let tabs = Self::tabs(p);
        for (i, (label, tab)) in [("Examples", Tab::Examples), ("Challenges", Tab::Challenges)].into_iter().enumerate()
        {
            draw_button(tabs[i], label, tabs[i].contains(mouse), self.tab == tab, f);
        }

        let t = get_time() as f32;
        for i in 0..self.count() {
            let r = Self::cell(p, i);
            let hov = r.contains(mouse);
            rrect(r, 10.0, fade(if hov { SURFACE_HI } else { SURFACE_2 }, f));
            rrect_lines(r, 10.0, 1.0, fade(if hov { BORDER_HI } else { BORDER }, f));
            match self.tab {
                Tab::Examples => {
                    let e = &examples::ALL[i];
                    text_bold(e.name, r.x + 12.0, r.y + 26.0, 14.0, fade(TEXT, f));
                    for (k, line) in wrap(e.about, r.w - 24.0, 11.0).iter().take(2).enumerate() {
                        text(line, r.x + 12.0, r.y + 46.0 + k as f32 * 14.0, 11.0, fade(TEXT_MUTED, f));
                    }
                    text("Open", r.x + r.w - 44.0, r.y + 26.0, 11.0, fade(if hov { ACCENT_HI } else { TEXT_DIM }, f));
                }
                Tab::Challenges => {
                    let c = &challenges::ALL[i];
                    let solved = done.iter().any(|d| d == c.id);
                    keycap(r.x + 12.0, r.y + 18.0, &(i + 1).to_string(), 10.0, f);
                    text_bold(c.name, r.x + 36.0, r.y + 23.0, 14.0, fade(TEXT, f));
                    for (k, line) in wrap(c.goal, r.w - 24.0, 11.0).iter().take(2).enumerate() {
                        text(line, r.x + 12.0, r.y + 46.0 + k as f32 * 14.0, 11.0, fade(TEXT_MUTED, f));
                    }
                    let (label, col) = if solved { ("Solved", SUCCESS) } else { ("Play", TEXT_DIM) };
                    if solved {
                        icons::zone_kind(ZoneKind::Goal, vec2(r.x + r.w - 58.0, r.y + 20.0), 18.0, fade(SUCCESS, f), t);
                    }
                    text(label, r.x + r.w - 44.0, r.y + 26.0, 11.0, fade(if hov { ACCENT_HI } else { col }, f));
                }
            }
        }
    }
}

/// Greedy word wrap to `width` px.
pub fn wrap(s: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines: Vec<String> = vec![];
    let mut cur = String::new();
    for word in s.split_whitespace() {
        let candidate = if cur.is_empty() { word.to_string() } else { format!("{cur} {word}") };
        if measure(&candidate, size) > width && !cur.is_empty() {
            lines.push(std::mem::replace(&mut cur, word.to_string()));
        } else {
            cur = candidate;
        }
    }
    if !cur.is_empty() {
        lines.push(cur);
    }
    lines
}
