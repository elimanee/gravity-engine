//! The equalizer window (`eqmain.bmp`): on / off, presets, a preamp and ten
//! band sliders, and the response curve.

use super::*;
use crate::audio::eq::MAX_DB;

pub const SIZE: Vec2 = vec2(275.0, 116.0);
/// Slider tops (skin px): the preamp, then the ten bands.
const SLIDER_Y: f32 = 38.0;
const SLIDER_TRAVEL: f32 = 51.0;
const PREAMP_X: f32 = 21.0;
const BAND_X: f32 = 78.0;
const BAND_STEP: f32 = 18.0;
/// The response graph.
const GRAPH: Rect = Rect { x: 86.0, y: 17.0, w: 113.0, h: 19.0 };

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum EqButton {
    On,
    Presets,
    Close,
}

impl EqButton {
    pub fn rect(self) -> Rect {
        match self {
            EqButton::On => Rect::new(14.0, 18.0, 26.0, 12.0),
            EqButton::Presets => Rect::new(217.0, 18.0, 44.0, 12.0),
            EqButton::Close => Rect::new(264.0, 3.0, 9.0, 9.0),
        }
    }

    pub fn cmd(self) -> PlayerCmd {
        match self {
            EqButton::On => PlayerCmd::EqOn,
            EqButton::Presets => PlayerCmd::EqPreset,
            EqButton::Close => PlayerCmd::ToggleEq,
        }
    }
}

/// Left edge of slider `i` (0 = preamp).
fn slider_x(i: usize) -> f32 {
    if i == 0 {
        PREAMP_X
    } else {
        BAND_X + (i - 1) as f32 * BAND_STEP
    }
}

/// Gain (dB) for the pointer at `y` (window px) on a slider.
pub fn slider_gain(y: f32) -> f32 {
    let t = ((y - SLIDER_Y - 5.5) / SLIDER_TRAVEL).clamp(0.0, 1.0);
    let g = MAX_DB - t * 2.0 * MAX_DB;
    // Snap to 0 dB near the middle, like the original.
    if g.abs() < 0.8 {
        0.0
    } else {
        g
    }
}

fn gain_of(v: &PlayerView, i: usize) -> f32 {
    if i == 0 {
        v.eq.preamp
    } else {
        v.eq.bands[i - 1]
    }
}

impl SkinPlayer {
    pub(super) fn eq_press(&mut self, _v: &PlayerView, local: Vec2, grab: Vec2, actions: &mut Vec<Action>) {
        if let Some(b) =
            [EqButton::On, EqButton::Presets, EqButton::Close].into_iter().find(|b| b.rect().contains(local))
        {
            self.pressed = Some(Press::Eq(b));
        } else if let Some(i) = (0..11).find(|&i| Rect::new(slider_x(i), SLIDER_Y, 14.0, 63.0).contains(local)) {
            self.drag = Some(Drag::EqSlider(i));
            actions.push(Action::Player(PlayerCmd::EqGain(i, slider_gain(local.y))));
        } else if local.y < TITLE_H {
            self.title_press(grab, actions);
        }
    }

    pub(super) fn eq_draw(&self, v: &PlayerView, o: Vec2, s: f32, local: Vec2) {
        match v.skin.filter(|k| k.has("eqmain")) {
            Some(skin) => self.eq_draw_skin(skin, v, o, s, local),
            None => self.eq_draw_builtin(v, o, s, local),
        }
    }

    fn eq_held(&self, b: EqButton, local: Vec2) -> bool {
        self.pressed == Some(Press::Eq(b)) && b.rect().contains(local)
    }

    fn eq_draw_skin(&self, skin: &Skin, v: &PlayerView, o: Vec2, s: f32, local: Vec2) {
        let sprite = |src: Rect, at: Vec2| skin.sprite("eqmain", src, at, o, s);
        sprite(Rect::new(0.0, 0.0, 275.0, 116.0), Vec2::ZERO);
        sprite(Rect::new(0.0, 134.0, 275.0, 14.0), Vec2::ZERO);
        let on = v.eq.on;
        let pressed = self.eq_held(EqButton::On, local);
        let on_x = match (on, pressed) {
            (false, false) => 10.0,
            (true, false) => 69.0,
            (false, true) => 128.0,
            (true, true) => 187.0,
        };
        sprite(Rect::new(on_x, 119.0, 26.0, 12.0), EqButton::On.rect().point());
        sprite(Rect::new(36.0, 119.0, 32.0, 12.0), vec2(40.0, 18.0));
        let presets_y = if self.eq_held(EqButton::Presets, local) { 176.0 } else { 164.0 };
        sprite(Rect::new(224.0, presets_y, 44.0, 12.0), EqButton::Presets.rect().point());
        let close_y = if self.eq_held(EqButton::Close, local) { 125.0 } else { 116.0 };
        sprite(Rect::new(0.0, close_y, 9.0, 9.0), EqButton::Close.rect().point());

        // Response curve on the graph background.
        sprite(Rect::new(0.0, 294.0, 113.0, 19.0), GRAPH.point());
        self.eq_curve(v, o, s, skin.vis[2], skin.vis[12]);

        for i in 0..11 {
            let g = gain_of(v, i);
            // 28 background frames, from +12 dB (0) to −12 dB (27).
            let frame = (((MAX_DB - g) / (2.0 * MAX_DB)) * 27.0).round().clamp(0.0, 27.0) as usize;
            let src = Rect::new(13.0 + (frame % 14) as f32 * 15.0, 164.0 + (frame / 14) as f32 * 65.0, 14.0, 63.0);
            sprite(src, vec2(slider_x(i), SLIDER_Y));
            let held = self.drag == Some(Drag::EqSlider(i));
            let y = SLIDER_Y + (MAX_DB - g) / (2.0 * MAX_DB) * SLIDER_TRAVEL;
            sprite(Rect::new(0.0, if held { 176.0 } else { 164.0 }, 11.0, 11.0), vec2(slider_x(i) + 1.0, y));
        }
    }

