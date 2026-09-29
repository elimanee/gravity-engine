//! A classic Winamp-style player window drawn with a Winamp 2 / Audacious
//! skin (or a built-in look when there is none): transport buttons, the
//! time, a scrolling title, a spectrum analyzer, volume and seek bars. It
//! drives the app's own audio player.

use super::theme::*;
use super::{Action, Input, PlayerCmd};
use crate::audio::NowPlaying;
use crate::config::{PPM, WALL_T};
use crate::skin::{glyph, time_digits, Layout, Skin};
use macroquad::prelude::*;

/// Bars in the mini analyzer (3 px wide, 1 px apart).
const VIS_BARS: usize = 19;
const TITLE_H: f32 = 14.0;
const MARQUEE_SPEED: f32 = 28.0;

#[derive(Clone, Copy, PartialEq, Debug)]
enum Button {
    Previous,
    Play,
    Pause,
    Stop,
    Next,
    Eject,
    Close,
}

impl Button {
    const TRANSPORT: [Button; 6] =
        [Button::Previous, Button::Play, Button::Pause, Button::Stop, Button::Next, Button::Eject];

    /// Top-left and size in the window (skin px).
    fn rect(self, l: &Layout) -> Rect {
        let (p, size) = match self {
            Button::Previous => (l.previous, vec2(23.0, 18.0)),
            Button::Play => (l.play, vec2(23.0, 18.0)),
            Button::Pause => (l.pause, vec2(23.0, 18.0)),
            Button::Stop => (l.stop, vec2(23.0, 18.0)),
            Button::Next => (l.next, vec2(22.0, 18.0)),
            Button::Eject => (l.eject, vec2(22.0, 16.0)),
            Button::Close => (l.close, vec2(9.0, 9.0)),
        };
        Rect::new(p.x, p.y, size.x, size.y)
    }

    /// Source rectangle in `cbuttons.bmp`.
    fn sprite(self, pressed: bool) -> Rect {
        let (x, w, h) = match self {
            Button::Previous => (0.0, 23.0, 18.0),
            Button::Play => (23.0, 23.0, 18.0),
            Button::Pause => (46.0, 23.0, 18.0),
            Button::Stop => (69.0, 23.0, 18.0),
            Button::Next => (92.0, 22.0, 18.0),
            Button::Eject => (114.0, 22.0, 16.0),
            Button::Close => (18.0, 9.0, 9.0),
        };
        Rect::new(x, if pressed { h } else { 0.0 }, w, h)
    }

    fn cmd(self) -> PlayerCmd {
        match self {
            Button::Previous => PlayerCmd::Previous,
            Button::Play => PlayerCmd::Play,
            Button::Pause => PlayerCmd::Pause,
            Button::Stop => PlayerCmd::Stop,
            Button::Next => PlayerCmd::Next,
            Button::Eject => PlayerCmd::Eject,
            Button::Close => PlayerCmd::Close,
        }
    }
}

/// What is being dragged.
#[derive(Clone, Copy, PartialEq)]
enum Drag {
    /// The window, by this offset from its corner (screen px).
    Window(Vec2),
    Volume,
    /// The seek bar, at this fraction.
    Seek(f32),
}

#[derive(Default)]
pub struct SkinPlayer {
    /// Top-left corner on screen; placed bottom-right the first time.
    pos: Option<Vec2>,
    drag: Option<Drag>,
    pressed: Option<Button>,
    last_title_click: f64,
    marquee: f32,
    bars: [f32; VIS_BARS],
    peaks: [f32; VIS_BARS],
}

/// What the window shows.
pub struct PlayerView<'a> {
    pub skin: Option<&'a Skin>,
    pub now: Option<&'a NowPlaying>,
    pub volume: f32,
    pub double: bool,
}

impl PlayerView<'_> {
    fn layout(&self) -> Layout {
        self.skin.map_or_else(Layout::default, |s| s.layout.clone())
    }

    fn scale(&self) -> f32 {
        if self.double {
            2.0
        } else {
            1.0
        }
    }
}

