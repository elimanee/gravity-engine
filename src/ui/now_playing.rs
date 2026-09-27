//! "Now playing" pill at the bottom centre.

use super::theme::*;
use super::{Action, Input};
use crate::audio::NowPlaying;
use crate::config::{PPM, WALL_T};
use crate::util::{approach, ellipsize};
use macroquad::prelude::*;

#[derive(Default)]
pub struct NowPlayingPill {
    shown: f32,
    /// Keeps the pill visible for a moment after a change.
    linger: f32,
    last_title: String,
    last_playing: bool,
}

impl NowPlayingPill {
    fn rect(label_w: f32, shown: f32) -> Rect {
        let (sw, sh) = (screen_width(), screen_height());
        let w = label_w + 70.0;
        let h = 34.0;
        Rect::new((sw - w) / 2.0, sh - WALL_T * PPM - 10.0 - h + (1.0 - shown) * 60.0, w, h)
    }

    fn label(np: &NowPlaying) -> String {
        let mut s = format!("{}  ·  {}", np.fmt, ellipsize(&np.title, 36));
        if let Some(d) = &np.detail {
            s.push_str(&format!("  ·  {d}"));
        }
        s
    }

    pub fn update(
        &mut self,
        dt: f32,
        np: Option<&NowPlaying>,
        paused: bool,
        input: &mut Input,
        actions: &mut Vec<Action>,
    ) {
        self.linger = (self.linger - dt).max(0.0);
        let Some(np) = np else {
            self.shown = approach(self.shown, 0.0, 10.0, dt);
            return;
        };
        if np.title != self.last_title || np.playing != self.last_playing {
            self.last_title = np.title.clone();
            self.last_playing = np.playing;
            self.linger = 4.0;
        }
        let near_bottom =
            input.mouse.y > screen_height() - 90.0 && (input.mouse.x - screen_width() / 2.0).abs() < 260.0;
        let want = paused || near_bottom || self.linger > 0.0;
        self.shown = approach(self.shown, if want { 1.0 } else { 0.0 }, 10.0, dt);
        if self.shown > 0.5 {
            let r = Self::rect(measure(&Self::label(np), 13.0), self.shown);
            if super::widgets::button(r, input) {
                actions.push(Action::ToggleAudio);
            }
            input.block(r);
        }
    }

    pub fn draw(&self, np: Option<&NowPlaying>, mouse: Vec2) {
        let Some(np) = np else { return };
        if self.shown < 0.01 {
            return;
        }
        let f = self.shown;
        let label = Self::label(np);
        let r = Self::rect(measure(&label, 13.0), f);
        panel(Rect::new(r.x, r.y, r.w, r.h), f);
        if r.contains(mouse) {
            rrect(r, RADIUS, fade(SURFACE_HI, f * 0.5));
        }
        // Equaliser bars / pause glyph.
        let (bx, cy) = (r.x + 18.0, r.y + r.h / 2.0);
        let t = get_time() as f32;
        if np.playing {
            for i in 0..4 {
                let hgt = 4.0 + ((t * 7.0 + i as f32 * 1.7).sin() * 0.5 + 0.5) * 10.0;
                rrect(Rect::new(bx - 6.0 + i as f32 * 4.0, cy + 7.0 - hgt, 3.0, hgt), 1.0, fade(SUCCESS, f));
            }
        } else {
            draw_rectangle(bx - 5.0, cy - 6.0, 3.5, 12.0, fade(TEXT_MUTED, f));
            draw_rectangle(bx + 1.5, cy - 6.0, 3.5, 12.0, fade(TEXT_MUTED, f));
        }
        text(&label, r.x + 36.0, baseline(cy, 13.0), 13.0, fade(if np.playing { TEXT } else { TEXT_DIM }, f));
        let hint = "P";
        keycap(r.x + r.w - 26.0, cy, hint, 10.0, f * 0.8);
    }
}
