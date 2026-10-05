//! Library panel (E): example scenes, challenges and the player's own
//! challenges.

use super::theme::*;
use super::widgets::*;
use super::{icons, Action, Fader, Input};
use crate::library::{challenges, examples};
use crate::physics::zones::ZoneKind;
use macroquad::prelude::*;
use std::collections::BTreeMap;

const COLS: usize = 4;
const CELL_W: f32 = 200.0;
const CELL_H: f32 = 96.0;
const GAP: f32 = 10.0;
const PAD: f32 = 18.0;
const HEAD: f32 = 76.0;
const ROWS: usize = 4;

/// Progress and the player's challenges, for drawing.
pub struct LibraryData<'a> {
    pub done: &'a [String],
    pub stars: &'a BTreeMap<String, u8>,
    /// (name, id, ink) of each of "My challenges".
    pub mine: &'a [(String, String, f32)],
}

#[derive(Clone, Copy, PartialEq, Default)]
enum Tab {
    #[default]
    Examples,
    Challenges,
    Mine,
}

#[derive(Default)]
pub struct Library {
    pub fader: Fader,
    tab: Tab,
    /// Rows scrolled past (mouse wheel).
    scroll: usize,
}

impl Library {
    /// Cells in the current tab.
    fn total(&self, mine: usize) -> usize {
        match self.tab {
            Tab::Examples => examples::ALL.len(),
            Tab::Challenges => challenges::ALL.len(),
            Tab::Mine => mine + 1,
        }
    }

    /// Cells shown (the rest is scrolled away).
    fn count(&self, mine: usize) -> usize {
        self.total(mine).saturating_sub(self.scroll * COLS).min(ROWS * COLS)
    }

    /// Rows that can be scrolled.
    fn max_scroll(&self, mine: usize) -> usize {
        self.total(mine).div_ceil(COLS).saturating_sub(ROWS)
    }

    fn panel(&self) -> Rect {
        let rows = ROWS;
        let w = COLS as f32 * CELL_W + (COLS - 1) as f32 * GAP + PAD * 2.0;
        let h = rows as f32 * CELL_H + (rows - 1) as f32 * GAP + PAD * 2.0 + HEAD;
        Rect::new((screen_width() - w) / 2.0, (screen_height() - h) / 2.0, w, h)
    }

    fn tabs(p: Rect) -> [Rect; 3] {
        let y = p.y + 44.0;
        [0.0, 1.0, 2.0].map(|k| Rect::new(p.x + PAD + k * 136.0, y, 130.0, 26.0))
    }

    /// The "Edit" button of a "My challenges" cell.
    fn edit_button(cell: Rect) -> Rect {
        Rect::new(cell.x + cell.w - 52.0, cell.y + cell.h - 28.0, 42.0, 20.0)
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
        self.scroll = 0;
    }

    pub fn update(&mut self, dt: f32, mine: usize, input: &mut Input, actions: &mut Vec<Action>) {
        self.fader.update(dt, 7.0);
        if !self.fader.open {
            return;
        }
        let p = self.panel();
        if input.wheel != 0.0 {
            let up = input.wheel > 0.0;
            self.scroll = if up { self.scroll.saturating_sub(1) } else { (self.scroll + 1).min(self.max_scroll(mine)) };
        }
        if input.left_pressed {
            let tabs = Self::tabs(p);
            let before = self.tab;
            if tabs[0].contains(input.mouse) {
                self.tab = Tab::Examples;
            } else if tabs[1].contains(input.mouse) {
                self.tab = Tab::Challenges;
            } else if tabs[2].contains(input.mouse) {
                self.tab = Tab::Mine;
            } else if let Some(slot) = (0..self.count(mine)).find(|&i| Self::cell(p, i).contains(input.mouse)) {
                let i = self.scroll * COLS + slot;
                let edit = Self::edit_button(Self::cell(p, slot)).contains(input.mouse);
                actions.push(match self.tab {
                    Tab::Examples => Action::LoadExample(i),
                    Tab::Challenges => Action::StartChallenge(i),
                    Tab::Mine if i == 0 => Action::OpenEditor(None),
                    Tab::Mine if edit => Action::OpenEditor(Some(i - 1)),
                    Tab::Mine => Action::StartCustom(i - 1),
                });
                self.fader.open = false;
            } else if !p.contains(input.mouse) {
                self.fader.open = false;
            }
            if self.tab != before {
                self.scroll = 0;
            }
        }
        // Modal: swallow all pointer input.
        input.over_ui = true;
        input.consumed |= input.left_pressed || input.right_pressed;
        input.wheel = 0.0;
    }