impl SkinPlayer {
    fn origin(&self, size: Vec2) -> Vec2 {
        let (sw, sh) = (screen_width(), screen_height());
        let p = self.pos.unwrap_or(vec2(sw - size.x - 16.0, sh - WALL_T * PPM - size.y - 16.0));
        vec2(p.x.clamp(0.0, (sw - size.x).max(0.0)), p.y.clamp(0.0, (sh - size.y).max(0.0)))
    }

    /// Feed the analyzer bands (call every frame the window is shown).
    pub fn animate(&mut self, dt: f32, bands: &[f32]) {
        self.marquee += dt * MARQUEE_SPEED;
        let per = (bands.len() / VIS_BARS).max(1);
        for i in 0..VIS_BARS {
            let v = bands.iter().skip(i * per).take(per).fold(0.0f32, |m, b| m.max(*b)).clamp(0.0, 1.0);
            // Fast attack, slow fall, like Winamp.
            self.bars[i] = if v > self.bars[i] { v } else { (self.bars[i] - dt * 2.2).max(v) };
            self.peaks[i] =
                if self.bars[i] > self.peaks[i] { self.bars[i] } else { (self.peaks[i] - dt * 0.6).max(0.0) };
        }
    }

    pub fn update(&mut self, v: &PlayerView, input: &mut Input, actions: &mut Vec<Action>) {
        let l = v.layout();
        let scale = v.scale();
        let size = l.size * scale;
        let o = self.origin(size);
        let local = (input.mouse - o) / scale;
        let window = Rect::new(o.x, o.y, size.x, size.y);

        if input.left_pressed && !input.consumed && window.contains(input.mouse) {
            let hit = Button::TRANSPORT.into_iter().chain([Button::Close]).find(|b| b.rect(&l).contains(local));
            let volume = Rect::new(l.volume.x, l.volume.y, 68.0, 13.0);
            let seek = Rect::new(l.position.x, l.position.y, 248.0, 10.0);
            if let Some(b) = hit {
                self.pressed = Some(b);
            } else if volume.contains(local) {
                self.drag = Some(Drag::Volume);
            } else if seek.contains(local) && v.now.is_some_and(|n| n.duration.is_some()) {
                self.drag = Some(Drag::Seek(seek_frac(local.x, &l)));
            } else if local.y < TITLE_H {
                let now = get_time();
                if now - self.last_title_click < 0.35 {
                    actions.push(Action::Player(PlayerCmd::DoubleSize));
                }
                self.last_title_click = now;
                self.drag = Some(Drag::Window(input.mouse - o));
            }
        }
        match self.drag {
            Some(Drag::Window(grab)) if input.left_down => self.pos = Some(input.mouse - grab),
            Some(Drag::Volume) if input.left_down => {
                let frac = ((local.x - l.volume.x - 7.0) / (68.0 - 14.0)).clamp(0.0, 1.0);
                actions.push(Action::Player(PlayerCmd::Volume(frac)));
            }
            Some(Drag::Seek(_)) if input.left_down => self.drag = Some(Drag::Seek(seek_frac(local.x, &l))),
            Some(Drag::Seek(f)) => {
                actions.push(Action::Player(PlayerCmd::Seek(f)));
                self.drag = None;
            }
            Some(_) => self.drag = None,
            None => {}
        }
        if !input.left_down {
            if let Some(b) = self.pressed.take() {
                if b.rect(&l).contains(local) {
                    actions.push(Action::Player(b.cmd()));
                }
            }
        }
        if window.contains(input.mouse) || self.drag.is_some() || self.pressed.is_some() {
            input.over_ui = true;
            input.consumed |= input.left_pressed || input.right_pressed;
            input.wheel = 0.0;
        }
    }

    /// The window is being dragged or a control is held.
    pub fn busy(&self) -> bool {
        self.drag.is_some() || self.pressed.is_some()
    }

    pub fn draw(&self, v: &PlayerView, mouse: Vec2) {
        let l = v.layout();
        let scale = v.scale();
        let o = self.origin(l.size * scale);
        let local = (mouse - o) / scale;
        match v.skin {
            Some(skin) => self.draw_skin(skin, v, o, scale, local),
            None => self.draw_builtin(v, &l, o, scale, local),
        }
    }