    /// The bands joined by a line across the graph.
    fn eq_curve(&self, v: &PlayerView, o: Vec2, s: f32, high: Color, low: Color) {
        let point = |k: usize| {
            let x = GRAPH.x + 2.0 + k as f32 * (GRAPH.w - 4.0) / 9.0;
            let g = v.eq.bands[k];
            let y = GRAPH.y + GRAPH.h / 2.0 - g / MAX_DB * (GRAPH.h / 2.0 - 1.5);
            (o + vec2(x, y) * s, g)
        };
        for k in 0..9 {
            let ((a, ga), (b, gb)) = (point(k), point(k + 1));
            let c = mix(low, high, ((ga + gb) / 2.0 / MAX_DB * 0.5 + 0.5).clamp(0.0, 1.0));
            let c = if v.eq.on { c } else { alpha(c, 0.45) };
            draw_line(a.x, a.y, b.x, b.y, s.max(1.0), c);
        }
    }

    fn eq_draw_builtin(&self, v: &PlayerView, o: Vec2, s: f32, local: Vec2) {
        let r = |x: f32, y: f32, w: f32, h: f32| Rect::new(o.x + x * s, o.y + y * s, w * s, h * s);
        panel(r(0.0, 0.0, SIZE.x, SIZE.y), 1.0);
        text_centered("EQUALIZER", o.x + SIZE.x / 2.0 * s, o.y + 9.0 * s, 8.0 * s, TEXT_DIM);
        let c = r(264.0, 3.0, 9.0, 9.0);
        draw_line(c.x + 2.0, c.y + 2.0, c.x + c.w - 2.0, c.y + c.h - 2.0, s, TEXT_MUTED);
        draw_line(c.x + c.w - 2.0, c.y + 2.0, c.x + 2.0, c.y + c.h - 2.0, s, TEXT_MUTED);
        for (b, label, lit) in [(EqButton::On, "ON", v.eq.on), (EqButton::Presets, "PRESET", false)] {
            let br = b.rect();
            let rr = r(br.x, br.y, br.w, br.h);
            let held = self.eq_held(b, local);
            rrect(
                rr,
                3.0 * s,
                if lit {
                    alpha(ACCENT, 0.4)
                } else if held {
                    SURFACE_HI
                } else {
                    SURFACE_2
                },
            );
            rrect_lines(rr, 3.0 * s, 1.0, if lit { ACCENT } else { BORDER });
            text_centered(label, rr.x + rr.w / 2.0, rr.y + rr.h / 2.0 + s, 6.5 * s, TEXT);
        }
        rrect(r(GRAPH.x, GRAPH.y, GRAPH.w, GRAPH.h), 2.0 * s, Color::new(0.02, 0.03, 0.05, 1.0));
        self.eq_curve(v, o, s, Color::new(1.0, 0.45, 0.35, 1.0), Color::new(0.35, 0.95, 0.45, 1.0));
        const LABELS: [&str; 11] = ["PRE", "60", "170", "310", "600", "1K", "3K", "6K", "12K", "14K", "16K"];
        for (i, label) in LABELS.iter().enumerate() {
            let x = slider_x(i);
            let track = r(x + 5.0, SLIDER_Y + 3.0, 4.0, SLIDER_TRAVEL + 5.0);
            rrect(track, 2.0 * s, SURFACE_HI);
            let mid = track.y + track.h / 2.0;
            let g = gain_of(v, i);
            let y = o.y + (SLIDER_Y + 5.5 + (MAX_DB - g) / (2.0 * MAX_DB) * SLIDER_TRAVEL) * s;
            let fill = if v.eq.on { alpha(ACCENT, 0.9) } else { alpha(TEXT_MUTED, 0.6) };
            draw_rectangle(track.x, y.min(mid), track.w, (y - mid).abs(), fill);
            let held = self.drag == Some(Drag::EqSlider(i));
            rrect(
                Rect::new(o.x + (x + 1.5) * s, y - 3.0 * s, 11.0 * s, 6.0 * s),
                2.0 * s,
                if held { TEXT } else { TEXT_DIM },
            );
            text_centered(label, o.x + (x + 7.0) * s, o.y + 110.0 * s, 5.5 * s, TEXT_MUTED);
        }
    }
}
