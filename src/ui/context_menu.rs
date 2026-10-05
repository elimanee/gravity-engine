//! Right-click menu on an object.

use super::theme::*;
use super::{Action, Fader, Input, ObjectCmd};
use crate::util::ellipsize;
use macroquad::prelude::*;
use rapier2d::prelude::RigidBodyHandle;

const W: f32 = 196.0;
const ITEM_H: f32 = 30.0;
const HEAD_H: f32 = 30.0;
const PAD: f32 = 6.0;
const FONT: f32 = 13.0;

const SCALES: &[(&str, f32)] = &[("50 %", 0.5), ("75 %", 0.75), ("125 %", 1.25), ("150 %", 1.5), ("200 %", 2.0)];

#[derive(Clone, Copy, PartialEq)]
enum Item {
    Resize,
    Duplicate,
    SizeAll,
    Properties,
    Pin,
    Jelly,
    Flag,
    Planet,
    Unlink,
    Delete,
}

impl Item {
    fn label(self, pinned: bool, planet: bool) -> &'static str {
        match self {
            Item::Resize => "Resize",
            Item::Duplicate => "Duplicate",
            Item::SizeAll => "Resize all",
            Item::Properties => "Properties…   (I)",
            Item::Unlink => "Detach links",
            Item::Pin if pinned => "Unpin",
            Item::Pin => "Pin in place",
            Item::Jelly => "Make it jelly   (U)",
            Item::Flag => "Hang it as a flag",
            Item::Planet if planet => "No longer a planet",
            Item::Planet => "Make it a planet",
            Item::Delete => "Delete",
        }
    }
    fn has_sub(self) -> bool {
        matches!(self, Item::Resize | Item::SizeAll)
    }
}

pub struct ContextMenu {
    pub fader: Fader,
    pos: Vec2,
    pub target: Option<RigidBodyHandle>,
    title: String,
    pinned: bool,
    planet: bool,
    items: Vec<Item>,
    sub: Option<usize>,
}

impl Default for ContextMenu {
    fn default() -> Self {
        ContextMenu {
            fader: Fader::default(),
            pos: Vec2::ZERO,
            target: None,
            title: String::new(),
            pinned: false,
            planet: false,
            items: vec![],
            sub: None,
        }
    }
}

impl ContextMenu {
    pub fn open(&mut self, at: Vec2, target: RigidBodyHandle, title: String, flags: (bool, bool, bool)) {
        let (pinned, linked, planet) = flags;
        self.items = vec![
            Item::Resize,
            Item::Duplicate,
            Item::SizeAll,
            Item::Properties,
            Item::Pin,
            Item::Jelly,
            Item::Flag,
            Item::Planet,
        ];
        if linked {
            self.items.push(Item::Unlink);
        }
        self.items.push(Item::Delete);
        let h = self.height();
        self.pos =
            vec2(at.x.min(screen_width() - W * 2.0 - 8.0).max(4.0), at.y.min(screen_height() - h - 4.0).max(4.0));
        self.target = Some(target);
        self.title = ellipsize(&title, 24);
        self.pinned = pinned;
        self.planet = planet;
        self.sub = None;
        self.fader.open = true;
    }

    pub fn close(&mut self) {
        self.fader.open = false;
        self.target = None;
        self.sub = None;
    }

    fn height(&self) -> f32 {
        HEAD_H + self.items.len() as f32 * ITEM_H + PAD * 2.0 + 9.0
    }

    fn panel(&self) -> Rect {
        Rect::new(self.pos.x, self.pos.y, W, self.height())
    }

    fn item_rect(&self, i: usize) -> Rect {
        let sep = if self.items[i] == Item::Delete { 9.0 } else { 0.0 };
        Rect::new(self.pos.x + PAD, self.pos.y + HEAD_H + PAD + i as f32 * ITEM_H + sep, W - PAD * 2.0, ITEM_H)
    }

    fn sub_panel(&self, i: usize) -> Rect {
        let h = SCALES.len() as f32 * ITEM_H + PAD * 2.0;
        let y = (self.item_rect(i).y - PAD).min(screen_height() - h - 4.0);
        Rect::new(self.pos.x + W + 4.0, y, 120.0, h)
    }

    fn sub_rect(&self, i: usize, j: usize) -> Rect {
        let p = self.sub_panel(i);
        Rect::new(p.x + PAD, p.y + PAD + j as f32 * ITEM_H, p.w - PAD * 2.0, ITEM_H)
    }