    pub fn draw(&self, data: &LibraryData, mouse: Vec2) {
        let done = data.done;
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
        let stars: u32 = challenges::ALL.iter().map(|c| *data.stars.get(c.id).unwrap_or(&0) as u32).sum();
        let n = challenges::ALL.len();
        let hint = format!("{solved} / {n} solved  ·  {stars} / {} stars  ·  E or Esc to close", n * 3);
        text(&hint, p.x + p.w - PAD - measure(&hint, 12.0), p.y + PAD + 12.0, 12.0, fade(TEXT_MUTED, f));

        let tabs = Self::tabs(p);
        let tab_list = [("Examples", Tab::Examples), ("Challenges", Tab::Challenges), ("My challenges", Tab::Mine)];
        for (i, (label, tab)) in tab_list.into_iter().enumerate() {
            draw_button(tabs[i], label, tabs[i].contains(mouse), self.tab == tab, f);
        }

        let t = get_time() as f32;
        let mine = data.mine.len();
        let more = self.max_scroll(mine);
        if more > 0 {
            let above = if self.scroll > 0 { "↑ " } else { "" };
            let below = if self.scroll < more { " ↓" } else { "" };
            let label = format!("{above}Scroll for more{below}");
            text_centered(&label, p.x + p.w / 2.0, p.y + p.h - 8.0, 11.0, fade(TEXT_MUTED, f));
        }
        for slot in 0..self.count(mine) {
            let i = self.scroll * COLS + slot;
            let r = Self::cell(p, slot);
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
                    text_bold(&fit(c.name, r.w - 36.0 - 58.0, 14.0), r.x + 36.0, r.y + 23.0, 14.0, fade(TEXT, f));
                    for (k, line) in wrap(c.goal, r.w - 24.0, 11.0).iter().take(2).enumerate() {
                        text(line, r.x + 12.0, r.y + 46.0 + k as f32 * 14.0, 11.0, fade(TEXT_MUTED, f));
                    }
                    let stars = data.stars.get(c.id).copied().unwrap_or(u8::from(solved));
                    draw_stars(r, stars, f);
                    let (label, col) = if solved { ("Solved", SUCCESS) } else { ("Play", TEXT_DIM) };
                    text(label, r.x + r.w - 44.0, r.y + 26.0, 11.0, fade(if hov { ACCENT_HI } else { col }, f));
                }
                Tab::Mine if i == 0 => {
                    icons::zone_kind(ZoneKind::Goal, vec2(r.x + 22.0, r.y + 22.0), 18.0, fade(ACCENT_HI, f), t);
                    text_bold("New challenge", r.x + 38.0, r.y + 27.0, 14.0, fade(TEXT, f));
                    let about = "Turn the current scene into a level: ball, goal, ink, test, save";
                    for (k, line) in wrap(about, r.w - 24.0, 11.0).iter().take(2).enumerate() {
                        text(line, r.x + 12.0, r.y + 46.0 + k as f32 * 14.0, 11.0, fade(TEXT_MUTED, f));
                    }
                }
                Tab::Mine => {
                    let (name, id, ink) = &data.mine[i - 1];
                    let title = fit(name, r.w - 12.0 - 58.0, 14.0);
                    text_bold(&title, r.x + 12.0, r.y + 26.0, 14.0, fade(TEXT, f));
                    text(&format!("{ink:.0} px of ink"), r.x + 12.0, r.y + 46.0, 11.0, fade(TEXT_MUTED, f));
                    draw_stars(r, data.stars.get(id).copied().unwrap_or(0), f);
                    let e = Self::edit_button(r);
                    draw_button(e, "Edit", e.contains(mouse), false, f);
                    text("Play", r.x + r.w - 44.0, r.y + 26.0, 11.0, fade(if hov { ACCENT_HI } else { TEXT_DIM }, f));
                }
            }
        }
    }
}

/// Up to three stars in the bottom-left corner of a cell (none: unsolved).
fn draw_stars(cell: Rect, stars: u8, f: f32) {
    if stars == 0 || f < 0.5 {
        return;
    }
    for k in 0..3 {
        icons::star(vec2(cell.x + 20.0 + k as f32 * 18.0, cell.y + cell.h - 16.0), 15.0, k < stars as usize);
    }
}

/// `s` shortened with an ellipsis to fit `width` px in bold.
fn fit(s: &str, width: f32, size: f32) -> String {
    if measure_bold(s, size) <= width {
        return s.to_string();
    }
    let mut out: String = s.to_string();
    while !out.is_empty() && measure_bold(&format!("{out}…"), size) > width {
        out.pop();
    }
    format!("{}…", out.trim_end())
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
