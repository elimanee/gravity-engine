//! Bar shown in the Sonic level editor: the level's name, what to place
//! (start, rings, badniks, monitors, hazards, posts, the goal, the boss,
//! springs, air vents, pumpkins, ground, platforms, loops) or erase,
//! whether Sonic.exe hunts the player, Play / Stop and Save.

use super::theme::*;
use super::widgets::*;
use super::{Action, Input, LevelCmd};
use crate::game::{BossKind, EnemyKind, MonitorKind};
use macroquad::prelude::*;

const W: f32 = 820.0;
const H: f32 = 104.0;
const MAX_NAME: usize = 40;

/// What can be placed (or erased) by clicking in the world.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Item {
    Start,
    Ring,
    Badnik,
    Monitor,
    Spikes,
    Post,
    Goal,
    Boss,
    Spring,
    Vent,
    Pumpkin,
    /// Not placed: switches Sonic.exe's hunt on or off.
    Hunter,
    Ground,
    Platform,
    Loop,
    Erase,
}

impl Item {
    pub const ALL: [Item; 16] = [
        Item::Start,
        Item::Ring,
        Item::Badnik,
        Item::Monitor,
        Item::Spikes,
        Item::Post,
        Item::Goal,
        Item::Boss,
        Item::Spring,
        Item::Vent,
        Item::Pumpkin,
        Item::Hunter,
        Item::Ground,
        Item::Platform,
        Item::Loop,
        Item::Erase,
    ];

    pub fn label(self) -> &'static str {
        match self {
            Item::Start => "Start",
            Item::Ring => "Rings",
            Item::Badnik => "Badnik",
            Item::Monitor => "Monitor",
            Item::Spikes => "Spikes",
            Item::Post => "Post",
            Item::Goal => "Goal",
            Item::Boss => "Boss",
            Item::Spring => "Spring",
            Item::Vent => "Vent",
            Item::Pumpkin => "Pumpkin",
            Item::Hunter => "Exe",
            Item::Ground => "Ground",
            Item::Platform => "Ledge",
            Item::Loop => "Loop",
            Item::Erase => "Erase",
        }
    }

    /// What to do with it, for the status line.
    pub fn hint(self) -> &'static str {
        match self {
            Item::Start => "Click where the character starts (dropped onto the ground)",
            Item::Ring => "Click for a ring  ·  drag for a row of them",
            Item::Badnik => {
                "Click to place (crawlers and crabs land on the ground)  ·  click Badnik again for another kind"
            }
            Item::Monitor => "Click above the ground  ·  click Monitor again to change its item",
            Item::Spikes => "Click above the ground for a block of spikes",
            Item::Post => "Click above the ground for a star post (checkpoint)",
            Item::Goal => "Click above the ground for the goal signpost",
            Item::Boss => "Click where the boss flies (it guards an arena)  ·  click Boss again: bomber or crane",
            Item::Spring => "Click above the ground for a spring",
            Item::Vent => "Click above the sea floor for an air vent (big bubbles to breathe under water)",
            Item::Pumpkin => "Click above the ground for a jack-o'-lantern (it lights the night)",
            Item::Hunter => "Sonic.exe hunts the player through the level (click again to stop him)",
            Item::Ground => "Drag across: solid ground from there down",
            Item::Platform => "Drag across for a ledge",
            Item::Loop => "Click above the ground for a loop",
            Item::Erase => "Click on something to take it away",
        }
    }
}

/// What the bar shows.
pub struct LevelView {
    pub item: Option<Item>,
    pub monitor: MonitorKind,
    pub badnik: EnemyKind,
    pub boss: BossKind,
    pub hunter: bool,
    pub testing: bool,
    pub has_start: bool,
    pub has_end: bool,
    pub saved: bool,
}

#[derive(Default)]
pub struct LevelBar {
    /// The name field has the keyboard.
    focus: bool,
}

struct Layout {
    panel: Rect,
    name: Rect,
    play: Rect,
    save: Rect,
    exit: Rect,
    items: [Rect; Item::ALL.len()],
}

fn layout(top: f32) -> Layout {
    let panel = Rect::new((screen_width() - W) / 2.0, top.max(10.0) + 6.0, W, H);
    let (x, y) = (panel.x + 14.0, panel.y + 10.0);
    let exit = Rect::new(panel.x + panel.w - 44.0, y, 32.0, 26.0);
    let save = Rect::new(exit.x - 78.0, y, 70.0, 26.0);
    let play = Rect::new(save.x - 78.0, y, 70.0, 26.0);
    let name = Rect::new(x + 150.0, y, play.x - 10.0 - (x + 150.0), 26.0);
    let n = Item::ALL.len() as f32;
    let w = (W - 28.0 - 3.0 * (n - 1.0)) / n;
    let items = std::array::from_fn(|k| Rect::new(x + k as f32 * (w + 3.0), y + 34.0, w, 26.0));
    Layout { panel, name, play, save, exit, items }
}