    fn title_text(now: Option<&NowPlaying>) -> String {
        match now {
            Some(n) => {
                let len = n.duration.map_or(String::new(), |d| format!(" ({}:{:02})", d as u64 / 60, d as u64 % 60));
                format!("1. {}{len}", n.title)
            }
            None => "Gravity Engine  ·  press eject to load music".to_string(),
        }
    }

    fn draw_skin(&self, skin: &Skin, v: &PlayerView, o: Vec2, scale: f32, local: Vec2) {
        let l = &skin.layout;
        let sprite = |sheet: &str, src: Rect, at: Vec2| skin.sprite(sheet, src, at, o, scale);
        sprite("main", Rect::new(0.0, 0.0, l.size.x, l.size.y), Vec2::ZERO);
        if skin.has("titlebar") && l.size.x <= 275.0 {
            sprite("titlebar", Rect::new(27.0, 0.0, 275.0, 14.0), Vec2::ZERO);
        }
        let held = |b: Button| self.pressed == Some(b) && b.rect(l).contains(local);
        if skin.has("titlebar") {
            sprite("titlebar", Button::Close.sprite(held(Button::Close)), l.close);
        }
        for b in Button::TRANSPORT {
            sprite("cbuttons", b.sprite(held(b)), b.rect(l).point());
        }

        let now = v.now;
        let playing = now.is_some_and(|n| n.playing);
        // Play / pause / stop indicator.
        let status = match now {
            Some(n) if n.playing => 0.0,
            Some(_) => 9.0,
            None => 18.0,
        };
        sprite("playpaus", Rect::new(status, 0.0, 9.0, 9.0), l.play_status + vec2(2.0, 0.0));

        // Time.
        if let Some(n) = now {
            let blink = !n.playing && (get_time() * 2.0) as i64 % 2 == 1;
            if !blink {
                for (i, d) in time_digits(n.position).iter().enumerate() {
                    sprite("numbers", Rect::new(*d as f32 * 9.0, 0.0, 9.0, 13.0), l.numbers[i + 1]);
                }
            }
        }

        // Scrolling title.
        if l.text_visible && skin.has("text") {
            let title = Self::title_text(now);
            let chars = (l.text_width / 5.0).floor() as usize;
            let line: Vec<char> = title.chars().collect();
            let shown: Vec<char> = if line.len() <= chars {
                line
            } else {
                let looped: Vec<char> = line.iter().copied().chain("  ***  ".chars()).collect();
                let start = (self.marquee / 5.0) as usize % looped.len();
                looped.iter().cycle().skip(start).take(chars).copied().collect()
            };
            for (i, c) in shown.iter().enumerate() {
                let (col, row) = glyph(*c);
                sprite(
                    "text",
                    Rect::new(col as f32 * 5.0, row as f32 * 6.0, 5.0, 6.0),
                    l.text + vec2(i as f32 * 5.0, 0.0),
                );
            }
        }

        // Stereo light, repeat on (playback loops).
        if l.size.x <= 275.0 && l.othertext_visible {
            sprite("monoster", Rect::new(0.0, if playing { 0.0 } else { 12.0 }, 29.0, 12.0), l.stereo);
            sprite("monoster", Rect::new(29.0, 12.0, 27.0, 12.0), l.mono);
        }
        sprite("shufrep", Rect::new(0.0, 30.0, 28.0, 15.0), l.repeat);
        sprite("shufrep", Rect::new(28.0, 0.0, 47.0, 15.0), l.shuffle);

        // Volume and balance.
        let frame = (v.volume * 27.0).round();
        sprite("volume", Rect::new(0.0, frame * 15.0, 68.0, 13.0), l.volume);
        if skin.sheet_size("volume").y >= 433.0 {
            let pressed = self.drag == Some(Drag::Volume);
            let x = l.volume.x + v.volume * (68.0 - 14.0);
            sprite("volume", Rect::new(if pressed { 0.0 } else { 15.0 }, 422.0, 14.0, 11.0), vec2(x, l.volume.y + 1.0));
        }
        sprite("balance", Rect::new(9.0, 0.0, 38.0, 13.0), l.balance);
        if skin.sheet_size("balance").y >= 433.0 {
            sprite("balance", Rect::new(15.0, 422.0, 14.0, 11.0), l.balance + vec2(12.0, 1.0));
        }

        // Seek bar.
        if let Some(frac) = self.seek_position(now) {
            sprite("posbar", Rect::new(0.0, 0.0, 248.0, 10.0), l.position);
            let pressed = matches!(self.drag, Some(Drag::Seek(_)));
            let x = l.position.x + frac * (248.0 - 29.0);
            sprite("posbar", Rect::new(if pressed { 278.0 } else { 248.0 }, 0.0, 29.0, 10.0), vec2(x, l.position.y));
        }

        if l.vis_visible {
            self.draw_bars(&skin.vis, l.vis, o, scale);
        }
    }