    pub fn update(&mut self, dt: f32, input: &mut Input, actions: &mut Vec<Action>) {
        self.fader.update(dt, 10.0);
        let Some(target) = self.target.filter(|_| self.fader.open) else { return };
        let m = input.mouse;

        // Hover opens submenus; the submenu stays while the pointer is on it.
        let over_sub = self.sub.is_some_and(|i| {
            let sp = self.sub_panel(i);
            let bridge = Rect::new(self.pos.x + W - 4.0, self.item_rect(i).y, 12.0, ITEM_H);
            sp.contains(m) || bridge.contains(m)
        });
        if !over_sub {
            if let Some(i) = (0..self.items.len()).find(|&i| self.item_rect(i).contains(m)) {
                self.sub = self.items[i].has_sub().then_some(i);
            } else if self.panel().contains(m) {
                self.sub = None;
            }
        }

        let in_menu = self.panel().contains(m) || self.sub.is_some_and(|i| self.sub_panel(i).contains(m));
        if in_menu {
            input.over_ui = true;
            input.wheel = 0.0;
        }
        if input.left_pressed || input.right_pressed {
            input.consumed = true;
            if input.left_pressed {
                if let Some(i) = self.sub {
                    for (j, (_, k)) in SCALES.iter().enumerate() {
                        if self.sub_rect(i, j).contains(m) {
                            let cmd = if self.items[i] == Item::Resize {
                                ObjectCmd::Resize(*k)
                            } else {
                                ObjectCmd::SizeAll(*k)
                            };
                            actions.push(Action::Object(cmd, target));
                            self.close();
                            return;
                        }
                    }
                }
                if let Some(i) = (0..self.items.len()).find(|&i| self.item_rect(i).contains(m)) {
                    let cmd = match self.items[i] {
                        Item::Duplicate => Some(ObjectCmd::Duplicate),
                        Item::Pin => Some(ObjectCmd::TogglePin),
                        Item::Properties => Some(ObjectCmd::Properties),
                        Item::Unlink => Some(ObjectCmd::Unlink),
                        Item::Jelly => Some(ObjectCmd::Jelly),
                        Item::Flag => Some(ObjectCmd::Flag),
                        Item::Planet => Some(ObjectCmd::Planet),
                        Item::Delete => Some(ObjectCmd::Delete),
                        _ => None,
                    };
                    if let Some(cmd) = cmd {
                        actions.push(Action::Object(cmd, target));
                        self.close();
                    }
                    return;
                }
            }
            if !in_menu {
                // Let a right click elsewhere re-open on another object.
                input.consumed = input.left_pressed;
                self.close();
            }
        }
    }

    pub fn draw(&self, mouse: Vec2) {
        if !self.fader.visible() {
            return;
        }
        let f = self.fader.value();
        let p = self.panel();
        let p = Rect::new(p.x, p.y - (1.0 - f) * 6.0, p.w, p.h);
        panel(p, f);
        text_bold(&self.title, p.x + 12.0, baseline(p.y + HEAD_H / 2.0 + 4.0, 12.0), 12.0, fade(TEXT_MUTED, f));
        draw_line(p.x + 8.0, p.y + HEAD_H + 2.0, p.x + p.w - 8.0, p.y + HEAD_H + 2.0, 1.0, fade(BORDER, f));

        for (i, &item) in self.items.iter().enumerate() {
            let r = self.item_rect(i);
            let hov = r.contains(mouse) || self.sub == Some(i);
            if item == Item::Delete {
                draw_line(p.x + 8.0, r.y - 5.0, p.x + p.w - 8.0, r.y - 5.0, 1.0, fade(BORDER, f));
            }
            let danger = item == Item::Delete;
            if hov {
                rrect(r, 6.0, fade(if danger { alpha(DANGER, 0.22) } else { SURFACE_HI }, f));
            }
            let c = if danger {
                DANGER
            } else if hov {
                TEXT
            } else {
                TEXT_DIM
            };
            text_left(r, 10.0, item.label(self.pinned, self.planet), FONT, fade(c, f));
            if item.has_sub() {
                let (ax, ay) = (r.x + r.w - 14.0, r.y + r.h / 2.0);
                draw_triangle(vec2(ax, ay - 4.0), vec2(ax, ay + 4.0), vec2(ax + 5.0, ay), fade(c, f));
            }
        }

        if let Some(i) = self.sub {
            let sp = self.sub_panel(i);
            panel(sp, f);
            for (j, (label, _)) in SCALES.iter().enumerate() {
                let r = self.sub_rect(i, j);
                let hov = r.contains(mouse);
                if hov {
                    rrect(r, 6.0, fade(SURFACE_HI, f));
                }
                text_left(r, 10.0, label, FONT, fade(if hov { TEXT } else { TEXT_DIM }, f));
            }
        }
    }
}