impl LevelBar {
    /// The name field is being typed in: keyboard shortcuts are off.
    pub fn typing(&self) -> bool {
        self.focus
    }

    pub fn update(&mut self, v: &LevelView, name: &mut String, top: f32, input: &mut Input, actions: &mut Vec<Action>) {
        let l = layout(top);
        if input.left_pressed && !v.testing {
            let was = self.focus;
            self.focus = l.name.contains(input.mouse);
            if self.focus && !was {
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
        if button(l.play, input) {
            actions.push(Action::Level(if v.testing { LevelCmd::Stop } else { LevelCmd::Play }));
        }
        if !v.testing {
            if button(l.save, input) {
                actions.push(Action::Level(LevelCmd::Save));
            }
            if button(l.exit, input) {
                actions.push(Action::Level(LevelCmd::Exit));
            }
            for (k, r) in l.items.iter().enumerate() {
                if button(*r, input) {
                    actions.push(Action::Level(LevelCmd::Pick(Item::ALL[k])));
                }
            }
        }
        input.block(if v.testing { Rect::new(l.play.x, l.play.y, l.play.w, l.play.h) } else { l.panel });
    }

    pub fn draw(&self, v: &LevelView, name: &str, top: f32, mouse: Vec2) {
        let l = layout(top);
        if v.testing {
            // Only Stop while playing it.
            draw_button(l.play, "Stop", l.play.contains(mouse), true, 1.0);
            return;
        }
        let p = l.panel;
        panel(p, 1.0);
        text_bold("SONIC LEVEL EDITOR", p.x + 14.0, p.y + 27.0, 11.0, ACCENT_HI);

        let r = l.name;
        rrect(r, 7.0, if self.focus { SURFACE_HI } else { SURFACE_2 });
        rrect_lines(r, 7.0, 1.0, if self.focus { ACCENT } else { BORDER });
        let shown = if name.is_empty() && !self.focus { "Name your level…" } else { name };
        text(shown, r.x + 10.0, r.y + 17.0, 13.0, if name.is_empty() { TEXT_MUTED } else { TEXT });
        if self.focus && (get_time() * 2.0) as i64 % 2 == 0 {
            let x = r.x + 11.0 + measure(name, 13.0);
            draw_line(x, r.y + 6.0, x, r.y + r.h - 6.0, 1.2, TEXT);
        }
        draw_button(l.play, "Play", l.play.contains(mouse), v.has_start && v.has_end && !v.saved, 1.0);
        draw_button(l.save, "Save", l.save.contains(mouse), false, 1.0);
        draw_button(l.exit, "", l.exit.contains(mouse), false, 1.0);
        let (cx, cy) = (l.exit.x + l.exit.w / 2.0, l.exit.y + l.exit.h / 2.0);
        draw_line(cx - 5.0, cy - 5.0, cx + 5.0, cy + 5.0, 1.6, TEXT);
        draw_line(cx + 5.0, cy - 5.0, cx - 5.0, cy + 5.0, 1.6, TEXT);

        for (k, r) in l.items.iter().enumerate() {
            let item = Item::ALL[k];
            let on = v.item == Some(item) || (item == Item::Hunter && v.hunter);
            let hov = r.contains(mouse);
            let bg = if on {
                mix(SURFACE_2, ACCENT, 0.35)
            } else if hov {
                SURFACE_HI
            } else {
                SURFACE_2
            };
            rrect(*r, 6.0, bg);
            rrect_lines(*r, 6.0, 1.0, if on { ACCENT_HI } else { BORDER });
            let label = match item {
                Item::Monitor if on => short(v.monitor),
                Item::Badnik if on => v.badnik.label(),
                Item::Boss if on => match v.boss {
                    BossKind::Bomber => "Bomber",
                    BossKind::Anchor => "Crane",
                },
                _ => item.label(),
            };
            let w = measure(label, 10.5);
            text(label, r.x + (r.w - w) / 2.0, r.y + 17.0, 10.5, if item == Item::Erase { DANGER } else { TEXT });
        }

        let (status, colour) = match v.item {
            Some(item) => (item.hint().to_string(), ACCENT_HI),
            None if !v.has_start => ("Place the start  ·  build with the items or any tool".to_string(), TEXT_DIM),
            None if !v.has_end => ("Place the goal (or a boss) so the level can end".to_string(), TEXT_DIM),
            None => ("Play to try it  ·  Save keeps it in Library → Sonic".to_string(), TEXT_DIM),
        };
        text(&status, p.x + 14.0, p.y + H - 12.0, 12.0, colour);
    }
}

fn short(k: MonitorKind) -> &'static str {
    match k {
        MonitorKind::Rings => "10 rings",
        MonitorKind::Shield => "Shield",
        MonitorKind::Invincible => "Stars",
        MonitorKind::Shoes => "Shoes",
        MonitorKind::Life => "1-up",
    }
}
