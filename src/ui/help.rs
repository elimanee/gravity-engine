//! F1 keybind reference.

use super::theme::*;
use super::{Fader, Input};
use macroquad::prelude::*;

const COLUMNS: &[&[(&str, &str)]] = &[
    &[
        ("#", "OBJECTS"),
        ("A", "Add images"),
        ("Drop", "Drop files onto the window"),
        ("N", "Shape spawner"),
        ("F", "Fetch 20 web buttons (88×31)"),
        ("Shift+F", "Fetch 20 game logos"),
        ("Right-click", "Object menu"),
        ("Del", "Delete object under cursor"),
        ("R", "Clear all objects"),
        ("O", "Drop a ragdoll (over an image: its head)"),
        ("U", "Jelly (over an object: it turns to jelly)"),
        ("Shift+U", "Cloth (over an image: a flag)"),
        ("#", "TOOLS"),
        ("Left-drag", "Use the current tool"),
        ("Tab", "Tool picker"),
        ("1 – 9", "Select a tool"),
        ("0", "Draw  (Shift pins)"),
        ("J", "Link: rope, spring, motor, glue"),
        ("Z", "Zone: wind, float, portal"),
        ("S", "Select: move, copy, glue"),
        ("K", "Pour sand, liquid, beads"),
        ("C", "Knife: cut ropes and objects"),
        ("Y", "Fire: burn things, melt ice"),
        ("L", "Gadgets: laser, thruster, cannon"),
        ("← ↑ → ↓", "Drive motors, fire gadgets"),
        ("Wheel", "Tool radius / thickness"),
        ("Shift+Wheel", "Tool strength"),
    ],
    &[
        ("#", "WORLD"),
        ("Space", "Pause · open settings"),
        (".", "Step one frame (paused)"),
        ("← (hold)", "Rewind time (Shift+← when ← drives)"),
        ("[  ]", "Slower / faster time"),
        ("+  -  Home", "Zoom in / out / fit"),
        ("Ctrl+Wheel", "Zoom at the pointer"),
        ("Middle-drag", "Pan the view"),
        ("B", "Next border mode"),
        ("G", "Next background"),
        ("Shift+G", "Background from image"),
        ("W", "Toggle window shake"),
        ("T", "Toggle trails"),
        ("H", "Toggle water"),
        ("#", "AUDIO"),
        ("M", "Load music / playlist"),
        ("P", "Play / pause music"),
        ("X", "Classic player (Winamp skins)"),
        ("Shift+X", "Physical player (throw it around)"),
        ("V", "Visualizer behind objects"),
        ("Shift+V", "Spawn a visualizer screen"),
    ],
    &[
        ("#", "EDIT"),
        ("Ctrl+Z", "Undo"),
        ("Ctrl+Y", "Redo"),
        ("Ctrl+C  V", "Copy / paste selection"),
        ("Ctrl+D", "Duplicate selection"),
        ("Ctrl+A", "Select everything"),
        ("Ctrl+G", "Glue selection together"),
        ("I", "Object properties"),
        ("#", "PLAY"),
        ("E", "Examples & challenges"),
        ("Shift+E", "Challenge editor"),
        ("Space", "Go! (in a challenge)"),
        ("R", "Retry (in a challenge)"),
        ("#", "SCENE"),
        ("Ctrl+S", "Save scene"),
        ("Ctrl+O", "Open scene or challenge"),
        ("F12", "Screenshot"),
        ("F11", "Record a GIF"),
        ("#", "OTHER"),
        ("D", "Debug overlay"),
        ("F1", "This help"),
        ("Esc", "Close / quit"),
        ("Q", "Quit"),
    ],
];

#[derive(Default)]
pub struct Help {
    pub fader: Fader,
}

impl Help {
    pub fn update(&mut self, dt: f32, input: &mut Input) {
        self.fader.update(dt, 7.0);
        if self.fader.open {
            if input.left_pressed || input.right_pressed {
                self.fader.open = false;
            }
            input.over_ui = true;
            input.consumed |= input.left_pressed || input.right_pressed;
            input.wheel = 0.0;
        }
    }

    pub fn draw(&self) {
        if !self.fader.visible() {
            return;
        }
        let f = self.fader.value();
        let (sw, sh) = (screen_width(), screen_height());
        draw_rectangle(0.0, 0.0, sw, sh, Color::new(0.0, 0.0, 0.02, 0.6 * f));

        let col_w = 262.0;
        let gap = 20.0;
        let pad = 26.0;
        let row_h = 25.0;
        let rows = COLUMNS.iter().map(|c| c.len()).max().unwrap_or(0);
        let w = COLUMNS.len() as f32 * col_w + (COLUMNS.len() - 1) as f32 * gap + pad * 2.0;
        let h = rows as f32 * row_h + pad * 2.0 + 44.0;
        let scale = (sw / (w + 20.0)).min(sh / (h + 20.0)).min(1.0);
        let (w, h) = (w * scale, h * scale);
        let x = (sw - w) / 2.0;
        let y = (sh - h) / 2.0 + (1.0 - f) * 12.0;
        let p = Rect::new(x, y, w, h);
        panel(p, f);

        let s = |v: f32| v * scale;
        text_bold("Controls", x + s(pad), y + s(pad) + s(14.0), s(20.0), fade(TEXT, f));
        let hint = format!("{} v{}  ·  click anywhere to close", crate::config::APP_NAME, crate::config::APP_VERSION);
        text(&hint, x + w - s(pad) - measure(&hint, s(12.0)), y + s(pad) + s(12.0), s(12.0), fade(TEXT_MUTED, f));

        for (ci, col) in COLUMNS.iter().enumerate() {
            let cx = x + s(pad) + ci as f32 * s(col_w + gap);
            let mut ry = y + s(pad) + s(44.0);
            for &(key, desc) in *col {
                if key == "#" {
                    super::widgets::section(cx, ry, s(col_w - 16.0), desc, f);
                } else {
                    let kw = keycap(cx, ry + s(12.0), key, s(11.0), f);
                    text(desc, cx + kw + s(10.0), baseline(ry + s(12.0), s(13.0)), s(13.0), fade(TEXT_DIM, f));
                }
                ry += s(row_h);
            }
        }
    }
}