    /// Where the seek thumb sits (None: nothing with a known length).
    fn seek_position(&self, now: Option<&NowPlaying>) -> Option<f32> {
        if let Some(Drag::Seek(f)) = self.drag {
            return Some(f);
        }
        let n = now?;
        let d = n.duration.filter(|d| *d > 0.0)?;
        Some(((n.position % d) / d).clamp(0.0, 1.0) as f32)
    }

    /// The mini spectrum analyzer (76 × 16 skin px).
    fn draw_bars(&self, vis: &[Color; 24], at: Vec2, o: Vec2, scale: f32) {
        for (i, (&v, &peak)) in self.bars.iter().zip(&self.peaks).enumerate() {
            let x = o.x + (at.x + i as f32 * 4.0) * scale;
            let h = (v * 16.0).round() as usize;
            for row in 0..h.min(16) {
                // Colour 17 at the bottom up to colour 2 at the top.
                let y = o.y + (at.y + 15.0 - row as f32) * scale;
                draw_rectangle(x, y, 3.0 * scale, scale, vis[17 - row.min(15)]);
            }
            let py = (peak * 16.0).round().min(16.0);
            if py >= 1.0 {
                draw_rectangle(x, o.y + (at.y + 16.0 - py) * scale, 3.0 * scale, scale, vis[23]);
            }
        }
    }

    /// No skin installed: the same window, drawn in the app's own style.
    fn draw_builtin(&self, v: &PlayerView, l: &Layout, o: Vec2, s: f32, local: Vec2) {
        let r = |x: f32, y: f32, w: f32, h: f32| Rect::new(o.x + x * s, o.y + y * s, w * s, h * s);
        let win = r(0.0, 0.0, l.size.x, l.size.y);
        panel(win, 1.0);
        let title = r(0.0, 0.0, l.size.x, TITLE_H);
        draw_rectangle(title.x + 4.0, title.y + 4.0, title.w - 8.0, title.h - 4.0, alpha(ACCENT, 0.18));
        text_centered("GRAVITY AMP", title.x + title.w / 2.0, title.y + title.h / 2.0 + 2.0, 8.0 * s, TEXT_DIM);
        let close = Button::Close.rect(l);
        let c = r(close.x, close.y, close.w, close.h);
        draw_line(c.x + 2.0, c.y + 2.0, c.x + c.w - 2.0, c.y + c.h - 2.0, s, TEXT_MUTED);
        draw_line(c.x + c.w - 2.0, c.y + 2.0, c.x + 2.0, c.y + c.h - 2.0, s, TEXT_MUTED);

        // Display.
        let lcd = r(10.0, 20.0, 94.0, 43.0);
        rrect(lcd, 3.0 * s, Color::new(0.02, 0.03, 0.05, 1.0));
        let now = v.now;
        let green = Color::new(0.35, 0.95, 0.45, 1.0);
        if let Some(n) = now {
            let t = time_digits(n.position);
            let label = format!("{}{}:{}{}", t[0], t[1], t[2], t[3]);
            text_bold(&label, o.x + l.numbers[1].x * s, o.y + (l.numbers[1].y + 11.0) * s, 13.0 * s, green);
        }
        let marquee = r(l.text.x - 3.0, l.text.y - 3.0, l.text_width + 6.0, 12.0);
        rrect(marquee, 2.0 * s, Color::new(0.02, 0.03, 0.05, 1.0));
        clip_text(&Self::title_text(now), marquee, self.marquee, 7.0 * s, green);
        let vis = [Color::new(0.02, 0.03, 0.05, 1.0); 2]
            .into_iter()
            .chain((2..18).map(|i| mix(Color::new(1.0, 0.3, 0.3, 1.0), green, (i - 2) as f32 / 15.0)))
            .chain(std::iter::repeat_n(WHITE, 6))
            .collect::<Vec<_>>();
        let vis: [Color; 24] = vis.try_into().unwrap_or([green; 24]);
        self.draw_bars(&vis, l.vis, o, s);

        // Buttons.
        for b in Button::TRANSPORT {
            let br = b.rect(l);
            let held = self.pressed == Some(b) && br.contains(local);
            let rr = r(br.x, br.y, br.w, br.h);
            rrect(rr, 3.0 * s, if held { SURFACE_HI } else { SURFACE_2 });
            rrect_lines(rr, 3.0 * s, 1.0, BORDER);
            let (cx, cy, u) = (rr.x + rr.w / 2.0, rr.y + rr.h / 2.0, 3.0 * s);
            let col = TEXT;
            let tri = |x: f32, dir: f32| {
                draw_triangle(vec2(x - u * dir, cy - u), vec2(x - u * dir, cy + u), vec2(x + u * dir, cy), col)
            };
            match b {
                Button::Previous => {
                    draw_rectangle(cx - u * 1.6, cy - u, s, u * 2.0, col);
                    tri(cx - u * 0.2, -1.0);
                }
                Button::Play => tri(cx, 1.0),
                Button::Pause => {
                    draw_rectangle(cx - u, cy - u, u * 0.7, u * 2.0, col);
                    draw_rectangle(cx + u * 0.3, cy - u, u * 0.7, u * 2.0, col);
                }
                Button::Stop => draw_rectangle(cx - u, cy - u, u * 2.0, u * 2.0, col),
                Button::Next => {
                    tri(cx + u * 0.2, 1.0);
                    draw_rectangle(cx + u * 1.3, cy - u, s, u * 2.0, col);
                }
                Button::Eject => {
                    draw_triangle(vec2(cx - u, cy), vec2(cx + u, cy), vec2(cx, cy - u * 1.2), col);
                    draw_rectangle(cx - u, cy + u * 0.4, u * 2.0, s, col);
                }
                Button::Close => {}
            }
        }

        // Volume and seek bars.
        let vol = r(l.volume.x, l.volume.y + 4.0, 68.0, 5.0);
        rrect(vol, 2.0 * s, SURFACE_HI);
        rrect(Rect::new(vol.x, vol.y, vol.w * v.volume, vol.h), 2.0 * s, alpha(ACCENT, 0.9));
        text("VOL", vol.x + vol.w + 4.0 * s, vol.y + 5.0 * s, 7.0 * s, TEXT_MUTED);
        let seek = r(l.position.x, l.position.y + 2.0, 248.0, 6.0);
        rrect(seek, 2.0 * s, SURFACE_HI);
        if let Some(f) = self.seek_position(now) {
            rrect(Rect::new(seek.x, seek.y, seek.w * f, seek.h), 2.0 * s, alpha(SUCCESS, 0.8));
        }
    }
}

fn seek_frac(x: f32, l: &Layout) -> f32 {
    ((x - l.position.x - 14.5) / (248.0 - 29.0)).clamp(0.0, 1.0)
}

/// `label` in `r`, scrolling when too long.
fn clip_text(label: &str, r: Rect, scroll: f32, size: f32, color: Color) {
    let w = measure(label, size);
    let pad = 3.0;
    super::widgets::clip(Some(r));
    if w <= r.w - pad * 2.0 {
        text(label, r.x + pad, r.y + r.h / 2.0 + size * 0.35, size, color);
    } else {
        let gap = 30.0;
        let off = (scroll * 1.2) % (w + gap);
        for k in 0..2 {
            text(label, r.x + pad - off + k as f32 * (w + gap), r.y + r.h / 2.0 + size * 0.35, size, color);
        }
    }
    super::widgets::clip(None);
}
